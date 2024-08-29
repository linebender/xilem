use std::fmt::format;

// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0
use masonry::widget::{CrossAxisAlignment, MainAxisAlignment};
use time::{util::days_in_year_month, Date, Month, OffsetDateTime};
use winit::error::EventLoopError;
use xilem::{
    view::{button, date, flex, label, sized_box, Axis, DateData, FlexExt as _, FlexSpacer},
    EventLoop, WidgetView, Xilem,
};
use xilem_core::{frozen, map_state};

struct Calendar {
    // selected_date: Date,
    // month: Month,
    // year: i32,
    date: DateData,
}

impl Calendar {
    fn new() -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            // selected_date: ,
            // month: now.month(),
            // year: now.year(),
            date: DateData::new(now.date(), now.month(), now.year()),
        }
    }
}
fn selected_date(selected_date: Date) -> impl WidgetView<Calendar> {
    flex((label("Selected date:"), label(format!("{selected_date}")))).direction(Axis::Horizontal)
}

/// A component to make a bigger than usual button
fn external_controls() -> impl WidgetView<Calendar> {
    flex((
        button("Today", |data: &mut Calendar| {
            data.date.selected_date = OffsetDateTime::now_utc().date();
        }),
        button("Tomorrow", |data: &mut Calendar| {
            data.date.selected_date = OffsetDateTime::now_utc().date().next_day().unwrap();
        }),
    ))
    .direction(Axis::Horizontal)
}

fn app_logic(data: &mut Calendar) -> impl WidgetView<Calendar> {
    flex((
        selected_date(data.date.selected_date),
        external_controls(),
        map_state(
            date(
                &mut data.date.selected_date,
                &mut data.date.month,
                &mut data.date.year,
            ),
            |data: &mut Calendar| &mut data.date,
        ),
    ))
    .direction(Axis::Vertical)
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .main_axis_alignment(MainAxisAlignment::Center)
}

fn main() -> Result<(), EventLoopError> {
    let data = Calendar::new();
    let app = Xilem::new(data, app_logic);
    app.run_windowed(EventLoop::with_user_event(), "Calendar".into())?;
    Ok(())
}
