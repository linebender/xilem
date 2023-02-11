// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

//! A checkbox widget.

use smallvec::SmallVec;
use tracing::{trace, trace_span, Span};

use crate::action::Action;
use crate::kurbo::{BezPath, Size};
use crate::piet::{LineCap, LineJoin, LinearGradient, RenderContext, StrokeStyle, UnitPoint};
use crate::widget::{Label, WidgetMut, WidgetRef};
use crate::{
    theme, ArcStr, BoxConstraints, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx,
    PaintCtx, StatusChange, Widget, WidgetPod,
};

/// A checkbox that can be toggled.
pub struct Checkbox {
    checked: bool,
    label: WidgetPod<Label>,
}

crate::declare_widget!(CheckboxMut, Checkbox);

impl Checkbox {
    /// Create a new `Checkbox` with a text label.
    pub fn new(checked: bool, text: impl Into<ArcStr>) -> Checkbox {
        Checkbox {
            checked,
            label: WidgetPod::new(Label::new(text)),
        }
    }

    /// Create a new `Checkbox` with the given label.
    pub fn from_label(checked: bool, label: Label) -> Checkbox {
        Checkbox {
            checked,
            label: WidgetPod::new(label),
        }
    }
}

impl<'a, 'b> CheckboxMut<'a, 'b> {
    pub fn set_checked(&mut self, checked: bool) {
        self.widget.checked = checked;
        self.ctx.request_paint();
    }

    /// Set the text.
    pub fn set_text(&mut self, new_text: impl Into<ArcStr>) {
        self.label_mut().set_text(new_text.into());
    }

    pub fn label_mut(&mut self) -> WidgetMut<'_, 'b, Label> {
        self.ctx.get_mut(&mut self.widget.label)
    }
}

impl Widget for Checkbox {
    fn on_event(&mut self, ctx: &mut EventCtx, event: &Event, _env: &Env) {
        match event {
            Event::MouseDown(_) => {
                if !ctx.is_disabled() {
                    ctx.set_active(true);
                    ctx.request_paint();
                    trace!("Checkbox {:?} pressed", ctx.widget_id());
                }
            }
            Event::MouseUp(_) => {
                if ctx.is_active() && !ctx.is_disabled() {
                    if ctx.is_hot() {
                        self.checked = !self.checked;
                        ctx.submit_action(Action::CheckboxChecked(self.checked));
                        trace!("Checkbox {:?} released", ctx.widget_id());
                    }
                    ctx.request_paint();
                }
                ctx.set_active(false);
            }
            _ => (),
        }
    }

    fn on_status_change(&mut self, ctx: &mut LifeCycleCtx, _event: &StatusChange, _env: &Env) {
        ctx.request_paint();
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, env: &Env) {
        self.label.lifecycle(ctx, event, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, env: &Env) -> Size {
        let x_padding = env.get(theme::WIDGET_CONTROL_COMPONENT_PADDING);
        let check_size = env.get(theme::BASIC_WIDGET_HEIGHT);

        let label_size = self.label.layout(ctx, bc, env);
        ctx.place_child(&mut self.label, (check_size + x_padding, 0.0).into(), env);

        let desired_size = Size::new(
            check_size + x_padding + label_size.width,
            check_size.max(label_size.height),
        );
        let our_size = bc.constrain(desired_size);
        let baseline = self.label.baseline_offset() + (our_size.height - label_size.height);
        ctx.set_baseline_offset(baseline);
        trace!("Computed layout: size={}, baseline={}", our_size, baseline);
        our_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, env: &Env) {
        let check_size = env.get(theme::BASIC_WIDGET_HEIGHT);
        let border_width = 1.;

        let rect = Size::new(check_size, check_size)
            .to_rect()
            .inset(-border_width / 2.)
            .to_rounded_rect(2.);

        //Paint the background
        let background_gradient = LinearGradient::new(
            UnitPoint::TOP,
            UnitPoint::BOTTOM,
            (
                env.get(theme::BACKGROUND_LIGHT),
                env.get(theme::BACKGROUND_DARK),
            ),
        );

        ctx.fill(rect, &background_gradient);

        let border_color = if ctx.is_hot() && !ctx.is_disabled() {
            env.get(theme::BORDER_LIGHT)
        } else {
            env.get(theme::BORDER_DARK)
        };

        ctx.stroke(rect, &border_color, border_width);

        if self.checked {
            // Paint the checkmark
            let mut path = BezPath::new();
            path.move_to((4.0, 9.0));
            path.line_to((8.0, 13.0));
            path.line_to((14.0, 5.0));

            let style = StrokeStyle::new()
                .line_cap(LineCap::Round)
                .line_join(LineJoin::Round);

            let brush = if ctx.is_disabled() {
                env.get(theme::DISABLED_TEXT_COLOR)
            } else {
                env.get(theme::TEXT_COLOR)
            };

            ctx.stroke_styled(path, &brush, 2., &style);
        }

        // Paint the text label
        self.label.paint(ctx, env);
    }

    fn children(&self) -> SmallVec<[WidgetRef<'_, dyn Widget>; 16]> {
        SmallVec::new()
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("Checkbox")
    }

    fn get_debug_text(&self) -> Option<String> {
        Some(format!(
            "[{}] {}",
            if self.checked { "X" } else { " " },
            self.label.as_ref().text()
        ))
    }
}

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
                    .with_text_color(PRIMARY_LIGHT)
                    .with_text_size(20.0),
            );

            let mut harness = TestHarness::create_with_size(checkbox, Size::new(50.0, 50.0));

            harness.render()
        };

        let image_2 = {
            let checkbox = Checkbox::new(false, "Hello world");

            let mut harness = TestHarness::create_with_size(checkbox, Size::new(50.0, 50.0));

            harness.edit_root_widget(|mut checkbox, _| {
                let mut checkbox = checkbox.downcast::<Checkbox>().unwrap();
                checkbox.set_checked(true);
                checkbox.set_text("The quick brown fox jumps over the lazy dog");

                let mut label = checkbox.label_mut();
                label.set_text_color(PRIMARY_LIGHT);
                label.set_text_size(20.0);
            });

            harness.render()
        };

        // We don't use assert_eq because we don't want rich assert
        assert!(image_1 == image_2);
    }
}
