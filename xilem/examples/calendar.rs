// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0
use masonry::widget::{CrossAxisAlignment, MainAxisAlignment};
use time::{Date, OffsetDateTime};
use winit::error::EventLoopError;
use xilem::{
    view::{button, flex, label, Axis, DatePicker, DatePickerMessage},
    EventLoop, WidgetView, Xilem,
};
use xilem_core::{adapt, MessageResult};

struct Calendar {
    selected_date: Date,
    date: DatePicker,
}

impl Calendar {
    fn new() -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            selected_date: now.date(),
            date: DatePicker::new(now.month(), now.year()),
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
            data.selected_date = OffsetDateTime::now_utc().date();
        }),
        button("Tomorrow", |data: &mut Calendar| {
            data.selected_date = OffsetDateTime::now_utc().date().next_day().unwrap();
        }),
    ))
    .direction(Axis::Horizontal)
}

fn app_logic(data: &mut Calendar) -> impl WidgetView<Calendar> {
    flex((
        selected_date(data.selected_date),
        external_controls(),
        adapt(
            data.date.view(&mut data.selected_date),
            |state: &mut Calendar, thunk| match thunk.call(&mut state.date) {
                MessageResult::Action(DatePickerMessage::Select(date)) => {
                    state.selected_date = date;
                    MessageResult::Action(())
                }
                MessageResult::Action(DatePickerMessage::Nop) => MessageResult::Nop,
                MessageResult::Action(DatePickerMessage::ChangeView) => MessageResult::Action(()),
                message_result => message_result.map(|_| ()),
            },
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
