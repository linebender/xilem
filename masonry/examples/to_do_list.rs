// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! This is a very small example of how to setup a masonry application.
//! It does the almost bare minimum while still being useful.

// On Windows platform, don't show a console when opening the app.
#![cfg_attr(not(test), windows_subsystem = "windows")]

use masonry::core::{ErasedAction, NewWidget, Properties, Widget, WidgetId, WidgetTag};
use masonry::dpi::LogicalSize;
use masonry::peniko::color::AlphaColor;
use masonry::properties::Padding;
use masonry::properties::types::Length;
use masonry::theme::default_property_set;
use masonry::widgets::{Button, ButtonPress, Flex, Label, Portal, TextAction, TextArea, TextInput};
use masonry_winit::app::{AppDriver, DriverCtx, NewWindow, WindowId};

const TEXT_INPUT_TAG: WidgetTag<TextInput> = WidgetTag::new("text-input");
const LIST_TAG: WidgetTag<Flex> = WidgetTag::new("list");
const WIDGET_SPACING: Length = Length::const_px(5.0);

struct Driver {
    next_task: String,
    window_id: WindowId,
}

impl AppDriver for Driver {
    fn on_action(
        &mut self,
        window_id: WindowId,
        ctx: &mut DriverCtx<'_, '_>,
        _widget_id: WidgetId,
        action: ErasedAction,
    ) {
        debug_assert_eq!(window_id, self.window_id, "unknown window");

        if action.is::<ButtonPress>() {
            let render_root = ctx.render_root(window_id);

            render_root.edit_widget_with_tag(TEXT_INPUT_TAG, |mut text_input| {
                let mut text_area = TextInput::text_mut(&mut text_input);
                TextArea::reset_text(&mut text_area, "");
            });
            render_root.edit_widget_with_tag(LIST_TAG, |mut list| {
                let child = Label::new(self.next_task.clone()).with_auto_id();
                Flex::add_child(&mut list, child);
            });
        } else if action.is::<TextAction>() {
            let action = action.downcast::<TextAction>().unwrap();
            match *action {
                TextAction::Changed(new_text) => {
                    self.next_task = new_text.clone();
                }
                TextAction::Entered(_) => {}
            }
        }
    }
}

/// Return initial to-do-list without items.
pub fn make_widget_tree() -> NewWidget<impl Widget> {
    let text_input = NewWidget::new_with_tag(
        TextInput::new("").with_placeholder("ex: 'Do the dishes', 'File my taxes', ..."),
        TEXT_INPUT_TAG,
    );
    let button = NewWidget::new(Button::with_text("Add task"));

    let list = Flex::column()
        .with_child(NewWidget::new_with_props(
            Flex::row()
                .with_flex_child(text_input, 1.0)
                .with_child(button),
            Properties::new().with(Padding::all(WIDGET_SPACING.get())),
        ))
        .with_spacer(WIDGET_SPACING);

    NewWidget::new(Portal::new(NewWidget::new_with_tag(list, LIST_TAG)))
}

fn main() {
    let window_size = LogicalSize::new(400.0, 400.0);
    let window_attributes = masonry_winit::winit::window::WindowAttributes::default()
        .with_title("To-do list")
        .with_resizable(true)
        .with_min_surface_size(window_size);
    let driver = Driver {
        next_task: String::new(),
        window_id: WindowId::next(),
    };

    let (event_sender, event_receiver) =
        std::sync::mpsc::channel::<masonry_winit::app::MasonryUserEvent>();

    let event_loop = masonry_winit::app::EventLoop::builder().build().unwrap();
    masonry_winit::app::run_with(
        event_loop,
        event_sender,
        event_receiver,
        vec![
            NewWindow::new_with_id(
                driver.window_id,
                window_attributes,
                make_widget_tree().erased(),
            )
            .with_base_color(AlphaColor::from_rgb8(2, 6, 23)),
        ],
        driver,
        default_property_set(),
    )
    .unwrap();
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use masonry_testing::{TestHarness, assert_render_snapshot};

    use super::*;

    #[test]
    fn screenshot_test() {
        let mut harness = TestHarness::create(default_property_set(), make_widget_tree());

        assert_render_snapshot!(harness, "example_to_do_list_initial");

        // TODO - Test clicking buttons
        // TODO - Test typing text
    }
}
