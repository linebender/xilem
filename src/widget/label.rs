// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

//! A label widget.

use kurbo::Affine;
use parley::layout::Alignment;
use parley::style::{FontFamily, FontStack, GenericFamily, StyleProperty};
use parley::{FontContext, Layout};
use smallvec::SmallVec;
use tracing::{trace, trace_span, Span};
use vello::peniko::{BlendMode, Brush};
use vello::Scene;

use crate::widget::WidgetRef;
use crate::{
    ArcStr, BoxConstraints, Color, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    PointerEvent, Size, StatusChange, TextEvent, Widget,
};

// added padding between the edges of the widget and the text.
const LABEL_X_PADDING: f64 = 2.0;

/// A widget displaying non-editable text.
pub struct Label {
    current_text: ArcStr,
    text_layout: Option<Layout<Brush>>,
    text_size: f32,
    font_family: FontFamily<'static>,
    line_break_mode: LineBreaking,
    disabled: bool,
    text_color: Color,
    alignment: Alignment,
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
        Self {
            current_text,
            text_layout: None,
            text_color: crate::theme::TEXT_COLOR,
            text_size: crate::theme::TEXT_SIZE_NORMAL as f32,
            font_family: FontFamily::Generic(GenericFamily::SystemUi),
            line_break_mode: LineBreaking::Overflow,
            disabled: false,
            alignment: Alignment::Start,
        }
    }

    /// Create a label with empty text.
    pub fn empty() -> Self {
        Self::new("")
    }

    // TODO - Rename methods
    /// Builder-style method for setting the text string.
    pub fn with_text(mut self, new_text: impl Into<ArcStr>) -> Self {
        self.current_text = new_text.into();
        // TODO - Rethink how layout caching works during the builder phase
        self.text_layout = None;
        self
    }

    /// Builder-style method for setting the text color.
    pub fn with_text_color(mut self, color: impl Into<Color>) -> Self {
        self.text_color = color.into();
        self
    }

    /// Builder-style method for setting the text size.
    pub fn with_text_size(mut self, size: impl Into<f32>) -> Self {
        self.text_size = size.into();
        self
    }

    /// Builder-style method for setting the font.
    pub fn with_font_family(mut self, font_family: impl Into<FontFamily<'static>>) -> Self {
        self.font_family = font_family.into();
        self
    }

    /// Builder-style method to set the [`LineBreaking`] behaviour.
    pub fn with_line_break_mode(mut self, mode: LineBreaking) -> Self {
        self.line_break_mode = mode;
        self
    }

    /// Builder-style method to set the [`Alignment`].
    pub fn with_text_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Return the current value of the label's text.
    pub fn text(&self) -> ArcStr {
        self.current_text.clone()
    }

    #[cfg(FALSE)]
    /// Return the offset of the first baseline relative to the bottom of the widget.
    pub fn baseline_offset(&self) -> f64 {
        let text_metrics = self.text_layout.layout_metrics();
        text_metrics.size.height - text_metrics.first_baseline
    }

    fn get_layout_mut(&mut self, font_cx: &mut FontContext) -> &mut Layout<Brush> {
        let color = if self.disabled {
            crate::theme::DISABLED_TEXT_COLOR
        } else {
            self.text_color
        };
        let mut lcx = parley::LayoutContext::new();
        let mut layout_builder = lcx.ranged_builder(font_cx, &self.current_text, 1.0);

        layout_builder.push_default(&StyleProperty::FontStack(FontStack::Single(
            self.font_family,
        )));
        layout_builder.push_default(&StyleProperty::FontSize(self.text_size));
        layout_builder.push_default(&StyleProperty::Brush(Brush::Solid(color)));

        // TODO - Refactor. This code is mostly copy-pasted from Xilem's text widget
        // Not super elegant.
        self.text_layout = Some(layout_builder.build());
        self.text_layout.as_mut().unwrap()
    }
}

