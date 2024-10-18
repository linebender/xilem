// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    core::{
        one_of::{OneOf, OneOfCtx, PhantomElementCtx},
        Mut,
    },
    DomNode, Pod, PodMut, ViewCtx, With,
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
            OneOf::A(e) => Pod {
                node: OneOf::A(e.node),
                props: OneOf::A(e.props),
            },
            OneOf::B(e) => Pod {
                node: OneOf::B(e.node),
                props: OneOf::B(e.props),
            },
            OneOf::C(e) => Pod {
                node: OneOf::C(e.node),
                props: OneOf::C(e.props),
            },
            OneOf::D(e) => Pod {
                node: OneOf::D(e.node),
                props: OneOf::D(e.props),
            },
            OneOf::E(e) => Pod {
                node: OneOf::E(e.node),
                props: OneOf::E(e.props),
            },
            OneOf::F(e) => Pod {
                node: OneOf::F(e.node),
                props: OneOf::F(e.props),
            },
            OneOf::G(e) => Pod {
                node: OneOf::G(e.node),
                props: OneOf::G(e.props),
            },
            OneOf::H(e) => Pod {
                node: OneOf::H(e.node),
                props: OneOf::H(e.props),
            },
            OneOf::I(e) => Pod {
                node: OneOf::I(e.node),
                props: OneOf::I(e.props),
            },
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
        (*elem_mut.node, *elem_mut.props) = match new_elem {
            OneOf::A(e) => (OneOf::A(e.node), OneOf::A(e.props)),
            OneOf::B(e) => (OneOf::B(e.node), OneOf::B(e.props)),
            OneOf::C(e) => (OneOf::C(e.node), OneOf::C(e.props)),
            OneOf::D(e) => (OneOf::D(e.node), OneOf::D(e.props)),
            OneOf::E(e) => (OneOf::E(e.node), OneOf::E(e.props)),
            OneOf::F(e) => (OneOf::F(e.node), OneOf::F(e.props)),
            OneOf::G(e) => (OneOf::G(e.node), OneOf::G(e.props)),
            OneOf::H(e) => (OneOf::H(e.node), OneOf::H(e.props)),
            OneOf::I(e) => (OneOf::I(e.node), OneOf::I(e.props)),
        };
    }

    fn with_downcast_a(elem: &mut Mut<Self::OneOfElement>, f: impl FnOnce(Mut<Pod<N1>>)) {
        let (OneOf::A(node), OneOf::A(props)) = (&mut elem.node, &mut elem.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, elem.parent, elem.was_removed));
    }

    fn with_downcast_b(elem: &mut Mut<Self::OneOfElement>, f: impl FnOnce(Mut<Pod<N2>>)) {
        let (OneOf::B(node), OneOf::B(props)) = (&mut elem.node, &mut elem.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, elem.parent, elem.was_removed));
    }

    fn with_downcast_c(elem: &mut Mut<Self::OneOfElement>, f: impl FnOnce(Mut<Pod<N3>>)) {
        let (OneOf::C(node), OneOf::C(props)) = (&mut elem.node, &mut elem.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, elem.parent, elem.was_removed));
    }

    fn with_downcast_d(elem: &mut Mut<Self::OneOfElement>, f: impl FnOnce(Mut<Pod<N4>>)) {
        let (OneOf::D(node), OneOf::D(props)) = (&mut elem.node, &mut elem.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, elem.parent, elem.was_removed));
    }

    fn with_downcast_e(elem: &mut Mut<Self::OneOfElement>, f: impl FnOnce(Mut<Pod<N5>>)) {
        let (OneOf::E(node), OneOf::E(props)) = (&mut elem.node, &mut elem.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, elem.parent, elem.was_removed));
    }

    fn with_downcast_f(elem: &mut Mut<Self::OneOfElement>, f: impl FnOnce(Mut<Pod<N6>>)) {
        let (OneOf::F(node), OneOf::F(props)) = (&mut elem.node, &mut elem.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, elem.parent, elem.was_removed));
    }

    fn with_downcast_g(elem: &mut Mut<Self::OneOfElement>, f: impl FnOnce(Mut<Pod<N7>>)) {
        let (OneOf::G(node), OneOf::G(props)) = (&mut elem.node, &mut elem.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, elem.parent, elem.was_removed));
    }

    fn with_downcast_h(elem: &mut Mut<Self::OneOfElement>, f: impl FnOnce(Mut<Pod<N8>>)) {
        let (OneOf::H(node), OneOf::H(props)) = (&mut elem.node, &mut elem.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, elem.parent, elem.was_removed));
    }

    fn with_downcast_i(elem: &mut Mut<Self::OneOfElement>, f: impl FnOnce(Mut<Pod<N9>>)) {
        let (OneOf::I(node), OneOf::I(props)) = (&mut elem.node, &mut elem.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, elem.parent, elem.was_removed));
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
    fn apply_props(&self, props: &mut Self::Props) {
        match (self, props) {
            (OneOf::A(el), OneOf::A(props)) => el.apply_props(props),
            (OneOf::B(el), OneOf::B(props)) => el.apply_props(props),
            (OneOf::C(el), OneOf::C(props)) => el.apply_props(props),
            (OneOf::D(el), OneOf::D(props)) => el.apply_props(props),
            (OneOf::E(el), OneOf::E(props)) => el.apply_props(props),
            (OneOf::F(el), OneOf::F(props)) => el.apply_props(props),
            (OneOf::G(el), OneOf::G(props)) => el.apply_props(props),
            (OneOf::H(el), OneOf::H(props)) => el.apply_props(props),
            (OneOf::I(el), OneOf::I(props)) => el.apply_props(props),
            _ => unreachable!(),
        }
    }
}

impl<T, A, B, C, D, E, F, G, H, I> With<T> for OneOf<A, B, C, D, E, F, G, H, I>
where
    A: With<T>,
    B: With<T>,
    C: With<T>,
    D: With<T>,
    E: With<T>,
    F: With<T>,
    G: With<T>,
    H: With<T>,
    I: With<T>,
{
    fn modifier(&mut self) -> &mut T {
        match self {
            OneOf::A(e) => <A as With<T>>::modifier(e),
            OneOf::B(e) => <B as With<T>>::modifier(e),
            OneOf::C(e) => <C as With<T>>::modifier(e),
            OneOf::D(e) => <D as With<T>>::modifier(e),
            OneOf::E(e) => <E as With<T>>::modifier(e),
            OneOf::F(e) => <F as With<T>>::modifier(e),
            OneOf::G(e) => <G as With<T>>::modifier(e),
            OneOf::H(e) => <H as With<T>>::modifier(e),
            OneOf::I(e) => <I as With<T>>::modifier(e),
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

impl<T> With<T> for Noop {
    fn modifier(&mut self) -> &mut T {
        match *self {}
    }
}

impl PhantomElementCtx for ViewCtx {
    type PhantomElement = Pod<Noop>;
}

impl DomNode for Noop {
    fn apply_props(&self, _props: &mut Self::Props) {
        match *self {}
    }

    type Props = Noop;
}
