// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A checkbox widget.

use accesskit::{Node, Role, Toggled};
use smallvec::{SmallVec, smallvec};
use tracing::{Span, trace, trace_span};
use vello::Scene;
use vello::kurbo::{Affine, BezPath, Cap, Join, Size, Stroke};

use crate::core::{
    AccessCtx, AccessEvent, Action, ArcStr, BoxConstraints, EventCtx, LayoutCtx, PaintCtx,
    PointerEvent, QueryCtx, RegisterCtx, TextEvent, Update, UpdateCtx, Widget, WidgetId, WidgetMut,
    WidgetPod,
};
use crate::theme;
use crate::util::{UnitPoint, fill_lin_gradient, stroke};
use crate::widgets::Label;

/// A checkbox that can be toggled.
///
#[doc = crate::include_screenshot!("widget/screenshots/masonry__widget__checkbox__tests__hello_checked.png", "Checkbox with checked state.")]
pub struct Checkbox {
    checked: bool,
    label: WidgetPod<Label>,
}

impl Checkbox {
    /// Create a new `Checkbox` with a text label.
    pub fn new(checked: bool, text: impl Into<ArcStr>) -> Self {
        Self {
            checked,
            label: WidgetPod::new(Label::new(text)),
        }
    }

    /// Create a new `Checkbox` with the given label.
    pub fn from_label(checked: bool, label: Label) -> Self {
        Self {
            checked,
            label: WidgetPod::new(label),
        }
    }
}

// --- MARK: WIDGETMUT ---
impl Checkbox {
    pub fn set_checked(this: &mut WidgetMut<'_, Self>, checked: bool) {
        this.widget.checked = checked;
        // Checked state impacts appearance and accessibility node
        this.ctx.request_render();
    }

    /// Set the text.
    ///
    /// We enforce this to be an `ArcStr` to make the allocation explicit.
    pub fn set_text(this: &mut WidgetMut<'_, Self>, new_text: ArcStr) {
        Label::set_text(&mut Self::label_mut(this), new_text);
    }

    pub fn label_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, Label> {
        this.ctx.get_mut(&mut this.widget.label)
    }
}

