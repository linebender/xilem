// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Implementation of the View trait for various kurbo shapes.

use crate::{
    core::{MessageResult, Mut, OrphanView, ViewId},
    modifiers::Attributes,
    DynMessage, Pod, ViewCtx, With, SVG_NS,
};
use peniko::kurbo::{BezPath, Circle, Line, Rect};

fn create_element<R>(
    name: &str,
    ctx: &mut ViewCtx,
    attr_size_hint: usize,
    f: impl FnOnce(Pod<web_sys::Element>) -> R,
) -> R {
    ctx.with_size_hint::<Attributes, _>(attr_size_hint, |ctx| {
        let element = if ctx.is_hydrating() {
            Pod::hydrate_element_with_ctx(Vec::new(), ctx.hydrate_node().unwrap(), ctx)
        } else {
            Pod::new_element_with_ctx(Vec::new(), SVG_NS, name, ctx)
        };
        f(element)
    })
}

impl<State: 'static, Action: 'static> OrphanView<Line, State, Action, DynMessage> for ViewCtx {
    type OrphanViewState = ();
    type OrphanElement = Pod<web_sys::SvgLineElement>;

    fn orphan_build(
        view: &Line,
        ctx: &mut ViewCtx,
    ) -> (Self::OrphanElement, Self::OrphanViewState) {
        create_element("line", ctx, 4, |element| {
            let mut element: Self::OrphanElement = element.into();
            let attrs: &mut Attributes = element.modifier();
            attrs.push(("x1", view.p0.x));
            attrs.push(("y1", view.p0.y));
            attrs.push(("x2", view.p1.x));
            attrs.push(("y2", view.p1.y));
            (element, ())
        })
    }

    fn orphan_rebuild(
        new: &Line,
        prev: &Line,
        (): &mut Self::OrphanViewState,
        _ctx: &mut ViewCtx,
        element: Mut<Self::OrphanElement>,
    ) {
        Attributes::rebuild(element, 4, |mut element| {
            let attrs: &mut Attributes = element.modifier();
            attrs.update_with_same_key("x1", &prev.p0.x, &new.p0.x);
            attrs.update_with_same_key("y1", &prev.p0.y, &new.p0.y);
            attrs.update_with_same_key("x2", &prev.p1.x, &new.p1.x);
            attrs.update_with_same_key("y2", &prev.p1.y, &new.p1.y);
        });
    }

    fn orphan_teardown(
        _view: &Line,
        (): &mut Self::OrphanViewState,
        _ctx: &mut ViewCtx,
        _element: Mut<Self::OrphanElement>,
    ) {
    }

    fn orphan_message(
        _view: &Line,
        (): &mut Self::OrphanViewState,
        _id_path: &[ViewId],
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
        create_element("rect", ctx, 4, |element| {
            let mut element: Self::OrphanElement = element.into();
            let attrs: &mut Attributes = element.modifier();
            attrs.push(("x", view.x0));
            attrs.push(("y", view.y0));
            attrs.push(("width", view.width()));
            attrs.push(("height", view.height()));
            (element, ())
        })
    }

    fn orphan_rebuild(
        new: &Rect,
        prev: &Rect,
        (): &mut Self::OrphanViewState,
        _ctx: &mut ViewCtx,
        element: Mut<Self::OrphanElement>,
    ) {
        Attributes::rebuild(element, 4, |mut element| {
            let attrs: &mut Attributes = element.modifier();
            attrs.update_with_same_key("x", &prev.x0, &new.x0);
            attrs.update_with_same_key("y", &prev.y0, &new.y0);
            attrs.update_with_same_key("width", &prev.width(), &new.width());
            attrs.update_with_same_key("height", &prev.height(), &new.height());
        });
    }

    fn orphan_teardown(
        _view: &Rect,
        (): &mut Self::OrphanViewState,
        _ctx: &mut ViewCtx,
        _element: Mut<Self::OrphanElement>,
    ) {
    }

    fn orphan_message(
        _view: &Rect,
        (): &mut Self::OrphanViewState,
        _id_path: &[ViewId],
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
        create_element("circle", ctx, 3, |element| {
            let mut element: Self::OrphanElement = element.into();
            let attrs: &mut Attributes = element.modifier();
            attrs.push(("cx", view.center.x));
            attrs.push(("cy", view.center.y));
            attrs.push(("r", view.radius));
            (element, ())
        })
    }

    fn orphan_rebuild(
        new: &Circle,
        prev: &Circle,
        (): &mut Self::OrphanViewState,
        _ctx: &mut ViewCtx,
        element: Mut<Self::OrphanElement>,
    ) {
        Attributes::rebuild(element, 3, |mut element| {
            let attrs: &mut Attributes = element.modifier();
            attrs.update_with_same_key("cx", &prev.center.x, &new.center.x);
            attrs.update_with_same_key("cy", &prev.center.y, &new.center.y);
            attrs.update_with_same_key("height", &prev.radius, &new.radius);
        });
    }

    fn orphan_teardown(
        _view: &Circle,
        (): &mut Self::OrphanViewState,
        _ctx: &mut ViewCtx,
        _element: Mut<Self::OrphanElement>,
    ) {
    }

    fn orphan_message(
        _view: &Circle,
        (): &mut Self::OrphanViewState,
        _id_path: &[ViewId],
        message: DynMessage,
        _app_state: &mut State,
    ) -> MessageResult<Action, DynMessage> {
        MessageResult::Stale(message)
    }
}

impl<State: 'static, Action: 'static> OrphanView<BezPath, State, Action, DynMessage> for ViewCtx {
    type OrphanViewState = ();
    type OrphanElement = Pod<web_sys::SvgPathElement>;

    fn orphan_build(
        view: &BezPath,
        ctx: &mut ViewCtx,
    ) -> (Self::OrphanElement, Self::OrphanViewState) {
        create_element("path", ctx, 1, |element| {
            let mut element: Self::OrphanElement = element.into();
            let attrs: &mut Attributes = element.modifier();
            attrs.push(("path", view.to_svg()));
            (element, ())
        })
    }

    fn orphan_rebuild(
        new: &BezPath,
        prev: &BezPath,
        (): &mut Self::OrphanViewState,
        _ctx: &mut ViewCtx,
        element: Mut<Self::OrphanElement>,
    ) {
        Attributes::rebuild(element, 1, |mut element| {
            let attrs: &mut Attributes = element.modifier();
            if attrs.was_recreated() {
                attrs.push(("path", new.to_svg()));
            } else if new != prev {
                attrs.mutate(|m| *m = ("path", new.to_svg()).into());
            } else {
                attrs.skip(1);
            }
        });
    }

    fn orphan_teardown(
        _view: &BezPath,
        _view_state: &mut Self::OrphanViewState,
        _ctx: &mut ViewCtx,
        _element: Mut<Self::OrphanElement>,
    ) {
    }

    fn orphan_message(
        _view: &BezPath,
        _view_state: &mut Self::OrphanViewState,
        _id_path: &[ViewId],
        message: DynMessage,
        _app_state: &mut State,
    ) -> MessageResult<Action, DynMessage> {
        MessageResult::Stale(message)
    }
}

// // TODO: RoundedRect
