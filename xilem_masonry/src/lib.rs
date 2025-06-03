// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! `xilem_masonry` provides Xilem views for the Masonry backend.
//!
//! Xilem is a portable, native UI framework written in Rust.
//! See [the Xilem documentation](https://docs.rs/xilem/latest/xilem/)
//! for details.
//!
//! [Masonry](masonry) is a foundational library for writing native GUI frameworks.
//!
//! Xilem's architecture uses lightweight view objects, diffing them to provide minimal
//! updates to a retained UI.
//!
//! `xilem_masonry` uses Masonry's widget tree as the retained UI.
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
// TODO: Remove any items listed as "Deferred"
#![cfg_attr(not(debug_assertions), allow(unused))]
#![expect(
    missing_debug_implementations,
    reason = "Deferred: Noisy. Requires same lint to be addressed in Masonry"
)]
#![expect(clippy::missing_assert_message, reason = "Deferred: Noisy")]
#![expect(elided_lifetimes_in_paths, reason = "Deferred: Noisy")]
// https://github.com/rust-lang/rust/pull/130025
#![expect(clippy::allow_attributes_without_reason, reason = "Deferred: Noisy")]

use std::collections::HashMap;
use std::sync::Arc;

use masonry::core::{
    FromDynWidget, Properties, Widget, WidgetId, WidgetMut, WidgetOptions, WidgetPod,
};

use view::{Transformed, transformed};

use crate::core::{
    AsyncCtx, MessageResult, Mut, RawProxy, SuperElement, View, ViewElement, ViewId,
    ViewPathTracker, ViewSequence,
};

pub use masonry::kurbo::{Affine, Vec2};
pub use masonry::parley::Alignment as TextAlignment;
pub use masonry::parley::style::FontWeight;
pub use masonry::peniko::{Blob, Color};
pub use masonry::widgets::{InsertNewline, LineBreaking};
pub use masonry::{dpi, palette};
pub use xilem_core as core;

/// Tokio is the async runner used with Xilem.
pub use tokio;

mod any_view;
mod one_of;
mod property_tuple;

pub mod style;
pub mod view;
pub use any_view::AnyWidgetView;
pub use property_tuple::PropertyTuple;

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
#[expect(missing_docs, reason = "TODO - Document these items")]
pub struct Pod<W: Widget + FromDynWidget + ?Sized> {
    pub widget: Box<W>,
    pub id: WidgetId,
    /// The options the widget will be created with.
    ///
    /// If changing transforms of widgets, prefer to use [`transformed`]
    /// (or [`WidgetView::transform`]).
    /// This has a protocol to ensure that multiple views changing the
    /// transform interoperate successfully.
    pub options: WidgetOptions,
    pub properties: Properties,
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
            options: WidgetOptions::default(),
            properties: Properties::new(),
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
            options: self.options,
            properties: self.properties,
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
        WidgetPod::new_with(self.widget, self.id, self.options, self.properties)
    }
    /// Finalise this `Pod` into a type-erased [`WidgetPod`].
    ///
    /// In most cases, you will use the return value for adding to a layout
    /// widget which supports heterogenous widgets.
    /// For example, [`Flex`](masonry::widgets::Flex) accepts type-erased widget pods.
    pub fn erased_widget_pod(self) -> WidgetPod<dyn Widget> {
        WidgetPod::new_with(self.widget, self.id, self.options, self.properties).erased()
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

#[expect(missing_docs, reason = "TODO - Document these items")]
pub trait WidgetView<State, Action = ()>:
    View<State, Action, ViewCtx, Element = Pod<Self::Widget>> + Send + Sync
{
    type Widget: Widget + FromDynWidget + ?Sized;

    /// Returns a boxed type erased [`AnyWidgetView`]
    ///
    /// # Examples
    /// ```
    /// use xilem_masonry::{view::label, WidgetView};
    ///
    /// # fn view<State: 'static>() -> impl WidgetView<State> + use<State> {
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
/// use xilem_masonry::{view::prose, WidgetViewSequence};
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

/// A context type passed to various methods of Xilem traits.
pub struct ViewCtx {
    /// The map from a widgets id to its position in the View tree.
    ///
    /// This includes only the widgets which might send actions
    widget_map: WidgetMap,
    id_path: Vec<ViewId>,
    proxy: Arc<dyn RawProxy>,
    runtime: tokio::runtime::Runtime,
    state_changed: bool,
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

#[expect(missing_docs, reason = "TODO - Document these items")]
impl ViewCtx {
    pub fn new(proxy: Arc<dyn RawProxy>, runtime: tokio::runtime::Runtime) -> Self {
        Self {
            widget_map: WidgetMap::default(),
            id_path: Vec::new(),
            proxy,
            runtime,
            state_changed: true,
        }
    }

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

    /// Whether the app's state changed since the last rebuild.
    ///
    /// This is useful for views whose current value depends on current app state.
    /// (That is, currently only virtual scrolling)
    pub fn state_changed(&self) -> bool {
        self.state_changed
    }

    pub fn set_state_changed(&mut self, value: bool) {
        self.state_changed = value;
    }

    pub fn teardown_leaf<W: Widget + FromDynWidget + ?Sized>(&mut self, widget: WidgetMut<W>) {
        self.widget_map.remove(&widget.ctx.widget_id());
    }

    pub fn get_id_path(&self, widget_id: WidgetId) -> Option<&Vec<ViewId>> {
        self.widget_map.get(&widget_id)
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
