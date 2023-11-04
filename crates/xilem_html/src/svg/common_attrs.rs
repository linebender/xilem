// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Cow;
use std::{any::Any, marker::PhantomData};

use peniko::Brush;
use xilem_core::{Id, MessageResult};

use crate::interfaces::{
    Element, SvgCircleElement, SvgElement, SvgEllipseElement, SvgGeometryElement,
    SvgGraphicsElement, SvgPathElement, SvgPolygonElement, SvgPolylineElement, SvgRectElement,
    SvgTextContentElement, SvgTextElement, SvgTextPathElement, SvgTextPositioningElement,
    SvgtSpanElement, SvgLineElement,
};
use crate::IntoAttributeValue;
use crate::{
    context::{ChangeFlags, Cx},
    view::{View, ViewMarker},
};

pub struct Fill<V, T, A = ()> {
    child: V,
    // This could reasonably be static Cow also, but keep things simple
    brush: Brush,
    phantom: PhantomData<fn() -> (T, A)>,
}

pub struct Stroke<V, T, A = ()> {
    child: V,
    // This could reasonably be static Cow also, but keep things simple
    brush: Brush,
    style: peniko::kurbo::Stroke,
    phantom: PhantomData<fn() -> (T, A)>,
}

pub fn fill<T, A, V>(child: V, brush: impl Into<Brush>) -> Fill<V, T, A> {
    Fill {
        child,
        brush: brush.into(),
        phantom: Default::default(),
    }
}

