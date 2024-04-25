use std::ops::Deref;

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
pub type BoxedMasonryView<T, A = ()> = Box<dyn AnyMasonryView<T, A>>;

impl<T: 'static, A: 'static> MasonryView<T, A> for BoxedMasonryView<T, A> {
    type Element = DynWidget;

    fn build(&self, cx: &mut ViewCx) -> masonry::WidgetPod<Self::Element> {
        self.deref().dyn_build(cx)
    }

    fn message(
        &self,
        id_path: &[ViewId],
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> crate::MessageResult<A> {
        self.deref().dyn_message(id_path, message, app_state)
    }

    fn rebuild(
        &self,
        cx: &mut ViewCx,
        prev: &Self,
        // _id: &mut Id,
        element: masonry::widget::WidgetMut<Self::Element>,
    ) -> ChangeFlags {
        self.deref().dyn_rebuild(cx, prev, element)
    }
}

/// A trait enabling type erasure of views.
pub trait AnyMasonryView<T, A = ()>: Send {
    fn as_any(&self) -> &dyn std::any::Any;

    fn dyn_build(&self, cx: &mut ViewCx) -> WidgetPod<DynWidget>;

    fn dyn_rebuild(
        &self,
        cx: &mut ViewCx,
        prev: &dyn AnyMasonryView<T, A>,
        element: WidgetMut<DynWidget>,
    ) -> ChangeFlags;

    fn dyn_message(
        &self,
        id_path: &[ViewId],
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> MessageResult<A>;
}

impl<T, A, V: MasonryView<T, A> + 'static> AnyMasonryView<T, A> for V {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn dyn_build(&self, cx: &mut ViewCx) -> WidgetPod<DynWidget> {
        let element = self.build(cx);
        WidgetPod::new(DynWidget {
            inner: element.boxed(),
        })
    }

    fn dyn_rebuild(
        &self,
        cx: &mut ViewCx,
        prev: &dyn AnyMasonryView<T, A>,
        mut element: WidgetMut<DynWidget>,
    ) -> ChangeFlags {
        // TODO: Does this need to have a custom view id to enable events sent
        // to an outdated view path to be caught and returned?
        // Should we store this generation in `element`? Seems plausible
        if let Some(prev) = prev.as_any().downcast_ref() {
            // If we were previously of this type, then do a normal rebuild
            element.downcast(|element| {
                if let Some(element) = element {
                    self.rebuild(cx, prev, element)
                } else {
                    eprintln!("downcast of element failed in dyn_rebuild");
                    ChangeFlags::UNCHANGED
                }
            })
        } else {
            // Otherwise, replace the element
            let new_element = self.build(cx);
            element.replace_inner(new_element.boxed());
            ChangeFlags::CHANGED
        }
    }

    fn dyn_message(
        &self,
        id_path: &[ViewId],
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        self.message(id_path, message, app_state)
    }
}

/// A widget whose only child can be dynamically replaced.
///
/// `WidgetPod<Box<dyn Widget>>` doesn't expose this possibility.
pub struct DynWidget {
    inner: WidgetPod<Box<dyn Widget>>,
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

impl DynWidgetMut<'_> {
    pub(crate) fn replace_inner(&mut self, widget: WidgetPod<Box<dyn Widget>>) {
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
