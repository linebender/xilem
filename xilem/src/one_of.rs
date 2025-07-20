// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Statically typed alternatives to the type-erased [`AnyWidgetView`](`crate::any_view::AnyWidgetView`).

use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, AccessEvent, BoxConstraints, ChildrenIds, EventCtx, FromDynWidget, LayoutCtx,
    NoAction, PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, TextEvent, Widget,
    WidgetPod,
};
use masonry::kurbo::{Point, Size};
use vello::Scene;

use crate::core::Mut;
use crate::core::one_of::OneOf;
use crate::{Pod, ViewCtx};

impl<
    A: Widget + FromDynWidget + ?Sized,
    B: Widget + FromDynWidget + ?Sized,
    C: Widget + FromDynWidget + ?Sized,
    D: Widget + FromDynWidget + ?Sized,
    E: Widget + FromDynWidget + ?Sized,
    F: Widget + FromDynWidget + ?Sized,
    G: Widget + FromDynWidget + ?Sized,
    H: Widget + FromDynWidget + ?Sized,
    I: Widget + FromDynWidget + ?Sized,
>
    crate::core::one_of::OneOfCtx<
        Pod<A>,
        Pod<B>,
        Pod<C>,
        Pod<D>,
        Pod<E>,
        Pod<F>,
        Pod<G>,
        Pod<H>,
        Pod<I>,
    > for ViewCtx
{
    type OneOfElement = Pod<OneOfWidget<A, B, C, D, E, F, G, H, I>>;

    fn with_downcast_a<R>(
        elem: &mut Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(Mut<'_, Pod<A>>) -> R,
    ) -> R {
        match elem.widget {
            OneOfWidget::A(a) => f(elem.ctx.get_mut(a)),
            _ => unreachable!(),
        }
    }
    fn with_downcast_b<R>(
        elem: &mut Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(Mut<'_, Pod<B>>) -> R,
    ) -> R {
        match elem.widget {
            OneOfWidget::B(b) => f(elem.ctx.get_mut(b)),
            _ => unreachable!(),
        }
    }
    fn with_downcast_c<R>(
        elem: &mut Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(Mut<'_, Pod<C>>) -> R,
    ) -> R {
        match elem.widget {
            OneOfWidget::C(c) => f(elem.ctx.get_mut(c)),
            _ => unreachable!(),
        }
    }
    fn with_downcast_d<R>(
        elem: &mut Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(Mut<'_, Pod<D>>) -> R,
    ) -> R {
        match elem.widget {
            OneOfWidget::D(d) => f(elem.ctx.get_mut(d)),
            _ => unreachable!(),
        }
    }
    fn with_downcast_e<R>(
        elem: &mut Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(Mut<'_, Pod<E>>) -> R,
    ) -> R {
        match elem.widget {
            OneOfWidget::E(e) => f(elem.ctx.get_mut(e)),
            _ => unreachable!(),
        }
    }
    fn with_downcast_f<R>(
        elem: &mut Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(Mut<'_, Pod<F>>) -> R,
    ) -> R {
        match elem.widget {
            OneOfWidget::F(f_) => f(elem.ctx.get_mut(f_)),
            _ => unreachable!(),
        }
    }
    fn with_downcast_g<R>(
        elem: &mut Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(Mut<'_, Pod<G>>) -> R,
    ) -> R {
        match elem.widget {
            OneOfWidget::G(g) => f(elem.ctx.get_mut(g)),
            _ => unreachable!(),
        }
    }
    fn with_downcast_h<R>(
        elem: &mut Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(Mut<'_, Pod<H>>) -> R,
    ) -> R {
        match elem.widget {
            OneOfWidget::H(h) => f(elem.ctx.get_mut(h)),
            _ => unreachable!(),
        }
    }
    fn with_downcast_i<R>(
        elem: &mut Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(Mut<'_, Pod<I>>) -> R,
    ) -> R {
        match elem.widget {
            OneOfWidget::I(i) => f(elem.ctx.get_mut(i)),
            _ => unreachable!(),
        }
    }
    fn upcast_one_of_element(
        &mut self,
        elem: OneOf<Pod<A>, Pod<B>, Pod<C>, Pod<D>, Pod<E>, Pod<F>, Pod<G>, Pod<H>, Pod<I>>,
    ) -> Self::OneOfElement {
        match elem {
            OneOf::A(w) => self.create_pod(OneOfWidget::A(w.new_widget.to_pod())),
            OneOf::B(w) => self.create_pod(OneOfWidget::B(w.new_widget.to_pod())),
            OneOf::C(w) => self.create_pod(OneOfWidget::C(w.new_widget.to_pod())),
            OneOf::D(w) => self.create_pod(OneOfWidget::D(w.new_widget.to_pod())),
            OneOf::E(w) => self.create_pod(OneOfWidget::E(w.new_widget.to_pod())),
            OneOf::F(w) => self.create_pod(OneOfWidget::F(w.new_widget.to_pod())),
            OneOf::G(w) => self.create_pod(OneOfWidget::G(w.new_widget.to_pod())),
            OneOf::H(w) => self.create_pod(OneOfWidget::H(w.new_widget.to_pod())),
            OneOf::I(w) => self.create_pod(OneOfWidget::I(w.new_widget.to_pod())),
        }
    }

    fn update_one_of_element_mut(
        elem_mut: &mut Mut<'_, Self::OneOfElement>,
        new_elem: OneOf<Pod<A>, Pod<B>, Pod<C>, Pod<D>, Pod<E>, Pod<F>, Pod<G>, Pod<H>, Pod<I>>,
    ) {
        let new_inner = match new_elem {
            OneOf::A(w) => OneOfWidget::A(w.new_widget.to_pod()),
            OneOf::B(w) => OneOfWidget::B(w.new_widget.to_pod()),
            OneOf::C(w) => OneOfWidget::C(w.new_widget.to_pod()),
            OneOf::D(w) => OneOfWidget::D(w.new_widget.to_pod()),
            OneOf::E(w) => OneOfWidget::E(w.new_widget.to_pod()),
            OneOf::F(w) => OneOfWidget::F(w.new_widget.to_pod()),
            OneOf::G(w) => OneOfWidget::G(w.new_widget.to_pod()),
            OneOf::H(w) => OneOfWidget::H(w.new_widget.to_pod()),
            OneOf::I(w) => OneOfWidget::I(w.new_widget.to_pod()),
        };
        let old_inner = std::mem::replace(elem_mut.widget, new_inner);
        match old_inner {
            OneOfWidget::A(w) => elem_mut.ctx.remove_child(w),
            OneOfWidget::B(w) => elem_mut.ctx.remove_child(w),
            OneOfWidget::C(w) => elem_mut.ctx.remove_child(w),
            OneOfWidget::D(w) => elem_mut.ctx.remove_child(w),
            OneOfWidget::E(w) => elem_mut.ctx.remove_child(w),
            OneOfWidget::F(w) => elem_mut.ctx.remove_child(w),
            OneOfWidget::G(w) => elem_mut.ctx.remove_child(w),
            OneOfWidget::H(w) => elem_mut.ctx.remove_child(w),
            OneOfWidget::I(w) => elem_mut.ctx.remove_child(w),
        }
        elem_mut.ctx.children_changed();
    }
}

impl crate::core::one_of::PhantomElementCtx for ViewCtx {
    type PhantomElement = Pod<dyn Widget>;
}

#[allow(
    unnameable_types,
    reason = "Implementation detail, public because of trait visibility rules"
)]
pub enum OneOfWidget<
    A: ?Sized,
    B: ?Sized,
    C: ?Sized,
    D: ?Sized,
    E: ?Sized,
    F: ?Sized,
    G: ?Sized,
    H: ?Sized,
    I: ?Sized,
> {
    A(WidgetPod<A>),
    B(WidgetPod<B>),
    C(WidgetPod<C>),
    D(WidgetPod<D>),
    E(WidgetPod<E>),
    F(WidgetPod<F>),
    G(WidgetPod<G>),
    H(WidgetPod<H>),
    I(WidgetPod<I>),
}

impl<
    A: Widget + FromDynWidget + ?Sized,
    B: Widget + FromDynWidget + ?Sized,
    C: Widget + FromDynWidget + ?Sized,
    D: Widget + FromDynWidget + ?Sized,
    E: Widget + FromDynWidget + ?Sized,
    F: Widget + FromDynWidget + ?Sized,
    G: Widget + FromDynWidget + ?Sized,
    H: Widget + FromDynWidget + ?Sized,
    I: Widget + FromDynWidget + ?Sized,
> Widget for OneOfWidget<A, B, C, D, E, F, G, H, I>
{
    type Action = NoAction;

    fn on_pointer_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &PointerEvent,
    ) {
    }
    fn on_text_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &TextEvent,
    ) {
    }
    fn on_access_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &AccessEvent,
    ) {
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        match self {
            Self::A(w) => ctx.register_child(w),
            Self::B(w) => ctx.register_child(w),
            Self::C(w) => ctx.register_child(w),
            Self::D(w) => ctx.register_child(w),
            Self::E(w) => ctx.register_child(w),
            Self::F(w) => ctx.register_child(w),
            Self::G(w) => ctx.register_child(w),
            Self::H(w) => ctx.register_child(w),
            Self::I(w) => ctx.register_child(w),
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        match self {
            Self::A(w) => {
                let size = ctx.run_layout(w, bc);
                ctx.place_child(w, Point::ORIGIN);
                size
            }
            Self::B(w) => {
                let size = ctx.run_layout(w, bc);
                ctx.place_child(w, Point::ORIGIN);
                size
            }
            Self::C(w) => {
                let size = ctx.run_layout(w, bc);
                ctx.place_child(w, Point::ORIGIN);
                size
            }
            Self::D(w) => {
                let size = ctx.run_layout(w, bc);
                ctx.place_child(w, Point::ORIGIN);
                size
            }
            Self::E(w) => {
                let size = ctx.run_layout(w, bc);
                ctx.place_child(w, Point::ORIGIN);
                size
            }
            Self::F(w) => {
                let size = ctx.run_layout(w, bc);
                ctx.place_child(w, Point::ORIGIN);
                size
            }
            Self::G(w) => {
                let size = ctx.run_layout(w, bc);
                ctx.place_child(w, Point::ORIGIN);
                size
            }
            Self::H(w) => {
                let size = ctx.run_layout(w, bc);
                ctx.place_child(w, Point::ORIGIN);
                size
            }
            Self::I(w) => {
                let size = ctx.run_layout(w, bc);
                ctx.place_child(w, Point::ORIGIN);
                size
            }
        }
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _scene: &mut Scene) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        match self {
            Self::A(w) => ChildrenIds::from_slice(&[w.id()]),
            Self::B(w) => ChildrenIds::from_slice(&[w.id()]),
            Self::C(w) => ChildrenIds::from_slice(&[w.id()]),
            Self::D(w) => ChildrenIds::from_slice(&[w.id()]),
            Self::E(w) => ChildrenIds::from_slice(&[w.id()]),
            Self::F(w) => ChildrenIds::from_slice(&[w.id()]),
            Self::G(w) => ChildrenIds::from_slice(&[w.id()]),
            Self::H(w) => ChildrenIds::from_slice(&[w.id()]),
            Self::I(w) => ChildrenIds::from_slice(&[w.id()]),
        }
    }
}
