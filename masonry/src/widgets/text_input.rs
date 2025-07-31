// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use accesskit::{Node, Role};
use tracing::{Span, trace_span};
use vello::Scene;
use vello::kurbo::{Affine, Point, Rect, Size};
use vello::peniko::Color;

use crate::core::{
    AccessCtx, ArcStr, BoxConstraints, ChildrenIds, LayoutCtx, NewWidget, NoAction, PaintCtx,
    Properties, PropertiesMut, PropertiesRef, RegisterCtx, Update, UpdateCtx, Widget, WidgetId,
    WidgetMut, WidgetPod,
};
use crate::properties::{
    Background, BorderColor, BorderWidth, BoxShadow, ContentColor, CornerRadius,
    DisabledBackground, Padding, PlaceholderColor,
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
    placeholder: WidgetPod<Label>,

    /// Whether to clip the contained text.
    clip: bool,
}

impl TextInput {
    /// Create a new `TextInput` with the given text.
    ///
    /// To use non-default text properties, use [`from_text_area`](Self::from_text_area) instead.
    pub fn new(text: &str) -> Self {
        Self::from_text_area(TextArea::new_editable(text).with_auto_id())
    }

    /// Create a new `TextInput` from a styled text area.
    pub fn from_text_area(text: NewWidget<TextArea<true>>) -> Self {
        Self {
            text: text.to_pod(),
            placeholder: NewWidget::new_with_props(Label::new(""), Properties::new()).to_pod(),
            clip: false,
        }
    }

    /// The text that will be displayed when this input is empty.
    ///
    /// To modify this on active text input, use [`set_placeholder`](Self::set_placeholder).
    pub fn with_placeholder(mut self, placeholder_text: impl Into<ArcStr>) -> Self {
        self.placeholder =
            NewWidget::new_with_props(Label::new(placeholder_text), Properties::new()).to_pod();
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

    /// Read the underlying text area.
    ///
    /// Useful for getting its ID, as most actions from the text input will be sent by the child.
    pub fn area_pod(&self) -> &WidgetPod<TextArea<true>> {
        &self.text
    }
}

// --- MARK: WIDGETMUT
impl TextInput {
    /// Edit the underlying text area.
    ///
    /// Used to modify most properties of the text.
    pub fn text_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, TextArea<true>> {
        this.ctx.get_mut(&mut this.widget.text)
    }

    /// Edit the child label representing the placeholder text.
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
}

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
        BorderWidth::prop_changed(ctx, property_type);
        CornerRadius::prop_changed(ctx, property_type);
        Padding::prop_changed(ctx, property_type);
        BoxShadow::prop_changed(ctx, property_type);

        // FIXME - Find more elegant way to propagate property to child.
        if property_type == TypeId::of::<PlaceholderColor>() {
            ctx.mutate_self_later(|mut input| {
                let mut input = input.downcast::<Self>();
                let color = input.get_prop::<PlaceholderColor>().color;
                let mut text_area = Self::placeholder_mut(&mut input);
                text_area.insert_prop(ContentColor::new(color));
            });
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::WidgetAdded => {
                // FIXME - Find more elegant way to propagate property to child.
                ctx.mutate_self_later(|mut input| {
                    let mut input = input.downcast::<Self>();
                    let color = input.get_prop::<PlaceholderColor>().color;
                    let mut text_area = Self::placeholder_mut(&mut input);
                    text_area.insert_prop(ContentColor::new(color));
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

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        let border = props.get::<BorderWidth>();
        let padding = props.get::<Padding>();
        let shadow = props.get::<BoxShadow>();

        let bc = *bc;
        let bc = border.layout_down(bc);
        let bc = padding.layout_down(bc);

        // TODO: Set minimum to deal with alignment
        let size = ctx.run_layout(&mut self.text, &bc);
        let baseline = ctx.child_baseline_offset(&self.text);

        let (size, baseline) = padding.layout_up(size, baseline);
        let (size, baseline) = border.layout_up(size, baseline);

        let pos = Point::ORIGIN;
        let pos = border.place_down(pos);
        let pos = padding.place_down(pos);
        ctx.place_child(&mut self.text, pos);

        let text_is_empty = {
            ctx.get_raw(&mut self.text)
                .0
                .text()
                .chars()
                .next()
                .is_none()
        };

        ctx.set_stashed(&mut self.placeholder, !text_is_empty);
        if text_is_empty {
            let _ = ctx.run_layout(&mut self.placeholder, &bc);
            ctx.place_child(&mut self.placeholder, pos);
        }

        if shadow.is_visible() {
            ctx.set_paint_insets(shadow.get_insets());
        }

        if self.clip {
            ctx.set_clip_path(Rect::from_origin_size(Point::ORIGIN, size));
        }

        ctx.set_baseline_offset(baseline);
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        let size = ctx.size();

        let border_width = props.get::<BorderWidth>();
        let border_radius = props.get::<CornerRadius>();
        let shadow = props.get::<BoxShadow>();
        let mut border_color = props.get::<BorderColor>();

        let bg = if ctx.is_disabled() {
            &props.get::<DisabledBackground>().0
        } else {
            props.get::<Background>()
        };

        let bg_rect = border_width.bg_rect(size, border_radius);
        let border_rect = border_width.border_rect(size, border_radius);

        // FIXME - Handle this properly
        if ctx.has_focus_target() {
            border_color = &BorderColor {
                color: Color::WHITE,
            };
        }

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
        _node: &mut Node,
    ) {
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

// TODO - Add more tests
#[cfg(test)]
mod tests {
    use vello::kurbo::Size;

    use super::*;
    use crate::core::StyleProperty;
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::default_property_set;
    use crate::widgets::TextArea;

    #[test]
    fn text_input_outline() {
        let text_input = NewWidget::new(TextInput::from_text_area(
            TextArea::new_editable("TextInput contents")
                .with_style(StyleProperty::FontSize(14.0))
                .with_auto_id(),
        ));
        let mut harness = TestHarness::create_with_size(
            default_property_set(),
            text_input,
            Size::new(150.0, 40.0),
        );

        assert_render_snapshot!(harness, "text_input_outline");

        let mut text_area_id = None;
        harness.edit_root_widget(|mut text_input| {
            let mut text_input = TextInput::text_mut(&mut text_input);
            text_area_id = Some(text_input.ctx.widget_id());

            TextArea::select_text(&mut text_input, "contents");
        });
        harness.focus_on(text_area_id);

        assert_render_snapshot!(harness, "text_input_selection");
    }
}
