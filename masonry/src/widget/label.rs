// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A label widget.

use accesskit::{NodeBuilder, Role};
use parley::layout::Alignment;
use parley::style::{FontFamily, FontStack};
use smallvec::SmallVec;
use tracing::{trace, trace_span, Span};
use vello::kurbo::{Affine, Point, Size};
use vello::peniko::BlendMode;
use vello::Scene;

use crate::text::{TextBrush, TextLayout};
use crate::widget::WidgetMut;
use crate::{
    AccessCtx, AccessEvent, ArcStr, BoxConstraints, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx,
    PaintCtx, PointerEvent, RegisterCtx, StatusChange, TextEvent, Widget, WidgetId,
};

// added padding between the edges of the widget and the text.
pub(super) const LABEL_X_PADDING: f64 = 2.0;

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

/// A widget displaying non-editable text.
pub struct Label {
    // We hardcode the underlying storage type as `ArcStr` for `Label`
    // More advanced use cases will almost certainly need a custom widget, anyway
    // (Rich text is not yet fully integrated, and so the architecture by which a label
    // has rich text properties specified still needs to be designed)
    text_layout: TextLayout<ArcStr>,
    line_break_mode: LineBreaking,
    show_disabled: bool,
    brush: TextBrush,
    skip_pointer: bool,
}

// --- MARK: BUILDERS ---
impl Label {
    /// Create a new label.
    pub fn new(text: impl Into<ArcStr>) -> Self {
        Self {
            text_layout: TextLayout::new(text.into(), crate::theme::TEXT_SIZE_NORMAL as f32),
            line_break_mode: LineBreaking::Overflow,
            show_disabled: true,
            brush: crate::theme::TEXT_COLOR.into(),
            skip_pointer: false,
        }
    }

    // TODO - Rename
    // TODO - Document
    pub fn with_skip_pointer(mut self, skip_pointer: bool) -> Self {
        self.skip_pointer = skip_pointer;
        self
    }

    pub fn text(&self) -> &ArcStr {
        self.text_layout.text()
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
impl WidgetMut<'_, Label> {
    pub fn text(&self) -> &ArcStr {
        self.widget.text_layout.text()
    }

    pub fn set_text_properties<R>(&mut self, f: impl FnOnce(&mut TextLayout<ArcStr>) -> R) -> R {
        let ret = f(&mut self.widget.text_layout);
        if self.widget.text_layout.needs_rebuild() {
            self.ctx.request_layout();
        }
        ret
    }

    pub fn set_text(&mut self, new_text: impl Into<ArcStr>) {
        let new_text = new_text.into();
        self.set_text_properties(|layout| layout.set_text(new_text));
    }

    #[doc(alias = "set_text_color")]
    pub fn set_text_brush(&mut self, brush: impl Into<TextBrush>) {
        let brush = brush.into();
        self.widget.brush = brush;
        if !self.ctx.is_disabled() {
            let brush = self.widget.brush.clone();
            self.set_text_properties(|layout| layout.set_brush(brush));
        }
    }
    pub fn set_text_size(&mut self, size: f32) {
        self.set_text_properties(|layout| layout.set_text_size(size));
    }
    pub fn set_alignment(&mut self, alignment: Alignment) {
        self.set_text_properties(|layout| layout.set_text_alignment(alignment));
    }
    pub fn set_font(&mut self, font_stack: FontStack<'static>) {
        self.set_text_properties(|layout| layout.set_font(font_stack));
    }
    pub fn set_font_family(&mut self, family: FontFamily<'static>) {
        self.set_font(FontStack::Single(family));
    }
    pub fn set_line_break_mode(&mut self, line_break_mode: LineBreaking) {
        self.widget.line_break_mode = line_break_mode;
        self.ctx.request_paint();
    }
}

// --- MARK: IMPL WIDGET ---
impl Widget for Label {
    fn on_pointer_event(&mut self, _ctx: &mut EventCtx, event: &PointerEvent) {
        match event {
            PointerEvent::PointerMove(_point) => {
                // TODO: Set cursor if over link
            }
            PointerEvent::PointerDown(_button, _state) => {
                // TODO: Start tracking currently pressed
                // (i.e. don't press)
            }
            PointerEvent::PointerUp(_button, _state) => {
                // TODO: Follow link (if not now dragging ?)
            }
            _ => {}
        }
    }

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {
        // If focused on a link and enter pressed, follow it?
        // TODO: This sure looks like each link needs its own widget, although I guess the challenge there is
        // that the bounding boxes can go e.g. across line boundaries?
    }

    fn on_access_event(&mut self, _ctx: &mut EventCtx, _event: &AccessEvent) {}

    fn register_children(&mut self, _ctx: &mut RegisterCtx) {}

    #[allow(missing_docs)]
    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, event: &StatusChange) {
        match event {
            StatusChange::FocusChanged(_) => {
                // TODO: Focus on first link
            }
            _ => {}
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle) {
        match event {
            LifeCycle::DisabledChanged(disabled) => {
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
            LifeCycle::BuildFocusChain => {}
            _ => {}
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        // Compute max_advance from box constraints
        let max_advance = if self.line_break_mode != LineBreaking::WordWrap {
            None
        } else if bc.max().width.is_finite() {
            Some(bc.max().width as f32 - 2. * LABEL_X_PADDING as f32)
        } else {
            None
        };
        self.text_layout.set_max_advance(max_advance);
        if self.text_layout.needs_rebuild() {
            let (font_ctx, layout_ctx) = ctx.text_contexts();
            self.text_layout.rebuild(font_ctx, layout_ctx);
        }
        // We ignore trailing whitespace for a label
        let text_size = self.text_layout.size();
        let label_size = Size {
            height: text_size.height,
            width: text_size.width + 2. * LABEL_X_PADDING,
        };
        let size = bc.constrain(label_size);
        trace!(
            "Computed layout: max={:?}. w={}, h={}",
            max_advance,
            size.width,
            size.height,
        );
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        if self.text_layout.needs_rebuild() {
            debug_panic!("Called Label paint before layout");
        }
        if self.line_break_mode == LineBreaking::Clip {
            let clip_rect = ctx.size().to_rect();
            scene.push_layer(BlendMode::default(), 1., Affine::IDENTITY, &clip_rect);
        }
        self.text_layout
            .draw(scene, Point::new(LABEL_X_PADDING, 0.0));

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

    fn skip_pointer(&self) -> bool {
        self.skip_pointer
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        SmallVec::new()
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("Label")
    }

    fn get_debug_text(&self) -> Option<String> {
        Some(self.text_layout.text().as_ref().to_string())
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
            .with_text_brush(PRIMARY_LIGHT)
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
                label.set_text("The quick brown fox jumps over the lazy dog");
                label.set_text_brush(PRIMARY_LIGHT);
                label.set_font_family(FontFamily::Generic(GenericFamily::Monospace));
                label.set_text_size(20.0);
                label.set_line_break_mode(LineBreaking::WordWrap);
                label.set_alignment(Alignment::Middle);
            });

            harness.render()
        };

        // We don't use assert_eq because we don't want rich assert
        assert!(image_1 == image_2);
    }
}
