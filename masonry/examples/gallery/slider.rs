// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::app::RenderRoot;
use masonry::core::{
    ErasedAction, FromDynWidget, Handled, NewWidget, StyleProperty, Widget, WidgetId, WidgetTag,
};
use masonry::layout::{AsUnit, Length};
use masonry::peniko::Color;
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::{Dimensions, ThumbColor, ThumbRadius, TrackColor, TrackThickness};
use masonry::widgets::{Flex, Label, Slider, SliderMoved, Step, StepInput};

use crate::demo::{CONTENT_GAP, DemoPage, ShellTags, wrap_in_shell};
use crate::{ColorSelected, HorizontalColorPicker};

fn desc(text: &str) -> NewWidget<Label> {
    Label::new(text)
        .with_style(StyleProperty::FontSize(14.0))
        .prepare()
        .with_props(Dimensions::width(120.px()))
}
pub(crate) struct SliderDemo {
    shell: ShellTags,
    value_label: WidgetTag<Label>,
    slider: WidgetTag<Slider>,
    track_active_color: WidgetTag<HorizontalColorPicker>,
    track_inactive_color: WidgetTag<HorizontalColorPicker>,
    track_thickness: WidgetTag<StepInput<f64>>,
    thumb_color: WidgetTag<HorizontalColorPicker>,
    thumb_radius: WidgetTag<StepInput<f64>>,
}

impl SliderDemo {
    pub(crate) fn new(shell: ShellTags) -> Self {
        Self {
            shell,
            value_label: WidgetTag::unique(),
            slider: WidgetTag::unique(),
            track_active_color: WidgetTag::unique(),
            track_inactive_color: WidgetTag::unique(),
            track_thickness: WidgetTag::unique(),
            thumb_color: WidgetTag::unique(),
            thumb_radius: WidgetTag::unique(),
        }
    }
}

// TODO: This default props should be get from the current theme.
const DEFAULT_THUMB_COLOR: Color = masonry::theme::TEXT_COLOR;
const DEFAULT_TRACK_ACTIVE_COLOR: Color = masonry::theme::ACCENT_COLOR;
const DEFAULT_TRACK_INACTIVE_COLOR: Color = masonry::theme::ZYNC_800;
const DEFAULT_TRACK_THICKNESS: f64 = 4.;
const DEFAULT_THUMB_RADIUS: f64 = 7.;

fn get_id<W>(rr: &RenderRoot, tag: WidgetTag<W>) -> WidgetId
where
    W: Widget + FromDynWidget + ?Sized,
{
    rr.get_widget_with_tag(tag).unwrap().id()
}

