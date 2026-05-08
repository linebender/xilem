// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::{any::TypeId, collections::BTreeMap, mem};

use accesskit::{Node, Role};
use include_doc_path::include_doc_path;
use tracing::{Span, trace_span};

use crate::core::{
    AccessCtx, ChildrenIds, CollectionWidget, LayoutCtx, MeasureCtx, NewWidget, NoAction, PaintCtx,
    PropertiesRef, RegisterCtx, UpdateCtx, UsesProperty, Widget, WidgetId, WidgetMut, WidgetPod,
};
use crate::imaging::Painter;
use crate::kurbo::{Axis, Point, Size};
use crate::layout::{AsUnit, LayoutSize, LenDef, LenReq, Length};
use crate::properties::Gap;
use crate::util::debug_panic;

/// A widget that arranges its children in a grid.
///
/// Children are drawn in index order,
/// i.e. each child is drawn on top of the other children with lower indices.
///
#[doc = concat!(
    "![Grid with buttons of various sizes](",
    include_doc_path!("screenshots/grid_with_changed_spacing.png"),
    ")",
)]
#[derive(Default)]
pub struct Grid {
    children: Vec<Child>,
    columns: Vec<GridTrackSize>,
    rows: Vec<GridTrackSize>,
}

#[derive(Debug)]
struct Child {
    widget: WidgetPod<dyn Widget>,
    params: GridParams,

    // these are transient values
    // these do not mean anything if not already initialized within the function
    x: u16,
    y: u16,
}

/// Describes how a grid track (row/column) should be sized.
#[derive(Clone, Copy, PartialEq)]
pub enum GridTrackSize {
    /// At least the minimum intrinsic size of the content and
    /// at most the maximum intrinsic size of the content.
    Auto,

    /// Minimum intrinsic size of the content.
    MinContent,

    /// Maximum intrinsic size of the content
    MaxContent,

    /// Fit content within the given size.
    FitContent(Length),

    /// Fixed at the given size.
    Fixed(Length),

    /// Fixed at the size that is equal to the size of the grid container
    /// times the given parameter.
    ///
    /// The parameter is in normalized percentage, e.g. 0.3, not 30%.
    Percentage(f64),

    /// Fraction of remaining space.
    Fraction(f64),
}

/// Parameters required when adding an item to a [`Grid`] container.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct GridParams {
    /// Defines which column in the grid the item should start.
    pub col: Option<u16>,

    /// Defines which row in the grid the item should start.
    pub row: Option<u16>,

    /// Defines how many column in the grid the item should occupy.
    pub width: u16,

    /// Defines how many row in the grid the item should occupy.
    pub height: u16,
}

// --- MARK: BUILDERS
impl Grid {
    /// Creates a new grid with no columns or rows.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder-style method to add a row to the grid.
    pub fn with_row(mut self, row: GridTrackSize) -> Self {
        self.rows.push(row);
        self
    }

    /// Builder-style method to add a column to the grid.
    pub fn with_column(mut self, column: GridTrackSize) -> Self {
        self.columns.push(column);
        self
    }

    /// Builder-style method to define the track sizes (heights) of the grid rows.
    pub fn with_rows(mut self, rows: impl IntoIterator<Item = GridTrackSize>) -> Self {
        self.rows = rows.into_iter().collect();
        self
    }

    /// Builder-style method to define the track sizes (widths) of the grid columns.
    pub fn with_columns(mut self, columns: impl IntoIterator<Item = GridTrackSize>) -> Self {
        self.columns = columns.into_iter().collect();
        self
    }

    /// Builder-style method to add a child widget.
    pub fn with(
        mut self,
        child: NewWidget<impl Widget + ?Sized>,
        params: impl Into<GridParams>,
    ) -> Self {
        let child = Child::new(child, params);
        self.children.push(child);
        self
    }
}

