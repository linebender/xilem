// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Cow;
use std::marker::PhantomData;

use peniko::Brush;
use xilem_core::{MessageResult, Mut, View};

use crate::IntoAttributeValue;
use crate::{attribute::ElementWithAttributes, ViewCtx, WithAttributes};

pub struct Fill<V, State, Action> {
    child: V,
    // This could reasonably be static Cow also, but keep things simple
    brush: Brush,
    phantom: PhantomData<fn() -> (State, Action)>,
}

pub struct Stroke<V, State, Action> {
    child: V,
    // This could reasonably be static Cow also, but keep things simple
    brush: Brush,
    style: peniko::kurbo::Stroke,
    phantom: PhantomData<fn() -> (State, Action)>,
}

pub fn fill<State, Action, V>(child: V, brush: impl Into<Brush>) -> Fill<V, State, Action> {
    Fill {
        child,
        brush: brush.into(),
        phantom: Default::default(),
    }
}

pub fn stroke<State, Action, V>(
    child: V,
    brush: impl Into<Brush>,
    style: peniko::kurbo::Stroke,
) -> Stroke<V, State, Action> {
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

impl<State, Action, V: View<State, Action, ViewCtx>> View<State, Action, ViewCtx>
    for Fill<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: View<State, Action, ViewCtx, Element: ElementWithAttributes>,
{
    type ViewState = (Cow<'static, str>, V::ViewState);
    type Element = V::Element;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (mut element, child_state) = self.child.build(ctx);
        let brush_svg_repr = Cow::from(brush_to_string(&self.brush));
        element.start_attribute_modifier();
        element.set_attribute("fill".into(), brush_svg_repr.clone().into_attr_value());
        element.end_attribute_modifier();
        (element, (brush_svg_repr, child_state))
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        (brush_svg_repr, child_state): &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        element.start_attribute_modifier();
        let mut element = self.child.rebuild(&prev.child, child_state, ctx, element);
        if self.brush != prev.brush {
            *brush_svg_repr = Cow::from(brush_to_string(&self.brush));
        }
        element.set_attribute("fill".into(), brush_svg_repr.clone().into_attr_value());
        element.end_attribute_modifier();
        element
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
    ) {
        self.child.teardown(&mut view_state.1, ctx, element);
    }

    fn message(
        &self,
        (_, child_state): &mut Self::ViewState,
        id_path: &[xilem_core::ViewId],
        message: xilem_core::DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        self.child.message(child_state, id_path, message, app_state)
    }
}

impl<State, Action, V> View<State, Action, ViewCtx> for Stroke<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: View<State, Action, ViewCtx, Element: ElementWithAttributes>,
{
    type ViewState = (Cow<'static, str>, V::ViewState);
    type Element = V::Element;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (mut element, child_state) = self.child.build(ctx);
        let brush_svg_repr = Cow::from(brush_to_string(&self.brush));
        element.start_attribute_modifier();
        element.set_attribute("stroke".into(), brush_svg_repr.clone().into_attr_value());
        element.set_attribute("stroke-width".into(), self.style.width.into_attr_value());
        element.end_attribute_modifier();
        (element, (brush_svg_repr, child_state))
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        (brush_svg_repr, child_state): &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        element.start_attribute_modifier();
        let mut element = self.child.rebuild(&prev.child, child_state, ctx, element);
        if self.brush != prev.brush {
            *brush_svg_repr = Cow::from(brush_to_string(&self.brush));
        }
        element.set_attribute("stroke".into(), brush_svg_repr.clone().into_attr_value());
        element.set_attribute("stroke-width".into(), self.style.width.into_attr_value());
        element.end_attribute_modifier();
        element
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
    ) {
        self.child.teardown(&mut view_state.1, ctx, element);
    }

    fn message(
        &self,
        (_, child_state): &mut Self::ViewState,
        id_path: &[xilem_core::ViewId],
        message: xilem_core::DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        self.child.message(child_state, id_path, message, app_state)
    }
}
