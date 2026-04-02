// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::app::RenderRoot;
use masonry::core::{
    NewWidget, PropertyStack, Selector, StyleProperty, Widget, WidgetId, WidgetOptions, WidgetTag,
};
use masonry::layout::AsUnit;
use masonry::peniko::color::AlphaColor;
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::{
    Background, BackwardColor, BorderColor, BorderWidth, ContentColor, CornerRadius, Dimensions,
    ForwardColor, HeatColor, StepInputStyle,
};
use masonry::theme::DISABLED_TEXT_COLOR;
use masonry::widgets::{Button, Flex, Label, StepInput};

use crate::demo::{DemoPage, ShellTags, wrap_in_shell};

const BALANCE_TOTAL: isize = 50_000;

pub(crate) struct StepInputDemo {
    shell: ShellTags,

    initialized: bool,
    tag_balance: WidgetTag<Button>,
    tag_left: WidgetTag<StepInput<isize>>,
    tag_right: WidgetTag<StepInput<isize>>,
    tag_custom: WidgetTag<StepInput<u16>>,
}

impl StepInputDemo {
    pub(crate) fn new(shell: ShellTags) -> Self {
        let tag_balance = WidgetTag::unique();
        let tag_left = WidgetTag::unique();
        let tag_right = WidgetTag::unique();
        let tag_custom = WidgetTag::unique();
        Self {
            shell,
            initialized: false,
            tag_balance,
            tag_left,
            tag_right,
            tag_custom,
        }
    }
}

fn desc(text: &str) -> NewWidget<Label> {
    Label::new(text)
        .with_style(StyleProperty::FontSize(14.0))
        .with_props(Dimensions::width(250.px()))
}

