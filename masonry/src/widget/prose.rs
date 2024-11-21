// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{Node, Role};
use smallvec::{smallvec, SmallVec};
use tracing::{trace_span, Span};
use vello::kurbo::{Point, Rect, Size};
use vello::Scene;

use crate::widget::WidgetMut;
use crate::{
    AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, PaintCtx, PointerEvent, QueryCtx,
    RegisterCtx, TextEvent, Update, UpdateCtx, Widget, WidgetId,
};

use super::{Padding, TextArea, WidgetPod};

/// Added padding between each horizontal edge of the widget
/// and the text in logical pixels.
///
/// This gives the text the some slight breathing room.
// The bottom padding is to workaround https://github.com/linebender/parley/issues/165
const PROSE_PADDING: Padding = Padding::new(0.0, 2.0, 5.0, 2.0);

/// The prose widget displays immutable text which can be
/// selected within.
///
/// The text can also be copied from, but cannot be modified by the user.
/// Note that copying is not yet implemented.
///
/// At runtime, most properties of the text will be set using [`text_mut`](Self::text_mut).
/// This is because `Prose` largely serves as a wrapper around a [`TextArea`].
///
/// This should be used instead of [`Label`](super::Label) for immutable text,
/// as it enables users to copy/paste from the text.
///
/// This widget has no actions.
pub struct Prose {
    text: WidgetPod<TextArea<false>>,

    /// Whether to clip the contained text.
    clip: bool,
}

impl Prose {
    /// Create a new `Prose` with the given text.
    ///
    /// To use non-default text properties, use [`from_text_region`](Self::from_text_region) instead.
    pub fn new(text: &str) -> Self {
        Self::from_text_region(TextArea::new_immutable(text))
    }

    /// Create a new `Prose` from a styled text area.
    pub fn from_text_region(text: TextArea<false>) -> Self {
        let text = text.with_padding_if_default(PROSE_PADDING);
        Self {
            text: WidgetPod::new(text),
            clip: false,
        }
    }

    /// Create a new `Prose` from a styled text area in a [`WidgetPod`].
    ///
    /// Note that the default padding used for prose will not be applied.
    pub fn from_text_region_pod(text: WidgetPod<TextArea<false>>) -> Self {
        Self { text, clip: false }
    }

    /// Whether to clip the text to the available space.
    ///
    /// If this is set to true, it is recommended, but not required, that this
    /// wraps a text area with [word wrapping](TextArea::with_word_wrap) enabled.
    ///
    /// To modify this on active prose, use [`set_clip`](Self::set_clip).
    pub fn with_clip(mut self, clip: bool) -> Self {
        self.clip = clip;
        self
    }

    /// Read the underlying text region. Useful for getting its ID.
    // This is a bit of a hack, to work around `from_text_region_pod` not being
    // able to set padding.
    pub fn region_pod(&self) -> &WidgetPod<TextArea<false>> {
        &self.text
    }
}

// --- MARK: WIDGETMUT ---
impl Prose {
    /// Edit the underlying text area.
    ///
    /// Used to modify most properties of the text.
    pub fn text_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, TextArea<false>> {
        this.ctx.get_mut(&mut this.widget.text)
    }

    /// Whether to clip the text to the available space.
    ///
    /// If this is set to true, it is recommended, but not required, that this
    /// wraps a text area with [word wrapping](TextArea::set_word_wrap) enabled.
    ///
    /// The runtime requivalent of [`with_clip`](Self::with_clip).
    pub fn set_clip(this: &mut WidgetMut<'_, Self>, clip: bool) {
        this.widget.clip = clip;
        this.ctx.request_layout();
    }
}

// --- MARK: IMPL WIDGET ---
impl Widget for Prose {
    fn on_pointer_event(&mut self, _: &mut EventCtx, _: &PointerEvent) {}

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}

    fn on_access_event(&mut self, _ctx: &mut EventCtx, _event: &AccessEvent) {}

    fn register_children(&mut self, ctx: &mut RegisterCtx) {
        ctx.register_child(&mut self.text);
    }

    fn update(&mut self, _ctx: &mut UpdateCtx, _event: &Update) {}

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        // TODO: Set minimum to deal with alignment
        let size = ctx.run_layout(&mut self.text, bc);
        ctx.place_child(&mut self.text, Point::ORIGIN);
        if self.clip {
            ctx.set_clip_path(Rect::from_origin_size(Point::ORIGIN, size));
        }
        size
    }

    fn paint(&mut self, _ctx: &mut PaintCtx, _scene: &mut Scene) {
        // All painting is handled by the child
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx, _node: &mut Node) {}

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        smallvec![self.text.id()]
    }

    fn make_trace_span(&self, ctx: &QueryCtx<'_>) -> Span {
        trace_span!("Prose", id = ctx.widget_id().trace())
    }

    fn get_debug_text(&self) -> Option<String> {
        self.clip.then(|| "(clip)".into())
    }
}

// TODO - Add more tests
#[cfg(test)]
mod tests {
    use parley::{layout::Alignment, StyleProperty};
    use vello::kurbo::Size;

    use crate::{
        assert_render_snapshot,
        testing::TestHarness,
        widget::{CrossAxisAlignment, Flex, Prose, TextArea},
    };

    #[test]
    /// A wrapping prose's alignment should be respected, regardless of
    /// its parent's alignment.
    fn prose_alignment_flex() {
        fn base_prose(alignment: Alignment) -> Prose {
            // Trailing whitespace is displayed when laying out prose.
            Prose::from_text_region(
                TextArea::new_immutable("Hello  ")
                    .with_style(StyleProperty::FontSize(10.0))
                    .with_alignment(alignment)
                    .with_word_wrap(true),
            )
        }
        let label1 = base_prose(Alignment::Start);
        let label2 = base_prose(Alignment::Middle);
        let label3 = base_prose(Alignment::End);
        let label4 = base_prose(Alignment::Start);
        let label5 = base_prose(Alignment::Middle);
        let label6 = base_prose(Alignment::End);
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
