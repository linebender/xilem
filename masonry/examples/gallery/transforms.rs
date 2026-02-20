// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::app::RenderRoot;
use masonry::core::{
    NewWidget, PropertySet, StyleProperty, Widget, WidgetId, WidgetOptions, WidgetTag,
};
use masonry::layout::AsUnit as _;
use masonry::peniko::Color;
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::{Background, Padding};
use masonry::vello::kurbo::{Affine, Vec2};
use masonry::widgets::{Button, Flex, Label, SizedBox};

use crate::demo::{CONTENT_GAP, DemoPage, SIDEBAR_GAP, ShellTags, wrap_in_shell};

pub(crate) struct TransformsDemo {
    shell: ShellTags,
    state_label: WidgetTag<Label>,
    target: WidgetTag<SizedBox>,
    btn_rotate_left: WidgetTag<Button>,
    btn_rotate_right: WidgetTag<Button>,
    btn_scale_down: WidgetTag<Button>,
    btn_scale_up: WidgetTag<Button>,
    btn_reset: WidgetTag<Button>,
    angle_rad: f64,
    scale: f64,
}

impl TransformsDemo {
    pub(crate) fn new(shell: ShellTags) -> Self {
        Self {
            shell,
            state_label: WidgetTag::unique(),
            target: WidgetTag::unique(),
            btn_rotate_left: WidgetTag::unique(),
            btn_rotate_right: WidgetTag::unique(),
            btn_scale_down: WidgetTag::unique(),
            btn_scale_up: WidgetTag::unique(),
            btn_reset: WidgetTag::unique(),
            angle_rad: 0.0,
            scale: 1.0,
        }
    }

    fn apply(&self, render_root: &mut RenderRoot) {
        let pivot = Vec2::new(80.0, 80.0);
        let transform = Affine::translate(pivot)
            .then_rotate(self.angle_rad)
            .then_scale(self.scale)
            .then_translate(-pivot);

        render_root.edit_widget_with_tag(self.target, |mut target| {
            target.set_transform(transform);
        });
        render_root.edit_widget_with_tag(self.state_label, |mut label| {
            Label::set_text(
                &mut label,
                format!(
                    "angle: {:.0}°   scale: {:.2}",
                    self.angle_rad.to_degrees(),
                    self.scale
                ),
            );
        });
    }

    fn matches_button(
        &self,
        render_root: &mut RenderRoot,
        tag: WidgetTag<Button>,
        widget_id: WidgetId,
    ) -> bool {
        render_root.get_widget_with_tag(tag).unwrap().id() == widget_id
    }
}

impl DemoPage for TransformsDemo {
    fn name(&self) -> &'static str {
        "Transforms"
    }

    fn shell_tags(&self) -> ShellTags {
        self.shell
    }

    fn build(&self) -> NewWidget<dyn Widget> {
        let state = NewWidget::new_with_tag(
            Label::new("angle: 0°   scale: 1.00").with_style(StyleProperty::FontSize(13.0)),
            self.state_label,
        );

        let controls = Flex::row()
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .with_fixed(NewWidget::new_with_tag(
                Button::with_text("⟲"),
                self.btn_rotate_left,
            ))
            .with_fixed_spacer(SIDEBAR_GAP)
            .with_fixed(NewWidget::new_with_tag(
                Button::with_text("⟳"),
                self.btn_rotate_right,
            ))
            .with_fixed_spacer(SIDEBAR_GAP)
            .with_fixed(NewWidget::new_with_tag(
                Button::with_text("−"),
                self.btn_scale_down,
            ))
            .with_fixed_spacer(SIDEBAR_GAP)
            .with_fixed(NewWidget::new_with_tag(
                Button::with_text("+"),
                self.btn_scale_up,
            ))
            .with_fixed_spacer(SIDEBAR_GAP)
            .with_fixed(NewWidget::new_with_tag(
                Button::with_text("Reset"),
                self.btn_reset,
            ));

        let target = NewWidget::new_with(
            SizedBox::new(
                Label::new("Transform me")
                    .with_style(StyleProperty::FontSize(14.0))
                    .with_auto_id(),
            )
            .size(160.0.px(), 160.0.px()),
            Some(self.target),
            WidgetOptions::default(),
            PropertySet::new()
                .with(Background::Color(Color::from_rgb8(0x35, 0x35, 0x35)))
                .with(Padding::all(12.0)),
        );

        let body = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(state)
            .with_fixed_spacer(CONTENT_GAP)
            .with_fixed(controls.with_auto_id())
            .with_fixed_spacer(CONTENT_GAP)
            .with_fixed(target);

        wrap_in_shell(self.shell, NewWidget::new(body).erased())
    }

    fn on_selected(&mut self, render_root: &mut RenderRoot) {
        self.apply(render_root);
    }

    fn on_button_press(&mut self, render_root: &mut RenderRoot, widget_id: WidgetId) -> bool {
        if self.matches_button(render_root, self.btn_rotate_left, widget_id) {
            self.angle_rad -= 15_f64.to_radians();
            self.apply(render_root);
            return true;
        }
        if self.matches_button(render_root, self.btn_rotate_right, widget_id) {
            self.angle_rad += 15_f64.to_radians();
            self.apply(render_root);
            return true;
        }
        if self.matches_button(render_root, self.btn_scale_down, widget_id) {
            self.scale = (self.scale / 1.1).clamp(0.3, 3.0);
            self.apply(render_root);
            return true;
        }
        if self.matches_button(render_root, self.btn_scale_up, widget_id) {
            self.scale = (self.scale * 1.1).clamp(0.3, 3.0);
            self.apply(render_root);
            return true;
        }
        if self.matches_button(render_root, self.btn_reset, widget_id) {
            self.angle_rad = 0.0;
            self.scale = 1.0;
            self.apply(render_root);
            return true;
        }
        false
    }
}
