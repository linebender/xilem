// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Xilem is a UI toolkit. It combines ideas from `Flutter`, `SwiftUI`, and `Elm`.
//! Like all of these, it uses lightweight view objects, diffing them to provide
//! minimal updates to a retained UI. Like `SwiftUI`, it is strongly typed.
//!
//! The talk *[Xilem: Let's Build High Performance Rust UI](https://www.youtube.com/watch?v=OvfNipIcRiQ)* by Raph Levien
//! was presented at the RustNL conference in 2024, and gives a video introduction to these ideas.
//! Xilem is implemented as a reactive layer on top of [Masonry][masonry], a widget toolkit which is developed alongside Xilem.
//! Masonry itself is built on top of a wide array of foundational Rust UI projects:
//!
//! * Rendering is provided by [Vello][masonry::vello], a high performance GPU compute-centric 2D renderer.
//! * GPU compute infrastructure is provided by [wgpu][masonry::vello::wgpu].
//! * Text layout is provided by [Parley][masonry::parley].
//! * Accessibility is provided by [AccessKit][] ([docs][accesskit_docs]).
//! * Window handling is provided by [winit][].
//!
//! Xilem can currently be considered to be in an alpha state. Lots of things need improvements (including this documentation!).
//!
//! There is also a [blog post][xilem_blog] from when Xilem was first introduced.
//!
//! ## Example
//!
//! A simple incrementing counter application looks like:
//!
//! ```rust,no_run
//! use winit::error::EventLoopError;
//! use xilem::view::{button, flex, label};
//! use xilem::{EventLoop, WindowOptions, WidgetView, Xilem};
//!
//! #[derive(Default)]
//! struct Counter {
//!     num: i32,
//! }
//!
//! fn app_logic(data: &mut Counter) -> impl WidgetView<Counter> + use<> {
//!     flex((
//!         label(format!("{}", data.num)),
//!         button("increment", |data: &mut Counter| data.num += 1),
//!     ))
//! }
//!
//! fn main() -> Result<(), EventLoopError> {
//!     let app = Xilem::new_simple(Counter::default(), app_logic, WindowOptions::new("Counter app"));
//!     app.run_in(EventLoop::builder())?;
//!     Ok(())
//! }
//! ```
//!
//! A key feature of Xilem's architecture is that the application's state, in this case `Counter`, is an arbitrary `'static` Rust type.
//! In this example, `app_logic` is the root component, which creates the view value it returns.
//! This, in turn, leads to corresponding Masonry widgets being created, in this case a button and a label.
//! When the button is pressed, the number will be incremented, and then `app_logic` will be re-ran.
//! The returned view will be compared with its previous value, which will minimally update the contents of these widgets.
//! As the `num` field's value has changed, the `label`'s formatted text will be different.
//! This means that the label widget's text will be updated, updating the value displayed to the user.
//! In this case, because the button is the same, it will not be updated.
//!
//! More examples can be found [in the repository][xilem_examples].
//!
//! **Note: The linked examples are for the `main` branch of Xilem. If you are using a released version, please view the examples in the tag for that release.**
//!
//! ## Reactive layer
//!
//! The core concepts of the reactive layer are explained in [Xilem Core][xilem_core].
//!
//! ## View elements
//!
//! The primitives your `Xilem` app’s view tree will generally be constructed from:
//!
//! * [`flex`][crate::view::flex]: defines how items will be arranged in a row or column
//! * [`grid`][crate::view::grid]: divides a window into regions and defines the relationship
//!   between inner elements in terms of size and position
//! * [`sized_box`][crate::view::sized_box]: forces its child to have a specific width and/or height
//! * [`split`][crate::view::split]: contains two views splitting the area either vertically or horizontally which can be resized.
//! * [`button`][crate::view::button]: basic button element
//! * [`image`][crate::view::image]: displays a bitmap image
//! * [`portal`][crate::view::portal]: a scrollable region
//! * [`progress_bar`][crate::view::progress_bar]: progress bar element
//! * [`prose`][crate::view::prose]: displays immutable, selectable text
//! * [`text_input`][crate::view::text_input]: allows text to be edited by the user
//! * [`task`][crate::view::task]: launch an async task which will run until the view is no longer in the tree
//! * [`zstack`][crate::view::zstack]: an element that lays out its children on top of each other
//!
//! You should also expect to use the adapters from Xilem Core, including:
//!
//! * [`lens`][crate::core::lens]: an adapter for using a component from a field of the current state.
//! * [`memoize`][crate::core::memoize]: allows you to avoid recreating views you know won't have changed, based on a key.
//!
//! ## Precise Capturing
//!
//! Throughout Xilem you will find usage of `+ use<>` in return types, which is the Rust syntax for [Precise Capturing](https://doc.rust-lang.org/stable/std/keyword.use.html#precise-capturing).
//! This is new syntax in the 2024 edition, and so it might be unfamiliar.
//! Here's a snippet from the Xilem examples:
//!
//! ```rust,no_run
//! # struct EmojiPagination;
//! # use xilem::WidgetView;
//! fn app_logic(data: &mut EmojiPagination) -> impl WidgetView<EmojiPagination> + use<> {
//!    // ...
//!    # xilem::view::label("Not meaningful!")
//! }
//! ```
//!
//! The precise capturing syntax in this case indicates that the returned view does not make use of the lifetime of `data`.
//! This is required because the view types in Xilem must be `'static`, but as of the 2024 edition, when `impl Trait` is used
//! for return types, Rust assumes that the return value will use the parameter's lifetimes.
//! That is a simplifying assumption for most Rust code, but this is mismatched with how Xilem works.
//!
//! [accesskit_docs]: masonry::accesskit
//! [AccessKit]: https://accesskit.dev/
//! [Druid]: https://crates.io/crates/druid
//! [Fontique]: https://crates.io/crates/fontique
//! [Masonry]: https://crates.io/crates/masonry
//! [Parley]: https://crates.io/crates/parley
//! [skrifa]: https://crates.io/crates/skrifa
//! [swash]: https://crates.io/crates/swash
//! [Vello]: https://crates.io/crates/vello
//! [winit]: https://crates.io/crates/winit
//! [xilem_blog]: https://raphlinus.github.io/rust/gui/2022/05/07/ui-architecture.html
//! [xilem_examples]: https://github.com/linebender/xilem/tree/main/xilem/examples

