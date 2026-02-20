// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use accesskit::{Node, Role};
use include_doc_path::include_doc_path;
use masonry_core::debug_panic;
use smallvec::SmallVec;
use tracing::{Span, trace_span};
use vello::Scene;

use crate::core::{
    AccessCtx, ArcStr, ChildrenIds, HasProperty, LayoutCtx, MeasureCtx, NewWidget, NoAction,
    PaintCtx, PropertiesMut, PropertiesRef, Property, RegisterCtx, Update, UpdateCtx, Widget,
    WidgetId, WidgetPod,
};
use crate::kurbo::{Affine, Axis, Cap, Join, Line, Size, Stroke};
use crate::layout::LenReq;
use crate::layout::{LayoutSize, Length, SizeDef, UnitPoint};
use crate::properties::ContentColor;
use crate::widgets::Label;

// TODO: Do proper hairline layout/paint after scale rework has happened.

/// A line to divide your content.
///
/// By default it is a thin solid line. A dash pattern can be configured with [`dash_pattern`].
///
/// There is support for a [`label`] and even arbitrary [`content`].
///
/// It has the following properties:
/// * [`ContentColor`] - defines the color of the dividing line.
///
/// [`dash_pattern`]: Self::dash_pattern
/// [`label`]: Self::label
/// [`content`]: Self::content
/// [`ContentColor`]: ContentColor
///
#[doc = concat!(
    "![Styled divider examples](",
    include_doc_path!("screenshots/divider_styled.png"),
    ")",
)]
pub struct Divider {
    axis: Axis,
    /// No set thickness means hairline - 1 device pixel.
    thickness: Option<Length>,
    dash_fit: DashFit,
    dash_pattern: SmallVec<[Length; 2]>,
    start_cap: Cap,
    end_cap: Cap,
    placement: Placement,
    content: Option<WidgetPod<dyn Widget>>,
    pad: Length,
    lines: SmallVec<[LineLayout; 2]>,
}

/// Describes the strategy how to display a dashed divider.
///
/// It has no effect on a solid line divider.
#[derive(Default, Copy, Clone, Debug)]
pub enum DashFit {
    /// Clip the end edge.
    ///
    /// The start edge is flush with a dash.
    /// Depending on the dash pattern and available space, the end edge might have a larger gap
    /// or the end edge might have a shorter partially drawn dash.
    ///
    /// Can look unpolished in static scenarios due to a break in the pattern at the end.
    /// Has stable visuals when resizing the divider,
    /// with end edge dashes gradually appearing or disappearing.
    #[default]
    Clip,
    /// Stretch the inner gap sizes to ensure only whole dashes are shown.
    ///
    /// The start and end edges are flush with dashes.
    ///
    /// Has a polished look in static scenarios due to a consistent dash and gap pattern.
    /// Has a wobble effect when resizing the divider,
    /// subtle at the edges and significant in the middle.
    Stretch,
    /// Draw only whole dashes aligned to the start.
    ///
    /// The start edge will be flush with a dash.
    /// The end edge will have a gap.
    ///
    /// Has a good look in static scenarios due to a consistent dash pattern. However,
    /// if there is a background color or a border then those may reveal a larger gap at the end.
    /// For such static scenarios [`Center`] and [`Stretch`] will work better.
    /// Has stable visuals when resizing the divider,
    /// with end edge dashes popping in and out of existence.
    ///
    /// [`Center`]: DashFit::Center
    /// [`Stretch`]: DashFit::Stretch
    Start,
    /// Draw only whole dashes aligned to the center.
    ///
    /// The start and edge edges will have equally sized gaps.
    ///
    /// Has a polished look in static scenarios due to a consistent dash and gap pattern.
    /// Has a wobble effect when resizing the divider,
    /// uniformly affecting the whole line.
    Center,
    /// Draw only whole dashes aligned to the end.
    ///
    /// The start edge will have a gap.
    /// The end edge will be flush with a dash.
    ///
    /// Has a good look in static scenarios due to a consistent dash pattern. However,
    /// if there is a background color or a border then those may reveal a larger gap at the start.
    /// For such static scenarios [`Center`] and [`Stretch`] will work better.
    /// Has a dragging effect when resizing the divider,
    /// with start edge dashes popping in and out of existence.
    ///
    /// [`Center`]: DashFit::Center
    /// [`Stretch`]: DashFit::Stretch
    End,
}

