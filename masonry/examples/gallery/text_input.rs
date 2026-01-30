// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::core::{NewWidget, StyleProperty, Widget};
use masonry::layout::Length;
use masonry::properties::types::CrossAxisAlignment;
use masonry::widgets::{Flex, Label, SizedBox, TextInput};

use crate::demo::{DemoPage, ShellTags, wrap_in_shell};

pub(crate) struct TextInputDemo {
    shell: ShellTags,
}

impl TextInputDemo {
    pub(crate) fn new(shell: ShellTags) -> Self {
        Self { shell }
    }
}

impl DemoPage for TextInputDemo {
    fn name(&self) -> &'static str {
        "Text input"
    }

    fn shell_tags(&self) -> ShellTags {
        self.shell
    }

    fn build(&self) -> NewWidget<dyn Widget> {
        let body = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(
                Label::new("Type in the text box:")
                    .with_style(StyleProperty::FontSize(14.0))
                    .with_auto_id(),
            )
            .with_fixed_spacer(Length::const_px(8.0))
            .with_fixed(
                SizedBox::new(
                    TextInput::new("")
                        .with_placeholder("Hello from Masonry…")
                        .with_auto_id(),
                )
                .height(Length::const_px(40.0))
                .with_auto_id(),
            );

        wrap_in_shell(self.shell, NewWidget::new(body).erased())
    }
}
