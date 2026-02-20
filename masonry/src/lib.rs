// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

// After you edit the crate's doc comment, run this command, then check README.md for any missing links
// cargo rdme --workspace-project=masonry

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/linebender/xilem/main/docs/assets/masonry-logo.svg"
)]

//! Masonry is a foundational framework for building GUI libraries in Rust.
//!
//! The developers of Masonry are developing [Xilem][], a reactive UI library built on top of Masonry.
//! Masonry's API is geared towards creating GUI libraries; if you are creating an application, we recommend also considering Xilem.
//!
//! Masonry gives you a platform-independent manager, which owns and maintains a widget tree.
//! It also gives you tools to inspect that widget tree at runtime, write unit tests on it, and generally have an easier time debugging and maintaining your app.
//!
//! The framework is not opinionated about what your user-facing abstraction will be: you can implement immediate-mode GUI, the Elm architecture, functional reactive GUI, etc., on top of Masonry.
//!
//! It *is* opinionated about its internals: things like text focus, pointer interactions and accessibility events are often handled in a centralized way.
//!
//! Masonry is built on top of:
//!
//! - [Vello][vello] and [wgpu][vello::wgpu] for 2D graphics.
//! - [Parley][parley] for the text stack.
//! - [AccessKit][accesskit] for plugging into accessibility APIs.
//!
//! Masonry can be used with any windowing library which allows the window content to be rendered using `wgpu`.
//! There are currently two backends for using Masonry to create operating system windows:
//!
//! - [Masonry Winit][masonry_winit] for most platforms.
//! - `masonry_android_view` for Android. This can currently be found in the [Android View repository](https://github.com/rust-mobile/android-view),
//!   and is not yet generally usable.
//!
//! <!-- TODO: Document that Masonry is a set of baseline widgets and properties built on Masonry core, which can also be used completely independently -->
//!
//! # Example
//!
//! The to-do-list example looks like this, using Masonry Winit as the backend:
//!
//! ```rust
//! use masonry::core::{ErasedAction, NewWidget, PropertySet, Widget, WidgetId, WidgetTag};
//! use masonry::dpi::LogicalSize;
//! use masonry::layout::Length;
//! use masonry::peniko::color::AlphaColor;
//! use masonry::properties::Padding;
//! use masonry::theme::default_property_set;
//! use masonry::widgets::{Button, ButtonPress, Flex, Label, Portal, TextAction, TextArea, TextInput};
//! use masonry_winit::app::{AppDriver, DriverCtx, NewWindow, WindowId};
//! use masonry_winit::winit::window::Window;
//!
//! const TEXT_INPUT_TAG: WidgetTag<TextInput> = WidgetTag::named("text-input");
//! const LIST_TAG: WidgetTag<Flex> = WidgetTag::named("list");
//! const WIDGET_SPACING: Length = Length::const_px(5.0);
//!
//! struct Driver {
//!     next_task: String,
//!     window_id: WindowId,
//! }
//!
//! impl AppDriver for Driver {
//!     fn on_action(
//!         &mut self,
//!         window_id: WindowId,
//!         ctx: &mut DriverCtx<'_, '_>,
//!         _widget_id: WidgetId,
//!         action: ErasedAction,
//!     ) {
//!         debug_assert_eq!(window_id, self.window_id, "unknown window");
//!
//!         if action.is::<ButtonPress>() {
//!             let render_root = ctx.render_root(window_id);
//!
//!             render_root.edit_widget_with_tag(TEXT_INPUT_TAG, |mut text_input| {
//!                 let mut text_area = TextInput::text_mut(&mut text_input);
//!                 TextArea::reset_text(&mut text_area, "");
//!             });
//!             render_root.edit_widget_with_tag(LIST_TAG, |mut list| {
//!                 let child = Label::new(self.next_task.clone()).with_auto_id();
//!                 Flex::add_fixed(&mut list, child);
//!             });
//!         } else if action.is::<TextAction>() {
//!             let action = action.downcast::<TextAction>().unwrap();
//!             match *action {
//!                 TextAction::Changed(new_text) => {
//!                     self.next_task = new_text.clone();
//!                 }
//!                 TextAction::Entered(_) => {}
//!             }
//!         }
//!     }
//! }
//!
//! /// Return initial to-do-list without items.
//! pub fn make_widget_tree() -> NewWidget<impl Widget> {
//!     let text_input = NewWidget::new_with_tag(
//!         TextInput::new("").with_placeholder("ex: 'Do the dishes', 'File my taxes', ..."),
//!         TEXT_INPUT_TAG,
//!     );
//!     let button = NewWidget::new(Button::with_text("Add task"));
//!
//!     let list = Flex::column()
//!         .with_fixed(NewWidget::new_with_props(
//!             Flex::row()
//!                 .with(text_input, 1.0)
//!                 .with_fixed(button),
//!             PropertySet::new().with(Padding::all(WIDGET_SPACING.get())),
//!         ))
//!         .with_fixed_spacer(WIDGET_SPACING);
//!
//!     NewWidget::new(Portal::new(NewWidget::new_with_tag(list, LIST_TAG)))
//! }
//!
//! fn main() {
//!     let window_size = LogicalSize::new(400.0, 400.0);
//!     let window_attributes = Window::default_attributes()
//!         .with_title("To-do list")
//!         .with_resizable(true)
//!         .with_min_inner_size(window_size);
//!     let driver = Driver {
//!         next_task: String::new(),
//!         window_id: WindowId::next(),
//!     };
//!     # make_widget_tree();
//!     # return;
//!
//!     let event_loop = masonry_winit::app::EventLoop::with_user_event()
//!         .build()
//!         .unwrap();
//!     masonry_winit::app::run_with(
//!         event_loop,
//!         vec![
//!             NewWindow::new_with_id(
//!                 driver.window_id,
//!                 window_attributes,
//!                 make_widget_tree().erased(),
//!             )
//!             .with_base_color(AlphaColor::from_rgb8(2, 6, 23)),
//!         ],
//!         driver,
//!         default_property_set(),
//!     )
//!     .unwrap();
//! }
//! ```
//!
//! Running this will open a window that looks like this:
//!
//! ![Screenshot of the to-do-list example][to-do-screenshot]
//!
//! # Feature flags
//!
//! The following crate [feature flags](https://doc.rust-lang.org/cargo/reference/features.html#dependency-features) are available:
//!
//! - `default`: Enables the default features of [Masonry Core][masonry_core], [Masonry Testing][masonry_testing]
//!   (if enabled via the `testing` feature), and [Vello][vello].
//! - `tracy`: Enables creating output for the [Tracy](https://github.com/wolfpld/tracy) profiler using [`tracing-tracy`][tracing_tracy].
//!   This can be used by installing Tracy and connecting to a Masonry with this feature enabled.
//! - `testing`: Re-exports the test harness from [Masonry Testing][masonry_testing].
//!
//! # Debugging features
//!
//! Masonry apps currently ship with several debugging features built in:
//!
//! - A rudimentary widget inspector - toggled by the F11 key.
//! - A debug mode painting widget layout rectangles - toggled by the F12 key.
//! - Optional automatic registration of a [tracing] subscriber, which outputs to the console and to a file in the dev profile.
//!
//! If you want to use your own subscriber, simply set it before starting masonry - in this case masonry will not set a subscriber.
//!
//! [masonry_winit]: https://crates.io/crates/masonry_winit
//! [Xilem]: https://github.com/linebender/xilem/tree/main/xilem
//! [tracing_tracy]: https://crates.io/crates/tracing-tracy
#![cfg_attr(
    not(docsrs),
    doc = "\n**Warning: This documentation is meant to be read on docs.rs. Screenshots may fail to load otherwise.**\n\n"
)]
// Screenshot generated in the unit test for the `to_do_list` example.
#![doc = concat!(
    "[to-do-screenshot]: ",
    include_doc_path::include_doc_path!("screenshots/example_to_do_list_initial.png")
)]
//!

