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
//! use xilem::{EventLoop, WidgetView, Xilem};
//!
//! #[derive(Default)]
//! struct Counter {
//!     num: i32,
//! }
//!
//! fn app_logic(data: &mut Counter) -> impl WidgetView<Counter> {
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
//! [accesskit_docs]: accesskit
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
// LINEBENDER LINT SET - lib.rs - v1
// See https://linebender.org/wiki/canonical-lints/
// These lints aren't included in Cargo.toml because they
// shouldn't apply to examples and tests
#![warn(unused_crate_dependencies)]
#![warn(clippy::print_stdout, clippy::print_stderr)]
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
#![expect(clippy::exhaustive_enums, reason = "Deferred: Noisy")]
#![expect(clippy::match_same_arms, reason = "Deferred: Noisy")]
#![expect(clippy::missing_assert_message, reason = "Deferred: Noisy")]
#![expect(elided_lifetimes_in_paths, reason = "Deferred: Noisy")]
// https://github.com/rust-lang/rust/pull/130025
#![allow(missing_docs, reason = "We have many as-yet undocumented items")]
#![expect(clippy::missing_errors_doc, reason = "Can be quite noisy?")]
#![expect(clippy::missing_panics_doc, reason = "Can be quite noisy?")]
#![expect(
    clippy::shadow_unrelated,
    reason = "Potentially controversial code style"
)]
#![expect(clippy::allow_attributes, reason = "Deferred: Noisy")]
#![expect(clippy::allow_attributes_without_reason, reason = "Deferred: Noisy")]

use std::collections::HashMap;
use std::sync::Arc;

use masonry::core::{FromDynWidget, Widget, WidgetId, WidgetMut, WidgetPod};
use masonry::dpi::LogicalSize;
use masonry::widgets::RootWidget;
use view::{transformed, Transformed};
use winit::error::EventLoopError;
use winit::window::{Window, WindowAttributes};

use crate::core::{
    AsyncCtx, MessageResult, Mut, RawProxy, SuperElement, View, ViewElement, ViewId,
    ViewPathTracker, ViewSequence,
};
pub use masonry::app::{EventLoop, EventLoopBuilder};
pub use masonry::kurbo::{Affine, Vec2};
pub use masonry::parley::style::FontWeight;
pub use masonry::parley::Alignment as TextAlignment;
pub use masonry::peniko::Color;
pub use masonry::widgets::LineBreaking;
pub use masonry::{dpi, palette};
pub use xilem_core as core;

/// Tokio is the async runner used with Xilem.
pub use tokio;

pub use winit;

mod any_view;
mod driver;
mod one_of;

pub mod view;
pub use any_view::AnyWidgetView;
pub use driver::{async_action, MasonryDriver, MasonryProxy, ASYNC_MARKER_WIDGET};

/// Runtime builder.
#[must_use = "A Xilem app does nothing unless ran."]
pub struct Xilem<State, Logic> {
    state: State,
    logic: Logic,
    runtime: tokio::runtime::Runtime,
    background_color: Color,
    // Font data to include in loading.
    fonts: Vec<Vec<u8>>,
}

