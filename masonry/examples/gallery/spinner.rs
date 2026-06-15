// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, Handled, NewWidget, PropertySet, Widget, WidgetId, WidgetTag};
use masonry::layout::AsUnit;
use masonry::peniko::Color;
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::{
    AnimationDuration, BorderColor, BorderWidth, TrackColor, TrackThickness,
};
use masonry::theme::TEXT_COLOR;
use masonry::widgets::{Button, ButtonPress, Flex, Label, SizedBox, Spinner, Step, StepInput};

use crate::demo::{DemoPage, ShellTags, wrap_in_shell};

pub(crate) struct SpinnerDemo {
    shell: ShellTags,
    duration: f64,
    thickness: f64,
    width: f64,
    height: f64,

    tag_spinner: WidgetTag<Spinner>,
    tag_spinner_box: WidgetTag<SizedBox>,
    tag_duration_input: WidgetTag<StepInput<f64>>,
    tag_thickness_input: WidgetTag<StepInput<f64>>,
    tag_width_input: WidgetTag<StepInput<f64>>,
    tag_height_input: WidgetTag<StepInput<f64>>,
    tag_theme_buttons: [WidgetTag<Button>; 4],
}

impl SpinnerDemo {
    pub(crate) fn new(shell: ShellTags) -> Self {
        Self {
            shell,
            duration: 1.2,
            thickness: 8.0,
            width: 140.0,
            height: 140.0,
            tag_spinner: WidgetTag::unique(),
            tag_spinner_box: WidgetTag::unique(),
            tag_duration_input: WidgetTag::unique(),
            tag_thickness_input: WidgetTag::unique(),
            tag_width_input: WidgetTag::unique(),
            tag_height_input: WidgetTag::unique(),
            tag_theme_buttons: std::array::from_fn(|_| WidgetTag::unique()),
        }
    }
}

fn desc(text: &str) -> NewWidget<Label> {
    Label::new(text)
        .with_style(masonry::core::StyleProperty::FontSize(14.0))
        .prepare()
        .with_props(masonry::properties::Dimensions::width(200.px()))
}

