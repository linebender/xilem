use masonry::widget::{Axis, CrossAxisAlignment, MainAxisAlignment};
use time::{Date, Month, OffsetDateTime};

use crate::{view::sized_box, WidgetView};

use super::{button, flex, FlexSpacer};

pub enum DatePickerMessage {
    Select(Date),
    ChangeView,
    Nop,
}

pub struct DatePicker {
    previous_date: Date,
    month: Month,
    year: i32,
}

impl DatePicker {
    pub fn new(month: Month, year: i32) -> Self {
        let previous_date = OffsetDateTime::now_utc().date();
        Self {
            previous_date,
            month,
            year,
        }
    }

    pub fn view(
        &mut self,
        selected_date: &mut Date,
    ) -> impl WidgetView<DatePicker, DatePickerMessage> {
        if self.previous_date != *selected_date {
            self.month = selected_date.month();
            self.year = selected_date.year();
            self.previous_date = selected_date.clone();
        }
        flex((self.date_controls(), self.date_grid(selected_date)))
            .direction(Axis::Vertical)
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .main_axis_alignment(MainAxisAlignment::Center)
    }

    fn date_controls(&self) -> impl WidgetView<DatePicker, DatePickerMessage> {
        let month = self.month;
        let year = self.year;

        flex((
            button("<", |data: &mut DatePicker| {
                data.month = data.month.previous();
                DatePickerMessage::ChangeView
            }),
            sized_box(button(format!("{month}"), |_| DatePickerMessage::Nop)).width(100.),
            button(">", |data: &mut DatePicker| {
                data.month = data.month.next();
                DatePickerMessage::ChangeView
            }),
            FlexSpacer::Fixed(40.),
            button("<", |data: &mut DatePicker| {
                data.year -= 1;
                DatePickerMessage::ChangeView
            }),
            button(format!("{year}"), |_| DatePickerMessage::Nop),
            button(">", |data: &mut DatePicker| {
                data.year += 1;
                DatePickerMessage::ChangeView
            }),
        ))
        .direction(Axis::Horizontal)
        .main_axis_alignment(MainAxisAlignment::Center)
    }

    // The selected_date in the interface is needed to highlight the currently selected date
    // It is currently not implemented
    fn date_grid(
        &self,
        _selected_date: &mut Date,
    ) -> impl WidgetView<DatePicker, DatePickerMessage> {
        const COLUMNS: u8 = 7;
        const ROWS: u8 = 5;
        let mut date = Date::from_calendar_date(self.year, self.month, 1).unwrap();
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
                columns.push(
                    sized_box(button(
                        format!("{day_number}"),
                        move |data: &mut DatePicker| {
                            // Set the selected_date
                            data.previous_date = date;
                            DatePickerMessage::Select(date)
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
}
