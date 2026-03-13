// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::app::RenderRoot;
use masonry::core::{NewWidget, PropertySet, StyleProperty, Widget, WidgetId, WidgetTag};
use masonry::layout::Length;
use masonry::parley::style::FontWeight;
use masonry::peniko::Color;
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::{Background, BorderColor, BorderWidth, CornerRadius, Padding};
use masonry::vello::kurbo::Vec2;
use masonry::widgets::{
    Align, Badge, BadgeCountOverflow, BadgePlacement, Badged, Button, Flex, Label, SizedBox,
};

use crate::demo::{DemoPage, ShellTags, wrap_in_shell};

pub(crate) struct BadgeDemo {
    shell: ShellTags,
    count: u32,
    decrement_btn: WidgetTag<Button>,
    increment_btn: WidgetTag<Button>,
    count_label: WidgetTag<Label>,
    inbox_badged: WidgetTag<Badged>,
}

impl BadgeDemo {
    pub(crate) fn new(shell: ShellTags) -> Self {
        Self {
            shell,
            count: 0,
            decrement_btn: WidgetTag::unique(),
            increment_btn: WidgetTag::unique(),
            count_label: WidgetTag::unique(),
            inbox_badged: WidgetTag::unique(),
        }
    }

    fn make_count_badge(count: u32) -> Option<NewWidget<dyn Widget>> {
        if count == 0 {
            return None;
        }
        Some(
            Badge::count_with_overflow(
                count,
                BadgeCountOverflow::Cap {
                    max: 9,
                    show_plus: true,
                },
            )
            .with_auto_id()
            .erased(),
        )
    }

    fn apply_count(&self, render_root: &mut RenderRoot) {
        render_root.edit_widget_with_tag(self.count_label, |mut label| {
            Label::set_text(&mut label, format!("count: {}", self.count));
        });

        render_root.edit_widget_with_tag(self.inbox_badged, |mut badged| {
            if let Some(badge) = Self::make_count_badge(self.count) {
                Badged::set_badge(&mut badged, badge);
            } else {
                Badged::clear_badge(&mut badged);
            }
        });
    }
}

