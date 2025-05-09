// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{Node, Role};
use masonry_winit::core::{
    AccessCtx, AccessEvent, BoxConstraints, EventCtx, FromDynWidget, LayoutCtx, PaintCtx,
    PointerEvent, PropertiesMut, PropertiesRef, QueryCtx, RegisterCtx, TextEvent, Widget, WidgetId,
    WidgetMut, WidgetPod,
};
use masonry_winit::kurbo::{Point, Size};
use smallvec::{SmallVec, smallvec};
use tracing::{Span, trace_span};
use vello::Scene;

use crate::core::{AnyElement, AnyView, Mut, SuperElement};
use crate::{Pod, ViewCtx};

/// A view which can have any underlying view type.
///
/// This can be used to return type erased views (such as from a trait),
/// or used to implement conditional display and switching of views.
///
/// Note that `Option` can also be used for conditionally displaying
/// views in a [`ViewSequence`](xilem_core::ViewSequence).
// TODO: Mention `Either` when we have implemented that?
pub type AnyWidgetView<State, Action = ()> =
    dyn AnyView<State, Action, ViewCtx, Pod<DynWidget>> + Send + Sync;

impl<W: Widget + FromDynWidget + ?Sized> SuperElement<Pod<W>, ViewCtx> for Pod<DynWidget> {
    fn upcast(ctx: &mut ViewCtx, child: Pod<W>) -> Self {
        ctx.new_pod(DynWidget {
            inner: child.erased_widget_pod(),
        })
    }

    fn with_downcast_val<R>(
        mut this: Self::Mut<'_>,
        f: impl FnOnce(Mut<Pod<W>>) -> R,
    ) -> (Self::Mut<'_>, R) {
        let ret = {
            let mut child = this.ctx.get_mut(&mut this.widget.inner);
            let downcast = child.downcast();
            f(downcast)
        };

        (this, ret)
    }
}

impl<W: Widget + FromDynWidget + ?Sized> AnyElement<Pod<W>, ViewCtx> for Pod<DynWidget> {
    fn replace_inner(mut this: Self::Mut<'_>, child: Pod<W>) -> Self::Mut<'_> {
        DynWidget::replace_inner(&mut this, child.erased_widget_pod());
        this
    }
}

/// A widget whose only child can be dynamically replaced.
#[allow(unnameable_types)] // This is an implementation detail of `AnyWidgetView`
pub struct DynWidget {
    inner: WidgetPod<dyn Widget>,
}

impl DynWidget {
    pub(crate) fn replace_inner(this: &mut WidgetMut<'_, Self>, widget: WidgetPod<dyn Widget>) {
        let old_widget = std::mem::replace(&mut this.widget.inner, widget);
        this.ctx.remove_child(old_widget);
    }
}

/// Forward all events to the child widget.
impl Widget for DynWidget {
    fn on_pointer_event(
        &mut self,
        _ctx: &mut EventCtx,
        _props: &mut PropertiesMut<'_>,
        _event: &PointerEvent,
    ) {
    }
    fn on_text_event(
        &mut self,
        _ctx: &mut EventCtx,
        _props: &mut PropertiesMut<'_>,
        _event: &TextEvent,
    ) {
    }
    fn on_access_event(
        &mut self,
        _ctx: &mut EventCtx,
        _props: &mut PropertiesMut<'_>,
        _event: &AccessEvent,
    ) {
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx) {
        ctx.register_child(&mut self.inner);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        let size = ctx.run_layout(&mut self.inner, bc);
        ctx.place_child(&mut self.inner, Point::ORIGIN);
        size
    }

    fn paint(&mut self, _ctx: &mut PaintCtx, _props: &PropertiesRef<'_>, _scene: &mut Scene) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        smallvec![self.inner.id()]
    }

    fn make_trace_span(&self, ctx: &QueryCtx<'_>) -> Span {
        trace_span!("DynWidget", id = ctx.widget_id().trace())
    }
}
