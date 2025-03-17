// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A progress bar widget.

use accesskit::{Node, Role};
use smallvec::{SmallVec, smallvec};
use tracing::{Span, trace_span};
use vello::Scene;

use crate::core::{
    AccessCtx, AccessEvent, ArcStr, BoxConstraints, EventCtx, LayoutCtx, PaintCtx, PointerEvent,
    PropertiesMut, PropertiesRef, QueryCtx, RegisterCtx, TextEvent, Update, UpdateCtx, Widget,
    WidgetId, WidgetMut, WidgetPod,
};
use crate::kurbo::{Point, Size};
use crate::theme;
use crate::util::{UnitPoint, fill_lin_gradient, stroke};
use crate::widgets::{Label, LineBreaking};

/// A progress bar.
///
#[doc = crate::include_screenshot!("widget/screenshots/masonry__widget__progress_bar__tests__25_percent_progressbar.png", "25% progress bar.")]
pub struct ProgressBar {
    /// A value in the range `[0, 1]` inclusive, where 0 is 0% and 1 is 100% complete.
    ///
    /// `None` variant can be used to show a progress bar without a percentage.
    /// It is also used if an invalid float (outside of [0, 1]) is passed.
    progress: Option<f64>,
    label: WidgetPod<Label>,
}

impl ProgressBar {
    /// Create a new `ProgressBar`.
    ///
    /// `progress` is a number between 0 and 1 inclusive. If it is `NaN`, then an
    /// indefinite progress bar will be shown.
    /// Otherwise, the input will be clamped to [0, 1].
    pub fn new(mut progress: Option<f64>) -> Self {
        clamp_progress(&mut progress);
        let label = WidgetPod::new(
            Label::new(Self::value(progress)).with_line_break_mode(LineBreaking::Overflow),
        );
        Self { progress, label }
    }

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

// --- MARK: WIDGETMUT ---
impl ProgressBar {
    pub fn set_progress(this: &mut WidgetMut<'_, Self>, mut progress: Option<f64>) {
        clamp_progress(&mut progress);
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
fn clamp_progress(progress: &mut Option<f64>) {
    if let Some(value) = progress {
        if value.is_nan() {
            *progress = None;
        } else {
            *progress = Some(value.clamp(0., 1.));
        }
    }
}

// --- MARK: IMPL WIDGET ---
impl Widget for ProgressBar {
    fn on_pointer_event(
        &mut self,
        _ctx: &mut EventCtx,
        _props: &mut PropertiesMut<'_>,
        _event: &PointerEvent,
    ) {
    }

    fn on_text_event(
        &mut self,
        _ctx: &mut EventCtx,
        _props: &mut PropertiesMut<'_>,
        _event: &TextEvent,
    ) {
    }

    fn on_access_event(
        &mut self,
        _ctx: &mut EventCtx,
        _props: &mut PropertiesMut<'_>,
        _event: &AccessEvent,
    ) {
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx) {
        ctx.register_child(&mut self.label);
    }

    fn update(&mut self, _ctx: &mut UpdateCtx, _props: &mut PropertiesMut<'_>, _event: &Update) {}

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        const DEFAULT_WIDTH: f64 = 400.;
        // TODO: Clearer constraints here
        let label_size = ctx.run_layout(&mut self.label, &bc.loosen());
        let desired_size = Size::new(
            DEFAULT_WIDTH.max(label_size.width),
            crate::theme::BASIC_WIDGET_HEIGHT.max(label_size.height),
        );
        let final_size = bc.constrain(desired_size);

        // center text
        let text_pos = Point::new(
            ((final_size.width - label_size.width) * 0.5).max(0.),
            ((final_size.height - label_size.height) * 0.5).max(0.),
        );
        ctx.place_child(&mut self.label, text_pos);
        final_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        let border_width = 1.;
        let size = ctx.size();
        let border_radius = 2.;

        let bg_rect = size
            .to_rect()
            .inset(-border_width)
            .to_rounded_rect(border_radius - border_width);
        let border_rect = size
            .to_rect()
            .inset(-border_width / 2.0)
            .to_rounded_rect(border_radius);

        let progress_rect_size = Size::new(
            ctx.size().width * self.progress.unwrap_or(1.),
            ctx.size().height,
        );
        let progress_rect = progress_rect_size
            .to_rect()
            .inset(-border_width)
            .to_rounded_rect(border_radius - border_width);

        fill_lin_gradient(
            scene,
            &bg_rect,
            [theme::BACKGROUND_LIGHT, theme::BACKGROUND_DARK],
            UnitPoint::TOP,
            UnitPoint::BOTTOM,
        );
        fill_lin_gradient(
            scene,
            &progress_rect,
            [theme::PRIMARY_LIGHT, theme::PRIMARY_DARK],
            UnitPoint::TOP,
            UnitPoint::BOTTOM,
        );
        stroke(scene, &border_rect, theme::BORDER_DARK, border_width);
    }

    fn accessibility_role(&self) -> Role {
        Role::ProgressIndicator
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_value(self.value_accessibility());
        if let Some(value) = self.progress {
            node.set_numeric_value(value * 100.0);
        }
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        smallvec![self.label.id()]
    }

    fn make_trace_span(&self, ctx: &QueryCtx<'_>) -> Span {
        trace_span!("ProgressBar", id = ctx.widget_id().trace())
    }

    fn get_debug_text(&self) -> Option<String> {
        Some(self.value_accessibility().into())
    }
}

// --- MARK: TESTS ---
#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;

    use super::*;
    use crate::assert_render_snapshot;
    use crate::testing::{TestHarness, TestWidgetExt, widget_ids};

    #[test]
    fn indeterminate_progressbar() {
        let [progressbar_id] = widget_ids();
        let widget = ProgressBar::new(None).with_id(progressbar_id);

        let mut harness = TestHarness::create(widget);

        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "indeterminate_progressbar");
    }

