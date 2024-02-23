// Copyright 2022 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{any::Any, marker::PhantomData};

use crate::geometry::Axis;
use crate::view::{Id, ViewMarker, ViewSequence};
use crate::widget::{self, ChangeFlags};
use crate::MessageResult;

use super::{Cx, TreeStructureTrackingSplice, View};

/// LinearLayout is a simple view which does layout for the specified ViewSequence.
///
/// Each Element is positioned on the specified Axis starting at the beginning with the given spacing
///
/// This View is only temporary is probably going to be replaced by something like Druid's Flex
/// widget.
pub struct LinearLayout<T, A, VT: ViewSequence<T, A>> {
    children: VT,
    spacing: f64,
    axis: Axis,
    phantom: PhantomData<fn() -> (T, A)>,
}

/// creates a vertical [`LinearLayout`].
pub fn v_stack<T, A, VT: ViewSequence<T, A>>(children: VT) -> LinearLayout<T, A, VT> {
    LinearLayout::new(children, Axis::Vertical)
}

/// creates a horizontal [`LinearLayout`].
pub fn h_stack<T, A, VT: ViewSequence<T, A>>(children: VT) -> LinearLayout<T, A, VT> {
    LinearLayout::new(children, Axis::Horizontal)
}

impl<T, A, VT: ViewSequence<T, A>> LinearLayout<T, A, VT> {
    pub fn new(children: VT, axis: Axis) -> Self {
        let phantom = Default::default();
        LinearLayout {
            children,
            phantom,
            spacing: 0.0,
            axis,
        }
    }

    pub fn with_spacing(mut self, spacing: f64) -> Self {
        self.spacing = spacing;
        self
    }
}

impl<T, A, VT: ViewSequence<T, A>> ViewMarker for LinearLayout<T, A, VT> {}

impl<T, A, VT: ViewSequence<T, A>> View<T, A> for LinearLayout<T, A, VT> {
    type State = VT::State;

    type Element = widget::LinearLayout;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let mut elements = vec![];
        let mut scratch = vec![];
        let mut tree_mutations = vec![];
        let mut splice =
            TreeStructureTrackingSplice::new(&mut elements, &mut scratch, &mut tree_mutations);
        let (id, state) = cx.with_new_id(|cx| self.children.build(cx, &mut splice));
        cx.mark_children_tree_structure_mutations(&mut tree_mutations);
        let column = widget::LinearLayout::new(elements, self.spacing, self.axis);
        (id, state, column)
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
        let mut tree_mutations = vec![]; // TODO(#160) could save some allocations by using View::State (scratch too)
        let mut splice = TreeStructureTrackingSplice::new(
            &mut element.children,
            &mut scratch,
            &mut tree_mutations,
        );
        let mut flags = cx.with_id(*id, |cx| {
            self.children
                .rebuild(cx, &prev.children, state, &mut splice)
        });
        cx.mark_children_tree_structure_mutations(&mut tree_mutations);

        if self.spacing != prev.spacing || self.axis != prev.axis {
            element.spacing = self.spacing;
            element.axis = self.axis;
            flags |= ChangeFlags::LAYOUT;
        }

        flags
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        event: Box<dyn Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        self.children.message(id_path, state, event, app_state)
    }
}
