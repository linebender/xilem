// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::core::{CollectionWidget, FromDynWidget, Widget, WidgetMut};
use masonry::widgets;

use crate::core::{
    AppendVec, ElementSplice, MessageCtx, MessageResult, Mut, SuperElement, View, ViewElement,
    ViewMarker, ViewSequence,
};
use crate::{Pod, ViewCtx, WidgetView};

pub use masonry::widgets::{GridParams, GridTrackSize};

/// A Grid layout divides a window into regions and defines the relationship
/// between inner elements in terms of size and position.
///
/// # Example
/// ```ignore
/// # use xilem_masonry as xilem;
/// use masonry::widgets::GridParams;
/// use xilem::view::{
///     text_button, grid, label, unit_fractions, GridExt,
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
/// grid((
///     label(state.int.to_string()).grid((0, 0, 3, ())),
///     text_button("Decrease by 1", |state: &mut State| state.int -= 1).grid((1, 1)),
///     text_button("To zero", |state: &mut State| state.int = 0).grid((2, 1)),
///     text_button("Increase by 1", |state: &mut State| state.int += 1).grid((3, 1)),
/// ))
/// .columns(unit_fractions(3))
/// .rows(unit_fractions(2))
/// .gap(GRID_GAP)
/// ```
/// Also see Calculator example [here](https://github.com/linebender/xilem/blob/main/xilem/examples/calc.rs) to learn more about grid layout.
pub fn grid<State: 'static, Action, Seq: GridSequence<State, Action>>(
    sequence: Seq,
) -> Grid<Seq, (), (), State, Action> {
    Grid {
        sequence,
        columns: (),
        rows: (),
        phantom: PhantomData,
    }
}

/// Helper function for quickly creating `n` tracks with which divides length equally between them.
///
/// Similar to CSS `repeat(n, 1fr)`.
pub fn unit_fractions(n: usize) -> impl GridTracks {
    hidden::CloneTracks(std::iter::repeat_n(GridTrackSize::FRACTION, n))
}

/// The [`View`] created by [`grid`] from a sequence.
///
/// See `grid` documentation for more context.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Grid<Seq, Col, Row, State, Action = ()> {
    sequence: Seq,
    columns: Col,
    rows: Row,

    /// Used to associate the State and Action in the call to `.grid()` with the State and Action
    /// used in the View implementation, to allow inference to flow backwards, allowing State and
    /// Action to be inferred properly
    phantom: PhantomData<fn() -> (State, Action)>,
}

impl<Seq, Col, Row, State, Action> Grid<Seq, Col, Row, State, Action> {
    /// Sets the column widths. See [`GridTrackSize`] for more info.
    pub fn columns<NewCol>(self, columns: NewCol) -> Grid<Seq, NewCol, Row, State, Action> {
        Grid {
            sequence: self.sequence,
            columns,
            rows: self.rows,
            phantom: PhantomData,
        }
    }

    /// Sets the row heights. See [`GridTrackSize`] for more info.
    pub fn rows<NewRow>(self, rows: NewRow) -> Grid<Seq, Col, NewRow, State, Action> {
        Grid {
            sequence: self.sequence,
            columns: self.columns,
            rows,
            phantom: PhantomData,
        }
    }
}

mod hidden {
    use super::{GridElement, GridTrackSize};
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

