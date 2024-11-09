// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

// False-positive with dev-dependencies only used in examples
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
// LINEBENDER LINT SET - v1
// See https://linebender.org/wiki/canonical-lints/
// These lints aren't included in Cargo.toml because they
// shouldn't apply to examples and tests
#![warn(unused_crate_dependencies)]
#![warn(clippy::print_stdout, clippy::print_stderr)]
#![cfg_attr(
    test,
    expect(
        unused_crate_dependencies,
        reason = "False-positive with dev-dependencies only used in examples"
    )
)]
// TODO: Remove any items listed as "Deferred"
#![deny(clippy::trivially_copy_pass_by_ref)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![cfg_attr(not(debug_assertions), allow(unused))]
#![expect(missing_debug_implementations, reason = "Deferred: Noisy")]
#![expect(unused_qualifications, reason = "Deferred: Noisy")]
#![expect(clippy::exhaustive_enums, reason = "Deferred: Noisy")]
#![expect(clippy::match_same_arms, reason = "Deferred: Noisy")]
#![expect(clippy::cast_possible_truncation, reason = "Deferred: Noisy")]
#![expect(clippy::missing_assert_message, reason = "Deferred: Noisy")]
#![expect(clippy::return_self_not_must_use, reason = "Deferred: Noisy")]
#![expect(elided_lifetimes_in_paths, reason = "Deferred: Noisy")]
#![expect(clippy::use_self, reason = "Deferred: Noisy")]
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
pub use masonry::{dpi, Color, TextAlignment, TextWeight};
pub use xilem_core as core;

/// Tokio is the async runner used with Xilem.
pub use tokio;

mod any_view;
mod driver;
mod one_of;

pub mod view;
pub use any_view::AnyWidgetView;
pub use driver::{async_action, MasonryDriver, MasonryProxy, ASYNC_MARKER_WIDGET};

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
        Xilem {
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
        let path = self.id_path.clone();
        self.widget_map.insert(id, path);
        value
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
