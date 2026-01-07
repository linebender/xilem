// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use accesskit::{Node, Role};
use tracing::{Span, trace_span};
use vello::Scene;

use crate::TextAlign;
use crate::core::{
    AccessCtx, ArcStr, ChildrenIds, HasProperty, LayoutCtx, MeasureCtx, NewWidget, NoAction,
    PaintCtx, PropertiesMut, PropertiesRef, RegisterCtx, Update, UpdateCtx, Widget, WidgetId,
    WidgetMut, WidgetPod,
};
use crate::kurbo::{Affine, Axis, Point, Size};
use crate::layout::{LayoutSize, LenReq, SizeDef};
use crate::properties::{
    Background, BorderColor, BorderWidth, BoxShadow, CaretColor, ContentColor, CornerRadius,
    DisabledBackground, FocusedBorderColor, LineBreaking, Padding, PlaceholderColor,
    SelectionColor, UnfocusedSelectionColor,
};
use crate::util::{fill, stroke};
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

impl HasProperty<Background> for TextInput {}
impl HasProperty<CaretColor> for TextInput {}
impl HasProperty<DisabledBackground> for TextInput {}
impl HasProperty<BorderColor> for TextInput {}
impl HasProperty<FocusedBorderColor> for TextInput {}
impl HasProperty<BorderWidth> for TextInput {}
impl HasProperty<BoxShadow> for TextInput {}
impl HasProperty<CornerRadius> for TextInput {}
impl HasProperty<Padding> for TextInput {}
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
        DisabledBackground::prop_changed(ctx, property_type);
        Background::prop_changed(ctx, property_type);
        BorderColor::prop_changed(ctx, property_type);
        FocusedBorderColor::prop_changed(ctx, property_type);
        BorderWidth::prop_changed(ctx, property_type);
        CornerRadius::prop_changed(ctx, property_type);
        Padding::prop_changed(ctx, property_type);
        // TODO: Draw shadows in post_paint.
        BoxShadow::prop_changed(ctx, property_type);

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
                ctx.request_paint_only();
            }
            _ => {}
        }
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<f64>,
    ) -> f64 {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        let border = props.get::<BorderWidth>();
        let padding = props.get::<Padding>();

        let border_length = border.length(axis).dp(scale);
        let padding_length = padding.length(axis).dp(scale);

        match len_req {
            LenReq::MaxContent | LenReq::MinContent => {
                let cross = axis.cross();
                let cross_space = cross_length.map(|cross_length| {
                    let cross_border_length = border.length(cross).dp(scale);
                    let cross_padding_length = padding.length(cross).dp(scale);
                    (cross_length - cross_border_length - cross_padding_length).max(0.)
                });

                let auto_size = SizeDef::req(axis, len_req);
                let context_size = LayoutSize::maybe(cross, cross_space);

                let text_length =
                    ctx.compute_length(&mut self.text, auto_size, context_size, axis, cross_space);

                text_length + border_length + padding_length
            }
            // We always want to use all the offered space,
            // even on the block axis as we have multi-line display.
            LenReq::FitContent(space) => space.max(border_length + padding_length),
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, props: &PropertiesRef<'_>, size: Size) {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        let border = props.get::<BorderWidth>();
        let padding = props.get::<Padding>();
        let shadow = props.get::<BoxShadow>();

        let space = border.size_down(size, scale);
        let space = padding.size_down(space, scale);

        ctx.run_layout(&mut self.text, space);

        let child_origin = Point::ORIGIN;
        let child_origin = border.origin_down(child_origin, scale);
        let child_origin = padding.origin_down(child_origin, scale);

        ctx.place_child(&mut self.text, child_origin);

        let baseline = ctx.child_baseline_offset(&self.text);
        let baseline = border.baseline_up(baseline, scale);
        let baseline = padding.baseline_up(baseline, scale);
        ctx.set_baseline_offset(baseline);

        let text_is_empty = ctx.get_raw(&mut self.text).0.is_empty();
        ctx.set_stashed(&mut self.placeholder, !text_is_empty);
        if text_is_empty {
            ctx.run_layout(&mut self.placeholder, space);
            ctx.place_child(&mut self.placeholder, child_origin);
        }

        if shadow.is_visible() {
            ctx.set_paint_insets(shadow.get_insets());
        }

        if self.clip {
            // TODO: We actually want to clip space not size, but we can't here right now.
            //       Need either a set_clip_path_for_specific_child or TextArea clip support.
            ctx.set_clip_path(size.to_rect());
        } else {
            ctx.clear_clip_path();
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        let size = ctx.size();

        let border_width = props.get::<BorderWidth>();
        let border_radius = props.get::<CornerRadius>();
        let shadow = props.get::<BoxShadow>();

        let bg = if ctx.is_disabled() {
            &props.get::<DisabledBackground>().0
        } else {
            props.get::<Background>()
        };

        let bg_rect = border_width.bg_rect(size, border_radius);
        let border_rect = border_width.border_rect(size, border_radius);

        let border_color = if ctx.has_focus_target() {
            &props.get::<FocusedBorderColor>().0
        } else {
            props.get::<BorderColor>()
        };

        shadow.paint(scene, Affine::IDENTITY, bg_rect);

        let brush = bg.get_peniko_brush_for_rect(bg_rect.rect());
        fill(scene, &bg_rect, &brush);
        stroke(scene, &border_rect, border_color.color, border_width.width);
    }

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
