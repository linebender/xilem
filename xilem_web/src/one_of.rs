// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use wasm_bindgen::UnwrapThrowExt;
use xilem_core::{
    one_of::{OneOf9, OneOfCtx, PhantomElementCtx},
    Mut,
};

use crate::{
    attribute::WithAttributes, class::WithClasses, style::WithStyle, AttributeValue, DomNode, Pod,
    PodMut, ViewCtx,
};

type CowStr = std::borrow::Cow<'static, str>;

impl<P1, P2, P3, P4, P5, P6, P7, P8, P9, N1, N2, N3, N4, N5, N6, N7, N8, N9>
    OneOfCtx<
        Pod<N1, P1>,
        Pod<N2, P2>,
        Pod<N3, P3>,
        Pod<N4, P4>,
        Pod<N5, P5>,
        Pod<N6, P6>,
        Pod<N7, P7>,
        Pod<N8, P8>,
        Pod<N9, P9>,
    > for ViewCtx
where
    P1: 'static,
    P2: 'static,
    P3: 'static,
    P4: 'static,
    P5: 'static,
    P6: 'static,
    P7: 'static,
    P8: 'static,
    P9: 'static,
    N1: DomNode<P1>,
    N2: DomNode<P2>,
    N3: DomNode<P3>,
    N4: DomNode<P4>,
    N5: DomNode<P5>,
    N6: DomNode<P6>,
    N7: DomNode<P7>,
    N8: DomNode<P8>,
    N9: DomNode<P9>,
{
    type OneOfElement =
        Pod<OneOf9<N1, N2, N3, N4, N5, N6, N7, N8, N9>, OneOf9<P1, P2, P3, P4, P5, P6, P7, P8, P9>>;

    fn upcast_one_of_element(
        elem: OneOf9<
            Pod<N1, P1>,
            Pod<N2, P2>,
            Pod<N3, P3>,
            Pod<N4, P4>,
            Pod<N5, P5>,
            Pod<N6, P6>,
            Pod<N7, P7>,
            Pod<N8, P8>,
            Pod<N9, P9>,
        >,
    ) -> Self::OneOfElement {
        match elem {
            OneOf9::A(e) => Pod {
                node: OneOf9::A(e.node),
                props: OneOf9::A(e.props),
            },
            OneOf9::B(e) => Pod {
                node: OneOf9::B(e.node),
                props: OneOf9::B(e.props),
            },
            OneOf9::C(e) => Pod {
                node: OneOf9::C(e.node),
                props: OneOf9::C(e.props),
            },
            OneOf9::D(e) => Pod {
                node: OneOf9::D(e.node),
                props: OneOf9::D(e.props),
            },
            OneOf9::E(e) => Pod {
                node: OneOf9::E(e.node),
                props: OneOf9::E(e.props),
            },
            OneOf9::F(e) => Pod {
                node: OneOf9::F(e.node),
                props: OneOf9::F(e.props),
            },
            OneOf9::G(e) => Pod {
                node: OneOf9::G(e.node),
                props: OneOf9::G(e.props),
            },
            OneOf9::H(e) => Pod {
                node: OneOf9::H(e.node),
                props: OneOf9::H(e.props),
            },
            OneOf9::I(e) => Pod {
                node: OneOf9::I(e.node),
                props: OneOf9::I(e.props),
            },
        }
    }

    fn update_one_of_element_mut(
        elem_mut: &mut Mut<'_, Self::OneOfElement>,
        new_elem: OneOf9<
            Pod<N1, P1>,
            Pod<N2, P2>,
            Pod<N3, P3>,
            Pod<N4, P4>,
            Pod<N5, P5>,
            Pod<N6, P6>,
            Pod<N7, P7>,
            Pod<N8, P8>,
            Pod<N9, P9>,
        >,
    ) {
        let old_node: &web_sys::Node = elem_mut.node.as_ref();
        let new_node: &web_sys::Node = new_elem.as_ref();
        if old_node != new_node {
            elem_mut
                .parent
                .replace_child(new_node, old_node)
                .unwrap_throw();
        }
        (*elem_mut.node, *elem_mut.props) = match new_elem {
            OneOf9::A(e) => (OneOf9::A(e.node), OneOf9::A(e.props)),
            OneOf9::B(e) => (OneOf9::B(e.node), OneOf9::B(e.props)),
            OneOf9::C(e) => (OneOf9::C(e.node), OneOf9::C(e.props)),
            OneOf9::D(e) => (OneOf9::D(e.node), OneOf9::D(e.props)),
            OneOf9::E(e) => (OneOf9::E(e.node), OneOf9::E(e.props)),
            OneOf9::F(e) => (OneOf9::F(e.node), OneOf9::F(e.props)),
            OneOf9::G(e) => (OneOf9::G(e.node), OneOf9::G(e.props)),
            OneOf9::H(e) => (OneOf9::H(e.node), OneOf9::H(e.props)),
            OneOf9::I(e) => (OneOf9::I(e.node), OneOf9::I(e.props)),
        };
    }

    fn with_downcast_a(
        elem: &mut Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(Mut<'_, Pod<N1, P1>>),
    ) {
        let (OneOf9::A(node), OneOf9::A(props)) = (&mut elem.node, &mut elem.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, elem.parent, elem.was_removed));
    }

    fn with_downcast_b(
        elem: &mut Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(Mut<'_, Pod<N2, P2>>),
    ) {
        let (OneOf9::B(node), OneOf9::B(props)) = (&mut elem.node, &mut elem.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, elem.parent, elem.was_removed));
    }

    fn with_downcast_c(
        elem: &mut Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(Mut<'_, Pod<N3, P3>>),
    ) {
        let (OneOf9::C(node), OneOf9::C(props)) = (&mut elem.node, &mut elem.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, elem.parent, elem.was_removed));
    }

    fn with_downcast_d(
        elem: &mut Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(Mut<'_, Pod<N4, P4>>),
    ) {
        let (OneOf9::D(node), OneOf9::D(props)) = (&mut elem.node, &mut elem.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, elem.parent, elem.was_removed));
    }

    fn with_downcast_e(
        elem: &mut Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(Mut<'_, Pod<N5, P5>>),
    ) {
        let (OneOf9::E(node), OneOf9::E(props)) = (&mut elem.node, &mut elem.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, elem.parent, elem.was_removed));
    }

    fn with_downcast_f(
        elem: &mut Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(Mut<'_, Pod<N6, P6>>),
    ) {
        let (OneOf9::F(node), OneOf9::F(props)) = (&mut elem.node, &mut elem.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, elem.parent, elem.was_removed));
    }

    fn with_downcast_g(
        elem: &mut Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(Mut<'_, Pod<N7, P7>>),
    ) {
        let (OneOf9::G(node), OneOf9::G(props)) = (&mut elem.node, &mut elem.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, elem.parent, elem.was_removed));
    }

    fn with_downcast_h(
        elem: &mut Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(Mut<'_, Pod<N8, P8>>),
    ) {
        let (OneOf9::H(node), OneOf9::H(props)) = (&mut elem.node, &mut elem.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, elem.parent, elem.was_removed));
    }

    fn with_downcast_i(
        elem: &mut Mut<'_, Self::OneOfElement>,
        f: impl FnOnce(Mut<'_, Pod<N9, P9>>),
    ) {
        let (OneOf9::I(node), OneOf9::I(props)) = (&mut elem.node, &mut elem.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, elem.parent, elem.was_removed));
    }
}

