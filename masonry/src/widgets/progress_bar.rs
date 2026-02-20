// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use accesskit::{Node, Role};
use include_doc_path::include_doc_path;
use tracing::{Span, trace_span};
use vello::Scene;

use crate::core::{
    AccessCtx, ArcStr, ChildrenIds, LayoutCtx, MeasureCtx, NewWidget, NoAction, PaintCtx,
    PrePaintProps, PropertiesMut, PropertiesRef, Property, PropertySet, RegisterCtx, Update,
    UpdateCtx, Widget, WidgetId, WidgetMut, WidgetPod, paint_background, paint_border,
    paint_box_shadow,
};
use crate::kurbo::{Axis, Size};
use crate::layout::{LayoutSize, LenReq, SizeDef};
use crate::peniko::{Color, Gradient};
use crate::properties::{BarColor, BorderColor, BorderWidth, CornerRadius, LineBreaking};
use crate::theme;
use crate::util::fill;
use crate::widgets::Label;

// TODO - NaN probably shouldn't be a meaningful value in our API.

/// A progress bar.
///
#[doc = concat!(
    "![25% progress bar](",
    include_doc_path!("screenshots/progress_bar_25_percent.png"),
    ")",
)]
pub struct ProgressBar {
    /// A value in the range `[0, 1]` inclusive, where 0 is 0% and 1 is 100% complete.
    ///
    /// `None` variant can be used to show a progress bar without a percentage.
    /// It is also used if an invalid float (outside of [0, 1]) is passed.
    progress: Option<f64>,
    label: WidgetPod<Label>,
}

// --- MARK: BUILDERS
impl ProgressBar {
    /// Creates a new `ProgressBar`.
    ///
    /// The progress value will be clamped to [0, 1].
    ///
    /// A `None` value (or NaN) will show an indeterminate progress bar.
    pub fn new(progress: Option<f64>) -> Self {
        let progress = clamp_progress(progress);
        let label_props = PropertySet::one(LineBreaking::Overflow);
        let label =
            NewWidget::new_with_props(Label::new(Self::value(progress)), label_props).to_pod();
        Self { progress, label }
    }
}

// --- MARK: METHODS
impl ProgressBar {
    fn value_accessibility(&self) -> Box<str> {
        if let Some(value) = self.progress {
            format!("{:.0}%", value * 100.).into()
        } else {
            "progress unspecified".into()
        }
    }

    fn value(progress: Option<f64>) -> ArcStr {
        if let Some(value) = progress {
            format!("{:.0}%", value * 100.).into()
        } else {
            "".into()
        }
    }
}

// --- MARK: WIDGETMUT
impl ProgressBar {
    /// Sets the progress displayed by the bar.
    ///
    /// The progress value will be clamped to [0, 1].
    ///
    /// A `None` value (or NaN) will show an indeterminate progress bar.
    pub fn set_progress(this: &mut WidgetMut<'_, Self>, progress: Option<f64>) {
        let progress = clamp_progress(progress);
        let progress_changed = this.widget.progress != progress;
        if progress_changed {
            this.widget.progress = progress;
            let mut label = this.ctx.get_mut(&mut this.widget.label);
            Label::set_text(&mut label, Self::value(progress));
        }
        this.ctx.request_layout();
        this.ctx.request_render();
    }
}

/// Helper to ensure progress is either a number between [0, 1] inclusive, or `None`.
///
/// NaNs are converted to `None`.
fn clamp_progress(progress: Option<f64>) -> Option<f64> {
    let progress = progress?;
    if progress.is_nan() {
        None
    } else {
        Some(progress.clamp(0., 1.))
    }
}

