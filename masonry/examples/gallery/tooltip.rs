// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::app::RenderRoot;
use masonry::core::{
    ErasedAction, Handled, NewWidget, StyleProperty, SubwindowPlacement, Widget, WidgetId,
    WidgetTag,
};
use masonry::properties::types::CrossAxisAlignment;
use masonry::subwindows::Tooltip;
use masonry::widgets::{Button, ButtonPress, Flex, Label};

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
        let show = NewWidget::new(Button::with_text("Show tooltip")).with_tag(self.show_btn);

        let body = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(
                Label::new("Click the button to create a tooltip.")
                    .with_style(StyleProperty::FontSize(14.0))
                    .prepare(),
            )
            .with_fixed_spacer(CONTENT_GAP)
            .with_fixed(show);

        wrap_in_shell(self.shell, NewWidget::new(body).erased())
    }

    fn on_action(
        &mut self,
        render_root: &mut RenderRoot,
        action: &ErasedAction,
        widget_id: WidgetId,
    ) -> Handled {
        if !action.is::<ButtonPress>() {
            return Handled::No;
        }

        let button = render_root.get_widget_with_tag(self.show_btn).unwrap();
        if widget_id != button.id() {
            return Handled::No;
        }

        let tooltip = NewWidget::new(Tooltip::new(
            Label::new("Hello from a tooltip!")
                .with_style(StyleProperty::FontSize(14.0))
                .prepare(),
        ));
        let ctx = button.ctx();
        let anchor = ctx.window_transform().transform_rect_bbox(ctx.border_box());
        render_root.add_subwindow(tooltip, SubwindowPlacement::Anchored(anchor));
        Handled::Yes
    }
}
