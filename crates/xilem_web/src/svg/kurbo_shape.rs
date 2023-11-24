// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

//! Implementation of the View trait for various kurbo shapes.

use peniko::kurbo::{BezPath, Circle, Line, Rect};
use std::borrow::Cow;

use xilem_core::{Id, MessageResult};

use crate::{
    context::{ChangeFlags, Cx},
    interfaces::sealed::Sealed,
    vecmap::VecMap,
    view::{View, ViewMarker},
    AttributeValue, IntoAttributeValue, SVG_NS,
};

macro_rules! generate_dom_interface_impl {
    ($dom_interface:ident, ($ty_name:ident)) => {
        impl<T, A> $crate::interfaces::$dom_interface<T, A> for $ty_name {}
    };
}

generate_dom_interface_impl!(SvgLineElement, (Line));
crate::interfaces::for_all_svg_line_element_ancestors!(generate_dom_interface_impl, (Line));

impl ViewMarker for Line {}
impl Sealed for Line {}

impl<T, A> View<T, A> for Line {
    type State = VecMap<Cow<'static, str>, AttributeValue>;
    type Element = web_sys::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        cx.add_new_attribute_to_current_element(&"x1".into(), &self.p0.x.into_attribute_value());
        cx.add_new_attribute_to_current_element(&"y1".into(), &self.p0.y.into_attribute_value());
        cx.add_new_attribute_to_current_element(&"x2".into(), &self.p1.x.into_attribute_value());
        cx.add_new_attribute_to_current_element(&"y2".into(), &self.p1.y.into_attribute_value());
        let (el, attributes) = cx.build_element(SVG_NS, "line");
        let id = Id::next();
        (id, attributes, el)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        _prev: &Self,
        _id: &mut Id,
        attributes: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        cx.add_new_attribute_to_current_element(&"x1".into(), &self.p0.x.into_attribute_value());
        cx.add_new_attribute_to_current_element(&"y1".into(), &self.p0.y.into_attribute_value());
        cx.add_new_attribute_to_current_element(&"x2".into(), &self.p1.x.into_attribute_value());
        cx.add_new_attribute_to_current_element(&"y2".into(), &self.p1.y.into_attribute_value());
        cx.rebuild_element(element, attributes)
    }

    fn message(
        &self,
        _id_path: &[Id],
        _state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        _app_state: &mut T,
    ) -> MessageResult<A> {
        MessageResult::Stale(message)
    }
}

generate_dom_interface_impl!(SvgRectElement, (Rect));
crate::interfaces::for_all_svg_rect_element_ancestors!(generate_dom_interface_impl, (Rect));

impl ViewMarker for Rect {}
impl Sealed for Rect {}

impl<T, A> View<T, A> for Rect {
    type State = VecMap<Cow<'static, str>, AttributeValue>;
    type Element = web_sys::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        cx.add_new_attribute_to_current_element(&"x".into(), &self.x0.into_attribute_value());
        cx.add_new_attribute_to_current_element(&"y".into(), &self.y0.into_attribute_value());
        let size = self.size();
        cx.add_new_attribute_to_current_element(
            &"width".into(),
            &size.width.into_attribute_value(),
        );
        cx.add_new_attribute_to_current_element(
            &"height".into(),
            &size.height.into_attribute_value(),
        );
        let (el, attributes) = cx.build_element(SVG_NS, "rect");
        let id = Id::next();
        (id, attributes, el)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        _prev: &Self,
        _id: &mut Id,
        attributes: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        cx.add_new_attribute_to_current_element(&"x".into(), &self.x0.into_attribute_value());
        cx.add_new_attribute_to_current_element(&"y".into(), &self.y0.into_attribute_value());
        let size = self.size();
        cx.add_new_attribute_to_current_element(
            &"width".into(),
            &size.width.into_attribute_value(),
        );
        cx.add_new_attribute_to_current_element(
            &"height".into(),
            &size.height.into_attribute_value(),
        );
        cx.rebuild_element(element, attributes)
    }

    fn message(
        &self,
        _id_path: &[Id],
        _state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        _app_state: &mut T,
    ) -> MessageResult<A> {
        MessageResult::Stale(message)
    }
}

generate_dom_interface_impl!(SvgCircleElement, (Circle));
crate::interfaces::for_all_svg_circle_element_ancestors!(generate_dom_interface_impl, (Circle));

impl ViewMarker for Circle {}
impl Sealed for Circle {}

impl<T, A> View<T, A> for Circle {
    type State = VecMap<Cow<'static, str>, AttributeValue>;
    type Element = web_sys::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        cx.add_new_attribute_to_current_element(
            &"cx".into(),
            &self.center.x.into_attribute_value(),
        );
        cx.add_new_attribute_to_current_element(
            &"cy".into(),
            &self.center.y.into_attribute_value(),
        );
        cx.add_new_attribute_to_current_element(&"r".into(), &self.radius.into_attribute_value());
        let (el, attributes) = cx.build_element(SVG_NS, "circle");
        let id = Id::next();
        (id, attributes, el)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        _prev: &Self,
        _id: &mut Id,
        attributes: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        cx.add_new_attribute_to_current_element(
            &"cx".into(),
            &self.center.x.into_attribute_value(),
        );
        cx.add_new_attribute_to_current_element(
            &"cy".into(),
            &self.center.y.into_attribute_value(),
        );
        cx.add_new_attribute_to_current_element(&"r".into(), &self.radius.into_attribute_value());
        cx.rebuild_element(element, attributes)
    }

    fn message(
        &self,
        _id_path: &[Id],
        _state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        _app_state: &mut T,
    ) -> MessageResult<A> {
        MessageResult::Stale(message)
    }
}

generate_dom_interface_impl!(SvgPathElement, (BezPath));
crate::interfaces::for_all_svg_path_element_ancestors!(generate_dom_interface_impl, (BezPath));

impl ViewMarker for BezPath {}
impl Sealed for BezPath {}

impl<T, A> View<T, A> for BezPath {
    type State = (Cow<'static, str>, VecMap<Cow<'static, str>, AttributeValue>);
    type Element = web_sys::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let svg_repr = Cow::from(self.to_svg());
        cx.add_new_attribute_to_current_element(
            &"d".into(),
            &svg_repr.clone().into_attribute_value(),
        );
        let (el, attributes) = cx.build_element(SVG_NS, "path");
        let id = Id::next();
        (id, (svg_repr, attributes), el)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        _id: &mut Id,
        (svg_repr, attributes): &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        // slight optimization to avoid serialization/allocation
        if self != prev {
            *svg_repr = Cow::from(self.to_svg());
        }
        cx.add_new_attribute_to_current_element(
            &"d".into(),
            &svg_repr.clone().into_attribute_value(),
        );
        cx.rebuild_element(element, attributes)
    }

    fn message(
        &self,
        _id_path: &[Id],
        _state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        _app_state: &mut T,
    ) -> MessageResult<A> {
        MessageResult::Stale(message)
    }
}

// TODO: RoundedRect
