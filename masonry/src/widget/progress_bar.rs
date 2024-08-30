// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A progress bar widget.

use accesskit::Role;
use kurbo::Point;
use smallvec::{smallvec, SmallVec};
use tracing::{trace, trace_span, Span};
use vello::Scene;

use crate::kurbo::Size;
use crate::paint_scene_helpers::{fill_lin_gradient, stroke, UnitPoint};
use crate::text::TextLayout;
use crate::widget::WidgetMut;
use crate::{
    theme, AccessCtx, AccessEvent, ArcStr, BoxConstraints, EventCtx, LayoutCtx, LifeCycle,
    LifeCycleCtx, PaintCtx, PointerEvent, StatusChange, TextEvent, Widget, WidgetId,
};

/// A progress bar
pub struct ProgressBar {
    /// A value in the range `[0, 1]` inclusive, where 0 is 0% and 1 is 100% complete.
    ///
    /// `None` variant can be used to show a progress bar without a percentage.
    /// It is also used if an invalid float (outside of [0, 1]) is passed.
    progress: Option<f32>,
    label: TextLayout<ArcStr>,
}

impl ProgressBar {
    /// Create a new `ProgressBar`.
    ///
    /// `progress` is a number between 0 and 1 inclusive. If it is `NaN`, then an
    /// indefinite progress bar will be shown.
    /// Otherwise, the input will be clamped to [0, 1].
    pub fn new(progress: Option<f32>) -> Self {
        let mut out = Self::new_indefinite();
        out.set_progress(progress);
        out
    }

    fn new_indefinite() -> Self {
        Self {
            progress: None,
            label: TextLayout::new("".into(), crate::theme::TEXT_SIZE_NORMAL as f32),
        }
    }

    fn set_progress(&mut self, mut progress: Option<f32>) {
        clamp_progress(&mut progress);
        // check to see if we can avoid doing work
        if self.progress != progress {
            self.progress = progress;
            self.update_text();
        }
    }

    /// Updates the text layout with the current part-complete value
    fn update_text(&mut self) {
        self.label.set_text(self.value().into());
    }

    fn value(&self) -> ArcStr {
        if let Some(value) = self.progress {
            format!("{:.0}%", value * 100.).into()
        } else {
            "".into()
        }
    }

    fn value_accessibility(&self) -> Box<str> {
        if let Some(value) = self.progress {
            format!("{:.0}%", value * 100.).into()
        } else {
            "progress unspecified".into()
        }
    }
}

// --- MARK: WIDGETMUT ---
impl WidgetMut<'_, ProgressBar> {
    pub fn set_progress(&mut self, progress: Option<f32>) {
        self.widget.set_progress(progress);
        self.ctx.request_layout();
        self.ctx.request_accessibility_update();
    }
}

/// Helper to ensure progress is either a number between [0, 1] inclusive, or `None`.
///
/// NaNs are converted to `None`.
fn clamp_progress(progress: &mut Option<f32>) {
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
    fn on_pointer_event(&mut self, _ctx: &mut EventCtx, event: &PointerEvent) {
        match event {
            // pointer events unhandled for now
            _ => (),
        }
    }

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}

    fn on_access_event(&mut self, ctx: &mut EventCtx, event: &AccessEvent) {
        if event.target == ctx.widget_id() {
            match event.action {
                // access events unhandled for now
                _ => {}
            }
        }
    }

    fn on_status_change(&mut self, ctx: &mut LifeCycleCtx, _event: &StatusChange) {
        ctx.request_paint();
    }

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle) {}

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        const DEFAULT_WIDTH: f64 = 400.;

        if self.label.needs_rebuild() {
            let (font_ctx, layout_ctx) = ctx.text_contexts();
            self.label.rebuild(font_ctx, layout_ctx);
        }
        let label_size = self.label.size();

        let desired_size = Size::new(
            DEFAULT_WIDTH.max(label_size.width),
            crate::theme::BASIC_WIDGET_HEIGHT.max(label_size.height),
        );
        let our_size = bc.constrain(desired_size);
        trace!("Computed layout: size={}", our_size);
        our_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        let border_width = 1.;

        if self.label.needs_rebuild() {
            debug_panic!("Called ProgressBar paint before layout");
        }

        let rect = ctx
            .size()
            .to_rect()
            .inset(-border_width / 2.)
            .to_rounded_rect(2.);

        fill_lin_gradient(
            scene,
            &rect,
            [theme::BACKGROUND_LIGHT, theme::BACKGROUND_DARK],
            UnitPoint::TOP,
            UnitPoint::BOTTOM,
        );

        stroke(scene, &rect, theme::BORDER_DARK, border_width);

        let progress_rect_size = Size::new(
            ctx.size().width * self.progress.unwrap_or(1.) as f64,
            ctx.size().height,
        );
        let progress_rect = progress_rect_size
            .to_rect()
            .inset(-border_width / 2.)
            .to_rounded_rect(2.);

        fill_lin_gradient(
            scene,
            &progress_rect,
            [theme::PRIMARY_LIGHT, theme::PRIMARY_DARK],
            UnitPoint::TOP,
            UnitPoint::BOTTOM,
        );
        stroke(scene, &progress_rect, theme::BORDER_DARK, border_width);

        // center text
        let widget_size = ctx.size();
        let label_size = self.label.size();
        let text_pos = Point::new(
            ((widget_size.width - label_size.width) * 0.5).max(0.),
            ((widget_size.height - label_size.height) * 0.5).max(0.),
        );
        self.label.draw(scene, text_pos);
    }

    fn accessibility_role(&self) -> Role {
        Role::ProgressIndicator
    }

    fn accessibility(&mut self, ctx: &mut AccessCtx) {
        ctx.current_node().set_value(self.value_accessibility());
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        smallvec![]
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("ProgressBar")
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
    use crate::testing::{widget_ids, TestHarness, TestWidgetExt};

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
}