// --- MARK: IMPL WIDGET ---
impl Widget for Checkbox {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        match event {
            PointerEvent::PointerDown(_, _) => {
                if !ctx.is_disabled() {
                    ctx.capture_pointer();
                    // Checked state impacts appearance and accessibility node
                    ctx.request_render();
                    trace!("Checkbox {:?} pressed", ctx.widget_id());
                }
            }
            PointerEvent::PointerUp(_, _) => {
                if ctx.is_pointer_capture_target() && ctx.is_hovered() && !ctx.is_disabled() {
                    self.checked = !self.checked;
                    ctx.submit_action(Action::CheckboxToggled(self.checked));
                    trace!("Checkbox {:?} released", ctx.widget_id());
                }
                // Checked state impacts appearance and accessibility node
                ctx.request_render();
            }
            _ => (),
        }
    }

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}

    fn on_access_event(&mut self, ctx: &mut EventCtx, event: &AccessEvent) {
        if ctx.target() == ctx.widget_id() {
            match event.action {
                accesskit::Action::Click => {
                    self.checked = !self.checked;
                    ctx.submit_action(Action::CheckboxToggled(self.checked));
                    // Checked state impacts appearance and accessibility node
                    ctx.request_render();
                }
                _ => {}
            }
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, event: &Update) {
        match event {
            Update::HoveredChanged(_) | Update::FocusChanged(_) | Update::DisabledChanged(_) => {
                ctx.request_paint_only();
            }

            _ => {}
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx) {
        ctx.register_child(&mut self.label);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        let x_padding = theme::WIDGET_CONTROL_COMPONENT_PADDING;
        let check_size = theme::BASIC_WIDGET_HEIGHT;

        let label_size = ctx.run_layout(&mut self.label, bc);
        ctx.place_child(&mut self.label, (check_size + x_padding, 0.0).into());

        let desired_size = Size::new(
            check_size + x_padding + label_size.width,
            check_size.max(label_size.height),
        );
        let our_size = bc.constrain(desired_size);
        let baseline =
            ctx.child_baseline_offset(&self.label) + (our_size.height - label_size.height);
        ctx.set_baseline_offset(baseline);
        our_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        let check_size = theme::BASIC_WIDGET_HEIGHT;
        let border_width = 1.;

        let rect = Size::new(check_size, check_size)
            .to_rect()
            .inset(-border_width / 2.)
            .to_rounded_rect(2.);

        fill_lin_gradient(
            scene,
            &rect,
            [theme::BACKGROUND_LIGHT, theme::BACKGROUND_DARK],
            UnitPoint::TOP,
            UnitPoint::BOTTOM,
        );

        let border_color = if ctx.is_hovered() && !ctx.is_disabled() {
            theme::BORDER_LIGHT
        } else {
            theme::BORDER_DARK
        };

        stroke(scene, &rect, border_color, border_width);

        if self.checked {
            // Paint the checkmark
            let mut path = BezPath::new();
            path.move_to((4.0, 9.0));
            path.line_to((8.0, 13.0));
            path.line_to((14.0, 5.0));

            let style = Stroke {
                width: 2.0,
                join: Join::Round,
                miter_limit: 10.0,
                start_cap: Cap::Round,
                end_cap: Cap::Round,
                dash_pattern: SmallVec::default(),
                dash_offset: 0.0,
            };

            let brush = if ctx.is_disabled() {
                theme::DISABLED_TEXT_COLOR
            } else {
                theme::TEXT_COLOR
            };

            scene.stroke(&style, Affine::IDENTITY, brush, None, &path);
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::CheckBox
    }

    fn accessibility(&mut self, ctx: &mut AccessCtx, node: &mut Node) {
        // IMPORTANT: We don't want to merge this code in practice, because
        // the child label already has a 'name' property.
        // This is more of a proof of concept of `get_raw_ref()`.
        if false {
            let label = ctx.get_raw_ref(&self.label);
            let name = label.widget().text().as_ref().to_string();
            node.set_value(name);
        }
        node.add_action(accesskit::Action::Click);
        if self.checked {
            node.set_toggled(Toggled::True);
        } else {
            node.set_toggled(Toggled::False);
        }
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        smallvec![self.label.id()]
    }

    fn make_trace_span(&self, ctx: &QueryCtx<'_>) -> Span {
        trace_span!("Checkbox", id = ctx.widget_id().trace())
    }

    fn get_debug_text(&self) -> Option<String> {
        if self.checked {
            Some("[X]".to_string())
        } else {
            Some("[ ]".to_string())
        }
    }
}

// --- MARK: TESTS ---
#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;

    use super::*;
    use crate::assert_render_snapshot;
    use crate::core::StyleProperty;
    use crate::testing::{TestHarness, TestWidgetExt, widget_ids};
    use crate::theme::PRIMARY_LIGHT;

    #[test]
    fn simple_checkbox() {
        let [checkbox_id] = widget_ids();
        let widget = Checkbox::new(false, "Hello").with_id(checkbox_id);

        let mut harness = TestHarness::create(widget);

        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "hello_unchecked");

        assert_eq!(harness.pop_action(), None);

        harness.mouse_click_on(checkbox_id);
        assert_eq!(
            harness.pop_action(),
            Some((Action::CheckboxToggled(true), checkbox_id))
        );

        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "hello_checked");

        harness.mouse_click_on(checkbox_id);
        assert_eq!(
            harness.pop_action(),
            Some((Action::CheckboxToggled(false), checkbox_id))
        );
    }

    #[test]
    fn edit_checkbox() {
        let image_1 = {
            let checkbox = Checkbox::from_label(
                true,
                Label::new("The quick brown fox jumps over the lazy dog")
                    .with_brush(PRIMARY_LIGHT)
                    .with_style(StyleProperty::FontSize(20.0)),
            );

            let mut harness = TestHarness::create_with_size(checkbox, Size::new(50.0, 50.0));

            harness.render()
        };

        let image_2 = {
            let checkbox = Checkbox::new(false, "Hello world");

            let mut harness = TestHarness::create_with_size(checkbox, Size::new(50.0, 50.0));

            harness.edit_root_widget(|mut checkbox| {
                let mut checkbox = checkbox.downcast::<Checkbox>();
                Checkbox::set_checked(&mut checkbox, true);
                Checkbox::set_text(
                    &mut checkbox,
                    ArcStr::from("The quick brown fox jumps over the lazy dog"),
                );

                let mut label = Checkbox::label_mut(&mut checkbox);
                Label::set_brush(&mut label, PRIMARY_LIGHT);
                Label::insert_style(&mut label, StyleProperty::FontSize(20.0));
            });

            harness.render()
        };

        // We don't use assert_eq because we don't want rich assert
        assert!(image_1 == image_2);
    }
}
