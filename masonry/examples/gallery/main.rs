// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A simple widget gallery for Masonry.
//!
//! This example demonstrates a common Masonry app architecture:
//! - A static widget tree built up-front.
//! - An [`AppDriver`] that responds to widget actions and mutates the tree.

// On Windows platform, don't show a console when opening the app.
#![cfg_attr(not(test), windows_subsystem = "windows")]

mod badge;
mod basics;
mod checkbox;
mod demo;
mod divider;
mod image;
mod kitchen_sink;
mod pagination;
mod progress;
mod radio_buttons;
mod slider;
mod spinner;
mod split;
mod step_input;
mod svg;
mod switch;
mod text_input;
mod tooltip;
mod transforms;

use masonry::core::{ErasedAction, NewWidget, StyleProperty, Widget as _, WidgetId, WidgetTag};
use masonry::dpi::LogicalSize;
use masonry::layout::Length;
use masonry::parley::style::FontWeight;
use masonry::properties::Padding;
use masonry::properties::types::CrossAxisAlignment;
use masonry::theme::default_property_set;
use masonry::widgets::{
    Button, ButtonPress, Checkbox, CheckboxToggled, Flex, IndexedStack, Label, Portal, SizedBox,
};
use masonry_winit::app::{AppDriver, DriverCtx, NewWindow, WindowId};
use masonry_winit::winit::window::Window;

use crate::demo::{DemoPage, new_demo_shell_tags};
use crate::switch::SwitchDemo;

const SIDEBAR_WIDTH: Length = Length::const_px(240.0);
const SIDEBAR_SCROLLBAR_INSET: Length = Length::const_px(12.0);
const LEFT_PANE_TOP_PADDING: Length = Length::const_px(12.0);
const LEFT_PANE_LEFT_PADDING: Length = Length::const_px(12.0);
const RIGHT_PANE_PADDING: Length = Length::const_px(12.0);

const DEMO_TITLE_FONT_SIZE: f32 = 20.0;

struct Driver {
    window_id: WindowId,
    selected_demo: usize,
    demo_disabled: Vec<bool>,
    sidebar_buttons: Vec<WidgetTag<Button>>,
    title_tag: WidgetTag<Label>,
    stack_tag: WidgetTag<IndexedStack>,
    demos: Vec<Box<dyn DemoPage>>,
}

impl Driver {
    fn apply_demo_disabled(
        &mut self,
        ctx: &mut DriverCtx<'_, '_>,
        window_id: WindowId,
        demo_idx: usize,
        disabled: bool,
    ) {
        let shell = self.demos[demo_idx].shell_tags();

        let (app_ctx, render_root) = ctx.render_root(window_id);
        render_root.edit_widget_with_tag(app_ctx, shell.disabled_toggle, |mut checkbox| {
            Checkbox::set_checked(&mut checkbox, disabled);
        });
        render_root.edit_widget_with_tag(app_ctx, shell.content_wrapper, |mut content| {
            content.ctx.set_disabled(disabled);
        });
    }

    fn select_demo(&mut self, ctx: &mut DriverCtx<'_, '_>, window_id: WindowId, idx: usize) {
        if idx >= self.demos.len() || idx == self.selected_demo {
            return;
        }

        self.selected_demo = idx;

        {
            let name = self.demos[idx].name();
            let (app_ctx, render_root) = ctx.render_root(window_id);
            render_root.edit_widget_with_tag(app_ctx, self.title_tag, |mut label| {
                Label::set_text(&mut label, name);
            });
            render_root.edit_widget_with_tag(app_ctx, self.stack_tag, |mut stack| {
                IndexedStack::set_active_child(&mut stack, idx);
            });
        }

        let disabled = self.demo_disabled[idx];
        self.apply_demo_disabled(ctx, window_id, idx, disabled);

        {
            let (app_ctx, render_root) = ctx.render_root(window_id);
            self.demos[idx].on_selected(app_ctx, render_root);
        }
    }
}

impl AppDriver for Driver {
    fn on_action(
        &mut self,
        window_id: WindowId,
        ctx: &mut DriverCtx<'_, '_>,
        widget_id: WidgetId,
        action: ErasedAction,
    ) {
        debug_assert_eq!(window_id, self.window_id, "unknown window");

        let (app_ctx, render_root) = ctx.render_root(window_id);

        // Sidebar button pressed: select demo.
        if action.is::<ButtonPress>() {
            // Sidebar: match by tagged ids.
            let selected_idx = {
                self.sidebar_buttons
                    .iter()
                    .enumerate()
                    .find_map(|(idx, tag)| {
                        let id = render_root.get_widget_with_tag(*tag).unwrap().id();
                        (id == widget_id).then_some(idx)
                    })
            };

            if let Some(idx) = selected_idx {
                self.select_demo(ctx, window_id, idx);
                return;
            }
        }

        // Per-demo "Disabled" checkbox toggle.
        if let Some(toggled) = action.downcast_ref::<CheckboxToggled>() {
            let toggled = toggled.0;
            let disabled_demo = {
                self.demos.iter().enumerate().find_map(|(idx, demo)| {
                    let shell = demo.shell_tags();
                    let id = render_root
                        .get_widget_with_tag(shell.disabled_toggle)
                        .unwrap()
                        .id();
                    (id == widget_id).then_some(idx)
                })
            };

            if let Some(idx) = disabled_demo {
                self.demo_disabled[idx] = toggled;
                self.apply_demo_disabled(ctx, window_id, idx, toggled);
                return;
            }
        }

        // Forward everything else to demos.
        for demo in &mut self.demos {
            if demo
                .on_action(app_ctx, render_root, &action, widget_id)
                .is_handled()
            {
                return;
            }
        }
    }
}

