// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    core::{MessageResult, Mut, View, ViewElement, ViewId, ViewMarker},
    modifiers::{AttributeModifier, Attributes, Modifier, With},
    DomView, DynMessage, ViewCtx,
};
use peniko::{kurbo, Brush};
use std::fmt::Write as _;
use std::marker::PhantomData;

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
    style: kurbo::Stroke,
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
    style: kurbo::Stroke,
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

fn opacity_attr_modifier(attr: &'static str, brush: &Brush) -> AttributeModifier {
    let opacity = match brush {
        Brush::Solid(color) if color.a != u8::MAX => Some(color.a as f64 / 255.0),
        _ => None,
    };

    (attr, opacity).into()
}

impl<V, State, Action> ViewMarker for Fill<V, State, Action> {}
impl<State, Action, V> View<State, Action, ViewCtx, DynMessage> for Fill<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: DomView<State, Action, Element: With<Attributes>>,
    for<'a> <V::Element as ViewElement>::Mut<'a>: With<Attributes>,
{
    type ViewState = V::ViewState;
    type Element = V::Element;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (mut element, state) =
            ctx.with_size_hint::<Attributes, _>(2, |ctx| self.child.build(ctx));
        let mut attrs = element.modifier();
        Attributes::push(&mut attrs, ("fill", brush_to_string(&self.brush)));
        Attributes::push(
            &mut attrs,
            opacity_attr_modifier("fill-opacity", &self.brush),
        );
        (element, state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<Self::Element>,
    ) {
        Attributes::rebuild(element, 2, |mut element| {
            self.child
                .rebuild(&prev.child, view_state, ctx, element.reborrow_mut());
            let mut attrs = element.modifier();
            if attrs.flags.was_created() {
                Attributes::push(&mut attrs, ("fill", brush_to_string(&self.brush)));
                Attributes::push(
                    &mut attrs,
                    opacity_attr_modifier("fill-opacity", &self.brush),
                );
            } else if self.brush != prev.brush {
                Attributes::mutate(&mut attrs, |m| {
                    *m = ("fill", brush_to_string(&self.brush)).into();
                });
                Attributes::mutate(&mut attrs, |m| {
                    *m = opacity_attr_modifier("fill-opacity", &self.brush);
                });
            } else {
                Attributes::skip(&mut attrs, 2);
            }
        });
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<Self::Element>,
    ) {
        self.child.teardown(view_state, ctx, element);
    }

    fn message(
        &self,
        child_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action, DynMessage> {
        self.child.message(child_state, id_path, message, app_state)
    }
}

fn push_stroke_modifiers(
    mut attrs: Modifier<'_, Attributes>,
    stroke: &kurbo::Stroke,
    brush: &Brush,
) {
    let dash_pattern =
        (!stroke.dash_pattern.is_empty()).then(|| join(&mut stroke.dash_pattern.iter(), " "));
    Attributes::push(&mut attrs, ("stroke", brush_to_string(brush)));
    Attributes::push(&mut attrs, opacity_attr_modifier("stroke-opacity", brush));
    Attributes::push(&mut attrs, ("stroke-dasharray", dash_pattern));

    let dash_offset = (stroke.dash_offset != 0.0).then_some(stroke.dash_offset);
    Attributes::push(&mut attrs, ("stroke-dashoffset", dash_offset));
    Attributes::push(&mut attrs, ("stroke-width", stroke.width));
}

// This function is not inlined to avoid unnecessary monomorphization, which may result in a bigger binary.
fn update_stroke_modifiers(
    mut attrs: Modifier<'_, Attributes>,
    prev_stroke: &kurbo::Stroke,
    next_stroke: &kurbo::Stroke,
    prev_brush: &Brush,
    next_brush: &Brush,
) {
    if attrs.flags.was_created() {
        push_stroke_modifiers(attrs, next_stroke, next_brush);
    } else {
        if next_brush != prev_brush {
            Attributes::mutate(&mut attrs, |m| {
                *m = ("stroke", brush_to_string(next_brush)).into();
            });
            Attributes::mutate(&mut attrs, |m| {
                *m = opacity_attr_modifier("stroke-opacity", next_brush);
            });
        } else {
            Attributes::skip(&mut attrs, 2);
        }
        if next_stroke.dash_pattern != prev_stroke.dash_pattern {
            let dash_pattern = (!next_stroke.dash_pattern.is_empty())
                .then(|| join(&mut next_stroke.dash_pattern.iter(), " "));
            Attributes::mutate(&mut attrs, |m| {
                *m = ("stroke-dasharray", dash_pattern).into();
            });
        } else {
            Attributes::skip(&mut attrs, 1);
        }
        if next_stroke.dash_offset != prev_stroke.dash_offset {
            let dash_offset = (next_stroke.dash_offset != 0.0).then_some(next_stroke.dash_offset);
            Attributes::mutate(&mut attrs, |m| {
                *m = ("stroke-dashoffset", dash_offset).into();
            });
        } else {
            Attributes::skip(&mut attrs, 1);
        }
        if next_stroke.width != prev_stroke.width {
            Attributes::mutate(&mut attrs, |m| {
                *m = ("stroke-width", next_stroke.width).into();
            });
        } else {
            Attributes::skip(&mut attrs, 1);
        }
    }
}

impl<V, State, Action> ViewMarker for Stroke<V, State, Action> {}
impl<State, Action, V> View<State, Action, ViewCtx, DynMessage> for Stroke<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: DomView<State, Action, Element: With<Attributes>>,
    for<'a> <V::Element as ViewElement>::Mut<'a>: With<Attributes>,
{
    type ViewState = V::ViewState;
    type Element = V::Element;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (mut element, state) =
            ctx.with_size_hint::<Attributes, _>(5, |ctx| self.child.build(ctx));
        push_stroke_modifiers(element.modifier(), &self.style, &self.brush);
        (element, state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<Self::Element>,
    ) {
        Attributes::rebuild(element, 5, |mut element| {
            self.child
                .rebuild(&prev.child, view_state, ctx, element.reborrow_mut());
            update_stroke_modifiers(
                element.modifier(),
                &prev.style,
                &self.style,
                &prev.brush,
                &self.brush,
            );
        });
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<Self::Element>,
    ) {
        self.child.teardown(view_state, ctx, element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action, DynMessage> {
        self.child.message(view_state, id_path, message, app_state)
    }
}
