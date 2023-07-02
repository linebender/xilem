// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

//! A highly experimental web framework using the xilem architecture.
//!
//! Run using `trunk serve`.

use wasm_bindgen::JsCast;

mod app;
//mod button;
mod class;
mod context;
mod event;
//mod div;
mod element;
mod text;
mod view;
#[cfg(feature = "typed")]
mod view_ext;

pub use xilem_core::MessageResult;

pub use app::App;
pub use class::class;
pub use context::ChangeFlags;
#[cfg(feature = "typed")]
pub use element::elements;
pub use element::{element, Element, ElementState};
#[cfg(feature = "typed")]
pub use event::events;
pub use event::{on_event, Event, OnEvent, OnEventState};
pub use text::{text, Text};
pub use view::{Adapt, AdaptThunk, Pod, View, ViewMarker, ViewSequence};
#[cfg(feature = "typed")]
pub use view_ext::ViewExt;

xilem_core::message!();

/// The HTML namespace: `http://www.w3.org/1999/xhtml`
pub const HTML_NS: &str = "http://www.w3.org/1999/xhtml";
/// The SVG namespace: `http://www.w3.org/2000/svg`
pub const SVG_NS: &str = "http://www.w3.org/2000/svg";
/// The MathML namespace: `http://www.w3.org/1998/Math/MathML`
pub const MATHML_NS: &str = "http://www.w3.org/1998/Math/MathML";

/// Helper to get the HTML document
pub fn document() -> web_sys::Document {
    let window = web_sys::window().expect("no global `window` exists");
    window.document().expect("should have a document on window")
}

/// Helper to get the HTML document body element
pub fn document_body() -> web_sys::HtmlElement {
    document().body().expect("HTML document missing body")
}

pub fn get_element_by_id(id: &str) -> web_sys::HtmlElement {
    document()
        .get_element_by_id(id)
        .unwrap()
        .dyn_into()
        .unwrap()
}
