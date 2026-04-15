// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::core::{NewWidget, PropertySet, StyleProperty, Widget};
use masonry::layout::{AsUnit as _, UnitPoint};
use masonry::peniko::Color;
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::{Background, Padding};
use masonry::widgets::{Align, ChildAlignment, Flex, Grid, GridParams, Label, SizedBox, ZStack};

use crate::demo::{CONTENT_GAP, DemoPage, ShellTags, wrap_in_shell};

pub(crate) struct KitchenSinkDemo {
    shell: ShellTags,
}

impl KitchenSinkDemo {
    pub(crate) fn new(shell: ShellTags) -> Self {
        Self { shell }
    }
}

impl DemoPage for KitchenSinkDemo {
    fn name(&self) -> &'static str {
        "Kitchen sink layout"
    }

    fn shell_tags(&self) -> ShellTags {
        self.shell
    }

    fn build(&self) -> NewWidget<dyn Widget> {
        let header = Label::new("A few widgets composed together.")
            .with_style(StyleProperty::FontSize(14.0))
            .prepare();

        let grid = Grid::with_dimensions(2, 2)
            .with(Label::new("Grid 0").prepare(), GridParams::new(0, 0, 1, 1))
            .with(Label::new("Grid 1").prepare(), GridParams::new(1, 0, 1, 1))
            .with(Label::new("Grid 2").prepare(), GridParams::new(0, 1, 1, 1))
            .with(Label::new("Grid 3").prepare(), GridParams::new(1, 1, 1, 1));

        let grid = NewWidget::new(SizedBox::new(grid.prepare())).with_props(
            PropertySet::new()
                .with(Background::Color(Color::from_rgb8(0x24, 0x24, 0x24)))
                .with(Padding::all(12.0)),
        );

        let bg = NewWidget::new(SizedBox::empty().size(220.0.px(), 120.0.px())).with_props(
            PropertySet::one(Background::Color(Color::from_rgb8(0x44, 0x22, 0x66))),
        );

        let fg = Align::centered(
            Label::new("ZStack overlay")
                .with_style(StyleProperty::FontSize(14.0))
                .prepare(),
        )
        .prepare();

        let stack = ZStack::new()
            .with(bg, ChildAlignment::ParentAligned)
            .with(fg, ChildAlignment::SelfAligned(UnitPoint::CENTER))
            .prepare();

        let body = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(header)
            .with_fixed_spacer(CONTENT_GAP)
            .with_fixed(grid)
            .with_fixed_spacer(CONTENT_GAP)
            .with_fixed(stack);

        wrap_in_shell(self.shell, NewWidget::new(body).erased())
    }
}
