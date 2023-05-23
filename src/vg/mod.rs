// Copyright 2023 The Xilem Authors.
// SPDX-License-Identifier: Apache-2.0

use std::{any::Any, marker::PhantomData};

use vello::{peniko::Color, SceneBuilder, SceneFragment};

use xilem_core::Id;

use crate::{
    view::{Cx, View, ViewMarker},
    widget::{
        AccessCx, BoxConstraints, ChangeFlags, Event, EventCx, LayoutCx, LifeCycle, LifeCycleCx,
        PaintCx, UpdateCx, Widget,
    },
    MessageResult,
};

mod color;
mod group;
mod kurbo_shape;
mod pointer;
pub use crate::vg::color::color;
pub use crate::vg::group::group;

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
    fn paint(&mut self, cx: &VgPaintCx, builder: &mut SceneBuilder);
}

pub trait AnyVgNode: VgNode {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn type_name(&self) -> &'static str;
}

pub struct VgPaintCx {
    color: Color,
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
        let mut changed = self.root.rebuild(cx, &prev.root, id, state, child_element);
        // TODO: be smart and fine grained
        changed |= ChangeFlags::PAINT;
        changed
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
    fn event(&mut self, _cx: &mut EventCx, event: &Event) {
        println!("vg widget got event {:?}", event);
    }

    fn lifecycle(&mut self, _cx: &mut LifeCycleCx, _event: &LifeCycle) {}

    fn update(&mut self, _cx: &mut UpdateCx) {}

    fn layout(&mut self, _cx: &mut LayoutCx, bc: &BoxConstraints) -> glazier::kurbo::Size {
        bc.constrain((500.0, 500.0))
    }

    fn accessibility(&mut self, _cx: &mut AccessCx) {}

    fn paint(&mut self, _cx: &mut PaintCx, builder: &mut vello::SceneBuilder) {
        let cx = VgPaintCx {
            color: Color::SLATE_BLUE,
        };
        self.root.paint(&cx, builder);
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

    fn paint_impl(&mut self, cx: &VgPaintCx) {
        let mut builder = SceneBuilder::for_fragment(&mut self.fragment);
        self.node.paint(cx, &mut builder);
    }

    fn paint(&mut self, cx: &VgPaintCx, builder: &mut SceneBuilder) {
        // TODO: this should be conditional on paint ChangeFlags
        self.paint_impl(cx);
        builder.append(&self.fragment, None);
    }
}
