use dpi::LogicalSize;
use parley::layout::Alignment;
use taffy::{Dimension, FlexDirection, GridPlacement, LengthPercentage, Line, Rect, Size};
use taffy::Display::{Block, Flex, Grid};
use winit::window::Window;
use masonry::app_driver::{AppDriver, DriverCtx};
use masonry::{Action, Color, Widget, WidgetId};
use masonry::widget::{TaffyLayout, Prose, RootWidget, SizedBox};


struct Driver {
}

impl AppDriver for Driver {
    fn on_action(&mut self, ctx: &mut DriverCtx<'_>, _widget_id: WidgetId, action: Action) {

    }
}

fn get_layout_with_style(style: taffy::Style) -> impl Widget {
    let label1 = SizedBox::new(
        Prose::new("Label 1")
            .with_text_size(14.0)
            .with_text_alignment(Alignment::Middle),
    )
        .border(Color::rgb8(150, 60, 90), 4.0);
    let label2 = SizedBox::new(
        Prose::new("Label 2")
            .with_text_size(10.0)
            .with_text_alignment(Alignment::Middle),
    )
        .border(Color::rgb8(40, 40, 80), 2.0);
    let label3 = SizedBox::new(
        Prose::new("Label 3: This is a long one. It will take up more space.")
            .with_text_size(10.0)
            .with_text_alignment(Alignment::Middle),
    )
        .border(Color::rgb8(20, 230, 80), 1.0);

    TaffyLayout::new(style)
        .with_child(label1, taffy::Style{
            flex_grow: 2.0,
            ..taffy::Style::default()
        })
        .with_child(label2, taffy::Style{
            flex_grow: 1.0,
            ..taffy::Style::default()
        })
        .with_child(label3, taffy::Style{
            flex_grow: 1.0,
            ..taffy::Style::default()
        })
}

fn get_custom_grid() -> impl Widget {
    let label1 = SizedBox::new(
        Prose::new("Label 1")
            .with_text_size(14.0)
            .with_text_alignment(Alignment::Middle),
    )
        .border(Color::rgb8(150, 60, 90), 4.0);
    let label2 = SizedBox::new(
        Prose::new("Label 2")
            .with_text_size(10.0)
            .with_text_alignment(Alignment::Middle),
    )
        .border(Color::rgb8(40, 40, 80), 2.0);
    let label3 = SizedBox::new(
        Prose::new("Label 3: This is a long one. It will take up more space.")
            .with_text_size(10.0)
            .with_text_alignment(Alignment::Middle),
    )
        .border(Color::rgb8(20, 230, 80), 1.0);

    TaffyLayout::new(taffy::Style{
        display: Grid,
        ..taffy::Style::default()
    })
        .with_child(label1, taffy::Style{
            grid_row: Line { start: GridPlacement::Span(1), end: GridPlacement::Span(1) },
            grid_column: Line { start: GridPlacement::Span(1), end: GridPlacement::Span(2) },
            ..taffy::Style::default()
        })
        .with_child(label2, taffy::Style{
            grid_row: Line { start: GridPlacement::Span(2), end: GridPlacement::Span(2) },
            grid_column: Line { start: GridPlacement::Span(1), end: GridPlacement::Span(1) },
            ..taffy::Style::default()
        })
        .with_child(label3, taffy::Style{
            grid_row: Line { start: GridPlacement::Span(2), end: GridPlacement::Span(2) },
            grid_column: Line { start: GridPlacement::Span(2), end: GridPlacement::Span(2) },
            ..taffy::Style::default()
        })
}

pub fn main() {
    let block_layout = get_layout_with_style(taffy::Style{
        display: Block,
        ..taffy::Style::default()
    });

    let flex_row_layout = get_layout_with_style(taffy::Style{
        display: Flex,
        flex_direction: FlexDirection::Row,
        ..taffy::Style::default()
    });

    let flex_col_layout = get_layout_with_style(taffy::Style{
        display: Flex,
        flex_direction: FlexDirection::Column,
        ..taffy::Style::default()
    });

    let grid_layout = get_layout_with_style(taffy::Style{
        display: Grid,
        ..taffy::Style::default()
    });

    let driver = Driver {};

    let mut section_title_style = taffy::Style {
        margin: Rect {
            left: taffy::LengthPercentageAuto::Length(5.0),
            right: taffy::LengthPercentageAuto::Length(5.0),
            top: taffy::LengthPercentageAuto::Length(15.0),
            bottom: taffy::LengthPercentageAuto::Length(5.0),
        },
        ..taffy::Style::default()
    };

    // The empty layout shows how it handles leaf nodes.
    let empty_layout = SizedBox::new(
        TaffyLayout::new(taffy::Style::default()),
    )
        .border(Color::rgb8(90, 90, 100), 5.0);

    let empty_style = taffy::Style{
        max_size: Size{
            width: Dimension::Length(200.0),
            height: Dimension::Length(30.0),
        },
        min_size: Size{
            width: Dimension::Length(100.0),
            height: Dimension::Length(20.0),
        },
        ..taffy::Style::default()
    };

    let mut vertical_flex_style = taffy::Style::default();
    vertical_flex_style.display = Flex;
    vertical_flex_style.flex_direction = FlexDirection::Column;
    let main_vertical_layout = TaffyLayout::new(vertical_flex_style)
        .with_child(Prose::new("Empty With Sizing"), section_title_style.clone())
        .with_child(empty_layout, empty_style)
        .with_child(Prose::new("Block"), section_title_style.clone())
        .with_child(block_layout, taffy::Style::default())
        .with_child(Prose::new("Flex Col"), section_title_style.clone())
        .with_child(flex_col_layout, taffy::Style::default())
        .with_child(Prose::new("Flex Row"), section_title_style.clone())
        .with_child(flex_row_layout, taffy::Style::default())
        .with_child(Prose::new("Default Grid"), section_title_style.clone())
        .with_child(grid_layout, taffy::Style::default())
        .with_child(Prose::new("Custom Grid"), section_title_style.clone())
        .with_child(get_custom_grid(), taffy::Style::default());

    let window_size = LogicalSize::new(800.0, 500.0);
    let window_attributes = Window::default_attributes()
        .with_title("Taffy Layout")
        .with_resizable(true)
        .with_inner_size(window_size);

    masonry::event_loop_runner::run(
        masonry::event_loop_runner::EventLoop::with_user_event(),
        window_attributes,
        RootWidget::new(main_vertical_layout),
        driver,
    )
        .unwrap();
}
