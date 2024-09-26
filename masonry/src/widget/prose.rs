// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{NodeBuilder, Role};
use parley::{
    layout::Alignment,
    style::{FontFamily, FontStack},
};
use smallvec::SmallVec;
use tracing::{trace, trace_span, Span};
use vello::{
    kurbo::{Affine, Point, Size},
    peniko::BlendMode,
    Scene,
};

use crate::widget::{LineBreaking, WidgetMut};
use crate::{
    text::{TextBrush, TextWithSelection},
    widget::label::LABEL_X_PADDING,
    AccessCtx, AccessEvent, ArcStr, BoxConstraints, CursorIcon, EventCtx, LayoutCtx, LifeCycle,
    LifeCycleCtx, PaintCtx, PointerEvent, RegisterCtx, StatusChange, TextEvent, Widget, WidgetId,
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
impl WidgetMut<'_, Prose> {
    pub fn text(&self) -> &ArcStr {
        self.widget.text_layout.text()
    }

    pub fn set_text_properties<R>(
        &mut self,
        f: impl FnOnce(&mut TextWithSelection<ArcStr>) -> R,
    ) -> R {
        let ret = f(&mut self.widget.text_layout);
        if self.widget.text_layout.needs_rebuild() {
            self.ctx.request_layout();
        }
        ret
    }

    /// Change the text. If the user currently has a selection in the box, this will delete that selection.
    ///
    /// We enforce this to be an `ArcStr` to make the allocation explicit.
    pub fn set_text(&mut self, new_text: ArcStr) {
        if self.ctx.is_focused() {
            tracing::info!(
                "Called reset_text on a focused `Prose`. This will lose the user's current selection"
            );
        }
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
                if !ctx.is_disabled() {
                    // TODO: Set cursor if over link
                    ctx.set_cursor(&CursorIcon::Text);
                    if ctx.has_pointer_capture()
                        && self.text_layout.pointer_move(inner_origin, state)
                    {
                        // We might have changed text colours, so we need to re-request a layout
                        ctx.request_layout();
                    }
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

    fn on_access_event(&mut self, _ctx: &mut EventCtx, _event: &AccessEvent) {
        // TODO - Handle accesskit::Action::SetTextSelection
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx) {}

    #[allow(missing_docs)]
    fn on_status_change(&mut self, ctx: &mut LifeCycleCtx, event: &StatusChange) {
        match event {
            StatusChange::FocusChanged(false) => {
                self.text_layout.focus_lost();
                ctx.request_layout();
                // TODO: Stop focusing on any links
            }
            StatusChange::FocusChanged(true) => {
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
            LifeCycle::BuildFocusChain => {
                // When we add links to `Prose`, they will probably need to be handled here.
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
        Role::Paragraph
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx, node: &mut NodeBuilder) {
        node.set_name(self.text().as_ref().to_string());
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
