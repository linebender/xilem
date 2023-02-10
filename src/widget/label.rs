// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

//! A label widget.

// TODO
// - set text
// - set text attributes

use druid_shell::Cursor;
use smallvec::SmallVec;
use tracing::{trace, trace_span, Span};

use crate::kurbo::Vec2;
use crate::text::{FontDescriptor, TextAlignment, TextLayout};
use crate::widget::WidgetRef;
use crate::{
    ArcStr, BoxConstraints, Color, Data, Env, Event, EventCtx, KeyOrValue, LayoutCtx, LifeCycle,
    LifeCycleCtx, PaintCtx, Point, RenderContext, Size, StatusChange, Widget,
};

// added padding between the edges of the widget and the text.
const LABEL_X_PADDING: f64 = 2.0;

/// A widget displaying non-editable text.
pub struct Label {
    current_text: ArcStr,
    text_layout: TextLayout<ArcStr>,
    line_break_mode: LineBreaking,

    disabled: bool,
    default_text_color: KeyOrValue<Color>,
}

crate::declare_widget!(LabelMut, Label);

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

impl Label {
    /// Create a new label.
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

    /// Create a label with empty text.
    pub fn empty() -> Self {
        Self {
            current_text: "".into(),
            text_layout: TextLayout::new(),
            line_break_mode: LineBreaking::Overflow,
            disabled: false,
            default_text_color: crate::theme::TEXT_COLOR.into(),
        }
    }

    /// Builder-style method for setting the text string.
    pub fn with_text(mut self, new_text: impl Into<ArcStr>) -> Self {
        self.text_layout.set_text(new_text.into());
        self
    }

    /// Builder-style method for setting the text color.
    ///
    /// The argument can be either a `Color` or a [`Key<Color>`].
    ///
    /// [`Key<Color>`]: ../struct.Key.html
    pub fn with_text_color(mut self, color: impl Into<KeyOrValue<Color>>) -> Self {
        let color = color.into();
        if !self.disabled {
            self.text_layout.set_text_color(color.clone());
        }
        self.default_text_color = color;
        self
    }

    /// Builder-style method for setting the text size.
    ///
    /// The argument can be either an `f64` or a [`Key<f64>`].
    ///
    /// [`Key<f64>`]: ../struct.Key.html
    pub fn with_text_size(mut self, size: impl Into<KeyOrValue<f64>>) -> Self {
        self.text_layout.set_text_size(size);
        self
    }

    // FIXME - with_font cancels with_text_size
    // TODO - write failing test for this case
    /// Builder-style method for setting the font.
    ///
    /// The argument can be a [`FontDescriptor`] or a [`Key<FontDescriptor>`]
    /// that refers to a font defined in the [`Env`].
    ///
    /// [`Key<FontDescriptor>`]: ../struct.Key.html
    pub fn with_font(mut self, font: impl Into<KeyOrValue<FontDescriptor>>) -> Self {
        self.text_layout.set_font(font);
        self
    }

    /// Builder-style method to set the [`LineBreaking`] behaviour.
    pub fn with_line_break_mode(mut self, mode: LineBreaking) -> Self {
        self.line_break_mode = mode;
        self
    }

    /// Builder-style method to set the [`TextAlignment`].
    pub fn with_text_alignment(mut self, alignment: TextAlignment) -> Self {
        self.text_layout.set_text_alignment(alignment);
        self
    }

    /// Return the current value of the label's text.
    pub fn text(&self) -> ArcStr {
        self.current_text.clone()
    }

    /// Return the offset of the first baseline relative to the bottom of the widget.
    pub fn baseline_offset(&self) -> f64 {
        let text_metrics = self.text_layout.layout_metrics();
        text_metrics.size.height - text_metrics.first_baseline
    }

    /// Draw this label's text at the provided `Point`, without internal padding.
    ///
    /// This is a convenience for widgets that want to use Label as a way
    /// of managing a dynamic or localized string, but want finer control
    /// over where the text is drawn.
    pub fn draw_at(&self, ctx: &mut PaintCtx, origin: impl Into<Point>) {
        self.text_layout.draw(ctx, origin)
    }
}

impl LabelMut<'_, '_> {
    /// Set the text.
    pub fn set_text(&mut self, new_text: impl Into<ArcStr>) {
        self.1.text_layout.set_text(new_text.into());
        self.0.request_layout();
    }

    /// Set the text color.
    ///
    /// The argument can be either a `Color` or a [`Key<Color>`].
    /// [`Key<Color>`]: ../struct.Key.html
    pub fn set_text_color(&mut self, color: impl Into<KeyOrValue<Color>>) {
        let color = color.into();
        if !self.1.disabled {
            self.1.text_layout.set_text_color(color.clone());
        }
        self.1.default_text_color = color;
        self.0.request_layout();
    }

    /// Set the text size.
    ///
    /// The argument can be either an `f64` or a [`Key<f64>`].
    ///
    /// [`Key<f64>`]: ../struct.Key.html
    pub fn set_text_size(&mut self, size: impl Into<KeyOrValue<f64>>) {
        self.1.text_layout.set_text_size(size);
        self.0.request_layout();
    }

