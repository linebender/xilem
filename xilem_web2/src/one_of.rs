use xilem_core::{Mut, OneOf2, OneOf2Ctx};

use crate::{
    attribute::WithAttributes, class::WithClasses, AttributeValue, CowStr, DomNode, DynNode, Pod,
    PodMut, ViewCtx,
};

impl<N1: DomNode, N2: DomNode> OneOf2Ctx<Pod<N1>, Pod<N2>> for ViewCtx {
    type OneOfTwoElement = Pod<OneOf2<N1, N2>>;

    fn upcast_one_of_two_element(elem: OneOf2<Pod<N1>, Pod<N2>>) -> Self::OneOfTwoElement {
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
        new_elem: OneOf2<Pod<N1>, Pod<N2>>,
    ) {
        (*elem_mut.node, *elem_mut.props) = match new_elem {
            OneOf2::A(e) => (OneOf2::A(e.node), OneOf2::A(e.props)),
            OneOf2::B(e) => (OneOf2::B(e.node), OneOf2::B(e.props)),
        };
    }

    fn with_downcast_a(
        elem: &mut Mut<'_, Self::OneOfTwoElement>,
        mut f: impl FnMut(Mut<'_, Pod<N1>>),
    ) {
        let (OneOf2::A(node), OneOf2::A(props)) = (&mut elem.node, &mut elem.props) else {
            unreachable!()
        };
        f(PodMut::new(node, props, elem.was_removed));
    }

    fn with_downcast_b(
        elem: &mut Mut<'_, Self::OneOfTwoElement>,
        mut f: impl FnMut(Mut<'_, Pod<N2>>),
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

impl<E1: DomNode, E2: DomNode> DomNode for OneOf2<E1, E2> {
    type Props = OneOf2<E1::Props, E2::Props>;

    fn update_node(&self, props: &mut Self::Props) {
        match (self, props) {
            (OneOf2::A(el), OneOf2::A(props)) => el.update_node(props),
            (OneOf2::B(el), OneOf2::B(props)) => el.update_node(props),
            _ => unreachable!(),
        }
    }

    fn into_dyn_node(mut self, mut props: Self::Props) -> Pod<DynNode> {
        match (&mut self, &mut props) {
            (OneOf2::A(el), OneOf2::A(props)) => el.update_node(props),
            (OneOf2::B(el), OneOf2::B(props)) => el.update_node(props),
            _ => unreachable!(),
        }
        Pod::into_dyn_node(self, props)
    }
}