// --- MARK: METHODS
impl Grid {
    fn assign_cell_to_auto_placed_children(&mut self) {
        if self.columns.is_empty() || self.rows.is_empty() {
            return;
        }

        let n_col = self.columns.len();
        let n_row = self.rows.len();
        let mut occupied = vec![false; n_col * n_row];
        let idx = |col: usize, row: usize| row * n_col + col;

        let place = |occupied: &mut [bool], child: &mut Child, col: u16, row: u16| {
            if col + child.params.width > n_col as u16 || row + child.params.height > n_row as u16 {
                debug_panic!("Child {child:?} with position ({col}, {row}) is out of grid bounds");
                return;
            }

            child.x = col;
            child.y = row;

            for row in row..row + child.params.height {
                let start = idx(col as _, row as _);
                occupied[start..start + child.params.width as usize].fill(true);
            }
        };

        // Pass 1: place children with fully explicit position
        for child in &mut self.children {
            if let (Some(col), Some(row)) = (child.params.col, child.params.row) {
                place(&mut occupied, child, col, row);
            }
        }

        // Pass 2: auto-placement
        let mut cursor = (0_usize, 0_usize);

        let place_and_advance =
            |occupied: &mut [bool],
             child: &mut Child,
             col: u16,
             row: u16,
             (cursor_col, cursor_row): &mut (usize, usize)| {
                place(occupied, child, col, row);

                *cursor_col = (col + child.params.width) as usize % n_col;
                *cursor_row = row as usize + (*cursor_col == 0) as usize;
            };

        let mut prefix = vec![0; occupied.len()];
        'child: for child in &mut self.children {
            let width = child.params.width as usize;
            let height = child.params.height as usize;

            match (child.params.col, child.params.row) {
                (Some(_), Some(_)) => continue, // already placed in pass 1
                (Some(col), None) => {
                    let mut consecutive = 0;
                    for row in 0..n_row {
                        let blocked =
                            (col..col + child.params.width).any(|c| occupied[idx(c as _, row)]);
                        if blocked {
                            consecutive = 0;
                            continue;
                        }
                        consecutive += 1;
                        if consecutive == child.params.height {
                            let start_row = row as u16 + 1 - child.params.height;
                            place_and_advance(&mut occupied, child, col, start_row, &mut cursor);
                            continue 'child;
                        }
                    }
                    debug_panic!(
                        "Child {child:?} could not be placed in the grid, no row has enough space",
                    );
                }
                (None, Some(row)) => {
                    let mut consecutive = 0;
                    for col in 0..n_col {
                        let blocked =
                            (row..row + child.params.height).any(|r| occupied[idx(col, r as _)]);
                        if blocked {
                            consecutive = 0;
                            continue;
                        }
                        consecutive += 1;
                        if consecutive == child.params.width {
                            let start_col = col as u16 + 1 - child.params.width;
                            place_and_advance(&mut occupied, child, start_col, row, &mut cursor);
                            continue 'child;
                        }
                    }
                    debug_panic!(
                        "Child {child:?} could not be placed in the grid, no column has enough space",
                    );
                }
                (None, None) => {
                    // build prefix sum
                    prefix.fill(0);
                    for col in 0..self.columns.len() {
                        for row in 0..self.rows.len() {
                            let cell = occupied[idx(col, row)] as u32;
                            let top = (row.checked_sub(1)).map_or(0, |r| prefix[idx(col, r)]);
                            let left = (col.checked_sub(1)).map_or(0, |c| prefix[idx(c, row)]);
                            let diag = (col.checked_sub(1))
                                .zip(row.checked_sub(1))
                                .map_or(0, |(c, r)| prefix[idx(c, r)]);

                            prefix[idx(col, row)] = cell + top + left - diag;
                        }
                    }

                    // query for empty rect
                    for row0 in cursor.1..=(n_row - height) {
                        let start_col = if row0 == cursor.1 { cursor.0 } else { 0 };
                        for col0 in start_col..=(n_col - width) {
                            let col1 = col0 + width - 1;
                            let row1 = row0 + height - 1;

                            let cell = prefix[idx(col1, row1)];
                            let top = (row0.checked_sub(1)).map_or(0, |r| prefix[idx(col1, r)]);
                            let left = (col0.checked_sub(1)).map_or(0, |c| prefix[idx(c, row1)]);
                            let diag = (col0.checked_sub(1))
                                .zip(row0.checked_sub(1))
                                .map_or(0, |(c, r)| prefix[idx(c, r)]);

                            if cell + diag - top - left == 0 {
                                place_and_advance(
                                    &mut occupied,
                                    child,
                                    col0 as _,
                                    row0 as _,
                                    &mut cursor,
                                );
                                continue 'child;
                            }
                        }
                    }

                    debug_panic!("Child {child:?} could not be placed in the grid");
                }
            }
        }
    }

    fn resolve_track_lengths(
        &mut self,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<Length>,
        gap: Length,
        context_length: Option<Length>,
        mut compute_length: impl FnMut(
            &mut WidgetPod<dyn Widget + 'static>,
            LenDef,
            LayoutSize,
            Axis,
            Option<Length>,
        ) -> Length,
    ) -> Vec<f64> {
        let tracks = match axis {
            Axis::Horizontal => &self.columns,
            Axis::Vertical => &self.rows,
        };

        if tracks.is_empty() {
            return Vec::new();
        }

        let context_length =
            context_length.map(|l| (l.get() - gap.get() * (tracks.len() - 1) as f64).px());

        let (track_auto_lengths, mut track_lengths): (Vec<_>, Vec<_>) = tracks
            .iter()
            .map(|t| match *t {
                // conforms to min-content measure of self
                GridTrackSize::Auto | GridTrackSize::MinContent => (LenDef::MinContent, 0.),
                GridTrackSize::MaxContent | GridTrackSize::Fraction(_) => (LenDef::MaxContent, 0.),
                GridTrackSize::FitContent(limit) => (LenDef::FitContent(limit), 0.),
                GridTrackSize::Fixed(length) => (LenDef::Fixed(length), length.get()),
                GridTrackSize::Percentage(percent) => match len_req {
                    LenReq::FitContent(space) => {
                        let fixed = space.get() * percent;
                        (LenDef::FitContent(fixed.px()), fixed)
                    }
                    _ => match context_length {
                        Some(length) => {
                            let fixed = length.get() * percent;
                            (LenDef::Fixed(fixed.px()), fixed)
                        }
                        _ => (LenDef::MinContent, 0.),
                    },
                },
            })
            .unzip();

        let mut multitrack_lengths = BTreeMap::new();

        for child in &mut self.children {
            if child.span(axis) == 1 {
                let track = child.pos(axis) as usize;
                let auto_length = track_auto_lengths[track];
                if !matches!(auto_length, LenDef::Fixed(_)) {
                    let computed = compute_length(
                        &mut child.widget,
                        auto_length,
                        LayoutSize::NONE,
                        axis,
                        cross_length,
                    );
                    track_lengths[track] = computed.get().max(track_lengths[track]);
                };
            } else {
                let pos = child.pos(axis);
                let span = child.span(axis);

                let start_track = pos as usize;
                let end_track = start_track + span as usize;

                let Some((min_length, max_length)) =
                    multitrack_lengths.entry((span, pos)).or_insert_with(|| {
                        tracks[start_track..end_track]
                            .iter()
                            .all(|t| !matches!(t, GridTrackSize::Fraction(_)))
                            .then_some((0., 0.))
                    })
                else {
                    continue;
                };

                let computed_min = compute_length(
                    &mut child.widget,
                    LenDef::MinContent,
                    LayoutSize::NONE,
                    axis,
                    cross_length,
                );
                let computed_max = compute_length(
                    &mut child.widget,
                    LenDef::MaxContent,
                    LayoutSize::NONE,
                    axis,
                    cross_length,
                );

                *min_length = computed_min.get().max(*min_length);
                *max_length = computed_max.get().max(*max_length);
            }
        }

        for ((span, pos), lengths) in multitrack_lengths {
            let Some((min_length, max_length)) = lengths else {
                continue;
            };

            let start_track = pos as usize;
            let end_track = start_track + span as usize;

            let [n_intrinsic, n_max] = track_auto_lengths[start_track..end_track]
                .iter()
                .map(|auto_length| match auto_length {
                    LenDef::MinContent | LenDef::FitContent(_) => [1_usize, 1],
                    LenDef::MaxContent => [0, 1],
                    LenDef::Fixed(_) => [0, 0],
                })
                .reduce(|[a, b], [c, d]| [a + c, b + d])
                .unwrap_or_default();
            let length_by_itself = track_lengths[start_track..end_track]
                .iter()
                .copied()
                .sum::<f64>()
                + gap.get() * span as f64;

            let min_contrib = min_length - length_by_itself;
            if min_contrib <= 0. {
                continue;
            }

            let per_track_min_contrib = min_contrib / n_intrinsic as f64;
            let (n_unfit, pushed) = track_auto_lengths[start_track..end_track]
                .iter()
                .zip(&mut track_lengths[start_track..])
                .filter_map(|(auto_length, length)| match *auto_length {
                    LenDef::FitContent(limit) if *length + per_track_min_contrib > limit.get() => {
                        Some(limit.get() - mem::replace(length, limit.get()))
                    }
                    _ => None,
                })
                .enumerate()
                .reduce(|(_, a), (i, b)| (i + 1, a + b))
                .unwrap_or((0, 0.));

            let per_track_min_contrib = (min_contrib - pushed) / (n_intrinsic - n_unfit) as f64;
            let per_track_max_contrib =
                per_track_min_contrib + (max_length - min_length) / n_max as f64;
            for (auto_length, length) in track_auto_lengths[start_track..end_track]
                .iter()
                .zip(&mut track_lengths[start_track..])
            {
                match auto_length {
                    LenDef::MinContent | LenDef::FitContent(_) => *length += per_track_min_contrib,
                    LenDef::MaxContent => *length += per_track_max_contrib,
                    LenDef::Fixed(_) => {}
                }
            }
        }

        let extract_bases = |track_lengths: Vec<f64>| track_lengths;
        let extract_bases_with_fr = |track_lengths: Vec<f64>, fr_unit: f64| {
            tracks
                .iter()
                .zip(track_lengths)
                .map(|(track, length)| match track {
                    GridTrackSize::Fraction(fr) => *fr * fr_unit,
                    _ => length,
                })
                .collect()
        };

        if len_req == LenReq::MinContent {
            return extract_bases_with_fr(track_lengths, 0.);
        }

        let Some(context_length) = context_length else {
            return if let Some(fr_unit) = tracks
                .iter()
                .zip(&track_lengths)
                .filter_map(|(track, length)| match track {
                    GridTrackSize::Fraction(fr) => Some(length / *fr),
                    _ => None,
                })
                .reduce(f64::max)
            {
                extract_bases_with_fr(track_lengths, fr_unit)
            } else {
                extract_bases(track_lengths)
            };
        };

        let (occupied_length, total_fr) = tracks
            .iter()
            .zip(&track_lengths)
            .map(|(track, length)| match *track {
                GridTrackSize::Fraction(fr) => (0., fr),
                _ => (*length, 0.),
            })
            .reduce(|(a, b), (c, d)| (a + c, b + d))
            .unwrap();

        let leftover_length = context_length.get() - occupied_length;
        if leftover_length <= 0. {
            return extract_bases(track_lengths);
        }
        if total_fr == 0. {
            let n_auto = tracks
                .iter()
                .filter(|t| matches!(t, GridTrackSize::Auto))
                .count();
            if n_auto != 0 {
                let per_auto_track_ext = leftover_length / n_auto as f64;
                tracks
                    .iter()
                    .zip(track_lengths)
                    .map(|(track, length)| match track {
                        GridTrackSize::Auto => length + per_auto_track_ext,
                        _ => length,
                    })
                    .collect()
            } else {
                extract_bases(track_lengths)
            }
        } else {
            extract_bases_with_fr(track_lengths, leftover_length / total_fr)
        }
    }
}

