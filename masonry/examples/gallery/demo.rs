// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::app::RenderRoot;
use masonry::core::{NewWidget, Widget, WidgetId, WidgetTag};
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

    // TODO - Replace with "on_action" method?

    fn on_button_press(&mut self, _render_root: &mut RenderRoot, _widget_id: WidgetId) -> bool {
        false
    }

    fn on_radio_button_selected(
        &mut self,
        _render_root: &mut RenderRoot,
        _widget_id: WidgetId,
    ) -> bool {
        false
    }

    fn on_checkbox_toggled(
        &mut self,
        _render_root: &mut RenderRoot,
        _widget_id: WidgetId,
        _toggled: bool,
    ) -> bool {
        false
    }

    fn on_slider_value(
        &mut self,
        _render_root: &mut RenderRoot,
        _widget_id: WidgetId,
        _value: f64,
    ) -> bool {
        false
    }

    fn on_switch_toggled(
        &mut self,
        _render_root: &mut RenderRoot,
        _widget_id: WidgetId,
        _toggled: bool,
    ) -> bool {
        false
    }
}

pub(crate) fn wrap_in_shell(
    shell: ShellTags,
    body: NewWidget<dyn Widget>,
) -> NewWidget<dyn Widget> {
    let disabled_toggle =
        NewWidget::new_with_tag(Checkbox::new(false, "Disabled"), shell.disabled_toggle);

    let header = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_fixed(disabled_toggle);

    let content = NewWidget::new_with_tag(SizedBox::new(body), shell.content_wrapper);

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
    NewWidget::new_with_tag(Button::with_text(name), tag)
}
