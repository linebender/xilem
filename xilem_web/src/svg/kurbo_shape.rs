// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Implementation of the View trait for various kurbo shapes.

use peniko::kurbo::{BezPath, Circle, Line, Rect};
use std::borrow::Cow;

use xilem_core::{MessageResult, Mut, OrphanView};

use crate::{
    element::ElementProps,
    elements::{build_element, ElementState},
    IntoAttributeValue, Pod, ViewCtx, WithAttributes, SVG_NS,
};

impl<State, Action> OrphanView<Line, State, Action> for ViewCtx {
    type ViewState = ElementState<()>;
    type Element = Pod<web_sys::SvgLineElement, ElementProps>;

    fn build(view: &Line, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (mut element, state): (Self::Element, ElementState<()>) =
            build_element::<State, Action, _, _, _>(&(), "line", SVG_NS, ctx);
        element.start_attribute_modifier();
        element.set_attribute("x1".into(), view.p0.x.into_attr_value());
        element.set_attribute("y1".into(), view.p0.y.into_attr_value());
        element.set_attribute("x2".into(), view.p1.x.into_attr_value());
        element.set_attribute("y2".into(), view.p1.y.into_attr_value());
        element.end_attribute_modifier();
        (element, state)
    }

    fn rebuild<'el>(
        new: &Line,
        _prev: &Line,
        _state: &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        mut element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        element.start_attribute_modifier();
        element.set_attribute("x1".into(), new.p0.x.into_attr_value());
        element.set_attribute("y1".into(), new.p0.y.into_attr_value());
        element.set_attribute("x2".into(), new.p1.x.into_attr_value());
        element.set_attribute("y2".into(), new.p1.y.into_attr_value());
        element.end_attribute_modifier();
        element
    }

    fn teardown(
        _view: &Line,
        _view_state: &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Self::Element>,
    ) {
    }

    fn message(
        _view: &Line,
        _view_state: &mut Self::ViewState,
        _id_path: &[xilem_core::ViewId],
        message: xilem_core::DynMessage,
        _app_state: &mut State,
    ) -> MessageResult<Action> {
        MessageResult::Stale(message)
    }
}

impl<State, Action> OrphanView<Rect, State, Action> for ViewCtx {
    type ViewState = ElementState<()>;
    type Element = Pod<web_sys::SvgRectElement, ElementProps>;

    fn build(view: &Rect, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (mut element, state): (Self::Element, ElementState<()>) =
            build_element::<State, Action, _, _, _>(&(), "rect", SVG_NS, ctx);
        element.start_attribute_modifier();
        element.set_attribute("x".into(), view.x0.into_attr_value());
        element.set_attribute("y".into(), view.y0.into_attr_value());
        element.set_attribute("width".into(), view.width().into_attr_value());
        element.set_attribute("height".into(), view.height().into_attr_value());
        element.end_attribute_modifier();
        (element, state)
    }

    fn rebuild<'el>(
        new: &Rect,
        _prev: &Rect,
        _state: &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        mut element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        element.start_attribute_modifier();
        element.set_attribute("x".into(), new.x0.into_attr_value());
        element.set_attribute("y".into(), new.y0.into_attr_value());
        element.set_attribute("width".into(), new.width().into_attr_value());
        element.set_attribute("height".into(), new.height().into_attr_value());
        element.end_attribute_modifier();
        element
    }

    fn teardown(
        _view: &Rect,
        _view_state: &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Self::Element>,
    ) {
    }

    fn message(
        _view: &Rect,
        _view_state: &mut Self::ViewState,
        _id_path: &[xilem_core::ViewId],
        message: xilem_core::DynMessage,
        _app_state: &mut State,
    ) -> MessageResult<Action> {
        MessageResult::Stale(message)
    }
}

impl<State, Action> OrphanView<Circle, State, Action> for ViewCtx {
    type ViewState = ElementState<()>;
    type Element = Pod<web_sys::SvgCircleElement, ElementProps>;

    fn build(view: &Circle, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (mut element, state): (Self::Element, ElementState<()>) =
            build_element::<State, Action, _, _, _>(&(), "circle", SVG_NS, ctx);
        element.start_attribute_modifier();
        element.set_attribute("cx".into(), view.center.x.into_attr_value());
        element.set_attribute("cy".into(), view.center.y.into_attr_value());
        element.set_attribute("r".into(), view.radius.into_attr_value());
        element.end_attribute_modifier();
        (element, state)
    }

    fn rebuild<'el>(
        new: &Circle,
        _prev: &Circle,
        _state: &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        mut element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        element.start_attribute_modifier();
        element.set_attribute("cx".into(), new.center.x.into_attr_value());
        element.set_attribute("cy".into(), new.center.y.into_attr_value());
        element.set_attribute("r".into(), new.radius.into_attr_value());
        element.end_attribute_modifier();
        element
    }

    fn teardown(
        _view: &Circle,
        _view_state: &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Self::Element>,
    ) {
    }

    fn message(
        _view: &Circle,
        _view_state: &mut Self::ViewState,
        _id_path: &[xilem_core::ViewId],
        message: xilem_core::DynMessage,
        _app_state: &mut State,
    ) -> MessageResult<Action> {
        MessageResult::Stale(message)
    }
}

impl<State, Action> OrphanView<BezPath, State, Action> for ViewCtx {
    type ViewState = (Cow<'static, str>, ElementState<()>);
    type Element = Pod<web_sys::SvgPathElement, ElementProps>;

    fn build(view: &BezPath, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (mut element, state): (Self::Element, ElementState<()>) =
            build_element::<State, Action, _, _, _>(&(), "path", SVG_NS, ctx);
        let svg_repr = Cow::from(view.to_svg());
        element.start_attribute_modifier();
        element.set_attribute("d".into(), svg_repr.clone().into_attr_value());
        element.end_attribute_modifier();
        (element, (svg_repr, state))
    }

    fn rebuild<'el>(
        new: &BezPath,
        prev: &BezPath,
        (svg_repr, _): &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        mut element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        // slight optimization to avoid serialization/allocation
        if new != prev {
            *svg_repr = Cow::from(new.to_svg());
        }
        element.start_attribute_modifier();
        element.set_attribute("d".into(), svg_repr.clone().into_attr_value());
        element.end_attribute_modifier();
        element
    }

    fn teardown(
        _view: &BezPath,
        _view_state: &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Self::Element>,
    ) {
    }

    fn message(
        _view: &BezPath,
        _view_state: &mut Self::ViewState,
        _id_path: &[xilem_core::ViewId],
        message: xilem_core::DynMessage,
        _app_state: &mut State,
    ) -> MessageResult<Action> {
        MessageResult::Stale(message)
    }
}

// TODO: RoundedRect
