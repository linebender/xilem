// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Shows how to use a grid layout in Masonry.

// On Windows platform, don't show a console when opening the app.
#![cfg_attr(not(test), windows_subsystem = "windows")]
use masonry::core::{Action, PointerButton, Properties, StyleProperty, WidgetId, WidgetPod};
use masonry::dpi::LogicalSize;
use masonry::parley::layout::Alignment;
use masonry::peniko::Color;
use masonry::properties::{BorderColor, BorderWidth};
use masonry::widgets::{Button, Grid, GridParams, Prose, RootWidget, SizedBox, TextArea};
use masonry_winit::app::{AppDriver, DriverCtx, WindowId};
use winit::window::Window;

struct Driver {
    grid_spacing: f64,
    window_id: WindowId,
}

impl AppDriver for Driver {
    fn on_action(
        &mut self,
        window_id: WindowId,
        ctx: &mut DriverCtx<'_, '_>,
        _widget_id: WidgetId,
        action: Action,
    ) {
        debug_assert_eq!(window_id, self.window_id, "unknown window");

        if let Action::ButtonPressed(button) = action {
            self.grid_spacing += match button {
                Some(PointerButton::Primary) => 1.0,
                Some(PointerButton::Secondary) => -1.0,
                _ => 0.5,
            };

            ctx.render_root(window_id).edit_root_widget(|mut root| {
                let mut root = root.downcast::<RootWidget>();
                let mut grid = RootWidget::child_mut(&mut root);
                let mut grid = grid.downcast::<Grid>();
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
    let label = Prose::from_text_area(
        TextArea::new_immutable("Change spacing by right and left clicking on the buttons")
            .with_style(StyleProperty::FontSize(14.0))
            .with_alignment(Alignment::Middle),
    );
    let label = SizedBox::new(label);

    let props = Properties::new()
        .with(BorderColor::new(Color::from_rgb8(40, 40, 80)))
        .with(BorderWidth::all(1.0));
    let label = SizedBox::new_pod(WidgetPod::new_with(
        Box::new(label),
        WidgetId::next(),
        Default::default(),
        props,
    ));

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
    let driver = Driver {
        grid_spacing: 1.0,
        window_id: WindowId::next(),
    };
    let main_widget = make_grid(driver.grid_spacing);

    let window_size = LogicalSize::new(800.0, 500.0);
    let window_attributes = Window::default_attributes()
        .with_title("Grid Layout")
        .with_resizable(true)
        .with_min_inner_size(window_size);

    masonry_winit::app::run(
        masonry_winit::app::EventLoop::with_user_event(),
        vec![(
            driver.window_id,
            window_attributes,
            Box::new(RootWidget::new(main_widget)),
        )],
        driver,
    )
    .unwrap();
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use masonry::assert_render_snapshot;
    use masonry::testing::TestHarness;
    use masonry::theme::default_property_set;

    use super::*;

    #[test]
    fn screenshot_test() {
        let mut harness =
            TestHarness::create(default_property_set(), RootWidget::new(make_grid(1.0)));
        assert_render_snapshot!(harness, "example_grid_masonry_initial");

        // TODO - Test clicking buttons
    }
}