    /// Set the font.
    ///
    /// The argument can be a [`FontDescriptor`] or a [`Key<FontDescriptor>`]
    /// that refers to a font defined in the [`Env`].
    ///
    /// [`Key<FontDescriptor>`]: ../struct.Key.html
    pub fn set_font(&mut self, font: impl Into<KeyOrValue<FontDescriptor>>) {
        self.1.text_layout.set_font(font);
        self.0.request_layout();
    }

    /// Set the [`LineBreaking`] behaviour.
    pub fn set_line_break_mode(&mut self, mode: LineBreaking) {
        self.1.line_break_mode = mode;
        self.0.request_layout();
    }

    /// Set the [`TextAlignment`] for this layout.
    pub fn set_text_alignment(&mut self, alignment: TextAlignment) {
        self.1.text_layout.set_text_alignment(alignment);
        self.0.request_layout();
    }
}

// --- TRAIT IMPLS ---

impl Widget for Label {
    fn on_event(&mut self, ctx: &mut EventCtx, event: &Event, _env: &Env) {
        match event {
            Event::MouseUp(event) => {
                // Account for the padding
                let pos = event.pos - Vec2::new(LABEL_X_PADDING, 0.0);
                if let Some(_link) = self.text_layout.link_for_pos(pos) {
                    todo!();
                    //ctx.submit_command(link.command.clone());
                    // See issue #21
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

    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, _event: &StatusChange, _env: &Env) {}

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, _env: &Env) {
        match event {
            LifeCycle::DisabledChanged(disabled) => {
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
        self.draw_at(ctx, origin)
    }

    fn children(&self) -> SmallVec<[WidgetRef<'_, dyn Widget>; 16]> {
        SmallVec::new()
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("Label")
    }

    fn get_debug_text(&self) -> Option<String> {
        Some(self.current_text.to_string())
    }
}

impl Data for LineBreaking {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

#[cfg(test)]
mod tests {
    use crate::piet::FontFamily;
    use insta::assert_debug_snapshot;

    use super::*;
    use crate::assert_render_snapshot;
    use crate::testing::TestHarness;
    use crate::theme::{PRIMARY_DARK, PRIMARY_LIGHT};
    use crate::widget::{Flex, SizedBox};

    #[test]
    fn simple_label() {
        let label = Label::new("Hello");

        let mut harness = TestHarness::create(label);

        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "hello");
    }

    #[test]
    fn styled_label() {
        let label = Label::new("The quick brown fox jumps over the lazy dog")
            .with_text_color(PRIMARY_LIGHT)
            .with_font(FontDescriptor::new(FontFamily::MONOSPACE))
            .with_text_size(20.0)
            .with_line_break_mode(LineBreaking::WordWrap)
            .with_text_alignment(TextAlignment::Center);

        let mut harness = TestHarness::create_with_size(label, Size::new(200.0, 200.0));

        assert_render_snapshot!(harness, "styled_label");
    }

    #[test]
    fn line_break_modes() {
        let widget = Flex::column()
            .with_flex_spacer(1.0)
            .with_child(
                SizedBox::new(
                    Label::new("The quick brown fox jumps over the lazy dog")
                        .with_line_break_mode(LineBreaking::WordWrap),
                )
                .width(200.0),
            )
            .with_spacer(20.0)
            .with_child(
                SizedBox::new(
                    Label::new("The quick brown fox jumps over the lazy dog")
                        .with_line_break_mode(LineBreaking::Clip),
                )
                .width(200.0),
            )
            .with_spacer(20.0)
            .with_child(
                SizedBox::new(
                    Label::new("The quick brown fox jumps over the lazy dog")
                        .with_line_break_mode(LineBreaking::Overflow),
                )
                .width(200.0),
            )
            .with_flex_spacer(1.0);

        let mut harness = TestHarness::create(widget);

        assert_render_snapshot!(harness, "line_break_modes");
    }

    #[test]
    fn edit_label() {
        let image_1 = {
            let label = Label::new("The quick brown fox jumps over the lazy dog")
                .with_text_color(PRIMARY_LIGHT)
                .with_font(FontDescriptor::new(FontFamily::MONOSPACE))
                .with_text_size(20.0)
                .with_line_break_mode(LineBreaking::WordWrap)
                .with_text_alignment(TextAlignment::Center);

            let mut harness = TestHarness::create_with_size(label, Size::new(50.0, 50.0));

            harness.render()
        };

        let image_2 = {
            let label = Label::new("Hello world")
                .with_text_color(PRIMARY_DARK)
                .with_text_size(40.0);

            let mut harness = TestHarness::create_with_size(label, Size::new(50.0, 50.0));

            harness.edit_root_widget(|mut label, _| {
                let mut label = label.downcast::<Label>().unwrap();
                label.set_text("The quick brown fox jumps over the lazy dog");
                label.set_text_color(PRIMARY_LIGHT);
                label.set_font(FontDescriptor::new(FontFamily::MONOSPACE));
                label.set_text_size(20.0);
                label.set_line_break_mode(LineBreaking::WordWrap);
                label.set_text_alignment(TextAlignment::Center);
            });

            harness.render()
        };

        // We don't use assert_eq because we don't want rich assert
        assert!(image_1 == image_2);
    }
}
