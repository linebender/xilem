// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

//! A label widget.

#![allow(clippy::single_match)]

use std::{thread, time};

use druid_shell::Cursor;
use masonry::kurbo::Vec2;
use masonry::promise::PromiseToken;
use masonry::text::TextLayout;
use masonry::widget::prelude::*;
use masonry::widget::WidgetRef;
use masonry::{AppLauncher, WindowDescription};
use masonry::{ArcStr, Color, KeyOrValue, Point};
use smallvec::SmallVec;
use tracing::{trace, trace_span, Span};

// added padding between the edges of the widget and the text.
const LABEL_X_PADDING: f64 = 2.0;

pub struct PromiseButton {
    value: u32,
    text_layout: TextLayout<ArcStr>,
    line_break_mode: LineBreaking,
    promise_token: PromiseToken<u32>,

    default_text_color: KeyOrValue<Color>,
}

/// Options for handling lines that are too wide for the label.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineBreaking {
    /// Lines are broken at word boundaries.
    WordWrap,
    /// Lines are truncated to the width of the label.
    Clip,
    /// Lines overflow the label.
    Overflow,
}

// --- METHODS ---

impl PromiseButton {
    /// Create a new `PromiseButton`.
    pub fn new(text: impl Into<ArcStr>) -> Self {
        let mut text_layout = TextLayout::new();
        text_layout.set_text(text.into());

        Self {
            value: 0,
            text_layout,
            line_break_mode: LineBreaking::Overflow,
            promise_token: PromiseToken::empty(),
            default_text_color: masonry::theme::TEXT_COLOR.into(),
        }
    }
}

// --- TRAIT IMPLS ---

impl Widget for PromiseButton {
    fn on_event(&mut self, ctx: &mut EventCtx, event: &Event, _env: &Env) {
        match event {
            Event::MouseUp(_event) => {
                let value = self.value;
                self.promise_token = ctx.compute_in_background(move |_| {
                    // "sleep" stands in for a long computation, a download, etc.
                    thread::sleep(time::Duration::from_millis(2000));
                    value + 1
                });
                self.text_layout.set_text("Loading ...".into());
                ctx.request_layout();
            }
            Event::MouseMove(event) => {
                // Account for the padding
                let pos = event.pos - Vec2::new(LABEL_X_PADDING, 0.0);

                if self.text_layout.link_for_pos(pos).is_some() {
                    ctx.set_cursor(&Cursor::Pointer);
                } else {
                    ctx.clear_cursor();
                }
            }
            Event::PromiseResult(result) => {
                if let Some(new_value) = result.try_get(self.promise_token) {
                    self.text_layout
                        .set_text(format!("New value: {new_value}").into());
                    self.value = new_value;
                    ctx.request_layout();
                }
            }
            _ => {}
        }
    }

    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, _event: &StatusChange, _env: &Env) {}

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, _env: &Env) {
        match event {
            LifeCycle::DisabledChanged(disabled) => {
                let color = if *disabled {
                    KeyOrValue::Key(masonry::theme::DISABLED_TEXT_COLOR)
                } else {
                    self.default_text_color.clone()
                };
                self.text_layout.set_text_color(color);
                ctx.request_layout();
            }
            _ => {}
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, env: &Env) -> Size {
        let width = match self.line_break_mode {
            LineBreaking::WordWrap => bc.max().width - LABEL_X_PADDING * 2.0,
            _ => f64::INFINITY,
        };

        self.text_layout.set_wrap_width(width);
        self.text_layout.rebuild_if_needed(ctx.text(), env);

        let text_metrics = self.text_layout.layout_metrics();
        ctx.set_baseline_offset(text_metrics.size.height - text_metrics.first_baseline);
        let size = bc.constrain(Size::new(
            text_metrics.size.width + 2. * LABEL_X_PADDING,
            text_metrics.size.height,
        ));
        trace!("Computed size: {}", size);
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _env: &Env) {
        let origin = Point::new(LABEL_X_PADDING, 0.0);
        let label_size = ctx.size();

        if self.line_break_mode == LineBreaking::Clip {
            ctx.clip(label_size.to_rect());
        }
        self.text_layout.draw(ctx, origin)
    }

    fn children(&self) -> SmallVec<[WidgetRef<'_, dyn Widget>; 16]> {
        SmallVec::new()
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("PromiseButton")
    }
}

// ---

fn main() {
    let main_window =
        WindowDescription::new(PromiseButton::new("Hello")).title("Blocking functions");
    AppLauncher::with_window(main_window)
        .log_to_console()
        .launch()
        .expect("launch failed");
}
