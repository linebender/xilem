// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use accesskit::{Node, Role, Toggled};
use include_doc_path::include_doc_path;
use masonry_core::debug_panic;
use tracing::{Span, trace, trace_span};
use vello::Scene;

use crate::core::{
    AccessCtx, AccessEvent, ArcStr, ChildrenIds, EventCtx, HasProperty, LayoutCtx, NewWidget,
    PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, TextEvent, Update,
    UpdateCtx, Widget, WidgetId, WidgetMut, WidgetPod, keyboard::Key, paint_background,
    paint_box_shadow,
};
use crate::core::{MeasureCtx, PrePaintProps};
use crate::kurbo::Circle;
use crate::kurbo::{Affine, Axis, Cap, Dashes, Join, Point, Size, Stroke};
use crate::layout::{LayoutSize, LenReq, SizeDef};
use crate::properties::FocusedBorderColor;
use crate::properties::{
    BorderColor, BorderWidth, CheckmarkColor, CheckmarkStrokeWidth, DisabledCheckmarkColor,
    HoveredBorderColor,
};
use crate::theme;
use crate::util::{fill, stroke};
use crate::widgets::{Label, RadioGroup};

/// A radio button that can be toggled.
///
#[doc = concat!(
    "![Radio button with checked state](",
    include_doc_path!("screenshots/radio_button_hello_checked.png"),
    ")",
)]
///
/// Emits [`RadioButtonSelected`] when selected.
pub struct RadioButton {
    selected: bool,
    // FIXME - Remove label child, have this widget only be a box with a checkmark.
    label: WidgetPod<Label>,
    parent_group: Option<WidgetId>,
}

impl RadioButton {
    /// Create a new `RadioButton` with a text label.
    pub fn new(checked: bool, text: impl Into<ArcStr>) -> Self {
        Self {
            selected: checked,
            label: WidgetPod::new(Label::new(text)),
            parent_group: None,
        }
    }

    /// Create a new `RadioButton` with the given label.
    pub fn from_label(checked: bool, label: NewWidget<Label>) -> Self {
        Self {
            selected: checked,
            label: label.to_pod(),
            parent_group: None,
        }
    }
}

// --- MARK: WIDGETMUT
impl RadioButton {
    /// Check or uncheck the box.
    pub fn set_checked(this: &mut WidgetMut<'_, Self>, checked: bool) {
        this.widget.selected = checked;
        // Checked state impacts appearance and accessibility node
        this.ctx.request_render();

        let Some(parent_id) = this.widget.parent_group else {
            return;
        };
        let self_id = this.ctx.widget_id();
        this.ctx.mutate_widget_later(parent_id, move |mut group| {
            let group = group.downcast::<RadioGroup>();
            Self::update_group(group, self_id, checked);
        });
    }

    /// Set the text.
    ///
    /// We enforce this to be an `ArcStr` to make the allocation explicit.
    pub fn set_text(this: &mut WidgetMut<'_, Self>, new_text: ArcStr) {
        Label::set_text(&mut Self::label_mut(this), new_text);
    }

    /// Get a mutable reference to the label.
    pub fn label_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, Label> {
        this.ctx.get_mut(&mut this.widget.label)
    }
}

impl HasProperty<DisabledCheckmarkColor> for RadioButton {}
impl HasProperty<CheckmarkColor> for RadioButton {}

/// The action type emitted by [`RadioButton`] when it is selected.
///
/// There is no equivalent action for deselection.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct RadioButtonSelected;

// --- MARK: HELPERS
impl RadioButton {
    fn select(&mut self, ctx: &mut EventCtx<'_>) {
        self.selected = true;
        ctx.submit_action::<RadioButtonSelected>(RadioButtonSelected);
        ctx.request_render();

        let Some(parent_id) = self.parent_group else {
            return;
        };
        let self_id = ctx.widget_id();
        ctx.mutate_widget_later(parent_id, move |mut group| {
            let group = group.downcast::<RadioGroup>();
            Self::update_group(group, self_id, true);
        });
    }

    fn update_group(mut group: WidgetMut<'_, RadioGroup>, self_id: WidgetId, selected: bool) {
        let selected_button = group.widget.selected_button;
        if let Some(button_id) = selected_button
            && button_id != self_id
            && selected
        {
            // We don't bother checking that the button still exists,
            // the mutate pass does that for us.
            group.ctx.mutate_widget_later(button_id, move |mut group| {
                let mut button = group.downcast::<Self>();
                button.widget.selected = false;
                button.ctx.request_render();
            });
            // TODO - Submit action from RadioGroup?
        }

        if let Some(button_id) = selected_button
            && button_id == self_id
            && !selected
        {
            group.widget.selected_button = None;
        }
        if selected {
            group.widget.selected_button = Some(self_id);
        }
    }
}

