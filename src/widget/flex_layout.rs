// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

//! A widget that arranges its children in a one-dimensional array.

use crate::{
    widget::{AccessCx, BoxConstraints, Event, *},
    Axis,
};

use glazier::kurbo::{common::FloatExt, Rect, Size};
use tracing::{instrument, trace, warn};

/// Panic in debug and tracing::error in release mode.
///
/// This macro is in some way a combination of `panic` and `debug_assert`,
/// but it will log the provided message instead of ignoring it in release builds.
///
/// It's useful when a backtrace would aid debugging but a crash can be avoided in release.
macro_rules! debug_panic {
    () => { ... };
    ($msg:expr) => {
        if cfg!(debug_assertions) {
            panic!($msg);
        } else {
            tracing::error!($msg);
        }
    };
    ($msg:expr,) => { debug_panic!($msg) };
    ($fmt:expr, $($arg:tt)+) => {
        if cfg!(debug_assertions) {
            panic!($fmt, $($arg)*);
        } else {
            tracing::error!($fmt, $($arg)*);
        }
    };
}

pub struct FlexLayout {
    pub(crate) children: Vec<Child>,
    pub(crate) axis: Axis,
    pub(crate) cross_alignment: CrossAxisAlignment,
    pub(crate) main_alignment: MainAxisAlignment,
    pub(crate) fill_major_axis: bool,
    old_bc: BoxConstraints,
}

impl Axis {
    /// Generate constraints with new values on the major axis.
    pub(crate) fn constraints(
        self,
        bc: &BoxConstraints,
        min_major: f64,
        major: f64,
    ) -> BoxConstraints {
        match self {
            Axis::Horizontal => BoxConstraints::new(
                Size::new(min_major, bc.min().height),
                Size::new(major, bc.max().height),
            ),
            Axis::Vertical => BoxConstraints::new(
                Size::new(bc.min().width, min_major),
                Size::new(bc.max().width, major),
            ),
        }
    }
}

/// The alignment of the widgets on the container's cross (or minor) axis.
///
/// If a widget is smaller than the container on the minor axis, this determines
/// where it is positioned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossAxisAlignment {
    /// Top or leading.
    ///
    /// In a vertical container, widgets are top aligned. In a horizontal
    /// container, their leading edges are aligned.
    Start,
    /// Widgets are centered in the container.
    Center,
    /// Bottom or trailing.
    ///
    /// In a vertical container, widgets are bottom aligned. In a horizontal
    /// container, their trailing edges are aligned.
    End,
    /// Align on the baseline.
    ///
    /// In a horizontal container, widgets are aligned along the calculated
    /// baseline. In a vertical container, this is equivalent to `End`.
    ///
    /// The calculated baseline is the maximum baseline offset of the children.
    Baseline,
    /// Fill the available space.
    ///
    /// The size on this axis is the size of the largest widget;
    /// other widgets must fill that space.
    Fill,
}

/// Arrangement of children on the main axis.
///
/// If there is surplus space on the main axis after laying out children, this
/// enum represents how children are laid out in this space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainAxisAlignment {
    /// Top or leading.
    ///
    /// Children are aligned with the top or leading edge, without padding.
    Start,
    /// Children are centered, without padding.
    Center,
    /// Bottom or trailing.
    ///
    /// Children are aligned with the bottom or trailing edge, without padding.
    End,
    /// Extra space is divided evenly between each child.
    SpaceBetween,
    /// Extra space is divided evenly between each child, as well as at the ends.
    SpaceEvenly,
    /// Space between each child, with less at the start and end.
    ///
    /// This divides space such that each child is separated by `n` units,
    /// and the start and end have `n/2` units of padding.
    SpaceAround,
}

impl FlexLayout {
    pub(crate) fn new(
        elements: Vec<Child>,
        axis: Axis,
        cross_alignment: CrossAxisAlignment,
        main_alignment: MainAxisAlignment,
        fill_major_axis: bool,
    ) -> FlexLayout {
        FlexLayout {
            children: elements,
            axis,
            cross_alignment,
            main_alignment,
            fill_major_axis,
            old_bc: BoxConstraints::tight(Size::ZERO),
        }
    }
}

impl Widget for FlexLayout {
    fn event(&mut self, cx: &mut EventCx, event: &Event) {
        for child in &mut self.children {
            child.widget.event(cx, event);
        }
    }

    fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle) {
        for child in &mut self.children {
            child.widget.lifecycle(cx, event);
        }
    }

    fn update(&mut self, cx: &mut UpdateCx) {
        for child in &mut self.children {
            child.widget.update(cx);
        }
    }

    fn layout(&mut self, cx: &mut LayoutCx, bc: &BoxConstraints) -> Size {
        bc.debug_check("Flex");
        // we loosen our constraints when passing to children.
        let loosened_bc = bc.loosen();

        // minor-axis values for all children
        let mut minor = self.axis.minor(bc.min());
        // these two are calculated but only used if we're baseline aligned
        let mut max_above_baseline = 0f64;
        let mut max_below_baseline = 0f64;
        let mut any_use_baseline = false;

        // indicates that the box constrains for the following children have changed. Therefore they
        // have to calculate layout again.
        let bc_changed = self.old_bc != *bc;
        let mut any_changed = bc_changed;
        self.old_bc = *bc;

        // Measure non-flex children.
        let mut major_non_flex = 0.0;
        let mut flex_sum = 0.0;
        for child in &mut self.children {
            match child.flex {
                None => {
                    // The BoxConstrains of fixed-children only depends on the BoxConstrains of the
                    // Flex widget.
                    let child_size = if bc_changed || child.widget.layout_requested() {
                        let alignment = child.alignment.unwrap_or(self.cross_alignment);
                        any_use_baseline |= alignment == CrossAxisAlignment::Baseline;

                        let old_size = child.widget.state.size;
                        let child_bc = self.axis.constraints(&loosened_bc, 0.0, std::f64::INFINITY);
                        let child_size = child.widget.layout(cx, &child_bc);

                        if child_size.width.is_infinite() {
                            warn!("A non-Flex child has an infinite width.");
                        }

                        if child_size.height.is_infinite() {
                            warn!("A non-Flex child has an infinite height.");
                        }

                        if old_size != child_size {
                            any_changed = true;
                        }

                        child_size
                    } else {
                        child.widget.state.size
                    };

                    // let baseline_offset = child.widget.baseline_offset();
                    let baseline_offset = 0.0;

                    major_non_flex += self.axis.major(child_size).expand();
                    minor = minor.max(self.axis.minor(child_size).expand());
                    max_above_baseline =
                        max_above_baseline.max(child_size.height - baseline_offset);
                    max_below_baseline = max_below_baseline.max(baseline_offset);
                }
                Some(flex) => flex_sum += flex,
            }
        }

        let total_major = self.axis.major(bc.max());
        let remaining = (total_major - major_non_flex).max(0.0);
        let mut remainder: f64 = 0.0;

        let mut major_flex: f64 = 0.0;
        let px_per_flex = remaining / flex_sum;
        // Measure flex children.
        for child in &mut self.children {
            if let Some(flex) = child.flex {
                // The BoxConstrains of flex-children depends on the size of every sibling, which
                // received layout earlier. Therefore we use any_changed.
                let child_size = if any_changed || child.widget.layout_requested() {
                    let alignment = child.alignment.unwrap_or(self.cross_alignment);
                    any_use_baseline |= alignment == CrossAxisAlignment::Baseline;

                    let desired_major = flex * px_per_flex + remainder;
                    let actual_major = desired_major.round();
                    remainder = desired_major - actual_major;

                    let old_size = child.widget.state.size;
                    let child_bc = self.axis.constraints(&loosened_bc, 0.0, actual_major);
                    let child_size = child.widget.layout(cx, &child_bc);

                    if old_size != child_size {
                        any_changed = true;
                    }

                    child_size
                } else {
                    child.widget.state.size
                };

                // let baseline_offset = child.widget.baseline_offset();
                let baseline_offset = 0.0;

                major_flex += self.axis.major(child_size).expand();
                minor = minor.max(self.axis.minor(child_size).expand());
                max_above_baseline = max_above_baseline.max(child_size.height - baseline_offset);
                max_below_baseline = max_below_baseline.max(baseline_offset);
            }
        }

        // figure out if we have extra space on major axis, and if so how to use it
        let extra = if self.fill_major_axis {
            (remaining - major_flex).max(0.0)
        } else {
            // if we are *not* expected to fill our available space this usually
            // means we don't have any extra, unless dictated by our constraints.
            (self.axis.major(bc.min()) - (major_non_flex + major_flex)).max(0.0)
        };

        let mut spacing = Spacing::new(self.main_alignment, extra, self.children.len());

        // the actual size needed to tightly fit the children on the minor axis.
        // Unlike the 'minor' var, this ignores the incoming constraints.
        let minor_dim = match self.axis {
            Axis::Horizontal if any_use_baseline => max_below_baseline + max_above_baseline,
            _ => minor,
        };

        let extra_height = minor - minor_dim.min(minor);

        let mut major = spacing.next().unwrap_or(0.);
        let mut child_paint_rect = Rect::ZERO;

        for child in &mut self.children {
            let child_size = child.widget.state.size;
            let alignment = child.alignment.unwrap_or(self.cross_alignment);
            let child_minor_offset = match alignment {
                // This will ignore baseline alignment if it is overridden on children,
                // but is not the default for the container. Is this okay?
                CrossAxisAlignment::Baseline if self.axis != Axis::Horizontal => {
                    // let child_baseline = child.widget.baseline_offset();
                    let child_baseline = 0.0;
                    let child_above_baseline = child_size.height - child_baseline;
                    extra_height + (max_above_baseline - child_above_baseline)
                }
                CrossAxisAlignment::Fill => {
                    let fill_size: Size = self.axis.pack(self.axis.major(child_size), minor_dim);
                    if child.widget.state.size != fill_size {
                        let child_bc = BoxConstraints::tight(fill_size);
                        //TODO: this is the second call of layout on the same child, which
                        // is bad, because it can lead to exponential increase in layout calls
                        // when used multiple times in the widget hierarchy.
                        child.widget.layout(cx, &child_bc);
                    }
                    0.0
                }
                _ => {
                    let extra_minor = minor_dim - self.axis.minor(child_size);
                    alignment.align(extra_minor)
                }
            };

            child
                .widget
                .set_origin(cx, self.axis.pack(major, child_minor_offset));
            // child_paint_rect = child_paint_rect.union(child.widget.state.paint_rect());
            child_paint_rect = child_paint_rect.union(
                child
                    .widget
                    .state
                    .size
                    .to_rect()
                    .with_origin(child.widget.state.origin),
            );
            major += self.axis.major(child_size).expand();
            major += spacing.next().unwrap_or(0.);
        }

        if flex_sum > 0.0 && total_major.is_infinite() {
            tracing::warn!("A child of Flex is flex, but Flex is unbounded.")
        }

        if flex_sum > 0.0 {
            major = total_major;
        }

        let my_size: Size = self.axis.pack(major, minor_dim);

        // if we don't have to fill the main axis, we loosen that axis before constraining
        let my_size = if !self.fill_major_axis {
            let max_major = self.axis.major(bc.max());
            self.axis.constraints(bc, 0.0, max_major).constrain(my_size)
        } else {
            bc.constrain(my_size)
        };

        let my_bounds = Rect::ZERO.with_size(my_size);
        let _insets = child_paint_rect - my_bounds;
        // cx.set_paint_insets(_insets);

        let baseline_offset = match self.axis {
            Axis::Horizontal => max_below_baseline,
            Axis::Vertical => self
                .children
                .last()
                .map(|last| {
                    let widget = &last.widget;
                    // let child_bl = widget.state.baseline_offset();
                    let child_bl = 0.0;
                    let child_max_y = widget
                        .state
                        .size
                        .to_rect()
                        .with_origin(widget.state.origin)
                        .max_y();
                    let extra_bottom_padding = my_size.height - child_max_y;
                    child_bl + extra_bottom_padding
                })
                .unwrap_or(0.0),
        };

        // cx.set_baseline_offset(baseline_offset);
        trace!(
            "Computed layout: size={}, baseline_offset={}",
            my_size,
            baseline_offset
        );
        my_size
    }

    fn accessibility(&mut self, cx: &mut AccessCx) {
        for child in &mut self.children {
            child.widget.accessibility(cx);
        }

        if cx.is_requested() {
            let mut builder = accesskit::NodeBuilder::new(accesskit::Role::GenericContainer);
            builder.set_children(
                self.children
                    .iter()
                    .map(|pod| pod.widget.id().into())
                    .collect::<Vec<accesskit::NodeId>>(),
            );
            cx.push_node(builder);
        }
    }

    fn paint(&mut self, cx: &mut PaintCx, builder: &mut vello::SceneBuilder) {
        for child in &mut self.children {
            trace!(flex = child.flex, "paint child");
            child.widget.paint(cx, builder);
        }
    }
}

