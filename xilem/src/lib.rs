// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! `Xilem` is a UI toolkit. It combines ideas from `Flutter`, `SwiftUI`, and `Elm`.
//! Like all of these, it uses lightweight view objects, diffing them to provide
//! minimal updates to a retained UI. Like `SwiftUI`, it is strongly typed. For more
//! details on `Xilem`'s reactive architecture see `Xilem`: an [architecture for UI in Rust].
//!
//! `Xilem`'s reactive layer is built on top of a wide array of foundational Rust UI projects, e.g.:
//!
//! * Widgets are provided by [Masonry], which is a fork of the now discontinued `Druid` UI toolkit.
//! * Rendering is provided by [Vello], a high performance GPU compute-centric 2D renderer.
//! * GPU compute infrastructure is provided by wgpu.
//! * Text support is provided by [Parley], [Fontique], [swash], and [skrifa].
//! * Accessibility is provided by [AccessKit].
//! * Window handling is provided by [winit].
//!
//! `Xilem` can currently be considered to be in an alpha state. Lots of things need improvements.
//!
//! ## Example
//! The simplest app looks like this:
//! ```rust,no_run
//! use winit::error::EventLoopError;
//! use xilem::view::{button, flex, label};
//! use xilem::{EventLoop, WidgetView, Xilem};
//!
//! #[derive(Default, Debug)]
//! struct AppState {
//!     num: i32,
//! }
//!
//! fn app_logic(data: &mut AppState) -> impl WidgetView<AppState> {
//!     flex((label(format!("{}", data.num)), button("increment", |data: &mut AppState| data.num+=1)))
//! }
//!
//! fn main() -> Result<(), EventLoopError> {
//!     let app = Xilem::new(AppState::default(), app_logic);
//!     app.run_windowed(EventLoop::with_user_event(), "Counter".into())?;
//!     Ok(())
//! }
//! ```
//! More examples available [here](https://github.com/linebender/xilem/tree/main/xilem/examples).
//!
//! ## View elements
//! The primitives your `Xilem` app’s view tree will generally be constructed from:
//! - [`flex`]: layout defines how items will be arranged in rows or columns.
//! - [`grid`]: layout divides a window into regions and defines the relationship
//!   between inner elements in terms of size and position.
//! - [`lens`]: an adapter which allows using a component which only uses one field
//!   of the current state.
//! - [`map action`]: provides a message that the parent view has to handle
//!   to update the state.
//! - [`adapt`]: the most flexible but also most verbose way to modularize the views
//!   by state and action.
//! - [`sized box`]: forces its child to have a specific width and/or height.
//! - [`button`]: basic button element.
//! - [`checkbox`]: an element which can be in checked and unchecked state.
//! - [`image`]: displays the bitmap `image`.
//! - [`label`]: a non-interactive text element.
//! - [`portal`]: a view which puts `child` into a scrollable region.
//! - [`progress bar`]: progress bar element.
//! - [`prose`]: displays immutable text which can be selected within.
//! - [`spinner`]: can be used to display that progress is happening on some process.
//! - [`task`]: launch a task which will run until the view is no longer in the tree.
//! - [`textbox`]: The textbox widget displays text which can be edited by the user.
//! - [`variable label`]: displays non-editable text, with a variable [weight].
//! - [`zstack`]: an element that lays out its children on top of each other.
//!
//! [architecture for UI in Rust]: https://raphlinus.github.io/rust/gui/2022/05/07/ui-architecture.html
//! [winit]: https://crates.io/crates/winit
//! [Druid]: https://crates.io/crates/druid
//! [Masonry]: https://crates.io/crates/masonry
//! [Vello]: https://crates.io/crates/vello
//! [Parley]: https://crates.io/crates/parley
//! [Fontique]: https://crates.io/crates/fontique
//! [swash]: https://crates.io/crates/swash
//! [skrifa]: https://crates.io/crates/skrifa
//! [AccessKit]: https://crates.io/crates/accesskit
//! [`flex`]: crate::view::flex
//! [`grid`]: crate::view::grid
//! [`lens`]: core::lens
//! [`map state`]: core::map_state
//! [`map action`]: core::map_action
//! [`adapt`]: core::adapt
//! [`sized box`]: crate::view::sized_box
//! [`button`]: crate::view::button
//! [`checkbox`]: crate::view::checkbox
//! [`image`]: crate::view::image
//! [`label`]: crate::view::label
//! [`portal`]: crate::view::portal
//! [`progress bar`]: crate::view::progress_bar
//! [`prose`]: crate::view::prose
//! [`spinner`]: crate::view::spinner
//! [`task`]: crate::view::task
//! [`textbox`]: crate::view::textbox
//! [`variable label`]: crate::view::variable_label
//! [`zstack`]: crate::view::zstack
//! [weight]: masonry::FontWeight

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

