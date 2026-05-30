// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Demonstrates the focus modifiers `.focus_on_appear(bool)` and `.focus(bool)`.
//!
//! - `.focus_on_appear`: index 0 of the `indexed_stack` holds a text field that gets auto focus
//!   each time the view becomes visible (it is stashed, not destroyed, when you switch away).
//!   The checkbox toggles the behavior at runtime.
//! - `focus`: an edge-triggered command driven by app state. toggle the checkboxes.
//!   `false->true` (rising edge -> focus), `true->false` (falling edge -> resign).

use masonry::dpi::LogicalSize;
use masonry_winit::app::{EventLoop, EventLoopBuilder};
use masonry::layout::AsUnit;
use winit::error::EventLoopError;
use xilem::focus::Focusable as _;
use xilem::style::{Padding, Style};
use xilem::view::{checkbox, flex_col, flex_row, indexed_stack, label, text_button, 
                  text_input, divider_h, CrossAxisAlignment};
use xilem::{WidgetView, WindowOptions, Xilem};


/// The state of the entire application.
struct FocusDemo {
    /// Active `indexed_stack` page.
    page: usize,
    /// Whether the page-0 field uses `focus_on_appear`.
    autofocus: bool,
    /// Contents of the page-0 (auto-focused) field.
    name: String,
    /// Contents of the page-1 field.
    other: String,
    /// Contents of the imperatively focused field.
    note: String,
    /// Drives `.focus(bool)` on the note field (edge-triggered).
    focus_note: bool,
     /// Drives `.focus(bool)` on the button (edge-triggered).
    focus_button: bool,
}

fn app_logic(data: &mut FocusDemo) -> impl WidgetView<FocusDemo> + use<> {
    flex_col((
        label(".focus_on_appear: switch to Tab 0 and its field gets focused when shown"),        
        flex_row((
            text_button("Tab 0", |data: &mut FocusDemo| data.page = 0),
            text_button("Tab 1", |data: &mut FocusDemo| data.page = 1)
                .focus(data.focus_button),
            checkbox(
                "autofocus on appear of page 0",
                data.autofocus,
                |data: &mut FocusDemo, checked| data.autofocus = checked,
            ),
        )),
        divider_h(),
        indexed_stack((
            flex_col((
                label("This is page 0."),
                text_input(data.name.clone(), |data: &mut FocusDemo, v| data.name = v)
                    .focus_on_appear(data.autofocus),
            )).cross_axis_alignment(CrossAxisAlignment::Start),
            flex_col((
                label("This is page 1."),
                label("Cycling is fun."),
            )).cross_axis_alignment(CrossAxisAlignment::Start),
        ))
        .active(data.page),
        divider_h(),
        label("focus(bool): edge-triggered. Rising edge focuses, falling edge removes focus."),
        text_input(data.note.clone(), |data: &mut FocusDemo, v| data.note = v)
            .focus(data.focus_note),
        checkbox(
            "focus second text_input on rising edge.",
            data.focus_note,
            |data: &mut FocusDemo, checked| data.focus_note = checked,
        ),
        checkbox(
            "focus \"Tab 1\" button on rising edge.",
            data.focus_button,
            |data: &mut FocusDemo, checked| data.focus_button = checked,
        ),        
    )).cross_axis_alignment(CrossAxisAlignment::Start)
    .padding(16.0.px())
}

fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let data = FocusDemo {
        page: 0,
        autofocus: true,
        name: "Xilem is fun.".into(),
        other: String::new(),
        note: String::new(),
        focus_note: false,
        focus_button: false,
    };

    let app = Xilem::new_simple(
        data,
        app_logic,
        WindowOptions::new("Xilem focus")
            .with_initial_inner_size(LogicalSize::new(520., 360.)),
    );
    app.run_in(event_loop)?;
    Ok(())
}

// Boilerplate code: Identical across all applications which support Android

fn main() -> Result<(), EventLoopError> {
    run(EventLoop::with_user_event())
}
