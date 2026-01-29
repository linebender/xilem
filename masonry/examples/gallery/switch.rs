// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::app::RenderRoot;
use masonry::core::{NewWidget, StyleProperty, Widget, WidgetId, WidgetTag};
use masonry::properties::types::CrossAxisAlignment;
use masonry::widgets::{Flex, Label, Switch};

use crate::demo::{CONTENT_GAP, DemoPage, ShellTags, wrap_in_shell};

pub(crate) struct SwitchDemo {
    shell: ShellTags,
    state_label: WidgetTag<Label>,
    switch: WidgetTag<Switch>,
}

impl SwitchDemo {
    pub(crate) fn new(shell: ShellTags) -> Self {
        Self {
            shell,
            state_label: WidgetTag::unique(),
            switch: WidgetTag::unique(),
        }
    }
}

impl DemoPage for SwitchDemo {
    fn name(&self) -> &'static str {
        "Switch"
    }

    fn shell_tags(&self) -> ShellTags {
        self.shell
    }

    fn build(&self) -> NewWidget<dyn Widget> {
        let switch = NewWidget::new_with_tag(Switch::new(false), self.switch);

        let body = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(NewWidget::new_with_tag(
                Label::new("On: false").with_style(StyleProperty::FontSize(13.0)),
                self.state_label,
            ))
            .with_fixed_spacer(CONTENT_GAP)
            .with_fixed(switch);

        wrap_in_shell(self.shell, NewWidget::new(body).erased())
    }

    fn on_switch_toggled(
        &mut self,
        render_root: &mut RenderRoot,
        widget_id: WidgetId,
        toggled: bool,
    ) -> bool {
        let switch_id = render_root.get_widget_with_tag(self.switch).unwrap().id();
        if widget_id != switch_id {
            return false;
        }

        render_root.edit_widget_with_tag(self.state_label, |mut label| {
            Label::set_text(&mut label, format!("On: {toggled}"));
        });
        render_root.edit_widget_with_tag(self.switch, |mut switch| {
            Switch::set_on(&mut switch, toggled);
        });
        true
    }
}
