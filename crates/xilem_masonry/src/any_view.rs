use std::{num::NonZeroU64, ops::Deref};

use masonry::{
    declare_widget,
    widget::{StoreInWidgetMut, WidgetMut, WidgetRef},
    BoxConstraints, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Point, PointerEvent,
    Size, StatusChange, TextEvent, Widget, WidgetPod,
};
use smallvec::SmallVec;
use vello::Scene;

use crate::{ChangeFlags, MasonryView, MessageResult, ViewCx, ViewId};

/// A view which can have any underlying view type.
///
/// This can be used to return type erased views (such as from a trait),
/// or used to implement conditional display and switching of views.
///
/// Note that `Option` can also be used for conditionally displaying
/// views in a [`ViewSequence`](crate::ViewSequence).
// TODO: Mention `Either` when we have implemented that?
pub type BoxedMasonryView<AppState, Action = ()> = Box<dyn AnyMasonryView<AppState, Action>>;

impl<T: 'static, A: 'static> MasonryView<T, A> for BoxedMasonryView<T, A> {
    type State = Box<dyn std::any::Any>;
    type Element = DynWidget;

    fn build(&self, cx: &mut ViewCx) -> (ViewId, Self::State, masonry::WidgetPod<Self::Element>) {
        self.deref().dyn_build(cx)
    }

    fn rebuild(
        &self,
        cx: &mut ViewCx,
        prev: &Self,
        id: &mut ViewId,
        state: &mut Self::State,
        element: masonry::widget::WidgetMut<Self::Element>,
    ) -> ChangeFlags {
        self.deref().dyn_rebuild(cx, prev, id, state, element)
    }

    fn message(
        &self,
        id_path: &[ViewId],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> crate::MessageResult<A> {
        self.deref().dyn_message(id_path, state, message, app_state)
    }
}

/// A trait enabling type erasure of views.
pub trait AnyMasonryView<AppState, Action = ()>: Send {
    fn as_any(&self) -> &dyn std::any::Any;

    fn dyn_build(&self, cx: &mut ViewCx) -> (ViewId, Box<dyn std::any::Any>, WidgetPod<DynWidget>);

    fn dyn_rebuild(
        &self,
        cx: &mut ViewCx,
        prev: &dyn AnyMasonryView<AppState, Action>,
        id: &mut ViewId,
        state: &mut Box<dyn std::any::Any>,
        element: WidgetMut<DynWidget>,
    ) -> ChangeFlags;

    fn dyn_message(
        &self,
        id_path: &[ViewId],
        state: &mut Box<dyn std::any::Any>,
        message: Box<dyn std::any::Any>,
        app_state: &mut AppState,
    ) -> MessageResult<Action>;
}

impl<AppState, Action, View> AnyMasonryView<AppState, Action> for View
where
    View: MasonryView<AppState, Action> + 'static,
    View::State: 'static,
{
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn dyn_build(&self, cx: &mut ViewCx) -> (ViewId, Box<dyn std::any::Any>, WidgetPod<DynWidget>) {
        let (id, state, element) = self.build(cx);
        let pod = WidgetPod::new(DynWidget {
            inner: element.boxed(),
            generation: NonZeroU64::new(1).unwrap(),
        });

        (id, Box::new(state), pod)
    }

    fn dyn_rebuild(
        &self,
        cx: &mut ViewCx,
        prev: &dyn AnyMasonryView<AppState, Action>,
        id: &mut ViewId,
        state: &mut Box<dyn std::any::Any>,
        mut element: WidgetMut<DynWidget>,
    ) -> ChangeFlags {
        // TODO: Does this need to have a custom view id to enable events sent
        // to an outdated view path to be caught and returned?
        // Should we store this generation in `element`? Seems plausible
        if let Some(prev) = prev.as_any().downcast_ref() {
            if let Some(state) = state.downcast_mut() {
                // If we were previously of this type, then do a normal rebuild
                element.downcast(|element| {
                    if let Some(element) = element {
                        self.rebuild(cx, prev, id, state, element)
                    } else {
                        tracing::error!("downcast of state failed in dyn_rebuild");
                        ChangeFlags::UNCHANGED
                    }
                })
            } else {
                tracing::error!("downcast of element failed in dyn_rebuild");
                ChangeFlags::UNCHANGED
            }
        } else {
            let (new_id, new_state, new_element) = self.build(cx);
            *id = new_id;
            *state = Box::new(new_state);
            element.replace_inner(new_element.boxed());
            ChangeFlags::CHANGED // Tree structure
        }
    }

    fn dyn_message(
        &self,
        id_path: &[ViewId],
        state: &mut Box<dyn std::any::Any>,
        message: Box<dyn std::any::Any>,
        app_state: &mut AppState,
    ) -> MessageResult<Action> {
        if let Some(state) = state.downcast_mut() {
            self.message(id_path, state, message, app_state)
        } else {
            tracing::error!("downcast of state failed in dyn_message");
            MessageResult::Stale(message)
        }
    }
}

/// A widget whose only child can be dynamically replaced.
///
/// `WidgetPod<Box<dyn Widget>>` doesn't expose this possibility.
pub struct DynWidget {
    inner: WidgetPod<Box<dyn Widget>>,
    // This might be a layer break?
    /// The generation of the inner widget, increases whenever the contained widget is replaced
    generation: NonZeroU64,
}

/// Forward all events to the child widget.
impl Widget for DynWidget {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        self.inner.on_pointer_event(ctx, event);
    }
    fn on_text_event(&mut self, ctx: &mut EventCtx, event: &TextEvent) {
        self.inner.on_text_event(ctx, event);
    }

    fn on_status_change(&mut self, _: &mut LifeCycleCtx, _: &StatusChange) {
        // Intentionally do nothing
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle) {
        self.inner.lifecycle(ctx, event);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        let size = self.inner.layout(ctx, bc);
        ctx.place_child(&mut self.inner, Point::ORIGIN);
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        self.inner.paint(ctx, scene);
    }

    fn children(&self) -> SmallVec<[WidgetRef<'_, dyn Widget>; 16]> {
        let mut vec = SmallVec::new();
        vec.push(self.inner.as_dyn());
        vec
    }
}

declare_widget!(DynWidgetMut, DynWidget);

impl DynWidget {
    pub fn generation(&self) -> NonZeroU64 {
        self.generation
    }

    pub fn next_generation(&self) -> NonZeroU64 {
        self.generation.checked_add(1).unwrap()
    }
}

impl DynWidgetMut<'_> {
    pub(crate) fn replace_inner(&mut self, widget: WidgetPod<Box<dyn Widget>>) {
        self.widget.generation = self.next_generation();
        self.widget.inner = widget;
        self.ctx.children_changed();
    }

    pub(crate) fn downcast<W: Widget + StoreInWidgetMut, R>(
        &mut self,
        f: impl FnOnce(Option<WidgetMut<'_, W>>) -> R,
    ) -> R {
        let mut get_mut = self.ctx.get_mut(&mut self.widget.inner);
        f(get_mut.downcast())
    }
}
