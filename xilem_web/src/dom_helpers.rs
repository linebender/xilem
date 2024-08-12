// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use web_sys::wasm_bindgen::JsCast;

/// Helper to get the HTML document body element
pub fn document_body() -> web_sys::HtmlElement {
    document().body().expect("HTML document missing body")
}

/// Helper to get the HTML document
pub fn document() -> web_sys::Document {
    let window = web_sys::window().expect("no global `window` exists");
    window.document().expect("should have a document on window")
}

/// Helper to get a DOM element by id
pub fn get_element_by_id(id: &str) -> web_sys::HtmlElement {
    document()
        .get_element_by_id(id)
        .unwrap()
        .dyn_into()
        .unwrap()
}

/// Helper to get the value from an HTML input element from a given event.
///
/// Returns `None` if the event isn't a valid input event or conversions fail.
pub fn input_event_target_value(event: &web_sys::Event) -> Option<String> {
    event
        .target()?
        .dyn_into::<web_sys::HtmlInputElement>()
        .ok()?
        .value()
        .into()
}