// --- MARK: IMPL CHILD
impl Child {
    fn new(child: NewWidget<impl Widget + ?Sized>, params: impl Into<GridParams>) -> Self {
        Self {
            widget: child.erased().to_pod(),
            params: params.into(),

            x: 0,
            y: 0,
        }
    }

    /// Returns the number of cells the child's area spans on the given `axis`.
    fn span(&self, axis: Axis) -> u16 {
        match axis {
            Axis::Horizontal => self.params.width,
            Axis::Vertical => self.params.height,
        }
    }

    fn pos(&self, axis: Axis) -> u16 {
        match axis {
            Axis::Horizontal => self.x,
            Axis::Vertical => self.y,
        }
    }
}

// --- MARK: IMPL GRIDTRACKSIZE
impl GridTrackSize {
    /// *One (1)* Fraction of remaining space.
    pub const FRACTION: Self = Self::Fraction(1.);
}

impl Default for GridTrackSize {
    fn default() -> Self {
        Self::FRACTION
    }
}

// --- MARK: IMPL GRIDPARAMS
impl GridParams {
    /// Creates grid parameters with the defaulted values.
    ///
    /// It sets the cell span to 1×1.
    pub fn new() -> Self {
        Self {
            col: None,
            row: None,
            width: 1,
            height: 1,
        }
    }

