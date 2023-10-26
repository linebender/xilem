// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

use std::{any::Any, marker::PhantomData};

use peniko::Brush;
use xilem_core::{Id, MessageResult};

use crate::{
    context::{ChangeFlags, Cx},
    view::{DomElement, View, ViewMarker},
};

pub struct Fill<T, V> {
    child: V,
    // This could reasonably be static Cow also, but keep things simple
    brush: Brush,
    phantom: PhantomData<T>,
}

pub struct Stroke<T, V> {
    child: V,
    // This could reasonably be static Cow also, but keep things simple
    brush: Brush,
    style: peniko::kurbo::Stroke,
    phantom: PhantomData<T>,
}

pub fn fill<T, V>(child: V, brush: impl Into<Brush>) -> Fill<T, V> {
    Fill {
        child,
        brush: brush.into(),
        phantom: Default::default(),
    }
}

pub fn stroke<T, V>(
    child: V,
    brush: impl Into<Brush>,
    style: peniko::kurbo::Stroke,
) -> Stroke<T, V> {
    Stroke {
        child,
        brush: brush.into(),
        style,
        phantom: Default::default(),
    }
}

fn brush_to_string(brush: &Brush) -> String {
    match brush {
        Brush::Solid(color) => {
            if color.a == 0 {
                "none".into()
            } else {
                format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b)
            }
        }
        _ => todo!("gradients not implemented"),
    }
}

impl<T, V> ViewMarker for Fill<T, V> {}

// TODO: make generic over A (probably requires Phantom)
impl<T, V: View<T>> View<T> for Fill<T, V> {
    type State = V::State;
    type Element = V::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, child_state, element) = self.child.build(cx);
        element
            .as_element_ref()
            .set_attribute("fill", &brush_to_string(&self.brush))
            .unwrap();
        (id, child_state, element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut V::Element,
    ) -> ChangeFlags {
        let prev_id = *id;
        let mut changed = self.child.rebuild(cx, &prev.child, id, state, element);
        if self.brush != prev.brush || prev_id != *id {
            element
                .as_element_ref()
                .set_attribute("fill", &brush_to_string(&self.brush))
                .unwrap();
            changed.insert(ChangeFlags::OTHER_CHANGE);
        }
        changed
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn Any>,
        app_state: &mut T,
    ) -> MessageResult<()> {
        self.child.message(id_path, state, message, app_state)
    }
}

impl<T, V> ViewMarker for Stroke<T, V> {}

// TODO: make generic over A (probably requires Phantom)
impl<T, V: View<T>> View<T> for Stroke<T, V> {
    type State = V::State;
    type Element = V::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, child_state, element) = self.child.build(cx);
        element
            .as_element_ref()
            .set_attribute("stroke", &brush_to_string(&self.brush))
            .unwrap();
        element
            .as_element_ref()
            .set_attribute("stroke-width", &format!("{}", self.style.width))
            .unwrap();
        (id, child_state, element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut V::Element,
    ) -> ChangeFlags {
        let prev_id = *id;
        let mut changed = self.child.rebuild(cx, &prev.child, id, state, element);
        if self.brush != prev.brush || prev_id != *id {
            element
                .as_element_ref()
                .set_attribute("fill", &brush_to_string(&self.brush))
                .unwrap();
            changed.insert(ChangeFlags::OTHER_CHANGE);
        }
        if self.style.width != prev.style.width || prev_id != *id {
            element
                .as_element_ref()
                .set_attribute("stroke-width", &format!("{}", self.style.width))
                .unwrap();
        }
        changed
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn Any>,
        app_state: &mut T,
    ) -> MessageResult<()> {
        self.child.message(id_path, state, message, app_state)
    }
}
