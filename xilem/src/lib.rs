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
//! * Rendering is provided by [Vello][masonry_winit::vello], a high performance GPU compute-centric 2D renderer.
//! * GPU compute infrastructure is provided by [wgpu][masonry_winit::vello::wgpu].
//! * Text layout is provided by [Parley][masonry_winit::parley].
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
//! use xilem::{EventLoop, WidgetView, Xilem};
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
//!     let app = Xilem::new(Counter::default(), app_logic);
//!     app.run_windowed(EventLoop::with_user_event(), "Counter app".into())?;
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
//! * [`textbox`][crate::view::textbox]: allows text to be edited by the user
//! * [`task`][crate::view::task]: launch an async task which will run until the view is no longer in the tree
//! * [`zstack`][crate::view::zstack]: an element that lays out its children on top of each other
//!
//! You should also expect to use the adapters from Xilem Core, including:
//!
//! * [`lens`][crate::core::lens]: an adapter for using a component from a field of the current state.
//! * [`memoize`][crate::core::memoize]: allows you to avoid recreating views you know won't have changed, based on a key.
//!
//! [accesskit_docs]: https://docs.rs/accesskit/latest/accesskit/
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
#![cfg_attr(not(debug_assertions), allow(unused))]
#![expect(
    missing_debug_implementations,
    reason = "Deferred: Noisy. Requires same lint to be addressed in Masonry"
)]
#![expect(elided_lifetimes_in_paths, reason = "Deferred: Noisy")]

use std::sync::Arc;

use masonry::core::{DefaultProperties, Widget};
use masonry::dpi::LogicalSize;
use masonry::theme::default_property_set;
use masonry::widgets::RootWidget;
use winit::error::EventLoopError;
use winit::window::{Window, WindowAttributes};
use xilem_core::RawProxy;

pub use masonry_winit::app::{EventLoop, EventLoopBuilder};
pub use masonry_winit::kurbo::{Affine, Vec2};
pub use masonry_winit::parley::Alignment as TextAlignment;
pub use masonry_winit::parley::style::FontWeight;
pub use masonry_winit::peniko::{Blob, Color};
pub use masonry_winit::widgets::{InsertNewline, LineBreaking};
pub use masonry_winit::{dpi, palette};
pub use xilem_core as core;

/// Tokio is the async runner used with Xilem.
pub use tokio;

pub use winit;

mod driver;

pub use xilem_masonry::AnyWidgetView;
pub use xilem_masonry::PropertyTuple;
pub use xilem_masonry::style;
pub use xilem_masonry::view;
pub use xilem_masonry::{Pod, ViewCtx, WidgetView, WidgetViewSequence};

pub use driver::{ASYNC_MARKER_WIDGET, MasonryDriver, MasonryProxy, async_action};

/// Runtime builder.
#[must_use = "A Xilem app does nothing unless ran."]
pub struct Xilem<State, Logic> {
    state: State,
    logic: Logic,
    runtime: tokio::runtime::Runtime,
    default_properties: Option<DefaultProperties>,
    background_color: Color,
    // Font data to include in loading.
    fonts: Vec<Blob<u8>>,
}

#[expect(missing_docs, reason = "TODO - Document these items")]
impl<State, Logic, View> Xilem<State, Logic>
where
    Logic: FnMut(&mut State) -> View,
    View: WidgetView<State>,
{
    /// Initialize the builder state for your app.
    pub fn new(state: State, logic: Logic) -> Self {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        Self {
            state,
            logic,
            runtime,
            default_properties: None,
            background_color: Color::BLACK,
            fonts: Vec::new(),
        }
    }

    /// Load a font when this `Xilem` is run.
    ///
    /// This is an interim API whilst font lifecycles are determined.
    pub fn with_font(mut self, data: impl Into<Blob<u8>>) -> Self {
        self.fonts.push(data.into());
        self
    }

    /// Sets main window background color.
    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = color;
        self
    }

    // TODO: Find better ways to customize default property set.
    /// Sets default properties of widget tree.
    pub fn with_default_properties(mut self, default_properties: DefaultProperties) -> Self {
        self.default_properties = Some(default_properties);
        self
    }

    // TODO: Make windows a specific view
    /// Run app with default window attributes.
    pub fn run_windowed(
        self,
        // We pass in the event loop builder to allow
        // This might need to be generic over the event type?
        event_loop: EventLoopBuilder,
        window_title: String,
    ) -> Result<(), EventLoopError>
    where
        State: 'static,
        Logic: 'static,
        View: 'static,
    {
        let window_size = LogicalSize::new(600., 800.);
        let window_attributes = Window::default_attributes()
            .with_title(window_title)
            .with_resizable(true)
            .with_min_inner_size(window_size);
        self.run_windowed_in(event_loop, window_attributes)
    }

    // TODO: Make windows into a custom view
    /// Run app with custom window attributes.
    pub fn run_windowed_in(
        mut self,
        mut event_loop: EventLoopBuilder,
        window_attributes: WindowAttributes,
    ) -> Result<(), EventLoopError>
    where
        State: 'static,
        Logic: 'static,
        View: 'static,
    {
        let event_loop = event_loop.build()?;
        let proxy = event_loop.create_proxy();
        let bg_color = self.background_color;
        let default_properties = self
            .default_properties
            .take()
            .unwrap_or_else(default_property_set);
        let (root_widget, driver) = self.into_driver(Arc::new(MasonryProxy(proxy)));
        masonry_winit::app::run_with(
            event_loop,
            window_attributes,
            root_widget,
            driver,
            default_properties,
            bg_color,
        )
    }

    pub fn into_driver(
        mut self,
        proxy: Arc<dyn RawProxy>,
    ) -> (
        impl Widget,
        MasonryDriver<State, Logic, View, View::ViewState>,
    ) {
        let first_view = (self.logic)(&mut self.state);
        let mut ctx = ViewCtx::new(proxy, self.runtime);
        let (pod, view_state) = first_view.build(&mut ctx);
        let root_widget = RootWidget::from_pod(pod.into_widget_pod().erased());
        let driver = MasonryDriver {
            current_view: first_view,
            logic: self.logic,
            state: self.state,
            ctx,
            view_state,
            fonts: self.fonts,
        };
        (root_widget, driver)
    }
}
