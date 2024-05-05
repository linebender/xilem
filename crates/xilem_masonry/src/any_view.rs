// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::{any::Any, ops::Deref};

use accesskit::Role;
use masonry::widget::{WidgetMut, WidgetRef};
use masonry::{
    AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    Point, PointerEvent, Size, StatusChange, TextEvent, Widget, WidgetPod,
};
use smallvec::SmallVec;
use vello::Scene;

use crate::{MasonryView, MessageResult, ViewCx, ViewId};

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
    type ViewState = AnyViewState;

    fn build(&self, cx: &mut ViewCx) -> (masonry::WidgetPod<Self::Element>, Self::ViewState) {
        self.deref().dyn_build(cx)
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> crate::MessageResult<A> {
        self.deref()
            .dyn_message(view_state, id_path, message, app_state)
    }

    fn rebuild(
        &self,
        view_state: &mut Self::ViewState,
        cx: &mut ViewCx,
        prev: &Self,
        element: masonry::widget::WidgetMut<Self::Element>,
    ) {
        self.deref()
            .dyn_rebuild(view_state, cx, prev.deref(), element);
    }
}

pub struct AnyViewState {
    inner_state: Box<dyn Any>,
    generation: u64,
}

/// A trait enabling type erasure of views.
pub trait AnyMasonryView<T, A = ()>: Send {
    fn as_any(&self) -> &dyn std::any::Any;

    fn dyn_build(&self, cx: &mut ViewCx) -> (WidgetPod<DynWidget>, AnyViewState);

    fn dyn_rebuild(
        &self,
        dyn_state: &mut AnyViewState,
        cx: &mut ViewCx,
        prev: &dyn AnyMasonryView<T, A>,
        element: WidgetMut<DynWidget>,
    );

    fn dyn_message(
        &self,
        dyn_state: &mut AnyViewState,
        id_path: &[ViewId],
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> MessageResult<A>;
}

impl<T, A, V: MasonryView<T, A> + 'static> AnyMasonryView<T, A> for V
where
    V::ViewState: Any,
{
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn dyn_build(&self, cx: &mut ViewCx) -> (masonry::WidgetPod<DynWidget>, AnyViewState) {
        let generation = 0;
        let (element, view_state) =
            cx.with_id(ViewId::for_type::<V>(generation), |cx| self.build(cx));
        (
            WidgetPod::new(DynWidget {
                inner: element.boxed(),
            }),
            AnyViewState {
                inner_state: Box::new(view_state),
                generation,
            },
        )
    }

    fn dyn_rebuild(
        &self,
        dyn_state: &mut AnyViewState,
        cx: &mut ViewCx,
        prev: &dyn AnyMasonryView<T, A>,
        mut element: WidgetMut<DynWidget>,
    ) {
        if let Some(prev) = prev.as_any().downcast_ref() {
            // If we were previously of this type, then do a normal rebuild
            DynWidget::downcast(&mut element, |element| {
                if let Some(element) = element {
                    if let Some(state) = dyn_state.inner_state.downcast_mut() {
                        cx.with_id(ViewId::for_type::<V>(dyn_state.generation), move |cx| {
                            self.rebuild(state, cx, prev, element);
                        });
                    } else {
                        tracing::error!("Unexpected element state type");
                    }
                } else {
                    eprintln!("downcast of element failed in dyn_rebuild");
                }
            });
        } else {
            // Otherwise, replace the element.

            // Increase the generation, because the underlying widget has been swapped out.
            // Overflow condition: Impossible to overflow, as u64 only ever incremented by 1
            // and starting at 0.
            dyn_state.generation = dyn_state.generation.wrapping_add(1);
            let (new_element, view_state) = cx
                .with_id(ViewId::for_type::<V>(dyn_state.generation), |cx| {
                    self.build(cx)
                });
            dyn_state.inner_state = Box::new(view_state);
            DynWidget::replace_inner(&mut element, new_element.boxed());
            cx.mark_changed();
        }
    }

    fn dyn_message(
        &self,
        dyn_state: &mut AnyViewState,
        id_path: &[ViewId],
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        let (start, rest) = id_path
            .split_first()
            .expect("Id path has elements for AnyView");
        if start.routing_id() != dyn_state.generation {
            return MessageResult::Stale(message);
        }
        if let Some(view_state) = dyn_state.inner_state.downcast_mut() {
            self.message(view_state, rest, message, app_state)
        } else {
            // Possibly softer failure?
            panic!("downcast error in dyn_message");
        }
    }
}

/// A widget whose only child can be dynamically replaced.
///
/// `WidgetPod<Box<dyn Widget>>` doesn't expose this possibility.
pub struct DynWidget {
    inner: WidgetPod<Box<dyn Widget>>,
}

impl DynWidget {
    pub(crate) fn replace_inner(
        this: &mut WidgetMut<'_, Self>,
        widget: WidgetPod<Box<dyn Widget>>,
    ) {
        this.widget.inner = widget;
        this.ctx.children_changed();
    }

    pub(crate) fn downcast<W: Widget, R>(
        this: &mut WidgetMut<'_, Self>,
        f: impl FnOnce(Option<WidgetMut<'_, W>>) -> R,
    ) -> R {
        let mut get_mut = this.ctx.get_mut(&mut this.widget.inner);
        f(get_mut.try_downcast())
    }
}

/// Forward all events to the child widget.
impl Widget for DynWidget {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        self.inner.on_pointer_event(ctx, event);
    }
    fn on_text_event(&mut self, ctx: &mut EventCtx, event: &TextEvent) {
        self.inner.on_text_event(ctx, event);
    }
    fn on_access_event(&mut self, ctx: &mut EventCtx, event: &AccessEvent) {
        self.inner.on_access_event(ctx, event);
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

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, ctx: &mut AccessCtx) {
        self.inner.accessibility(ctx);
    }

    fn children(&self) -> SmallVec<[WidgetRef<'_, dyn Widget>; 16]> {
        let mut vec = SmallVec::new();
        vec.push(self.inner.as_dyn());
        vec
    }
}
