// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::app::RenderRoot;
use masonry::core::{NewWidget, StyleProperty, Widget, WidgetId, WidgetTag};
use masonry::properties::types::CrossAxisAlignment;
use masonry::widgets::{Flex, Label};
use masonry::widgets::{RadioButton, RadioGroup};

use crate::demo::{CONTENT_GAP, DemoPage, ShellTags, wrap_in_shell};

pub(crate) struct RadioButtonsDemo {
    shell: ShellTags,
    state_label: WidgetTag<Label>,
}

impl RadioButtonsDemo {
    pub(crate) fn new(shell: ShellTags) -> Self {
        Self {
            shell,
            state_label: WidgetTag::unique(),
        }
    }
}

impl DemoPage for RadioButtonsDemo {
    fn name(&self) -> &'static str {
        "Radio buttons"
    }

    fn shell_tags(&self) -> ShellTags {
        self.shell
    }

    fn build(&self) -> NewWidget<dyn Widget> {
        let buttons = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .with_fixed(NewWidget::new(RadioButton::new(false, "Monday")))
            .with_fixed(NewWidget::new(RadioButton::new(false, "Tuesday")))
            .with_fixed(NewWidget::new(RadioButton::new(false, "Wednesday")))
            .with_fixed(NewWidget::new(RadioButton::new(false, "Thursday")))
            .with_fixed(NewWidget::new(RadioButton::new(false, "Friday")))
            .with_fixed(NewWidget::new(RadioButton::new(false, "Saturday")))
            .with_fixed(NewWidget::new(RadioButton::new(false, "Other Saturday")));
        let group = RadioGroup::new(NewWidget::new(buttons));

        let body = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(NewWidget::new_with_tag(
                Label::new("No button selected").with_style(StyleProperty::FontSize(13.0)),
                self.state_label,
            ))
            .with_fixed_spacer(CONTENT_GAP)
            .with_fixed(NewWidget::new(group));

        wrap_in_shell(self.shell, NewWidget::new(body).erased())
    }

    fn on_radio_button_selected(
        &mut self,
        render_root: &mut RenderRoot,
        widget_id: WidgetId,
    ) -> bool {
        let selected_text = render_root.edit_widget(widget_id, |mut button| {
            let mut button = button.downcast::<RadioButton>();
            let label = RadioButton::label_mut(&mut button);
            label.widget.text().clone()
        });

        render_root.edit_widget_with_tag(self.state_label, move |mut label| {
            Label::set_text(&mut label, format!("Selected: {selected_text}"));
        });
        true
    }
}
