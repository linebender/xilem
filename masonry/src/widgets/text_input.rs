// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use accesskit::{Node, Role};
use tracing::{Span, trace_span};
use vello::Scene;

use crate::TextAlign;
use crate::core::{
    AccessCtx, ArcStr, ChildrenIds, HasProperty, LayoutCtx, MeasureCtx, NewWidget, NoAction,
    PaintCtx, PrePaintProps, PropertiesMut, PropertiesRef, RegisterCtx, Update, UpdateCtx, Widget,
    WidgetId, WidgetMut, WidgetPod, paint_background, paint_border, paint_box_shadow,
};
use crate::kurbo::{Axis, Point, Size};
use crate::layout::{LayoutSize, LenReq};
use crate::properties::{
    CaretColor, ContentColor, FocusedBorderColor, LineBreaking, PlaceholderColor, SelectionColor,
    UnfocusedSelectionColor,
};
use crate::widgets::{Label, TextArea};

/// The text input widget displays text which can be edited by the user,
/// inside a surrounding box.
///
/// This currently does not support newlines entered by the user,
/// although pre-existing newlines are handled correctly.
///
/// This widget itself does not emit any actions.
/// However, the child widget will do so, as it is user editable.
/// The ID of the child can be accessed using [`area_pod`](Self::area_pod).
///
/// At runtime, most properties of the text will be set using [`text_mut`](Self::text_mut).
/// This is because `TextInput` largely serves as a wrapper around a [`TextArea`].
pub struct TextInput {
    text: WidgetPod<TextArea<true>>,

    // TODO: We want placeholder to match wordwrap property of main text.
    // TODO: We want placeholder to clip even when wordwrap is enabled.
    placeholder: WidgetPod<Label>,
    placeholder_text: ArcStr,

    /// The text alignment for both the text area and placeholder.
    text_alignment: TextAlign,

    /// Whether to clip the contained text.
    clip: bool,
}

// --- MARK: BUILDERS
impl TextInput {
    /// Creates a new `TextInput` with the given text.
    ///
    /// To use non-default text properties, use [`from_text_area`](Self::from_text_area) instead.
    pub fn new(text: &str) -> Self {
        Self::from_text_area(TextArea::new_editable(text).with_auto_id())
    }

    /// Creates a new `TextInput` from a styled text area.
    pub fn from_text_area(text: NewWidget<TextArea<true>>) -> Self {
        Self {
            text: text.to_pod(),
            placeholder: Label::new("").with_props(LineBreaking::Clip).to_pod(),
            placeholder_text: "".into(),
            text_alignment: TextAlign::default(),
            clip: false,
        }
    }

    /// Sets the text alignment for both the input text and placeholder.
    pub fn with_text_alignment(mut self, text_alignment: TextAlign) -> Self {
        self.text_alignment = text_alignment;
        self
    }

    /// The text that will be displayed when this input is empty.
    ///
    /// To modify this on active text input, use [`set_placeholder`](Self::set_placeholder).
    pub fn with_placeholder(mut self, placeholder_text: impl Into<ArcStr>) -> Self {
        let placeholder_text = placeholder_text.into();
        let label = Label::new(placeholder_text.clone()).with_text_alignment(self.text_alignment);
        self.placeholder = label.with_props(LineBreaking::Clip).to_pod();
        self.placeholder_text = placeholder_text;
        self
    }

    /// Whether to clip the text to the drawn boundaries.
    ///
    /// If this is set to true, it is recommended, but not required, that this
    /// wraps a text area with [word wrapping](TextArea::with_word_wrap) enabled.
    ///
    /// To modify this on active text input, use [`set_clip`](Self::set_clip).
    pub fn with_clip(mut self, clip: bool) -> Self {
        self.clip = clip;
        self
    }
}

// --- MARK: METHODS
impl TextInput {
    /// Reads the underlying text area.
    ///
    /// Useful for getting its ID, as most actions from the text input will be sent by the child.
    pub fn area_pod(&self) -> &WidgetPod<TextArea<true>> {
        &self.text
    }
}

// --- MARK: WIDGETMUT
impl TextInput {
    /// Edits the underlying text area.
    ///
    /// Used to modify most properties of the text.
    pub fn text_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, TextArea<true>> {
        this.ctx.get_mut(&mut this.widget.text)
    }

    /// Edits the child label representing the placeholder text.
    pub fn placeholder_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, Label> {
        this.ctx.get_mut(&mut this.widget.placeholder)
    }

