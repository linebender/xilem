// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, marker::PhantomData, ops::Range};

use masonry::core::{AnyWidget, FromDynWidget, WidgetPod};
use masonry::widgets::{self, VirtualScrollAction};
use private::VirtualScrollState;
use xilem_core::{AsyncCtx, DynMessage, MessageResult, View, ViewId, ViewMarker, ViewPathTracker};

use crate::{Pod, ViewCtx, WidgetView};

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
pub struct VirtualScroll<State, Action, ChildrenViews, F, Element: ?Sized> {
    phantom: PhantomData<fn() -> (WidgetPod<Element>, State, Action, ChildrenViews)>,
    func: F,
    valid_range: Range<i64>,
}

/// Component for [`VirtualScroll`].
///
/// Arguments:
/// - `valid_range` is the range of ids which are supported.
/// - `func` is the component for this element's children.
///   It is provided with the app's state and the index of the child.
///
/// For full details, see the documentation on the [view type](VirtualScroll).
pub fn virtual_scroll<State, Action, ChildrenViews, F, Element>(
    valid_range: Range<i64>,
    func: F,
) -> VirtualScroll<State, Action, ChildrenViews, F, Element>
where
    ChildrenViews: WidgetView<State, Action, Widget = Element>,
    F: Fn(&mut State, i64) -> ChildrenViews + 'static,
    Element: AnyWidget + FromDynWidget + ?Sized,
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
    use std::{collections::HashMap, sync::Arc};

    use masonry::widgets::VirtualScrollAction;
    use xilem_core::ViewId;

    #[expect(
        unnameable_types,
        reason = "Not meaningful public API; required to be public due to design of View trait"
    )]
    pub struct VirtualScrollState<View, State> {
        pub(super) pending_action: Option<VirtualScrollAction>,
        pub(super) previous_views: HashMap<i64, View>,
        pub(super) current_updated: bool,
        pub(super) pending_children_update: bool,
        pub(super) current_views: HashMap<i64, View>,
        pub(super) view_states: HashMap<i64, ChildState<State>>,
        pub(super) my_path: Arc<[ViewId]>,
        pub(super) cleanup_queue: Vec<i64>,
    }

    pub(super) struct ChildState<State> {
        pub(super) state: State,
        pub(super) requested_rebuild: bool,
    }
}

/// Create the view id used for child views.
///
/// This is a minimal function around [`i64::cast_unsigned`] (which is unstable, so polyfilled).
const fn view_id_for_index(idx: i64) -> ViewId {
    /* i64::cast_unsigned is unstable */
    ViewId::new(idx as u64)
}

/// Get the index stored in the view id.
///
/// This is a minimal function around [`u64::cast_signed`] (which is unstable, so polyfilled).
const fn index_for_view_id(id: ViewId) -> i64 {
    /* u64::cast_unsigned is unstable */
    id.routing_id() as i64
}

#[derive(Debug)]
struct UpdateVirtualChildren;