impl LabelMut<'_> {
    /// Set the text.
    pub fn set_text(&mut self, new_text: impl Into<ArcStr>) {
        self.widget.current_text = new_text.into();
        self.widget.text_layout = None;
        self.ctx.request_layout();
    }

    /// Set the text color.
    pub fn set_text_color(&mut self, color: impl Into<Color>) {
        self.widget.text_color = color.into();
        self.ctx.request_layout();
    }

    /// Set the text size.
    pub fn set_text_size(&mut self, size: impl Into<f32>) {
        self.widget.text_size = size.into();
        self.ctx.request_layout();
    }

    /// Set the font.
    pub fn set_font_family(&mut self, font_family: impl Into<FontFamily<'static>>) {
        self.widget.font_family = font_family.into();
        self.ctx.request_layout();
    }

    /// Set the [`LineBreaking`] behaviour.
    pub fn set_line_break_mode(&mut self, mode: LineBreaking) {
        self.widget.line_break_mode = mode;
        self.ctx.request_layout();
    }

    /// Set the [`Alignment`] for this layout.
    pub fn set_text_alignment(&mut self, alignment: Alignment) {
        self.widget.alignment = alignment;
        self.ctx.request_layout();
    }
}

// --- TRAIT IMPLS ---

impl Widget for Label {
    fn on_pointer_event(&mut self, _ctx: &mut EventCtx, _event: &PointerEvent) {}

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}

    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, _event: &StatusChange) {}

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle) {
        match event {
            LifeCycle::DisabledChanged(_) => {
                // TODO - only request paint
                ctx.request_layout();
            }
            _ => {}
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        // Compute max_advance from box constraints
        let max_advance = if self.line_break_mode != LineBreaking::WordWrap {
            None
        } else if bc.max().width.is_finite() {
            Some(bc.max().width as f32 - 2. * LABEL_X_PADDING as f32)
        } else if bc.min().width.is_sign_negative() {
            Some(0.0)
        } else {
            None
        };

        // TODO - Handle baseline

        // Lay text out
        let alignment = self.alignment;
        let layout = self.get_layout_mut(ctx.font_ctx());
        layout.break_all_lines(max_advance, alignment);
        let size = Size {
            width: layout.width() as f64 + 2. * LABEL_X_PADDING,
            height: layout.height() as f64,
        };
        let size = bc.constrain(size);
        trace!(
            "Computed layout: max={:?}. w={}, h={}",
            max_advance,
            size.width,
            size.height,
        );
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        if let Some(text_layout) = &self.text_layout {
            if self.line_break_mode == LineBreaking::Clip {
                let clip_rect = ctx.size().to_rect();
                scene.push_layer(BlendMode::default(), 1., Affine::IDENTITY, &clip_rect);
            }

            crate::text_helpers::render_text(
                scene,
                Affine::translate((LABEL_X_PADDING, 0.)),
                text_layout,
            );

            if self.line_break_mode == LineBreaking::Clip {
                scene.pop_layer();
            }
        }
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

// TODO - reenable tests
#[cfg(test)]
mod tests {
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
            .with_font_family(FontFamily::Generic(GenericFamily::Monospace))
            .with_text_size(20.0)
            .with_line_break_mode(LineBreaking::WordWrap)
            .with_text_alignment(Alignment::Middle);

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
                .with_font_family(FontFamily::Generic(GenericFamily::Monospace))
                .with_text_size(20.0)
                .with_line_break_mode(LineBreaking::WordWrap)
                .with_text_alignment(Alignment::Middle);

            let mut harness = TestHarness::create_with_size(label, Size::new(50.0, 50.0));

            harness.render()
        };

        let image_2 = {
            let label = Label::new("Hello world")
                .with_text_color(PRIMARY_DARK)
                .with_text_size(40.0);

            let mut harness = TestHarness::create_with_size(label, Size::new(50.0, 50.0));

            harness.edit_root_widget(|mut label| {
                let mut label = label.downcast::<Label>().unwrap();
                label.set_text("The quick brown fox jumps over the lazy dog");
                label.set_text_color(PRIMARY_LIGHT);
                label.set_font_family(FontFamily::Generic(GenericFamily::Monospace));
                label.set_text_size(20.0);
                label.set_line_break_mode(LineBreaking::WordWrap);
                label.set_text_alignment(Alignment::Middle);
            });

            harness.render()
        };

        // We don't use assert_eq because we don't want rich assert
        assert!(image_1 == image_2);
    }
}