impl DemoPage for StepInputDemo {
    fn name(&self) -> &'static str {
        "Step input"
    }

    fn shell_tags(&self) -> ShellTags {
        self.shell
    }

    fn on_selected(&mut self, render_root: &mut RenderRoot) {
        if self.initialized {
            return;
        }

        let mut stack = PropertyStack::new();

        stack.push(
            Selector::new(),
            (BorderWidth::all(2.), CornerRadius::all(20.)),
        );

        stack.push(
            Selector::new(),
            (
                Background::Color(AlphaColor::from_rgb8(0xf2, 0xf4, 0xf8)),
                ContentColor::new(AlphaColor::from_rgb8(0x3e, 0x4b, 0x6c)),
                BackwardColor::new(AlphaColor::from_rgb8(0xab, 0x24, 0x24)),
                ForwardColor::new(AlphaColor::from_rgb8(0x0b, 0x67, 0x43)),
                HeatColor::new(AlphaColor::from_rgb8(0xff, 0x6c, 0x00)),
                BorderColor::new(AlphaColor::from_rgba8(0xf2, 0xf4, 0xf8, 0x7f)),
            ),
        );
        stack.push(
            Selector::new().with_disabled(false).with_hovered(true),
            BorderColor::new(AlphaColor::from_rgb8(0xf2, 0xf4, 0xf8)),
        );
        stack.push(
            Selector::new().with_disabled(false).with_focused(true),
            BorderColor::new(AlphaColor::from_rgb8(0x28, 0x8c, 0xd9)),
        );
        stack.push(
            Selector::new().with_disabled(true),
            (
                ContentColor::new(DISABLED_TEXT_COLOR),
                Background::Color(AlphaColor::from_rgb8(0x00, 0x00, 0x00)),
            ),
        );

        let id = render_root.property_arena().insert(stack);
        render_root.edit_widget_with_tag(self.tag_custom, |mut widget| {
            widget.ctx.set_property_stack(id);
        });

        self.initialized = true;
    }

    fn on_button_press(&mut self, render_root: &mut RenderRoot, widget_id: WidgetId) -> bool {
        let id_balance = render_root
            .get_widget_with_tag(self.tag_balance)
            .unwrap()
            .id();
        if widget_id == id_balance {
            render_root.edit_widget_with_tag(self.tag_left, |mut widget| {
                StepInput::set_base(&mut widget, BALANCE_TOTAL / 2);
            });
            render_root.edit_widget_with_tag(self.tag_right, |mut widget| {
                StepInput::set_base(&mut widget, BALANCE_TOTAL / 2);
            });
            true
        } else {
            false
        }
    }

    fn on_step(&mut self, render_root: &mut RenderRoot, widget_id: WidgetId, value: isize) -> bool {
        let id_left = render_root.get_widget_with_tag(self.tag_left).unwrap().id();
        let id_right = render_root
            .get_widget_with_tag(self.tag_right)
            .unwrap()
            .id();

        if widget_id == id_left {
            render_root.edit_widget_with_tag(self.tag_right, |mut widget| {
                StepInput::set_base(&mut widget, BALANCE_TOTAL - value);
            });
            true
        } else if widget_id == id_right {
            render_root.edit_widget_with_tag(self.tag_left, |mut widget| {
                StepInput::set_base(&mut widget, BALANCE_TOTAL - value);
            });
            true
        } else {
            false
        }
    }

    fn build(&self) -> NewWidget<dyn Widget> {
        let main = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(
                Flex::row()
                    .with_fixed(desc("There are two styles"))
                    .with(
                        NewWidget::new_with(
                            StepInput::new(BALANCE_TOTAL / 2, 1, 0, BALANCE_TOTAL),
                            Some(self.tag_left),
                            WidgetOptions::default(),
                            StepInputStyle::Basic,
                        ),
                        3.,
                    )
                    .with(
                        NewWidget::new_with_tag(Button::with_text("Balance"), self.tag_balance),
                        2.,
                    )
                    .with(
                        NewWidget::new_with(
                            StepInput::new(BALANCE_TOTAL / 2, 1, 0, BALANCE_TOTAL),
                            Some(self.tag_right),
                            WidgetOptions::default(),
                            StepInputStyle::Flow,
                        ),
                        3.,
                    )
                    .with_auto_id(),
            )
            .with_fixed(
                Flex::row()
                    .with_fixed(desc("Step size 100"))
                    .with(
                        StepInput::new(0, 100, i16::MIN, i16::MAX).with_props(StepInputStyle::Flow),
                        1.,
                    )
                    .with_auto_id(),
            )
            .with_fixed(
                Flex::row()
                    .with_fixed(desc("Step size 0.01, Snap 2.0 (Hold Ctrl/Cmd)"))
                    .with(
                        StepInput::new(50., 0.01, 0., 100.)
                            .with_snap(2.)
                            .with_props(StepInputStyle::Flow),
                        1.,
                    )
                    .with_auto_id(),
            )
            .with_fixed(
                Flex::row()
                    .with_fixed(desc("Hold Shift for 10x, Alt for 0.1x speed"))
                    .with(
                        StepInput::new(0, 1, i32::MIN, i32::MAX).with_props(StepInputStyle::Flow),
                        1.,
                    )
                    .with_auto_id(),
            )
            .with_fixed(
                Flex::row()
                    .with_fixed(desc("Supports wrapping"))
                    .with(
                        StepInput::new(0, 1, u8::MIN, u8::MAX)
                            .with_wrap(true)
                            .with_props(StepInputStyle::Flow),
                        1.,
                    )
                    .with_auto_id(),
            )
            .with_fixed(
                Flex::row()
                    .with_fixed(desc("Supports custom display"))
                    .with(
                        StepInput::new(16000, 1000, 0, u32::MAX)
                            .with_display(|ctx| {
                                if ctx.value >= 1_000_000 {
                                    let value = ctx.value / 1_000_000;
                                    format!("{value}M")
                                } else if ctx.value >= 1_000 {
                                    let value = ctx.value / 1_000;
                                    format!("{value}K")
                                } else {
                                    format!("{}", ctx.value)
                                }
                            })
                            .with_props(StepInputStyle::Flow),
                        1.,
                    )
                    .with_auto_id(),
            )
            .with_fixed(
                Flex::row()
                    .with_fixed(desc("Visuals can be customized"))
                    .with(
                        NewWidget::new_with(
                            StepInput::new(u16::MAX / 2, 1, 0, u16::MAX),
                            Some(self.tag_custom),
                            WidgetOptions::default(),
                            StepInputStyle::Flow,
                        )
                        .with_class("custom"),
                        1.,
                    )
                    .with_auto_id(),
            );

        wrap_in_shell(self.shell, NewWidget::new(main).erased())
    }
}
