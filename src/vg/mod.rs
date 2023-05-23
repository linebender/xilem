// Copyright 2023 The Xilem Authors.
// SPDX-License-Identifier: Apache-2.0

use std::{any::Any, marker::PhantomData};

use glazier::kurbo::{Affine, Circle};
use vello::{
    peniko::{Color, Fill},
    SceneBuilder, SceneFragment,
};

use xilem_core::Id;

use crate::{
    view::{Cx, View, ViewMarker},
    widget::{
        AccessCx, AnyWidget, BoxConstraints, ChangeFlags, Event, EventCx, LayoutCx, LifeCycle,
        LifeCycleCx, PaintCx, UpdateCx, Widget,
    },
    MessageResult,
};

pub struct Vg<V, T> {
    root: V,
    phantom: PhantomData<fn() -> T>,
}

pub struct VgWidget {
    root: VgPod,
}

pub struct VgPod {
    node: Box<dyn AnyVgNode>,
    // We may want to expand these to include some of our own flags.
    flags: ChangeFlags,
    fragment: SceneFragment,
}

pub trait VgNode {
    // TODO: do need context
    fn paint(&mut self, builder: &mut SceneBuilder);
}

pub trait AnyVgNode: VgNode {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn type_name(&self) -> &'static str;
}

pub fn vg<T, V: VgView<T>>(root: V) -> Vg<V, T> {
    Vg {
        root,
        phantom: Default::default(),
    }
}

impl<N: VgNode + 'static> AnyVgNode for N {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

xilem_core::generate_view_trait! {VgView, VgNode, Cx, ChangeFlags; : Send}
xilem_core::generate_viewsequence_trait! {VgViewSequence, VgView, VgViewMarker, VgNode, Cx, ChangeFlags, VgPod; : Send}

// Need to actually implement AnyVgWidget to make the following work:

//xilem_core::generate_anyview_trait! {VgView, Cx, ChangeFlags, AnyVgWidget + Send}

impl<T: Send, V: VgView<T>> ViewMarker for Vg<V, T> {}

impl<T: Send, V: VgView<T>> View<T> for Vg<V, T>
where
    V::Element: 'static,
{
    type State = V::State;
    type Element = VgWidget;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, state, element) = self.root.build(cx);
        let root = VgPod::new(element);
        let widget = VgWidget { root };
        // Discussion question: do we need our own id?
        (id, state, widget)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        let child_element = element.root.downcast_mut().unwrap();
        self.root.rebuild(cx, &prev.root, id, state, child_element)
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn Any>,
        app_state: &mut T,
    ) -> MessageResult<()> {
        self.root.message(id_path, state, message, app_state)
    }
}

impl Widget for VgWidget {
    fn event(&mut self, _cx: &mut EventCx, _event: &Event) {}

    fn lifecycle(&mut self, _cx: &mut LifeCycleCx, _event: &LifeCycle) {}

    fn update(&mut self, _cx: &mut UpdateCx) {}

    fn layout(&mut self, _cx: &mut LayoutCx, bc: &BoxConstraints) -> glazier::kurbo::Size {
        bc.constrain((500.0, 500.0))
    }

    fn accessibility(&mut self, _cx: &mut AccessCx) {}

    fn paint(&mut self, _cx: &mut PaintCx, builder: &mut vello::SceneBuilder) {
        self.root.paint(builder);
    }
}

impl VgPod {
    fn new(node: impl VgNode + 'static) -> Self {
        VgPod {
            node: Box::new(node),
            flags: ChangeFlags::default(),
            fragment: SceneFragment::new(),
        }
    }

    fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        (self.node).as_any_mut().downcast_mut()
    }

    fn mark(&mut self, flags: ChangeFlags) -> ChangeFlags {
        self.flags |= flags;
        flags.upwards()
    }

    fn paint_impl(&mut self) {
        let mut builder = SceneBuilder::for_fragment(&mut self.fragment);
        self.node.paint(&mut builder);
    }

    fn paint(&mut self, builder: &mut SceneBuilder) {
        self.paint_impl();
        builder.append(&self.fragment, None);
    }
}

pub struct CircleNode {
    circle: Circle,
}

impl VgNode for CircleNode {
    fn paint(&mut self, builder: &mut SceneBuilder) {
        // TODO: obviously we need a way to set this.
        let color = Color::rgb8(0, 0, 255);
        builder.fill(Fill::EvenOdd, Affine::IDENTITY, &color, None, &self.circle);
    }
}

impl<T> VgView<T> for Circle {
    type State = ();

    type Element = CircleNode;

    fn build(&self, _cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let id = Id::next();
        let element = CircleNode { circle: *self };
        (id, (), element)
    }

    fn rebuild(
        &self,
        _cx: &mut Cx,
        prev: &Self,
        _id: &mut Id,
        _state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        let mut changed = ChangeFlags::default();
        if self != prev {
            element.circle = *self;
            changed |= ChangeFlags::PAINT;
        }
        changed
    }

    fn message(
        &self,
        _id_path: &[Id],
        _state: &mut Self::State,
        message: Box<dyn Any>,
        _app_state: &mut T,
    ) -> MessageResult<()> {
        MessageResult::Stale(message)
    }
}
