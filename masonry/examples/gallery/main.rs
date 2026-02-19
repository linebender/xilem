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
mod progress;
mod radio_buttons;
mod slider;
mod spinner;
mod split;
mod switch;
mod text_input;
mod tooltip;
mod transforms;

use masonry::core::{ErasedAction, NewWidget, StyleProperty, Widget as _, WidgetId, WidgetTag};
use masonry::dpi::LogicalSize;
use masonry::parley::style::FontWeight;
use masonry::properties::Padding;
use masonry::properties::types::CrossAxisAlignment;
use masonry::theme::default_property_set;
use masonry::widgets::{
    Button, ButtonPress, Checkbox, CheckboxToggled, Flex, IndexedStack, Label, Portal,
    RadioButtonSelected, SizedBox, SwitchToggled,
};
use masonry_winit::app::{AppDriver, DriverCtx, NewWindow, WindowId};
use masonry_winit::winit::window::Window;

use crate::demo::{DemoPage, new_demo_shell_tags};
use crate::switch::SwitchDemo;

const SIDEBAR_WIDTH: masonry::layout::Length = masonry::layout::Length::const_px(240.0);
const SIDEBAR_SCROLLBAR_INSET: f64 = 12.0;
const LEFT_PANE_TOP_PADDING: f64 = 12.0;
const LEFT_PANE_LEFT_PADDING: f64 = 12.0;
const RIGHT_PANE_PADDING: f64 = 12.0;

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

        let render_root = ctx.render_root(window_id);
        render_root.edit_widget_with_tag(shell.disabled_toggle, |mut checkbox| {
            Checkbox::set_checked(&mut checkbox, disabled);
        });
        render_root.edit_widget_with_tag(shell.content_wrapper, |mut content| {
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
            let render_root = ctx.render_root(window_id);
            render_root.edit_widget_with_tag(self.title_tag, |mut label| {
                Label::set_text(&mut label, name);
            });
            render_root.edit_widget_with_tag(self.stack_tag, |mut stack| {
                IndexedStack::set_active_child(&mut stack, idx);
            });
        }

        let disabled = self.demo_disabled[idx];
        self.apply_demo_disabled(ctx, window_id, idx, disabled);

        {
            let render_root = ctx.render_root(window_id);
            self.demos[idx].on_selected(render_root);
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

        // Button presses: either sidebar selection or demo-specific buttons.
        if action.is::<ButtonPress>() {
            // Sidebar: match by tagged ids.
            let selected_idx = {
                let render_root = ctx.render_root(window_id);
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

            // Forward to demos.
            let handled = {
                let render_root = ctx.render_root(window_id);
                self.demos
                    .iter_mut()
                    .any(|demo| demo.on_button_press(render_root, widget_id))
            };

            if handled {
                return;
            }
        }

        // Checkbox toggles: first handle the per-demo "Disabled" toggle, then demo-specific checkboxes.
        let action = match action.downcast::<CheckboxToggled>() {
            Ok(toggled) => {
                let toggled = toggled.0;

                // Demo disabled toggle: identified by tag.
                let disabled_demo = {
                    let render_root = ctx.render_root(window_id);
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

                let handled = {
                    let render_root = ctx.render_root(window_id);
                    self.demos
                        .iter_mut()
                        .any(|demo| demo.on_checkbox_toggled(render_root, widget_id, toggled))
                };

                if handled {
                    return;
                }

                return;
            }
            Err(action) => action,
        };

        // Selected radio button.
        let action = match action.downcast::<RadioButtonSelected>() {
            Ok(_) => {
                let handled = {
                    let render_root = ctx.render_root(window_id);
                    self.demos
                        .iter_mut()
                        .any(|demo| demo.on_radio_button_selected(render_root, widget_id))
                };

                if handled {
                    return;
                }

                return;
            }
            Err(action) => action,
        };

        // Switch toggles.
        let action = match action.downcast::<SwitchToggled>() {
            Ok(toggled) => {
                let toggled = toggled.0;
                let handled = {
                    let render_root = ctx.render_root(window_id);
                    self.demos
                        .iter_mut()
                        .any(|demo| demo.on_switch_toggled(render_root, widget_id, toggled))
                };

                if handled {
                    return;
                }

                return;
            }
            Err(action) => action,
        };

        // Slider values.
        let Ok(value) = action.downcast::<f64>() else {
            return;
        };
        let value = *value;

        let handled = {
            let render_root = ctx.render_root(window_id);
            self.demos
                .iter_mut()
                .any(|demo| demo.on_slider_value(render_root, widget_id, value))
        };

        let _ = handled;
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
        Box::new(progress::ProgressDemo::new(new_demo_shell_tags())),
        Box::new(radio_buttons::RadioButtonsDemo::new(new_demo_shell_tags())),
        Box::new(slider::SliderDemo::new(new_demo_shell_tags())),
        Box::new(spinner::SpinnerDemo::new(new_demo_shell_tags())),
        Box::new(split::SplitDemo::new(new_demo_shell_tags())),
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
    let list = NewWidget::new_with_props(
        SizedBox::new(list.with_auto_id()),
        Padding {
            top: LEFT_PANE_TOP_PADDING,
            bottom: 0.0,
            left: LEFT_PANE_LEFT_PADDING,
            right: SIDEBAR_SCROLLBAR_INSET,
        },
    );

    let sidebar = SizedBox::new(Portal::new(list).constrain_horizontal(true).with_auto_id())
        .width(SIDEBAR_WIDTH);

    let stack = demos
        .iter()
        .fold(IndexedStack::new(), |stack, demo| stack.with(demo.build()));

    let title_tag = WidgetTag::unique();
    let stack_tag = WidgetTag::unique();

    let right_panel = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(NewWidget::new_with_tag(
            Label::new(demos[0].name())
                .with_style(StyleProperty::FontSize(DEMO_TITLE_FONT_SIZE))
                .with_style(StyleProperty::FontWeight(FontWeight::BOLD)),
            title_tag,
        ))
        .with(NewWidget::new_with_tag(stack, stack_tag), 1.0);

    let right_panel = NewWidget::new_with_props(
        SizedBox::new(right_panel.with_auto_id()),
        Padding::all(RIGHT_PANE_PADDING),
    );

    let root = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(sidebar.with_auto_id())
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
