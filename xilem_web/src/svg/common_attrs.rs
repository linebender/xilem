// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;
use std::{borrow::Cow, fmt::Write as _};

use peniko::Brush;
use xilem_core::{MessageResult, Mut, View, ViewId, ViewMarker};

use crate::{
    attribute::{ElementWithAttributes, WithAttributes},
    DynMessage, IntoAttributeValue, ViewCtx,
};

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

/// Rather general join string function, might be reused somewhere else as well...
fn join(iter: &mut impl Iterator<Item: std::fmt::Display>, sep: &str) -> String {
    match iter.next() {
        None => String::new(),
        Some(first_elt) => {
            // estimate lower bound of capacity needed
            let (lower, _) = iter.size_hint();
            let mut result = String::with_capacity(sep.len() * lower);
            write!(&mut result, "{}", first_elt).unwrap();
            iter.for_each(|elt| {
                result.push_str(sep);
                write!(&mut result, "{}", elt).unwrap();
            });
            result
        }
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

fn add_opacity_to_element(brush: &Brush, element: &mut impl WithAttributes, attr: &'static str) {
    let opacity = match brush {
        Brush::Solid(color) if color.a != u8::MAX => Some(color.a as f64 / 255.0),
        _ => None,
    };
    element.set_attribute(attr.into(), opacity.into_attr_value());
}

impl<V, State, Action> ViewMarker for Fill<V, State, Action> {}
impl<State, Action, V> View<State, Action, ViewCtx, DynMessage> for Fill<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: View<State, Action, ViewCtx, DynMessage, Element: ElementWithAttributes>,
{
    type ViewState = (Cow<'static, str>, V::ViewState);
    type Element = V::Element;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (mut element, child_state) = self.child.build(ctx);
        let brush_svg_repr = Cow::from(brush_to_string(&self.brush));
        element.start_attribute_modifier();
        element.set_attribute("fill".into(), brush_svg_repr.clone().into_attr_value());
        add_opacity_to_element(&self.brush, &mut element, "fill-opacity");
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
        add_opacity_to_element(&self.brush, &mut element, "fill-opacity");
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
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action, DynMessage> {
        self.child.message(child_state, id_path, message, app_state)
    }
}

pub struct StrokeState<ChildState> {
    brush_svg_repr: Cow<'static, str>,
    stroke_dash_pattern_svg_repr: Option<Cow<'static, str>>,
    child_state: ChildState,
}

impl<V, State, Action> ViewMarker for Stroke<V, State, Action> {}
impl<State, Action, V> View<State, Action, ViewCtx, DynMessage> for Stroke<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: View<State, Action, ViewCtx, DynMessage, Element: ElementWithAttributes>,
{
    type ViewState = StrokeState<V::ViewState>;
    type Element = V::Element;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (mut element, child_state) = self.child.build(ctx);

        element.start_attribute_modifier();

        let brush_svg_repr = Cow::from(brush_to_string(&self.brush));
        element.set_attribute("stroke".into(), brush_svg_repr.clone().into_attr_value());
        let stroke_dash_pattern_svg_repr = (!self.style.dash_pattern.is_empty())
            .then(|| Cow::from(join(&mut self.style.dash_pattern.iter(), " ")));
        let dash_pattern = stroke_dash_pattern_svg_repr.clone().into_attr_value();
        element.set_attribute("stroke-dasharray".into(), dash_pattern);
        let dash_offset = (self.style.dash_offset != 0.0).then_some(self.style.dash_offset);
        element.set_attribute("stroke-dashoffset".into(), dash_offset.into_attr_value());
        element.set_attribute("stroke-width".into(), self.style.width.into_attr_value());
        add_opacity_to_element(&self.brush, &mut element, "stroke-opacity");

        element.end_attribute_modifier();
        (
            element,
            StrokeState {
                brush_svg_repr,
                stroke_dash_pattern_svg_repr,
                child_state,
            },
        )
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        StrokeState {
            brush_svg_repr,
            stroke_dash_pattern_svg_repr,
            child_state,
        }: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        element.start_attribute_modifier();

        let mut element = self.child.rebuild(&prev.child, child_state, ctx, element);

        if self.brush != prev.brush {
            *brush_svg_repr = Cow::from(brush_to_string(&self.brush));
        }
        element.set_attribute("stroke".into(), brush_svg_repr.clone().into_attr_value());
        if self.style.dash_pattern != prev.style.dash_pattern {
            *stroke_dash_pattern_svg_repr = (!self.style.dash_pattern.is_empty())
                .then(|| Cow::from(join(&mut self.style.dash_pattern.iter(), " ")));
        }
        let dash_pattern = stroke_dash_pattern_svg_repr.clone().into_attr_value();
        element.set_attribute("stroke-dasharray".into(), dash_pattern);
        let dash_offset = (self.style.dash_offset != 0.0).then_some(self.style.dash_offset);
        element.set_attribute("stroke-dashoffset".into(), dash_offset.into_attr_value());
        element.set_attribute("stroke-width".into(), self.style.width.into_attr_value());
        add_opacity_to_element(&self.brush, &mut element, "stroke-opacity");

        element.end_attribute_modifier();
        element
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
    ) {
        self.child
            .teardown(&mut view_state.child_state, ctx, element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action, DynMessage> {
        self.child
            .message(&mut view_state.child_state, id_path, message, app_state)
    }
}
