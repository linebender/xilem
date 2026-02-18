// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{Node, Role};
use include_doc_path::include_doc_path;
use tracing::{Span, trace_span};
use vello::Scene;

use crate::core::{
    AccessCtx, AccessEvent, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, NewWidget, NoAction,
    PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, TextEvent, Update,
    UpdateCtx, Widget, WidgetId, WidgetMut, WidgetPod,
};
use crate::kurbo::{Axis, Point, Size};
use crate::layout::LenReq;
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
#[doc = concat!(
    "![Multiple lines with different alignments](",
    include_doc_path!("screenshots/prose_alignment_flex.png"),
    ")",
)]
pub struct Prose {
    text: WidgetPod<TextArea<false>>,

    /// Whether to clip the contained text.
    clip: bool,
}

// --- MARK: BUILDERS
impl Prose {
    /// Creates a new `Prose` with the given text.
    ///
    /// To use non-default text properties, use [`from_text_area`](Self::from_text_area) instead.
    pub fn new(text: &str) -> Self {
        Self::from_text_area(TextArea::new_immutable(text).with_auto_id())
    }

    /// Creates a new `Prose` from a styled text area.
    pub fn from_text_area(text: NewWidget<TextArea<false>>) -> Self {
        Self {
            text: text.to_pod(),
            clip: false,
        }
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
}

// --- MARK: METHODS
impl Prose {
    /// Returns the `WidgetPod` of the underlying text area. Useful for getting its ID.
    // This is a bit of a hack, to work around `from_text_area_pod` not being
    // able to set padding.
    pub fn text_area_pod(&self) -> &WidgetPod<TextArea<false>> {
        &self.text
    }
}

// --- MARK: WIDGETMUT
impl Prose {
    /// Returns the underlying text area.
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
    type Action = NoAction;

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

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        _len_req: LenReq,
        cross_length: Option<f64>,
    ) -> f64 {
        ctx.redirect_measurement(&mut self.text, axis, cross_length)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        ctx.run_layout(&mut self.text, size);
        ctx.place_child(&mut self.text, Point::ORIGIN);

        if self.clip {
            let border_box = size.to_rect() + ctx.border_box_insets();
            ctx.set_clip_path(border_box);
        } else {
            ctx.clear_clip_path();
        }
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

// --- MARK: TESTS
// TODO - Add more tests
#[cfg(test)]
mod tests {
    use parley::StyleProperty;

    use super::*;
    use crate::TextAlign;
    use crate::core::PropertySet;
    use crate::kurbo::Size;
    use crate::layout::AsUnit;
    use crate::properties::Gap;
    use crate::properties::types::CrossAxisAlignment;
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::test_property_set;
    use crate::widgets::{Flex, SizedBox, TextArea};

    #[test]
    /// A wrapping prose's text alignment should be respected, regardless of
    /// its parent's text alignment.
    fn prose_clipping() {
        let prose = Prose::from_text_area(
            TextArea::new_immutable("Truncated text - you should not see this")
                .with_style(StyleProperty::FontSize(14.0))
                .with_word_wrap(false)
                .with_auto_id(),
        )
        .with_clip(true)
        .with_auto_id();

        let root_widget = Flex::row()
            .with_fixed(SizedBox::new(prose).width(60.px()).with_auto_id())
            .with_auto_id();

        let mut harness =
            TestHarness::create_with_size(test_property_set(), root_widget, Size::new(200.0, 40.0));

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
                    .with_word_wrap(true)
                    .with_auto_id(),
            )
        }
        let prose1 = base_prose(TextAlign::Start);
        let prose2 = base_prose(TextAlign::Center);
        let prose3 = base_prose(TextAlign::End);
        let prose4 = base_prose(TextAlign::Start);
        let prose5 = base_prose(TextAlign::Center);
        let prose6 = base_prose(TextAlign::End);
        let flex = Flex::column()
            .with(prose1.with_auto_id(), CrossAxisAlignment::Start)
            .with(prose2.with_auto_id(), CrossAxisAlignment::Start)
            .with(prose3.with_auto_id(), CrossAxisAlignment::Start)
            .with(prose4.with_auto_id(), CrossAxisAlignment::Center)
            .with(prose5.with_auto_id(), CrossAxisAlignment::Center)
            .with(prose6.with_auto_id(), CrossAxisAlignment::Center);
        let flex = NewWidget::new_with_props(flex, PropertySet::one(Gap::ZERO));

        let mut harness =
            TestHarness::create_with_size(test_property_set(), flex, Size::new(200.0, 120.0));

        assert_render_snapshot!(harness, "prose_alignment_flex");
    }
}