impl DemoPage for BadgeDemo {
    fn name(&self) -> &'static str {
        "Badge"
    }

    fn shell_tags(&self) -> ShellTags {
        self.shell
    }

    fn build(&self) -> NewWidget<dyn Widget> {
        const GAP: Length = Length::const_px(10.0);
        const SECTION_GAP: Length = Length::const_px(16.0);

        let new_badge = Badge::with_text("New");

        let beta_badge = NewWidget::new_with_props(
            Badge::with_text("Beta"),
            PropertySet::new().with(Background::Color(Color::from_rgb8(0xd9, 0x77, 0x06))),
        );

        let outline_badge = NewWidget::new_with_props(
            Badge::with_text("99+"),
            PropertySet::new()
                .with(Background::Color(Color::TRANSPARENT))
                .with(BorderWidth { width: 1.0 })
                .with(BorderColor {
                    color: Color::from_rgb8(0x71, 0x71, 0x7a),
                }),
        );

        let inbox = Badged::new(
            Button::with_text("Inbox").with_auto_id(),
            Badge::count(3).with_auto_id(),
        )
        .with_badge_placement(BadgePlacement::TopRight)
        .with_badge_offset(Vec2::new(2.0, -2.0))
        .with_auto_id();

        let inbox_zero = Badged::new_optional(
            Button::with_text("Empty inbox").with_auto_id(),
            Badge::count_nonzero(0).map(|b| b.with_auto_id().erased()),
        )
        .with_badge_placement(BadgePlacement::TopRight)
        .with_badge_offset(Vec2::new(2.0, -2.0))
        .with_auto_id();

        let inbox_overflow = Badged::new(
            Button::with_text("Big inbox").with_auto_id(),
            Badge::count(120).with_auto_id(),
        )
        .with_badge_placement(BadgePlacement::TopRight)
        .with_badge_offset(Vec2::new(2.0, -2.0))
        .with_auto_id();

        let interactive_inbox = NewWidget::new_with_tag(
            Badged::new_optional(
                Button::with_text("Interactive inbox").with_auto_id(),
                Self::make_count_badge(self.count),
            )
            .with_badge_placement(BadgePlacement::TopRight)
            .with_badge_offset(Vec2::new(2.0, -2.0)),
            self.inbox_badged,
        );

        let decrement_btn = NewWidget::new_with_tag(Button::with_text("−"), self.decrement_btn);
        let increment_btn = NewWidget::new_with_tag(Button::with_text("+"), self.increment_btn);
        let count_label = NewWidget::new_with_tag(
            Label::new(format!("count: {}", self.count)).with_style(StyleProperty::FontSize(13.0)),
            self.count_label,
        );

        let avatar = NewWidget::new_with_props(
            SizedBox::new(
                Align::centered(
                    Label::new("AB")
                        .with_style(StyleProperty::FontSize(22.0))
                        .with_style(StyleProperty::FontWeight(FontWeight::BOLD))
                        .with_auto_id(),
                )
                .with_auto_id(),
            )
            .size(Length::const_px(72.0), Length::const_px(72.0)),
            PropertySet::new()
                .with(Background::Color(Color::from_rgb8(0x3f, 0x3f, 0x46)))
                .with(CornerRadius { radius: 999.0 })
                .with(Padding::all(0.0)),
        );

        let online_dot = NewWidget::new_with_props(
            Badge::new(
                SizedBox::empty()
                    .size(Length::const_px(10.0), Length::const_px(10.0))
                    .with_auto_id(),
            ),
            PropertySet::new()
                .with(Padding::all(0.0))
                .with(CornerRadius { radius: 999.0 })
                .with(BorderWidth { width: 0.0 })
                .with(Background::Color(Color::from_rgb8(0x22, 0xc5, 0x5e))),
        );

        let avatar_with_status = Badged::new(avatar.erased(), online_dot.erased())
            .with_badge_placement(BadgePlacement::BottomRight)
            .with_badge_offset(Vec2::new(-2.0, -2.0))
            .with_auto_id();

        let body = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .with_fixed(
                Label::new("Badges are non-interactive, decorative labels.")
                    .with_style(StyleProperty::FontSize(14.0))
                    .with_auto_id(),
            )
            .with_fixed_spacer(GAP)
            .with_fixed(
                Flex::row()
                    .with_fixed(new_badge.with_auto_id())
                    .with_fixed_spacer(GAP)
                    .with_fixed(beta_badge)
                    .with_fixed_spacer(GAP)
                    .with_fixed(outline_badge)
                    .with_auto_id(),
            );

        let body = body
            .with_fixed_spacer(SECTION_GAP)
            .with_fixed(
                Label::new("Decorate other widgets with Badged:")
                    .with_style(StyleProperty::FontSize(14.0))
                    .with_auto_id(),
            )
            .with_fixed_spacer(GAP)
            .with_fixed(
                Flex::row()
                    .cross_axis_alignment(CrossAxisAlignment::Center)
                    .with_fixed(inbox)
                    .with_fixed_spacer(Length::const_px(12.0))
                    .with_fixed(inbox_zero)
                    .with_fixed_spacer(Length::const_px(12.0))
                    .with_fixed(inbox_overflow)
                    .with_fixed_spacer(Length::const_px(18.0))
                    .with_fixed(avatar_with_status)
                    .with_auto_id(),
            );

        let body = body
            .with_fixed_spacer(SECTION_GAP)
            .with_fixed(
                Label::new("Interactive count (0 hides, 10 shows 9+):")
                    .with_style(StyleProperty::FontSize(14.0))
                    .with_auto_id(),
            )
            .with_fixed_spacer(GAP)
            .with_fixed(
                Flex::row()
                    .cross_axis_alignment(CrossAxisAlignment::Center)
                    .with_fixed(decrement_btn)
                    .with_fixed_spacer(Length::const_px(6.0))
                    .with_fixed(increment_btn)
                    .with_fixed_spacer(Length::const_px(10.0))
                    .with_fixed(count_label)
                    .with_fixed_spacer(Length::const_px(18.0))
                    .with_fixed(interactive_inbox)
                    .with_auto_id(),
            );

        wrap_in_shell(self.shell, NewWidget::new(body).erased())
    }

    fn on_selected(&mut self, render_root: &mut RenderRoot) {
        self.apply_count(render_root);
    }

    fn on_button_press(&mut self, render_root: &mut RenderRoot, widget_id: WidgetId) -> bool {
        let dec_id = render_root
            .get_widget_with_tag(self.decrement_btn)
            .unwrap()
            .id();
        let inc_id = render_root
            .get_widget_with_tag(self.increment_btn)
            .unwrap()
            .id();

        if widget_id == dec_id {
            self.count = self.count.saturating_sub(1);
            self.apply_count(render_root);
            return true;
        }

        if widget_id == inc_id {
            self.count = self.count.saturating_add(1);
            self.apply_count(render_root);
            return true;
        }

        false
    }
}
