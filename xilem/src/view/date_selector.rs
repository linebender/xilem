use masonry::widget::{Axis, CrossAxisAlignment, MainAxisAlignment};
use time::{Date, Month};

use crate::{view::sized_box, WidgetView};

use super::{button, flex, FlexSpacer};

pub struct DateData {
    pub selected_date: Date,
    pub month: Month,
    pub year: i32,
}

impl DateData {
    pub fn new(selected_date: Date, month: Month, year: i32) -> Self {
        Self {
            selected_date,
            month,
            year,
        }
    }
}

pub fn date(
    selected_date: &mut Date,
    month: &mut Month,
    year: &mut i32,
) -> impl WidgetView<DateData> {
    flex((
        date_controls(month, year),
        date_grid(selected_date, month, year),
    ))
    .direction(Axis::Vertical)
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .main_axis_alignment(MainAxisAlignment::Center)
}

fn date_controls(month: &mut Month, year: &mut i32) -> impl WidgetView<DateData> {
    flex((
        button("<", |data: &mut DateData| {
            data.month = data.month.previous();
        }),
        sized_box(button(format!("{month}"), |_| {})).width(100.),
        button(">", |data: &mut DateData| {
            data.month = data.month.next();
        }),
        FlexSpacer::Fixed(40.),
        button("<", |data: &mut DateData| {
            data.year -= 1;
        }),
        button(format!("{year}"), |_| {}),
        button(">", |data: &mut DateData| {
            data.year += 1;
        }),
    ))
    .direction(Axis::Horizontal)
    .main_axis_alignment(MainAxisAlignment::Center)
}

// The selected_date in the interface is needed to highlight the currently selected date
// It is currently not implemented
fn date_grid(
    _selected_date: &mut Date,
    month: &mut Month,
    year: &mut i32,
) -> impl WidgetView<DateData> {
    const COLUMNS: u8 = 7;
    const ROWS: u8 = 5;
    let mut date = Date::from_calendar_date(*year, *month, 1).unwrap();
    let days_from_monday = date.weekday().number_days_from_monday();

    for _day in 0..days_from_monday {
        date = date.previous_day().unwrap();
    }

    let mut rows = Vec::new();
    for _row in 0..ROWS {
        let mut columns = Vec::new();
        for _column in 0..COLUMNS {
            // Add buttons of each row into columns vec
            let day_number = date.day();
            let date_copy = date;
            columns.push(
                sized_box(button(
                    format!("{day_number}"),
                    move |data: &mut DateData| {
                        // Set the selected_date
                        data.selected_date = date_copy;
                    },
                ))
                .width(50.),
            );
            date = date.next_day().unwrap();
        }
        // Add column vec into flex with horizontal axis
        // Add flex into rows vec
        rows.push(
            flex(columns)
                .direction(Axis::Horizontal)
                .main_axis_alignment(MainAxisAlignment::Center)
                .gap(10.),
        );
    }
    // Add row vec into flex with vertical axis
    flex(rows).direction(Axis::Vertical)
}