#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/46134943?s=48&v=4")]
// LINEBENDER LINT SET - lib.rs - v3
// See https://linebender.org/wiki/canonical-lints/
// These lints shouldn't apply to examples or tests.
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
// These lints shouldn't apply to examples.
#![warn(clippy::print_stdout, clippy::print_stderr)]
// Targeting e.g. 32-bit means structs containing usize can give false positives for 64-bit.
#![cfg_attr(target_pointer_width = "64", warn(clippy::trivially_copy_pass_by_ref))]
// END LINEBENDER LINT SET
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![cfg_attr(
    test,
    expect(
        unused_crate_dependencies,
        reason = "False-positive with dev-dependencies only used in examples"
    )
)]
// TODO: Remove any items listed as "Deferred"
#![expect(
    missing_debug_implementations,
    reason = "Deferred: Noisy. Requires same lint to be addressed in Masonry"
)]
#![expect(clippy::missing_assert_message, reason = "Deferred: Noisy")]

pub use masonry;
pub use masonry::kurbo::{Affine, Vec2};
pub use masonry::parley::Alignment as TextAlign;
pub use masonry::parley::style::FontWeight;
pub use masonry::peniko::{Blob, Color, Image, ImageFormat};
pub use masonry::widgets::InsertNewline;
pub use masonry::{dpi, palette};
pub use masonry_winit::app::{EventLoop, EventLoopBuilder, WindowId};
pub use window_options::WindowOptions;
pub use xilem_core as core;

/// Tokio is the async runner used with Xilem.
pub use tokio;

pub use winit;

mod any_view;
mod app;
mod driver;
mod one_of;
mod pod;
mod property_tuple;
mod view_ctx;
mod widget_view;
mod window_options;
mod window_view;

pub mod style;
pub mod view;
pub use any_view::AnyWidgetView;
pub use driver::{ASYNC_MARKER_WIDGET, MasonryDriver, async_action};
pub use property_tuple::PropertyTuple;

pub use app::{AppState, ExitOnClose, Xilem};
pub use pod::Pod;
pub use view_ctx::ViewCtx;
pub use widget_view::{WidgetView, WidgetViewSequence};

// FIXME - Remove these re-exports.
pub(crate) use xilem_core::{MessageResult, View, ViewId};