// --- MARK: IMPL WIDGET
impl Widget for ProgressBar {
    type Action = NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.label);
    }

    fn property_changed(&mut self, ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if BarColor::matches(property_type)
            || BorderWidth::matches(property_type)
            || BorderColor::matches(property_type)
            || CornerRadius::matches(property_type)
        {
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
        // TODO: Move this to theme?
        const DEFAULT_WIDTH: f64 = 400.; // In logical pixels

        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        let auto_length = len_req.into();
        let context_size = LayoutSize::maybe(axis.cross(), cross_length);

        let label_length = ctx.compute_length(
            &mut self.label,
            auto_length,
            context_size,
            axis,
            cross_length,
        );

        let potential_length = match axis {
            Axis::Horizontal => match len_req {
                LenReq::MinContent | LenReq::MaxContent => DEFAULT_WIDTH * scale,
                LenReq::FitContent(space) => space,
            },
            Axis::Vertical => theme::BASIC_WIDGET_HEIGHT.dp(scale),
        };

        // Make sure we always report a length big enough to fit our painting
        potential_length.max(label_length)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let label_size = ctx.compute_size(&mut self.label, SizeDef::fit(size), size.into());
        ctx.run_layout(&mut self.label, label_size);

        let child_origin = ((size - label_size).to_vec2() * 0.5).to_point();
        ctx.place_child(&mut self.label, child_origin);
    }

    fn pre_paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        let bbox = ctx.border_box();
        let p = PrePaintProps::fetch(ctx, props);

        paint_box_shadow(scene, bbox, p.box_shadow, p.corner_radius);
        paint_background(scene, bbox, p.background, p.border_width, p.corner_radius);
        // We need to delay painting the border until after we paint the filled bar area.
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        let border_box = ctx.border_box();
        let border_width = props.get::<BorderWidth>();
        let corner_radius = props.get::<CornerRadius>();
        let border_color = props.get::<BorderColor>();

        let progress = self.progress.unwrap_or(1.);
        if progress > 0. {
            // The bar width is without the borders.
            let bar_width = border_box.width() - 2. * border_width.width;
            if bar_width > 0. {
                let bar_color = props.get::<BarColor>().0;
                // Paint with a gradient so we get a straight line slice of the rounded rect.
                let gradient = Gradient::new_linear((0., 0.), (bar_width, 0.)).with_stops([
                    (0., bar_color),
                    (progress as f32, bar_color),
                    (progress as f32, Color::TRANSPARENT),
                    (1., Color::TRANSPARENT),
                ]);

                // Currently bg_rect() gives a rect without borders, so we can use it.
                // However in the future when bg_rect() gets expanded to include borders,
                // we'll need to create a special sans-border rect for this fill.
                let bg_rect = border_width.bg_rect(border_box, corner_radius);

                fill(scene, &bg_rect, &gradient);
            }
        }

        paint_border(scene, border_box, border_color, border_width, corner_radius);
    }

    fn accessibility_role(&self) -> Role {
        Role::ProgressIndicator
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        node.set_min_numeric_value(0.0);
        node.set_max_numeric_value(1.0);
        if let Some(value) = self.progress {
            node.set_numeric_value(value);
        }
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.label.id()])
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("ProgressBar", id = id.trace())
    }

    fn get_debug_text(&self) -> Option<String> {
        Some(self.value_accessibility().into())
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{NewWidget, PropertySet};
    use crate::palette;
    use crate::properties::{BorderColor, CornerRadius};
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::test_property_set;

    #[test]
    fn indeterminate_progressbar() {
        let widget = NewWidget::new(ProgressBar::new(None));

        let window_size = Size::new(150.0, 60.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "progress_bar_indeterminate");
    }

    #[test]
    fn _5_percent_styled_progressbar() {
        let widget = ProgressBar::new(Some(0.05)).with_props((
            CornerRadius::all(50.),
            BorderWidth::all(10.),
            BorderColor::new(palette::css::PINK),
        ));
        let window_size = Size::new(150.0, 60.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "progress_bar_5_percent_styled");
    }

    #[test]
    fn _95_percent_styled_progressbar() {
        let widget = ProgressBar::new(Some(0.95)).with_props((
            CornerRadius::all(50.),
            BorderWidth::all(10.),
            BorderColor::new(palette::css::PINK),
        ));
        let window_size = Size::new(150.0, 60.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "progress_bar_95_percent_styled");
    }

    #[test]
    fn _0_percent_progressbar() {
        let widget = NewWidget::new(ProgressBar::new(Some(0.)));
        let window_size = Size::new(150.0, 60.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "progress_bar_0_percent");
    }

    #[test]
    fn _25_percent_progressbar() {
        let widget = NewWidget::new(ProgressBar::new(Some(0.25)));
        let window_size = Size::new(150.0, 60.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "progress_bar_25_percent");
    }

    #[test]
    fn _50_percent_progressbar() {
        let widget = NewWidget::new(ProgressBar::new(Some(0.5)));
        let window_size = Size::new(150.0, 60.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "progress_bar_50_percent");
    }

    #[test]
    fn _75_percent_progressbar() {
        let widget = NewWidget::new(ProgressBar::new(Some(0.75)));
        let window_size = Size::new(150.0, 60.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "progress_bar_75_percent");
    }

    #[test]
    fn _100_percent_progressbar() {
        let widget = NewWidget::new(ProgressBar::new(Some(1.)));
        let window_size = Size::new(150.0, 60.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), widget, window_size);

        assert_render_snapshot!(harness, "progress_bar_100_percent");
    }

    #[test]
    fn edit_progressbar() {
        let image_1 = {
            let bar = ProgressBar::new(Some(0.5))
                .with_props(PropertySet::new().with(BarColor(palette::css::PURPLE)));

            let mut harness =
                TestHarness::create_with_size(test_property_set(), bar, Size::new(60.0, 20.0));

            harness.render()
        };

        let image_2 = {
            let bar = NewWidget::new(ProgressBar::new(None));

            let mut harness =
                TestHarness::create_with_size(test_property_set(), bar, Size::new(60.0, 20.0));

            harness.edit_root_widget(|mut bar| {
                ProgressBar::set_progress(&mut bar, Some(0.5));
                bar.insert_prop(BarColor(palette::css::PURPLE));
            });

            harness.render()
        };

        // We don't use assert_eq because we don't want rich assert
        assert!(image_1 == image_2);
    }
}
