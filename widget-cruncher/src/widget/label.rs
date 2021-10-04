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
use std::ops::{Deref, DerefMut};

use druid_shell::Cursor;

use crate::kurbo::Vec2;
use crate::text::{TextAlignment, TextLayout};
use crate::widget::prelude::*;
use crate::{ArcStr, Color, Data, FontDescriptor, KeyOrValue, Point};
use tracing::{instrument, trace};

// added padding between the edges of the widget and the text.
const LABEL_X_PADDING: f64 = 2.0;

pub struct Label {
    current_text: ArcStr,
    text_layout: TextLayout<ArcStr>,
    line_break_mode: LineBreaking,

    disabled: bool,
    default_text_color: KeyOrValue<Color>,
}

/// Options for handling lines that are too wide for the label.
#[derive(Debug, Clone, Copy, PartialEq, Data)]
pub enum LineBreaking {
    /// Lines are broken at word boundaries.
    WordWrap,
    /// Lines are truncated to the width of the label.
    Clip,
    /// Lines overflow the label.
    Overflow,
}

// --- METHODS ---

impl Label {
    /// Create a new `Label`.
    pub fn new(text: impl Into<ArcStr>) -> Self {
        let current_text = text.into();
        let mut text_layout = TextLayout::new();
        text_layout.set_text(current_text.clone());

        Self {
            current_text,
            text_layout,
            line_break_mode: LineBreaking::Overflow,
            disabled: false,
            default_text_color: crate::theme::TEXT_COLOR.into(),
        }
    }

    pub fn empty() -> Self {
        Self {
            current_text: "".into(),
            text_layout: TextLayout::new(),
            line_break_mode: LineBreaking::Overflow,
            disabled: false,
            default_text_color: crate::theme::TEXT_COLOR.into(),
        }
    }

    /// Builder-style method for setting the text color.
    ///
    /// The argument can be either a `Color` or a [`Key<Color>`].
    ///
    /// [`Key<Color>`]: ../struct.Key.html
    pub fn with_text_color(mut self, color: impl Into<KeyOrValue<Color>>) -> Self {
        self.set_text_color(color);
        self
    }

    /// Builder-style method for setting the text size.
    ///
    /// The argument can be either an `f64` or a [`Key<f64>`].
    ///
    /// [`Key<f64>`]: ../struct.Key.html
    pub fn with_text_size(mut self, size: impl Into<KeyOrValue<f64>>) -> Self {
        self.set_text_size(size);
        self
    }

    /// Builder-style method for setting the font.
    ///
    /// The argument can be a [`FontDescriptor`] or a [`Key<FontDescriptor>`]
    /// that refers to a font defined in the [`Env`].
    ///
    /// [`Env`]: ../struct.Env.html
    /// [`FontDescriptor`]: ../struct.FontDescriptor.html
    /// [`Key<FontDescriptor>`]: ../struct.Key.html
    pub fn with_font(mut self, font: impl Into<KeyOrValue<FontDescriptor>>) -> Self {
        self.set_font(font);
        self
    }

    /// Builder-style method to set the [`LineBreaking`] behaviour.
    ///
    /// [`LineBreaking`]: enum.LineBreaking.html
    pub fn with_line_break_mode(mut self, mode: LineBreaking) -> Self {
        self.set_line_break_mode(mode);
        self
    }

    /// Builder-style method to set the [`TextAlignment`].
    ///
    /// [`TextAlignment`]: enum.TextAlignment.html
    pub fn with_text_alignment(mut self, alignment: TextAlignment) -> Self {
        self.set_text_alignment(alignment);
        self
    }

    /// Return the current value of the label's text.
    pub fn text(&self) -> ArcStr {
        self.current_text.clone()
    }

    /// Set the text.
    pub fn set_text(&mut self, new_text: impl Into<ArcStr>) {
        self.text_layout.set_text(new_text.into());
    }

    /// Set the text color.
    ///
    /// The argument can be either a `Color` or a [`Key<Color>`].
    ///
    /// If you change this property, you are responsible for calling
    /// [`request_layout`] to ensure the label is updated.
    ///
    /// [`request_layout`]: ../struct.EventCtx.html#method.request_layout
    /// [`Key<Color>`]: ../struct.Key.html
    pub fn set_text_color(&mut self, color: impl Into<KeyOrValue<Color>>) {
        let color = color.into();
        if !self.disabled {
            self.text_layout.set_text_color(color.clone());
        }
        self.default_text_color = color;
    }

