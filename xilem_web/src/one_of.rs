// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use wasm_bindgen::UnwrapThrowExt;
use xilem_core::{
    one_of::{OneOf, OneOfCtx, PhantomElementCtx},
    Mut,
};

use crate::{
    attribute::WithAttributes, class::WithClasses, style::WithStyle, AttributeValue, DomNode, Pod,
    PodMut, ViewCtx,
};

type CowStr = std::borrow::Cow<'static, str>;

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
        elem_mut: &mut Mut<'_, Self::OneOfElement>,
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

    fn with_downcast_a(elem: &mut Mut<'_, Self::OneOfElement>, f: impl FnOnce(Mut<'_, Pod<N1>>)) {
        let (OneOf::A(node), OneOf::A(props)) = (&mut elem.node, &mut elem.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, elem.parent, elem.was_removed));
    }

    fn with_downcast_b(elem: &mut Mut<'_, Self::OneOfElement>, f: impl FnOnce(Mut<'_, Pod<N2>>)) {
        let (OneOf::B(node), OneOf::B(props)) = (&mut elem.node, &mut elem.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, elem.parent, elem.was_removed));
    }

    fn with_downcast_c(elem: &mut Mut<'_, Self::OneOfElement>, f: impl FnOnce(Mut<'_, Pod<N3>>)) {
        let (OneOf::C(node), OneOf::C(props)) = (&mut elem.node, &mut elem.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, elem.parent, elem.was_removed));
    }

    fn with_downcast_d(elem: &mut Mut<'_, Self::OneOfElement>, f: impl FnOnce(Mut<'_, Pod<N4>>)) {
        let (OneOf::D(node), OneOf::D(props)) = (&mut elem.node, &mut elem.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, elem.parent, elem.was_removed));
    }

    fn with_downcast_e(elem: &mut Mut<'_, Self::OneOfElement>, f: impl FnOnce(Mut<'_, Pod<N5>>)) {
        let (OneOf::E(node), OneOf::E(props)) = (&mut elem.node, &mut elem.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, elem.parent, elem.was_removed));
    }

    fn with_downcast_f(elem: &mut Mut<'_, Self::OneOfElement>, f: impl FnOnce(Mut<'_, Pod<N6>>)) {
        let (OneOf::F(node), OneOf::F(props)) = (&mut elem.node, &mut elem.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, elem.parent, elem.was_removed));
    }

    fn with_downcast_g(elem: &mut Mut<'_, Self::OneOfElement>, f: impl FnOnce(Mut<'_, Pod<N7>>)) {
        let (OneOf::G(node), OneOf::G(props)) = (&mut elem.node, &mut elem.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, elem.parent, elem.was_removed));
    }

    fn with_downcast_h(elem: &mut Mut<'_, Self::OneOfElement>, f: impl FnOnce(Mut<'_, Pod<N8>>)) {
        let (OneOf::H(node), OneOf::H(props)) = (&mut elem.node, &mut elem.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, elem.parent, elem.was_removed));
    }

    fn with_downcast_i(elem: &mut Mut<'_, Self::OneOfElement>, f: impl FnOnce(Mut<'_, Pod<N9>>)) {
        let (OneOf::I(node), OneOf::I(props)) = (&mut elem.node, &mut elem.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, elem.parent, elem.was_removed));
    }
}

pub enum Noop {}

impl PhantomElementCtx for ViewCtx {
    type PhantomElement = Pod<Noop>;
}

impl WithAttributes for Noop {
    fn rebuild_attribute_modifier(&mut self) {
        unreachable!()
    }

    fn mark_end_of_attribute_modifier(&mut self) {
        unreachable!()
    }

    fn set_attribute(&mut self, _name: &CowStr, _value: &Option<AttributeValue>) {
        unreachable!()
    }
}

impl WithClasses for Noop {
    fn rebuild_class_modifier(&mut self) {
        unreachable!()
    }

    fn add_class(&mut self, _class_name: &CowStr) {
        unreachable!()
    }

    fn remove_class(&mut self, _class_name: &CowStr) {
        unreachable!()
    }

    fn mark_end_of_class_modifier(&mut self) {
        unreachable!()
    }
}

impl WithStyle for Noop {
    fn rebuild_style_modifier(&mut self) {
        unreachable!()
    }

    fn set_style(&mut self, _name: &CowStr, _value: &Option<CowStr>) {
        unreachable!()
    }

