// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Implementation of the View trait for various kurbo shapes.

use peniko::kurbo::{BezPath, Circle, Line, Rect};
use std::borrow::Cow;

use xilem_core::{Id, MessageResult};

use crate::{
    context::{ChangeFlags, Cx, HtmlProps},
    interfaces::sealed::Sealed,
    view::{View, ViewMarker},
    IntoAttributeValue, SVG_NS,
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
    type State = HtmlProps;
    type Element = web_sys::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        cx.add_attr_to_element(&"x1".into(), &self.p0.x.into_attr_value());
        cx.add_attr_to_element(&"y1".into(), &self.p0.y.into_attr_value());
        cx.add_attr_to_element(&"x2".into(), &self.p1.x.into_attr_value());
        cx.add_attr_to_element(&"y2".into(), &self.p1.y.into_attr_value());
        let (el, props) = cx.build_element(SVG_NS, "line");
        let id = Id::next();
        (id, props, el)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        _prev: &Self,
        _id: &mut Id,
        props: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        cx.add_attr_to_element(&"x1".into(), &self.p0.x.into_attr_value());
        cx.add_attr_to_element(&"y1".into(), &self.p0.y.into_attr_value());
        cx.add_attr_to_element(&"x2".into(), &self.p1.x.into_attr_value());
        cx.add_attr_to_element(&"y2".into(), &self.p1.y.into_attr_value());
        cx.rebuild_element(element, props)
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
    type State = HtmlProps;
    type Element = web_sys::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        cx.add_attr_to_element(&"x".into(), &self.x0.into_attr_value());
        cx.add_attr_to_element(&"y".into(), &self.y0.into_attr_value());
        let size = self.size();
        cx.add_attr_to_element(&"width".into(), &size.width.into_attr_value());
        cx.add_attr_to_element(&"height".into(), &size.height.into_attr_value());
        let (el, props) = cx.build_element(SVG_NS, "rect");
        let id = Id::next();
        (id, props, el)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        _prev: &Self,
        _id: &mut Id,
        props: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        cx.add_attr_to_element(&"x".into(), &self.x0.into_attr_value());
        cx.add_attr_to_element(&"y".into(), &self.y0.into_attr_value());
        let size = self.size();
        cx.add_attr_to_element(&"width".into(), &size.width.into_attr_value());
        cx.add_attr_to_element(&"height".into(), &size.height.into_attr_value());
        cx.rebuild_element(element, props)
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
    type State = HtmlProps;
    type Element = web_sys::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        cx.add_attr_to_element(&"cx".into(), &self.center.x.into_attr_value());
        cx.add_attr_to_element(&"cy".into(), &self.center.y.into_attr_value());
        cx.add_attr_to_element(&"r".into(), &self.radius.into_attr_value());
        let (el, props) = cx.build_element(SVG_NS, "circle");
        let id = Id::next();
        (id, props, el)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        _prev: &Self,
        _id: &mut Id,
        props: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        cx.add_attr_to_element(&"cx".into(), &self.center.x.into_attr_value());
        cx.add_attr_to_element(&"cy".into(), &self.center.y.into_attr_value());
        cx.add_attr_to_element(&"r".into(), &self.radius.into_attr_value());
        cx.rebuild_element(element, props)
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
    type State = (Cow<'static, str>, HtmlProps);
    type Element = web_sys::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let svg_repr = Cow::from(self.to_svg());
        cx.add_attr_to_element(&"d".into(), &svg_repr.clone().into_attr_value());
        let (el, props) = cx.build_element(SVG_NS, "path");
        let id = Id::next();
        (id, (svg_repr, props), el)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        _id: &mut Id,
        (svg_repr, props): &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        // slight optimization to avoid serialization/allocation
        if self != prev {
            *svg_repr = Cow::from(self.to_svg());
        }
        cx.add_attr_to_element(&"d".into(), &svg_repr.clone().into_attr_value());
        cx.rebuild_element(element, props)
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