    #[doc(hidden)]
    #[expect(
        unnameable_types,
        reason = "Implementation detail, public because of trait visibility rules"
    )]
    pub trait GridTracks {
        fn to_tracks(&self) -> impl Iterator<Item = GridTrackSize>;
    }

    impl GridTracks for GridTrackSize {
        fn to_tracks(&self) -> impl Iterator<Item = Self> {
            std::iter::once(*self)
        }
    }

    impl<const N: usize> GridTracks for [GridTrackSize; N] {
        fn to_tracks(&self) -> impl Iterator<Item = GridTrackSize> {
            self.iter().copied()
        }
    }

    impl GridTracks for Vec<GridTrackSize> {
        fn to_tracks(&self) -> impl Iterator<Item = GridTrackSize> {
            self.iter().copied()
        }
    }

    pub(super) struct CloneTracks<T>(pub(super) T);

    impl<T: Clone + IntoIterator<Item = GridTrackSize>> GridTracks for CloneTracks<T> {
        fn to_tracks(&self) -> impl Iterator<Item = GridTrackSize> {
            self.0.clone().into_iter()
        }
    }

    impl GridTracks for () {
        fn to_tracks(&self) -> impl Iterator<Item = GridTrackSize> {
            std::iter::empty()
        }
    }

    impl<A: GridTracks> GridTracks for (A,) {
        fn to_tracks(&self) -> impl Iterator<Item = GridTrackSize> {
            self.0.to_tracks()
        }
    }

    macro_rules! impl_grid_tracks {
        (
            // We could use the ${index} metavariable here once it's stable
            // https://veykril.github.io/tlborm/decl-macros/minutiae/metavar-expr.html
            $($marker: ident, $seq: ident, $idx: tt);+
        ) => {
            impl<$($seq: GridTracks,)+> GridTracks for ($($seq,)+) {
                fn to_tracks(&self) -> impl Iterator<Item = GridTrackSize> {
                    std::iter::empty()
                    $(.chain(self.$idx.to_tracks()))+
                }
            }
        };
    }

    // We implement for tuples of length up to 16. 0 and 1 are special cased to be more efficient
    impl_grid_tracks!(M0, Seq0, 0; M1, Seq1, 1);
    impl_grid_tracks!(M0, Seq0, 0; M1, Seq1, 1; M2, Seq2, 2);
    impl_grid_tracks!(M0, Seq0, 0; M1, Seq1, 1; M2, Seq2, 2; M3, Seq3, 3);
    impl_grid_tracks!(M0, Seq0, 0; M1, Seq1, 1; M2, Seq2, 2; M3, Seq3, 3; M4, Seq4, 4);
    impl_grid_tracks!(M0, Seq0, 0; M1, Seq1, 1; M2, Seq2, 2; M3, Seq3, 3; M4, Seq4, 4; M5, Seq5, 5);
    impl_grid_tracks!(M0, Seq0, 0; M1, Seq1, 1; M2, Seq2, 2; M3, Seq3, 3; M4, Seq4, 4; M5, Seq5, 5; M6, Seq6, 6);
    impl_grid_tracks!(M0, Seq0, 0; M1, Seq1, 1; M2, Seq2, 2; M3, Seq3, 3; M4, Seq4, 4; M5, Seq5, 5; M6, Seq6, 6; M7, Seq7, 7);
    impl_grid_tracks!(M0, Seq0, 0; M1, Seq1, 1; M2, Seq2, 2; M3, Seq3, 3; M4, Seq4, 4; M5, Seq5, 5; M6, Seq6, 6; M7, Seq7, 7; M8, Seq8, 8);
    impl_grid_tracks!(M0, Seq0, 0; M1, Seq1, 1; M2, Seq2, 2; M3, Seq3, 3; M4, Seq4, 4; M5, Seq5, 5; M6, Seq6, 6; M7, Seq7, 7; M8, Seq8, 8; M9, Seq9, 9);
    impl_grid_tracks!(M0, Seq0, 0; M1, Seq1, 1; M2, Seq2, 2; M3, Seq3, 3; M4, Seq4, 4; M5, Seq5, 5; M6, Seq6, 6; M7, Seq7, 7; M8, Seq8, 8; M9, Seq9, 9; M10, Seq10, 10);
    impl_grid_tracks!(M0, Seq0, 0; M1, Seq1, 1; M2, Seq2, 2; M3, Seq3, 3; M4, Seq4, 4; M5, Seq5, 5; M6, Seq6, 6; M7, Seq7, 7; M8, Seq8, 8; M9, Seq9, 9; M10, Seq10, 10; M11, Seq11, 11);
    impl_grid_tracks!(M0, Seq0, 0; M1, Seq1, 1; M2, Seq2, 2; M3, Seq3, 3; M4, Seq4, 4; M5, Seq5, 5; M6, Seq6, 6; M7, Seq7, 7; M8, Seq8, 8; M9, Seq9, 9; M10, Seq10, 10; M11, Seq11, 11; M12, Seq12, 12);
    impl_grid_tracks!(M0, Seq0, 0; M1, Seq1, 1; M2, Seq2, 2; M3, Seq3, 3; M4, Seq4, 4; M5, Seq5, 5; M6, Seq6, 6; M7, Seq7, 7; M8, Seq8, 8; M9, Seq9, 9; M10, Seq10, 10; M11, Seq11, 11; M12, Seq12, 12; M13, Seq13, 13);
    impl_grid_tracks!(M0, Seq0, 0; M1, Seq1, 1; M2, Seq2, 2; M3, Seq3, 3; M4, Seq4, 4; M5, Seq5, 5; M6, Seq6, 6; M7, Seq7, 7; M8, Seq8, 8; M9, Seq9, 9; M10, Seq10, 10; M11, Seq11, 11; M12, Seq12, 12; M13, Seq13, 13; M14, Seq14, 14);
    impl_grid_tracks!(M0, Seq0, 0; M1, Seq1, 1; M2, Seq2, 2; M3, Seq3, 3; M4, Seq4, 4; M5, Seq5, 5; M6, Seq6, 6; M7, Seq7, 7; M8, Seq8, 8; M9, Seq9, 9; M10, Seq10, 10; M11, Seq11, 11; M12, Seq12, 12; M13, Seq13, 13; M14, Seq14, 14; M15, Seq15, 15);
}

