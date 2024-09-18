// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::widget::GridParams;
use masonry::{
    widget::{self, WidgetMut},
    Widget,
};
use xilem_core::{
    AppendVec, DynMessage, ElementSplice, MessageResult, Mut, SuperElement, View, ViewElement,
    ViewMarker, ViewSequence,
};

use crate::{Pod, ViewCtx, WidgetView};

pub fn grid<State, Action, Seq: GridSequence<State, Action>>(
    sequence: Seq,
    width: i32,
    height: i32,
) -> Grid<Seq, State, Action> {
    Grid {
        sequence,
        spacing: 0.0,
        phantom: PhantomData,
        height,
        width,
    }
}

pub struct Grid<Seq, State, Action = ()> {
    sequence: Seq,
    spacing: f64,
    width: i32,
    height: i32,
    /// Used to associate the State and Action in the call to `.grid()` with the State and Action
    /// used in the View implementation, to allow inference to flow backwards, allowing State and
    /// Action to be inferred properly
    phantom: PhantomData<fn() -> (State, Action)>,
}

impl<Seq, State, Action> Grid<Seq, State, Action> {
    #[track_caller]
    pub fn spacing(mut self, spacing: f64) -> Self {
        if spacing.is_finite() && spacing >= 0.0 {
            self.spacing = spacing;
        } else {
            panic!("Invalid `spacing` {spacing}; expected a non-negative finite value.")
        }
        self
    }
}

impl<Seq, State, Action> ViewMarker for Grid<Seq, State, Action> {}

impl<State, Action, Seq> View<State, Action, ViewCtx> for Grid<Seq, State, Action>
where
    State: 'static,
    Action: 'static,
    Seq: GridSequence<State, Action>,
{
    type Element = Pod<widget::Grid>;

    type ViewState = Seq::SeqState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let mut elements = AppendVec::default();
        let mut widget = widget::Grid::with_dimensions(self.width, self.height);
        widget = widget.with_spacing(self.spacing);
        let seq_state = self.sequence.seq_build(ctx, &mut elements);
        for child in elements.into_inner() {
            widget = match child {
                GridElement::Child(child, params) => widget.with_child_pod(child.inner, params),
            }
        }
        (ctx.new_pod(widget), seq_state)
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        if prev.height != self.height {
            element.set_height(self.height);
            ctx.mark_changed();
        }
        if prev.width != self.width {
            element.set_width(self.width);
            ctx.mark_changed();
        }
        if prev.spacing != self.spacing {
            element.set_spacing(self.spacing);
            ctx.mark_changed();
        }

        let mut splice = GridSplice::new(element);
        self.sequence
            .seq_rebuild(&prev.sequence, view_state, ctx, &mut splice);
        debug_assert!(splice.scratch.is_empty());
        splice.element
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
    ) {
        let mut splice = GridSplice::new(element);
        self.sequence.seq_teardown(view_state, ctx, &mut splice);
        debug_assert!(splice.scratch.into_inner().is_empty());
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[xilem_core::ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        self.sequence
            .seq_message(view_state, id_path, message, app_state)
    }
}

// Used to become a reference form for editing. It's provided to rebuild and teardown.
impl ViewElement for GridElement {
    type Mut<'w> = GridElementMut<'w>;
}

// Used to allow the item to be used as a generic item in ViewSequence.
impl SuperElement<GridElement, ViewCtx> for GridElement {
    fn upcast(_ctx: &mut ViewCtx, child: GridElement) -> Self {
        child
    }

    fn with_downcast_val<R>(
        mut this: Mut<'_, Self>,
        f: impl FnOnce(Mut<'_, GridElement>) -> R,
    ) -> (Self::Mut<'_>, R) {
        let r = {
            let parent = this.parent.reborrow_mut();
            let reborrow = GridElementMut {
                idx: this.idx,
                parent,
            };
            f(reborrow)
        };
        (this, r)
    }
}

impl<W: Widget> SuperElement<Pod<W>, ViewCtx> for GridElement {
    fn upcast(ctx: &mut ViewCtx, child: Pod<W>) -> Self {
        // Getting here means that the widget didn't use .grid_item or .grid_pos.
        // This currently places the widget in the top left cell.
        // There is not much else, beyond purposefully failing, that can be done here,
        // because there isn't enough information to determine an appropriate spot
        // for the widget.
        GridElement::Child(ctx.boxed_pod(child), GridParams::new(1, 1, 1, 1))
    }

    fn with_downcast_val<R>(
        mut this: Mut<'_, Self>,
        f: impl FnOnce(Mut<'_, Pod<W>>) -> R,
    ) -> (Mut<'_, Self>, R) {
        let ret = {
            let mut child = this
                .parent
                .child_mut(this.idx)
                .expect("This is supposed to be a widget");
            let downcast = child.downcast();
            f(downcast)
        };

        (this, ret)
    }
}

// Used for building and rebuilding the ViewSequence
impl ElementSplice<GridElement> for GridSplice<'_> {
    fn with_scratch<R>(&mut self, f: impl FnOnce(&mut AppendVec<GridElement>) -> R) -> R {
        let ret = f(&mut self.scratch);
        for element in self.scratch.drain() {
            match element {
                GridElement::Child(child, params) => {
                    self.element
                        .insert_grid_child_pod(self.idx, child.inner, params);
                }
            };
            self.idx += 1;
        }
        ret
    }