pub enum Noop {}

impl PhantomElementCtx for ViewCtx {
    type PhantomElement = Pod<Noop, Noop>;
}

impl WithAttributes for Noop {
    fn start_attribute_modifier(&mut self) {
        unreachable!()
    }

    fn end_attribute_modifier(&mut self) {
        unreachable!()
    }

    fn set_attribute(&mut self, _name: CowStr, _value: Option<AttributeValue>) {
        unreachable!()
    }
}

impl WithClasses for Noop {
    fn start_class_modifier(&mut self) {
        unreachable!()
    }

    fn add_class(&mut self, _class_name: CowStr) {
        unreachable!()
    }

    fn remove_class(&mut self, _class_name: CowStr) {
        unreachable!()
    }

    fn end_class_modifier(&mut self) {
        unreachable!()
    }
}

impl WithStyle for Noop {
    fn start_style_modifier(&mut self) {
        unreachable!()
    }

    fn set_style(&mut self, _name: CowStr, _value: Option<CowStr>) {
        unreachable!()
    }

    fn end_style_modifier(&mut self) {
        unreachable!()
    }
}

impl<T> AsRef<T> for Noop {
    fn as_ref(&self) -> &T {
        unreachable!()
    }
}

impl<P> DomNode<P> for Noop {
    fn apply_props(&self, _props: &mut P) {
        unreachable!()
    }
}

