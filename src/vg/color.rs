// Copyright 2023 The Xilem Authors.
// SPDX-License-Identifier: Apache-2.0

//! Set color as View wrapper.
//!
//! There are a lot of potential ways to get color (and other styling information)
//! into vector graphics nodes, but we're going to to it here as a View wrapper
//! because that will generalize to setting CSS classes, and also support a fairly
//! parallel implementation with DOM nodes.

use vello::{peniko::Color, SceneBuilder};
use xilem_core::{Id, MessageResult, VecSplice};

use crate::{view::Cx, widget::ChangeFlags};

use super::{VgNode, VgPaintCx, VgPod, VgView, VgViewMarker, VgViewSequence};

pub struct ColorView<VS> {
    children: VS,
    color: Color,
}

pub struct ColorNode {
    children: Vec<VgPod>,
    color: Color,
}

pub fn color<VS>(children: VS, color: Color) -> ColorView<VS> {
    ColorView { children, color }
}

impl<VS> VgViewMarker for ColorView<VS> {}

impl<T, VS: VgViewSequence<T, ()>> VgView<T> for ColorView<VS> {
    type State = VS::State;
    type Element = ColorNode;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let mut children = vec![];
        let (id, state) = cx.with_new_id(|cx| self.children.build(cx, &mut children));
        let node = ColorNode {
            children,
            color: self.color,
        };
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
        let mut changed = cx.with_id(*id, |cx| {
            self.children
                .rebuild(cx, &prev.children, state, &mut splice)
        });
        if self.color != prev.color {
            changed |= element.set_color(self.color);
        }
        changed
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

impl VgNode for ColorNode {
    fn paint(&mut self, _cx: &VgPaintCx, builder: &mut SceneBuilder) {
        let child_cx = VgPaintCx { color: self.color };
        for child in &mut self.children {
            child.paint(&child_cx, builder);
        }
    }
}

impl ColorNode {
    fn set_color(&mut self, color: Color) -> ChangeFlags {
        self.color = color;
        ChangeFlags::PAINT
    }
}