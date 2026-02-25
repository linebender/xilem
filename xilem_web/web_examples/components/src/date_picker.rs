// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use time::{Date, Duration, OffsetDateTime};
use xilem_web::{elements::html, interfaces::Element, modifiers::style};

#[derive(Debug, Default)]
pub(crate) struct State {
    pub selected: Option<Date>,
    pub popup_open: bool,
    pub shown_month: Option<Date>,
}

pub(crate) enum Action {
    DateChanged(Option<Date>),
    Cancelled,
}

impl xilem_web::Action for Action {}

// /// Renders only the calendar popup (always visible, no input field).
// /// Intended for use in overlays where the caller controls positioning and backdrop.
pub(crate) fn view(state: &State) -> impl Element<State, Action> + use<> {
    let clear_button = html::button("Clear").on_click(|state: &mut State, _| {
        state.selected = None;
        state.shown_month = None;
        Action::DateChanged(None)
    });

    let today_button = html::button("Today").on_click(|state: &mut State, _| {
        let today = today();
        state.selected = Some(today);
        state.shown_month = Some(today);
        Action::DateChanged(Some(today))
    });

    let cancel_button = html::button("\u{2716}")
        .class("ml-auto")
        .on_click(|_: &mut State, _| Action::Cancelled);

    html::div((
        html::div((clear_button, today_button, cancel_button)).class("actions"),
        navigation_bar(state),
        calendar_grid(state),
    ))
    .style((!state.popup_open).then_some(style("display", "none")))
    .class("date-picker")
}

fn navigation_bar(model: &State) -> impl Element<State, Action> + use<> {
    let shown = model.shown_month.unwrap_or_else(today);

    let title = format!("{} {}", shown.month(), shown.year());

    html::div((
        html::button("<").on_click(|model: &mut State, _| {
            let cur = model.shown_month.unwrap_or_else(today);
            model.shown_month = Some(shift_month(cur, -1));
        }),
        html::button("\u{00AB}").on_click(|model: &mut State, _| {
            let cur = model.shown_month.unwrap_or_else(today);
            model.shown_month = Some(shift_month(cur, -12));
        }),
        html::span(title).class("title"),
        html::button("\u{00BB}").on_click(|model: &mut State, _| {
            let cur = model.shown_month.unwrap_or_else(today);
            model.shown_month = Some(shift_month(cur, 12));
        }),
        html::button(">").on_click(|model: &mut State, _| {
            let cur = model.shown_month.unwrap_or_else(today);

            model.shown_month = Some(shift_month(cur, 1));
        }),
    ))
    .class("nav-bar")
}

fn calendar_grid(model: &State) -> impl Element<State, Action> + use<> {
    let now = OffsetDateTime::now_local().unwrap();
    let today = Date::from_calendar_date(now.year(), now.month(), now.day()).unwrap();
    let shown_month = model.shown_month.unwrap_or(today);

    let start = month_start(shown_month);
    let mut day = start;

    let weeks = (0..6).map(|_| {
        let days = (0..7).map(|_| {
            let cur_day = day;
            day += Duration::days(1);

            let class = if cur_day.month() != shown_month.month() {
                Some("not-in-month")
            } else if Some(cur_day) == model.selected {
                Some("selected")
            } else if cur_day == today {
                Some("today")
            } else {
                None
            };

            html::td(cur_day.day().to_string())
                .on_click(move |mdl: &mut State, _event| {
                    mdl.selected = Some(cur_day);
                    mdl.popup_open = false;
                    Action::DateChanged(Some(cur_day))
                })
                .class("day")
                .class(class)
        });

        html::tr(days.collect::<Vec<_>>())
    });

    let header = html::tr(WEEKDAYS.iter().map(|d| html::th(*d)).collect::<Vec<_>>());

    html::table((html::thead(header), html::tbody(weeks.collect::<Vec<_>>()))).class("calendar")
}

const WEEKDAYS: [&str; 7] = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];

fn month_start(date: Date) -> Date {
    let first = Date::from_calendar_date(date.year(), date.month(), 1).unwrap();
    let weekday = first.weekday().number_from_monday();
    first - Duration::days(i64::from(weekday - 1))
}

fn shift_month(base: Date, months: i16) -> Date {
    let mut year = base.year();
    let mut month = base.month() as i16 + months;

    while month > 12 {
        month -= 12;
        year += 1;
    }
    while month < 1 {
        month += 12;
        year -= 1;
    }

    let day = base
        .day()
        .min(days_in_month(year, u8::try_from(month).unwrap()));
    let month = time::Month::try_from(u8::try_from(month).unwrap()).unwrap();

    Date::from_calendar_date(year, month, day).unwrap()
}

fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => unreachable!(),
    }
}

fn today() -> Date {
    let now = OffsetDateTime::now_local().unwrap();
    Date::from_calendar_date(now.year(), now.month(), now.day()).unwrap()
}
