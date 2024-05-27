// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::Role;
use masonry::widget::{WidgetMut, WidgetRef};
use masonry::{
    AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    Point, PointerEvent, Size, StatusChange, TextEvent, Widget, WidgetPod,
};
use smallvec::SmallVec;
use vello::Scene;
use xilem_core::{AnyElement, AnyView, SuperElement};

use crate::{Pod, ViewCtx};

/// A view which can have any underlying view type.
///
/// This can be used to return type erased views (such as from a trait),
/// or used to implement conditional display and switching of views.
///
/// Note that `Option` can also be used for conditionally displaying
/// views in a [`ViewSequence`](crate::ViewSequence).
// TODO: Mention `Either` when we have implemented that?
pub type AnyWidgetView<State, Action = ()> =
    dyn AnyView<State, Action, ViewCtx, Pod<DynWidget>> + Send + Sync;

impl<W: Widget> SuperElement<Pod<W>> for Pod<Box<dyn Widget>> {
    fn upcast(child: Pod<W>) -> Self {
        child.inner.boxed().into()
    }

    fn with_downcast_val<R>(
        mut this: Self::Mut<'_>,
        f: impl FnOnce(<Pod<W> as xilem_core::ViewElement>::Mut<'_>) -> R,
    ) -> (Self::Mut<'_>, R) {
        let downcast = this.downcast();
        let ret = f(downcast);
        (this, ret)
    }
}

impl<W: Widget> SuperElement<Pod<W>> for Pod<DynWidget> {
    fn upcast(child: Pod<W>) -> Self {
        WidgetPod::new(DynWidget {
            inner: child.inner.boxed(),
        })
        .into()
    }

    fn with_downcast_val<R>(
        mut this: Self::Mut<'_>,
        f: impl FnOnce(<Pod<W> as xilem_core::ViewElement>::Mut<'_>) -> R,
    ) -> (Self::Mut<'_>, R) {
        let ret = {
            let mut child = this.ctx.get_mut(&mut this.widget.inner);
            let downcast = child.downcast();
            f(downcast)
        };

        (this, ret)
    }
}

impl<W: Widget> AnyElement<Pod<W>> for Pod<DynWidget> {
    fn replace_inner(mut this: Self::Mut<'_>, child: Pod<W>) -> Self::Mut<'_> {
        DynWidget::replace_inner(&mut this, child.inner.boxed());
        this
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