fn build_demos() -> Vec<Box<dyn DemoPage>> {
    let mut demos: Vec<Box<dyn DemoPage>> = vec![
        Box::new(basics::BasicsDemo::new(new_demo_shell_tags())),
        Box::new(badge::BadgeDemo::new(new_demo_shell_tags())),
        Box::new(checkbox::CheckboxDemo::new(new_demo_shell_tags())),
        Box::new(divider::DividerDemo::new(new_demo_shell_tags())),
        Box::new(image::ImageDemo::new(new_demo_shell_tags())),
        Box::new(kitchen_sink::KitchenSinkDemo::new(new_demo_shell_tags())),
        Box::new(pagination::PaginationDemo::new(new_demo_shell_tags())),
        Box::new(progress::ProgressDemo::new(new_demo_shell_tags())),
        Box::new(radio_buttons::RadioButtonsDemo::new(new_demo_shell_tags())),
        Box::new(slider::SliderDemo::new(new_demo_shell_tags())),
        Box::new(spinner::SpinnerDemo::new(new_demo_shell_tags())),
        Box::new(split::SplitDemo::new(new_demo_shell_tags())),
        Box::new(step_input::StepInputDemo::new(new_demo_shell_tags())),
        Box::new(svg::SvgDemo::new(new_demo_shell_tags())),
        Box::new(SwitchDemo::new(new_demo_shell_tags())),
        Box::new(text_input::TextInputDemo::new(new_demo_shell_tags())),
        Box::new(tooltip::TooltipDemo::new(new_demo_shell_tags())),
        Box::new(transforms::TransformsDemo::new(new_demo_shell_tags())),
    ];
    demos.sort_by_key(|d| d.name());
    demos
}

fn main() {
    let demos = build_demos();

    // Sidebar button tags (one per demo).
    let sidebar_buttons: Vec<WidgetTag<Button>> =
        (0..demos.len()).map(|_| WidgetTag::unique()).collect();

    let list = {
        let mut column = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch);
        for (idx, demo) in demos.iter().enumerate() {
            if idx != 0 {
                column = column.with_fixed_spacer(demo::SIDEBAR_GAP);
            }
            column = column.with_fixed(demo::build_sidebar_button(
                sidebar_buttons[idx],
                demo.name(),
            ));
        }
        column
    };

    // Padding so the first item isn't flush with the window, and a right inset so an overlay
    // scrollbar doesn't sit on top of the buttons.
    let list = NewWidget::new(SizedBox::new(list.prepare())).with_props(Padding {
        top: LEFT_PANE_TOP_PADDING,
        bottom: Length::ZERO,
        left: LEFT_PANE_LEFT_PADDING,
        right: SIDEBAR_SCROLLBAR_INSET,
    });

    let sidebar =
        SizedBox::new(Portal::new(list).constrain_horizontal(true).prepare()).width(SIDEBAR_WIDTH);

    let stack = demos
        .iter()
        .fold(IndexedStack::new(), |stack, demo| stack.with(demo.build()));

    let title_tag = WidgetTag::unique();
    let stack_tag = WidgetTag::unique();

    let right_panel = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(
            NewWidget::new(
                Label::new(demos[0].name())
                    .with_style(StyleProperty::FontSize(DEMO_TITLE_FONT_SIZE))
                    .with_style(StyleProperty::FontWeight(FontWeight::BOLD)),
            )
            .with_tag(title_tag),
        )
        .with(NewWidget::new(stack).with_tag(stack_tag), 1.0);

    let right_panel = NewWidget::new(SizedBox::new(right_panel.prepare()))
        .with_props(Padding::all(RIGHT_PANE_PADDING));

    let root = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(sidebar.prepare())
        .with(right_panel, 1.0);

    let driver = Driver {
        window_id: WindowId::next(),
        selected_demo: 0,
        demo_disabled: vec![false; demos.len()],
        sidebar_buttons,
        title_tag,
        stack_tag,
        demos,
    };

    let window_size = LogicalSize::new(900.0, 600.0);
    let window_attributes = Window::default_attributes()
        .with_title("Masonry Gallery")
        .with_resizable(true)
        .with_min_inner_size(window_size);

    masonry_winit::app::run(
        vec![NewWindow::new_with_id(
            driver.window_id,
            window_attributes,
            NewWidget::new(root).erased(),
        )],
        driver,
        default_property_set(),
    )
    .unwrap();
}
