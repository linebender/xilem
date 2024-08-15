// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A progress bar widget.

use accesskit::Role;
use kurbo::{Affine, Point, Rect};
use smallvec::{smallvec, SmallVec};
use tracing::{trace, trace_span, Span};
use vello::peniko::{BlendMode, Color};
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
    /// Animation state
    // TODO should we cache the gradient used for the animation
    animation: Animation,
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
            animation: Animation::new(),
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

    /// How much of the widget should the progress bar take up (in range `[0, 1]`)
    fn rel_bar_len(&self) -> f64 {
        self.progress.unwrap_or(1.).into()
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

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle) {
        match event {
            LifeCycle::WidgetAdded => ctx.request_anim_frame(),
            LifeCycle::AnimFrame(nanos) => {
                // TODO use timer for 'passive' part of the animation
                let nanos = *nanos as f64 / 1_000_000.;
                self.animation.step(nanos);
                ctx.request_anim_frame();
                ctx.request_paint();
            }
            _ => (),
        }
    }

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

        // update animation parameters
        self.animation
            .set_bar_len(our_size.width * self.rel_bar_len());
        self.animation.set_anim_width(our_size.height);

        trace!("Computed layout: size={}", our_size);
        our_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        let border_width = 1.;
        let size = ctx.size();

        if self.label.needs_rebuild() {
            debug_panic!("Called ProgressBar paint before layout");
        }

        let rect = size.to_rect().inset(-border_width / 2.).to_rounded_rect(2.);

        fill_lin_gradient(
            scene,
            &rect,
            [theme::BACKGROUND_LIGHT, theme::BACKGROUND_DARK],
            UnitPoint::TOP,
            UnitPoint::BOTTOM,
        );

        stroke(scene, &rect, theme::BORDER_DARK, border_width);

        let bar_len = self.rel_bar_len();
        let progress_rect_size = Size::new(size.width * bar_len, size.height);
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

        // animation
        scene.push_layer(BlendMode::default(), 1., Affine::IDENTITY, &progress_rect);
        const WHITE_TRANSPARENT: Color = Color::rgba8(255, 255, 255, 0);
        const WHITE_SEMITRANSPARENT: Color = Color::rgba8(255, 255, 255, 64);
        if let Some(pos) = self.animation.position() {
            let rect = Rect::from_origin_size((pos, 0.), (size.height, size.height));
            fill_lin_gradient(
                scene,
                &rect,
                [WHITE_TRANSPARENT, WHITE_SEMITRANSPARENT, WHITE_TRANSPARENT],
                UnitPoint::new(0., 0.4),
                UnitPoint::new(1., 0.6),
            );
        }
        scene.pop_layer();

        // center text
        let label_size = self.label.size();
        let text_pos = Point::new(
            ((size.width - label_size.width) * 0.5).max(0.),
            ((size.height - label_size.height) * 0.5).max(0.),
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

struct Animation {
    // configs
    /// The time to wait between animations, in ms.
    time_between_anims_ms: f64,
    /// The number of ms to take to cover 100 pixels (speed of animation)
    ms_per_100px: f64,

    /// a value in `[0, 1)`` that indicates how far through a cycle we are
    cycle_position: f64,
    /// The total length of the progress bar, used to calculate speed
    bar_len_px: f64,
    /// How wide the animation will be (for positioning)
    anim_width_px: f64,
}

impl Animation {
    /// 2 secs between animations
    const DEFAULT_TIME_BETWEEN_ANIMS_MS: f64 = 2000.;
    /// cover 500px per second
    const DEFAULT_MS_PER_100PX: f64 = 500.;

    fn new() -> Self {
        Self {
            time_between_anims_ms: Self::DEFAULT_TIME_BETWEEN_ANIMS_MS,
            ms_per_100px: Self::DEFAULT_MS_PER_100PX,

            cycle_position: 0.,
            // We set the bar length to some arbitrary value. If it is not updated then the animation
            // will change speed depending on the actual bar length
            bar_len_px: 400.,
            // Again, this should be overwritten with the correct value
            anim_width_px: 18.,
        }
    }

    fn set_bar_len(&mut self, bar_len: f64) {
        self.bar_len_px = bar_len;
        // no need to update cycle position
    }

    fn set_anim_width(&mut self, anim_width: f64) {
        self.anim_width_px = anim_width;
    }

    /// The position to draw the animation (or `None` if we shouldn't draw)
    fn position(&self) -> Option<f64> {
        let cycle_time = self.cycle_time();
        let waiting_end = self.time_between_anims_ms / cycle_time;
        if self.cycle_position < waiting_end {
            return None;
        }
        // We now know we are in the draw state
        // scale position to be on `[0, 1)` in the draw section
        let pos = (self.cycle_position - waiting_end) / (1. - waiting_end);
        // scale to pixels
        let pos = pos * self.bar_len_px;
        // scale pos so we start off the end and finish off the end
        // 0 -> -anim_width, end -> end
        let scale = (self.anim_width_px + self.bar_len_px) / self.bar_len_px;
        let pos = pos * scale - self.anim_width_px;
        Some(pos)
    }

    /// Step the animation forward by `delta_ms` milliseconds
    fn step(&mut self, delta_ms: f64) {
        // update time
        let delta = delta_ms / self.cycle_time();
        self.cycle_position = (self.cycle_position + delta).rem_euclid(1.);
    }

    /// milliseconds to move 1 pixel
    fn ms_per_pixel(&self) -> f64 {
        self.ms_per_100px * 0.01
    }

    /// Time for a whole animation cycle
    fn cycle_time(&self) -> f64 {
        self.time_between_anims_ms + self.ms_per_pixel() * self.bar_len_px
    }

    fn reset(&mut self) {
        self.cycle_position = 0.;
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