impl<State, Logic, View> Xilem<State, Logic>
where
    Logic: FnMut(&mut State) -> View,
    View: WidgetView<State>,
{
    pub fn new(state: State, logic: Logic) -> Self {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        Self {
            state,
            logic,
            runtime,
            background_color: Color::BLACK,
            fonts: Vec::new(),
        }
    }

    /// Load a font when this `Xilem` is run.
    ///
    /// This is an interim API whilst font lifecycles are determined.
    pub fn with_font(mut self, data: impl Into<Vec<u8>>) -> Self {
        self.fonts.push(data.into());
        self
    }

    /// Sets main window background color.
    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = color;
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
        self,
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
        let (root_widget, driver) = self.into_driver(Arc::new(MasonryProxy(proxy)));
        masonry::app::run_with(event_loop, window_attributes, root_widget, driver, bg_color)
    }

    pub fn into_driver(
        mut self,
        proxy: Arc<dyn RawProxy>,
    ) -> (
        impl Widget,
        MasonryDriver<State, Logic, View, View::ViewState>,
    ) {
        let first_view = (self.logic)(&mut self.state);
        let mut ctx = ViewCtx {
            widget_map: WidgetMap::default(),
            id_path: Vec::new(),
            proxy,
            runtime: self.runtime,
        };
        let (pod, view_state) = first_view.build(&mut ctx);
        let root_widget = RootWidget::from_pod(pod.into_widget_pod());
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

/// A container for a yet to be inserted [Masonry](masonry) widget
/// to be used with Xilem.
///
/// This exists for two reasons:
/// 1) The nearest equivalent type in Masonry, [`WidgetPod`], can't have
///    [Xilem Core](xilem_core) traits implemented on it due to Rust's orphan rules.
/// 2) `WidgetPod` is also used during a Widget's lifetime to contain its children,
///    and so might not actually own the underlying widget value.
///    When creating widgets in Xilem, layered views all want access to the - using
///    `WidgetPod` for this purpose would require fallible unwrapping.
pub struct Pod<W: Widget + FromDynWidget + ?Sized> {
    pub widget: Box<W>,
    pub id: WidgetId,
    /// The transform the widget will be created with.
    ///
    /// If changing transforms of widgets, prefer to use [`transformed`]
    /// (or [`WidgetView::transform`]).
    /// This has a protocol to ensure that multiple views changing the
    /// transform interoperate successfully.
    pub transform: Affine,
}

impl<W: Widget + FromDynWidget> Pod<W> {
    /// Create a new `Pod` from a `widget`.
    ///
    /// This contains the widget value, and other metadata which will
    /// be used when that widget is added to a Masonry tree.
    pub fn new(widget: W) -> Self {
        Self {
            widget: Box::new(widget),
            id: WidgetId::next(),
            transform: Affine::default(),
        }
    }
}

impl<W: Widget + FromDynWidget + ?Sized> Pod<W> {
    /// Type-erase the contained widget.
    ///
    /// Convert a `Pod` pointing to a widget of a specific concrete type
    /// `Pod` pointing to a `dyn Widget`.
    pub fn erased(self) -> Pod<dyn Widget> {
        Pod {
            widget: self.widget.as_box_dyn(),
            id: self.id,
            transform: self.transform,
        }
    }
    /// Finalise this `Pod`, converting into a [`WidgetPod`].
    ///
    /// In most cases, you will use the return value when creating a
    /// widget with a single child.
    /// For example, button widgets have a label child.
    ///
    /// If you're adding the widget to a layout container widget,
    /// which can contain heterogenous widgets, you will probably
    /// prefer to use [`Self::erased_widget_pod`].
    pub fn into_widget_pod(self) -> WidgetPod<W> {
        WidgetPod::new_with_id_and_transform(self.widget, self.id, self.transform)
    }
    /// Finalise this `Pod` into a type-erased [`WidgetPod`].
    ///
    /// In most cases, you will use the return value for adding to a layout
    /// widget which supports heterogenous widgets.
    /// For example, [`Flex`](masonry::widgets::Flex) accepts type-erased widget pods.
    pub fn erased_widget_pod(self) -> WidgetPod<dyn Widget> {
        WidgetPod::new_with_id_and_transform(self.widget, self.id, self.transform).erased()
    }
}

impl<W: Widget + FromDynWidget + ?Sized> ViewElement for Pod<W> {
    type Mut<'a> = WidgetMut<'a, W>;
}

impl<W: Widget + FromDynWidget + ?Sized> SuperElement<Pod<W>, ViewCtx> for Pod<dyn Widget> {
    fn upcast(_: &mut ViewCtx, child: Pod<W>) -> Self {
        child.erased()
    }

    fn with_downcast_val<R>(
        mut this: Self::Mut<'_>,
        f: impl FnOnce(Mut<Pod<W>>) -> R,
    ) -> (Self::Mut<'_>, R) {
        let downcast = this.downcast();
        let ret = f(downcast);
        (this, ret)
    }
}

pub trait WidgetView<State, Action = ()>:
    View<State, Action, ViewCtx, Element = Pod<Self::Widget>> + Send + Sync
{
    type Widget: Widget + FromDynWidget + ?Sized;

    /// Returns a boxed type erased [`AnyWidgetView`]
    ///
    /// # Examples
    /// ```
    /// use xilem::{view::label, WidgetView};
    ///
    /// # fn view<State: 'static>() -> impl WidgetView<State> {
    /// label("a label").boxed()
    /// # }
    ///
    /// ```
    fn boxed(self) -> Box<AnyWidgetView<State, Action>>
    where
        State: 'static,
        Action: 'static,
        Self: Sized,
    {
        Box::new(self)
    }

    /// This widget with a 2d transform applied.
    ///
    /// See [`transformed`] for similar functionality with a builder-API using this.
    /// The return type is the same as for `transformed`, and so also has these
    /// builder methods.
    fn transform(self, by: Affine) -> Transformed<Self, State, Action>
    where
        Self: Sized,
    {
        transformed(self).transform(by)
    }
}

impl<V, State, Action, W> WidgetView<State, Action> for V
where
    V: View<State, Action, ViewCtx, Element = Pod<W>> + Send + Sync,
    W: Widget + FromDynWidget + ?Sized,
{
    type Widget = W;
}

/// An ordered sequence of widget views, it's used for `0..N` views.
/// See [`ViewSequence`] for more technical details.
///
/// # Examples
///
/// ```
/// use xilem::{view::prose, WidgetViewSequence};
///
/// fn prose_sequence<State: 'static>(
///     texts: impl Iterator<Item = &'static str>,
/// ) -> impl WidgetViewSequence<State> {
///     texts.map(prose).collect::<Vec<_>>()
/// }
/// ```
pub trait WidgetViewSequence<State, Action = ()>:
    ViewSequence<State, Action, ViewCtx, Pod<any_view::DynWidget>>
{
}

impl<Seq, State, Action> WidgetViewSequence<State, Action> for Seq where
    Seq: ViewSequence<State, Action, ViewCtx, Pod<any_view::DynWidget>>
{
}

type WidgetMap = HashMap<WidgetId, Vec<ViewId>>;

pub struct ViewCtx {
    /// The map from a widgets id to its position in the View tree.
    ///
    /// This includes only the widgets which might send actions
    widget_map: WidgetMap,
    id_path: Vec<ViewId>,
    proxy: Arc<dyn RawProxy>,
    runtime: tokio::runtime::Runtime,
}

impl ViewPathTracker for ViewCtx {
    fn push_id(&mut self, id: ViewId) {
        self.id_path.push(id);
    }

    fn pop_id(&mut self) {
        self.id_path.pop();
    }

    fn view_path(&mut self) -> &[ViewId] {
        &self.id_path
    }
}

impl ViewCtx {
    pub fn new_pod<W: Widget + FromDynWidget>(&mut self, widget: W) -> Pod<W> {
        Pod::new(widget)
    }

    pub fn with_leaf_action_widget<W: Widget + FromDynWidget + ?Sized>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Pod<W>,
    ) -> (Pod<W>, ()) {
        (self.with_action_widget(f), ())
    }

    pub fn with_action_widget<W: Widget + FromDynWidget + ?Sized>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Pod<W>,
    ) -> Pod<W> {
        let value = f(self);
        self.record_action(value.id);
        value
    }

    /// Record that the actions from the widget `id` should be routed to this view.
    pub fn record_action(&mut self, id: WidgetId) {
        let path = self.id_path.clone();
        self.widget_map.insert(id, path);
    }

    pub fn teardown_leaf<W: Widget + FromDynWidget + ?Sized>(&mut self, widget: WidgetMut<W>) {
        self.widget_map.remove(&widget.ctx.widget_id());
    }

    pub fn runtime(&self) -> &tokio::runtime::Runtime {
        &self.runtime
    }
}

impl AsyncCtx for ViewCtx {
    fn proxy(&mut self) -> Arc<dyn RawProxy> {
        self.proxy.clone()
    }
}
