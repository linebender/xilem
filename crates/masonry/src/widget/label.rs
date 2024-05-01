// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use kurbo::{Affine, Point, Size};
use parley::layout::Alignment;
use smallvec::SmallVec;
use tracing::trace;
use vello::{
    peniko::{BlendMode, Brush, Color},
    Scene,
};

use crate::{
    declare_widget,
    text2::{layout::TextLayout, TextStorage},
    widget::WidgetRef,
    ArcStr, BoxConstraints, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, PointerEvent,
    StatusChange, TextEvent, Widget,
};

use super::WidgetMut;

// added padding between the edges of the widget and the text.
const LABEL_X_PADDING: f64 = 2.0;

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
pub struct Label<T: TextStorage = ArcStr> {
    text_layout: TextLayout<T>,
    line_break_mode: LineBreaking,
    allow_disabled: bool,
    // disabled: bool,
}

declare_widget!(LabelMut, Label<T: (TextStorage)>);

impl<T: TextStorage> Label<T> {
    /// Create a new label.
    pub fn new(text: T) -> Self {
        Self {
            text_layout: TextLayout::new(text, crate::theme::TEXT_SIZE_NORMAL as f32),
            line_break_mode: LineBreaking::Overflow,
            allow_disabled: true,
        }
    }

    pub fn with_text_color(mut self, color: Color) -> Self {
        self.set_color(color);
        self
    }

    pub fn with_text_size(mut self, size: f32) -> Self {
        self.set_text_size(size);
        self
    }

    pub fn with_text_alignment(mut self, alignment: Alignment) -> Self {
        self.set_text_alignment(alignment);
        self
    }
}

impl Label<ArcStr> {
    /// Create a label with empty text.
    pub fn empty() -> Self {
        Self::new("".into())
    }
}

// TODO: Is this the right API for this?
// Mostly this just shortcuts adding helper methods for all of the items
impl<T: TextStorage> Deref for Label<T> {
    type Target = TextLayout<T>;
    fn deref(&self) -> &Self::Target {
        &self.text_layout
    }
}

impl<T: TextStorage> DerefMut for Label<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.text_layout
    }
}

impl<T: TextStorage> LabelMut<'_, T> {
    pub fn text(&self) -> &T {
        self.text_layout.text()
    }

    pub fn set_text_properties<R>(&mut self, f: impl FnOnce(&mut TextLayout<T>) -> R) -> R {
        let ret = f(&mut self.widget.text_layout);
        if self.widget.text_layout.needs_rebuild() {
            self.ctx.request_layout();
        }
        ret
    }

    pub fn set_text(&mut self, new_text: T) {
        self.set_text_properties(|ctx| ctx.set_text(new_text));
    }

    pub fn set_color(&mut self, color: Color) {
        self.set_text_properties(|ctx| ctx.set_color(color));
    }
    pub fn set_text_size(&mut self, size: f32) {
        self.set_text_properties(|ctx| ctx.set_text_size(size));
    }
    pub fn set_text_alignment(&mut self, alignment: Alignment) {
        self.set_text_properties(|ctx| ctx.set_text_alignment(alignment));
    }
}

impl<T: TextStorage> Widget for Label<T> {
    fn on_pointer_event(&mut self, _ctx: &mut EventCtx, event: &PointerEvent) {
        match event {
            PointerEvent::PointerMove(_point) => {
                // TODO: Set cursor if over link
            }
            PointerEvent::PointerDown(_button, _state) => {
                // TODO: Start tracking currently pressed link
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
                if self.allow_disabled {
                    if *disabled {
                        self.text_layout.set_overrid_brush(Some(Brush::Solid(
                            crate::theme::DISABLED_TEXT_COLOR,
                        )))
                    } else {
                        self.text_layout.set_overrid_brush(None)
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