    /// Creates grid parameters for the given position.
    ///
    /// It sets the cell span to 1×1.
    pub fn pos(col: u16, row: u16) -> Self {
        Self::new().with_col(col).with_row(row)
    }

    /// Builder-style method to define at which column in the grid the item should start.
    pub fn with_col(mut self, col: u16) -> Self {
        self.col = Some(col);
        self
    }

    /// Builder-style method to define at which row in the grid the item should start.
    pub fn with_row(mut self, row: u16) -> Self {
        self.row = Some(row);
        self
    }

    /// Builder-style method to define at which column and row in the grid the item should start.
    pub fn with_pos(self, col: u16, row: u16) -> Self {
        self.with_col(col).with_row(row)
    }

    /// Builder-style method to define how many column in the grid the item should occupy.
    ///
    /// # Panics
    ///
    /// When debug assertions are on, panics if the width is zero.
    pub fn with_width(mut self, mut width: u16) -> Self {
        if width == 0 {
            debug_panic!(
                "Grid width value should be a positive nonzero number; got {}",
                width
            );
            width = 1;
        }
        self.width = width;
        self
    }

    /// Builder-style method to define how many row in the grid the item should occupy.
    ///
    /// # Panics
    ///
    /// When debug assertions are on, panics if the height is zero.
    pub fn with_height(mut self, mut height: u16) -> Self {
        if height == 0 {
            debug_panic!(
                "Grid height value should be a positive nonzero number; got {}",
                height
            );
            height = 1;
        }
        self.height = height;
        self
    }

    /// Builder-style method to define how many column and row in the grid the item should occupy.
    ///
    /// # Panics
    ///
    /// When debug assertions are on, panics if either the width or the height is zero.
    pub fn with_span(self, width: u16, height: u16) -> Self {
        self.with_width(width).with_height(height)
    }
}

impl Default for GridParams {
    fn default() -> Self {
        Self::new()
    }
}

impl From<(u16, u16)> for GridParams {
    fn from((col, row): (u16, u16)) -> Self {
        Self::pos(col, row)
    }
}

impl From<(u16, u16, u16, u16)> for GridParams {
    fn from((col, row, width, height): (u16, u16, u16, u16)) -> Self {
        Self::pos(col, row).with_span(width, height)
    }
}

