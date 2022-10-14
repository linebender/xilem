// Copyright 2019 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! A checkbox widget.

use crate::action::Action;
use crate::contexts::WidgetCtx;
use crate::kurbo::{BezPath, Size};
use crate::piet::{LineCap, LineJoin, LinearGradient, RenderContext, StrokeStyle, UnitPoint};
use crate::widget::prelude::*;
use crate::widget::{Label, StoreInWidgetMut, WidgetMut, WidgetRef};
use crate::ArcStr;
use crate::{theme, WidgetPod};

use smallvec::SmallVec;
use tracing::{trace, trace_span, Span};

/// A checkbox that toggles a `bool`.
pub struct Checkbox {
    checked: bool,
    label: WidgetPod<Label>,
}

pub struct CheckboxMut<'a, 'b>(WidgetCtx<'a, 'b>, &'a mut Checkbox);

impl Checkbox {
    /// Create a new `Checkbox` with a text label.
    pub fn new(checked: bool, text: impl Into<ArcStr>) -> Checkbox {
        Checkbox {
            checked,
            label: WidgetPod::new(Label::new(text)),
        }
    }

    pub fn from_label(checked: bool, label: Label) -> Checkbox {
        Checkbox {
            checked,
            label: WidgetPod::new(label),
        }
    }
}

impl<'a, 'b> CheckboxMut<'a, 'b> {
    pub fn set_checked(&mut self, checked: bool) {
        self.1.checked = checked;
        self.0.request_paint();
    }

    /// Set the text.
    pub fn set_text(&mut self, new_text: impl Into<ArcStr>) {
        self.label_mut().set_text(new_text.into());
    }

    pub fn label_mut(&mut self) -> WidgetMut<'_, 'b, Label> {
        self.0.get_mut(&mut self.1.label)
    }
}

impl Widget for Checkbox {
    fn on_event(&mut self, ctx: &mut EventCtx, event: &Event, _env: &Env) {
        ctx.init();
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
        ctx.init();
        ctx.request_paint();
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, env: &Env) {
        self.label.lifecycle(ctx, event, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, env: &Env) -> Size {
        ctx.init();

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
        ctx.init();
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
            self.label.widget().text()
        ))
    }
}

impl StoreInWidgetMut for Checkbox {
    type Mut<'a, 'b: 'a> = CheckboxMut<'a, 'b>;

    fn get_widget_and_ctx<'s: 'r, 'a: 'r, 'b: 'a, 'r>(
        widget_mut: &'s mut Self::Mut<'a, 'b>,
    ) -> (&'r mut Self, &'r mut WidgetCtx<'a, 'b>) {
        (widget_mut.1, &mut widget_mut.0)
    }

    fn from_widget_and_ctx<'a, 'b>(
        widget: &'a mut Self,
        ctx: WidgetCtx<'a, 'b>,
    ) -> Self::Mut<'a, 'b> {
        CheckboxMut(ctx, widget)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_render_snapshot;
    use crate::testing::widget_ids;
    use crate::testing::Harness;
    use crate::testing::TestWidgetExt;
    use crate::theme::PRIMARY_LIGHT;
    use insta::assert_debug_snapshot;

    #[test]
    fn simple_checkbox() {
        let [checkbox_id] = widget_ids();
        let widget = Checkbox::new(false, "Hello").with_id(checkbox_id);

        let mut harness = Harness::create(widget);

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

            let mut harness = Harness::create_with_size(checkbox, Size::new(50.0, 50.0));

            harness.render()
        };

        let image_2 = {
            let checkbox = Checkbox::new(false, "Hello world");

            let mut harness = Harness::create_with_size(checkbox, Size::new(50.0, 50.0));

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
