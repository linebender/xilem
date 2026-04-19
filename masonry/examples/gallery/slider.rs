// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, Handled, NewWidget, StyleProperty, Widget, WidgetId, WidgetTag};
use masonry::properties::types::CrossAxisAlignment;
use masonry::widgets::{Flex, Label, Slider, SliderMoved};

use crate::demo::{CONTENT_GAP, DemoPage, ShellTags, wrap_in_shell};

pub(crate) struct SliderDemo {
    shell: ShellTags,
    value_label: WidgetTag<Label>,
    slider: WidgetTag<Slider>,
}

impl SliderDemo {
    pub(crate) fn new(shell: ShellTags) -> Self {
        Self {
            shell,
            value_label: WidgetTag::unique(),
            slider: WidgetTag::unique(),
        }
    }
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
            .with_fixed(slider);

        wrap_in_shell(self.shell, NewWidget::new(body).erased())
    }

    fn on_action(
        &mut self,
        render_root: &mut RenderRoot,
        action: &ErasedAction,
        widget_id: WidgetId,
    ) -> Handled {
        let Some(value) = action.downcast_ref::<SliderMoved>() else {
            return Handled::No;
        };
        let value = value.value;

        let id = render_root.get_widget_with_tag(self.slider).unwrap().id();
        if widget_id != id {
            return Handled::No;
        }
        render_root.edit_widget_with_tag(self.value_label, |mut label| {
            Label::set_text(&mut label, format!("Value: {value:.3}"));
        });
        Handled::Yes
    }
}