impl DemoPage for SliderDemo {
    fn name(&self) -> &'static str {
        "Slider"
    }

    fn shell_tags(&self) -> ShellTags {
        self.shell
    }

    fn build(&self) -> NewWidget<dyn Widget> {
        let slider =
            NewWidget::new(Slider::new(-1.0, 1.0, 0.0).with_step(0.001)).with_tag(self.slider);

        let body = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(
                NewWidget::new(
                    Label::new("Value: 0.000").with_style(StyleProperty::FontSize(13.0)),
                )
                .with_tag(self.value_label),
            )
            .with_fixed_spacer(CONTENT_GAP)
            .with_fixed(slider)
            .with_fixed_spacer(CONTENT_GAP)
            .with_fixed(
                Flex::row()
                    .with_fixed(desc("Thumb Radius"))
                    .with(
                        StepInput::new(DEFAULT_THUMB_RADIUS, 0.1, 4.0, 30.0)
                            .prepare()
                            .with_tag(self.thumb_radius),
                        1.,
                    )
                    .prepare(),
            )
            .with_fixed_spacer(CONTENT_GAP)
            .with_fixed(
                Flex::row()
                    .with_fixed(desc("Thumb Color"))
                    .with(
                        HorizontalColorPicker::new(DEFAULT_THUMB_COLOR)
                            .prepare()
                            .with_tag(self.thumb_color),
                        1.,
                    )
                    .prepare(),
            )
            .with_fixed_spacer(CONTENT_GAP)
            .with_fixed(
                Flex::row()
                    .with_fixed(desc("Track Thickness"))
                    .with(
                        StepInput::new(DEFAULT_TRACK_THICKNESS, 0.1, 4.0, 30.0)
                            .prepare()
                            .with_tag(self.track_thickness),
                        1.,
                    )
                    .prepare(),
            )
            .with_fixed_spacer(CONTENT_GAP)
            .with_fixed(
                Flex::row()
                    .with_fixed(desc("Track Active Color"))
                    .with(
                        HorizontalColorPicker::new(DEFAULT_TRACK_ACTIVE_COLOR)
                            .prepare()
                            .with_tag(self.track_active_color),
                        1.,
                    )
                    .prepare(),
            )
            .with_fixed_spacer(CONTENT_GAP)
            .with_fixed(
                Flex::row()
                    .with_fixed(desc("Track Inactive Color"))
                    .with(
                        HorizontalColorPicker::new(DEFAULT_TRACK_INACTIVE_COLOR)
                            .prepare()
                            .with_tag(self.track_inactive_color),
                        1.,
                    )
                    .prepare(),
            );

        wrap_in_shell(self.shell, NewWidget::new(body).erased())
    }

    fn on_action(
        &mut self,
        render_root: &mut RenderRoot,
        action: &ErasedAction,
        widget_id: WidgetId,
    ) -> Handled {
        if let Some(value) = action.downcast_ref::<ColorSelected>() {
            if get_id(render_root, self.thumb_color) == widget_id {
                render_root.edit_widget_with_tag(self.slider, |mut slider| {
                    slider.insert_prop(ThumbColor(value.color));
                });
            } else if get_id(render_root, self.track_active_color) == widget_id {
                render_root.edit_widget_with_tag(self.slider, |mut slider| {
                    if let Some(mut prop) = slider.insert_prop(TrackColor {
                        ..Default::default()
                    }) {
                        prop.active = value.color;
                        slider.insert_prop(prop);
                    } else {
                        slider.insert_prop(TrackColor {
                            active: value.color,
                            inactive: DEFAULT_TRACK_INACTIVE_COLOR,
                        });
                    }
                });
            } else if get_id(render_root, self.track_inactive_color) == widget_id {
                render_root.edit_widget_with_tag(self.slider, |mut slider| {
                    if let Some(mut prop) = slider.insert_prop(TrackColor {
                        ..Default::default()
                    }) {
                        prop.inactive = value.color;
                        slider.insert_prop(prop);
                    } else {
                        slider.insert_prop(TrackColor {
                            active: DEFAULT_TRACK_ACTIVE_COLOR,
                            inactive: value.color,
                        });
                    }
                });
            };
            return Handled::Yes;
        };

        if let Some(value) = action.downcast_ref::<SliderMoved>() {
            let value = value.value;

            if get_id(render_root, self.slider) == widget_id {
                render_root.edit_widget_with_tag(self.value_label, |mut label| {
                    Label::set_text(&mut label, format!("Value: {value:.3}"));
                });
            }
            return Handled::Yes;
        };

        if let Some(value) = action.downcast_ref::<Step<f64>>() {
            let value = value.value;

            if get_id(render_root, self.thumb_radius) == widget_id {
                render_root.edit_widget_with_tag(self.slider, |mut slider| {
                    slider.insert_prop(ThumbRadius(Length::const_px(value)));
                });
            } else if get_id(render_root, self.track_thickness) == widget_id {
                render_root.edit_widget_with_tag(self.slider, |mut slider| {
                    slider.insert_prop(TrackThickness(Length::const_px(value)));
                });
            }
            return Handled::Yes;
        };

        Handled::No
    }
}
