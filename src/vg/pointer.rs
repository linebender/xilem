// Copyright 2023 The Xilem Authors.
// SPDX-License-Identifier: Apache-2.0

//! Wiring up of pointer (mouse) events.
//!
//! Note: this is just a stub at this point. It needs to be expanded to
//! create a PointerNode, and the VgNode trait also needs to be expanded
//! to handle the infrastructure for hot state, etc.

use std::any::Any;

use xilem_core::{Id, IdPath, MessageResult};

use crate::{view::Cx, widget::ChangeFlags};

use super::{VgPod, VgView, VgViewMarker};

pub struct Pointer<V, F> {
    child: V,
    callback: F,
}

pub struct PointerNode {
    id_path: IdPath,
    child: VgPod,
}

// Note: these two are the same as xilemsvg and we might move them to a
// common source of truth.

#[derive(Debug)]
pub enum PointerMsg {
    Down(PointerDetails),
    Move(PointerDetails),
    Up(PointerDetails),
}

#[derive(Debug)]
pub struct PointerDetails {
    pub id: i32,
    pub button: i16,
    pub x: f64,
    pub y: f64,
}

impl<V, F> VgViewMarker for Pointer<V, F> {}

impl<T, F: Fn(&mut T, PointerMsg) + Send, V: VgView<T>> VgView<T> for Pointer<V, F> {
    type State = V::State;
    type Element = V::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        self.child.build(cx)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        self.child.rebuild(cx, &prev.child, id, state, element)
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn Any>,
        app_state: &mut T,
    ) -> MessageResult<()> {
        match message.downcast() {
            Ok(msg) => {
                (self.callback)(app_state, *msg);
                MessageResult::Action(())
            }
            Err(message) => self.child.message(id_path, state, message, app_state),
        }
    }
}
