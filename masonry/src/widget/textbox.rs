// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{Node, Role};
use smallvec::{smallvec, SmallVec};
use tracing::{trace_span, Span};
use vello::kurbo::{Affine, Insets, Point, Rect, Size, Stroke};
use vello::peniko::Color;
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
/// This makes it so that the surrounding box isn't crowding out the text.
const TEXTBOX_PADDING: Padding = Padding::all(5.0);

/// The margin added around textboxes to allow the boundaries to be visible inside the window edge.
const TEXTBOX_MARGIN: Padding = Padding::horizontal(2.0);

/// The textbox widget displays text which can be edited by the user,
/// inside a surrounding box.
///
/// This currently does not support newlines entered by the user,
/// although pre-existing newlines are handled correctly.
///
/// This widget itself does not emit any actions.
/// However, the child widget will do so, as it is user editable.
/// The ID of the child can be accessed using [`region_pod`](Self::region_pod).
///
/// At runtime, most properties of the text will be set using [`text_mut`](Self::text_mut).
/// This is because `Textbox` largely serves as a wrapper around a [`TextArea`].
pub struct Textbox {
    text: WidgetPod<TextArea<true>>,

    /// Whether to clip the contained text.
    clip: bool,
}

impl Textbox {
    /// Create a new `Prose` with the given text.
    ///
    /// To use non-default text properties, use [`from_text_region`](Self::from_text_region) instead.
    pub fn new(text: &str) -> Self {
        Self::from_text_region(TextArea::new_editable(text))
    }

    /// Create a new `Prose` from a styled text area.
    pub fn from_text_region(text: TextArea<true>) -> Self {
        let text = text.with_padding_if_default(TEXTBOX_PADDING);
        Self {
            text: WidgetPod::new(text),
            clip: false,
        }
    }

    /// Create a new `Prose` from a styled text area in a [`WidgetPod`].
    ///
    /// Note that the default padding used for prose will not apply.
    pub fn from_text_region_pod(text: WidgetPod<TextArea<true>>) -> Self {
        Self { text, clip: false }
    }

    /// Whether to clip the text to the drawn boundaries.
    ///
    /// If this is set to true, it is recommended, but not required, that this
    /// wraps a text area with [word wrapping](TextArea::with_word_wrap) enabled.
    ///
    /// To modify this on active textbox, use [`set_clip`](Self::set_clip).
    pub fn with_clip(mut self, clip: bool) -> Self {
        self.clip = clip;
        self
    }

    /// Read the underlying text region.
    ///
    /// Useful for getting its ID, as most actions from the textbox will be sent by the child.
    pub fn region_pod(&self) -> &WidgetPod<TextArea<true>> {
        &self.text
    }
}

// --- MARK: WIDGETMUT ---
impl Textbox {
    /// Edit the underlying text area.
    ///
    /// Used to modify most properties of the text.
    pub fn text_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, TextArea<true>> {
        this.ctx.get_mut(&mut this.widget.text)
    }

    /// Whether to clip the text to the drawn boundaries.
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
impl Widget for Textbox {
    fn on_pointer_event(&mut self, _: &mut EventCtx, _: &PointerEvent) {}

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}

    fn on_access_event(&mut self, _ctx: &mut EventCtx, _event: &AccessEvent) {}

    fn register_children(&mut self, ctx: &mut RegisterCtx) {
        ctx.register_child(&mut self.text);
    }

    fn update(&mut self, _ctx: &mut UpdateCtx, _event: &Update) {}

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        let margin = TEXTBOX_MARGIN;
        // Shrink constraints by padding inset
        let margin_size = Size::new(margin.leading + margin.trailing, margin.top + margin.bottom);
        let child_bc = bc.shrink(margin_size);
        // TODO: Set minimum to deal with alignment
        let size = ctx.run_layout(&mut self.text, &child_bc);
        // TODO: How do we handle RTL here?
        ctx.place_child(&mut self.text, Point::new(margin.leading, margin.top));
        if self.clip {
            ctx.set_clip_path(Rect::from_origin_size(Point::ORIGIN, size));
        }
        size + margin_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        let size = ctx.size();
        let outline_rect = size.to_rect().inset(Insets::new(
            -TEXTBOX_MARGIN.leading,
            -TEXTBOX_MARGIN.top,
            -TEXTBOX_MARGIN.trailing,
            -TEXTBOX_MARGIN.bottom,
        ));
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            Color::WHITE,
            None,
            &outline_rect,
        );
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
