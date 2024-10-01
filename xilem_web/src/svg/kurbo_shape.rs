// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Implementation of the View trait for various kurbo shapes.

use peniko::kurbo::{BezPath, Circle, Line, Rect};
use xilem_core::{MessageResult, Mut, OrphanView};

use crate::{
    attribute::WithAttributes, AttributeValue, Attributes, DynMessage, IntoAttributeValue, Pod,
    ViewCtx, SVG_NS,
};

fn create_element(name: &str, ctx: &mut ViewCtx, attr_size_hint: usize) -> Pod<web_sys::Element> {
    ctx.add_modifier_size_hint::<Attributes>(attr_size_hint);
    #[cfg(feature = "hydration")]
    if ctx.is_hydrating() {
        Pod::hydrate_element_with_ctx(Vec::new(), ctx.hydrate_node().unwrap(), ctx)
    } else {
        Pod::new_element_with_ctx(Vec::new(), SVG_NS, name, ctx)
    }
    #[cfg(not(feature = "hydration"))]
    Pod::new_element_with_ctx(Vec::new(), SVG_NS, name, ctx)
}

impl<State: 'static, Action: 'static> OrphanView<Line, State, Action, DynMessage> for ViewCtx {
    type OrphanViewState = ();
    type OrphanElement = Pod<web_sys::SvgLineElement>;

    fn orphan_build(
        view: &Line,
        ctx: &mut ViewCtx,
    ) -> (Self::OrphanElement, Self::OrphanViewState) {
        let mut element: Self::OrphanElement = create_element("line", ctx, 4).into();
        element.set_attribute(&"x1".into(), &view.p0.x.into_attr_value());
        element.set_attribute(&"y1".into(), &view.p0.y.into_attr_value());
        element.set_attribute(&"x2".into(), &view.p1.x.into_attr_value());
        element.set_attribute(&"y2".into(), &view.p1.y.into_attr_value());
        element.mark_end_of_attribute_modifier();
        (element, ())
    }

    fn orphan_rebuild<'el>(
        new: &Line,
        _prev: &Line,
        (): &mut Self::OrphanViewState,
        _ctx: &mut ViewCtx,
        mut element: Mut<'el, Self::OrphanElement>,
    ) -> Mut<'el, Self::OrphanElement> {
        element.rebuild_attribute_modifier();
        element.set_attribute(&"x1".into(), &new.p0.x.into_attr_value());
        element.set_attribute(&"y1".into(), &new.p0.y.into_attr_value());
        element.set_attribute(&"x2".into(), &new.p1.x.into_attr_value());
        element.set_attribute(&"y2".into(), &new.p1.y.into_attr_value());
        element.mark_end_of_attribute_modifier();
        element
    }

    fn orphan_teardown(
        _view: &Line,
        (): &mut Self::OrphanViewState,
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Self::OrphanElement>,
    ) {
    }

    fn orphan_message(
        _view: &Line,
        (): &mut Self::OrphanViewState,
        _id_path: &[xilem_core::ViewId],
        message: DynMessage,
        _app_state: &mut State,
    ) -> MessageResult<Action, DynMessage> {
        MessageResult::Stale(message)
    }
}

impl<State: 'static, Action: 'static> OrphanView<Rect, State, Action, DynMessage> for ViewCtx {
    type OrphanViewState = ();
    type OrphanElement = Pod<web_sys::SvgRectElement>;

    fn orphan_build(
        view: &Rect,
        ctx: &mut ViewCtx,
    ) -> (Self::OrphanElement, Self::OrphanViewState) {
        let mut element: Self::OrphanElement = create_element("rect", ctx, 4).into();
        element.set_attribute(&"x".into(), &view.x0.into_attr_value());
        element.set_attribute(&"y".into(), &view.y0.into_attr_value());
        element.set_attribute(&"width".into(), &view.width().into_attr_value());
        element.set_attribute(&"height".into(), &view.height().into_attr_value());
        element.mark_end_of_attribute_modifier();
        (element, ())
    }