impl<State, Action, ChildrenViews, F, Element: AnyWidget + FromDynWidget + ?Sized> ViewMarker
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
    Element: AnyWidget + FromDynWidget + ?Sized,
{
    type Element = Pod<widgets::VirtualScroll<Element>>;

    type ViewState = VirtualScrollState<ChildrenViews, ChildrenViews::ViewState>;

    fn build(&self, ctx: &mut ViewCtx, _: &mut State) -> (Self::Element, Self::ViewState) {
        // TODO: How does the anchor interact with Xilem?
        // Setting that seems like an imperative action?
        let widget = Pod::new(
            widgets::VirtualScroll::<Element>::new(0).with_valid_range(self.valid_range.clone()),
        );
        ctx.record_action(widget.id);
        (
            widget,
            private::VirtualScrollState {
                pending_action: None,
                previous_views: HashMap::default(),
                current_updated: false,
                pending_children_update: false,
                current_views: HashMap::default(),
                view_states: HashMap::default(),
                my_path: ctx.view_path().into(),
                cleanup_queue: Vec::default(),
            },
        )
    }

    // TODO(DJMcNab): Remove this back/forth/back/forth messaging, now that this is no longer true
    // This implementation is ugly. This is needed because the `rebuild` function
    // doesn't have access to the app's state. The way we handle this is:
    // 1) If the app's state has changed since the last rebuild (i.e. the `app_logic` function has been rerun),
    //    we send a message to ourselves to recreate all of our loaded children.
    // 2) If there are new versions of our children, we rebuild/build those.
    // 3) We make sure to rebuild all children which have requested a rebuild.

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: xilem_core::Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        if self.valid_range != prev.valid_range {
            widgets::VirtualScroll::set_valid_range(&mut element, self.valid_range.clone());
        }
        if ctx.state_changed() && !view_state.pending_children_update {
            let proxy = ctx.proxy();
            proxy
                .send_message(
                    view_state.my_path.clone(),
                    DynMessage::new(UpdateVirtualChildren),
                )
                .unwrap();
            view_state.pending_children_update = true;
            // We think it would be fine to not actually rebuild here (and wait for the message to be handled)
            // but rebuilding here does still work
        }
        let used_action = view_state.pending_action.is_some();
        if let Some(pending_action) = view_state.pending_action.take() {
            debug_assert!(view_state.current_updated);
            widgets::VirtualScroll::will_handle_action(&mut element, &pending_action);
        }
        if !view_state.current_updated {
            // Our state hasn't changed, but we need to rebuild any children which have requested it.
            for (idx, view) in &view_state.previous_views {
                let child_state = view_state
                    .view_states
                    .get_mut(idx)
                    .expect("`view_states` is always in sync with `previous_views`");
                if child_state.requested_rebuild {
                    ctx.with_id(view_id_for_index(*idx), |ctx| {
                        // Note that we rebuild the view with itself, because we're only actioning a requested rebuild.
                        // This should be a no-op other than whatever caused the requested rebuild.
                        view.rebuild(
                            view,
                            &mut child_state.state,
                            ctx,
                            widgets::VirtualScroll::child_mut(&mut element, *idx),
                            app_state,
                        );
                    });
                    child_state.requested_rebuild = false;
                }
            }
        } else {
            // Otherwise, our set of loaded children has changed, and/or our loaded children all have a new version.
            debug_assert!(view_state.cleanup_queue.is_empty());
            // Remove any children which have been unloaded.
            for (&idx, child) in &view_state.previous_views {
                if view_state.current_views.contains_key(&idx) {
                    // We will handle this in the second loop.
                    continue;
                }
                debug_assert!(
                    used_action,
                    "Xilem VirtualScroll: Would remove an item even though we weren't handling an action."
                );
                ctx.with_id(view_id_for_index(idx), |ctx| {
                    let child_state = view_state
                        .view_states
                        .get_mut(&idx)
                        .expect("`view_states` is always in sync with `previous_views`");
                    child.teardown(
                        &mut child_state.state,
                        ctx,
                        widgets::VirtualScroll::child_mut(&mut element, idx),
                        app_state,
                    );
                    widgets::VirtualScroll::remove_child(&mut element, idx);
                    view_state.cleanup_queue.push(idx);
                });
            }
            for to_cleanup in view_state.cleanup_queue.drain(..) {
                view_state
                    .previous_views
                    .remove(&to_cleanup)
                    .expect("Cleanup index is real item in list");
                view_state
                    .view_states
                    .remove(&to_cleanup)
                    .expect("`view_states` is always in sync with `previous_views`");
            }
            for (idx, child) in view_state.current_views.drain() {
                ctx.with_id(view_id_for_index(idx), |ctx| {
                    if let Some(child_prev) = view_state.previous_views.get(&idx) {
                        // If there was previously a version of this view (i.e. we only updated it)
                        // then perform only a rebuild.
                        let child_state = view_state
                            .view_states
                            .get_mut(&idx)
                            .expect("`view_states` is always in sync with `previous_views`");
                        child.rebuild(
                            child_prev,
                            &mut child_state.state,
                            ctx,
                            widgets::VirtualScroll::child_mut(&mut element, idx),
                            app_state,
                        );
                        child_state.requested_rebuild = false;
                        view_state.previous_views.insert(idx, child);
                    } else {
                        debug_assert!(used_action);
                        // Otherwise, build the first version of this view.
                        let (new_child, child_state) = child.build(ctx, app_state);
                        widgets::VirtualScroll::add_child(
                            &mut element,
                            idx,
                            new_child.into_widget_pod(),
                        );

                        view_state.previous_views.insert(idx, child);
                        view_state.view_states.insert(
                            idx,
                            private::ChildState {
                                state: child_state,
                                requested_rebuild: false,
                            },
                        );
                    }
                });
            }
            // We have just "used" up the current states, so mark them as inactive.
            view_state.current_updated = false;
        }
        debug_assert_eq!(
            view_state.previous_views.len(),
            view_state.view_states.len()
        );
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: xilem_core::Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        for (&idx, child) in &view_state.previous_views {
            ctx.with_id(view_id_for_index(idx), |ctx| {
                let view_state = view_state.view_states.get_mut(&idx).unwrap();
                child.teardown(
                    &mut view_state.state,
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
        id_path: &[xilem_core::ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> xilem_core::MessageResult<Action> {
        if let [first, tail @ ..] = id_path {
            let child_idx = index_for_view_id(*first);
            let target = view_state.previous_views.get(&child_idx);
            if let Some(target) = target {
                let state = view_state.view_states.get_mut(&child_idx).unwrap();
                let result = target.message(&mut state.state, tail, message, app_state);
                if matches!(result, MessageResult::RequestRebuild) {
                    state.requested_rebuild = true;
                }
                return result;
            } else {
                tracing::error!("Message sent type in VirtualScroll::message: {message:?}");
                return MessageResult::Stale(message);
            }
        }
        if message.is::<VirtualScrollAction>() {
            let action = message.downcast::<VirtualScrollAction>().unwrap();

            view_state.current_updated = true;
            // We know that the `current_views` have not been applied, so we can just brute force overwrite them.
            view_state.current_views.clear();
            for new_targets in action.target.clone() {
                // TODO: Ideally, we'd avoid updating the already existing items
                // Doing so however dramatically increases the complexity in `rebuild`
                view_state
                    .current_views
                    .insert(new_targets, (self.func)(app_state, new_targets));
            }
            view_state.pending_action = Some(*action);
            MessageResult::RequestRebuild
        } else if message.is::<UpdateVirtualChildren>() {
            view_state.current_updated = true;
            view_state.current_views.clear();
            view_state.pending_children_update = false;
            for &key in view_state.previous_views.keys() {
                view_state
                    .current_views
                    .insert(key, (self.func)(app_state, key));
            }
            MessageResult::RequestRebuild
        } else {
            tracing::error!("Wrong message type in VirtualScroll::message: {message:?}");
            MessageResult::Stale(message)
        }
    }
}