/// Describes the strategy where to place the divider's content.
#[derive(Default, Copy, Clone, Debug)]
pub enum Placement {
    /// Place the content at the start.
    ///
    /// There will be a single divider line after the content until the end edge.
    Start,
    /// Place the content at the center.
    ///
    /// There will be two divider lines on each side of the content until the start/end edges.
    #[default]
    Center,
    /// Place the content at the end.
    ///
    /// There will be a single divider line from the start until the content's start edge.
    End,
}

/// Cached layout of divider lines.
#[derive(Clone, Debug)]
struct LineLayout {
    line: Line,
    dashes: SmallVec<[f64; 4]>,
}

// --- MARK: BUILDERS
impl Divider {
    /// Creates a new [`Divider`] parallel with the given `axis`.
    pub fn new(axis: Axis) -> Self {
        Self {
            axis,
            thickness: None,
            dash_fit: DashFit::default(),
            dash_pattern: SmallVec::default(),
            start_cap: Cap::Butt,
            end_cap: Cap::Butt,
            placement: Placement::default(),
            content: None,
            pad: Length::const_px(5.),
            lines: SmallVec::default(),
        }
    }

    /// Creates a new horizontal [`Divider`].
    pub fn horizontal() -> Self {
        Self::new(Axis::Horizontal)
    }

    /// Creates a new vertical [`Divider`].
    pub fn vertical() -> Self {
        Self::new(Axis::Vertical)
    }

    /// Returns `self` with the given line `thickness`.
    pub fn thickness(mut self, thickness: Length) -> Self {
        self.thickness = Some(thickness);
        self
    }

    /// Returns `self` with the line thickness set to hairline, i.e. 1 device pixel.
    pub fn hairline(mut self) -> Self {
        self.thickness = None;
        self
    }

    /// Returns `self` with the given `dash_fit`.
    pub fn dash_fit(mut self, dash_fit: DashFit) -> Self {
        self.dash_fit = dash_fit;
        self
    }

    /// Returns `self` with the given `dash_pattern`.
    ///
    /// The pattern defines the lengths of dashes in alternating on/off order.
    /// * `10` - 10px dashes and 10px gaps
    /// * `10, 5` - 10px dashes with 5px gaps
    /// * `10, 5, 20, 30` - 10 px dash, 5px gap, 20px dash, 30px gap
    ///
    /// The pattern can be even longer and in any case will repeat to fill the whole divider space.
    ///
    /// The pattern must contain an even number of lengths. With exceptions for zero and one, where
    /// zero lengths means a solid line and one length will be used for both the dash and the gap.
    /// When given any other uneven number of the lengths, the last length will be ignored.
    ///
    /// # Panics
    ///
    /// Panics if `dash_pattern` contains an uneven number of entries of 3 or more
    /// and debug assertions are enabled.
    pub fn dash_pattern(mut self, dash_pattern: &[Length]) -> Self {
        let mut dash_pattern = SmallVec::from_slice(dash_pattern);
        // Paint code assumes an even number for simplicity of implementation.
        let len = dash_pattern.len();
        if len == 1 {
            dash_pattern.push(dash_pattern[0]);
        } else if len > 0 && !len.is_multiple_of(2) {
            debug_panic!(
                "The divider dash pattern must have an even number of lengths. Received {len}"
            );
            dash_pattern.pop();
        }
        self.dash_pattern = dash_pattern;
        self
    }

    /// Returns `self` with the given `cap` used both for start and end.
    ///
    /// Use [`start_cap`] or [`end_cap`] to set different edge caps.
    ///
    /// Defaults to [`Cap::Butt`].
    ///
    /// [`start_cap`]: Self::start_cap
    /// [`end_cap`]: Self::end_cap
    pub fn cap(mut self, cap: Cap) -> Self {
        self.start_cap = cap;
        self.end_cap = cap;
        self
    }

    /// Returns `self` with the given starting `cap`.
    ///
    /// Use [`cap`] to set the cap for both the start and the end.
    ///
    /// Defaults to [`Cap::Butt`].
    ///
    /// [`cap`]: Self::cap
    pub fn start_cap(mut self, cap: Cap) -> Self {
        self.start_cap = cap;
        self
    }

