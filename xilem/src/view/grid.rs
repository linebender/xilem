// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::core::{FromDynWidget, Widget, WidgetMut};
use masonry::properties::types::Length;
use masonry::widgets;

use crate::core::{
    AppendVec, ElementSplice, MessageContext, MessageResult, Mut, SuperElement, View, ViewElement,
    ViewMarker, ViewSequence,
};
use crate::{Pod, ViewCtx, WidgetView};

pub use masonry::widgets::GridParams;
/// A Grid layout divides a window into regions and defines the relationship
/// between inner elements in terms of size and position.
///
/// # Example
/// ```ignore
/// use masonry::widgets::GridParams;
/// use xilem::view::{
///     button, grid, label, GridExt,
/// };
///
/// const GRID_GAP: f64 = 2.;
///
/// #[derive(Default)]
/// struct State {
///     int: i32,
/// }
///
/// let mut state = State::default();
///
/// grid(
///     (
///         label(state.int.to_string()).grid_item(GridParams::new(0, 0, 3, 1)),
///         button(label("Decrease by 1"), |state: &mut State| state.int -= 1).grid_pos(1, 1),
///         button(label("To zero"), |state: &mut State| state.int = 0).grid_pos(2, 1),
///         button(label("Increase by 1"), |state: &mut State| state.int += 1).grid_pos(3, 1),
///     ),
///     3,
///     2,
/// )
/// .spacing(GRID_GAP)
/// ```
/// Also see Calculator example [here](https://github.com/linebender/xilem/blob/main/xilem/examples/calc.rs) to learn more about grid layout.
pub fn grid<State, Action, Seq: GridSequence<State, Action>>(
    sequence: Seq,
    width: i32,
    height: i32,
) -> Grid<Seq, State, Action> {
    Grid {
        sequence,
        spacing: Length::ZERO,
        height,
        width,
        phantom: PhantomData,
    }
}

/// The [`View`] created by [`grid`] from a sequence, which also consumes custom width and height.
///
/// See `grid` documentation for more context.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Grid<Seq, State, Action = ()> {
    sequence: Seq,
    spacing: Length,
    width: i32,
    height: i32,

    /// Used to associate the State and Action in the call to `.grid()` with the State and Action
    /// used in the View implementation, to allow inference to flow backwards, allowing State and
    /// Action to be inferred properly
    phantom: PhantomData<fn() -> (State, Action)>,
}

impl<Seq, State, Action> Grid<Seq, State, Action> {
    /// Set the spacing (both vertical and horizontal) between grid items.
    #[track_caller]
    pub fn spacing(mut self, spacing: Length) -> Self {
        self.spacing = spacing;
        self
    }
}

mod hidden {
    use super::GridElement;
    use crate::core::AppendVec;

    #[doc(hidden)]
    #[expect(
        unnameable_types,
        reason = "Implementation detail, public because of trait visibility rules"
    )]
    pub struct GridState<SeqState> {
        pub(crate) seq_state: SeqState,
        pub(crate) scratch: AppendVec<GridElement>,
    }
}

use hidden::GridState;

impl<Seq, State, Action> ViewMarker for Grid<Seq, State, Action> {}

impl<State, Action, Seq> View<State, Action, ViewCtx> for Grid<Seq, State, Action>
where
    State: 'static,
    Action: 'static,
    Seq: GridSequence<State, Action>,
{
    type Element = Pod<widgets::Grid>;

    type ViewState = GridState<Seq::SeqState>;

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let mut elements = AppendVec::default();
        let mut widget = widgets::Grid::with_dimensions(self.width, self.height);
        widget = widget.with_spacing(self.spacing);
        let seq_state = self.sequence.seq_build(ctx, &mut elements, app_state);
        for element in elements.drain() {
            widget = widget.with_child(element.child.new_widget, element.params);
        }
        let pod = ctx.create_pod(widget);
        (
            pod,
            GridState {
                seq_state,
                scratch: elements,
            },
        )
    }

    fn rebuild(
        &self,
        prev: &Self,
        GridState { seq_state, scratch }: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        if prev.height != self.height {
            widgets::Grid::set_height(&mut element, self.height);
        }
        if prev.width != self.width {
            widgets::Grid::set_width(&mut element, self.width);
        }
        if prev.spacing != self.spacing {
            widgets::Grid::set_spacing(&mut element, self.spacing);
        }

        let mut splice = GridSplice::new(element, scratch);
        self.sequence
            .seq_rebuild(&prev.sequence, seq_state, ctx, &mut splice, app_state);
        debug_assert!(scratch.is_empty());
    }

    fn teardown(
        &self,
        GridState { seq_state, scratch }: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        let mut splice = GridSplice::new(element, scratch);
        self.sequence
            .seq_teardown(seq_state, ctx, &mut splice, app_state);
        debug_assert!(scratch.is_empty());
    }

    fn message(
        &self,
        GridState { seq_state, scratch }: &mut Self::ViewState,
        message: &mut MessageContext,
        element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        let mut splice = GridSplice::new(element, scratch);
        let result = self
            .sequence
            .seq_message(seq_state, message, &mut splice, app_state);
        debug_assert!(scratch.is_empty());
        result
    }
}

// Used to become a reference form for editing. It's provided to rebuild and teardown.
impl ViewElement for GridElement {
    type Mut<'w> = GridElementMut<'w>;
}

