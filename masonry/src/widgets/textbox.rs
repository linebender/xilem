// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use accesskit::{Node, Role};
use smallvec::{SmallVec, smallvec};
use tracing::{Span, trace_span};
use vello::Scene;
use vello::kurbo::{Affine, Point, Rect, Size};

use crate::core::{
    AccessCtx, BoxConstraints, LayoutCtx, PaintCtx, PropertiesMut, PropertiesRef, RegisterCtx,
    Update, UpdateCtx, Widget, WidgetId, WidgetMut, WidgetPod,
};
use crate::properties::{
    Background, BorderColor, BorderWidth, BoxShadow, CornerRadius, DisabledBackground, Padding,
};
use crate::util::{fill, stroke};
use crate::widgets::TextArea;

/// The textbox widget displays text which can be edited by the user,
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
/// This is because `Textbox` largely serves as a wrapper around a [`TextArea`].
pub struct Textbox {
    text: WidgetPod<TextArea<true>>,

    /// Whether to clip the contained text.
    clip: bool,
}

impl Textbox {
    /// Create a new `Textbox` with the given text.
    ///
    /// To use non-default text properties, use [`from_text_area`](Self::from_text_area) instead.
    pub fn new(text: &str) -> Self {
        Self::from_text_area(TextArea::new_editable(text))
    }

    /// Create a new `Textbox` from a styled text area.
    pub fn from_text_area(text: TextArea<true>) -> Self {
        Self {
            text: WidgetPod::new(text),
            clip: false,
        }
    }

    /// Create a new `Textbox` from a styled text area in a [`WidgetPod`].
    ///
    /// Note that the default padding used for textbox will not apply.
    pub fn from_text_area_pod(text: WidgetPod<TextArea<true>>) -> Self {
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

    /// Read the underlying text area.
    ///
    /// Useful for getting its ID, as most actions from the textbox will be sent by the child.
    pub fn area_pod(&self) -> &WidgetPod<TextArea<true>> {
        &self.text
    }
}

// --- MARK: WIDGETMUT
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

// --- MARK: IMPL WIDGET
impl Widget for Textbox {
    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.text);
    }

    fn property_changed(&mut self, ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        DisabledBackground::prop_changed(ctx, property_type);
        Background::prop_changed(ctx, property_type);
        BorderColor::prop_changed(ctx, property_type);
        BorderWidth::prop_changed(ctx, property_type);
        CornerRadius::prop_changed(ctx, property_type);
        Padding::prop_changed(ctx, property_type);
        BoxShadow::prop_changed(ctx, property_type);
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
        let border_color = props.get::<BorderColor>();

        let bg = if ctx.is_disabled() {
            &props.get::<DisabledBackground>().0
        } else {
            props.get::<Background>()
        };

        let bg_rect = border_width.bg_rect(size, border_radius);
        let border_rect = border_width.border_rect(size, border_radius);

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

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        smallvec![self.text.id()]
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
    use crate::assert_render_snapshot;
    use crate::core::StyleProperty;
    use crate::testing::TestHarness;
    use crate::theme::default_property_set;
    use crate::widgets::TextArea;

    #[test]
    fn textbox_outline() {
        let textbox = Textbox::from_text_area(
            TextArea::new_editable("Textbox contents").with_style(StyleProperty::FontSize(14.0)),
        );
        let mut harness =
            TestHarness::create_with_size(default_property_set(), textbox, Size::new(150.0, 40.0));

        assert_render_snapshot!(harness, "textbox_outline");

        let mut text_area_id = None;
        harness.edit_root_widget(|mut textbox| {
            let mut textbox = textbox.downcast::<Textbox>();
            let mut textbox = Textbox::text_mut(&mut textbox);
            text_area_id = Some(textbox.ctx.widget_id());

            TextArea::select_text(&mut textbox, "contents");
        });
        harness.focus_on(text_area_id);

        assert_render_snapshot!(harness, "textbox_selection");
    }
}
