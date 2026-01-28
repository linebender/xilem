// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::app::RenderRoot;
use masonry::core::{NewWidget, StyleProperty, Widget, WidgetId, WidgetTag};
use masonry::layers::Tooltip;
use masonry::properties::types::CrossAxisAlignment;
use masonry::vello::kurbo::Point;
use masonry::widgets::{Button, Flex, Label};

use crate::demo::{CONTENT_GAP, DemoPage, ShellTags, wrap_in_shell};

pub(crate) struct TooltipDemo {
    shell: ShellTags,
    show_btn: WidgetTag<Button>,
}

impl TooltipDemo {
    pub(crate) fn new(shell: ShellTags) -> Self {
        Self {
            shell,
            show_btn: WidgetTag::unique(),
        }
    }
}

impl DemoPage for TooltipDemo {
    fn name(&self) -> &'static str {
        "Tooltip"
    }

    fn shell_tags(&self) -> ShellTags {
        self.shell
    }

    fn build(&self) -> NewWidget<dyn Widget> {
        let show = NewWidget::new_with_tag(Button::with_text("Show tooltip"), self.show_btn);

        let body = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(
                Label::new("Click the button to create a tooltip layer.")
                    .with_style(StyleProperty::FontSize(14.0))
                    .with_auto_id(),
            )
            .with_fixed_spacer(CONTENT_GAP)
            .with_fixed(show);

        wrap_in_shell(self.shell, NewWidget::new(body).erased())
    }

    fn on_button_press(&mut self, render_root: &mut RenderRoot, widget_id: WidgetId) -> bool {
        let id = render_root.get_widget_with_tag(self.show_btn).unwrap().id();
        if widget_id != id {
            return false;
        }

        let tooltip = NewWidget::new(Tooltip::new(
            Label::new("Hello from a tooltip layer!")
                .with_style(StyleProperty::FontSize(14.0))
                .with_auto_id(),
        ));
        render_root.add_layer(tooltip.erased(), Point::new(320.0, 120.0));
        true
    }
}
