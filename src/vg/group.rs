// Copyright 2023 The Xilem Authors.
// SPDX-License-Identifier: Apache-2.0

use vello::SceneBuilder;
use xilem_core::{Id, MessageResult, VecSplice};

use crate::{view::Cx, widget::ChangeFlags};

use super::{VgNode, VgPod, VgView, VgViewMarker, VgViewSequence};

/// Vector graphics group view.
///
/// It would absolutely make sense for this node to also take on a transform, and
/// perhaps other attributes such as group opacity. But for now, simple collection.
pub struct Group<VS> {
    children: VS,
}

pub struct GroupNode {
    children: Vec<VgPod>,
}

pub fn group<VS>(children: VS) -> Group<VS> {
    Group { children }
}

impl VgNode for GroupNode {
    fn paint(&mut self, builder: &mut SceneBuilder) {
        for child in &mut self.children {
            child.paint(builder);
        }
    }
}

impl<VS> VgViewMarker for Group<VS> {}

impl<T, VS> VgView<T> for Group<VS>
where
    VS: VgViewSequence<T, ()>,
{
    type State = VS::State;
    type Element = GroupNode;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let mut children = vec![];
        let (id, state) = cx.with_new_id(|cx| self.children.build(cx, &mut children));
        let node = GroupNode { children };
        (id, state, node)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        let mut scratch = vec![];
        let mut splice = VecSplice::new(&mut element.children, &mut scratch);
        cx.with_id(*id, |cx| {
            self.children
                .rebuild(cx, &prev.children, state, &mut splice)
        })
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> MessageResult<()> {
        self.children.message(id_path, state, message, app_state)
    }
}
