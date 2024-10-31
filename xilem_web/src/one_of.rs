// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    core::{
        one_of::{OneOf, OneOfCtx, PhantomElementCtx},
        Mut,
    },
    DomNode, Pod, PodFlags, PodMut, ViewCtx,
};
use wasm_bindgen::UnwrapThrowExt;

impl<N1, N2, N3, N4, N5, N6, N7, N8, N9>
    OneOfCtx<Pod<N1>, Pod<N2>, Pod<N3>, Pod<N4>, Pod<N5>, Pod<N6>, Pod<N7>, Pod<N8>, Pod<N9>>
    for ViewCtx
where
    N1: DomNode,
    N2: DomNode,
    N3: DomNode,
    N4: DomNode,
    N5: DomNode,
    N6: DomNode,
    N7: DomNode,
    N8: DomNode,
    N9: DomNode,
{
    type OneOfElement = Pod<OneOf<N1, N2, N3, N4, N5, N6, N7, N8, N9>>;

    fn upcast_one_of_element(
        &mut self,
        elem: OneOf<
            Pod<N1>,
            Pod<N2>,
            Pod<N3>,
            Pod<N4>,
            Pod<N5>,
            Pod<N6>,
            Pod<N7>,
            Pod<N8>,
            Pod<N9>,
        >,
    ) -> Self::OneOfElement {
        match elem {
            OneOf::A(e) => Pod::new(OneOf::A(e.node), OneOf::A(e.props), e.flags),
            OneOf::B(e) => Pod::new(OneOf::B(e.node), OneOf::B(e.props), e.flags),
            OneOf::C(e) => Pod::new(OneOf::C(e.node), OneOf::C(e.props), e.flags),
            OneOf::D(e) => Pod::new(OneOf::D(e.node), OneOf::D(e.props), e.flags),
            OneOf::E(e) => Pod::new(OneOf::E(e.node), OneOf::E(e.props), e.flags),
            OneOf::F(e) => Pod::new(OneOf::F(e.node), OneOf::F(e.props), e.flags),
            OneOf::G(e) => Pod::new(OneOf::G(e.node), OneOf::G(e.props), e.flags),
            OneOf::H(e) => Pod::new(OneOf::H(e.node), OneOf::H(e.props), e.flags),
            OneOf::I(e) => Pod::new(OneOf::I(e.node), OneOf::I(e.props), e.flags),
        }
    }

    fn update_one_of_element_mut(
        elem_mut: &mut Mut<Self::OneOfElement>,
        new_elem: OneOf<
            Pod<N1>,
            Pod<N2>,
            Pod<N3>,
            Pod<N4>,
            Pod<N5>,
            Pod<N6>,
            Pod<N7>,
            Pod<N8>,
            Pod<N9>,
        >,
    ) {
        let old_node: &web_sys::Node = elem_mut.node.as_ref();
        let new_node: &web_sys::Node = new_elem.as_ref();
        if old_node != new_node {
            if let Some(parent) = elem_mut.parent {
                parent.replace_child(new_node, old_node).unwrap_throw();
            }
        }
        (*elem_mut.node, *elem_mut.props, *elem_mut.flags) = match new_elem {
            OneOf::A(e) => (OneOf::A(e.node), OneOf::A(e.props), e.flags),
            OneOf::B(e) => (OneOf::B(e.node), OneOf::B(e.props), e.flags),
            OneOf::C(e) => (OneOf::C(e.node), OneOf::C(e.props), e.flags),
            OneOf::D(e) => (OneOf::D(e.node), OneOf::D(e.props), e.flags),
            OneOf::E(e) => (OneOf::E(e.node), OneOf::E(e.props), e.flags),
            OneOf::F(e) => (OneOf::F(e.node), OneOf::F(e.props), e.flags),
            OneOf::G(e) => (OneOf::G(e.node), OneOf::G(e.props), e.flags),
            OneOf::H(e) => (OneOf::H(e.node), OneOf::H(e.props), e.flags),
            OneOf::I(e) => (OneOf::I(e.node), OneOf::I(e.props), e.flags),
        };
    }

    fn with_downcast_a(e: &mut Mut<Self::OneOfElement>, f: impl FnOnce(Mut<Pod<N1>>)) {
        let (OneOf::A(node), OneOf::A(props)) = (&mut e.node, &mut e.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, e.flags, e.parent, e.was_removed));
    }

    fn with_downcast_b(e: &mut Mut<Self::OneOfElement>, f: impl FnOnce(Mut<Pod<N2>>)) {
        let (OneOf::B(node), OneOf::B(props)) = (&mut e.node, &mut e.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, e.flags, e.parent, e.was_removed));
    }

    fn with_downcast_c(e: &mut Mut<Self::OneOfElement>, f: impl FnOnce(Mut<Pod<N3>>)) {
        let (OneOf::C(node), OneOf::C(props)) = (&mut e.node, &mut e.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, e.flags, e.parent, e.was_removed));
    }

    fn with_downcast_d(e: &mut Mut<Self::OneOfElement>, f: impl FnOnce(Mut<Pod<N4>>)) {
        let (OneOf::D(node), OneOf::D(props)) = (&mut e.node, &mut e.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, e.flags, e.parent, e.was_removed));
    }

    fn with_downcast_e(e: &mut Mut<Self::OneOfElement>, f: impl FnOnce(Mut<Pod<N5>>)) {
        let (OneOf::E(node), OneOf::E(props)) = (&mut e.node, &mut e.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, e.flags, e.parent, e.was_removed));
    }

    fn with_downcast_f(e: &mut Mut<Self::OneOfElement>, f: impl FnOnce(Mut<Pod<N6>>)) {
        let (OneOf::F(node), OneOf::F(props)) = (&mut e.node, &mut e.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, e.flags, e.parent, e.was_removed));
    }

    fn with_downcast_g(e: &mut Mut<Self::OneOfElement>, f: impl FnOnce(Mut<Pod<N7>>)) {
        let (OneOf::G(node), OneOf::G(props)) = (&mut e.node, &mut e.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, e.flags, e.parent, e.was_removed));
    }

    fn with_downcast_h(e: &mut Mut<Self::OneOfElement>, f: impl FnOnce(Mut<Pod<N8>>)) {
        let (OneOf::H(node), OneOf::H(props)) = (&mut e.node, &mut e.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, e.flags, e.parent, e.was_removed));
    }

    fn with_downcast_i(e: &mut Mut<Self::OneOfElement>, f: impl FnOnce(Mut<Pod<N9>>)) {
        let (OneOf::I(node), OneOf::I(props)) = (&mut e.node, &mut e.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, e.flags, e.parent, e.was_removed));
    }
}