    /// Returns `self` with the given ending `cap`.
    ///
    /// Use [`cap`] to set the cap for both the start and the end.
    ///
    /// Defaults to [`Cap::Butt`].
    ///
    /// [`cap`]: Self::cap
    pub fn end_cap(mut self, cap: Cap) -> Self {
        self.end_cap = cap;
        self
    }

    /// Returns `self` with the given content `placement`.
    ///
    /// Defaults to [`Placement::Center`].
    pub fn placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }

    /// Returns `self` with the given `content`.
    ///
    /// For simple text labels use [`label`].
    ///
    /// [`label`]: Self::label
    pub fn content(mut self, content: NewWidget<impl Widget + ?Sized>) -> Self {
        self.content = Some(content.erased().to_pod());
        self
    }

    /// Returns `self` with the given `text` label.
    ///
    /// For more control over the label use [`content`].
    ///
    /// [`content`]: Self::content
    pub fn label(self, text: impl Into<ArcStr>) -> Self {
        self.content(Label::new(text).with_auto_id())
    }

    /// Returns `self` with the given `pad`.
    ///
    /// This `pad` determines the amount of space between the divider line and the content.
    /// It does nothing when there is no content.
    ///
    /// The default value is 5px.
    pub fn pad(mut self, pad: Length) -> Self {
        self.pad = pad;
        self
    }
}

/// Returns `(length, gap_counts)`.
///
/// `length` is the length of the stroke that will result in only whole dashes.
/// `gap_counts` specifies how many times each gap was used.
fn length_with_whole_dashes(space: f64, dashes: &[f64]) -> (f64, SmallVec<[f64; 3]>) {
    let n = dashes.len();

    let mut return_zero = false;
    if n < 2 || !n.is_multiple_of(2) {
        debug_panic!("Can only calculate whole dashes for actual dash pattern pairs");
        return_zero = true;
    }
    if dashes.iter().any(|&v| v < 0.) {
        debug_panic!("Negative dash lengths are not allowed");
        return_zero = true;
    }
    let cycle_length: f64 = dashes.iter().sum();
    if cycle_length <= 0. || space <= 0. {
        return_zero = true;
    }
    if return_zero {
        return (0., SmallVec::from_elem(0., n / 2));
    }

    // Start from the maximum number of full cycles that fit the space.
    let full_cycles = (space / cycle_length).floor();
    let mut gap_counts = SmallVec::from_elem(full_cycles, n / 2);
    let mut space_used = full_cycles * cycle_length;
    let mut space_free = space - space_used;

    // Track the index of the (dash, gap) pair of at the end of the line.
    let mut end_pair = n / 2 - 1;

    // Add as many (dash, gap) pairs as possible to form the final partial cycle.
    for (i, pair) in dashes.chunks_exact(2).enumerate() {
        let (dash, gap) = (pair[0], pair[1]);
        if dash > space_free {
            break;
        }
        space_used += dash + gap;
        space_free = space - space_used;
        gap_counts[i] += 1.;
        end_pair = i;
    }

    // We're measuring the length that ends with a dash, so remove the last gap.
    let length = if space_used > 0. {
        gap_counts[end_pair] -= 1.;
        let end_gap = dashes[end_pair * 2 + 1];
        space_used - end_gap
    } else {
        0.
    };

    (length, gap_counts)
}

/// Distributes the available `space` between all the gaps.
///
/// Takes into account both the occurrence of the gaps and their relative significance,
/// i.e. larger gaps get more of the available space.
fn stretch_gaps(dashes: &mut [f64], gap_counts: &[f64], space: f64) {
    if space <= 0. {
        return;
    }

    let n = dashes.len();
    let gaps: SmallVec<[f64; 3]> = (1..n).step_by(2).map(|i| dashes[i]).collect();

    if gaps.len() != gap_counts.len() {
        debug_panic!("Every gap needs to have an occurrence count");
        return;
    }

    let gaps_sum: f64 = gaps
        .iter()
        .zip(gap_counts.iter())
        .map(|(gap, count)| gap * count)
        .sum();
    let gap_counts_sum: f64 = gap_counts.iter().sum();

    for (idx, (gap, &count)) in gaps.into_iter().zip(gap_counts.iter()).enumerate() {
        if count <= 0. {
            continue;
        }
        // Guard against the case where all gaps have a zero length
        let gap_fraction = if gaps_sum != 0. {
            gap * count / gaps_sum
        } else {
            count / gap_counts_sum
        };
        let add_per_gap_type = space * gap_fraction;
        let add_per_occurrence = add_per_gap_type / count;
        dashes[idx * 2 + 1] += add_per_occurrence;
    }
}