// Used to allow the item to be used as a generic item in ViewSequence.
impl SuperElement<Self, ViewCtx> for GridElement {
    fn upcast(_ctx: &mut ViewCtx, child: Self) -> Self {
        child
    }

    fn with_downcast_val<R>(
        mut this: Mut<'_, Self>,
        f: impl FnOnce(Mut<'_, Self>) -> R,
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

impl<W: Widget + FromDynWidget + ?Sized> SuperElement<Pod<W>, ViewCtx> for GridElement {
    fn upcast(_: &mut ViewCtx, child: Pod<W>) -> Self {
        // Getting here means that the widget didn't use .grid_item or .grid_pos.
        // This currently places the widget in the top left cell.
        // There is not much else, beyond purposefully failing, that can be done here,
        // because there isn't enough information to determine an appropriate spot
        // for the widget.
        Self {
            child: child.erased(),
            // TODO - Should be 0, 0?
            params: GridParams::new(1, 1, 1, 1),
        }
    }

    fn with_downcast_val<R>(
        mut this: Mut<'_, Self>,
        f: impl FnOnce(Mut<'_, Pod<W>>) -> R,
    ) -> (Mut<'_, Self>, R) {
        let ret = {
            let mut child = widgets::Grid::child_mut(&mut this.parent, this.idx);
            let downcast = child.downcast();
            f(downcast)
        };

        (this, ret)
    }
}

// Used for building and rebuilding the ViewSequence
impl ElementSplice<GridElement> for GridSplice<'_, '_> {
    fn with_scratch<R>(&mut self, f: impl FnOnce(&mut AppendVec<GridElement>) -> R) -> R {
        let ret = f(self.scratch);
        for element in self.scratch.drain() {
            widgets::Grid::insert_grid_child_at(
                &mut self.element,
                self.idx,
                element.child.new_widget,
                element.params,
            );
            self.idx += 1;
        }
        ret
    }

    fn insert(&mut self, element: GridElement) {
        widgets::Grid::insert_grid_child_at(
            &mut self.element,
            self.idx,
            element.child.new_widget,
            element.params,
        );
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

    fn index(&self) -> usize {
        self.idx
    }

    fn delete<R>(&mut self, f: impl FnOnce(Mut<'_, GridElement>) -> R) -> R {
        let ret = {
            let child = GridElementMut {
                parent: self.element.reborrow_mut(),
                idx: self.idx,
            };
            f(child)
        };
        widgets::Grid::remove_child(&mut self.element, self.idx);
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
    /// use masonry::widgets::GridParams;
    /// use xilem::{view::{button, label, prose, grid, GridExt}};
    /// # use xilem::{WidgetView};
    ///
    /// # fn view<State: 'static>() -> impl WidgetView<State> {
    /// grid((
    ///     button(label("click me"), |_| ()).grid_item(GridParams::new(0, 0, 2, 1)),
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
    /// use masonry::widgets::GridParams;
    /// use xilem::{view::{button, label, prose, grid, GridExt}};
    /// # use xilem::{WidgetView};
    ///
    /// # fn view<State: 'static>() -> impl WidgetView<State> {
    /// grid((
    ///     button(label("click me"), |_| ()).grid_pos(0, 0),
    ///     prose("a prose").grid_pos(1, 1),
    /// ), 2, 2)
    /// # }
    /// ```
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

/// A child widget within a [`Grid`] view.
pub struct GridElement {
    /// The child widget.
    child: Pod<dyn Widget>,
    /// The grid parameters of the child widget.
    params: GridParams,
}

/// A mutable reference to a [`GridElement`], used internally by Xilem traits.
pub struct GridElementMut<'w> {
    parent: WidgetMut<'w, widgets::Grid>,
    idx: usize,
}

// Used for manipulating the ViewSequence.
struct GridSplice<'w, 's> {
    idx: usize,
    element: WidgetMut<'w, widgets::Grid>,
    scratch: &'s mut AppendVec<GridElement>,
}

impl<'w, 's> GridSplice<'w, 's> {
    fn new(element: WidgetMut<'w, widgets::Grid>, scratch: &'s mut AppendVec<GridElement>) -> Self {
        Self {
            idx: 0,
            element,
            scratch,
        }
    }
}

/// A `WidgetView` that can be used within a [`Grid`] [`View`]
pub struct GridItem<V, State, Action> {
    view: V,
    params: GridParams,
    phantom: PhantomData<fn() -> (State, Action)>,
}

/// Creates a [`GridItem`] from a view and [`GridParams`].
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

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let (pod, state) = self.view.build(ctx, app_state);
        (
            GridElement {
                child: pod.erased(),
                params: self.params,
            },
            state,
        )
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        {
            if self.params != prev.params {
                widgets::Grid::update_child_grid_params(
                    &mut element.parent,
                    element.idx,
                    self.params,
                );
            }
            let mut child = widgets::Grid::child_mut(&mut element.parent, element.idx);
            self.view
                .rebuild(&prev.view, view_state, ctx, child.downcast(), app_state);
        }
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        let mut child = widgets::Grid::child_mut(&mut element.parent, element.idx);
        self.view
            .teardown(view_state, ctx, child.downcast(), app_state);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageContext,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        let mut child = widgets::Grid::child_mut(&mut element.parent, element.idx);
        self.view
            .message(view_state, message, child.downcast(), app_state)
    }
}
