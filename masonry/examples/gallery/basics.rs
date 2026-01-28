// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::core::{NewWidget, StyleProperty, Widget};
use masonry::layout::Length;
use masonry::properties::types::CrossAxisAlignment;
use masonry::widgets::{Button, Flex, Label};

use crate::demo::{DemoPage, ShellTags, wrap_in_shell};

pub(crate) struct BasicsDemo {
    shell: ShellTags,
}

impl BasicsDemo {
    pub(crate) fn new(shell: ShellTags) -> Self {
        Self { shell }
    }
}

impl DemoPage for BasicsDemo {
    fn name(&self) -> &'static str {
        "Basics: label + buttons"
    }

    fn shell_tags(&self) -> ShellTags {
        self.shell
    }

    fn build(&self) -> NewWidget<dyn Widget> {
        let body = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(
                Label::new("This is a Masonry widget gallery.")
                    .with_style(StyleProperty::FontSize(18.0))
                    .with_auto_id(),
            )
            .with_fixed_spacer(Length::const_px(12.0))
            .with_fixed(Button::with_text("A button").with_auto_id())
            .with_fixed_spacer(Length::const_px(6.0))
            .with_fixed(Button::with_text("Another button").with_auto_id());

        wrap_in_shell(self.shell, NewWidget::new(body).erased())
    }
}