/// Returns the `(start, end)` of the line in relation to the given `space`.
///
/// This ensures that a dashed line, described by `dashes`, will honor its `dash_fit` strategy.
/// For a solid line it will just return `(0., space)`.
///
/// Depending on the fit strategy `dashes` might be modified as well.
fn line_length(space: f64, dashes: &mut [f64], dash_fit: DashFit) -> (f64, f64) {
    let (start, length) = if dashes.is_empty() {
        // Solid line
        (0., space)
    } else {
        // Dashed line
        if let DashFit::Clip = dash_fit {
            (0., space)
        } else {
            let (length, gap_counts) = length_with_whole_dashes(space, dashes);
            let extra = space - length;
            match dash_fit {
                DashFit::Clip => unreachable!(),
                DashFit::Stretch => {
                    stretch_gaps(dashes, &gap_counts, extra);
                    (0., space)
                }
                DashFit::Start => (0., length),
                DashFit::Center => (extra * 0.5, length),
                DashFit::End => (extra, length),
            }
        }
    };
    (start, start + length)
}

impl Divider {
    /// Returns the total cap overhang of both ends of the divider line.
    fn total_cap_overhang(&self, thickness: f64) -> f64 {
        let mut overhang = 0.;
        for cap in [self.start_cap, self.end_cap] {
            match cap {
                Cap::Butt => (),
                Cap::Square | Cap::Round => {
                    overhang += thickness * 0.5;
                }
            }
        }
        overhang
    }
}

impl HasProperty<ContentColor> for Divider {}

