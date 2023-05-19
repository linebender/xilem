// Copyright 2023 The Xilem Authors.
// SPDX-License-Identifier: Apache-2.0

use std::{any::Any, marker::PhantomData};

use vello::{SceneBuilder, SceneFragment};

use xilem_core::Id;

use crate::{
    view::{Cx, View, ViewMarker},
    widget::{
        AccessCx, BoxConstraints, ChangeFlags, Event, EventCx, LayoutCx, LifeCycle, LifeCycleCx,
        PaintCx, UpdateCx, Widget,
    },
    MessageResult,
};

pub struct Vg<V> {
    root: V,
}

pub struct VgWidget {
    root: VgPod,
}

pub struct VgPod {
    node: Box<dyn VgNode>,
    fragment: SceneFragment,
}

pub trait VgNode {
    // TODO: do need context
    fn paint(&mut self, builder: &mut SceneBuilder);
}

/// A view trait for vector graphics content.
///
/// This trait is generally parallel to the main View trait,
/// but is specialized for vector graphics.
///
/// Potentially interesting future work: reduce cut'n'paste.
pub trait VgView<T, A = ()>: Send {
    /// Associated state for the view.
    type State: Send;

    /// The associated widget for the view.
    type Element: VgNode;

    /// Build the associated widget and initialize state.
    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element);

    /// Update the associated widget.
    ///
    /// Returns `true` when anything has changed.
    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags;

    /// Propagate a message.
    ///
    /// Handle a message, propagating to children if needed. Here, `id_path` is a slice
    /// of ids beginning at a child of this view.
    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn Any>,
        app_state: &mut T,
    ) -> MessageResult<A>;
}

impl<T> ViewMarker for Vg<T> {}

impl<T: Send, V: VgView<T>> View<T> for Vg<V> {
    type State = V::State;
    type Element = VgWidget;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, state, element) = self.root.build(cx);
        todo!()
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        todo!()
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn Any>,
        app_state: &mut T,
    ) -> MessageResult<()> {
        todo!()
    }
}

impl Widget for VgWidget {
    fn event(&mut self, cx: &mut EventCx, event: &Event) {
        todo!()
    }

    fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle) {
        todo!()
    }

    fn update(&mut self, cx: &mut UpdateCx) {
        todo!()
    }

    fn layout(&mut self, cx: &mut LayoutCx, bc: &BoxConstraints) -> glazier::kurbo::Size {
        todo!()
    }

    fn accessibility(&mut self, cx: &mut AccessCx) {
        todo!()
    }

    fn paint(&mut self, cx: &mut PaintCx, builder: &mut vello::SceneBuilder) {
        self.root.paint(builder);
    }
}

impl VgPod {
    fn paint_impl(&mut self) {
        let mut builder = SceneBuilder::for_fragment(&mut self.fragment);
        self.node.paint(&mut builder);
    }

    fn paint(&mut self, builder: &mut SceneBuilder) {
        self.paint_impl();
        builder.append(&self.fragment, None);
    }
}