impl From<(u16, ())> for GridParams {
    fn from((col, ()): (u16, ())) -> Self {
        Self::new().with_col(col)
    }
}

impl From<((), u16)> for GridParams {
    fn from(((), row): ((), u16)) -> Self {
        Self::new().with_row(row)
    }
}

impl From<((), u16, u16, u16)> for GridParams {
    fn from(((), row, width, height): ((), u16, u16, u16)) -> Self {
        Self::new().with_row(row).with_span(width, height)
    }
}

impl From<(u16, (), u16, u16)> for GridParams {
    fn from((col, (), width, height): (u16, (), u16, u16)) -> Self {
        Self::new().with_col(col).with_span(width, height)
    }
}

impl From<((), (), u16, u16)> for GridParams {
    fn from(((), (), width, height): ((), (), u16, u16)) -> Self {
        Self::new().with_span(width, height)
    }
}

impl From<((), (), u16, ())> for GridParams {
    fn from(((), (), width, ()): ((), (), u16, ())) -> Self {
        Self::new().with_width(width)
    }
}

impl From<((), (), (), u16)> for GridParams {
    fn from(((), (), (), height): ((), (), (), u16)) -> Self {
        Self::new().with_height(height)
    }
}

impl From<(u16, u16, (), u16)> for GridParams {
    fn from((col, row, (), height): (u16, u16, (), u16)) -> Self {
        Self::pos(col, row).with_height(height)
    }
}

impl From<(u16, u16, u16, ())> for GridParams {
    fn from((col, row, width, ()): (u16, u16, u16, ())) -> Self {
        Self::pos(col, row).with_width(width)
    }
}

impl From<((), u16, (), u16)> for GridParams {
    fn from(((), row, (), height): ((), u16, (), u16)) -> Self {
        Self::new().with_row(row).with_height(height)
    }
}

impl From<((), u16, u16, ())> for GridParams {
    fn from(((), row, width, ()): ((), u16, u16, ())) -> Self {
        Self::new().with_row(row).with_width(width)
    }
}

impl From<(u16, (), (), u16)> for GridParams {
    fn from((col, (), (), height): (u16, (), (), u16)) -> Self {
        Self::new().with_col(col).with_height(height)
    }
}

impl From<(u16, (), u16, ())> for GridParams {
    fn from((col, (), width, ()): (u16, (), u16, ())) -> Self {
        Self::new().with_col(col).with_width(width)
    }
}

impl From<()> for GridParams {
    fn from((): ()) -> Self {
        Self::new()
    }
}

// --- MARK: WIDGETMUT
impl Grid {
    /// Sets the column sizes of the grid.
    pub fn set_columns(this: &mut WidgetMut<'_, Self>, columns: Vec<GridTrackSize>) {
        this.widget.columns = columns;
        this.ctx.request_layout();
    }

    /// Sets the row sizes of the grid.
    pub fn set_rows(this: &mut WidgetMut<'_, Self>, rows: Vec<GridTrackSize>) {
        this.widget.rows = rows;
        this.ctx.request_layout();
    }
}

// --- MARK: COLLECTIONWIDGET
impl CollectionWidget<GridParams> for Grid {
    /// Returns the number of children.
    fn len(&self) -> usize {
        self.children.len()
    }

    /// Returns `true` if there are no children.
    fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    /// Returns a mutable reference to the child widget at the given index.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds.
    fn get_mut<'t>(this: &'t mut WidgetMut<'_, Self>, idx: usize) -> WidgetMut<'t, dyn Widget> {
        let child = &mut this.widget.children[idx].widget;
        this.ctx.get_mut(child)
    }

    /// Appends a child widget to the collection.
    fn add(
        this: &mut WidgetMut<'_, Self>,
        child: NewWidget<impl Widget + ?Sized>,
        params: impl Into<GridParams>,
    ) {
        let child = Child::new(child, params);
        this.widget.children.push(child);
        this.ctx.children_changed();
    }

    /// Inserts a child widget at the given index.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is larger than the number of children.
    fn insert(
        this: &mut WidgetMut<'_, Self>,
        idx: usize,
        child: NewWidget<impl Widget + ?Sized>,
        params: impl Into<GridParams>,
    ) {
        let child = Child::new(child, params);
        this.widget.children.insert(idx, child);
        this.ctx.children_changed();
    }

    /// Replaces the child widget at the given index.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds.
    fn set(
        this: &mut WidgetMut<'_, Self>,
        idx: usize,
        child: NewWidget<impl Widget + ?Sized>,
        params: impl Into<GridParams>,
    ) {
        let child = Child::new(child, params);
        let old_child = mem::replace(&mut this.widget.children[idx], child);
        this.ctx.remove_child(old_child.widget);
    }

    /// Sets the child params at the given index.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds.
    fn set_params(this: &mut WidgetMut<'_, Self>, idx: usize, params: impl Into<GridParams>) {
        let child = &mut this.widget.children[idx];
        child.params = params.into();
        this.ctx.request_layout();
    }