// --- MARK: IMPL WIDGET
impl Widget for RadioButton {
    type Action = RadioButtonSelected;

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        match event {
            PointerEvent::Down { .. } => {
                ctx.capture_pointer();
                trace!("RadioButton {:?} pressed", ctx.widget_id());
            }
            PointerEvent::Up { .. } => {
                if ctx.is_active() && ctx.is_hovered() {
                    trace!("RadioButton {:?} released", ctx.widget_id());
                    self.select(ctx);
                }
            }
            _ => (),
        }
    }

    fn on_text_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &TextEvent,
    ) {
        match event {
            TextEvent::Keyboard(event) if event.state.is_up() => {
                if matches!(&event.key, Key::Character(c) if c == " ") {
                    self.select(ctx);
                }
            }
            _ => (),
        }
    }

    fn accepts_focus(&self) -> bool {
        // RadioButton can be tab-focused...
        true
    }

    fn on_access_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &AccessEvent,
    ) {
        match event.action {
            accesskit::Action::Click => {
                self.select(ctx);
            }
            _ => {}
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::WidgetAdded => {
                let Some((_, id)) = ctx.nearest_ancestor::<RadioGroup>() else {
                    let id = ctx.widget_id();
                    debug_panic!("RadioButton {id} is not a child of a RadioGroup");
                    return;
                };

                self.parent_group = Some(id);
            }
            Update::HoveredChanged(_)
            | Update::ActiveChanged(_)
            | Update::FocusChanged(_)
            | Update::DisabledChanged(_) => {
                ctx.request_paint_only();
            }

            _ => {}
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.label);
    }

    fn property_changed(&mut self, ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        CheckmarkStrokeWidth::prop_changed(ctx, property_type);
        DisabledCheckmarkColor::prop_changed(ctx, property_type);
        CheckmarkColor::prop_changed(ctx, property_type);
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<f64>,
    ) -> f64 {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        let check_side = theme::BASIC_WIDGET_HEIGHT.dp(scale);
        let check_padding = theme::WIDGET_CONTROL_COMPONENT_PADDING.dp(scale);

        let calc_other_length = |axis| match axis {
            Axis::Horizontal => check_side + check_padding,
            Axis::Vertical => 0.,
        };
        let other_length = calc_other_length(axis);

        let cross = axis.cross();
        let cross_space = cross_length.map(|cross_length| {
            let cross_other_length = calc_other_length(cross);
            (cross_length - cross_other_length).max(0.)
        });

        let auto_length = len_req.reduce(other_length).into();
        let context_size = LayoutSize::maybe(cross, cross_space);

        let label_length = ctx.compute_length(
            &mut self.label,
            auto_length,
            context_size,
            axis,
            cross_space,
        );

        match axis {
            Axis::Horizontal => label_length + other_length,
            Axis::Vertical => label_length.max(check_side) + other_length,
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        let check_side = theme::BASIC_WIDGET_HEIGHT.dp(scale);
        let check_padding = theme::WIDGET_CONTROL_COMPONENT_PADDING.dp(scale);

        let space = Size::new(
            (size.width - (check_side + check_padding)).max(0.),
            size.height,
        );

        let label_size = ctx.compute_size(&mut self.label, SizeDef::fit(space), space.into());
        ctx.run_layout(&mut self.label, label_size);

        let label_origin = Point::new(check_side + check_padding, 0.);
        ctx.place_child(&mut self.label, label_origin);

        let label_baseline = ctx.child_baseline_offset(&self.label);
        let label_bottom = label_origin.y + label_size.height;
        let bottom_gap = size.height - label_bottom;
        ctx.set_baseline_offset(label_baseline + bottom_gap);
    }

    fn pre_paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        let bbox = ctx.border_box();
        let p = PrePaintProps::fetch(ctx, props);

        paint_box_shadow(scene, bbox, p.box_shadow, p.corner_radius);
        paint_background(scene, bbox, p.background, p.border_width, p.corner_radius);

        // Paint focus indicator around the entire widget (box + label)
        if ctx.is_focus_target() || ctx.is_hovered() {
            // TODO: Replace this custom implementation with the general paint_border()

            let focus_rect = bbox.inflate(2.0, 2.0);

            let focus_color = p.border_color.color;
            let focus_width = 2.0;
            let focus_radius = 4.0;

            let focus_stroke = Stroke {
                width: focus_width,
                join: Join::Round,
                miter_limit: 10.0,
                start_cap: Cap::Round,
                end_cap: Cap::Round,
                dash_pattern: Dashes::default(),
                dash_offset: 0.0,
            };
            let focus_path = focus_rect.to_rounded_rect(focus_radius);
            scene.stroke(
                &focus_stroke,
                Affine::IDENTITY,
                focus_color,
                None,
                &focus_path,
            );
        }
        // Skip painting the regular border while the check border uses that property
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        let is_focused = ctx.is_focus_target();
        let is_hovered = ctx.is_hovered();

        let check_side = theme::BASIC_WIDGET_HEIGHT.dp(scale);
        let check_size = Size::new(check_side, check_side);

        let border_width = props.get::<BorderWidth>();

        let border_circle = Circle::new(
            check_size.to_rect().center(),
            (check_side - border_width.width) * 0.5,
        );

        let border_color = if is_focused {
            &props.get::<FocusedBorderColor>().0
        } else if is_hovered {
            &props.get::<HoveredBorderColor>().0
        } else {
            props.get::<BorderColor>()
        };

        // Paint the radio button border
        stroke(
            scene,
            &border_circle,
            border_color.color,
            border_width.width,
        );

        // Paint the radio button box background and border
        stroke(
            scene,
            &border_circle,
            border_color.color,
            border_width.width,
        );

        // Paint the checkmark if checked
        if self.selected {
            let brush = if ctx.is_disabled() {
                &props.get::<DisabledCheckmarkColor>().0
            } else {
                props.get::<CheckmarkColor>()
            };

            // TODO: Create a prop for ellipse size. Default: 50% of border size
            let check_circle = Circle::new(check_size.to_rect().center(), (check_side) * 0.25);
            fill(scene, &check_circle, brush.color);
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::RadioButton
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        node.add_action(accesskit::Action::Click);
        if self.selected {
            node.set_toggled(Toggled::True);
        } else {
            node.set_toggled(Toggled::False);
        }
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.label.id()])
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("RadioButton", id = id.trace())
    }

    fn get_debug_text(&self) -> Option<String> {
        if self.selected {
            Some("(X)".to_string())
        } else {
            Some("( )".to_string())
        }
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Properties, StyleProperty};
    use crate::properties::ContentColor;
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::{ACCENT_COLOR, default_property_set};
    use crate::widgets::Flex;

    #[test]
    fn simple_radio_button() {
        let widget = NewWidget::new(RadioButton::new(false, "Hello"));
        let widget = NewWidget::new(RadioGroup::new(widget));

        let window_size = Size::new(100.0, 40.0);
        let mut harness =
            TestHarness::create_with_size(default_property_set(), widget, window_size);
        let radio_id = harness.root_id();

        assert_render_snapshot!(harness, "radio_button_hello_unchecked");

        assert!(harness.pop_action_erased().is_none());

        harness.mouse_click_on(radio_id);
        assert_eq!(
            harness.pop_action::<RadioButtonSelected>(),
            Some((RadioButtonSelected, radio_id))
        );

        assert_render_snapshot!(harness, "radio_button_hello_hovered");

        harness.edit_root_widget(|mut group| {
            let mut radio = RadioGroup::child_mut(&mut group);
            let mut radio = radio.downcast();
            RadioButton::set_checked(&mut radio, true);
        });

        assert_render_snapshot!(harness, "radio_button_hello_checked");

        harness.focus_on(None);
        harness.press_tab_key(false);
        assert_eq!(harness.focused_widget().map(|w| w.id()), Some(radio_id));
    }

    #[test]
    fn radio_button_focus_indicator() {
        use crate::properties::types::MainAxisAlignment;

        let radio = NewWidget::new(RadioButton::new(true, "Focus test"));
        let radio_id = radio.id();
        let group = NewWidget::new(RadioGroup::new(radio));

        let root = NewWidget::new(
            Flex::row()
                .with_fixed(group)
                .main_axis_alignment(MainAxisAlignment::Center),
        );
        let mut harness =
            TestHarness::create_with_size(default_property_set(), root, Size::new(120.0, 40.0));

        harness.focus_on(Some(radio_id));
        assert_render_snapshot!(harness, "radio_button_focus_focused");
    }

    #[test]
    fn edit_radio_button() {
        let image_1 = {
            let label = Label::new("The quick brown fox jumps over the lazy dog")
                .with_style(StyleProperty::FontSize(20.0));
            let label = NewWidget::new_with_props(
                label,
                Properties::new().with(ContentColor::new(ACCENT_COLOR)),
            );
            let radio = NewWidget::new(RadioButton::from_label(true, label));
            let group = NewWidget::new(RadioGroup::new(radio));

            let mut harness =
                TestHarness::create_with_size(default_property_set(), group, Size::new(50.0, 50.0));

            harness.render()
        };

        let image_2 = {
            let radio = NewWidget::new(RadioButton::new(false, "Hello world"));
            let group = NewWidget::new(RadioGroup::new(radio));

            let mut harness =
                TestHarness::create_with_size(default_property_set(), group, Size::new(50.0, 50.0));

            harness.edit_root_widget(|mut group| {
                let mut radio = RadioGroup::child_mut(&mut group);
                let mut radio = radio.downcast();
                RadioButton::set_checked(&mut radio, true);
                RadioButton::set_text(
                    &mut radio,
                    ArcStr::from("The quick brown fox jumps over the lazy dog"),
                );

                let mut label = RadioButton::label_mut(&mut radio);
                label.insert_prop(ContentColor::new(ACCENT_COLOR));
                Label::insert_style(&mut label, StyleProperty::FontSize(20.0));
            });

            harness.render()
        };

        // We don't use assert_eq because we don't want rich assert
        assert!(image_1 == image_2);
    }
}