    fn mark_end_of_style_modifier(&mut self) {
        unreachable!()
    }

    fn get_style(&self, _name: &str) -> Option<&CowStr> {
        unreachable!()
    }

    fn was_updated(&self, _name: &str) -> bool {
        unreachable!()
    }
}

impl<T> AsRef<T> for Noop {
    fn as_ref(&self) -> &T {
        unreachable!()
    }
}

impl DomNode for Noop {
    fn apply_props(&self, _props: &mut Self::Props) {
        unreachable!()
    }

    type Props = Noop;
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
    > WithAttributes for OneOf<E1, E2, E3, E4, E5, E6, E7, E8, E9>
{
    fn rebuild_attribute_modifier(&mut self) {
        match self {
            OneOf::A(e) => e.rebuild_attribute_modifier(),
            OneOf::B(e) => e.rebuild_attribute_modifier(),
            OneOf::C(e) => e.rebuild_attribute_modifier(),
            OneOf::D(e) => e.rebuild_attribute_modifier(),
            OneOf::E(e) => e.rebuild_attribute_modifier(),
            OneOf::F(e) => e.rebuild_attribute_modifier(),
            OneOf::G(e) => e.rebuild_attribute_modifier(),
            OneOf::H(e) => e.rebuild_attribute_modifier(),
            OneOf::I(e) => e.rebuild_attribute_modifier(),
        }
    }

    fn mark_end_of_attribute_modifier(&mut self) {
        match self {
            OneOf::A(e) => e.mark_end_of_attribute_modifier(),
            OneOf::B(e) => e.mark_end_of_attribute_modifier(),
            OneOf::C(e) => e.mark_end_of_attribute_modifier(),
            OneOf::D(e) => e.mark_end_of_attribute_modifier(),
            OneOf::E(e) => e.mark_end_of_attribute_modifier(),
            OneOf::F(e) => e.mark_end_of_attribute_modifier(),
            OneOf::G(e) => e.mark_end_of_attribute_modifier(),
            OneOf::H(e) => e.mark_end_of_attribute_modifier(),
            OneOf::I(e) => e.mark_end_of_attribute_modifier(),
        }
    }

    fn set_attribute(&mut self, name: &CowStr, value: &Option<AttributeValue>) {
        match self {
            OneOf::A(e) => e.set_attribute(name, value),
            OneOf::B(e) => e.set_attribute(name, value),
            OneOf::C(e) => e.set_attribute(name, value),
            OneOf::D(e) => e.set_attribute(name, value),
            OneOf::E(e) => e.set_attribute(name, value),
            OneOf::F(e) => e.set_attribute(name, value),
            OneOf::G(e) => e.set_attribute(name, value),
            OneOf::H(e) => e.set_attribute(name, value),
            OneOf::I(e) => e.set_attribute(name, value),
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
    > WithClasses for OneOf<E1, E2, E3, E4, E5, E6, E7, E8, E9>
{
    fn rebuild_class_modifier(&mut self) {
        match self {
            OneOf::A(e) => e.rebuild_class_modifier(),
            OneOf::B(e) => e.rebuild_class_modifier(),
            OneOf::C(e) => e.rebuild_class_modifier(),
            OneOf::D(e) => e.rebuild_class_modifier(),
            OneOf::E(e) => e.rebuild_class_modifier(),
            OneOf::F(e) => e.rebuild_class_modifier(),
            OneOf::G(e) => e.rebuild_class_modifier(),
            OneOf::H(e) => e.rebuild_class_modifier(),
            OneOf::I(e) => e.rebuild_class_modifier(),
        }
    }

    fn add_class(&mut self, class_name: &CowStr) {
        match self {
            OneOf::A(e) => e.add_class(class_name),
            OneOf::B(e) => e.add_class(class_name),
            OneOf::C(e) => e.add_class(class_name),
            OneOf::D(e) => e.add_class(class_name),
            OneOf::E(e) => e.add_class(class_name),
            OneOf::F(e) => e.add_class(class_name),
            OneOf::G(e) => e.add_class(class_name),
            OneOf::H(e) => e.add_class(class_name),
            OneOf::I(e) => e.add_class(class_name),
        }
    }

    fn remove_class(&mut self, class_name: &CowStr) {
        match self {
            OneOf::A(e) => e.remove_class(class_name),
            OneOf::B(e) => e.remove_class(class_name),
            OneOf::C(e) => e.remove_class(class_name),
            OneOf::D(e) => e.remove_class(class_name),
            OneOf::E(e) => e.remove_class(class_name),
            OneOf::F(e) => e.remove_class(class_name),
            OneOf::G(e) => e.remove_class(class_name),
            OneOf::H(e) => e.remove_class(class_name),
            OneOf::I(e) => e.remove_class(class_name),
        }
    }

    fn mark_end_of_class_modifier(&mut self) {
        match self {
            OneOf::A(e) => e.mark_end_of_class_modifier(),
            OneOf::B(e) => e.mark_end_of_class_modifier(),
            OneOf::C(e) => e.mark_end_of_class_modifier(),
            OneOf::D(e) => e.mark_end_of_class_modifier(),
            OneOf::E(e) => e.mark_end_of_class_modifier(),
            OneOf::F(e) => e.mark_end_of_class_modifier(),
            OneOf::G(e) => e.mark_end_of_class_modifier(),
            OneOf::H(e) => e.mark_end_of_class_modifier(),
            OneOf::I(e) => e.mark_end_of_class_modifier(),
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
    > WithStyle for OneOf<E1, E2, E3, E4, E5, E6, E7, E8, E9>
{
    fn rebuild_style_modifier(&mut self) {
        match self {
            OneOf::A(e) => e.rebuild_style_modifier(),
            OneOf::B(e) => e.rebuild_style_modifier(),
            OneOf::C(e) => e.rebuild_style_modifier(),
            OneOf::D(e) => e.rebuild_style_modifier(),
            OneOf::E(e) => e.rebuild_style_modifier(),
            OneOf::F(e) => e.rebuild_style_modifier(),
            OneOf::G(e) => e.rebuild_style_modifier(),
            OneOf::H(e) => e.rebuild_style_modifier(),
            OneOf::I(e) => e.rebuild_style_modifier(),
        }
    }

    fn set_style(&mut self, name: &CowStr, value: &Option<CowStr>) {
        match self {
            OneOf::A(e) => e.set_style(name, value),
            OneOf::B(e) => e.set_style(name, value),
            OneOf::C(e) => e.set_style(name, value),
            OneOf::D(e) => e.set_style(name, value),
            OneOf::E(e) => e.set_style(name, value),
            OneOf::F(e) => e.set_style(name, value),
            OneOf::G(e) => e.set_style(name, value),
            OneOf::H(e) => e.set_style(name, value),
            OneOf::I(e) => e.set_style(name, value),
        }
    }

    fn mark_end_of_style_modifier(&mut self) {
        match self {
            OneOf::A(e) => e.mark_end_of_style_modifier(),
            OneOf::B(e) => e.mark_end_of_style_modifier(),
            OneOf::C(e) => e.mark_end_of_style_modifier(),
            OneOf::D(e) => e.mark_end_of_style_modifier(),
            OneOf::E(e) => e.mark_end_of_style_modifier(),
            OneOf::F(e) => e.mark_end_of_style_modifier(),
            OneOf::G(e) => e.mark_end_of_style_modifier(),
            OneOf::H(e) => e.mark_end_of_style_modifier(),
            OneOf::I(e) => e.mark_end_of_style_modifier(),
        }
    }

    fn get_style(&self, name: &str) -> Option<&CowStr> {
        match self {
            OneOf::A(e) => e.get_style(name),
            OneOf::B(e) => e.get_style(name),
            OneOf::C(e) => e.get_style(name),
            OneOf::D(e) => e.get_style(name),
            OneOf::E(e) => e.get_style(name),
            OneOf::F(e) => e.get_style(name),
            OneOf::G(e) => e.get_style(name),
            OneOf::H(e) => e.get_style(name),
            OneOf::I(e) => e.get_style(name),
        }
    }

    fn was_updated(&self, name: &str) -> bool {
        match self {
            OneOf::A(e) => e.was_updated(name),
            OneOf::B(e) => e.was_updated(name),
            OneOf::C(e) => e.was_updated(name),
            OneOf::D(e) => e.was_updated(name),
            OneOf::E(e) => e.was_updated(name),
            OneOf::F(e) => e.was_updated(name),
            OneOf::G(e) => e.was_updated(name),
            OneOf::H(e) => e.was_updated(name),
            OneOf::I(e) => e.was_updated(name),
        }
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
