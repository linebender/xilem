use kurbo::{Affine, Point, Size};
use parley::{
    layout::Alignment,
    style::{FontFamily, FontStack},
};
use smallvec::SmallVec;
use tracing::trace;
use vello::{
    peniko::{BlendMode, Brush, Color},
    Scene,
};

use crate::{
    text2::{Selectable, TextWithSelection},
    widget::label::LABEL_X_PADDING,
    BoxConstraints, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, PointerEvent,
    StatusChange, TextEvent, Widget,
};

use super::{LineBreaking, WidgetMut, WidgetRef};

/// The prose widget is a widget which displays text which can be
/// selected with keyboard and mouse, and which can be copied from,
/// but cannot be modified by the user.
///
/// This should be preferred over [`Label`](super::Label) for most
/// immutable text, other than that within
pub struct Prose<T: Selectable> {
    text_layout: TextWithSelection<T>,
    line_break_mode: LineBreaking,
    show_disabled: bool,
}

impl<T: Selectable> Prose<T> {
    pub fn new(text: T) -> Self {
        Prose {
            text_layout: TextWithSelection::new(text, crate::theme::TEXT_SIZE_NORMAL as f32),
            line_break_mode: LineBreaking::WordWrap,
            show_disabled: true,
        }
    }

    // TODO: Can we reduce code duplication with `Label` widget somehow?
    pub fn text(&self) -> &T {
        self.text_layout.text()
    }

    pub fn with_text_color(mut self, color: Color) -> Self {
        self.text_layout.set_color(color);
        self
    }

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

impl<T: Selectable> WidgetMut<'_, Prose<T>> {
    pub fn text(&self) -> &T {
        self.widget.text_layout.text()
    }

    pub fn set_text_properties<R>(&mut self, f: impl FnOnce(&mut TextWithSelection<T>) -> R) -> R {
        let ret = f(&mut self.widget.text_layout);
        if self.widget.text_layout.needs_rebuild() {
            self.ctx.request_layout();
        }
        ret
    }

    pub fn set_text(&mut self, new_text: T) {
        self.set_text_properties(|layout| layout.set_text(new_text));
    }

    pub fn set_text_color(&mut self, color: Color) {
        self.set_text_properties(|layout| layout.set_color(color));
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
        self.set_font(FontStack::Single(family))
    }
    pub fn set_line_break_mode(&mut self, line_break_mode: LineBreaking) {
        self.widget.line_break_mode = line_break_mode;
        self.ctx.request_paint();
    }
}

impl<T: Selectable> Widget for Prose<T> {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        match event {
            PointerEvent::PointerMove(_point) => {
                if !ctx.is_disabled() {
                    // TODO: Set cursor if over link
                    // TODO: Move selection if selecting
                }
            }
            PointerEvent::PointerDown(_button, _state) => {
                // TODO: Start tracking currently pressed link
                if !ctx.is_disabled() {
                    ctx.set_active(true);
                }
            }
            PointerEvent::PointerUp(_button, _state) => {
                // TODO: Follow link (if not now dragging ?)
                if !ctx.is_disabled() {}
                ctx.set_active(false);
            }
            PointerEvent::PointerLeave(_state) => {}
            _ => {}
        }
    }

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {
        // If focused on a link and enter pressed, follow it?
        // TODO: This sure looks like each link needs its own widget, although I guess the challenge there is
        // that the bounding boxes can go e.g. across line boundaries?
    }

    #[allow(missing_docs)]
    fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, event: &StatusChange) {
        match event {
            StatusChange::FocusChanged(false) => {
                // TODO: Release focus
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
                        self.text_layout.set_override_brush(Some(Brush::Solid(
                            crate::theme::DISABLED_TEXT_COLOR,
                        )))
                    } else {
                        self.text_layout.set_override_brush(None)
                    }
                }
                // TODO: Parley seems to require a relayout when colours change
                ctx.request_layout();
            }
            LifeCycle::BuildFocusChain => {
                if !self.text_layout.text().links().is_empty() {
                    tracing::warn!("Links present in text, but not yet integrated")
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
            Some(bc.max().width as f32 - 2. * LABEL_X_PADDING as f32)
        } else if bc.min().width.is_sign_negative() {
            Some(0.0)
        } else {
            None
        };
        self.text_layout.set_max_advance(max_advance);
        if self.text_layout.needs_rebuild() {
            self.text_layout.rebuild(ctx.font_ctx());
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

    fn children(&self) -> SmallVec<[WidgetRef<'_, dyn Widget>; 16]> {
        SmallVec::new()
    }

    fn get_debug_text(&self) -> Option<String> {
        Some(self.text_layout.text().as_str().to_string())
    }
}
