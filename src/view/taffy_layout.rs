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

use vello::peniko::Color;

use crate::view::{Id, TreeStructureSplice, ViewMarker, ViewSequence};
use crate::widget::{self, ChangeFlags};
use crate::MessageResult;

use super::{Cx, View};

/// `TaffyLayout` is a container view which does layout for the specified `ViewSequence`.
///
/// Children are positioned according to the Block, Flexbox or CSS Grid algorithm, depending
/// on the display style set. If the children are themselves instances of `TaffyLayout`, then
/// they can set styles to control how they placed, sized, and aligned.
pub struct TaffyLayout<T, A, VT: ViewSequence<T, A>> {
    children: VT,
    style: taffy::Style,
    background_color: Option<Color>,
    phantom: PhantomData<fn() -> (T, A)>,
}

/// Creates a Flexbox Column [`TaffyLayout`].
pub fn flex_column<T, A, VT: ViewSequence<T, A>>(children: VT) -> TaffyLayout<T, A, VT> {
    TaffyLayout::new_flex(children, taffy::FlexDirection::Column)
}

/// Creates a Flexbox Row [`TaffyLayout`].
pub fn flex_row<T, A, VT: ViewSequence<T, A>>(children: VT) -> TaffyLayout<T, A, VT> {
    TaffyLayout::new_flex(children, taffy::FlexDirection::Row)
}

/// Creates a Block layout [`TaffyLayout`].
pub fn div<T, A, VT: ViewSequence<T, A>>(children: VT) -> TaffyLayout<T, A, VT> {
    TaffyLayout::new(children, taffy::Display::Block)
}

/// Creates a CSS Grid [`TaffyLayout`].
pub fn grid<T, A, VT: ViewSequence<T, A>>(children: VT) -> TaffyLayout<T, A, VT> {
    TaffyLayout::new(children, taffy::Display::Grid)
}

impl<T, A, VT: ViewSequence<T, A>> TaffyLayout<T, A, VT> {
    pub fn new(children: VT, display: taffy::Display) -> Self {
        let phantom = Default::default();
        TaffyLayout {
            children,
            style: taffy::Style {
                display,
                ..Default::default()
            },
            background_color: None,
            phantom,
        }
    }

    pub fn new_flex(children: VT, flex_direction: taffy::FlexDirection) -> Self {
        let phantom = Default::default();
        let display = taffy::Display::Flex;
        TaffyLayout {
            children,
            style: taffy::Style {
                display,
                flex_direction,
                ..Default::default()
            },
            background_color: None,
            phantom,
        }
    }

    pub fn with_style(mut self, style_modifier: impl FnOnce(&mut taffy::Style)) -> Self {
        style_modifier(&mut self.style);
        self
    }

    pub fn with_background_color(mut self, color: impl Into<Color>) -> Self {
        self.background_color = Some(color.into());
        self
    }
}

impl<T, A, VT: ViewSequence<T, A>> ViewMarker for TaffyLayout<T, A, VT> {}

impl<T, A, VT: ViewSequence<T, A>> View<T, A> for TaffyLayout<T, A, VT> {
    type State = VT::State;

    type Element = widget::TaffyLayout;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let mut elements = vec![];
        let mut scratch = vec![];
        let mut splice = TreeStructureSplice::new(&mut elements, &mut scratch);
        let (id, state) = cx.with_new_id(|cx| self.children.build(cx, &mut splice));
        let column = widget::TaffyLayout::new(elements, self.style.clone(), self.background_color);
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
        let mut splice = TreeStructureSplice::new(&mut element.children, &mut scratch);
        let mut flags = cx.with_id(*id, |cx| {
            self.children
                .rebuild(cx, &prev.children, state, &mut splice)
        });

        if self.background_color != prev.background_color {
            element.background_color = self.background_color;
            flags |= ChangeFlags::PAINT;
        }

        if self.style != prev.style {
            element.style = self.style.clone();
            flags |= ChangeFlags::LAYOUT | ChangeFlags::PAINT;
        }

        // Clear layout cache if the layout ChangeFlag is set
        if flags.contains(ChangeFlags::LAYOUT) || flags.contains(ChangeFlags::TREE) {
            element.cache.clear();
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