impl CrossAxisAlignment {
    /// Given the difference between the size of the container and the size
    /// of the child (on their minor axis) return the necessary offset for
    /// this alignment.
    fn align(self, val: f64) -> f64 {
        match self {
            CrossAxisAlignment::Start => 0.0,
            // in vertical layout, baseline is equivalent to center
            CrossAxisAlignment::Center | CrossAxisAlignment::Baseline => (val / 2.0).round(),
            CrossAxisAlignment::End => val,
            CrossAxisAlignment::Fill => 0.0,
        }
    }
}

struct Spacing {
    alignment: MainAxisAlignment,
    extra: f64,
    n_children: usize,
    index: usize,
    equal_space: f64,
    remainder: f64,
}

impl Spacing {
    /// Given the provided extra space and children count,
    /// this returns an iterator of `f64` spacing,
    /// where the first element is the spacing before any children
    /// and all subsequent elements are the spacing after children.
    fn new(alignment: MainAxisAlignment, extra: f64, n_children: usize) -> Spacing {
        let extra = if extra.is_finite() { extra } else { 0. };
        let equal_space = if n_children > 0 {
            match alignment {
                MainAxisAlignment::Center => extra / 2.,
                MainAxisAlignment::SpaceBetween => extra / (n_children - 1).max(1) as f64,
                MainAxisAlignment::SpaceEvenly => extra / (n_children + 1) as f64,
                MainAxisAlignment::SpaceAround => extra / (2 * n_children) as f64,
                _ => 0.,
            }
        } else {
            0.
        };
        Spacing {
            alignment,
            extra,
            n_children,
            index: 0,
            equal_space,
            remainder: 0.,
        }
    }

