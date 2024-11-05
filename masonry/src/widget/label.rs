// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A label widget.

use accesskit::{NodeBuilder, Role};
use parley::fontique::Weight;
use parley::layout::Alignment;
use parley::style::{FontFamily, FontStack};
use smallvec::SmallVec;
use tracing::{trace_span, Span};
use vello::kurbo::{Affine, Point, Size};
use vello::peniko::BlendMode;
use vello::Scene;

use crate::text::{ArcStr, TextBrush, TextLayout};
use crate::widget::WidgetMut;
use crate::{
    AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, PaintCtx, PointerEvent,
    RegisterCtx, TextEvent, Update, UpdateCtx, Widget, WidgetId,
};

// added padding between the edges of the widget and the text.
pub(super) const PROSE_X_PADDING: f64 = 2.0;

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

/// A widget displaying non-interactive text.
///
/// This is useful for creating interactive widgets which internally
/// need support for displaying text, such as a button.
pub struct Label {
    text: ArcStr,
    text_changed: bool,
    text_layout: TextLayout,
    line_break_mode: LineBreaking,
    show_disabled: bool,
    brush: TextBrush,
}

// --- MARK: BUILDERS ---
impl Label {
    /// Create a new label.
    pub fn new(text: impl Into<ArcStr>) -> Self {
        Self {
            text: text.into(),
            text_changed: false,
            text_layout: TextLayout::new(crate::theme::TEXT_SIZE_NORMAL as f32),
            line_break_mode: LineBreaking::Overflow,
            show_disabled: true,
            brush: crate::theme::TEXT_COLOR.into(),
        }
    }

    pub fn text(&self) -> &ArcStr {
        &self.text
    }

    #[doc(alias = "with_text_color")]
    pub fn with_text_brush(mut self, brush: impl Into<TextBrush>) -> Self {
        self.text_layout.set_brush(brush);
        self
    }

    #[doc(alias = "with_font_size")]
    pub fn with_text_size(mut self, size: f32) -> Self {
        self.text_layout.set_text_size(size);
        self
    }

    pub fn with_weight(mut self, weight: Weight) -> Self {
        self.text_layout.set_weight(weight);
        self
    }

    pub fn with_text_alignment(mut self, alignment: Alignment) -> Self {
        self.text_layout.set_text_alignment(alignment);
        self
    }

    pub fn with_font(mut self, font: FontStack<'static>) -> Self {
        self.text_layout.set_font(font);
        self
    }
    pub fn with_font_family(self, font: FontFamily<'static>) -> Self {
        self.with_font(FontStack::Single(font))
    }

    pub fn with_line_break_mode(mut self, line_break_mode: LineBreaking) -> Self {
        self.line_break_mode = line_break_mode;
        self
    }

    /// Create a label with empty text.
    pub fn empty() -> Self {
        Self::new("")
    }
}

// --- MARK: WIDGETMUT ---
impl Label {
    pub fn set_text_properties<R>(
        this: &mut WidgetMut<'_, Self>,
        f: impl FnOnce(&mut TextLayout) -> R,
    ) -> R {
        let ret = f(&mut this.widget.text_layout);
        if this.widget.text_layout.needs_rebuild() {
            this.ctx.request_layout();
        }
        ret
    }

    pub fn set_text(this: &mut WidgetMut<'_, Self>, new_text: impl Into<ArcStr>) {
        let new_text = new_text.into();
        this.widget.text = new_text;
        this.widget.text_changed = true;
        this.ctx.request_layout();
    }

    #[doc(alias = "set_text_color")]
    pub fn set_text_brush(this: &mut WidgetMut<'_, Self>, brush: impl Into<TextBrush>) {
        let brush = brush.into();
        this.widget.brush = brush;
        if !this.ctx.is_disabled() {
            let brush = this.widget.brush.clone();
            Self::set_text_properties(this, |layout| layout.set_brush(brush));
        }
    }
    pub fn set_text_size(this: &mut WidgetMut<'_, Self>, size: f32) {
        Self::set_text_properties(this, |layout| layout.set_text_size(size));
    }
    pub fn set_weight(this: &mut WidgetMut<'_, Self>, weight: Weight) {
        Self::set_text_properties(this, |layout| layout.set_weight(weight));
    }
    pub fn set_alignment(this: &mut WidgetMut<'_, Self>, alignment: Alignment) {
        Self::set_text_properties(this, |layout| layout.set_text_alignment(alignment));
    }
    pub fn set_font(this: &mut WidgetMut<'_, Self>, font_stack: FontStack<'static>) {
        Self::set_text_properties(this, |layout| layout.set_font(font_stack));
    }
    pub fn set_font_family(this: &mut WidgetMut<'_, Self>, family: FontFamily<'static>) {
        Self::set_font(this, FontStack::Single(family));
    }
    pub fn set_line_break_mode(this: &mut WidgetMut<'_, Self>, line_break_mode: LineBreaking) {
        this.widget.line_break_mode = line_break_mode;
        this.ctx.request_layout();
    }
}