    /// The text that will be displayed when this input is empty.
    ///
    /// The runtime equivalent of [`with_placeholder`](Self::with_placeholder).
    pub fn set_placeholder(this: &mut WidgetMut<'_, Self>, placeholder_text: impl Into<ArcStr>) {
        Label::set_text(&mut Self::placeholder_mut(this), placeholder_text);
    }

    /// Whether to clip the text to the drawn boundaries.
    ///
    /// If this is set to true, it is recommended, but not required, that this
    /// wraps a text area with [word wrapping](TextArea::set_word_wrap) enabled.
    ///
    /// The runtime equivalent of [`with_clip`](Self::with_clip).
    pub fn set_clip(this: &mut WidgetMut<'_, Self>, clip: bool) {
        this.widget.clip = clip;
        this.ctx.request_layout();
    }

    /// Sets the text alignment for both the input text and placeholder.
    pub fn set_text_alignment(this: &mut WidgetMut<'_, Self>, text_alignment: TextAlign) {
        this.widget.text_alignment = text_alignment;
        TextArea::set_text_alignment(&mut Self::text_mut(this), text_alignment);
        Label::set_text_alignment(&mut Self::placeholder_mut(this), text_alignment);
    }
}

impl HasProperty<CaretColor> for TextInput {}
impl HasProperty<PlaceholderColor> for TextInput {}
impl HasProperty<SelectionColor> for TextInput {}
impl HasProperty<UnfocusedSelectionColor> for TextInput {}

