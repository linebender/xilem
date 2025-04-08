// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::{
    core::Widget,
    widgets::{self, VirtualScrollAction},
};
use private::VirtualScrollState;
use xilem_core::{AsyncCtx, DynMessage, MessageResult, View, ViewId, ViewMarker, ViewPathTracker};

use crate::{Pod, ViewCtx, WidgetView};

pub struct VirtualScroll<State, Action, ChildrenViews, F, Element> {
    phantom: PhantomData<fn() -> (Element, State, Action, ChildrenViews)>,
    // TODO: Work out whether `func` need to be zero sized?
    // TODO: Assume for the sake of argument that it does.
    func: F,
    // TODO: If https://github.com/linebender/xilem/pull/906 gets merged.
    // valid_range: Range<i64>,
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
        pub(super) current_active: bool,
        pub(super) pending_children_update: bool,
        pub(super) current_views: HashMap<i64, View>,
        pub(super) view_states: HashMap<i64, ChildState<State>>,
        pub(super) my_path: Arc<[ViewId]>,
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

#[derive(Debug)]
struct UpdateVirtualChildren;

impl<State, Action, ChildrenViews, F, Element: Widget> ViewMarker
    for VirtualScroll<State, Action, ChildrenViews, F, Element>
{
}
impl<State, Action, Element: Widget, ChildrenViews, F> View<State, Action, ViewCtx>
    for VirtualScroll<State, Action, ChildrenViews, F, Element>
where
    State: 'static,
    Action: 'static,
    ChildrenViews: WidgetView<State, Action, Widget = Element>,
    F: Fn(&mut State, i64) -> ChildrenViews + 'static,
{
    type Element = Pod<widgets::VirtualScroll<Element>>;

    type ViewState = VirtualScrollState<ChildrenViews, ChildrenViews::ViewState>;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        // TODO: How does the anchor interact with Xilem?
        // Setting that seems like an imperative action?
        let widget = Pod::new(widgets::VirtualScroll::<Element>::new(0));
        ctx.record_action(widget.id);
        (widget, todo!())
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: xilem_core::Mut<'_, Self::Element>,
    ) {
        if ctx.state_changed() && !view_state.pending_children_update {
            let proxy = ctx.proxy();
            proxy.send_message(view_state.my_path.clone(), Box::new(UpdateVirtualChildren));
            view_state.pending_children_update = true;
            // Either rebuilding or not rebuilding would be fine here. We choose not to
            // because we know another rebuild is coming once this message is handled.
        }
        if !view_state.current_active {
            // If the current values are not active, we still need to rebuild
            // any children which have requested a rebuild.
            debug_assert!(view_state.pending_action.is_none(),);
            debug_assert_eq!(
                view_state.previous_views.len(),
                view_state.view_states.len(),
                "View states should be updated as views are removed"
            );
            for (idx, view) in &view_state.previous_views {
                let state = view_state
                    .view_states
                    .get_mut(idx)
                    .expect("All View states are accounted for");
                if state.requested_rebuild {
                    ctx.with_id(view_id_for_index(*idx), |ctx| {
                        // Note that we rebuild the view with itself, because we're only actioning a requested rebuild.
                        // This should be a no-op other than again the requested rebuild.
                        view.rebuild(
                            view,
                            &mut state.state,
                            ctx,
                            widgets::VirtualScroll::child_mut(&mut element, *idx),
                        );
                    });
                    state.requested_rebuild = false;
                }
            }
        } else {
            if let Some(pending_action) = view_state.pending_action.take() {
                debug_assert!(view_state.current_active);
                widgets::VirtualScroll::will_handle_action(&mut element, &pending_action);
            } else {
                // We should never change the number of children unless we're responding to an action
                debug_assert_eq!(
                    view_state.previous_views.len(),
                    view_state.current_views.len()
                );
            }
            let prev = &view_state.previous_views;
            // Apply the difference between the two sets of views
            // If we know they aren't the same, we can avoid one of these loops.

            std::mem::swap(
                &mut view_state.previous_views,
                &mut view_state.current_views,
            );
            // We have just "used" up the current states, so mark them as inactive
            view_state.current_active = false;
        }

        todo!()
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: xilem_core::Mut<'_, Self::Element>,
    ) {
        todo!()
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[xilem_core::ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> xilem_core::MessageResult<Action> {
        if let [first, tail @ ..] = id_path {
            // let result = self.children[idx].message(...);
            // if result.request_rebuild {...}
            todo!()
        }
        if message.as_any().is::<masonry::core::Action>() {
            let action = message.downcast::<masonry::core::Action>().unwrap();
            if let masonry::core::Action::Other(action) = *action {
                if !action.is::<VirtualScrollAction>() {
                    tracing::error!("Wrong action type in VirtualScroll::message: {action:?}");
                    // Ideally we'd avoid this extra box, but it's not easy to write this kind of code in a clean way
                    return MessageResult::Stale(Box::new(masonry::core::Action::Other(action)));
                }
                // We check then unwrap to avoid unwrapping a box (also, it makes the check path an early-exit)
                let action = action.downcast::<VirtualScrollAction>().unwrap();

                view_state.current_active = true;
                // TODO: Do we know that the current views are actually current? I'm not sure we do.
                view_state.current_views.clear();
                for new_targets in action.target.clone() {
                    view_state
                        .current_views
                        .insert(new_targets, (self.func)(app_state, new_targets));
                }
                view_state.pending_action = Some(*action);
                MessageResult::RequestRebuild
            } else {
                tracing::error!("Wrong action type in VirtualScroll::message: {action:?}");
                MessageResult::Stale(action)
            }
        } else if message.as_any().is::<UpdateVirtualChildren>() {
            std::mem::swap(
                &mut view_state.previous_views,
                &mut view_state.current_views,
            );
            view_state.current_active = true;
            view_state.current_views.clear();
            view_state.pending_children_update = false;
            for state in view_state.previous_views.keys() {
                view_state
                    .current_views
                    .insert(new_targets, (self.func)(app_state, new_targets));
            }
            MessageResult::RequestRebuild
        } else {
            tracing::error!("Wrong message type in VirtualScroll::message: {message:?}");
            MessageResult::Stale(message)
        }
    }
}