// --- MARK: IMPL WIDGET ---
impl Widget for Label {
    fn on_pointer_event(&mut self, _ctx: &mut EventCtx, _event: &PointerEvent) {}

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}

    fn on_access_event(&mut self, _ctx: &mut EventCtx, _event: &AccessEvent) {}

    fn register_children(&mut self, _ctx: &mut RegisterCtx) {}

    fn update(&mut self, ctx: &mut UpdateCtx, event: &Update) {
        match event {
            Update::DisabledChanged(disabled) => {
                if self.show_disabled {
                    if *disabled {
                        self.text_layout
                            .set_brush(crate::theme::DISABLED_TEXT_COLOR);
                    } else {
                        self.text_layout.set_brush(self.brush.clone());
                    }
                }
                // TODO: Parley seems to require a relayout when colours change
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
            Some(bc.max().width as f32 - 2. * PROSE_X_PADDING as f32)
        } else if bc.min().width.is_sign_negative() {
            Some(0.0)
        } else {
            None
        };
        self.text_layout.set_max_advance(max_advance);
        if self.text_layout.needs_rebuild() || self.text_changed {
            let (font_ctx, layout_ctx) = ctx.text_contexts();
            self.text_layout
                .rebuild(font_ctx, layout_ctx, &self.text, self.text_changed);
            self.text_changed = false;
        }
        // We would like to ignore trailing whitespace for a label.
        // However, Parley doesn't make that an option when using `max_advance`.
        // If we aren't wrapping words, we can safely ignore this, however.
        let text_size = self.text_layout.size();
        let label_size = Size {
            height: text_size.height,
            width: text_size.width + 2. * PROSE_X_PADDING,
        };
        bc.constrain(label_size)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        if self.text_layout.needs_rebuild() {
            debug_panic!(
                "Called {name}::paint with invalid layout",
                name = self.short_type_name()
            );
        }
        if self.line_break_mode == LineBreaking::Clip {
            let clip_rect = ctx.size().to_rect();
            scene.push_layer(BlendMode::default(), 1., Affine::IDENTITY, &clip_rect);
        }
        self.text_layout
            .draw(scene, Point::new(PROSE_X_PADDING, 0.0));

        if self.line_break_mode == LineBreaking::Clip {
            scene.pop_layer();
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Label
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx, node: &mut NodeBuilder) {
        node.set_name(self.text().as_ref().to_string());
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        SmallVec::new()
    }

    fn accepts_pointer_interaction(&self) -> bool {
        false
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("Label")
    }

    fn get_debug_text(&self) -> Option<String> {
        Some(self.text.to_string())
    }
}

// --- MARK: TESTS ---
#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;
    use parley::style::GenericFamily;

    use super::*;
    use crate::assert_render_snapshot;
    use crate::testing::TestHarness;
    use crate::theme::{PRIMARY_DARK, PRIMARY_LIGHT};
    use crate::widget::{CrossAxisAlignment, Flex, SizedBox};

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
            .with_text_brush(PRIMARY_LIGHT)
            .with_font_family(FontFamily::Generic(GenericFamily::Monospace))
            .with_text_size(20.0)
            .with_line_break_mode(LineBreaking::WordWrap)
            .with_text_alignment(Alignment::Middle);

        let mut harness = TestHarness::create_with_size(label, Size::new(200.0, 200.0));

        assert_render_snapshot!(harness, "styled_label");
    }

    #[test]
    /// A wrapping label's alignment should be respected, regardkess of
    /// its parent's alignment.
    fn label_alignment_flex() {
        fn base_label() -> Label {
            Label::new("Hello")
                .with_text_size(10.0)
                .with_line_break_mode(LineBreaking::WordWrap)
        }
        let label1 = base_label().with_text_alignment(Alignment::Start);
        let label2 = base_label().with_text_alignment(Alignment::Middle);
        let label3 = base_label().with_text_alignment(Alignment::End);
        let label4 = base_label().with_text_alignment(Alignment::Start);
        let label5 = base_label().with_text_alignment(Alignment::Middle);
        let label6 = base_label().with_text_alignment(Alignment::End);
        let flex = Flex::column()
            .with_flex_child(label1, CrossAxisAlignment::Start)
            .with_flex_child(label2, CrossAxisAlignment::Start)
            .with_flex_child(label3, CrossAxisAlignment::Start)
            .with_flex_child(label4, CrossAxisAlignment::Center)
            .with_flex_child(label5, CrossAxisAlignment::Center)
            .with_flex_child(label6, CrossAxisAlignment::Center)
            .gap(0.0);

        let mut harness = TestHarness::create_with_size(flex, Size::new(80.0, 80.0));

        assert_render_snapshot!(harness, "label_alignment_flex");
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
                .with_text_brush(PRIMARY_LIGHT)
                .with_font_family(FontFamily::Generic(GenericFamily::Monospace))
                .with_text_size(20.0)
                .with_line_break_mode(LineBreaking::WordWrap)
                .with_text_alignment(Alignment::Middle);

            let mut harness = TestHarness::create_with_size(label, Size::new(50.0, 50.0));

            harness.render()
        };

        let image_2 = {
            let label = Label::new("Hello world")
                .with_text_brush(PRIMARY_DARK)
                .with_text_size(40.0);

            let mut harness = TestHarness::create_with_size(label, Size::new(50.0, 50.0));

            harness.edit_root_widget(|mut label| {
                let mut label = label.downcast::<Label>();
                Label::set_text(&mut label, "The quick brown fox jumps over the lazy dog");
                Label::set_text_brush(&mut label, PRIMARY_LIGHT);
                Label::set_font_family(&mut label, FontFamily::Generic(GenericFamily::Monospace));
                Label::set_text_size(&mut label, 20.0);
                Label::set_line_break_mode(&mut label, LineBreaking::WordWrap);
                Label::set_alignment(&mut label, Alignment::Middle);
            });

            harness.render()
        };

        // We don't use assert_eq because we don't want rich assert
        assert!(image_1 == image_2);
    }
}
