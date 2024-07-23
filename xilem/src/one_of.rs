// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Statically typed alternatives to the type-erased [`AnyView`](`crate::AnyView`).

use accesskit::Role;
use masonry::{
    AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    PointerEvent, Size, StatusChange, TextEvent, Widget, WidgetId, WidgetPod,
};
use smallvec::{smallvec, SmallVec};
use vello::Scene;

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

    fn with_downcast_a(
        elem: &mut xilem_core::Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(xilem_core::Mut<'_, Pod<A>>),
    ) {
        match elem.widget {
            OneOfWidget::A(a) => f(elem.ctx.get_mut(a)),
            _ => unreachable!(),
        }
    }
    fn with_downcast_b(
        elem: &mut xilem_core::Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(xilem_core::Mut<'_, Pod<B>>),
    ) {
        match elem.widget {
            OneOfWidget::B(b) => f(elem.ctx.get_mut(b)),
            _ => unreachable!(),
        }
    }
    fn with_downcast_c(
        elem: &mut xilem_core::Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(xilem_core::Mut<'_, Pod<C>>),
    ) {
        match elem.widget {
            OneOfWidget::C(c) => f(elem.ctx.get_mut(c)),
            _ => unreachable!(),
        }
    }
    fn with_downcast_d(
        elem: &mut xilem_core::Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(xilem_core::Mut<'_, Pod<D>>),
    ) {
        match elem.widget {
            OneOfWidget::D(d) => f(elem.ctx.get_mut(d)),
            _ => unreachable!(),
        }
    }
    fn with_downcast_e(
        elem: &mut xilem_core::Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(xilem_core::Mut<'_, Pod<E>>),
    ) {
        match elem.widget {
            OneOfWidget::E(e) => f(elem.ctx.get_mut(e)),
            _ => unreachable!(),
        }
    }
    fn with_downcast_f(
        elem: &mut xilem_core::Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(xilem_core::Mut<'_, Pod<F>>),
    ) {
        match elem.widget {
            OneOfWidget::F(f_) => f(elem.ctx.get_mut(f_)),
            _ => unreachable!(),
        }
    }
    fn with_downcast_g(
        elem: &mut xilem_core::Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(xilem_core::Mut<'_, Pod<G>>),
    ) {
        match elem.widget {
            OneOfWidget::G(g) => f(elem.ctx.get_mut(g)),
            _ => unreachable!(),
        }
    }
    fn with_downcast_h(
        elem: &mut xilem_core::Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(xilem_core::Mut<'_, Pod<H>>),
    ) {
        match elem.widget {
            OneOfWidget::H(h) => f(elem.ctx.get_mut(h)),
            _ => unreachable!(),
        }
    }
    fn with_downcast_i(
        elem: &mut xilem_core::Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(xilem_core::Mut<'_, Pod<I>>),
    ) {
        match elem.widget {
            OneOfWidget::I(i) => f(elem.ctx.get_mut(i)),
            _ => unreachable!(),
        }
    }
    fn upcast_one_of_element(
        elem: xilem_core::one_of::OneOf9<
            Pod<A>,
            Pod<B>,
            Pod<C>,
            Pod<D>,
            Pod<E>,
            Pod<F>,
            Pod<G>,
            Pod<H>,
            Pod<I>,
        >,
    ) -> Self::OneOfElement {
        match elem {
            xilem_core::one_of::OneOf9::A(w) => Pod::new(OneOfWidget::A(w.inner)),
            xilem_core::one_of::OneOf9::B(w) => Pod::new(OneOfWidget::B(w.inner)),
            xilem_core::one_of::OneOf9::C(w) => Pod::new(OneOfWidget::C(w.inner)),
            xilem_core::one_of::OneOf9::D(w) => Pod::new(OneOfWidget::D(w.inner)),
            xilem_core::one_of::OneOf9::E(w) => Pod::new(OneOfWidget::E(w.inner)),
            xilem_core::one_of::OneOf9::F(w) => Pod::new(OneOfWidget::F(w.inner)),
            xilem_core::one_of::OneOf9::G(w) => Pod::new(OneOfWidget::G(w.inner)),
            xilem_core::one_of::OneOf9::H(w) => Pod::new(OneOfWidget::H(w.inner)),
            xilem_core::one_of::OneOf9::I(w) => Pod::new(OneOfWidget::I(w.inner)),
        }
    }

    fn update_one_of_element_mut(
        elem_mut: &mut xilem_core::Mut<'_, Self::OneOfElement>,
        new_elem: xilem_core::one_of::OneOf9<
            Pod<A>,
            Pod<B>,
            Pod<C>,
            Pod<D>,
            Pod<E>,
            Pod<F>,
            Pod<G>,
            Pod<H>,
            Pod<I>,
        >,
    ) {
        let new_inner = match new_elem {
            xilem_core::one_of::OneOf9::A(w) => OneOfWidget::A(w.inner),
            xilem_core::one_of::OneOf9::B(w) => OneOfWidget::B(w.inner),
            xilem_core::one_of::OneOf9::C(w) => OneOfWidget::C(w.inner),
            xilem_core::one_of::OneOf9::D(w) => OneOfWidget::D(w.inner),
            xilem_core::one_of::OneOf9::E(w) => OneOfWidget::E(w.inner),
            xilem_core::one_of::OneOf9::F(w) => OneOfWidget::F(w.inner),
            xilem_core::one_of::OneOf9::G(w) => OneOfWidget::G(w.inner),
            xilem_core::one_of::OneOf9::H(w) => OneOfWidget::H(w.inner),
            xilem_core::one_of::OneOf9::I(w) => OneOfWidget::I(w.inner),
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

#[allow(unnameable_types)] // Public because of trait visibility rules, but has no public API.
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
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        match self {
            OneOfWidget::A(w) => w.on_pointer_event(ctx, event),
            OneOfWidget::B(w) => w.on_pointer_event(ctx, event),
            OneOfWidget::C(w) => w.on_pointer_event(ctx, event),
            OneOfWidget::D(w) => w.on_pointer_event(ctx, event),
            OneOfWidget::E(w) => w.on_pointer_event(ctx, event),
            OneOfWidget::F(w) => w.on_pointer_event(ctx, event),
            OneOfWidget::G(w) => w.on_pointer_event(ctx, event),
            OneOfWidget::H(w) => w.on_pointer_event(ctx, event),
            OneOfWidget::I(w) => w.on_pointer_event(ctx, event),
        }
    }
    fn on_text_event(&mut self, ctx: &mut EventCtx, event: &TextEvent) {
        match self {
            OneOfWidget::A(w) => w.on_text_event(ctx, event),
            OneOfWidget::B(w) => w.on_text_event(ctx, event),
            OneOfWidget::C(w) => w.on_text_event(ctx, event),
            OneOfWidget::D(w) => w.on_text_event(ctx, event),
            OneOfWidget::E(w) => w.on_text_event(ctx, event),
            OneOfWidget::F(w) => w.on_text_event(ctx, event),
            OneOfWidget::G(w) => w.on_text_event(ctx, event),
            OneOfWidget::H(w) => w.on_text_event(ctx, event),
            OneOfWidget::I(w) => w.on_text_event(ctx, event),
        }
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx, event: &AccessEvent) {
        match self {
            OneOfWidget::A(w) => w.on_access_event(ctx, event),
            OneOfWidget::B(w) => w.on_access_event(ctx, event),
            OneOfWidget::C(w) => w.on_access_event(ctx, event),
            OneOfWidget::D(w) => w.on_access_event(ctx, event),
            OneOfWidget::E(w) => w.on_access_event(ctx, event),
            OneOfWidget::F(w) => w.on_access_event(ctx, event),
            OneOfWidget::G(w) => w.on_access_event(ctx, event),
            OneOfWidget::H(w) => w.on_access_event(ctx, event),
            OneOfWidget::I(w) => w.on_access_event(ctx, event),
        }
    }

    #[allow(missing_docs)]
    fn on_status_change(&mut self, _: &mut LifeCycleCtx, _: &StatusChange) {
        // Intentionally do nothing
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle) {
        match self {
            OneOfWidget::A(w) => w.lifecycle(ctx, event),
            OneOfWidget::B(w) => w.lifecycle(ctx, event),
            OneOfWidget::C(w) => w.lifecycle(ctx, event),
            OneOfWidget::D(w) => w.lifecycle(ctx, event),
            OneOfWidget::E(w) => w.lifecycle(ctx, event),
            OneOfWidget::F(w) => w.lifecycle(ctx, event),
            OneOfWidget::G(w) => w.lifecycle(ctx, event),
            OneOfWidget::H(w) => w.lifecycle(ctx, event),
            OneOfWidget::I(w) => w.lifecycle(ctx, event),
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        match self {
            OneOfWidget::A(w) => w.layout(ctx, bc),
            OneOfWidget::B(w) => w.layout(ctx, bc),
            OneOfWidget::C(w) => w.layout(ctx, bc),
            OneOfWidget::D(w) => w.layout(ctx, bc),
            OneOfWidget::E(w) => w.layout(ctx, bc),
            OneOfWidget::F(w) => w.layout(ctx, bc),
            OneOfWidget::G(w) => w.layout(ctx, bc),
            OneOfWidget::H(w) => w.layout(ctx, bc),
            OneOfWidget::I(w) => w.layout(ctx, bc),
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        match self {
            OneOfWidget::A(w) => w.paint(ctx, scene),
            OneOfWidget::B(w) => w.paint(ctx, scene),
            OneOfWidget::C(w) => w.paint(ctx, scene),
            OneOfWidget::D(w) => w.paint(ctx, scene),
            OneOfWidget::E(w) => w.paint(ctx, scene),
            OneOfWidget::F(w) => w.paint(ctx, scene),
            OneOfWidget::G(w) => w.paint(ctx, scene),
            OneOfWidget::H(w) => w.paint(ctx, scene),
            OneOfWidget::I(w) => w.paint(ctx, scene),
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, ctx: &mut AccessCtx) {
        match self {
            OneOfWidget::A(w) => w.accessibility(ctx),
            OneOfWidget::B(w) => w.accessibility(ctx),
            OneOfWidget::C(w) => w.accessibility(ctx),
            OneOfWidget::D(w) => w.accessibility(ctx),
            OneOfWidget::E(w) => w.accessibility(ctx),
            OneOfWidget::F(w) => w.accessibility(ctx),
            OneOfWidget::G(w) => w.accessibility(ctx),
            OneOfWidget::H(w) => w.accessibility(ctx),
            OneOfWidget::I(w) => w.accessibility(ctx),
        }
    }

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
