// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::app::RenderRoot;
use masonry::core::{NewWidget, StyleProperty, Widget, WidgetId, WidgetTag};
use masonry::properties::types::CrossAxisAlignment;
use masonry::widgets::{Checkbox, Flex, Label, ProgressBar, Slider};

use crate::demo::{CONTENT_GAP, DemoPage, ShellTags, wrap_in_shell};

pub(crate) struct ProgressDemo {
    shell: ShellTags,
    bar: WidgetTag<ProgressBar>,
    value_label: WidgetTag<Label>,
    slider: WidgetTag<Slider>,
    indeterminate: WidgetTag<Checkbox>,
    value: f64,
    is_indeterminate: bool,
}

impl ProgressDemo {
    pub(crate) fn new(shell: ShellTags) -> Self {
        Self {
            shell,
            bar: WidgetTag::unique(),
            value_label: WidgetTag::unique(),
            slider: WidgetTag::unique(),
            indeterminate: WidgetTag::unique(),
            value: 0.35,
            is_indeterminate: false,
        }
    }

    fn apply(&mut self, render_root: &mut RenderRoot) {
        let progress = if self.is_indeterminate {
            None
        } else {
            Some(self.value)
        };

        render_root.edit_widget_with_tag(self.bar, |mut bar| {
            ProgressBar::set_progress(&mut bar, progress);
        });

        render_root.edit_widget_with_tag(self.value_label, |mut label| {
            Label::set_text(
                &mut label,
                if let Some(value) = progress {
                    format!("Value: {:.0}%", value * 100.0)
                } else {
                    "Value: indeterminate".to_string()
                },
            );
        });

        render_root.edit_widget_with_tag(self.indeterminate, |mut checkbox| {
            Checkbox::set_checked(&mut checkbox, self.is_indeterminate);
        });
    }
}

impl DemoPage for ProgressDemo {
    fn name(&self) -> &'static str {
        "Progress bar"
    }

    fn shell_tags(&self) -> ShellTags {
        self.shell
    }

    fn build(&self) -> NewWidget<dyn Widget> {
        let slider = NewWidget::new_with_tag(
            Slider::new(0.0, 1.0, self.value).with_step(0.01),
            self.slider,
        );
        let indeterminate =
            NewWidget::new_with_tag(Checkbox::new(false, "Indeterminate"), self.indeterminate);

        let body = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(NewWidget::new_with_tag(
                ProgressBar::new(Some(self.value)),
                self.bar,
            ))
            .with_fixed(NewWidget::new_with_tag(
                Label::new("Value: 35%").with_style(StyleProperty::FontSize(13.0)),
                self.value_label,
            ))
            .with_fixed_spacer(CONTENT_GAP)
            .with_fixed(slider)
            .with_fixed_spacer(CONTENT_GAP)
            .with_fixed(indeterminate);

        wrap_in_shell(self.shell, NewWidget::new(body).erased())
    }

    fn on_selected(&mut self, render_root: &mut RenderRoot) {
        self.apply(render_root);
    }

    fn on_checkbox_toggled(
        &mut self,
        render_root: &mut RenderRoot,
        widget_id: WidgetId,
        toggled: bool,
    ) -> bool {
        let id = render_root
            .get_widget_with_tag(self.indeterminate)
            .unwrap()
            .id();
        if widget_id != id {
            return false;
        }
        self.is_indeterminate = toggled;
        self.apply(render_root);
        true
    }

    fn on_slider_value(
        &mut self,
        render_root: &mut RenderRoot,
        widget_id: WidgetId,
        value: f64,
    ) -> bool {
        let id = render_root.get_widget_with_tag(self.slider).unwrap().id();
        if widget_id != id {
            return false;
        }
        self.value = value.clamp(0.0, 1.0);
        self.apply(render_root);
        true
    }
}