    fn next_space(&mut self) -> f64 {
        let desired_space = self.equal_space + self.remainder;
        let actual_space = desired_space.round();
        self.remainder = desired_space - actual_space;
        actual_space
    }
}

impl Iterator for Spacing {
    type Item = f64;

    fn next(&mut self) -> Option<f64> {
        if self.index > self.n_children {
            return None;
        }
        let result = {
            if self.n_children == 0 {
                self.extra
            } else {
                #[allow(clippy::match_bool)]
                match self.alignment {
                    MainAxisAlignment::Start => match self.index == self.n_children {
                        true => self.extra,
                        false => 0.,
                    },
                    MainAxisAlignment::End => match self.index == 0 {
                        true => self.extra,
                        false => 0.,
                    },
                    MainAxisAlignment::Center => match self.index {
                        0 => self.next_space(),
                        i if i == self.n_children => self.next_space(),
                        _ => 0.,
                    },
                    MainAxisAlignment::SpaceBetween => match self.index {
                        0 => 0.,
                        i if i != self.n_children => self.next_space(),
                        _ => match self.n_children {
                            1 => self.next_space(),
                            _ => 0.,
                        },
                    },
                    MainAxisAlignment::SpaceEvenly => self.next_space(),
                    MainAxisAlignment::SpaceAround => {
                        if self.index == 0 || self.index == self.n_children {
                            self.next_space()
                        } else {
                            self.next_space() + self.next_space()
                        }
                    }
                }
            }
        };
        self.index += 1;
        Some(result)
    }
}

