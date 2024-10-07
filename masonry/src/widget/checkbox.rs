// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A checkbox widget.

use accesskit::{DefaultActionVerb, NodeBuilder, Role, Toggled};
use smallvec::{smallvec, SmallVec};
use tracing::{trace, trace_span, Span};
use vello::kurbo::{Affine, BezPath, Cap, Join, Size, Stroke};
use vello::Scene;

use crate::action::Action;
use crate::paint_scene_helpers::{fill_lin_gradient, stroke, UnitPoint};
use crate::text::ArcStr;
use crate::widget::{Label, WidgetMut};
use crate::{
    theme, AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, LifeCycleCtx, PaintCtx,
    PointerEvent, RegisterCtx, StatusChange, TextEvent, Widget, WidgetId, WidgetPod,
};

/// A checkbox that can be toggled.
pub struct Checkbox {
    checked: bool,
    label: WidgetPod<Label>,
}

impl Checkbox {
    /// Create a new `Checkbox` with a text label.
    pub fn new(checked: bool, text: impl Into<ArcStr>) -> Checkbox {
        Checkbox {
            checked,
            label: WidgetPod::new(Label::new(text).with_skip_pointer(true)),
        }
    }

    /// Create a new `Checkbox` with the given label.
    pub fn from_label(checked: bool, label: Label) -> Checkbox {
        Checkbox {
            checked,
            label: WidgetPod::new(label.with_skip_pointer(true)),
        }
    }
}

// --- MARK: WIDGETMUT ---
impl WidgetMut<'_, Checkbox> {
    pub fn set_checked(&mut self, checked: bool) {
        self.widget.checked = checked;
        self.ctx.request_paint();
        self.ctx.request_accessibility_update();
    }

    /// Set the text.
    ///
    /// We enforce this to be an `ArcStr` to make the allocation explicit.
    pub fn set_text(&mut self, new_text: ArcStr) {
        self.label_mut().set_text(new_text);
    }

    pub fn label_mut(&mut self) -> WidgetMut<'_, Label> {
        self.ctx.get_mut(&mut self.widget.label)
    }
}

// --- MARK: IMPL WIDGET ---
impl Widget for Checkbox {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        match event {
            PointerEvent::PointerDown(_, _) => {
                if !ctx.is_disabled() {
                    ctx.capture_pointer();
                    ctx.request_paint();
                    trace!("Checkbox {:?} pressed", ctx.widget_id());
                }
            }
            PointerEvent::PointerUp(_, _) => {
                if ctx.has_pointer_capture() && ctx.is_hot() && !ctx.is_disabled() {
                    self.checked = !self.checked;
                    ctx.submit_action(Action::CheckboxChecked(self.checked));
                    ctx.request_accessibility_update();
                    trace!("Checkbox {:?} released", ctx.widget_id());
                }
                ctx.request_paint();
            }
            _ => (),
        }
    }

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}

    fn on_access_event(&mut self, ctx: &mut EventCtx, event: &AccessEvent) {
        if event.target == ctx.widget_id() {
            match event.action {
                accesskit::Action::Default => {
                    self.checked = !self.checked;
                    ctx.submit_action(Action::CheckboxChecked(self.checked));
                    ctx.request_paint();
                    ctx.request_accessibility_update();
                }
                _ => {}
            }
        }
    }

    fn on_status_change(&mut self, ctx: &mut LifeCycleCtx, _event: &StatusChange) {
        ctx.request_paint();
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
        trace!("Computed layout: size={}, baseline={}", our_size, baseline);
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

        let border_color = if ctx.is_hot() && !ctx.is_disabled() {
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
                dash_pattern: Default::default(),
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

    fn accessibility(&mut self, ctx: &mut AccessCtx, node: &mut NodeBuilder) {
        // IMPORTANT: We don't want to merge this code in practice, because
        // the child label already has a 'name' property.
        // This is more of a proof of concept of `get_raw_ref()`.
        if false {
            let label = ctx.get_raw_ref(&self.label);
            let name = label.widget().text().as_ref().to_string();
            node.set_name(name);
        }
        if self.checked {
            node.set_toggled(Toggled::True);
            node.set_default_action_verb(DefaultActionVerb::Uncheck);
        } else {
            node.set_toggled(Toggled::False);
            node.set_default_action_verb(DefaultActionVerb::Check);
        }
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        smallvec![self.label.id()]
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("Checkbox")
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
    use crate::testing::{widget_ids, TestHarness, TestWidgetExt};
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
            Some((Action::CheckboxChecked(true), checkbox_id))
        );

        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "hello_checked");

        harness.mouse_click_on(checkbox_id);
        assert_eq!(
            harness.pop_action(),
            Some((Action::CheckboxChecked(false), checkbox_id))
        );
    }

    #[test]
    fn edit_checkbox() {
        let image_1 = {
            let checkbox = Checkbox::from_label(
                true,
                Label::new("The quick brown fox jumps over the lazy dog")
                    .with_text_brush(PRIMARY_LIGHT)
                    .with_text_size(20.0),
            );

            let mut harness = TestHarness::create_with_size(checkbox, Size::new(50.0, 50.0));

            harness.render()
        };

        let image_2 = {
            let checkbox = Checkbox::new(false, "Hello world");

            let mut harness = TestHarness::create_with_size(checkbox, Size::new(50.0, 50.0));

            harness.edit_root_widget(|mut checkbox| {
                let mut checkbox = checkbox.downcast::<Checkbox>();
                checkbox.set_checked(true);
                checkbox.set_text(ArcStr::from("The quick brown fox jumps over the lazy dog"));

                let mut label = checkbox.label_mut();
                label.set_text_brush(PRIMARY_LIGHT);
                label.set_text_size(20.0);
            });

            harness.render()
        };

        // We don't use assert_eq because we don't want rich assert
        assert!(image_1 == image_2);
    }
}
