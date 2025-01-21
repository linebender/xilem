// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Shows how to use a grid layout in Masonry.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use masonry::dpi::LogicalSize;
use masonry::text::StyleProperty;
use masonry::widget::{Button, Grid, GridParams, Prose, RootWidget, SizedBox, TextArea};
use masonry::{Action, AppDriver, Color, DriverCtx, PointerButton, WidgetId};
use parley::layout::Alignment;
use winit::window::Window;

struct Driver {
    grid_spacing: f64,
}

impl AppDriver for Driver {
    fn on_action(&mut self, ctx: &mut DriverCtx<'_>, _widget_id: WidgetId, action: Action) {
        if let Action::ButtonPressed(button) = action {
            if button == PointerButton::Primary {
                self.grid_spacing += 1.0;
            } else if button == PointerButton::Secondary {
                self.grid_spacing -= 1.0;
            } else {
                self.grid_spacing += 0.5;
            }

            ctx.render_root().edit_root_widget(|mut root| {
                let mut root = root.downcast::<RootWidget<Grid>>();
                let mut grid = RootWidget::child_mut(&mut root);
                Grid::set_spacing(&mut grid, self.grid_spacing);
            });
        }
    }
}

fn grid_button(params: GridParams) -> Button {
    Button::new(format!(
        "X: {}, Y: {}, W: {}, H: {}",
        params.x, params.y, params.width, params.height
    ))
}

fn make_grid(grid_spacing: f64) -> Grid {
    let label = SizedBox::new(Prose::from_text_area(
        TextArea::new_immutable("Change spacing by right and left clicking on the buttons")
            .with_style(StyleProperty::FontSize(14.0))
            .with_alignment(Alignment::Middle),
    ))
    .border(Color::from_rgb8(40, 40, 80), 1.0);
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
    let mut main_widget = Grid::with_dimensions(4, 4)
        .with_spacing(grid_spacing)
        .with_child(label, GridParams::new(1, 0, 1, 1));
    for button_input in button_inputs {
        let button = grid_button(button_input);
        main_widget = main_widget.with_child(button, button_input);
    }

    main_widget
}

fn main() {
    let driver = Driver { grid_spacing: 1.0 };
    let main_widget = make_grid(driver.grid_spacing);

    let window_size = LogicalSize::new(800.0, 500.0);
    let window_attributes = Window::default_attributes()
        .with_title("Grid Layout")
        .with_resizable(true)
        .with_min_inner_size(window_size);

    masonry::event_loop_runner::run(
        masonry::event_loop_runner::EventLoop::with_user_event(),
        window_attributes,
        RootWidget::new(main_widget),
        driver,
    )
    .unwrap();
}

// --- MARK: TESTS ---
#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;
    use masonry::assert_render_snapshot;
    use masonry::testing::TestHarness;

    use super::*;

    #[test]
    fn screenshot_test() {
        let mut harness = TestHarness::create(make_grid(1.0));
        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "grid_masonry");

        // TODO - Test clicking buttons
    }
}
