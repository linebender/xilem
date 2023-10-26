// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

//! Implementation of the View trait for various kurbo shapes.

use kurbo::{BezPath, Circle, Line, Rect};
use web_sys::Element;

use xilem_core::{Id, MessageResult};

use crate::{
    context::{ChangeFlags, Cx},
    view::{View, ViewMarker},
};

impl ViewMarker for Line {}

impl<T> View<T> for Line {
    type State = ();
    type Element = Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Element) {
        let el = cx
            .document()
            .create_element_ns(Some("http://www.w3.org/2000/svg"), "line")
            .unwrap();
        el.set_attribute("x1", &format!("{}", self.p0.x)).unwrap();
        el.set_attribute("y1", &format!("{}", self.p0.y)).unwrap();
        el.set_attribute("x2", &format!("{}", self.p1.x)).unwrap();
        el.set_attribute("y2", &format!("{}", self.p1.y)).unwrap();
        let id = Id::next();
        (id, (), el)
    }

    fn rebuild(
        &self,
        _cx: &mut Cx,
        prev: &Self,
        _id: &mut Id,
        _state: &mut Self::State,
        element: &mut Element,
    ) -> ChangeFlags {
        let mut is_changed = ChangeFlags::default();
        if self.p0.x != prev.p0.x {
            element
                .set_attribute("x1", &format!("{}", self.p0.x))
                .unwrap();
            is_changed |= ChangeFlags::OTHER_CHANGE;
        }
        if self.p0.y != prev.p0.y {
            element
                .set_attribute("y1", &format!("{}", self.p0.y))
                .unwrap();
            is_changed |= ChangeFlags::OTHER_CHANGE;
        }
        if self.p1.x != prev.p1.x {
            element
                .set_attribute("x2", &format!("{}", self.p1.x))
                .unwrap();
            is_changed |= ChangeFlags::OTHER_CHANGE;
        }
        if self.p1.y != prev.p1.y {
            element
                .set_attribute("y2", &format!("{}", self.p1.y))
                .unwrap();
            is_changed |= ChangeFlags::OTHER_CHANGE;
        }
        is_changed
    }

    fn message(
        &self,
        _id_path: &[Id],
        _state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        _app_state: &mut T,
    ) -> MessageResult<()> {
        MessageResult::Stale(message)
    }
}

impl ViewMarker for Rect {}

impl<T> View<T> for Rect {
    type State = ();
    type Element = Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Element) {
        let el = cx
            .document()
            .create_element_ns(Some("http://www.w3.org/2000/svg"), "rect")
            .unwrap();
        el.set_attribute("x", &format!("{}", self.x0)).unwrap();
        el.set_attribute("y", &format!("{}", self.y0)).unwrap();
        let size = self.size();
        el.set_attribute("width", &format!("{}", size.width))
            .unwrap();
        el.set_attribute("height", &format!("{}", size.height))
            .unwrap();
        let id = Id::next();
        (id, (), el)
    }

    fn rebuild(
        &self,
        _cx: &mut Cx,
        prev: &Self,
        _id: &mut Id,
        _state: &mut Self::State,
        element: &mut Element,
    ) -> ChangeFlags {
        let mut is_changed = ChangeFlags::default();
        if self.x0 != prev.x0 {
            element.set_attribute("x", &format!("{}", self.x0)).unwrap();
            is_changed |= ChangeFlags::OTHER_CHANGE;
        }
        if self.y0 != prev.y0 {
            element.set_attribute("y", &format!("{}", self.y0)).unwrap();
            is_changed |= ChangeFlags::OTHER_CHANGE;
        }
        let size = self.size();
        let prev_size = prev.size();
        if size.width != prev_size.width {
            element
                .set_attribute("width", &format!("{}", size.width))
                .unwrap();
            is_changed |= ChangeFlags::OTHER_CHANGE;
        }
        if size.height != prev_size.height {
            element
                .set_attribute("height", &format!("{}", size.height))
                .unwrap();
            is_changed |= ChangeFlags::OTHER_CHANGE;
        }
        is_changed
    }

    fn message(
        &self,
        _id_path: &[Id],
        _state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        _app_state: &mut T,
    ) -> MessageResult<()> {
        MessageResult::Stale(message)
    }
}

impl ViewMarker for Circle {}

impl<T> View<T> for Circle {
    type State = ();
    type Element = Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Element) {
        let el = cx
            .document()
            .create_element_ns(Some("http://www.w3.org/2000/svg"), "circle")
            .unwrap();
        el.set_attribute("cx", &format!("{}", self.center.x))
            .unwrap();
        el.set_attribute("cy", &format!("{}", self.center.y))
            .unwrap();
        el.set_attribute("r", &format!("{}", self.radius)).unwrap();
        let id = Id::next();
        (id, (), el)
    }

    fn rebuild(
        &self,
        _cx: &mut Cx,
        prev: &Self,
        _id: &mut Id,
        _state: &mut Self::State,
        element: &mut Element,
    ) -> ChangeFlags {
        let mut is_changed = ChangeFlags::default();
        if self.center.x != prev.center.x {
            element
                .set_attribute("cx", &format!("{}", self.center.x))
                .unwrap();
            is_changed |= ChangeFlags::OTHER_CHANGE;
        }
        if self.center.y != prev.center.y {
            element
                .set_attribute("cy", &format!("{}", self.center.y))
                .unwrap();
            is_changed |= ChangeFlags::OTHER_CHANGE;
        }
        if self.radius != prev.radius {
            element
                .set_attribute("r", &format!("{}", self.radius))
                .unwrap();
            is_changed |= ChangeFlags::OTHER_CHANGE;
        }
        is_changed
    }

    fn message(
        &self,
        _id_path: &[Id],
        _state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        _app_state: &mut T,
    ) -> MessageResult<()> {
        MessageResult::Stale(message)
    }
}

impl ViewMarker for BezPath {}

impl<T> View<T> for BezPath {
    type State = ();
    type Element = Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Element) {
        let el = cx
            .document()
            .create_element_ns(Some("http://www.w3.org/2000/svg"), "path")
            .unwrap();
        el.set_attribute("d", &self.to_svg()).unwrap();
        let id = Id::next();
        (id, (), el)
    }

    fn rebuild(
        &self,
        _d: &mut Cx,
        prev: &Self,
        _id: &mut Id,
        _state: &mut Self::State,
        element: &mut Element,
    ) -> ChangeFlags {
        let mut is_changed = ChangeFlags::default();
        if self != prev {
            element.set_attribute("d", &self.to_svg()).unwrap();
            is_changed |= ChangeFlags::OTHER_CHANGE;
        }
        is_changed
    }

    fn message(
        &self,
        _id_path: &[Id],
        _state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        _app_state: &mut T,
    ) -> MessageResult<()> {
        MessageResult::Stale(message)
    }
}

// TODO: RoundedRect
