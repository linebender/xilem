// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{Node, Role};
use tracing::{Span, trace_span};
use vello::Scene;
use vello::kurbo::{Point, Size};

use crate::core::{
    AccessCtx, AccessEvent, BoxConstraints, ChildrenIds, EventCtx, LayoutCtx, PaintCtx,
    PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, TextEvent, Update, UpdateCtx, Widget,
    WidgetId, WidgetMut, WidgetPod,
};
use crate::util::include_screenshot;
use crate::widgets::TextArea;

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
///
#[doc = include_screenshot!("prose_alignment_flex.png", "Multiple lines with different alignments.")]
pub struct Prose {
    text: WidgetPod<TextArea<false>>,

    /// Whether to clip the contained text.
    clip: bool,
}

impl Prose {
    /// Create a new `Prose` with the given text.
    ///
    /// To use non-default text properties, use [`from_text_area`](Self::from_text_area) instead.
    pub fn new(text: &str) -> Self {
        Self::from_text_area(TextArea::new_immutable(text))
    }

    /// Create a new `Prose` from a styled text area.
    pub fn from_text_area(text: TextArea<false>) -> Self {
        Self {
            text: WidgetPod::new(text),
            clip: false,
        }
    }

    /// Create a new `Prose` from a styled text area in a [`WidgetPod`].
    pub fn from_text_area_pod(text: WidgetPod<TextArea<false>>) -> Self {
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

    /// Read the underlying text area. Useful for getting its ID.
    // This is a bit of a hack, to work around `from_text_area_pod` not being
    // able to set padding.
    pub fn text_area_pod(&self) -> &WidgetPod<TextArea<false>> {
        &self.text
    }
}

// --- MARK: WIDGETMUT
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
    /// The runtime equivalent of [`with_clip`](Self::with_clip).
    pub fn set_clip(this: &mut WidgetMut<'_, Self>, clip: bool) {
        this.widget.clip = clip;
        this.ctx.request_layout();
    }
}

// --- MARK: IMPL WIDGET
impl Widget for Prose {
    fn on_pointer_event(
        &mut self,
        _: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _: &PointerEvent,
    ) {
    }

    fn on_text_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &TextEvent,
    ) {
    }

    fn on_access_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &AccessEvent,
    ) {
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.text);
    }

    fn update(
        &mut self,
        _ctx: &mut UpdateCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &Update,
    ) {
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        // TODO: Set minimum to deal with alignment
        let size = ctx.run_layout(&mut self.text, bc);
        ctx.place_child(&mut self.text, Point::ORIGIN);
        if self.clip {
            ctx.set_clip_path(size.to_rect());
        } else {
            ctx.clear_clip_path();
        }
        size
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _scene: &mut Scene) {
        // All painting is handled by the child
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.text.id()])
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("Prose", id = id.trace())
    }

    fn get_debug_text(&self) -> Option<String> {
        self.clip.then(|| "(clip)".into())
    }
}

// TODO - Add more tests
#[cfg(test)]
mod tests {
    use parley::StyleProperty;
    use vello::kurbo::Size;

    use super::*;
    use crate::TextAlign;
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::default_property_set;
    use crate::widgets::{CrossAxisAlignment, Flex, SizedBox, TextArea};

    #[test]
    /// A wrapping prose's text alignment should be respected, regardless of
    /// its parent's text alignment.
    fn prose_clipping() {
        let prose = Prose::from_text_area(
            TextArea::new_immutable("Truncated text - you should not see this")
                .with_style(StyleProperty::FontSize(14.0))
                .with_word_wrap(false),
        )
        .with_clip(true);

        let root_widget = Flex::row().with_child(SizedBox::new(prose).width(60.));

        let mut harness = TestHarness::create_with_size(
            default_property_set(),
            root_widget,
            Size::new(200.0, 40.0),
        );

        assert_render_snapshot!(harness, "prose_clipping");
    }

    #[test]
    /// A wrapping prose's alignment should be respected, regardless of
    /// its parent's alignment.
    fn prose_alignment_flex() {
        fn base_prose(text_alignment: TextAlign) -> Prose {
            // Trailing whitespace is displayed when laying out prose.
            Prose::from_text_area(
                TextArea::new_immutable("Hello  ")
                    .with_style(StyleProperty::FontSize(14.0))
                    .with_text_alignment(text_alignment)
                    .with_word_wrap(true),
            )
        }
        let prose1 = base_prose(TextAlign::Start);
        let prose2 = base_prose(TextAlign::Center);
        let prose3 = base_prose(TextAlign::End);
        let prose4 = base_prose(TextAlign::Start);
        let prose5 = base_prose(TextAlign::Center);
        let prose6 = base_prose(TextAlign::End);
        let flex = Flex::column()
            .with_flex_child(prose1, CrossAxisAlignment::Start)
            .with_flex_child(prose2, CrossAxisAlignment::Start)
            .with_flex_child(prose3, CrossAxisAlignment::Start)
            .with_flex_child(prose4, CrossAxisAlignment::Center)
            .with_flex_child(prose5, CrossAxisAlignment::Center)
            .with_flex_child(prose6, CrossAxisAlignment::Center)
            .with_gap(0.0);

        let mut harness =
            TestHarness::create_with_size(default_property_set(), flex, Size::new(200.0, 120.0));

        assert_render_snapshot!(harness, "prose_alignment_flex");
    }
}