    #[test]
    fn _0_percent_progressbar() {
        let [_0percent] = widget_ids();

        let widget = ProgressBar::new(Some(0.)).with_id(_0percent);
        let mut harness = TestHarness::create(widget);
        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "0_percent_progressbar");
    }

    #[test]
    fn _25_percent_progressbar() {
        let [_25percent] = widget_ids();

        let widget = ProgressBar::new(Some(0.25)).with_id(_25percent);
        let mut harness = TestHarness::create(widget);
        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "25_percent_progressbar");
    }

    #[test]
    fn _50_percent_progressbar() {
        let [_50percent] = widget_ids();

        let widget = ProgressBar::new(Some(0.5)).with_id(_50percent);
        let mut harness = TestHarness::create(widget);
        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "50_percent_progressbar");
    }

    #[test]
    fn _75_percent_progressbar() {
        let [_75percent] = widget_ids();

        let widget = ProgressBar::new(Some(0.75)).with_id(_75percent);
        let mut harness = TestHarness::create(widget);
        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "75_percent_progressbar");
    }

    #[test]
    fn _100_percent_progressbar() {
        let [_100percent] = widget_ids();

        let widget = ProgressBar::new(Some(1.)).with_id(_100percent);
        let mut harness = TestHarness::create(widget);
        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "100_percent_progressbar");
    }

    #[test]
    fn edit_progressbar() {
        let image_1 = {
            let bar = ProgressBar::new(Some(0.5));

            let mut harness = TestHarness::create_with_size(bar, Size::new(60.0, 20.0));

            harness.render()
        };

        let image_2 = {
            let bar = ProgressBar::new(None);

            let mut harness = TestHarness::create_with_size(bar, Size::new(60.0, 20.0));

            harness.edit_root_widget(|mut label| {
                let mut bar = label.downcast::<ProgressBar>();
                ProgressBar::set_progress(&mut bar, Some(0.5));
            });

            harness.render()
        };

        // We don't use assert_eq because we don't want rich assert
        assert!(image_1 == image_2);
    }
}
