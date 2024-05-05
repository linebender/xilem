// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::Role;
use kurbo::{Affine, Point, Size, Stroke};
use parley::{
    layout::Alignment,
    style::{FontFamily, FontStack},
};
use smallvec::SmallVec;
use tracing::trace;
use vello::{
    peniko::{BlendMode, Color},
    Scene,
};

use crate::{
    text2::{EditableText, TextBrush, TextEditor, TextWithSelection},
    AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    PointerEvent, StatusChange, TextEvent, Widget,
};

use super::{LineBreaking, WidgetMut, WidgetRef};

const TEXTBOX_PADDING: f64 = 4.0;

/// The textbox widget is a widget which shows text which can be edited by the user
///
/// For immutable text [`Prose`](super::Prose) should be preferred
pub struct Textbox<T: EditableText> {
    editor: TextEditor<T>,
    line_break_mode: LineBreaking,
    show_disabled: bool,
    brush: TextBrush,
}

impl<T: EditableText> Textbox<T> {
    pub fn new(text: T) -> Self {
        Textbox {
            editor: TextEditor::new(text, crate::theme::TEXT_SIZE_NORMAL as f32),
            line_break_mode: LineBreaking::WordWrap,
            show_disabled: true,
            brush: crate::theme::TEXT_COLOR.into(),
        }
    }

    // TODO: Can we reduce code duplication with `Label` widget somehow?
    pub fn text(&self) -> &T {
        self.editor.text()
    }

    #[doc(alias = "with_text_color")]
    pub fn with_text_brush(mut self, brush: impl Into<TextBrush>) -> Self {
        self.brush = brush.into();
        self.editor.set_brush(self.brush.clone());
        self
    }

    pub fn with_text_size(mut self, size: f32) -> Self {
        self.editor.set_text_size(size);
        self
    }

    pub fn with_text_alignment(mut self, alignment: Alignment) -> Self {
        self.editor.set_text_alignment(alignment);
        self
    }

    pub fn with_font(mut self, font: FontStack<'static>) -> Self {
        self.editor.set_font(font);
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

impl<T: EditableText> WidgetMut<'_, Textbox<T>> {
    pub fn text(&self) -> &T {
        self.widget.editor.text()
    }

    pub fn set_text_properties<R>(&mut self, f: impl FnOnce(&mut TextWithSelection<T>) -> R) -> R {
        let ret = f(&mut self.widget.editor);
        if self.widget.editor.needs_rebuild() {
            self.ctx.request_layout();
        }
        ret
    }

    pub fn set_text(&mut self, new_text: T) {
        // FIXME - Right now doing this resets the caret to the start of the text
        // It's not clear whether this is the right behaviour, or if there even
        // is one.
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

impl<T: EditableText> Widget for Textbox<T> {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        let window_origin = ctx.widget_state.window_origin();
        let inner_origin = Point::new(
            window_origin.x + TEXTBOX_PADDING,
            window_origin.y + TEXTBOX_PADDING,
        );
        match event {
            PointerEvent::PointerDown(button, state) => {
                if !ctx.is_disabled() {
                    // TODO: Start tracking currently pressed link?
                    let made_change = self.editor.pointer_down(inner_origin, state, *button);
                    if made_change {
                        ctx.request_layout();
                        ctx.request_paint();
                        ctx.request_focus();
                        ctx.set_active(true);
                    }
                }
            }
            PointerEvent::PointerMove(state) => {
                if !ctx.is_disabled() {
                    // TODO: Set cursor if over link
                    ctx.set_cursor(&winit::window::CursorIcon::Text);
                    if ctx.is_active() && self.editor.pointer_move(inner_origin, state) {
                        // We might have changed text colours, so we need to re-request a layout
                        ctx.request_layout();
                        ctx.request_paint();
                    }
                }
            }
            PointerEvent::PointerUp(button, state) => {
                // TODO: Follow link (if not now dragging ?)
                if !ctx.is_disabled() && ctx.is_active() {
                    self.editor.pointer_up(inner_origin, state, *button);
                }
                ctx.set_active(false);
            }
            PointerEvent::PointerLeave(_state) => {
                ctx.set_active(false);
            }
            _ => {}
        }
    }

    fn on_text_event(&mut self, ctx: &mut EventCtx, event: &TextEvent) {
        let result = self.editor.text_event(ctx, event);
        // If focused on a link and enter pressed, follow it?
        if result.is_handled() {
            ctx.set_handled();
            // TODO: only some handlers need this repaint
            ctx.request_layout();
            ctx.request_paint();
        }
    }

    fn on_access_event(&mut self, _ctx: &mut EventCtx, _event: &AccessEvent) {
        // TODO - Handle accesskit::Action::SetTextSelection
        // TODO - Handle accesskit::Action::ReplaceSelectedText
        // TODO - Handle accesskit::Action::SetValue
    }

    #[allow(missing_docs)]
    fn on_status_change(&mut self, ctx: &mut LifeCycleCtx, event: &StatusChange) {
        match event {
            StatusChange::FocusChanged(false) => {
                self.editor.focus_lost();
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
                        self.editor.set_brush(crate::theme::DISABLED_TEXT_COLOR);
                    } else {
                        self.editor.set_brush(self.brush.clone());
                    }
                }
                // TODO: Parley seems to require a relayout when colours change
                ctx.request_layout();
            }
            LifeCycle::BuildFocusChain => {
                if !self.editor.text().links().is_empty() {
                    tracing::warn!("Links present in text, but not yet integrated");
                }
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
            Some(bc.max().width as f32 - 2. * TEXTBOX_PADDING as f32)
        } else if bc.min().width.is_sign_negative() {
            Some(0.0)
        } else {
            None
        };
        self.editor.set_max_advance(max_advance);
        if self.editor.needs_rebuild() {
            self.editor.rebuild(ctx.font_ctx());
        }
        // We ignore trailing whitespace for a label
        let text_size = self.editor.size();
        let label_size = Size {
            height: text_size.height + 2. * TEXTBOX_PADDING,
            // TODO: Better heuristic here?
            width: bc.max().width - 20.,
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
        if self.editor.needs_rebuild() {
            debug_panic!("Called Label paint before layout");
        }
        if self.line_break_mode == LineBreaking::Clip {
            let clip_rect = ctx.size().to_rect();
            scene.push_layer(BlendMode::default(), 1., Affine::IDENTITY, &clip_rect);
        }

        self.editor
            .draw(scene, Point::new(TEXTBOX_PADDING, TEXTBOX_PADDING));

        let outline_rect = ctx.size().to_rect().inset(1.0);
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            Color::WHITE,
            None,
            &outline_rect,
        );
        if self.line_break_mode == LineBreaking::Clip {
            scene.pop_layer();
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::TextInput
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx) {
        // TODO
    }

    fn children(&self) -> SmallVec<[WidgetRef<'_, dyn Widget>; 16]> {
        SmallVec::new()
    }

    fn get_debug_text(&self) -> Option<String> {
        Some(self.editor.text().as_str().chars().take(100).collect())
    }
}
