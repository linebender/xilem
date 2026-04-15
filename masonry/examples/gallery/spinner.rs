// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::core::{NewWidget, Widget};
use masonry::layout::AsUnit as _;
use masonry::properties::types::CrossAxisAlignment;
use masonry::widgets::{Flex, SizedBox, Spinner};

use crate::demo::{DemoPage, ShellTags, wrap_in_shell};

pub(crate) struct SpinnerDemo {
    shell: ShellTags,
}

impl SpinnerDemo {
    pub(crate) fn new(shell: ShellTags) -> Self {
        Self { shell }
    }
}

impl DemoPage for SpinnerDemo {
    fn name(&self) -> &'static str {
        "Spinner"
    }

    fn shell_tags(&self) -> ShellTags {
        self.shell
    }

    fn build(&self) -> NewWidget<dyn Widget> {
        let body = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .with_fixed(
                SizedBox::new(Spinner::new().with_auto_id())
                    .size(80.0.px(), 80.0.px())
                    .with_auto_id(),
            );

        wrap_in_shell(self.shell, NewWidget::new(body).erased())
    }
}
