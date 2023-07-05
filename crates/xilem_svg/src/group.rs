// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

//! Group

use web_sys::Element;

use xilem_core::{Id, MessageResult, VecSplice};

use crate::{
    context::{ChangeFlags, Cx},
    view::{Pod, View, ViewMarker, ViewSequence},
};

pub struct Group<VS> {
    children: VS,
}

pub struct GroupState<S> {
    state: S,
    elements: Vec<Pod>,
}

pub fn group<VS>(children: VS) -> Group<VS> {
    Group { children }
}

impl<VS> ViewMarker for Group<VS> {}

impl<T, A, VS> View<T, A> for Group<VS>
where
    VS: ViewSequence<T, A>,
{
    type State = GroupState<VS::State>;
    type Element = web_sys::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Element) {
        let el = cx
            .document()
            .create_element_ns(Some("http://www.w3.org/2000/svg"), "g")
            .unwrap();
        let mut elements = vec![];
        let (id, state) = cx.with_new_id(|cx| self.children.build(cx, &mut elements));
        for child in &elements {
            el.append_child(child.0.as_element_ref()).unwrap();
        }
        let group_state = GroupState { state, elements };
        (id, group_state, el)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Element,
    ) -> ChangeFlags {
        let mut scratch = vec![];
        let mut splice = VecSplice::new(&mut state.elements, &mut scratch);
        let mut changed = cx.with_id(*id, |cx| {
            self.children
                .rebuild(cx, &prev.children, &mut state.state, &mut splice)
        });
        if changed.contains(ChangeFlags::STRUCTURE) {
            // This is crude and will result in more DOM traffic than needed.
            // The right thing to do is diff the new state of the children id
            // vector against the old, and derive DOM mutations from that.
            while let Some(child) = element.first_child() {
                _ = element.remove_child(&child);
            }
            for child in &state.elements {
                _ = element.append_child(child.0.as_element_ref());
            }
            // TODO: we may want to propagate that something changed
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
            .message(id_path, &mut state.state, message, app_state)
    }
}