    /// Set the text size.
    ///
    /// The argument can be either an `f64` or a [`Key<f64>`].
    ///
    /// If you change this property, you are responsible for calling
    /// [`request_layout`] to ensure the label is updated.
    ///
    /// [`request_layout`]: ../struct.EventCtx.html#method.request_layout
    /// [`Key<f64>`]: ../struct.Key.html
    pub fn set_text_size(&mut self, size: impl Into<KeyOrValue<f64>>) {
        self.text_layout.set_text_size(size);
    }

    /// Set the font.
    ///
    /// The argument can be a [`FontDescriptor`] or a [`Key<FontDescriptor>`]
    /// that refers to a font defined in the [`Env`].
    ///
    /// If you change this property, you are responsible for calling
    /// [`request_layout`] to ensure the label is updated.
    ///
    /// [`request_layout`]: ../struct.EventCtx.html#method.request_layout
    /// [`Env`]: ../struct.Env.html
    /// [`FontDescriptor`]: ../struct.FontDescriptor.html
    /// [`Key<FontDescriptor>`]: ../struct.Key.html
    pub fn set_font(&mut self, font: impl Into<KeyOrValue<FontDescriptor>>) {
        self.text_layout.set_font(font);
    }

    /// Set the [`LineBreaking`] behaviour.
    ///
    /// If you change this property, you are responsible for calling
    /// [`request_layout`] to ensure the label is updated.
    ///
    /// [`request_layout`]: ../struct.EventCtx.html#method.request_layout
    /// [`LineBreaking`]: enum.LineBreaking.html
    pub fn set_line_break_mode(&mut self, mode: LineBreaking) {
        self.line_break_mode = mode;
    }

    /// Set the [`TextAlignment`] for this layout.
    ///
    /// [`TextAlignment`]: enum.TextAlignment.html
    pub fn set_text_alignment(&mut self, alignment: TextAlignment) {
        self.text_layout.set_text_alignment(alignment);
    }

    /// Draw this label's text at the provided `Point`, without internal padding.
    ///
    /// This is a convenience for widgets that want to use Label as a way
    /// of managing a dynamic or localized string, but want finer control
    /// over where the text is drawn.
    pub fn draw_at(&self, ctx: &mut PaintCtx, origin: impl Into<Point>) {
        self.text_layout.draw(ctx, origin)
    }

    /// Return the offset of the first baseline relative to the bottom of the widget.
    pub fn baseline_offset(&self) -> f64 {
        let text_metrics = self.text_layout.layout_metrics();
        text_metrics.size.height - text_metrics.first_baseline
    }
}

// --- TRAIT IMPLS ---

impl Widget for Label {
    #[instrument(name = "Label", level = "trace", skip(self, ctx, event, _env))]
    fn on_event(&mut self, ctx: &mut EventCtx, event: &Event, _env: &Env) {
        match event {
            Event::MouseUp(event) => {
                // Account for the padding
                let pos = event.pos - Vec2::new(LABEL_X_PADDING, 0.0);
                if let Some(link) = self.text_layout.link_for_pos(pos) {
                    todo!();
                    //ctx.submit_command(link.command.clone());
                }
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
            _ => {}
        }
    }

    #[instrument(name = "Label", level = "trace", skip(self, ctx, event, _env))]
    fn on_status_change(&mut self, ctx: &mut LifeCycleCtx, event: &StatusChange, _env: &Env) {
        match event {
            StatusChange::DisabledChanged(disabled) => {
                let color = if *disabled {
                    KeyOrValue::Key(crate::theme::DISABLED_TEXT_COLOR)
                } else {
                    self.default_text_color.clone()
                };
                self.text_layout.set_text_color(color);
                ctx.request_layout();
            }
            _ => {}
        }
    }

    #[instrument(name = "Label", level = "trace", skip(self, ctx, event, _env))]
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, _env: &Env) {}

    #[instrument(name = "Label", level = "trace", skip(self, ctx, bc, env))]
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, env: &Env) -> Size {
        bc.debug_check("Label");

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

    #[instrument(name = "Label", level = "trace", skip(self, ctx, _env))]
    fn paint(&mut self, ctx: &mut PaintCtx, _env: &Env) {
        let origin = Point::new(LABEL_X_PADDING, 0.0);
        let label_size = ctx.size();

        if self.line_break_mode == LineBreaking::Clip {
            ctx.clip(label_size.to_rect());
        }
        self.draw_at(ctx, origin)
    }

    fn children(&self) -> SmallVec<[&dyn AsWidgetPod; 16]> {
        SmallVec::new()
    }

    fn children_mut(&mut self) -> SmallVec<[&mut dyn AsWidgetPod; 16]> {
        SmallVec::new()
    }
}
