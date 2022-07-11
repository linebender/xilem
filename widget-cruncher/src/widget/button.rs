// Copyright 2018 The Druid Authors.
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

//! A button widget.

use crate::action::Action;
use crate::widget::prelude::*;
use crate::widget::WidgetRef;
use crate::widget::{Label, WidgetPod};
use crate::{theme, ArcStr, Insets, LinearGradient, UnitPoint};
use smallvec::SmallVec;
use tracing::{trace, trace_span, Span};

use super::widget_mut::WidgetMut;

// the minimum padding added to a button.
// NOTE: these values are chosen to match the existing look of TextBox; these
// should be reevaluated at some point.
const LABEL_INSETS: Insets = Insets::uniform_xy(8., 2.);

/// A button with a text label.
pub struct Button {
    label: WidgetPod<Label>,
}

impl Button {
    /// Create a new button with a text label.
    ///
    /// Use the [`.on_click`] method to provide a closure to be called when the
    /// button is clicked.
    ///
    /// # Examples
    ///
    /// ```
    /// use druid::widget::Button;
    ///
    /// let button = Button::new("Increment").on_click(|_ctx, data: &mut u32, _env| {
    ///     *data += 1;
    /// });
    /// ```
    ///
    /// [`.on_click`]: #method.on_click
    pub fn new(text: impl Into<ArcStr>) -> Button {
        Button::from_label(Label::new(text))
    }

    /// Create a new button with the provided [`Label`].
    ///
    /// Use the [`.on_click`] method to provide a closure to be called when the
    /// button is clicked.
    ///
    /// # Examples
    ///
    /// ```
    /// use druid::Color;
    /// use druid::widget::{Button, Label};
    ///
    /// let button = Button::from_label(Label::new("Increment").with_text_color(Color::grey(0.5))).on_click(|_ctx, data: &mut u32, _env| {
    ///     *data += 1;
    /// });
    /// ```
    ///
    /// [`Label`]: struct.Label.html
    /// [`.on_click`]: #method.on_click
    pub fn from_label(label: Label) -> Button {
        Button {
            label: WidgetPod::new(label),
        }
    }
}

impl<'a, 'b> WidgetMut<'a, 'b, Button> {
    /// Set the text.
    pub fn set_text(&mut self, new_text: impl Into<ArcStr>) {
        self.get_label_view().set_text(new_text.into());
    }

    pub fn get_label_view(&mut self) -> WidgetMut<'_, 'b, Label> {
        WidgetMut {
            global_state: self.global_state,
            parent_widget_state: self.widget_state,
            widget_state: &mut self.widget.label.state,
            widget: &mut self.widget.label.inner,
        }
    }
}

impl Widget for Button {
    fn on_event(&mut self, ctx: &mut EventCtx, event: &Event, _env: &Env) {
        ctx.init();
        match event {
            Event::MouseDown(_) => {
                if !ctx.is_disabled() {
                    ctx.set_active(true);
                    ctx.request_paint();
                    trace!("Button {:?} pressed", ctx.widget_id());
                }
            }
            Event::MouseUp(_) => {
                if ctx.is_active() && !ctx.is_disabled() {
                    ctx.submit_action(Action::ButtonPressed);
                    ctx.request_paint();
                    trace!("Button {:?} released", ctx.widget_id());
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
        ctx.init();
        self.label.lifecycle(ctx, event, env)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, env: &Env) -> Size {
        ctx.init();

        let baseline = self.label.baseline_offset();
        ctx.set_baseline_offset(baseline + LABEL_INSETS.y1);

        let padding = Size::new(LABEL_INSETS.x_value(), LABEL_INSETS.y_value());
        let label_bc = bc.shrink(padding).loosen();

        let label_size = self.label.layout(ctx, &label_bc, env);

        // HACK: to make sure we look okay at default sizes when beside a textbox,
        // we make sure we will have at least the same height as the default textbox.
        let min_height = env.get(theme::BORDERED_WIDGET_HEIGHT);

        let button_size = bc.constrain(Size::new(
            label_size.width + padding.width,
            (label_size.height + padding.height).max(min_height),
        ));

        let label_offset = (button_size.to_vec2() - label_size.to_vec2()) / 2.0;
        self.label.set_origin(ctx, env, label_offset.to_point());

        trace!("Computed button size: {}", button_size);
        button_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, env: &Env) {
        ctx.init();
        let is_active = ctx.is_active() && !ctx.is_disabled();
        let is_hot = ctx.is_hot();
        let size = ctx.size();
        let stroke_width = env.get(theme::BUTTON_BORDER_WIDTH);

        let rounded_rect = size
            .to_rect()
            .inset(-stroke_width / 2.0)
            .to_rounded_rect(env.get(theme::BUTTON_BORDER_RADIUS));

        let bg_gradient = if ctx.is_disabled() {
            LinearGradient::new(
                UnitPoint::TOP,
                UnitPoint::BOTTOM,
                (
                    env.get(theme::DISABLED_BUTTON_LIGHT),
                    env.get(theme::DISABLED_BUTTON_DARK),
                ),
            )
        } else if is_active {
            LinearGradient::new(
                UnitPoint::TOP,
                UnitPoint::BOTTOM,
                (env.get(theme::BUTTON_DARK), env.get(theme::BUTTON_LIGHT)),
            )
        } else {
            LinearGradient::new(
                UnitPoint::TOP,
                UnitPoint::BOTTOM,
                (env.get(theme::BUTTON_LIGHT), env.get(theme::BUTTON_DARK)),
            )
        };

        let border_color = if is_hot && !ctx.is_disabled() {
            env.get(theme::BORDER_LIGHT)
        } else {
            env.get(theme::BORDER_DARK)
        };

        ctx.stroke(rounded_rect, &border_color, stroke_width);
        ctx.fill(rounded_rect, &bg_gradient);

        self.label.paint(ctx, env);
    }

    fn children(&self) -> SmallVec<[WidgetRef<'_, dyn Widget>; 16]> {
        SmallVec::new()
    }

    fn make_trace_span(&self) -> Span {
        trace_span!("Button")
    }

    fn get_debug_text(&self) -> Option<String> {
        Some(self.label.widget().text().to_string())
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
    fn simple_button() {
        let [button_id] = widget_ids();
        let widget = Button::new("Hello").with_id(button_id);

        let mut harness = Harness::create(widget);

        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "hello");

        assert_eq!(harness.pop_action(), None);

        harness.mouse_click_on(button_id);
        assert_eq!(
            harness.pop_action(),
            Some((Action::ButtonPressed, button_id))
        );
    }

    #[test]
    fn edit_button() {
        let image_1 = {
            let button = Button::from_label(
                Label::new("The quick brown fox jumps over the lazy dog")
                    .with_text_color(PRIMARY_LIGHT)
                    .with_text_size(20.0),
            );

            let mut harness = Harness::create_with_size(button, Size::new(50.0, 50.0));

            harness.render()
        };

        let image_2 = {
            let button = Button::new("Hello world");

            let mut harness = Harness::create_with_size(button, Size::new(50.0, 50.0));

            harness.edit_root_widget(|mut button, _| {
                let mut button = button.downcast::<Button>().unwrap();
                button.set_text("The quick brown fox jumps over the lazy dog");

                let mut label = button.get_label_view();
                label.set_text_color(PRIMARY_LIGHT);
                label.set_text_size(20.0);
            });

            harness.render()
        };

        // We don't use assert_eq because we don't want rich assert
        assert!(image_1 == image_2);
    }
}
