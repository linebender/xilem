// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Statically typed alternatives to the type-erased [`AnyView`](`crate::AnyView`).

use accesskit::{NodeBuilder, Role};
use masonry::{
    AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, PaintCtx, Point, PointerEvent,
    RegisterCtx, Size, TextEvent, Widget, WidgetId, WidgetPod,
};
use smallvec::{smallvec, SmallVec};
use vello::Scene;

use crate::core::one_of::OneOf;
use crate::core::Mut;
use crate::{Pod, ViewCtx};

impl<
        A: Widget,
        B: Widget,
        C: Widget,
        D: Widget,
        E: Widget,
        F: Widget,
        G: Widget,
        H: Widget,
        I: Widget,
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

    fn with_downcast_a(elem: &mut Mut<Self::OneOfElement>, f: impl FnOnce(Mut<Pod<A>>)) {
        match elem.widget {
            OneOfWidget::A(a) => f(elem.ctx.get_mut(a)),
            _ => unreachable!(),
        }
    }
    fn with_downcast_b(elem: &mut Mut<Self::OneOfElement>, f: impl FnOnce(Mut<Pod<B>>)) {
        match elem.widget {
            OneOfWidget::B(b) => f(elem.ctx.get_mut(b)),
            _ => unreachable!(),
        }
    }
    fn with_downcast_c(elem: &mut Mut<Self::OneOfElement>, f: impl FnOnce(Mut<Pod<C>>)) {
        match elem.widget {
            OneOfWidget::C(c) => f(elem.ctx.get_mut(c)),
            _ => unreachable!(),
        }
    }
    fn with_downcast_d(elem: &mut Mut<Self::OneOfElement>, f: impl FnOnce(Mut<Pod<D>>)) {
        match elem.widget {
            OneOfWidget::D(d) => f(elem.ctx.get_mut(d)),
            _ => unreachable!(),
        }
    }
    fn with_downcast_e(elem: &mut Mut<Self::OneOfElement>, f: impl FnOnce(Mut<Pod<E>>)) {
        match elem.widget {
            OneOfWidget::E(e) => f(elem.ctx.get_mut(e)),
            _ => unreachable!(),
        }
    }
    fn with_downcast_f(elem: &mut Mut<Self::OneOfElement>, f: impl FnOnce(Mut<Pod<F>>)) {
        match elem.widget {
            OneOfWidget::F(f_) => f(elem.ctx.get_mut(f_)),
            _ => unreachable!(),
        }
    }
    fn with_downcast_g(elem: &mut Mut<Self::OneOfElement>, f: impl FnOnce(Mut<Pod<G>>)) {
        match elem.widget {
            OneOfWidget::G(g) => f(elem.ctx.get_mut(g)),
            _ => unreachable!(),
        }
    }
    fn with_downcast_h(elem: &mut Mut<Self::OneOfElement>, f: impl FnOnce(Mut<Pod<H>>)) {
        match elem.widget {
            OneOfWidget::H(h) => f(elem.ctx.get_mut(h)),
            _ => unreachable!(),
        }
    }
    fn with_downcast_i(elem: &mut Mut<Self::OneOfElement>, f: impl FnOnce(Mut<Pod<I>>)) {
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
            OneOf::A(w) => self.new_pod(OneOfWidget::A(w.inner)),
            OneOf::B(w) => self.new_pod(OneOfWidget::B(w.inner)),
            OneOf::C(w) => self.new_pod(OneOfWidget::C(w.inner)),
            OneOf::D(w) => self.new_pod(OneOfWidget::D(w.inner)),
            OneOf::E(w) => self.new_pod(OneOfWidget::E(w.inner)),
            OneOf::F(w) => self.new_pod(OneOfWidget::F(w.inner)),
            OneOf::G(w) => self.new_pod(OneOfWidget::G(w.inner)),
            OneOf::H(w) => self.new_pod(OneOfWidget::H(w.inner)),
            OneOf::I(w) => self.new_pod(OneOfWidget::I(w.inner)),
        }
    }

    fn update_one_of_element_mut(
        elem_mut: &mut Mut<Self::OneOfElement>,
        new_elem: OneOf<Pod<A>, Pod<B>, Pod<C>, Pod<D>, Pod<E>, Pod<F>, Pod<G>, Pod<H>, Pod<I>>,
    ) {
        let new_inner = match new_elem {
            OneOf::A(w) => OneOfWidget::A(w.inner),
            OneOf::B(w) => OneOfWidget::B(w.inner),
            OneOf::C(w) => OneOfWidget::C(w.inner),
            OneOf::D(w) => OneOfWidget::D(w.inner),
            OneOf::E(w) => OneOfWidget::E(w.inner),
            OneOf::F(w) => OneOfWidget::F(w.inner),
            OneOf::G(w) => OneOfWidget::G(w.inner),
            OneOf::H(w) => OneOfWidget::H(w.inner),
            OneOf::I(w) => OneOfWidget::I(w.inner),
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
    type PhantomElement = Pod<Box<dyn Widget>>;
}

#[allow(unnameable_types)] // reason: Implementation detail, public because of trait visibility rules
pub enum OneOfWidget<A, B, C, D, E, F, G, H, I> {
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
        A: Widget,
        B: Widget,
        C: Widget,
        D: Widget,
        E: Widget,
        F: Widget,
        G: Widget,
        H: Widget,
        I: Widget,
    > Widget for OneOfWidget<A, B, C, D, E, F, G, H, I>
{
    fn on_pointer_event(&mut self, _ctx: &mut EventCtx, _event: &PointerEvent) {}
    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}
    fn on_access_event(&mut self, _ctx: &mut EventCtx, _event: &AccessEvent) {}

    fn register_children(&mut self, ctx: &mut RegisterCtx) {
        match self {
            OneOfWidget::A(w) => ctx.register_child(w),
            OneOfWidget::B(w) => ctx.register_child(w),
            OneOfWidget::C(w) => ctx.register_child(w),
            OneOfWidget::D(w) => ctx.register_child(w),
            OneOfWidget::E(w) => ctx.register_child(w),
            OneOfWidget::F(w) => ctx.register_child(w),
            OneOfWidget::G(w) => ctx.register_child(w),
            OneOfWidget::H(w) => ctx.register_child(w),
            OneOfWidget::I(w) => ctx.register_child(w),
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        match self {
            OneOfWidget::A(w) => {
                let size = ctx.run_layout(w, bc);
                ctx.place_child(w, Point::ORIGIN);
                size
            }
            OneOfWidget::B(w) => {
                let size = ctx.run_layout(w, bc);
                ctx.place_child(w, Point::ORIGIN);
                size
            }
            OneOfWidget::C(w) => {
                let size = ctx.run_layout(w, bc);
                ctx.place_child(w, Point::ORIGIN);
                size
            }
            OneOfWidget::D(w) => {
                let size = ctx.run_layout(w, bc);
                ctx.place_child(w, Point::ORIGIN);
                size
            }
            OneOfWidget::E(w) => {
                let size = ctx.run_layout(w, bc);
                ctx.place_child(w, Point::ORIGIN);
                size
            }
            OneOfWidget::F(w) => {
                let size = ctx.run_layout(w, bc);
                ctx.place_child(w, Point::ORIGIN);
                size
            }
            OneOfWidget::G(w) => {
                let size = ctx.run_layout(w, bc);
                ctx.place_child(w, Point::ORIGIN);
                size
            }
            OneOfWidget::H(w) => {
                let size = ctx.run_layout(w, bc);
                ctx.place_child(w, Point::ORIGIN);
                size
            }
            OneOfWidget::I(w) => {
                let size = ctx.run_layout(w, bc);
                ctx.place_child(w, Point::ORIGIN);
                size
            }
        }
    }

    fn paint(&mut self, _ctx: &mut PaintCtx, _scene: &mut Scene) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx, _node: &mut NodeBuilder) {}

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        match self {
            OneOfWidget::A(w) => smallvec![w.id()],
            OneOfWidget::B(w) => smallvec![w.id()],
            OneOfWidget::C(w) => smallvec![w.id()],
            OneOfWidget::D(w) => smallvec![w.id()],
            OneOfWidget::E(w) => smallvec![w.id()],
            OneOfWidget::F(w) => smallvec![w.id()],
            OneOfWidget::G(w) => smallvec![w.id()],
            OneOfWidget::H(w) => smallvec![w.id()],
            OneOfWidget::I(w) => smallvec![w.id()],
        }
    }
}
