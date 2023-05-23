// Copyright 2023 The Xilem Authors.
// SPDX-License-Identifier: Apache-2.0

use std::any::Any;

use glazier::kurbo::{Affine, Circle};
use vello::{
    peniko::{Color, Fill},
    SceneBuilder,
};
use xilem_core::{Id, MessageResult};

use crate::{view::Cx, widget::ChangeFlags};

use super::{VgNode, VgView, VgViewMarker};

pub struct CircleNode {
    circle: Circle,
}

impl VgNode for CircleNode {
    fn paint(&mut self, builder: &mut SceneBuilder) {
        // TODO: obviously we need a way to set this.
        let color = Color::rgb8(0, 0, 255);
        builder.fill(Fill::EvenOdd, Affine::IDENTITY, &color, None, &self.circle);
    }
}

impl VgViewMarker for Circle {}

impl<T> VgView<T> for Circle {
    type State = ();

    type Element = CircleNode;

    fn build(&self, _cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let id = Id::next();
        let element = CircleNode { circle: *self };
        (id, (), element)
    }

    fn rebuild(
        &self,
        _cx: &mut Cx,
        prev: &Self,
        _id: &mut Id,
        _state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        let mut changed = ChangeFlags::default();
        if self != prev {
            element.circle = *self;
            changed |= ChangeFlags::PAINT;
        }
        changed
    }

    fn message(
        &self,
        _id_path: &[Id],
        _state: &mut Self::State,
        message: Box<dyn Any>,
        _app_state: &mut T,
    ) -> MessageResult<()> {
        MessageResult::Stale(message)
    }
}