// LINEBENDER LINT SET - lib.rs - v3
// See https://linebender.org/wiki/canonical-lints/
// These lints shouldn't apply to examples or tests.
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
// These lints shouldn't apply to examples.
#![warn(clippy::print_stdout, clippy::print_stderr)]
// Targeting e.g. 32-bit means structs containing usize can give false positives for 64-bit.
#![cfg_attr(target_pointer_width = "64", warn(clippy::trivially_copy_pass_by_ref))]
// END LINEBENDER LINT SET
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(
    test,
    expect(
        unused_crate_dependencies,
        reason = "False-positive with dev-dependencies only used in examples"
    )
)]
// TODO: Remove any items listed as "Deferred"
#![expect(missing_debug_implementations, reason = "Deferred: Noisy")]
#![expect(clippy::cast_possible_truncation, reason = "Deferred: Noisy")]
#![expect(clippy::single_match, reason = "General policy not decided")]

// TODO - re-add #[doc(hidden)]
pub mod doc;

#[cfg(test)]
mod tests;

pub mod layers;
pub mod properties;
pub mod theme;
pub mod widgets;

pub use accesskit;
pub use parley::{Alignment as TextAlign, AlignmentOptions as TextAlignOptions};
pub use vello::peniko::color::palette;
pub use vello::{kurbo, peniko};
pub use {dpi, parley, vello};

pub use masonry_core::{app, core, layout, ui_events, util};
#[cfg(any(feature = "testing", test))]
pub use masonry_testing as testing;
