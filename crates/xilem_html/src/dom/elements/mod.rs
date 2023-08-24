mod generated;
use std::marker::PhantomData;

pub use generated::*;

use wasm_bindgen::{JsCast, UnwrapThrowExt};
use xilem_core::{Id, MessageResult, VecSplice};

use crate::{
    vecmap::VecMap,
    view::{DomElement, DomNode},
    AttributeValue, ChangeFlags, Cx, Pod, View, ViewMarker, ViewSequence,
};

use super::interfaces::{Element, EventTarget, Node};

type CowStr = std::borrow::Cow<'static, str>;

/// The state associated with a HTML element `View`.
///
/// Stores handles to the child elements and any child state, as well as attributes and event listeners
pub struct ElementState<ViewSeqState> {
    pub(crate) children_states: ViewSeqState,
    pub(crate) attributes: VecMap<CowStr, AttributeValue>,
    pub(crate) child_elements: Vec<Pod>,
    pub(crate) scratch: Vec<Pod>,
}

pub struct CustomElement<El, Children = ()> {
    name: CowStr,
    children: Children,
    phantom: PhantomData<fn() -> El>,
}

impl<El, Children> ViewMarker for CustomElement<El, Children> {}

impl<T, A, El, Children> View<T, A> for CustomElement<El, Children>
where
    Children: ViewSequence<T, A>,
    El: DomElement + JsCast,
{
    type State = ElementState<Children::State>;

    type Element = El;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let el = cx.create_html_element(self.node_name());

        let mut child_elements = vec![];
        let (id, children_states) =
            cx.with_new_id(|cx| self.children.build(cx, &mut child_elements));

        for child in &child_elements {
            el.append_child(child.0.as_node_ref()).unwrap_throw();
        }

        // Set the id used internally to the `data-debugid` attribute.
        // This allows the user to see if an element has been re-created or only altered.
        #[cfg(debug_assertions)]
        el.set_attribute("data-debugid", &id.to_raw().to_string())
            .unwrap_throw();

        let el = el.dyn_into().unwrap_throw();
        let state = ElementState {
            children_states,
            child_elements,
            scratch: vec![],
            attributes: Default::default(),
        };
        (id, state, el)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        let mut changed = ChangeFlags::empty();

        // update tag name
        if prev.name != self.name {
            // recreate element
            let parent = element
                .as_element_ref()
                .parent_element()
                .expect_throw("this element was mounted and so should have a parent");
            parent.remove_child(element.as_node_ref()).unwrap_throw();
            let new_element = cx.create_html_element(&self.name);
            // TODO could this be combined with child updates?
            while element.as_element_ref().child_element_count() > 0 {
                new_element
                    .append_child(&element.as_element_ref().child_nodes().get(0).unwrap_throw())
                    .unwrap_throw();
            }
            *element = new_element.dyn_into().unwrap_throw();
            changed |= ChangeFlags::STRUCTURE;
        }


        let element = element.as_element_ref();

        cx.apply_attribute_changes(element, &mut state.attributes);

        // update children
        let mut splice = VecSplice::new(&mut state.child_elements, &mut state.scratch);
        changed |= cx.with_id(*id, |cx| {
            self.children
                .rebuild(cx, &prev.children, &mut state.children_states, &mut splice)
        });
        if changed.contains(ChangeFlags::STRUCTURE) {
            // This is crude and will result in more DOM traffic than needed.
            // The right thing to do is diff the new state of the children id
            // vector against the old, and derive DOM mutations from that.
            while let Some(child) = element.first_child() {
                element.remove_child(&child).unwrap_throw();
            }
            for child in &state.child_elements {
                element.append_child(child.0.as_node_ref()).unwrap_throw();
            }
            changed.remove(ChangeFlags::STRUCTURE);
        }
        changed
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        self.children
            .message(id_path, &mut state.children_states, message, app_state)
    }
}

impl<El, Children> EventTarget for CustomElement<El, Children> {}

impl<El, Children> Node for CustomElement<El, Children> {
    fn node_name(&self) -> &str {
        &self.name
    }
}

impl<T, A, El: DomElement + JsCast, Children: ViewSequence<T, A>> Element<T, A>
    for CustomElement<El, Children>
{
}
