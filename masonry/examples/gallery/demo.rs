// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, Handled, NewWidget, Widget, WidgetId, WidgetTag};
use masonry::layout::Length;
use masonry::properties::types::CrossAxisAlignment;
use masonry::widgets::{Button, Checkbox, Flex, SizedBox};

pub(crate) const CONTENT_GAP: Length = Length::const_px(10.0);
pub(crate) const SIDEBAR_GAP: Length = Length::const_px(2.0);

#[derive(Clone, Copy)]
pub(crate) struct ShellTags {
    pub disabled_toggle: WidgetTag<Checkbox>,
    pub content_wrapper: WidgetTag<SizedBox>,
}

pub(crate) trait DemoPage {
    fn name(&self) -> &'static str;
    fn shell_tags(&self) -> ShellTags;
    fn build(&self) -> NewWidget<dyn Widget>;

    fn on_selected(&mut self, _render_root: &mut RenderRoot) {}

    fn on_action(
        &mut self,
        render_root: &mut RenderRoot,
        action: &ErasedAction,
        widget_id: WidgetId,
    ) -> Handled {
        #![expect(unused_variables, reason = "Default impl")]
        Handled::No
    }
}

pub(crate) fn wrap_in_shell(
    shell: ShellTags,
    body: NewWidget<dyn Widget>,
) -> NewWidget<dyn Widget> {
    let disabled_toggle =
        NewWidget::new(Checkbox::new(false, "Disabled")).with_tag(shell.disabled_toggle);

    let header = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_fixed(disabled_toggle);

    let content = NewWidget::new(SizedBox::new(body)).with_tag(shell.content_wrapper);

    NewWidget::new(
        Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(header.with_auto_id())
            .with_fixed_spacer(CONTENT_GAP)
            .with(content, 1.0),
    )
    .erased()
}

pub(crate) fn new_demo_shell_tags() -> ShellTags {
    ShellTags {
        disabled_toggle: WidgetTag::unique(),
        content_wrapper: WidgetTag::unique(),
    }
}

pub(crate) fn build_sidebar_button(
    tag: WidgetTag<Button>,
    name: &'static str,
) -> NewWidget<Button> {
    NewWidget::new(Button::with_text(name)).with_tag(tag)
}
