// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Implementation of the View trait for various kurbo shapes.

use crate::{
    core::{MessageResult, Mut, OrphanView, ViewId},
    modifiers::{Attributes, WithModifier},
    DynMessage, FromWithContext, Pod, ViewCtx, SVG_NS,
};
use peniko::kurbo::{BezPath, Circle, Line, Rect};

fn create_element<R>(
    name: &str,
    ctx: &mut ViewCtx,
    attr_size_hint: usize,
    f: impl FnOnce(Pod<web_sys::Element>, &mut ViewCtx) -> R,
) -> R {
    ctx.with_size_hint::<Attributes, _>(attr_size_hint, |ctx| {
        let element = if ctx.is_hydrating() {
            Pod::hydrate_element_with_ctx(Vec::new(), ctx)
        } else {
            Pod::new_element_with_ctx(Vec::new(), SVG_NS, name, ctx)
        };
        f(element, ctx)
    })
}

impl<State: 'static, Action: 'static> OrphanView<Line, State, Action, DynMessage> for ViewCtx {
    type OrphanViewState = ();
    type OrphanElement = Pod<web_sys::SvgLineElement>;

    fn orphan_build(
        view: &Line,
        ctx: &mut ViewCtx,
    ) -> (Self::OrphanElement, Self::OrphanViewState) {
        create_element("line", ctx, 4, |element, ctx| {
            let mut element = Self::OrphanElement::from_with_ctx(element, ctx);
            let attrs = &mut element.modifier();
            Attributes::push(attrs, ("x1", view.p0.x));
            Attributes::push(attrs, ("y1", view.p0.y));
            Attributes::push(attrs, ("x2", view.p1.x));
            Attributes::push(attrs, ("y2", view.p1.y));
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
            let attrs = &mut element.modifier();
            Attributes::update_with_same_key(attrs, "x1", &prev.p0.x, &new.p0.x);
            Attributes::update_with_same_key(attrs, "y1", &prev.p0.y, &new.p0.y);
            Attributes::update_with_same_key(attrs, "x2", &prev.p1.x, &new.p1.x);
            Attributes::update_with_same_key(attrs, "y2", &prev.p1.y, &new.p1.y);
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
        create_element("rect", ctx, 4, |element, ctx| {
            let mut element = Self::OrphanElement::from_with_ctx(element, ctx);
            let attrs = &mut element.modifier();
            Attributes::push(attrs, ("x", view.x0));
            Attributes::push(attrs, ("y", view.y0));
            Attributes::push(attrs, ("width", view.width()));
            Attributes::push(attrs, ("height", view.height()));
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
            let attrs = &mut element.modifier();
            Attributes::update_with_same_key(attrs, "x", &prev.x0, &new.x0);
            Attributes::update_with_same_key(attrs, "y", &prev.y0, &new.y0);
            Attributes::update_with_same_key(attrs, "width", &prev.width(), &new.width());
            Attributes::update_with_same_key(attrs, "height", &prev.height(), &new.height());
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
        create_element("circle", ctx, 3, |element, ctx| {
            let mut element = Self::OrphanElement::from_with_ctx(element, ctx);
            let attrs = &mut element.modifier();
            Attributes::push(attrs, ("cx", view.center.x));
            Attributes::push(attrs, ("cy", view.center.y));
            Attributes::push(attrs, ("r", view.radius));
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
            let attrs = &mut element.modifier();
            Attributes::update_with_same_key(attrs, "cx", &prev.center.x, &new.center.x);
            Attributes::update_with_same_key(attrs, "cy", &prev.center.y, &new.center.y);
            Attributes::update_with_same_key(attrs, "height", &prev.radius, &new.radius);
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
        create_element("path", ctx, 1, |element, ctx| {
            let mut element = Self::OrphanElement::from_with_ctx(element, ctx);
            let attrs = &mut element.modifier();
            Attributes::push(attrs, ("d", view.to_svg()));
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
            let attrs = &mut element.modifier();
            if attrs.flags.was_created() {
                Attributes::push(attrs, ("d", new.to_svg()));
            } else if new != prev {
                Attributes::mutate(attrs, |m| *m = ("d", new.to_svg()).into());
            } else {
                Attributes::skip(attrs, 1);
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

// TODO: RoundedRect