impl DemoPage for SpinnerDemo {
    fn name(&self) -> &'static str {
        "Spinner"
    }

    fn shell_tags(&self) -> ShellTags {
        self.shell
    }

    fn on_action(
        &mut self,
        render_root: &mut RenderRoot,
        action: &ErasedAction,
        widget_id: WidgetId,
    ) -> Handled {
        macro_rules! matches_tag {
            ($tag:expr) => {
                Some(widget_id) == render_root.get_widget_with_tag($tag).map(|w| w.id())
            };
        }

        if action.is::<ButtonPress>() {
            let track_color = match () {
                _ if matches_tag!(self.tag_theme_buttons[0]) => Some(TrackColor {
                    active: TEXT_COLOR,
                    inactive: Color::TRANSPARENT,
                }),
                _ if matches_tag!(self.tag_theme_buttons[1]) => Some(TrackColor {
                    active: Color::from_rgba8(0x36, 0x21, 0x8f, 0xff),
                    inactive: Color::from_rgba8(0xff, 0xff, 0xff, 0x2a),
                }),
                _ if matches_tag!(self.tag_theme_buttons[2]) => Some(TrackColor {
                    active: Color::from_rgba8(0xff, 0xff, 0xff, 0xff),
                    inactive: Color::from_rgba8(0x2a, 0x00, 0x96, 0xff),
                }),
                _ if matches_tag!(self.tag_theme_buttons[3]) => Some(TrackColor {
                    active: Color::from_rgba8(0xff, 0x50, 0x50, 0xff),
                    inactive: Color::from_rgba8(0xb7, 0x00, 0x00, 0xff),
                }),
                _ => None,
            };
            if let Some(prop) = track_color {
                render_root.edit_widget_with_tag(self.tag_spinner, |mut w| {
                    w.insert_prop(prop);
                });
                return Handled::Yes;
            }
        }

        let Some(step) = action.downcast_ref::<Step<f64>>() else {
            return Handled::No;
        };
        let value = step.value;

        match () {
            _ if matches_tag!(self.tag_duration_input) => {
                self.duration = value;
                render_root.edit_widget_with_tag(self.tag_spinner, |mut w| {
                    w.insert_prop(AnimationDuration { seconds: value });
                });
                Handled::Yes
            }
            _ if matches_tag!(self.tag_thickness_input) => {
                self.thickness = value;
                render_root.edit_widget_with_tag(self.tag_spinner, |mut w| {
                    w.insert_prop(TrackThickness(value.px()));
                });
                Handled::Yes
            }
            _ if matches_tag!(self.tag_width_input) => {
                self.width = value;
                render_root.edit_widget_with_tag(self.tag_spinner_box, |mut w| {
                    SizedBox::set_width(&mut w, value.px());
                });
                Handled::Yes
            }
            _ if matches_tag!(self.tag_height_input) => {
                self.height = value;
                render_root.edit_widget_with_tag(self.tag_spinner_box, |mut w| {
                    SizedBox::set_height(&mut w, value.px());
                });
                Handled::Yes
            }
            _ => Handled::No,
        }
    }

    fn build(&self) -> NewWidget<dyn Widget> {
        let props = PropertySet::new()
            .with(BorderColor::new(Color::from_rgb8(40, 40, 80)))
            .with(BorderWidth::all(1.px()))
            .with(AnimationDuration {
                seconds: self.duration,
            })
            .with(TrackThickness(self.thickness.px()));

        let preview = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .with_fixed(
                NewWidget::new(
                    SizedBox::new(
                        NewWidget::new(Spinner::new())
                            .with_props(props)
                            .with_tag(self.tag_spinner),
                    )
                    .size(self.width.px(), self.height.px()),
                )
                .with_tag(self.tag_spinner_box),
            );

        let theme_buttons = Flex::row()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with(
                NewWidget::new(Button::with_text("Default")).with_tag(self.tag_theme_buttons[0]),
                1.,
            )
            .with(
                NewWidget::new(Button::with_text("Transparent Blue"))
                    .with_tag(self.tag_theme_buttons[1]),
                1.,
            )
            .with(
                NewWidget::new(Button::with_text("Opaque Blue"))
                    .with_tag(self.tag_theme_buttons[2]),
                1.,
            )
            .with(
                NewWidget::new(Button::with_text("Red")).with_tag(self.tag_theme_buttons[3]),
                1.,
            );

        // Helper closures to quickly assemble repetitive UI layout definitions
        let row = |label, val, range_start, range_end, step, tag| {
            Flex::row()
                .with_fixed(desc(label))
                .with(
                    NewWidget::new(StepInput::new(val, step, range_start, range_end)).with_tag(tag),
                    1.,
                )
                .prepare()
        };

        let controls = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(row(
                "Animation Duration (s)",
                self.duration,
                0.1,
                6.0,
                0.1,
                self.tag_duration_input,
            ))
            .with_fixed(row(
                "Track Thickness (px)",
                self.thickness,
                0.0,
                40.0,
                1.0,
                self.tag_thickness_input,
            ))
            .with_fixed(row(
                "Width (px)",
                self.width,
                0.0,
                400.0,
                10.0,
                self.tag_width_input,
            ))
            .with_fixed(row(
                "Height (px)",
                self.height,
                0.0,
                400.0,
                10.0,
                self.tag_height_input,
            ))
            .with_fixed(
                Flex::row()
                    .with_fixed(desc("Track Colors"))
                    .with(theme_buttons.prepare(), 1.)
                    .prepare(),
            );

        let main_layout = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(preview.prepare())
            .with_fixed(controls.prepare());

        wrap_in_shell(self.shell, NewWidget::new(main_layout).erased())
    }
}