    /// Swaps the index of two children.
    ///
    /// This also swaps the [`GridParams`] `x` and `y` with the other child.
    ///
    /// # Panics
    ///
    /// Panics if `a` or `b` are out of bounds.
    fn swap(this: &mut WidgetMut<'_, Self>, a: usize, b: usize) {
        let (a_x, a_y) = (this.widget.children[a].x, this.widget.children[a].y);
        let (b_x, b_y) = (this.widget.children[b].x, this.widget.children[b].y);

        this.widget.children.swap(a, b);

        (this.widget.children[a].x, this.widget.children[a].y) = (a_x, a_y);
        (this.widget.children[b].x, this.widget.children[b].y) = (b_x, b_y);

        this.ctx.children_changed();
    }

    /// Removes the child at the given index.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds.
    fn remove(this: &mut WidgetMut<'_, Self>, idx: usize) {
        let child = this.widget.children.remove(idx);
        this.ctx.remove_child(child.widget);
    }

    /// Removes all children.
    fn clear(this: &mut WidgetMut<'_, Self>) {
        for child in this.widget.children.drain(..) {
            this.ctx.remove_child(child.widget);
        }
    }
}

impl UsesProperty<Gap> for Grid {}

// --- MARK: IMPL WIDGET
impl Widget for Grid {
    type Action = NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        for child in self.children.iter_mut() {
            ctx.register_child(&mut child.widget);
        }
    }

    fn property_changed(&mut self, ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        Gap::prop_changed(ctx, property_type);
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<Length>,
    ) -> Length {
        let cache = ctx.property_cache();
        let gap = props.get::<Gap>(cache);

        let gap_length = gap.gap.get();

        self.assign_cell_to_auto_placed_children();

        let track_lengths = self.resolve_track_lengths(
            axis,
            len_req,
            cross_length,
            gap.gap,
            ctx.context_size().length(axis),
            |child, auto_length, context_size, axis, cross_length| {
                ctx.compute_length(child, auto_length, context_size, axis, cross_length)
            },
        );

        if track_lengths.is_empty() {
            return 0.px();
        }

        let computed = Length::px(
            gap_length * (track_lengths.len() - 1) as f64 + track_lengths.into_iter().sum::<f64>(),
        );

        match len_req {
            LenReq::FitContent(space) => computed.min(space),
            _ => computed,
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, props: &PropertiesRef<'_>, size: Size) {
        let cache = ctx.property_cache();
        let gap = props.get::<Gap>(cache);

        let gap_length = gap.gap.get();

        self.assign_cell_to_auto_placed_children();

        let mut resolve_track = |grid: &mut Self, axis: Axis| {
            grid.resolve_track_lengths(
                axis,
                LenReq::FitContent(size.get_coord(axis).px()),
                Some(size.get_coord(axis.cross()).px()),
                gap.gap,
                Some(size.get_coord(axis).px()),
                |child, auto_length, context_size, axis, cross_length| {
                    ctx.compute_length(child, auto_length, context_size, axis, cross_length)
                },
            )
            .into_iter()
            .scan(0., |offset, length| {
                Some(mem::replace(offset, *offset + length + gap_length))
            })
            .collect::<Vec<_>>()
        };

        let col_offsets = resolve_track(self, Axis::Horizontal);
        let row_offsets = resolve_track(self, Axis::Vertical);

        let mut baselines = None::<(u16, u16, f64, u16, u16, f64)>;

        for child in &mut self.children {
            let col = child.x as usize;
            let row = child.y as usize;
            let width = child.params.width as usize;
            let height = child.params.height as usize;

            if col + width > self.columns.len() || row + height > self.rows.len() {
                continue;
            }

            let size = Size::new(
                col_offsets
                    .get(col + width)
                    .map_or(size.width, |o| o - gap_length)
                    - col_offsets[col],
                row_offsets
                    .get(row + height)
                    .map_or(size.height, |o| o - gap_length)
                    - row_offsets[row],
            );

            ctx.run_layout(&mut child.widget, size);

            let origin = Point::new(col_offsets[col], row_offsets[row]);
            ctx.place_child(&mut child.widget, origin);

            let (first_baseline, last_baseline) = ctx.child_aligned_baselines(&child.widget);
            let last_row = child.y + child.params.height;

            baselines = Some(match baselines {
                None => (
                    child.x,
                    child.y,
                    origin.y + first_baseline,
                    child.x,
                    last_row,
                    origin.y + last_baseline,
                ),
                Some((mut col0, mut row0, mut base0, mut col1, mut row1, mut base1)) => {
                    if (child.y, child.x) < (row0, col0) {
                        (col0, row0, base0) = (child.x, child.y, origin.y + first_baseline);
                    }
                    if (last_row, child.x) > (row1, col1) {
                        (col1, row1, base1) = (child.x, last_row, origin.y + last_baseline);
                    }
                    (col0, row0, base0, col1, row1, base1)
                }
            });
        }

        if let Some((_, _, first_baseline, _, _, last_baseline)) = baselines {
            ctx.set_baselines(first_baseline, last_baseline);
        } else {
            ctx.clear_baselines();
        }
    }

    fn paint(
        &mut self,
        _ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        _painter: &mut Painter<'_>,
    ) {
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        self.children
            .iter()
            .map(|child| child.widget.id())
            .collect()
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("Grid", id = id.trace())
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::AsUnit;
    use crate::palette;
    use crate::properties::types::CrossAxisAlignment;
    use crate::properties::{Background, Dimensions, Padding};
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::test_property_set;
    use crate::widgets::{Button, Flex, Label};

    #[test]
    fn test_grid_basics() {
        // Start with a 1x1 grid
        let widget = NewWidget::new(
            Grid::new()
                .with_column(GridTrackSize::FRACTION)
                .with_row(GridTrackSize::FRACTION)
                .with(Button::with_text("A").prepare(), (0, 0)),
        )
        .with_props(Dimensions::STRETCH);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, (200, 200));
        // Snapshot with the single widget.
        assert_render_snapshot!(harness, "grid_initial_1x1");

        // Expand it to a 4x4 grid
        harness.edit_root_widget(|mut grid| {
            Grid::set_columns(&mut grid, vec![GridTrackSize::FRACTION; 4]);
        });
        assert_render_snapshot!(harness, "grid_expanded_4x1");

        harness.edit_root_widget(|mut grid| {
            Grid::set_rows(&mut grid, vec![GridTrackSize::FRACTION; 4]);
        });
        assert_render_snapshot!(harness, "grid_expanded_4x4");

        // Add a widget that takes up more than one horizontal cell
        harness.edit_root_widget(|mut grid| {
            Grid::add(&mut grid, Button::with_text("B").prepare(), (1, 0, 3, 1));
        });
        assert_render_snapshot!(harness, "grid_with_horizontal_widget");

        // Add a widget that takes up more than one vertical cell
        harness.edit_root_widget(|mut grid| {
            Grid::add(&mut grid, Button::with_text("C").prepare(), (0, 1, 1, 3));
        });
        assert_render_snapshot!(harness, "grid_with_vertical_widget");

        // Add a widget that takes up more than one horizontal and vertical cell
        harness.edit_root_widget(|mut grid| {
            Grid::add(&mut grid, Button::with_text("D").prepare(), (1, 1, 2, 2));
        });
        assert_render_snapshot!(harness, "grid_with_2x2_widget");

        // Change the gap
        harness.edit_root_widget(|mut grid| {
            grid.insert_prop(Gap::new(7.px()));
        });
        assert_render_snapshot!(harness, "grid_with_changed_spacing");
    }

    #[test]
    fn test_widget_removal_and_modification() {
        let widget = NewWidget::new(
            Grid::new()
                .with_columns([GridTrackSize::FRACTION; 2])
                .with_rows([GridTrackSize::FRACTION; 2])
                .with(Button::with_text("A").prepare(), (0, 0)),
        );
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, (200, 200));
        // Snapshot with the single widget.
        assert_render_snapshot!(harness, "grid_initial_2x2");

        // Now remove the widget
        harness.edit_root_widget(|mut grid| {
            Grid::remove(&mut grid, 0);
        });
        assert_render_snapshot!(harness, "grid_2x2_with_removed_widget");

        // Add it back
        harness.edit_root_widget(|mut grid| {
            Grid::add(&mut grid, Button::with_text("A").prepare(), (0, 0));
        });
        assert_render_snapshot!(harness, "grid_initial_2x2"); // Should be back to the original state

        // Test replacement
        harness.edit_root_widget(|mut grid| {
            Grid::remove(&mut grid, 0);
            Grid::add(&mut grid, Button::with_text("X").prepare(), (0, 0));
        });
        harness.edit_root_widget(|mut grid| {
            Grid::set(&mut grid, 0, Button::with_text("A").prepare(), (0, 0));
        });
        assert_render_snapshot!(harness, "grid_initial_2x2"); // Should be back to the original state

        // Change the grid params to position it on the other corner
        harness.edit_root_widget(|mut grid| {
            Grid::set_params(&mut grid, 0, (1, 1));
        });
        assert_render_snapshot!(harness, "grid_moved_2x2_1");

        // Now make it take up the entire grid
        harness.edit_root_widget(|mut grid| {
            Grid::set_params(&mut grid, 0, (0, 0, 2, 2));
        });
        assert_render_snapshot!(harness, "grid_moved_2x2_2");
    }

    #[test]
    fn test_widget_order() {
        let widget = NewWidget::new(
            Grid::new()
                .with_columns([GridTrackSize::FRACTION; 2])
                .with_rows([GridTrackSize::FRACTION; 2])
                .with(Button::with_text("A").prepare(), (0, 0)),
        );
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, (200, 200));
        // Snapshot with the single widget.
        assert_render_snapshot!(harness, "grid_initial_2x2");

        // Order sets the draw order, so draw a widget over A by adding it after
        harness.edit_root_widget(|mut grid| {
            Grid::add(&mut grid, Button::with_text("B").prepare(), (0, 0));
        });
        assert_render_snapshot!(harness, "grid_2x2_with_overlapping_b");

        // Draw a widget under the others by putting it at index 0
        // Make it wide enough to see it stick out, with half of it under A and B.
        harness.edit_root_widget(|mut grid| {
            Grid::insert(&mut grid, 0, Button::with_text("C").prepare(), (0, 0, 2, 1));
        });
        assert_render_snapshot!(harness, "grid_2x2_with_overlapping_c");
    }

    #[test]
    fn grid_baselines() {
        let grid = Grid::new()
            .with_columns([GridTrackSize::FRACTION; 3])
            .with_rows([GridTrackSize::FRACTION; 3])
            .with(
                Label::new("A\nB").prepare().with_props((
                    Padding::from_vh(0.px(), 0.px()),
                    Background::Color(palette::css::ORANGE),
                )),
                (1, 0),
            )
            .with(
                Label::new("C\nD").prepare().with_props((
                    Padding::from_vh(8.px(), 0.px()),
                    Background::Color(palette::css::DARK_BLUE),
                )),
                (0, 0, 1, 2),
            )
            .with(
                Label::new("E\nF").prepare().with_props((
                    Padding::from_vh(16.px(), 0.px()),
                    Background::Color(palette::css::DARK_SALMON),
                )),
                (2, 0, 1, 3),
            )
            .with(
                Label::new("G\nH").prepare().with_props((
                    Padding::from_vh(24.px(), 0.px()),
                    Background::Color(palette::css::DARK_SLATE_BLUE),
                )),
                (1, 1, 1, 2),
            )
            .prepare()
            .with_props(Dimensions::width(80.px()));

        let root = Flex::row()
            .cross_axis_alignment(CrossAxisAlignment::FirstBaseline)
            .with_fixed(Label::new("Out").prepare())
            .with_fixed(grid)
            .prepare()
            .with_props(Padding::all(10.px()));

        let mut harness = TestHarness::create_with_size(test_property_set(), root, (150, 200));

        assert_render_snapshot!(harness, "grid_baselines_first");

        harness.edit_root_widget(|mut root| {
            Flex::set_cross_axis_alignment(&mut root, CrossAxisAlignment::LastBaseline);
        });

        assert_render_snapshot!(harness, "grid_baselines_last");
    }

    #[test]
    fn grid_nonuniform() {
        let widget = NewWidget::new(
            Grid::new()
                .with_columns([
                    GridTrackSize::MinContent,
                    GridTrackSize::MaxContent,
                    GridTrackSize::Fraction(1.),
                ])
                .with_rows([
                    GridTrackSize::Auto,
                    GridTrackSize::Fixed(30.px()),
                    GridTrackSize::Percentage(0.2),
                    GridTrackSize::Fraction(2.),
                    GridTrackSize::Fraction(1.),
                ])
                .with(
                    Label::new("Min Content")
                        .prepare()
                        .with_props(Background::Color(palette::css::CHOCOLATE.with_alpha(0.5))),
                    (),
                )
                .with(
                    Label::new("Max Content")
                        .prepare()
                        .with_props(Background::Color(palette::css::OLIVE.with_alpha(0.5))),
                    (),
                )
                .with(
                    Label::new("1×3")
                        .prepare()
                        .with_props(Background::Color(palette::css::ORANGE_RED.with_alpha(0.5))),
                    (2, 0, (), 3),
                )
                .with(
                    Label::new("20px")
                        .prepare()
                        .with_props(Background::Color(palette::css::MAGENTA.with_alpha(0.5))),
                    (1, ()),
                )
                .with(
                    Label::new("30%")
                        .prepare()
                        .with_props(Background::Color(palette::css::SEA_GREEN.with_alpha(0.5))),
                    (),
                )
                .with(
                    Label::new("2×2")
                        .prepare()
                        .with_props(Background::Color(palette::css::AQUAMARINE.with_alpha(0.5))),
                    (1, 2, 2, 2),
                )
                .with(
                    Label::new("2fr")
                        .prepare()
                        .with_props(Background::Color(palette::css::PURPLE.with_alpha(0.5))),
                    (),
                )
                .with(
                    Label::new("1fr")
                        .prepare()
                        .with_props(Background::Color(palette::css::GOLD.with_alpha(0.5))),
                    (2, ()),
                ),
        )
        .with_props(Gap::new(5.px()))
        .with_props(Dimensions::STRETCH);

        let mut harness = TestHarness::create_with_size(test_property_set(), widget, (300, 300));
        assert_render_snapshot!(harness, "grid_nonuniform");
    }
}
