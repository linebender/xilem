// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

//! A highly experimental web framework using the xilem architecture.
//!
//! Run using `trunk serve`.

use wasm_bindgen::JsCast;

mod app;
mod class;
mod context;
mod diff;
mod element;
mod event;
mod view;
#[cfg(feature = "typed")]
mod view_ext;

pub use xilem_core::MessageResult;

pub use app::App;
pub use class::class;
pub use context::{ChangeFlags, Cx};
#[cfg(feature = "typed")]
pub use element::elements;
pub use element::{element, Element};
#[cfg(feature = "typed")]
pub use event::events;
pub use event::{on_event, Action, Event, OnEvent, OptionalAction};
pub use view::{
    memoize, static_view, Adapt, AdaptThunk, AnyView, Either, Memoize, Pod, View, ViewMarker,
    ViewSequence,
};
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

/// Returns a handle to the element with the given ID.
///
/// # Panics
///
/// This function will panic if no element with the given ID exists.
#[track_caller]
pub fn get_element_by_id(id: &str) -> web_sys::HtmlElement {
    let el = match document().get_element_by_id(id) {
        Some(el) => el,
        None => panic!("no element exists with the ID `{id}`"),
    };
    el.dyn_into()
        .expect("get_element_by_id could not cast `Element` into `HtmlElement`")
}
