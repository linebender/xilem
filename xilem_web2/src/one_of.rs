use xilem_core::{Mut, OneOf2, OneOf2Ctx};

use crate::{
    attribute::WithAttributes, class::WithClasses, AttributeValue, DomNode, Pod, PodMut, ViewCtx,
};

type CowStr = std::borrow::Cow<'static, str>;

impl<P1: 'static, P2: 'static, N1: DomNode<P1>, N2: DomNode<P2>> OneOf2Ctx<Pod<N1, P1>, Pod<N2, P2>>
    for ViewCtx
{
    type OneOfTwoElement = Pod<OneOf2<N1, N2>, OneOf2<P1, P2>>;

    fn upcast_one_of_two_element(elem: OneOf2<Pod<N1, P1>, Pod<N2, P2>>) -> Self::OneOfTwoElement {
        match elem {
            OneOf2::A(e) => Pod {
                node: OneOf2::A(e.node),
                props: OneOf2::A(e.props),
            },
            OneOf2::B(e) => Pod {
                node: OneOf2::B(e.node),
                props: OneOf2::B(e.props),
            },
        }
    }

    fn update_one_of_two_element_mut(
        elem_mut: &mut Mut<'_, Self::OneOfTwoElement>,
        new_elem: OneOf2<Pod<N1, P1>, Pod<N2, P2>>,
    ) {
        (*elem_mut.node, *elem_mut.props) = match new_elem {
            OneOf2::A(e) => (OneOf2::A(e.node), OneOf2::A(e.props)),
            OneOf2::B(e) => (OneOf2::B(e.node), OneOf2::B(e.props)),
        };
    }

    fn with_downcast_a(
        elem: &mut Mut<'_, Self::OneOfTwoElement>,
        f: impl FnOnce(Mut<'_, Pod<N1, P1>>),
    ) {
        let (OneOf2::A(node), OneOf2::A(props)) = (&mut elem.node, &mut elem.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, elem.was_removed));
    }

    fn with_downcast_b(
        elem: &mut Mut<'_, Self::OneOfTwoElement>,
        f: impl FnOnce(Mut<'_, Pod<N2, P2>>),
    ) {
        let (OneOf2::B(node), OneOf2::B(props)) = (&mut elem.node, &mut elem.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, elem.was_removed));
    }
}

impl<E1: WithAttributes, E2: WithAttributes> WithAttributes for OneOf2<E1, E2> {
    fn start_attribute_modifier(&mut self) {
        match self {
            OneOf2::A(e) => e.start_attribute_modifier(),
            OneOf2::B(e) => e.start_attribute_modifier(),
        }
    }

    fn end_attribute_modifier(&mut self) {
        match self {
            OneOf2::A(e) => e.end_attribute_modifier(),
            OneOf2::B(e) => e.end_attribute_modifier(),
        }
    }

    fn set_attribute(&mut self, name: CowStr, value: Option<AttributeValue>) {
        match self {
            OneOf2::A(e) => e.set_attribute(name, value),
            OneOf2::B(e) => e.set_attribute(name, value),
        }
    }
}

impl<E1: WithClasses, E2: WithClasses> WithClasses for OneOf2<E1, E2> {
    fn start_class_modifier(&mut self) {
        match self {
            OneOf2::A(e) => e.start_class_modifier(),
            OneOf2::B(e) => e.start_class_modifier(),
        }
    }

    fn add_class(&mut self, class_name: CowStr) {
        match self {
            OneOf2::A(e) => e.add_class(class_name),
            OneOf2::B(e) => e.add_class(class_name),
        }
    }

    fn remove_class(&mut self, class_name: CowStr) {
        match self {
            OneOf2::A(e) => e.remove_class(class_name),
            OneOf2::B(e) => e.remove_class(class_name),
        }
    }

    fn end_class_modifier(&mut self) {
        match self {
            OneOf2::A(e) => e.end_class_modifier(),
            OneOf2::B(e) => e.end_class_modifier(),
        }
    }
}

impl<P1, P2, E1: DomNode<P1>, E2: DomNode<P2>> DomNode<OneOf2<P1, P2>> for OneOf2<E1, E2> {
    fn apply_props(&self, props: &mut OneOf2<P1, P2>) {
        match (self, props) {
            (OneOf2::A(el), OneOf2::A(props)) => el.apply_props(props),
            (OneOf2::B(el), OneOf2::B(props)) => el.apply_props(props),
            _ => unreachable!(),
        }
    }
}