use masonry::dpi::LogicalSize;
use masonry::widget::{RootWidget, WidgetMut};
use masonry::{event_loop_runner, Widget, WidgetId, WidgetPod};
use winit::error::EventLoopError;
use winit::window::{Window, WindowAttributes};

use crate::core::{
    AsyncCtx, MessageResult, Mut, RawProxy, SuperElement, View, ViewElement, ViewId,
    ViewPathTracker, ViewSequence,
};
pub use masonry::event_loop_runner::{EventLoop, EventLoopBuilder};
pub use masonry::{dpi, palette, Affine, Color, FontWeight, TextAlignment, Vec2};
pub use xilem_core as core;

/// Tokio is the async runner used with Xilem.
pub use tokio;

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
    /// Run app with default windows attributes.
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
    /// Run app with custom windows attributes.
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
        event_loop_runner::run_with(event_loop, window_attributes, root_widget, driver, bg_color)
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
        let root_widget = RootWidget::from_pod(pod.inner);
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

/// A container for a [Masonry](masonry) widget to be used with Xilem.
///
/// Equivalent to [`WidgetPod<W>`], but in the [`xilem`](crate) crate to work around the orphan rule.
pub struct Pod<W: Widget> {
    pub inner: WidgetPod<W>,
    // TODO: Maybe this should just be a (WidgetId, W) pair.
}

impl<W: Widget> ViewElement for Pod<W> {
    type Mut<'a> = WidgetMut<'a, W>;
}

impl<W: Widget> SuperElement<Pod<W>, ViewCtx> for Pod<Box<dyn Widget>> {
    fn upcast(ctx: &mut ViewCtx, child: Pod<W>) -> Self {
        ctx.boxed_pod(child)
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
    type Widget: Widget;

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
}

impl<V, State, Action, W> WidgetView<State, Action> for V
where
    V: View<State, Action, ViewCtx, Element = Pod<W>> + Send + Sync,
    W: Widget,
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
    pub fn new_pod<W: Widget>(&mut self, widget: W) -> Pod<W> {
        Pod {
            inner: WidgetPod::new(widget),
        }
    }

    pub fn new_pod_with_transform<W: Widget>(&mut self, widget: W, transform: Affine) -> Pod<W> {
        Pod {
            inner: WidgetPod::new_with_transform(widget, transform),
        }
    }

    pub fn boxed_pod<W: Widget>(&mut self, pod: Pod<W>) -> Pod<Box<dyn Widget>> {
        Pod {
            inner: pod.inner.boxed(),
        }
    }

    pub fn with_leaf_action_widget<E: Widget>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Pod<E>,
    ) -> (Pod<E>, ()) {
        (self.with_action_widget(f), ())
    }

    pub fn with_action_widget<E: Widget>(&mut self, f: impl FnOnce(&mut Self) -> Pod<E>) -> Pod<E> {
        let value = f(self);
        let id = value.inner.id();
        self.record_action(id);
        value
    }

    /// Record that the actions from the widget `id` should be routed to this view.
    pub fn record_action(&mut self, id: WidgetId) {
        let path = self.id_path.clone();
        self.widget_map.insert(id, path);
    }

    pub fn teardown_leaf<E: Widget>(&mut self, widget: WidgetMut<E>) {
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