// --- MARK: IMPL WIDGET
impl Widget for Divider {
    type Action = NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        if let Some(content) = &mut self.content {
            ctx.register_child(content);
        }
    }

    fn property_changed(&mut self, ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if ContentColor::matches(property_type) {
            ctx.request_paint_only();
        }
    }

    fn update(
        &mut self,
        _ctx: &mut UpdateCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &Update,
    ) {
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<f64>,
    ) -> f64 {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        const DEFAULT_LENGTH: f64 = 100.;
        let thickness = self.thickness.map(|t| t.dp(scale)).unwrap_or(1.);

        let content_length = if let Some(content) = &mut self.content {
            let auto_length = len_req.into();
            let context_size = LayoutSize::maybe(axis.cross(), cross_length);
            ctx.compute_length(content, auto_length, context_size, axis, cross_length)
        } else {
            0.
        };

        if axis == self.axis {
            match len_req {
                LenReq::MinContent => content_length,
                LenReq::MaxContent => (DEFAULT_LENGTH * scale).max(content_length),
                LenReq::FitContent(space) => space.max(content_length),
            }
        } else {
            thickness.max(content_length)
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        // Clear any previously laid out lines.
        self.lines.clear();

        /// Stores the line layout info for painting.
        fn lay_out_line(
            this: &mut Divider,
            start: f64,
            end: f64,
            cross_pos: f64,
            dashes: SmallVec<[f64; 4]>,
            thickness: f64,
        ) {
            let cap_offset = match this.start_cap {
                Cap::Butt => 0.,
                Cap::Square | Cap::Round => thickness * 0.5,
            };
            let p1 = this.axis.pack_point(cap_offset + start, cross_pos);
            let p2 = this.axis.pack_point(cap_offset + end, cross_pos);
            this.lines.push(LineLayout {
                line: Line::new(p1, p2),
                dashes,
            });
        }

        let thickness = self.thickness.map(|t| t.dp(scale)).unwrap_or(1.);
        let cross_pos = size.get_coord(self.axis.cross()) * 0.5;
        let mut dashes: SmallVec<[f64; 4]> =
            self.dash_pattern.iter().map(|l| l.dp(scale)).collect();

        if let Some(content) = &mut self.content {
            let content_size = ctx.compute_size(content, SizeDef::fit(size), size.into());
            ctx.run_layout(content, content_size);

            let placement = match self.placement {
                Placement::Start => match self.axis {
                    Axis::Horizontal => UnitPoint::LEFT,
                    Axis::Vertical => UnitPoint::TOP,
                },
                Placement::Center => UnitPoint::CENTER,
                Placement::End => match self.axis {
                    Axis::Horizontal => UnitPoint::RIGHT,
                    Axis::Vertical => UnitPoint::BOTTOM,
                },
            };
            let content_origin = placement.resolve((size - content_size).to_rect());
            ctx.place_child(content, content_origin);

            ctx.derive_baselines(content);

            let pad = self.pad.dp(scale);
            let mut line_space = size.get_coord(self.axis)
                - self.total_cap_overhang(thickness)
                - content_size.get_coord(self.axis)
                - pad;

            if line_space < 0. {
                // No space for line drawing
                return;
            }

            let (start, end) = match self.placement {
                // Content at the start, line has a start offset
                Placement::Start => {
                    let (start, end) = line_length(line_space, &mut dashes, self.dash_fit);
                    let offset = content_size.get_coord(self.axis) + pad;
                    (start + offset, end + offset)
                }
                // Content in the middle, two lines surrounding it
                Placement::Center => {
                    // Need to account for an extra pad, and divide the space in two
                    line_space = ((line_space - pad) * 0.5).max(0.);
                    let (start, end) = line_length(line_space, &mut dashes, self.dash_fit);
                    // Lay out the first line at the start
                    lay_out_line(self, start, end, cross_pos, dashes.clone(), thickness);
                    // Lay out the second line on the other side of the content,
                    // using the general code path.
                    let offset = content_origin.get_coord(self.axis)
                        + content_size.get_coord(self.axis)
                        + pad;
                    (start + offset, end + offset)
                }
                // Content at the end, line has no offset
                Placement::End => line_length(line_space, &mut dashes, self.dash_fit),
            };

            lay_out_line(self, start, end, cross_pos, dashes, thickness);
        } else {
            // Single line, no content
            ctx.clear_baselines();

            let line_space = size.get_coord(self.axis) - self.total_cap_overhang(thickness);
            if line_space < 0. {
                // No space for line drawing
                return;
            }
            let (start, end) = line_length(line_space, &mut dashes, self.dash_fit);
            lay_out_line(self, start, end, cross_pos, dashes, thickness);
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        // TODO: Remove HACK: After scale factor rework this can be a simple 1.
        let one_dp = 1. / ctx.get_scale_factor();

        let color = props.get::<ContentColor>();
        let thickness = self.thickness.map(|t| t.dp(scale)).unwrap_or(one_dp);

        for line in &self.lines {
            let style = Stroke {
                width: thickness,
                join: Join::Miter,
                dash_pattern: line.dashes.clone(),
                start_cap: self.start_cap,
                end_cap: self.end_cap,
                ..Default::default()
            };
            scene.stroke(&style, Affine::IDENTITY, color.color, None, &line.line);
        }
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
        let mut ids = ChildrenIds::new();
        if let Some(content) = &self.content {
            ids.push(content.id());
        }
        ids
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("Divider", id = id.trace())
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::AsUnit;
    use crate::palette;
    use crate::properties::types::{CrossAxisAlignment, MainAxisAlignment};
    use crate::properties::{Background, Dimensions, Gap, Padding};
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::test_property_set;
    use crate::widgets::{Flex, SizedBox, Spinner};

    fn pattern(values: &[usize]) -> Vec<Length> {
        values.iter().map(|&v| Length::px(v as f64)).collect()
    }

    #[test]
    fn simple() {
        let root = Flex::row()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(
                Flex::column()
                    .with_fixed(Label::new("Above").with_auto_id())
                    .with_fixed(Divider::horizontal().with_auto_id())
                    .with_fixed(Label::new("Below").with_auto_id())
                    .with_auto_id(),
            )
            .with_fixed(
                Divider::vertical()
                    .thickness(5.px())
                    .with_props(ContentColor::new(palette::css::DARK_SALMON)),
            )
            .with(
                Flex::column()
                    .with_fixed(Label::new("Another above").with_props(Dimensions::height(50.px())))
                    .with_fixed(Divider::horizontal().with_auto_id())
                    .with_auto_id(),
                1.,
            )
            .with_props(Gap::ZERO);

        let mut harness =
            TestHarness::create_with_size(test_property_set(), root, Size::new(350., 80.));

        assert_render_snapshot!(harness, "divider_simple");
    }

    #[test]
    fn styled() {
        let root = SizedBox::new(
            Flex::column()
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_fixed(
                    Divider::horizontal()
                        .thickness(5.px())
                        .dash_pattern(&pattern(&[1, 10, 5, 20]))
                        .cap(Cap::Round)
                        .with_props(ContentColor::new(palette::css::BLANCHED_ALMOND)),
                )
                .with_fixed(
                    Divider::horizontal()
                        .thickness(10.px())
                        .dash_pattern(&pattern(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]))
                        .with_props(ContentColor::new(palette::css::LAVENDER)),
                )
                .with_fixed(
                    Divider::horizontal()
                        .thickness(10.px())
                        .dash_pattern(&pattern(&[5, 20]))
                        .start_cap(Cap::Round)
                        .end_cap(Cap::Square)
                        .with_props(ContentColor::new(palette::css::ORANGE)),
                )
                .with_fixed(
                    Divider::horizontal()
                        .hairline()
                        .dash_pattern(&pattern(&[5, 1]))
                        .with_props(ContentColor::new(palette::css::CRIMSON)),
                )
                .with_fixed(
                    Divider::horizontal()
                        .thickness(3.px())
                        .dash_pattern(&pattern(&[1, 6, 1, 12]))
                        .cap(Cap::Round)
                        .with_props(ContentColor::new(palette::css::GOLD)),
                )
                .with_fixed(
                    Divider::horizontal()
                        .thickness(2.px())
                        .dash_pattern(&pattern(&[5, 5]))
                        .dash_fit(DashFit::Stretch)
                        .pad(20.px())
                        .label("O")
                        .with_props(ContentColor::new(palette::css::HOT_PINK)),
                )
                .with_auto_id(),
        )
        .with_props(Padding::all(10.));

        let mut harness =
            TestHarness::create_with_size(test_property_set(), root, Size::new(150., 120.));

        assert_render_snapshot!(harness, "divider_styled");
    }

    #[test]
    fn dash_fit() {
        let divider_fit = |fit: DashFit| {
            Divider::horizontal()
                .thickness(5.px())
                .dash_pattern(&pattern(&[20, 10, 10, 20]))
                .dash_fit(fit)
                .with_props(ContentColor::new(palette::css::MAGENTA))
        };

        let root = SizedBox::new(
            Flex::column()
                .main_axis_alignment(MainAxisAlignment::Center)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_fixed(divider_fit(DashFit::Clip))
                .with_fixed(divider_fit(DashFit::Stretch))
                .with_fixed(divider_fit(DashFit::Start))
                .with_fixed(divider_fit(DashFit::Center))
                .with_fixed(divider_fit(DashFit::End))
                .with_props(Background::Color(palette::css::MIDNIGHT_BLUE)),
        )
        .with_props(Padding::all(10.));

        let mut harness =
            TestHarness::create_with_size(test_property_set(), root, Size::new(155., 90.));

        assert_render_snapshot!(harness, "divider_dash_fit");
    }

    #[test]
    fn label() {
        let root = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(
                Divider::horizontal()
                    .label("Start")
                    .placement(Placement::Start)
                    .with_auto_id(),
            )
            .with_fixed(
                Divider::horizontal()
                    .label("Center")
                    .placement(Placement::Center)
                    .with_auto_id(),
            )
            .with_fixed(
                Divider::horizontal()
                    .label("End")
                    .placement(Placement::End)
                    .with_auto_id(),
            )
            .with(
                Flex::row()
                    .main_axis_alignment(MainAxisAlignment::Center)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .with_fixed(
                        Divider::vertical()
                            .label("Start")
                            .placement(Placement::Start)
                            .with_auto_id(),
                    )
                    .with_fixed(
                        Divider::vertical()
                            .label("Center")
                            .placement(Placement::Center)
                            .with_auto_id(),
                    )
                    .with_fixed(
                        Divider::vertical()
                            .label("End")
                            .placement(Placement::End)
                            .with_auto_id(),
                    )
                    .with_auto_id(),
                1.,
            )
            .with_auto_id();

        let mut harness =
            TestHarness::create_with_size(test_property_set(), root, Size::new(200., 200.));

        assert_render_snapshot!(harness, "divider_label");
    }

    #[test]
    fn content() {
        let content = Spinner::new().with_props(Dimensions::fixed(30.px(), 30.px()));
        let root = Divider::horizontal()
            .content(content)
            .with_props(Dimensions::STRETCH);

        let mut harness =
            TestHarness::create_with_size(test_property_set(), root, Size::new(100., 60.));

        assert_render_snapshot!(harness, "divider_content");
    }
}
