// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, marker::PhantomData, ops::Range};

use masonry::core::{FromDynWidget, Widget, WidgetPod};
use masonry::util::debug_panic;
use masonry::widgets::{self, VirtualScrollAction};
use private::VirtualScrollState;

use crate::core::{MessageContext, MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker};
use crate::{Pod, ViewCtx, WidgetView};

/// The view type for [`virtual_scroll`].
///
/// See its documentation for details.
pub struct VirtualScroll<State, Action, ChildrenViews, F, Element: ?Sized> {
    phantom: PhantomData<fn() -> (WidgetPod<Element>, State, Action, ChildrenViews)>,
    func: F,
    valid_range: Range<i64>,
}

/// A (vertical) virtual scrolling View, for Masonry's [`VirtualScroll`](widgets::VirtualScroll).
///
/// Virtual scrolling is a technique to improve performance when scrolling through long lists, by
/// only loading (and therefore laying out, drawing, processing for event handling), the items visible to the user.
///
/// The implementation has some caveats, which are discussed in the documentation of the [underlying widget](widgets::VirtualScroll).
///
/// Whenever this view is rebuilt, all of the loaded children are rebuild.
/// The child creation function is a "component" context, (alike to the usual `app_logic` functions), which means
/// that changing the app's state in this function will *not* cause a rebuild or rerunning of the app
/// logic (this avoids infinite loops).
/// It is correct for `func` to capture, if necessary.
/// However, it also has access to the app's state, so this is unlikely to be needed.
///
/// Arguments:
/// - `valid_range` is the range of ids which are supported.
/// - `func` is the component for this element's children.
///   It is provided with the app's state and the index of the child.
///
/// In rare circumstances, the index of the child could be outside of the requested valid range (this is
/// most likely to happen if the valid range changes due to something in `app_logic` updating it - e.g.
/// if a counter which decrements every time a parent component is called is used for the valid range).
/// As such, you should avoid panicking if the index is outside of a range you expect, and you are
/// changing the valid range. We expect this limitation to be lifted in the future.
///
/// For full details, see the documentation on the [view type](VirtualScroll).
pub fn virtual_scroll<State, Action, ChildrenViews, F, Element>(
    valid_range: Range<i64>,
    func: F,
) -> VirtualScroll<State, Action, ChildrenViews, F, Element>
where
    ChildrenViews: WidgetView<State, Action, Widget = Element>,
    F: Fn(&mut State, i64) -> ChildrenViews + 'static,
    Element: Widget + FromDynWidget + ?Sized,
{
    VirtualScroll {
        phantom: PhantomData,
        func,
        valid_range,
    }
}

/// Component for a [`VirtualScroll`] with unlimited children.
///
/// Arguments:
/// - `func` is the component for this element's children.
///   It is provided with the app's state and the index of the child.
///
/// For full details, see the documentation on the [view type](VirtualScroll).
pub fn unlimited_virtual_scroll<State, Action, ChildrenViews, F, Element>(
    func: F,
) -> VirtualScroll<State, Action, ChildrenViews, F, Element>
where
    ChildrenViews: WidgetView<State, Action, Widget = Element>,
    F: Fn(&mut State, i64) -> ChildrenViews + 'static,
{
    VirtualScroll {
        phantom: PhantomData,
        func,
        valid_range: i64::MIN..i64::MAX,
    }
}

mod private {
    use masonry::widgets::VirtualScrollAction;

    use std::collections::HashMap;

    #[expect(
        unnameable_types,
        reason = "Not meaningful public API; required to be public due to design of View trait"
    )]
    pub struct VirtualScrollState<View, State> {
        pub(super) pending_action: Option<VirtualScrollAction>,
        pub(super) children: HashMap<i64, ChildState<View, State>>,
    }

    pub(super) struct ChildState<View, State> {
        pub(super) view: View,
        pub(super) state: State,
    }
}

/// Create the view id used for child views.
const fn view_id_for_index(idx: i64) -> ViewId {
    ViewId::new(idx.cast_unsigned())
}

/// Get the index stored in the view id.
const fn index_for_view_id(id: ViewId) -> i64 {
    id.routing_id().cast_signed()
}

impl<State, Action, ChildrenViews, F, Element: Widget + FromDynWidget + ?Sized> ViewMarker
    for VirtualScroll<State, Action, ChildrenViews, F, Element>
{
}
impl<State, Action, Element, ChildrenViews, F> View<State, Action, ViewCtx>
    for VirtualScroll<State, Action, ChildrenViews, F, Element>
