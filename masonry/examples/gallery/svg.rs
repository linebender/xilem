// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, Handled, NewWidget, StyleProperty, Widget, WidgetId};
use masonry::properties::types::CrossAxisAlignment;
use masonry::widgets::{Flex, Label, Svg};
use resvg::usvg::{self, Tree};

use crate::demo::{CONTENT_GAP, DemoPage, ShellTags, wrap_in_shell};

fn tiger() -> Arc<Tree> {
    let svg_bytes = include_bytes!("../assets/tiger.svg");
    let opts = usvg::Options::default();
    Arc::new(Tree::from_data(svg_bytes, &opts).unwrap())
}

pub(crate) struct SvgDemo {
    shell: ShellTags,
}

impl SvgDemo {
    pub(crate) fn new(shell: ShellTags) -> Self {
        Self { shell }
    }
}

impl DemoPage for SvgDemo {
    fn name(&self) -> &'static str {
        "SVG"
    }

    fn shell_tags(&self) -> ShellTags {
        self.shell
    }

    fn build(&self) -> NewWidget<dyn Widget> {
        let svg = Svg::new(tiger()).with_auto_id();

        let body = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(
                Label::new("SVG widget")
                    .with_style(StyleProperty::FontSize(14.0))
                    .with_auto_id(),
            )
            .with_fixed_spacer(CONTENT_GAP)
            .with(svg, 1.0);

        wrap_in_shell(self.shell, NewWidget::new(body).erased())
    }

    fn on_action(
        &mut self,
        _render_root: &mut RenderRoot,
        _action: &ErasedAction,
        _widget_id: WidgetId,
    ) -> Handled {
        Handled::No
    }
}