pub fn stroke<T, A, V>(
    child: V,
    brush: impl Into<Brush>,
    style: peniko::kurbo::Stroke,
) -> Stroke<V, T, A> {
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

// manually implement interfaces, because multiple independent DOM interfaces use the View
impl<T, A, E: SvgGraphicsElement<T, A>> Element<T, A> for Fill<E, T, A> {}
impl<T, A, E: SvgGraphicsElement<T, A>> SvgElement<T, A> for Fill<E, T, A> {}
impl<T, A, E: SvgGraphicsElement<T, A>> SvgGraphicsElement<T, A> for Fill<E, T, A> {}
// descendants of SvgGeometryElement (with the exception of SvgLineElement)
impl<T, A, E: SvgGeometryElement<T, A>> SvgGeometryElement<T, A> for Fill<E, T, A> {}
impl<T, A, E: SvgCircleElement<T, A>> SvgCircleElement<T, A> for Fill<E, T, A> {}
impl<T, A, E: SvgEllipseElement<T, A>> SvgEllipseElement<T, A> for Fill<E, T, A> {}
impl<T, A, E: SvgPathElement<T, A>> SvgPathElement<T, A> for Fill<E, T, A> {}
impl<T, A, E: SvgPolygonElement<T, A>> SvgPolygonElement<T, A> for Fill<E, T, A> {}
impl<T, A, E: SvgPolylineElement<T, A>> SvgPolylineElement<T, A> for Fill<E, T, A> {}
impl<T, A, E: SvgRectElement<T, A>> SvgRectElement<T, A> for Fill<E, T, A> {}
// descendants of SvgTextContentElement
impl<T, A, E: SvgTextContentElement<T, A>> SvgTextContentElement<T, A> for Fill<E, T, A> {}
impl<T, A, E: SvgTextPathElement<T, A>> SvgTextPathElement<T, A> for Fill<E, T, A> {}
impl<T, A, E: SvgTextPositioningElement<T, A>> SvgTextPositioningElement<T, A> for Fill<E, T, A> {}
impl<T, A, E: SvgTextElement<T, A>> SvgTextElement<T, A> for Fill<E, T, A> {}
impl<T, A, E: SvgtSpanElement<T, A>> SvgtSpanElement<T, A> for Fill<E, T, A> {}

impl<T, A, V> ViewMarker for Fill<V, T, A> {}
impl<T, A, V> crate::interfaces::sealed::Sealed for Fill<V, T, A> {}

impl<T, A, V: View<T, A>> View<T, A> for Fill<V, T, A> {
    type State = (Cow<'static, str>, V::State);
    type Element = V::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let brush_svg_repr = Cow::from(brush_to_string(&self.brush));
        cx.add_new_attribute_to_current_element(
            &"fill".into(),
            &brush_svg_repr.clone().into_attribute_value(),
        );
        let (id, child_state, element) = self.child.build(cx);
        (id, (brush_svg_repr, child_state), element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        (brush_svg_repr, child_state): &mut Self::State,
        element: &mut V::Element,
    ) -> ChangeFlags {
        if self.brush != prev.brush {
            *brush_svg_repr = Cow::from(brush_to_string(&self.brush));
        }
        cx.add_new_attribute_to_current_element(
            &"fill".into(),
            &brush_svg_repr.clone().into_attribute_value(),
        );
        self.child
            .rebuild(cx, &prev.child, id, child_state, element)
    }

    fn message(
        &self,
        id_path: &[Id],
        (_, child_state): &mut Self::State,
        message: Box<dyn Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        self.child.message(id_path, child_state, message, app_state)
    }
}

// manually implement interfaces, because multiple independent DOM interfaces use the View
impl<T, A, E: SvgGraphicsElement<T, A>> Element<T, A> for Stroke<E, T, A> {}
impl<T, A, E: SvgGraphicsElement<T, A>> SvgElement<T, A> for Stroke<E, T, A> {}
impl<T, A, E: SvgGraphicsElement<T, A>> SvgGraphicsElement<T, A> for Stroke<E, T, A> {}
// descendants of SvgGeometryElement
impl<T, A, E: SvgGeometryElement<T, A>> SvgGeometryElement<T, A> for Stroke<E, T, A> {}
impl<T, A, E: SvgCircleElement<T, A>> SvgCircleElement<T, A> for Stroke<E, T, A> {}
impl<T, A, E: SvgEllipseElement<T, A>> SvgEllipseElement<T, A> for Stroke<E, T, A> {}
impl<T, A, E: SvgLineElement<T, A>> SvgLineElement<T, A> for Stroke<E, T, A> {}
impl<T, A, E: SvgPathElement<T, A>> SvgPathElement<T, A> for Stroke<E, T, A> {}
impl<T, A, E: SvgPolygonElement<T, A>> SvgPolygonElement<T, A> for Stroke<E, T, A> {}
impl<T, A, E: SvgPolylineElement<T, A>> SvgPolylineElement<T, A> for Stroke<E, T, A> {}
impl<T, A, E: SvgRectElement<T, A>> SvgRectElement<T, A> for Stroke<E, T, A> {}
// descendants of SvgTextContentElement
impl<T, A, E: SvgTextContentElement<T, A>> SvgTextContentElement<T, A> for Stroke<E, T, A> {}
impl<T, A, E: SvgTextPathElement<T, A>> SvgTextPathElement<T, A> for Stroke<E, T, A> {}
impl<T, A, E: SvgTextPositioningElement<T, A>> SvgTextPositioningElement<T, A> for Stroke<E, T, A> {}
impl<T, A, E: SvgTextElement<T, A>> SvgTextElement<T, A> for Stroke<E, T, A> {}
impl<T, A, E: SvgtSpanElement<T, A>> SvgtSpanElement<T, A> for Stroke<E, T, A> {}

impl<T, A, V> ViewMarker for Stroke<V, T, A> {}
impl<T, A, V> crate::interfaces::sealed::Sealed for Stroke<V, T, A> {}

impl<T, A, V: View<T, A>> View<T, A> for Stroke<V, T, A> {
    type State = (Cow<'static, str>, V::State);
    type Element = V::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let brush_svg_repr = Cow::from(brush_to_string(&self.brush));
        cx.add_new_attribute_to_current_element(
            &"stroke".into(),
            &brush_svg_repr.clone().into_attribute_value(),
        );
        cx.add_new_attribute_to_current_element(
            &"stroke-width".into(),
            &self.style.width.into_attribute_value(),
        );
        let (id, child_state, element) = self.child.build(cx);
        (id, (brush_svg_repr, child_state), element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        (brush_svg_repr, child_state): &mut Self::State,
        element: &mut V::Element,
    ) -> ChangeFlags {
        if self.brush != prev.brush {
            *brush_svg_repr = Cow::from(brush_to_string(&self.brush));
        }
        cx.add_new_attribute_to_current_element(
            &"stroke".into(),
            &brush_svg_repr.clone().into_attribute_value(),
        );
        cx.add_new_attribute_to_current_element(
            &"stroke-width".into(),
            &self.style.width.into_attribute_value(),
        );
        self.child
            .rebuild(cx, &prev.child, id, child_state, element)
    }

    fn message(
        &self,
        id_path: &[Id],
        (_, child_state): &mut Self::State,
        message: Box<dyn Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        self.child.message(id_path, child_state, message, app_state)
    }
}