where
    State: 'static,
    Action: 'static,
    ChildrenViews: WidgetView<State, Action, Widget = Element>,
    F: Fn(&mut State, i64) -> ChildrenViews + 'static,
    Element: Widget + FromDynWidget + ?Sized,
{
    type Element = Pod<widgets::VirtualScroll<Element>>;

    type ViewState = VirtualScrollState<ChildrenViews, ChildrenViews::ViewState>;

    fn build(&self, ctx: &mut ViewCtx, _: &mut State) -> (Self::Element, Self::ViewState) {
        // TODO: How does the anchor interact with Xilem?
        // Setting that seems like an imperative action?
        let pod = Pod::new(
            widgets::VirtualScroll::<Element>::new(0).with_valid_range(self.valid_range.clone()),
        );
        ctx.record_action(pod.new_widget.id());
        (
            pod,
            private::VirtualScrollState {
                pending_action: None,
                children: HashMap::default(),
            },
        )
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        let valid_range_changed = self.valid_range != prev.valid_range;
        if valid_range_changed {
            widgets::VirtualScroll::set_valid_range(&mut element, self.valid_range.clone());
        }
        // TODO: This code should be moved into `Self::message` once it becomes possible to
        // make a build/rebuild/teardown context there.
        //
        // This is because we could now be requesting items which are outside the claimed "valid range".
        // Naïvely, one might expect this to be impossible (because we only request rebuild, so the `app_logic` isn't ran)
        // However, even in these cases, things like `lens` will still generate a new view, so it's conceivable that
        // the valid range has changed. As such, we document the possibility of these requests above.
        if let Some(pending_action) = view_state.pending_action.take() {
            widgets::VirtualScroll::will_handle_action(&mut element, &pending_action);
            // Teardown the old items
            for idx in pending_action.old_active.clone() {
                if !pending_action.target.contains(&idx) {
                    let Some(mut child_state) = view_state.children.remove(&idx) else {
                        debug_panic!(
                            "Tried to remove {idx} from virtual scroll {pending_action:?}, but it wasn't already present."
                        );
                        continue;
                    };
                    ctx.with_id(view_id_for_index(idx), |ctx| {
                        child_state.view.teardown(
                            &mut child_state.state,
                            ctx,
                            widgets::VirtualScroll::child_mut(&mut element, idx),
                            app_state,
                        );
                        widgets::VirtualScroll::remove_child(&mut element, idx);
                    });
                }
            }
            // Build all new items. Whilst we're here, rebuild all the others.
            // This avoids needing to carefully track which ones we just built.
            for idx in pending_action.target.clone() {
                if let Some(child) = view_state.children.get_mut(&idx) {
                    debug_assert!(
                        pending_action.old_active.contains(&idx),
                        "{idx} was asked to be removed in {pending_action:?}, but wasn't already present."
                    );
                    let next_child = (self.func)(app_state, idx);
                    // Rebuild this existing item
                    ctx.with_id(view_id_for_index(idx), |ctx| {
                        next_child.rebuild(
                            &child.view,
                            &mut child.state,
                            ctx,
                            widgets::VirtualScroll::child_mut(&mut element, idx),
                            app_state,
                        );
                        child.view = next_child;
                    });
                } else {
                    let new_child = (self.func)(app_state, idx);
                    // Build the new item
                    ctx.with_id(view_id_for_index(idx), |ctx| {
                        let (new_element, child_state) = new_child.build(ctx, app_state);
                        widgets::VirtualScroll::add_child(
                            &mut element,
                            idx,
                            new_element.new_widget,
                        );
                        view_state.children.insert(
                            idx,
                            private::ChildState {
                                view: new_child,
                                state: child_state,
                            },
                        )
                    });
                }
            }
        } else {
            // Rebuild all existing items
            for (&idx, child) in &mut view_state.children {
                let next_child = (self.func)(app_state, idx);
                ctx.with_id(view_id_for_index(idx), |ctx| {
                    next_child.rebuild(
                        &child.view,
                        &mut child.state,
                        ctx,
                        widgets::VirtualScroll::child_mut(&mut element, idx),
                        app_state,
                    );
                    child.view = next_child;
                });
            }
        }
        debug_assert_eq!(
            element.widget.len(),
            view_state.children.len(),
            "VirtualScroll: Child added outside of the control of Xilem."
        );
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        for (&idx, child) in &mut view_state.children {
            ctx.with_id(view_id_for_index(idx), |ctx| {
                child.view.teardown(
                    &mut child.state,
                    ctx,
                    widgets::VirtualScroll::child_mut(&mut element, idx),
                    app_state,
                );
            });
        }
        ctx.teardown_leaf(element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageContext,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> xilem_core::MessageResult<Action> {
        if let Some(first) = message.take_first() {
            let child_idx = index_for_view_id(first);
            let target = view_state.children.get_mut(&child_idx);
            // TODO: Unfortunately, this isn't robust, because the message might be trying to reach a previous child.
            // We definitely don't want an O(n) storage of data for previous generations, but using a u64 generation
            // can never reasonably overflow (i.e. we should use two viewids here).
            if let Some(target) = target {
                let result = target.view.message(
                    &mut target.state,
                    message,
                    widgets::VirtualScroll::child_mut(&mut element, child_idx),
                    app_state,
                );
                return result;
            } else {
                tracing::error!("Message sent type in VirtualScroll::message: {message:?}");
                return MessageResult::Stale;
            }
        }
        if let Some(action) = message.take_message::<VirtualScrollAction>() {
            // TODO: We should be able to rebuild here (we have the element)
            // but we currently can't make a `ViewCtx`
            view_state.pending_action = Some(*action);
            // We need rebuild to be called now.
            MessageResult::RequestRebuild
        } else {
            tracing::error!(?message, "Wrong message type in VirtualScroll::message");
            MessageResult::Stale
        }
    }
}