impl<
        E1: WithAttributes,
        E2: WithAttributes,
        E3: WithAttributes,
        E4: WithAttributes,
        E5: WithAttributes,
        E6: WithAttributes,
        E7: WithAttributes,
        E8: WithAttributes,
        E9: WithAttributes,
    > WithAttributes for OneOf9<E1, E2, E3, E4, E5, E6, E7, E8, E9>
{
    fn start_attribute_modifier(&mut self) {
        match self {
            OneOf9::A(e) => e.start_attribute_modifier(),
            OneOf9::B(e) => e.start_attribute_modifier(),
            OneOf9::C(e) => e.start_attribute_modifier(),
            OneOf9::D(e) => e.start_attribute_modifier(),
            OneOf9::E(e) => e.start_attribute_modifier(),
            OneOf9::F(e) => e.start_attribute_modifier(),
            OneOf9::G(e) => e.start_attribute_modifier(),
            OneOf9::H(e) => e.start_attribute_modifier(),
            OneOf9::I(e) => e.start_attribute_modifier(),
        }
    }

    fn end_attribute_modifier(&mut self) {
        match self {
            OneOf9::A(e) => e.end_attribute_modifier(),
            OneOf9::B(e) => e.end_attribute_modifier(),
            OneOf9::C(e) => e.end_attribute_modifier(),
            OneOf9::D(e) => e.end_attribute_modifier(),
            OneOf9::E(e) => e.end_attribute_modifier(),
            OneOf9::F(e) => e.end_attribute_modifier(),
            OneOf9::G(e) => e.end_attribute_modifier(),
            OneOf9::H(e) => e.end_attribute_modifier(),
            OneOf9::I(e) => e.end_attribute_modifier(),
        }
    }

    fn set_attribute(&mut self, name: CowStr, value: Option<AttributeValue>) {
        match self {
            OneOf9::A(e) => e.set_attribute(name, value),
            OneOf9::B(e) => e.set_attribute(name, value),
            OneOf9::C(e) => e.set_attribute(name, value),
            OneOf9::D(e) => e.set_attribute(name, value),
            OneOf9::E(e) => e.set_attribute(name, value),
            OneOf9::F(e) => e.set_attribute(name, value),
            OneOf9::G(e) => e.set_attribute(name, value),
            OneOf9::H(e) => e.set_attribute(name, value),
            OneOf9::I(e) => e.set_attribute(name, value),
        }
    }
}

impl<
        E1: WithClasses,
        E2: WithClasses,
        E3: WithClasses,
        E4: WithClasses,
        E5: WithClasses,
        E6: WithClasses,
        E7: WithClasses,
        E8: WithClasses,
        E9: WithClasses,
    > WithClasses for OneOf9<E1, E2, E3, E4, E5, E6, E7, E8, E9>
{
    fn start_class_modifier(&mut self) {
        match self {
            OneOf9::A(e) => e.start_class_modifier(),
            OneOf9::B(e) => e.start_class_modifier(),
            OneOf9::C(e) => e.start_class_modifier(),
            OneOf9::D(e) => e.start_class_modifier(),
            OneOf9::E(e) => e.start_class_modifier(),
            OneOf9::F(e) => e.start_class_modifier(),
            OneOf9::G(e) => e.start_class_modifier(),
            OneOf9::H(e) => e.start_class_modifier(),
            OneOf9::I(e) => e.start_class_modifier(),
        }
    }

    fn add_class(&mut self, class_name: CowStr) {
        match self {
            OneOf9::A(e) => e.add_class(class_name),
            OneOf9::B(e) => e.add_class(class_name),
            OneOf9::C(e) => e.add_class(class_name),
            OneOf9::D(e) => e.add_class(class_name),
            OneOf9::E(e) => e.add_class(class_name),
            OneOf9::F(e) => e.add_class(class_name),
            OneOf9::G(e) => e.add_class(class_name),
            OneOf9::H(e) => e.add_class(class_name),
            OneOf9::I(e) => e.add_class(class_name),
        }
    }

    fn remove_class(&mut self, class_name: CowStr) {
        match self {
            OneOf9::A(e) => e.remove_class(class_name),
            OneOf9::B(e) => e.remove_class(class_name),
            OneOf9::C(e) => e.remove_class(class_name),
            OneOf9::D(e) => e.remove_class(class_name),
            OneOf9::E(e) => e.remove_class(class_name),
            OneOf9::F(e) => e.remove_class(class_name),
            OneOf9::G(e) => e.remove_class(class_name),
            OneOf9::H(e) => e.remove_class(class_name),
            OneOf9::I(e) => e.remove_class(class_name),
        }
    }

    fn end_class_modifier(&mut self) {
        match self {
            OneOf9::A(e) => e.end_class_modifier(),
            OneOf9::B(e) => e.end_class_modifier(),
            OneOf9::C(e) => e.end_class_modifier(),
            OneOf9::D(e) => e.end_class_modifier(),
            OneOf9::E(e) => e.end_class_modifier(),
            OneOf9::F(e) => e.end_class_modifier(),
            OneOf9::G(e) => e.end_class_modifier(),
            OneOf9::H(e) => e.end_class_modifier(),
            OneOf9::I(e) => e.end_class_modifier(),
        }
    }
}

