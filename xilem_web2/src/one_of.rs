use xilem_core::{
    DynMessage, MessageResult, Mut, OneOf2, OneOf2Ctx, View, ViewId, ViewPathTracker,
};

use crate::{
    attribute::WithAttributes, class::WithClasses, AttributeValue, CowStr, DomNode, DynNode, Pod,
    PodMut, ViewCtx,
};

impl<N1: DomNode, N2: DomNode> OneOf2Ctx<Pod<N1>, Pod<N2>> for ViewCtx {
    type OneOf2Element = Pod<OneOf2<N1, N2>>;

    fn upcast_one_of_2_element(elem: OneOf2<Pod<N1>, Pod<N2>>) -> Self::OneOf2Element {
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

    fn rebuild_a<'a, State, Action, Context, V>(
        new: &V,
        prev: &V,
        view_state: &mut V::ViewState,
        ctx: &mut Context,
        mut elem: Mut<'a, Self::OneOf2Element>,
    ) -> Mut<'a, Self::OneOf2Element>
    where
        Context: ViewPathTracker,
        V: View<State, Action, Context, Element = Pod<N1>>,
    {
        let (OneOf2::A(node), OneOf2::A(props)) = (&mut elem.node, &mut elem.props) else {
            unreachable!()
        };
        let elem_a = PodMut {
            node,
            props,
            was_removed: elem.was_removed,
        };
        V::rebuild(new, prev, view_state, ctx, elem_a);
        elem
    }

    fn rebuild_b<'a, State, Action, Context, V>(
        new: &V,
        prev: &V,
        view_state: &mut V::ViewState,
        ctx: &mut Context,
        mut elem: Mut<'a, Self::OneOf2Element>,
    ) -> Mut<'a, Self::OneOf2Element>
    where
        Context: ViewPathTracker,
        V: View<State, Action, Context, Element = Pod<N2>>,
    {
        let (OneOf2::B(node), OneOf2::B(props)) = (&mut elem.node, &mut elem.props) else {
            unreachable!()
        };
        let elem_a = PodMut {
            node,
            props,
            was_removed: elem.was_removed,
        };
        new.rebuild(prev, view_state, ctx, elem_a);
        elem
    }

    fn update_one_of_2_element_mut(
        elem_mut: &mut Mut<'_, Self::OneOf2Element>,
        new_elem: OneOf2<Pod<N1>, Pod<N2>>,
    ) {
        (*elem_mut.node, *elem_mut.props) = match new_elem {
            OneOf2::A(e) => (OneOf2::A(e.node), OneOf2::A(e.props)),
            OneOf2::B(e) => (OneOf2::B(e.node), OneOf2::B(e.props)),
        };
    }

    fn teardown<State, Action, Context, V1, V2>(
        view: &OneOf2<V1, V2>,
        view_state: &mut OneOf2<V1::ViewState, V2::ViewState>,
        ctx: &mut Context,
        elem: &mut Mut<'_, Self::OneOf2Element>,
    ) where
        Context: ViewPathTracker,
        V1: View<State, Action, Context, Element = Pod<N1>>,
        V2: View<State, Action, Context, Element = Pod<N2>>,
    {
        match (view, view_state, &mut elem.node, &mut elem.props) {
            (OneOf2::A(view), OneOf2::A(state), OneOf2::A(node), OneOf2::A(props)) => {
                let pod_mut = PodMut {
                    node,
                    props,
                    was_removed: elem.was_removed,
                };
                view.teardown(state, ctx, pod_mut);
            }
            (OneOf2::B(view), OneOf2::B(state), OneOf2::B(node), OneOf2::B(props)) => {
                let pod_mut = PodMut {
                    node,
                    props,
                    was_removed: elem.was_removed,
                };
                view.teardown(state, ctx, pod_mut);
            }
            _ => unreachable!(),
        }
    }

    fn message<State, Action, Context, V1, V2>(
        view: &OneOf2<V1, V2>,
        view_state: &mut OneOf2<V1::ViewState, V2::ViewState>,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action>
    where
        Context: ViewPathTracker,
        V1: View<State, Action, Context, Element = Pod<N1>>,
        V2: View<State, Action, Context, Element = Pod<N2>>,
    {
        match (view, view_state) {
            (OneOf2::A(view), OneOf2::A(state)) => view.message(state, id_path, message, app_state),
            (OneOf2::B(view), OneOf2::B(state)) => view.message(state, id_path, message, app_state),
            _ => unreachable!(),
        }
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