use hidden::{GridState, GridTracks};

impl<Seq, Col, Row, State, Action> ViewMarker for Grid<Seq, Col, Row, State, Action> {}

impl<State, Action, Seq, Col, Row> View<State, Action, ViewCtx>
    for Grid<Seq, Col, Row, State, Action>
where
    State: 'static,
    Action: 'static,
    Col: GridTracks + 'static,
    Row: GridTracks + 'static,
    Seq: GridSequence<State, Action>,
{
    type Element = Pod<widgets::Grid>;

    type ViewState = GridState<Seq::SeqState>;

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let mut elements = AppendVec::default();
        let mut widget = widgets::Grid::new()
            .with_columns(self.columns.to_tracks())
            .with_rows(self.rows.to_tracks());
        let seq_state = self.sequence.seq_build(ctx, &mut elements, app_state);
        for element in elements.drain() {
            widget = widget.with(element.child.new_widget, element.params);
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
        if !prev.rows.to_tracks().eq(self.rows.to_tracks()) {
            widgets::Grid::set_rows(&mut element, self.rows.to_tracks().collect());
        }
        if !prev.columns.to_tracks().eq(self.columns.to_tracks()) {
            widgets::Grid::set_columns(&mut element, self.columns.to_tracks().collect());
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
    ) {
        let mut splice = GridSplice::new(element, scratch);
        self.sequence.seq_teardown(seq_state, ctx, &mut splice);
        debug_assert!(scratch.is_empty());
    }

    fn message(
        &self,
        GridState { seq_state, scratch }: &mut Self::ViewState,
        message: &mut MessageCtx,
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
        // Getting here means that the widget didn't use .grid.
        Self {
            child: child.erased(),
            // TODO - Should be 0, 0?
            params: GridParams::new(),
        }
    }

    fn with_downcast_val<R>(
        mut this: Mut<'_, Self>,
        f: impl FnOnce(Mut<'_, Pod<W>>) -> R,
    ) -> (Mut<'_, Self>, R) {
        let ret = {
            let mut child = widgets::Grid::get_mut(&mut this.parent, this.idx);
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
            widgets::Grid::insert(
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
        widgets::Grid::insert(
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
        widgets::Grid::remove(&mut self.element, self.idx);
        ret
    }
}

/// `GridSequence` is what allows an input to the grid that contains all the grid elements.
pub trait GridSequence<State: 'static, Action = ()>:
    ViewSequence<State, Action, ViewCtx, GridElement>
{
}

impl<Seq, State, Action> GridSequence<State, Action> for Seq
where
    Seq: ViewSequence<State, Action, ViewCtx, GridElement>,
    State: 'static,
{
}

/// A trait which extends a [`WidgetView`] with methods to provide parameters for a grid item
pub trait GridExt<State: 'static, Action>: WidgetView<State, Action> {
    /// Applies [`impl Into<GridParams>`](`GridParams`) to this view. This allows the view
    /// to be placed as a child within a [`Grid`] [`View`].
    ///
    /// # Examples
    /// ```
    /// # use xilem_masonry as xilem;
    /// use masonry::widgets::GridParams;
    /// use xilem::view::{text_button, prose, grid, unit_fractions, GridExt};
    /// # use xilem::WidgetView;
    ///
    /// # fn view<State: 'static>() -> impl WidgetView<State> {
    /// grid((
    ///     text_button("click me", |_| ()).grid((0, 0, 2, ())),
    ///     prose("a prose").grid((1, 1)),
    /// ))
    /// .columns(unit_fractions(2))
    /// .rows(unit_fractions(2))
    /// # }
    /// ```
    fn grid(self, params: impl Into<GridParams>) -> GridItem<Self, State, Action>
    where
        Action: 'static,
        Self: Sized,
    {
        grid_item(self, params)
    }
}

impl<State: 'static, Action, V: WidgetView<State, Action>> GridExt<State, Action> for V {}

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
                widgets::Grid::set_params(&mut element.parent, element.idx, self.params);
            }
            let mut child = widgets::Grid::get_mut(&mut element.parent, element.idx);
            self.view
                .rebuild(&prev.view, view_state, ctx, child.downcast(), app_state);
        }
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        let mut child = widgets::Grid::get_mut(&mut element.parent, element.idx);
        self.view.teardown(view_state, ctx, child.downcast());
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        let mut child = widgets::Grid::get_mut(&mut element.parent, element.idx);
        self.view
            .message(view_state, message, child.downcast(), app_state)
    }
}