impl<
        E1: WithStyle,
        E2: WithStyle,
        E3: WithStyle,
        E4: WithStyle,
        E5: WithStyle,
        E6: WithStyle,
        E7: WithStyle,
        E8: WithStyle,
        E9: WithStyle,
    > WithStyle for OneOf9<E1, E2, E3, E4, E5, E6, E7, E8, E9>
{
    fn start_style_modifier(&mut self) {
        match self {
            OneOf9::A(e) => e.start_style_modifier(),
            OneOf9::B(e) => e.start_style_modifier(),
            OneOf9::C(e) => e.start_style_modifier(),
            OneOf9::D(e) => e.start_style_modifier(),
            OneOf9::E(e) => e.start_style_modifier(),
            OneOf9::F(e) => e.start_style_modifier(),
            OneOf9::G(e) => e.start_style_modifier(),
            OneOf9::H(e) => e.start_style_modifier(),
            OneOf9::I(e) => e.start_style_modifier(),
        }
    }

    fn set_style(&mut self, name: CowStr, value: Option<CowStr>) {
        match self {
            OneOf9::A(e) => e.set_style(name, value),
            OneOf9::B(e) => e.set_style(name, value),
            OneOf9::C(e) => e.set_style(name, value),
            OneOf9::D(e) => e.set_style(name, value),
            OneOf9::E(e) => e.set_style(name, value),
            OneOf9::F(e) => e.set_style(name, value),
            OneOf9::G(e) => e.set_style(name, value),
            OneOf9::H(e) => e.set_style(name, value),
            OneOf9::I(e) => e.set_style(name, value),
        }
    }

    fn end_style_modifier(&mut self) {
        match self {
            OneOf9::A(e) => e.end_style_modifier(),
            OneOf9::B(e) => e.end_style_modifier(),
            OneOf9::C(e) => e.end_style_modifier(),
            OneOf9::D(e) => e.end_style_modifier(),
            OneOf9::E(e) => e.end_style_modifier(),
            OneOf9::F(e) => e.end_style_modifier(),
            OneOf9::G(e) => e.end_style_modifier(),
            OneOf9::H(e) => e.end_style_modifier(),
            OneOf9::I(e) => e.end_style_modifier(),
        }
    }
}

impl<P1, P2, P3, P4, P5, P6, P7, P8, P9, E1, E2, E3, E4, E5, E6, E7, E8, E9>
    DomNode<OneOf9<P1, P2, P3, P4, P5, P6, P7, P8, P9>>
    for OneOf9<E1, E2, E3, E4, E5, E6, E7, E8, E9>
where
    E1: DomNode<P1>,
    E2: DomNode<P2>,
    E3: DomNode<P3>,
    E4: DomNode<P4>,
    E5: DomNode<P5>,
    E6: DomNode<P6>,
    E7: DomNode<P7>,
    E8: DomNode<P8>,
    E9: DomNode<P9>,
{
    fn apply_props(&self, props: &mut OneOf9<P1, P2, P3, P4, P5, P6, P7, P8, P9>) {
        match (self, props) {
            (OneOf9::A(el), OneOf9::A(props)) => el.apply_props(props),
            (OneOf9::B(el), OneOf9::B(props)) => el.apply_props(props),
            (OneOf9::C(el), OneOf9::C(props)) => el.apply_props(props),
            (OneOf9::D(el), OneOf9::D(props)) => el.apply_props(props),
            (OneOf9::E(el), OneOf9::E(props)) => el.apply_props(props),
            (OneOf9::F(el), OneOf9::F(props)) => el.apply_props(props),
            (OneOf9::G(el), OneOf9::G(props)) => el.apply_props(props),
            (OneOf9::H(el), OneOf9::H(props)) => el.apply_props(props),
            (OneOf9::I(el), OneOf9::I(props)) => el.apply_props(props),
            _ => unreachable!(),
        }
    }
}
