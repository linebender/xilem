// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::core::{NewWidget, StyleProperty, Widget};
use masonry::layout::AsUnit;
use masonry::palette;
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::{ContentColor, Dimensions};
use masonry::widgets::{DashFit, Divider, Flex, Image, Label, Placement};
use vello::kurbo::Cap;

use crate::demo::{DemoPage, ShellTags, wrap_in_shell};
use crate::image::make_image_data;

pub(crate) struct DividerDemo {
    shell: ShellTags,
}

impl DividerDemo {
    pub(crate) fn new(shell: ShellTags) -> Self {
        Self { shell }
    }
}

fn desc(text: &str) -> NewWidget<Label> {
    Label::new(text)
        .with_style(StyleProperty::FontSize(14.0))
        .with_auto_id()
}

impl DemoPage for DividerDemo {
    fn name(&self) -> &'static str {
        "Divider"
    }

    fn shell_tags(&self) -> ShellTags {
        self.shell
    }

    fn build(&self) -> NewWidget<dyn Widget> {
        let divider_fit = |dash_fit: DashFit| {
            Divider::horizontal()
                .thickness(2.px())
                .dash_pattern(&[10.px(), 10.px()])
                .dash_fit(dash_fit)
                .with_props(ContentColor::new(palette::css::DARK_CYAN))
        };
        let divider_label = |text: &str, placement: Placement| {
            Divider::horizontal()
                .label(text)
                .placement(placement)
                .with_auto_id()
        };

        let img = Image::new(make_image_data()).with_props(Dimensions::fixed(30.px(), 30.px()));

        let main = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(desc("It can be simple"))
            .with_fixed(Divider::horizontal().with_auto_id())
            .with_fixed(desc("It can be stylish"))
            .with_fixed(
                Divider::horizontal()
                    .thickness(4.px())
                    .dash_pattern(&[1.px(), 10.px()])
                    .dash_fit(DashFit::Stretch)
                    .cap(Cap::Round)
                    .with_props(ContentColor::new(palette::css::DARK_ORANGE)),
            )
            .with_fixed(desc(
                "It supports multiple dash fitting strategies. Resize the window to see.",
            ))
            .with_fixed(desc("Clip - clips the end edge"))
            .with_fixed(divider_fit(DashFit::Clip))
            .with_fixed(desc("Stretch - expands the middle gaps"))
            .with_fixed(divider_fit(DashFit::Stretch))
            .with_fixed(desc(
                "Start - shows only whole dashes, aligned to the start",
            ))
            .with_fixed(divider_fit(DashFit::Start))
            .with_fixed(desc(
                "Center - shows only whole dashes, aligned to the middle",
            ))
            .with_fixed(divider_fit(DashFit::Center))
            .with_fixed(desc("End - shows only whole dashes, aligned to the end"))
            .with_fixed(divider_fit(DashFit::End))
            .with_fixed(desc("It supports text labels"))
            .with_fixed(divider_label("At the start", Placement::Start))
            .with_fixed(divider_label("In the middle", Placement::Center))
            .with_fixed(divider_label("At the end", Placement::End))
            .with_fixed(desc("It even supports arbitrary content!"))
            .with_fixed(Divider::horizontal().content(img).with_auto_id())
            .with_auto_id();

        let sidebar = Flex::column()
            .with_fixed(desc("🔍"))
            .with_fixed(desc("📂"))
            .with_fixed(desc("💾"))
            .with_fixed(desc("✂️"))
            .with_fixed(desc("🏷️"))
            .with_auto_id();

        let body = Flex::row()
            .with_fixed(sidebar)
            .with_fixed(
                Divider::vertical()
                    .thickness(3.px())
                    .label("👉")
                    .with_props(ContentColor::new(palette::css::DARK_SEA_GREEN)),
            )
            .with(main, 1.0);

        wrap_in_shell(self.shell, NewWidget::new(body).erased())
    }
}