    fn orphan_rebuild<'el>(
        new: &Rect,
        _prev: &Rect,
        (): &mut Self::OrphanViewState,
        _ctx: &mut ViewCtx,
        mut element: Mut<'el, Self::OrphanElement>,
    ) -> Mut<'el, Self::OrphanElement> {
        element.rebuild_attribute_modifier();
        element.set_attribute(&"x".into(), &new.x0.into_attr_value());
        element.set_attribute(&"y".into(), &new.y0.into_attr_value());
        element.set_attribute(&"width".into(), &new.width().into_attr_value());
        element.set_attribute(&"height".into(), &new.height().into_attr_value());
        element.mark_end_of_attribute_modifier();
        element
    }

    fn orphan_teardown(
        _view: &Rect,
        (): &mut Self::OrphanViewState,
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Self::OrphanElement>,
    ) {
    }

    fn orphan_message(
        _view: &Rect,
        (): &mut Self::OrphanViewState,
        _id_path: &[xilem_core::ViewId],
        message: DynMessage,
        _app_state: &mut State,
    ) -> MessageResult<Action, DynMessage> {
        MessageResult::Stale(message)
    }
}

impl<State: 'static, Action: 'static> OrphanView<Circle, State, Action, DynMessage> for ViewCtx {
    type OrphanViewState = ();
    type OrphanElement = Pod<web_sys::SvgCircleElement>;

    fn orphan_build(
        view: &Circle,
        ctx: &mut ViewCtx,
    ) -> (Self::OrphanElement, Self::OrphanViewState) {
        let mut element: Self::OrphanElement = create_element("circle", ctx, 3).into();
        element.set_attribute(&"cx".into(), &view.center.x.into_attr_value());
        element.set_attribute(&"cy".into(), &view.center.y.into_attr_value());
        element.set_attribute(&"r".into(), &view.radius.into_attr_value());
        element.mark_end_of_attribute_modifier();
        (element, ())
    }

    fn orphan_rebuild<'el>(
        new: &Circle,
        _prev: &Circle,
        (): &mut Self::OrphanViewState,
        _ctx: &mut ViewCtx,
        mut element: Mut<'el, Self::OrphanElement>,
    ) -> Mut<'el, Self::OrphanElement> {
        element.rebuild_attribute_modifier();
        element.set_attribute(&"cx".into(), &new.center.x.into_attr_value());
        element.set_attribute(&"cy".into(), &new.center.y.into_attr_value());
        element.set_attribute(&"r".into(), &new.radius.into_attr_value());
        element.mark_end_of_attribute_modifier();
        element
    }

    fn orphan_teardown(
        _view: &Circle,
        (): &mut Self::OrphanViewState,
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Self::OrphanElement>,
    ) {
    }

    fn orphan_message(
        _view: &Circle,
        (): &mut Self::OrphanViewState,
        _id_path: &[xilem_core::ViewId],
        message: DynMessage,
        _app_state: &mut State,
    ) -> MessageResult<Action, DynMessage> {
        MessageResult::Stale(message)
    }
}

impl<State: 'static, Action: 'static> OrphanView<BezPath, State, Action, DynMessage> for ViewCtx {
    type OrphanViewState = Option<AttributeValue>;
    type OrphanElement = Pod<web_sys::SvgPathElement>;

    fn orphan_build(
        view: &BezPath,
        ctx: &mut ViewCtx,
    ) -> (Self::OrphanElement, Self::OrphanViewState) {
        let mut element: Self::OrphanElement = create_element("path", ctx, 1).into();
        let svg_repr = view.to_svg().into_attr_value();
        element.set_attribute(&"d".into(), &svg_repr);
        element.mark_end_of_attribute_modifier();
        (element, svg_repr)
    }

    fn orphan_rebuild<'el>(
        new: &BezPath,
        prev: &BezPath,
        svg_repr: &mut Self::OrphanViewState,
        _ctx: &mut ViewCtx,
        mut element: Mut<'el, Self::OrphanElement>,
    ) -> Mut<'el, Self::OrphanElement> {
        // slight optimization to avoid serialization/allocation
        if new != prev {
            *svg_repr = new.to_svg().into_attr_value();
        }
        element.rebuild_attribute_modifier();
        element.set_attribute(&"d".into(), svg_repr);
        element.mark_end_of_attribute_modifier();
        element
    }

    fn orphan_teardown(
        _view: &BezPath,
        _view_state: &mut Self::OrphanViewState,
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Self::OrphanElement>,
    ) {
    }

    fn orphan_message(
        _view: &BezPath,
        _view_state: &mut Self::OrphanViewState,
        _id_path: &[xilem_core::ViewId],
        message: DynMessage,
        _app_state: &mut State,
    ) -> MessageResult<Action, DynMessage> {
        MessageResult::Stale(message)
    }
}

// TODO: RoundedRect
