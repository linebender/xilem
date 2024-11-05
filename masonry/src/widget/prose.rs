// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{NodeBuilder, Role};
use parley::layout::Alignment;
use parley::style::{FontFamily, FontStack};
use smallvec::SmallVec;
use tracing::{trace_span, Span};
use vello::kurbo::{Affine, Point, Size};
use vello::peniko::BlendMode;
use vello::Scene;

use crate::text::{ArcStr, TextBrush, TextWithSelection};
use crate::widget::label::LABEL_X_PADDING;
use crate::widget::{LineBreaking, WidgetMut};
use crate::{
    AccessCtx, AccessEvent, BoxConstraints, CursorIcon, EventCtx, LayoutCtx, PaintCtx,
    PointerEvent, QueryCtx, RegisterCtx, TextEvent, Update, UpdateCtx, Widget, WidgetId,
};

/// The prose widget is a widget which displays text which can be
/// selected with keyboard and mouse, and which can be copied from,
/// but cannot be modified by the user.
///
/// This should be preferred over [`Label`](super::Label) for most
/// immutable text, other than that within
pub struct Prose {
    // See `Label` for discussion of the choice of text type
    text_layout: TextWithSelection<ArcStr>,
    line_break_mode: LineBreaking,
    show_disabled: bool,
    brush: TextBrush,
}

// --- MARK: BUILDERS ---
impl Prose {
    pub fn new(text: impl Into<ArcStr>) -> Self {
        Prose {
            text_layout: TextWithSelection::new(text.into(), crate::theme::TEXT_SIZE_NORMAL as f32),
            line_break_mode: LineBreaking::WordWrap,
            show_disabled: true,
            brush: crate::theme::TEXT_COLOR.into(),
        }
    }

    // TODO: Can we reduce code duplication with `Label` widget somehow?
    pub fn text(&self) -> &ArcStr {
        self.text_layout.text()
    }

    #[doc(alias = "with_text_color")]
    pub fn with_text_brush(mut self, brush: impl Into<TextBrush>) -> Self {
        self.brush = brush.into();
        self.text_layout.set_brush(self.brush.clone());
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
}

// --- MARK: WIDGETMUT ---
impl Prose {
    pub fn set_text_properties<R>(
        this: &mut WidgetMut<'_, Self>,
        f: impl FnOnce(&mut TextWithSelection<ArcStr>) -> R,
    ) -> R {
        let ret = f(&mut this.widget.text_layout);
        if this.widget.text_layout.needs_rebuild() {
            this.ctx.request_layout();
        }
        ret
    }

    /// Change the text. If the user currently has a selection in the box, this will delete that selection.
    ///
    /// We enforce this to be an `ArcStr` to make the allocation explicit.
    pub fn set_text(this: &mut WidgetMut<'_, Self>, new_text: ArcStr) {
        if this.ctx.is_focused() {
            tracing::info!(
                "Called reset_text on a focused `Prose`. This will lose the user's current selection"
            );
        }
        Self::set_text_properties(this, |layout| layout.set_text(new_text));
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
impl Widget for Prose {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        let window_origin = ctx.widget_state.window_origin();
        let inner_origin = Point::new(window_origin.x + LABEL_X_PADDING, window_origin.y);
        match event {
            PointerEvent::PointerDown(button, state) => {
                if !ctx.is_disabled() {
                    // TODO: Start tracking currently pressed link?
                    let made_change = self.text_layout.pointer_down(inner_origin, state, *button);
                    if made_change {
                        ctx.request_layout();
                        ctx.request_focus();
                        ctx.capture_pointer();
                    }
                }
            }
            PointerEvent::PointerMove(state) => {
                if !ctx.is_disabled()
                    && ctx.has_pointer_capture()
                    && self.text_layout.pointer_move(inner_origin, state)
                {
                    // We might have changed text colours, so we need to re-request a layout
                    ctx.request_layout();
                }
            }
            PointerEvent::PointerUp(button, state) => {
                // TODO: Follow link (if not now dragging ?)
                if !ctx.is_disabled() && ctx.has_pointer_capture() {
                    self.text_layout.pointer_up(inner_origin, state, *button);
                }
            }
            _ => {}
        }
    }

    fn on_text_event(&mut self, ctx: &mut EventCtx, event: &TextEvent) {
        // If focused on a link and enter pressed, follow it?
        let result = self.text_layout.text_event(event);
        if result.is_handled() {
            ctx.set_handled();
            // TODO: only some handlers need this repaint
            ctx.request_layout();
        }
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx, event: &AccessEvent) {
        match event.action {
            accesskit::Action::SetTextSelection => {
                if self.text_layout.set_selection_from_access_event(event) {
                    ctx.request_layout();
                }
            }
            _ => (),
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx) {}

    fn update(&mut self, ctx: &mut UpdateCtx, event: &Update) {
        match event {
            Update::FocusChanged(false) => {
                self.text_layout.focus_lost();
                ctx.request_layout();
                // TODO: Stop focusing on any links
            }
            Update::FocusChanged(true) => {
                // TODO: Focus on first link
            }
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
            // TODO: Does Prose have different needs here?
            Some(bc.max().width as f32 - 2. * LABEL_X_PADDING as f32)
        } else if bc.min().width.is_sign_negative() {
            Some(0.0)
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
            .draw(scene, Point::new(LABEL_X_PADDING, 0.0));

        if self.line_break_mode == LineBreaking::Clip {
            scene.pop_layer();
        }
    }

    fn get_cursor(&self, _ctx: &QueryCtx, _pos: Point) -> CursorIcon {
        // TODO: Set cursor if over link
        CursorIcon::Text
    }

    fn accessibility_role(&self) -> Role {
        Role::Document
    }

    fn accessibility(&mut self, ctx: &mut AccessCtx, node: &mut NodeBuilder) {
        node.set_read_only();
        self.text_layout.accessibility(ctx.tree_update, node);
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        SmallVec::new()
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("Prose")
    }

    fn get_debug_text(&self) -> Option<String> {
        Some(self.text_layout.text().as_ref().chars().take(100).collect())
    }
}

// TODO - Add more tests
#[cfg(test)]
mod tests {
    use parley::layout::Alignment;
    use vello::kurbo::Size;

    use crate::{
        assert_render_snapshot,
        testing::TestHarness,
        widget::{CrossAxisAlignment, Flex, LineBreaking, Prose},
    };

    #[test]
    /// A wrapping prose's alignment should be respected, regardkess of
    /// its parent's alignment.
    fn prose_alignment_flex() {
        fn base_label() -> Prose {
            // Trailing whitespace is displayed when laying out prose.
            Prose::new("Hello    ")
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

        assert_render_snapshot!(harness, "prose_alignment_flex");
    }
}
