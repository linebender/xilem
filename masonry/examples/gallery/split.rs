// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::core::{NewWidget, PropertySet, StyleProperty, Widget};
use masonry::layout::AsUnit as _;
use masonry::peniko::Color;
use masonry::properties::{Background, Padding};
use masonry::widgets::{Label, SizedBox, Split};

use crate::demo::{DemoPage, ShellTags, wrap_in_shell};

pub(crate) struct SplitDemo {
    shell: ShellTags,
}

impl SplitDemo {
    pub(crate) fn new(shell: ShellTags) -> Self {
        Self { shell }
    }
}

impl DemoPage for SplitDemo {
    fn name(&self) -> &'static str {
        "Split"
    }

    fn shell_tags(&self) -> ShellTags {
        self.shell
    }

    fn build(&self) -> NewWidget<dyn Widget> {
        let left = NewWidget::new_with_props(
            SizedBox::new(
                Label::new("Left pane (drag the divider)")
                    .with_style(StyleProperty::FontSize(14.0))
                    .with_auto_id(),
            ),
            PropertySet::new()
                .with(Background::Color(Color::from_rgb8(0x1f, 0x2a, 0x44)))
                .with(Padding::all(12.0)),
        );

        let right = NewWidget::new_with_props(
            SizedBox::new(
                Label::new("Right pane")
                    .with_style(StyleProperty::FontSize(14.0))
                    .with_auto_id(),
            ),
            PropertySet::new()
                .with(Background::Color(Color::from_rgb8(0x2b, 0x3c, 0x2f)))
                .with(Padding::all(12.0)),
        );

        let body = SizedBox::new(Split::new(left, right).split_fraction(0.33).with_auto_id())
            .height(260.0.px());

        wrap_in_shell(self.shell, NewWidget::new(body).erased())
    }
}