impl<N1, N2, N3, N4, N5, N6, N7, N8, N9> DomNode for OneOf<N1, N2, N3, N4, N5, N6, N7, N8, N9>
where
    N1: DomNode,
    N2: DomNode,
    N3: DomNode,
    N4: DomNode,
    N5: DomNode,
    N6: DomNode,
    N7: DomNode,
    N8: DomNode,
    N9: DomNode,
{
    type Props = OneOf<
        N1::Props,
        N2::Props,
        N3::Props,
        N4::Props,
        N5::Props,
        N6::Props,
        N7::Props,
        N8::Props,
        N9::Props,
    >;
    fn apply_props(&self, props: &mut Self::Props, flags: &mut PodFlags) {
        match (self, props) {
            (OneOf::A(el), OneOf::A(props)) => el.apply_props(props, flags),
            (OneOf::B(el), OneOf::B(props)) => el.apply_props(props, flags),
            (OneOf::C(el), OneOf::C(props)) => el.apply_props(props, flags),
            (OneOf::D(el), OneOf::D(props)) => el.apply_props(props, flags),
            (OneOf::E(el), OneOf::E(props)) => el.apply_props(props, flags),
            (OneOf::F(el), OneOf::F(props)) => el.apply_props(props, flags),
            (OneOf::G(el), OneOf::G(props)) => el.apply_props(props, flags),
            (OneOf::H(el), OneOf::H(props)) => el.apply_props(props, flags),
            (OneOf::I(el), OneOf::I(props)) => el.apply_props(props, flags),
            _ => unreachable!(),
        }
    }
}

#[allow(unnameable_types)] // reason: Implementation detail, public because of trait visibility rules
pub enum Noop {}

impl<T> AsRef<T> for Noop {
    fn as_ref(&self) -> &T {
        match *self {}
    }
}

impl<T> AsMut<T> for Noop {
    fn as_mut(&mut self) -> &mut T {
        match *self {}
    }
}

impl PhantomElementCtx for ViewCtx {
    type PhantomElement = Pod<Noop>;
}

impl DomNode for Noop {
    fn apply_props(&self, _props: &mut Self::Props, _: &mut PodFlags) {
        match *self {}
    }

    type Props = Noop;
}
