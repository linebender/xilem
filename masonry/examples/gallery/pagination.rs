// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::app::RenderRoot;
use masonry::core::{NewWidget, Widget, WidgetId, WidgetTag};
use masonry::widgets::{Flex, Label, Pagination, StepInput};

use crate::demo::{DemoPage, ShellTags, wrap_in_shell};

pub(crate) struct PaginationDemo {
    shell: ShellTags,

    tag_page_count: WidgetTag<StepInput<isize>>,
    tag_page_active: WidgetTag<StepInput<isize>>,
    tag_buttons_start: WidgetTag<StepInput<isize>>,
    tag_buttons_end: WidgetTag<StepInput<isize>>,
    tag_buttons_total: WidgetTag<StepInput<isize>>,
    tag_content: WidgetTag<Label>,
    tag_pagination: WidgetTag<Pagination>,
}

impl PaginationDemo {
    pub(crate) fn new(shell: ShellTags) -> Self {
        let tag_page_count = WidgetTag::unique();
        let tag_page_active = WidgetTag::unique();
        let tag_buttons_start = WidgetTag::unique();
        let tag_buttons_end = WidgetTag::unique();
        let tag_buttons_total = WidgetTag::unique();
        let tag_content = WidgetTag::unique();
        let tag_pagination = WidgetTag::unique();

        Self {
            shell,
            tag_page_count,
            tag_page_active,
            tag_buttons_start,
            tag_buttons_end,
            tag_buttons_total,
            tag_content,
            tag_pagination,
        }
    }
}

impl DemoPage for PaginationDemo {
    fn name(&self) -> &'static str {
        "Pagination"
    }

    fn shell_tags(&self) -> ShellTags {
        self.shell
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "example stepinput is always isize"
    )]
    fn on_step(&mut self, render_root: &mut RenderRoot, widget_id: WidgetId, value: isize) -> bool {
        let id_page_count = render_root
            .get_widget_with_tag(self.tag_page_count)
            .unwrap()
            .id();
        let id_page_active = render_root
            .get_widget_with_tag(self.tag_page_active)
            .unwrap()
            .id();
        let id_buttons_start = render_root
            .get_widget_with_tag(self.tag_buttons_start)
            .unwrap()
            .id();
        let id_buttons_end = render_root
            .get_widget_with_tag(self.tag_buttons_end)
            .unwrap()
            .id();
        let id_buttons_total = render_root
            .get_widget_with_tag(self.tag_buttons_total)
            .unwrap()
            .id();

        if widget_id == id_page_count {
            render_root.edit_widget_with_tag(self.tag_pagination, |mut widget| {
                Pagination::set_page_count(&mut widget, value as usize);
            });
            true
        } else if widget_id == id_page_active {
            render_root.edit_widget_with_tag(self.tag_content, |mut content| {
                Label::set_text(&mut content, format!("Now on page {}", value));
            });
            render_root.edit_widget_with_tag(self.tag_pagination, |mut widget| {
                Pagination::set_active_page(&mut widget, (value - 1) as usize);
            });
            true
        } else if widget_id == id_buttons_start {
            render_root.edit_widget_with_tag(self.tag_pagination, |mut widget| {
                Pagination::set_buttons_start(&mut widget, value as u8);
            });
            true
        } else if widget_id == id_buttons_end {
            render_root.edit_widget_with_tag(self.tag_pagination, |mut widget| {
                Pagination::set_buttons_end(&mut widget, value as u8);
            });
            true
        } else if widget_id == id_buttons_total {
            render_root.edit_widget_with_tag(self.tag_pagination, |mut widget| {
                Pagination::set_buttons_total(&mut widget, value as u8);
            });
            true
        } else {
            false
        }
    }

    fn on_page_change(
        &mut self,
        render_root: &mut RenderRoot,
        widget_id: WidgetId,
        page_idx: usize,
    ) -> bool {
        let id_pagination = render_root
            .get_widget_with_tag(self.tag_pagination)
            .unwrap()
            .id();
        if widget_id != id_pagination {
            return false;
        }

        render_root.edit_widget_with_tag(self.tag_content, |mut content| {
            Label::set_text(&mut content, format!("Now on page {}", page_idx + 1));
        });
        render_root.edit_widget_with_tag(self.tag_page_active, |mut widget| {
            StepInput::set_base(&mut widget, (page_idx + 1).cast_signed());
        });

        true
    }

    fn build(&self) -> NewWidget<dyn Widget> {
        let body = Flex::column()
            .with_fixed(
                Flex::row()
                    .with(Label::new("Total pages").prepare(), 1.)
                    .with(
                        NewWidget::new(StepInput::new(20, 1, 0, 1000))
                            .with_tag(self.tag_page_count),
                        1.,
                    )
                    .prepare(),
            )
            .with_fixed(
                Flex::row()
                    .with(Label::new("Active page").prepare(), 1.)
                    .with(
                        NewWidget::new(StepInput::new(1, 1, 1, 1000))
                            .with_tag(self.tag_page_active),
                        1.,
                    )
                    .prepare(),
            )
            .with_fixed(
                Flex::row()
                    .with(Label::new("Buttons start").prepare(), 1.)
                    .with(
                        NewWidget::new(StepInput::new(1, 1, 0, 255))
                            .with_tag(self.tag_buttons_start),
                        1.,
                    )
                    .prepare(),
            )
            .with_fixed(
                Flex::row()
                    .with(Label::new("Buttons end").prepare(), 1.)
                    .with(
                        NewWidget::new(StepInput::new(1, 1, 0, 255)).with_tag(self.tag_buttons_end),
                        1.,
                    )
                    .prepare(),
            )
            .with_fixed(
                Flex::row()
                    .with(Label::new("Buttons total").prepare(), 1.)
                    .with(
                        NewWidget::new(StepInput::new(9, 1, 0, 255))
                            .with_tag(self.tag_buttons_total),
                        1.,
                    )
                    .prepare(),
            )
            .with_fixed(NewWidget::new(Label::new("Some data here ..")).with_tag(self.tag_content))
            .with_fixed(NewWidget::new(Pagination::new(20)).with_tag(self.tag_pagination));

        wrap_in_shell(self.shell, NewWidget::new(body).erased())
    }
}
