// Copyright 2019 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! A label widget.

use smallvec::SmallVec;
use std::{thread, time};
use tracing::{trace, trace_span, Span};

use druid_shell::Cursor;
use widget_cruncher::kurbo::Vec2;
use widget_cruncher::promise::PromiseToken;
use widget_cruncher::text::TextLayout;
use widget_cruncher::widget::prelude::*;
use widget_cruncher::{ArcStr, Color, KeyOrValue, Point};

use widget_cruncher::{AppLauncher, WindowDesc};

// added padding between the edges of the widget and the text.
const LABEL_X_PADDING: f64 = 2.0;

pub struct PromiseButton {
    value: u32,
    text_layout: TextLayout<ArcStr>,
    line_break_mode: LineBreaking,
    // TODO - PromiseToken dummy constructor?
    promise_token: Option<PromiseToken<u32>>,

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
        let current_text = text.into();
        let mut text_layout = TextLayout::new();
        text_layout.set_text(current_text.clone());

        Self {
            value: 0,
            text_layout,
            line_break_mode: LineBreaking::Overflow,
            promise_token: None,
            default_text_color: widget_cruncher::theme::TEXT_COLOR.into(),
        }
    }
}

// --- TRAIT IMPLS ---

impl Widget for PromiseButton {
    fn on_event(&mut self, ctx: &mut EventCtx, event: &Event, _env: &Env) {
        ctx.init();
        match event {
            Event::MouseUp(_event) => {
                let value = self.value;
                let token = ctx.compute_in_background(move |_| {
                    // "sleep" stands in for a long computation, a download, etc.
                    thread::sleep(time::Duration::from_millis(2000));
                    value + 1
                });
                self.promise_token = Some(token);
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
                if let Some(promise_token) = self.promise_token {
                    if let Some(new_value) = result.try_get(promise_token) {
                        self.text_layout
                            .set_text(format!("New value: {}", new_value).into());
                        self.value = new_value;
                        ctx.request_layout();
                    }
                }
            }
            _ => {}
        }
    }

    fn on_status_change(&mut self, ctx: &mut LifeCycleCtx, event: &StatusChange, _env: &Env) {
        ctx.init();
        match event {
            StatusChange::DisabledChanged(disabled) => {
                let color = if *disabled {
                    KeyOrValue::Key(widget_cruncher::theme::DISABLED_TEXT_COLOR)
                } else {
                    self.default_text_color.clone()
                };
                self.text_layout.set_text_color(color);
                ctx.request_layout();
            }
            _ => {}
        }
    }

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _env: &Env) {}

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, env: &Env) -> Size {
        ctx.init();
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
        ctx.init();
        let origin = Point::new(LABEL_X_PADDING, 0.0);
        let label_size = ctx.size();

        if self.line_break_mode == LineBreaking::Clip {
            ctx.clip(label_size.to_rect());
        }
        self.text_layout.draw(ctx, origin)
    }

    fn children(&self) -> SmallVec<[&dyn AsWidgetPod; 16]> {
        SmallVec::new()
    }

    fn children_mut(&mut self) -> SmallVec<[&mut dyn AsWidgetPod; 16]> {
        SmallVec::new()
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("PromiseButton")
    }
}

// ---

fn main() {
    let main_window = WindowDesc::new(PromiseButton::new("Hello")).title("Blocking functions");
    AppLauncher::with_window(main_window)
        .log_to_console()
        .launch()
        .expect("launch failed");
}