// --- MARK: IMPL WIDGET
impl Widget for TextInput {
    type Action = NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.text);
        ctx.register_child(&mut self.placeholder);
    }

    fn property_changed(&mut self, ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        // FIXME - Find more elegant way to propagate property to child.
        if property_type == TypeId::of::<CaretColor>() {
            ctx.mutate_self_later(|mut input| {
                let mut input = input.downcast::<Self>();
                let color = *input.get_prop::<CaretColor>();
                let mut text_area = Self::text_mut(&mut input);
                text_area.insert_prop(color);
            });
        } else if property_type == TypeId::of::<SelectionColor>() {
            ctx.mutate_self_later(|mut input| {
                let mut input = input.downcast::<Self>();
                let color = *input.get_prop::<SelectionColor>();
                let mut text_area = Self::text_mut(&mut input);
                text_area.insert_prop(color);
            });
        } else if property_type == TypeId::of::<UnfocusedSelectionColor>() {
            ctx.mutate_self_later(|mut input| {
                let mut input = input.downcast::<Self>();
                let color = *input.get_prop::<UnfocusedSelectionColor>();
                let mut text_area = Self::text_mut(&mut input);
                text_area.insert_prop(color);
            });
        } else if property_type == TypeId::of::<PlaceholderColor>() {
            ctx.mutate_self_later(|mut input| {
                let mut input = input.downcast::<Self>();
                let color = input.get_prop::<PlaceholderColor>().color;
                let mut label = Self::placeholder_mut(&mut input);
                label.insert_prop(ContentColor::new(color));
            });
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::WidgetAdded => {
                // FIXME - Find more elegant way to propagate property to child.
                ctx.mutate_self_later(|mut input| {
                    let mut input = input.downcast::<Self>();
                    let color = *input.get_prop::<CaretColor>();
                    let mut text_area = Self::text_mut(&mut input);
                    text_area.insert_prop(color);
                });
                ctx.mutate_self_later(|mut input| {
                    let mut input = input.downcast::<Self>();
                    let color = *input.get_prop::<SelectionColor>();
                    let mut text_area = Self::text_mut(&mut input);
                    text_area.insert_prop(color);
                });
                ctx.mutate_self_later(|mut input| {
                    let mut input = input.downcast::<Self>();
                    let color = *input.get_prop::<UnfocusedSelectionColor>();
                    let mut text_area = Self::text_mut(&mut input);
                    text_area.insert_prop(color);
                });
                ctx.mutate_self_later(|mut input| {
                    let mut input = input.downcast::<Self>();
                    let color = input.get_prop::<PlaceholderColor>().color;
                    let mut label = Self::placeholder_mut(&mut input);
                    label.insert_prop(ContentColor::new(color));
                });
            }
            // We check for `ChildFocusChanged` instead of `FocusChanged`
            // because the actual widget that receives focus is the child `TextArea`
            Update::ChildFocusChanged(_) => {
                ctx.request_pre_paint();
            }
            _ => {}
        }
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<f64>,
    ) -> f64 {
        match len_req {
            LenReq::MaxContent | LenReq::MinContent => {
                let auto_length = len_req.into();
                let context_size = LayoutSize::maybe(axis.cross(), cross_length);

                ctx.compute_length(
                    &mut self.text,
                    auto_length,
                    context_size,
                    axis,
                    cross_length,
                )
            }
            // We always want to use all the offered space,
            // even on the block axis as we have multi-line display.
            LenReq::FitContent(space) => space,
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        ctx.run_layout(&mut self.text, size);

        let child_origin = Point::ORIGIN;
        ctx.place_child(&mut self.text, child_origin);

        let child_baseline = ctx.child_baseline_offset(&self.text);
        ctx.set_baseline_offset(child_baseline);

        let text_is_empty = ctx.get_raw(&mut self.text).0.is_empty();
        ctx.set_stashed(&mut self.placeholder, !text_is_empty);
        if text_is_empty {
            ctx.run_layout(&mut self.placeholder, size);
            ctx.place_child(&mut self.placeholder, child_origin);
        }

        if self.clip {
            ctx.set_clip_path(size.to_rect());
        } else {
            ctx.clear_clip_path();
        }
    }

    fn pre_paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        let bbox = ctx.border_box();
        let mut p = PrePaintProps::fetch(ctx, props);

        // We want to show a focus border if our child TextArea is focused
        if ctx.has_focus_target()
            && let Some(fb) = props.get_defined::<FocusedBorderColor>()
        {
            p.border_color = &fb.0;
        }

        paint_box_shadow(scene, bbox, p.box_shadow, p.corner_radius);
        paint_background(scene, bbox, p.background, p.border_width, p.corner_radius);
        paint_border(scene, bbox, p.border_color, p.border_width, p.corner_radius);
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _scene: &mut Scene) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        node.set_placeholder(self.placeholder_text.to_string());
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.text.id(), self.placeholder.id()])
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
    use masonry_testing::TestHarnessParams;

    use super::*;
    use crate::core::{StyleProperty, TextEvent};
    use crate::kurbo::Size;
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::test_property_set;
    use crate::widgets::TextArea;

    const HARNESS_PARAMS: TestHarnessParams = {
        let mut params = TestHarnessParams::DEFAULT;
        params.window_size = Size::new(150.0, 40.0);
        params.root_padding = 15;
        params
    };

    #[test]
    fn text_input_outline() {
        let text_input = NewWidget::new(TextInput::from_text_area(
            TextArea::new_editable("TextInput contents")
                .with_style(StyleProperty::FontSize(14.0))
                .with_auto_id(),
        ));
        let mut harness = TestHarness::create_with(test_property_set(), text_input, HARNESS_PARAMS);

        assert_render_snapshot!(harness, "text_input_outline");

        let mut text_area_id = None;
        harness.edit_root_widget(|mut text_input| {
            let mut text_input = TextInput::text_mut(&mut text_input);
            text_area_id = Some(text_input.ctx.widget_id());

            TextArea::select_text(&mut text_input, "contents");
        });
        harness.focus_on(text_area_id);

        assert_render_snapshot!(harness, "text_input_selection");

        harness.process_text_event(TextEvent::WindowFocusChange(false));

        assert_render_snapshot!(harness, "text_input_selection_unfocused");

        harness.process_text_event(TextEvent::WindowFocusChange(true));
        harness.animate_ms(500 + 1);

        assert_render_snapshot!(harness, "text_input_cursor_blink");
    }

    #[test]
    fn placeholder() {
        let text_input = NewWidget::new(
            TextInput::from_text_area(
                TextArea::new_editable("")
                    .with_style(StyleProperty::FontSize(14.0))
                    .with_auto_id(),
            )
            .with_placeholder("HELLO WORLD"),
        );

        let mut harness = TestHarness::create_with(test_property_set(), text_input, HARNESS_PARAMS);

        assert_render_snapshot!(harness, "text_input_placeholder");
    }

    #[test]
    fn text_input_clips() {
        let text_input = NewWidget::new(
            TextInput::from_text_area(
                TextArea::new_editable("TextInput contents which should be clipped")
                    .with_style(StyleProperty::FontSize(14.0))
                    .with_word_wrap(false)
                    .with_auto_id(),
            )
            .with_clip(true),
        );
        let mut harness = TestHarness::create_with(test_property_set(), text_input, HARNESS_PARAMS);

        assert_render_snapshot!(harness, "text_input_clip");
    }
}
