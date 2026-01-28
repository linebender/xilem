// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::app::RenderRoot;
use masonry::core::{NewWidget, StyleProperty, Widget, WidgetId, WidgetTag};
use masonry::properties::types::CrossAxisAlignment;
use masonry::widgets::{Checkbox, Flex, Label};

use crate::demo::{CONTENT_GAP, DemoPage, ShellTags, wrap_in_shell};

pub(crate) struct CheckboxDemo {
    shell: ShellTags,
    state_label: WidgetTag<Label>,
    checkbox: WidgetTag<Checkbox>,
}

impl CheckboxDemo {
    pub(crate) fn new(shell: ShellTags) -> Self {
        Self {
            shell,
            state_label: WidgetTag::unique(),
            checkbox: WidgetTag::unique(),
        }
    }
}

impl DemoPage for CheckboxDemo {
    fn name(&self) -> &'static str {
        "Checkbox"
    }

    fn shell_tags(&self) -> ShellTags {
        self.shell
    }

    fn build(&self) -> NewWidget<dyn Widget> {
        let checkbox = NewWidget::new_with_tag(Checkbox::new(false, "Check me"), self.checkbox);

        let body = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(NewWidget::new_with_tag(
                Label::new("Checked: false").with_style(StyleProperty::FontSize(13.0)),
                self.state_label,
            ))
            .with_fixed_spacer(CONTENT_GAP)
            .with_fixed(checkbox);

        wrap_in_shell(self.shell, NewWidget::new(body).erased())
    }

    fn on_checkbox_toggled(
        &mut self,
        render_root: &mut RenderRoot,
        widget_id: WidgetId,
        checked: bool,
    ) -> bool {
        let checkbox_id = render_root.get_widget_with_tag(self.checkbox).unwrap().id();
        if widget_id != checkbox_id {
            return false;
        }

        render_root.edit_widget_with_tag(self.state_label, |mut label| {
            Label::set_text(&mut label, format!("Checked: {checked}"));
        });
        render_root.edit_widget_with_tag(self.checkbox, |mut checkbox| {
            Checkbox::set_checked(&mut checkbox, checked);
        });
        true
    }
}
