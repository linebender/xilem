// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Shows how to use a grid layout in Masonry.

// On Windows platform, don't show a console when opening the app.
#![cfg_attr(not(test), windows_subsystem = "windows")]

use masonry::TextAlign;
use masonry::core::{
    ErasedAction, NewWidget, PointerButton, PropertySet, StyleProperty, Widget as _, WidgetId,
};
use masonry::dpi::LogicalSize;
use masonry::layout::Length;
use masonry::peniko::Color;
use masonry::properties::{BorderColor, BorderWidth, Gap};
use masonry::theme::default_property_set;
use masonry::widgets::{Button, ButtonPress, Grid, GridParams, Prose, SizedBox, TextArea};
use masonry_winit::app::{AppDriver, DriverCtx, NewWindow, WindowId};
use masonry_winit::winit::window::Window;

struct Driver {
    grid_gap: f64,
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
            let button = action.downcast::<ButtonPress>().unwrap().button;
            self.grid_gap += match button {
                Some(PointerButton::Primary) => 1.0,
                Some(PointerButton::Secondary) => -1.0,
                _ => 0.5,
            };

            ctx.render_root(window_id).edit_base_layer(|mut root| {
                let mut grid = root.downcast::<Grid>();
                grid.insert_prop(Gap::new(Length::px(self.grid_gap)));
            });
        }
    }
}

fn grid_button(params: GridParams) -> Button {
    Button::with_text(format!(
        "X: {}, Y: {}, W: {}, H: {}",
        params.x, params.y, params.width, params.height
    ))
}

/// Creates a grid with a bunch of buttons
pub fn make_grid(grid_gap: f64) -> NewWidget<Grid> {
    let label = Prose::from_text_area(
        TextArea::new_immutable("Change spacing by right and left clicking on the buttons")
            .with_style(StyleProperty::FontSize(14.0))
            .with_text_alignment(TextAlign::Center)
            .with_auto_id(),
    );
    let label = SizedBox::new(label.with_auto_id());

    let props = PropertySet::new()
        .with(BorderColor::new(Color::from_rgb8(40, 40, 80)))
        .with(BorderWidth::all(1.0));
    let label = SizedBox::new(NewWidget::new_with_props(label, props));

    let button_inputs = vec![
        GridParams {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        },
        GridParams {
            x: 2,
            y: 0,
            width: 2,
            height: 1,
        },
        GridParams {
            x: 0,
            y: 1,
            width: 1,
            height: 2,
        },
        GridParams {
            x: 1,
            y: 1,
            width: 2,
            height: 2,
        },
        GridParams {
            x: 3,
            y: 1,
            width: 1,
            height: 1,
        },
        GridParams {
            x: 3,
            y: 2,
            width: 1,
            height: 1,
        },
        GridParams {
            x: 0,
            y: 3,
            width: 4,
            height: 1,
        },
    ];

    // Arrange widgets in a 4 by 4 grid.
    let mut main_widget =
        Grid::with_dimensions(4, 4).with(label.with_auto_id(), GridParams::new(1, 0, 1, 1));
    for button_input in button_inputs {
        let button = grid_button(button_input);
        main_widget = main_widget.with(button.with_auto_id(), button_input);
    }

    NewWidget::new_with_props(
        main_widget,
        PropertySet::one(Gap::new(Length::px(grid_gap))),
    )
}

fn main() {
    let driver = Driver {
        grid_gap: 1.0,
        window_id: WindowId::next(),
    };
    let main_widget = make_grid(driver.grid_gap);

    let window_size = LogicalSize::new(800.0, 500.0);
    let window_attributes = Window::default_attributes()
        .with_title("Grid Layout")
        .with_resizable(true)
        .with_min_inner_size(window_size);

    masonry_winit::app::run(
        vec![NewWindow::new_with_id(
            driver.window_id,
            window_attributes,
            main_widget.erased(),
        )],
        driver,
        default_property_set(),
    )
    .unwrap();
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use masonry::theme::default_property_set;
    use masonry_testing::{TestHarness, TestHarnessParams, assert_render_snapshot};

    use super::*;

    #[test]
    fn screenshot_test() {
        let mut test_params = TestHarnessParams::default();
        // This is a screenshot of an example, so it being slightly larger than a normal test is expected.
        test_params.max_screenshot_size = 16 * TestHarnessParams::KIBIBYTE;
        let mut harness =
            TestHarness::create_with(default_property_set(), make_grid(1.0), test_params);
        assert_render_snapshot!(harness, "example_grid_masonry_initial");

        // TODO - Test clicking buttons
    }
}