pub struct Child {
    pub(crate) widget: Pod,
    pub(crate) alignment: Option<CrossAxisAlignment>,
    pub(crate) flex: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn test_main_axis_alignment_spacing() {
        // The following alignment strategy is based on how
        // Chrome 80 handles it with CSS flex.

        let vec = |a, e, n| -> Vec<f64> { Spacing::new(a, e, n).collect() };

        let a = MainAxisAlignment::Start;
        assert_eq!(vec(a, 10., 0), vec![10.]);
        assert_eq!(vec(a, 10., 1), vec![0., 10.]);
        assert_eq!(vec(a, 10., 2), vec![0., 0., 10.]);
        assert_eq!(vec(a, 10., 3), vec![0., 0., 0., 10.]);

        let a = MainAxisAlignment::End;
        assert_eq!(vec(a, 10., 0), vec![10.]);
        assert_eq!(vec(a, 10., 1), vec![10., 0.]);
        assert_eq!(vec(a, 10., 2), vec![10., 0., 0.]);
        assert_eq!(vec(a, 10., 3), vec![10., 0., 0., 0.]);

        let a = MainAxisAlignment::Center;
        assert_eq!(vec(a, 10., 0), vec![10.]);
        assert_eq!(vec(a, 10., 1), vec![5., 5.]);
        assert_eq!(vec(a, 10., 2), vec![5., 0., 5.]);
        assert_eq!(vec(a, 10., 3), vec![5., 0., 0., 5.]);
        assert_eq!(vec(a, 1., 0), vec![1.]);
        assert_eq!(vec(a, 3., 1), vec![2., 1.]);
        assert_eq!(vec(a, 5., 2), vec![3., 0., 2.]);
        assert_eq!(vec(a, 17., 3), vec![9., 0., 0., 8.]);

        let a = MainAxisAlignment::SpaceBetween;
        assert_eq!(vec(a, 10., 0), vec![10.]);
        assert_eq!(vec(a, 10., 1), vec![0., 10.]);
        assert_eq!(vec(a, 10., 2), vec![0., 10., 0.]);
        assert_eq!(vec(a, 10., 3), vec![0., 5., 5., 0.]);
        assert_eq!(vec(a, 33., 5), vec![0., 8., 9., 8., 8., 0.]);
        assert_eq!(vec(a, 34., 5), vec![0., 9., 8., 9., 8., 0.]);
        assert_eq!(vec(a, 35., 5), vec![0., 9., 9., 8., 9., 0.]);
        assert_eq!(vec(a, 36., 5), vec![0., 9., 9., 9., 9., 0.]);
        assert_eq!(vec(a, 37., 5), vec![0., 9., 10., 9., 9., 0.]);
        assert_eq!(vec(a, 38., 5), vec![0., 10., 9., 10., 9., 0.]);
        assert_eq!(vec(a, 39., 5), vec![0., 10., 10., 9., 10., 0.]);

        let a = MainAxisAlignment::SpaceEvenly;
        assert_eq!(vec(a, 10., 0), vec![10.]);
        assert_eq!(vec(a, 10., 1), vec![5., 5.]);
        assert_eq!(vec(a, 10., 2), vec![3., 4., 3.]);
        assert_eq!(vec(a, 10., 3), vec![3., 2., 3., 2.]);
        assert_eq!(vec(a, 33., 5), vec![6., 5., 6., 5., 6., 5.]);
        assert_eq!(vec(a, 34., 5), vec![6., 5., 6., 6., 5., 6.]);
        assert_eq!(vec(a, 35., 5), vec![6., 6., 5., 6., 6., 6.]);
        assert_eq!(vec(a, 36., 5), vec![6., 6., 6., 6., 6., 6.]);
        assert_eq!(vec(a, 37., 5), vec![6., 6., 7., 6., 6., 6.]);
        assert_eq!(vec(a, 38., 5), vec![6., 7., 6., 6., 7., 6.]);
        assert_eq!(vec(a, 39., 5), vec![7., 6., 7., 6., 7., 6.]);

        let a = MainAxisAlignment::SpaceAround;
        assert_eq!(vec(a, 10., 0), vec![10.]);
        assert_eq!(vec(a, 10., 1), vec![5., 5.]);
        assert_eq!(vec(a, 10., 2), vec![3., 5., 2.]);
        assert_eq!(vec(a, 10., 3), vec![2., 3., 3., 2.]);
        assert_eq!(vec(a, 33., 5), vec![3., 7., 6., 7., 7., 3.]);
        assert_eq!(vec(a, 34., 5), vec![3., 7., 7., 7., 7., 3.]);
        assert_eq!(vec(a, 35., 5), vec![4., 7., 7., 7., 7., 3.]);
        assert_eq!(vec(a, 36., 5), vec![4., 7., 7., 7., 7., 4.]);
        assert_eq!(vec(a, 37., 5), vec![4., 7., 8., 7., 7., 4.]);
        assert_eq!(vec(a, 38., 5), vec![4., 7., 8., 8., 7., 4.]);
        assert_eq!(vec(a, 39., 5), vec![4., 8., 7., 8., 8., 4.]);
    }
}
