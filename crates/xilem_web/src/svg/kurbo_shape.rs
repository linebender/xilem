// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

//! Implementation of the View trait for various kurbo shapes.

use peniko::kurbo::{BezPath, Circle, Line, Rect};
use std::borrow::Cow;
use wasm_bindgen::{JsCast, UnwrapThrowExt};

use xilem_core::{Id, MessageResult};

use crate::{
    context::{ChangeFlags, Cx},
    elements::ElementProps,
    interfaces::ElementProps as _,
    vecmap::{vec_map, VecMap},
    view::{View, ViewMarker},
    AttributeValue, IntoAttributeValue, SVG_NS,
};

fn build_element<E: JsCast>(
    document: &web_sys::Document,
    name: &str,
    attributes: VecMap<Cow<'static, str>, AttributeValue>,
) -> (Id, ElementProps, E) {
    let el = document
        .create_element_ns(Some(SVG_NS), name)
        .expect_throw("could not create element");

    for (name, value) in &attributes {
        el.set_attribute(name, &value.serialize()).unwrap_throw();
    }

    (
        Id::next(),
        ElementProps {
            attributes,
            ..Default::default()
        },
        el.dyn_into().unwrap_throw(),
    )
}

macro_rules! generate_dom_interface_impl {
    ($dom_interface:ident, ($ty_name:ident)) => {
        impl<T, A> $crate::interfaces::$dom_interface<T, A> for $ty_name {}
    };
}

generate_dom_interface_impl!(SvgLineElement, (Line));
crate::interfaces::for_all_svg_line_element_ancestors!(generate_dom_interface_impl, (Line));

impl ViewMarker for Line {}

impl<T, A> View<T, A> for Line {
    type State = ElementProps;
    type Element = web_sys::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let attributes = vec_map![
            Cow::from("x1") => AttributeValue::F64(self.p0.x),
            Cow::from("y1") => AttributeValue::F64(self.p0.y),
            Cow::from("x1") => AttributeValue::F64(self.p1.x),
            Cow::from("y1") => AttributeValue::F64(self.p1.y),
        ];
        build_element(cx.document(), "line", attributes)
    }

    fn rebuild(
        &self,
        _cx: &mut Cx,
        _prev: &Self,
        _id: &mut Id,
        props: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        props.set_attribute(None, &"x1".into(), &self.p0.x.into_attr_value());
        props.set_attribute(None, &"y1".into(), &self.p0.y.into_attr_value());
        props.set_attribute(None, &"x2".into(), &self.p1.x.into_attr_value());
        props.set_attribute(None, &"y2".into(), &self.p1.y.into_attr_value());
        props.apply_changes(element)
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

impl<T, A> View<T, A> for Rect {
    type State = ElementProps;
    type Element = web_sys::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let size = self.size();

        let attributes = vec_map![
            Cow::from("x") => AttributeValue::F64(self.x0),
            Cow::from("y") => AttributeValue::F64(self.y0),
            Cow::from("width") => AttributeValue::F64(size.width),
            Cow::from("height") => AttributeValue::F64(size.height),
        ];
        build_element(cx.document(), "rect", attributes)
    }

    fn rebuild(
        &self,
        _cx: &mut Cx,
        _prev: &Self,
        _id: &mut Id,
        props: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        let size = self.size();
        props.set_attribute(None, &"x".into(), &self.x0.into_attr_value());
        props.set_attribute(None, &"y".into(), &self.y0.into_attr_value());
        props.set_attribute(None, &"width".into(), &size.width.into_attr_value());
        props.set_attribute(None, &"height".into(), &size.height.into_attr_value());
        props.apply_changes(element)
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

impl<T, A> View<T, A> for Circle {
    type State = ElementProps;
    type Element = web_sys::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let attributes = vec_map![
            Cow::from("cx") => AttributeValue::F64(self.center.x),
            Cow::from("cy") => AttributeValue::F64(self.center.y),
            Cow::from("r") => AttributeValue::F64(self.radius),
        ];
        build_element(cx.document(), "circle", attributes)
    }

    fn rebuild(
        &self,
        _cx: &mut Cx,
        _prev: &Self,
        _id: &mut Id,
        props: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        props.set_attribute(None, &"cx".into(), &self.center.x.into_attr_value());
        props.set_attribute(None, &"cy".into(), &self.center.y.into_attr_value());
        props.set_attribute(None, &"r".into(), &self.radius.into_attr_value());
        props.apply_changes(element)
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

pub struct BezPathState {
    path_value: AttributeValue,
    props: ElementProps,
}

impl crate::interfaces::ElementProps for BezPathState {
    fn set_attribute(
        &mut self,
        element: Option<&web_sys::Element>,
        name: &Cow<'static, str>,
        value: &Option<AttributeValue>,
    ) {
        self.props.set_attribute(element, name, value);
    }

    fn set_class(&mut self, element: Option<&web_sys::Element>, class: Cow<'static, str>) {
        self.props.set_class(element, class);
    }

    fn set_style(
        &mut self,
        element: Option<&web_sys::Element>,
        key: Cow<'static, str>,
        value: Cow<'static, str>,
    ) {
        self.props.set_style(element, key, value);
    }
}

impl ViewMarker for BezPath {}

impl<T, A> View<T, A> for BezPath {
    type State = BezPathState;
    type Element = web_sys::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let path_value = AttributeValue::String(Cow::from(self.to_svg()));
        let attributes = vec_map![ Cow::from("d") => path_value.clone() ];
        let (id, props, el) = build_element(cx.document(), "path", attributes);
        (id, BezPathState { path_value, props }, el)
    }

    fn rebuild(
        &self,
        _cx: &mut Cx,
        prev: &Self,
        _id: &mut Id,
        BezPathState { path_value, props }: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        // slight optimization to avoid serialization/allocation
        if self != prev {
            *path_value = AttributeValue::String(Cow::from(self.to_svg()));
        }
        props.set_attribute(None, &"d".into(), &Some(path_value.clone()));
        props.apply_changes(element)
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