    fn insert(&mut self, element: GridElement) {
        match element {
            GridElement::Child(child, params) => {
                self.element
                    .insert_grid_child_pod(self.idx, child.inner, params);
            }
        };
        self.idx += 1;
    }

    fn mutate<R>(&mut self, f: impl FnOnce(Mut<'_, GridElement>) -> R) -> R {
        let child = GridElementMut {
            parent: self.element.reborrow_mut(),
            idx: self.idx,
        };
        let ret = f(child);
        self.idx += 1;
        ret
    }

    fn skip(&mut self, n: usize) {
        self.idx += n;
    }

    fn delete<R>(&mut self, f: impl FnOnce(Mut<'_, GridElement>) -> R) -> R {
        let ret = {
            let child = GridElementMut {
                parent: self.element.reborrow_mut(),
                idx: self.idx,
            };
            f(child)
        };
        self.element.remove_child(self.idx);
        ret
    }
}

/// `GridSequence` is what allows an input to the grid that contains all the grid elements.
pub trait GridSequence<State, Action = ()>:
    ViewSequence<State, Action, ViewCtx, GridElement>
{
}

impl<Seq, State, Action> GridSequence<State, Action> for Seq where
    Seq: ViewSequence<State, Action, ViewCtx, GridElement>
{
}

/// A trait which extends a [`WidgetView`] with methods to provide parameters for a grid item
pub trait GridExt<State, Action>: WidgetView<State, Action> {
    /// Applies [`impl Into<GridParams>`](`GridParams`) to this view. This allows the view
    /// to be placed as a child within a [`Grid`] [`View`].
    ///
    /// # Examples
    /// ```
    /// use masonry::widget::GridParams;
    /// use xilem::{view::{button, prose, grid, GridExt}};
    /// # use xilem::{WidgetView};
    ///
    /// # fn view<State: 'static>() -> impl WidgetView<State> {
    /// grid((
    ///     button("click me", |_| ()).grid_item(GridParams::new(0, 0, 2, 1)),
    ///     prose("a prose").grid_item(GridParams::new(1, 1, 1, 1)),
    /// ), 2, 2)
    /// # }
    /// ```
    fn grid_item(self, params: impl Into<GridParams>) -> GridItem<Self, State, Action>
    where
        State: 'static,
        Action: 'static,
        Self: Sized,
    {
        grid_item(self, params)
    }

    /// Applies a [`impl Into<GridParams>`](`GridParams`) with the specified position to this view.
    /// This allows the view to be placed as a child within a [`Grid`] [`View`].
    /// For instances where a grid item is expected to take up multiple cell units,
    /// use [`GridExt::grid_item`]
    ///
    /// # Examples
    /// ```
    /// use masonry::widget::GridParams;
    /// use xilem::{view::{button, prose, grid, GridExt}};
    /// # use xilem::{WidgetView};
    ///
    /// # fn view<State: 'static>() -> impl WidgetView<State> {
    /// grid((
    ///     button("click me", |_| ()).grid_pos(0, 0),
    ///     prose("a prose").grid_pos(1, 1),
    /// ), 2, 2)
    /// # }
    fn grid_pos(self, x: i32, y: i32) -> GridItem<Self, State, Action>
    where
        State: 'static,
        Action: 'static,
        Self: Sized,
    {
        grid_item(self, GridParams::new(x, y, 1, 1))
    }
}

impl<State, Action, V: WidgetView<State, Action>> GridExt<State, Action> for V {}

pub enum GridElement {
    Child(Pod<Box<dyn Widget>>, GridParams),
}

pub struct GridElementMut<'w> {
    parent: WidgetMut<'w, widget::Grid>,
    idx: usize,
}

// Used for manipulating the ViewSequence.
pub struct GridSplice<'w> {
    idx: usize,
    element: WidgetMut<'w, widget::Grid>,
    scratch: AppendVec<GridElement>,
}

impl<'w> GridSplice<'w> {
    fn new(element: WidgetMut<'w, widget::Grid>) -> Self {
        Self {
            idx: 0,
            element,
            scratch: AppendVec::default(),
        }
    }
}

/// A `WidgetView` that can be used within a [`Grid`] [`View`]
pub struct GridItem<V, State, Action> {
    view: V,
    params: GridParams,
    phantom: PhantomData<fn() -> (State, Action)>,
}

pub fn grid_item<V, State, Action>(
    view: V,
    params: impl Into<GridParams>,
) -> GridItem<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    GridItem {
        view,
        params: params.into(),
        phantom: PhantomData,
    }
}

impl<V, State, Action> ViewMarker for GridItem<V, State, Action> {}

impl<State, Action, V> View<State, Action, ViewCtx> for GridItem<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    type Element = GridElement;

    type ViewState = V::ViewState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (pod, state) = self.view.build(ctx);
        (GridElement::Child(ctx.boxed_pod(pod), self.params), state)
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        {
            if self.params != prev.params {
                element
                    .parent
                    .update_child_grid_params(element.idx, self.params);
            }
            let mut child = element
                .parent
                .child_mut(element.idx)
                .expect("GridWrapper always has a widget child");
            self.view
                .rebuild(&prev.view, view_state, ctx, child.downcast());
        }
        element
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        let mut child = element
            .parent
            .child_mut(element.idx)
            .expect("GridWrapper always has a widget child");
        self.view.teardown(view_state, ctx, child.downcast());
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[xilem_core::ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        self.view.message(view_state, id_path, message, app_state)
    }
}
