// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Model version of Masonry for exploration

use core::any::Any;

use xilem_core::{
    DynMessage, MessageResult, Mut, SuperElement, View, ViewElement, ViewId, ViewMarker,
    ViewPathTracker,
};

pub fn app_logic(_: &mut u32) -> impl WidgetView<u32> {
    Button {}
}

pub fn main() {
    let view = app_logic(&mut 10);
    let mut ctx = ViewCtx { path: vec![] };
    let (_widget_tree, _state) = view.build(&mut ctx);
    // TODO: dbg!(widget_tree);
}

// Toy version of Masonry
pub trait Widget: 'static + Any {
    fn as_mut_any(&mut self) -> &mut dyn Any;
}
pub struct WidgetPod<W: Widget> {
    widget: W,
}
pub struct WidgetMut<'a, W: Widget> {
    value: &'a mut W,
}
impl Widget for Box<dyn Widget> {
    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
}

// Model version of xilem_masonry (`xilem`)

// Hmm, this implementation can't exist in `xilem` if `xilem_core` and/or `masonry` are a different crate
// due to the orphan rules...
impl<W: Widget> ViewElement for WidgetPod<W> {
    type Mut<'a> = WidgetMut<'a, W>;
}

impl ViewMarker for Button {}
impl<State, Action> View<State, Action, ViewCtx> for Button {
    type Element = WidgetPod<ButtonWidget>;
    type ViewState = ();

    fn build(&self, _ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        (
            WidgetPod {
                widget: ButtonWidget {},
            },
            (),
        )
    }

    fn rebuild<'el>(
        &self,
        _prev: &Self,
        _view_state: &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        // Nothing to do
        element
    }

    fn teardown(
        &self,
        _view_state: &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Self::Element>,
    ) {
        // Nothing to do
    }

    fn message(
        &self,
        _view_state: &mut Self::ViewState,
        _id_path: &[ViewId],
        _message: DynMessage,
        _app_state: &mut State,
    ) -> MessageResult<Action> {
        MessageResult::Nop
    }
}

pub struct Button {}

pub struct ButtonWidget {}
impl Widget for ButtonWidget {
    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
}

impl<W: Widget> SuperElement<WidgetPod<W>, ViewCtx> for WidgetPod<Box<dyn Widget>> {
    fn upcast(_ctx: &mut ViewCtx, child: WidgetPod<W>) -> Self {
        WidgetPod {
            widget: Box::new(child.widget),
        }
    }
    fn with_downcast_val<R>(
        this: Self::Mut<'_>,
        f: impl FnOnce(<WidgetPod<W> as ViewElement>::Mut<'_>) -> R,
    ) -> (Self::Mut<'_>, R) {
        let value = WidgetMut {
        value: this.value.as_mut_any().downcast_mut().expect(
            "this widget should have been created from a child widget of type `W` in `Self::upcast`",
        ),
    };
        let ret = f(value);
        (this, ret)
    }
}

pub struct ViewCtx {
    path: Vec<ViewId>,
}

impl ViewPathTracker for ViewCtx {
    fn push_id(&mut self, id: ViewId) {
        self.path.push(id);
    }

    fn pop_id(&mut self) {
        self.path.pop();
    }

    fn view_path(&mut self) -> &[ViewId] {
        &self.path
    }
}

pub trait WidgetView<State, Action = ()>:
    View<State, Action, ViewCtx, Element = WidgetPod<Self::Widget>> + Send + Sync
{
    type Widget: Widget + Send + Sync;
}

impl<V, State, Action, W> WidgetView<State, Action> for V
where
    V: View<State, Action, ViewCtx, Element = WidgetPod<W>> + Send + Sync,
    W: Widget + Send + Sync,
{
    type Widget = W;
}
