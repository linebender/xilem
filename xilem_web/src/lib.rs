// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

// https://linebender.org/blog/doc-include
//! <!-- This license link is in a .rustdoc-hidden section, but we may as well give the correct link -->
//! [LICENSE]: https://github.com/linebender/xilem/blob/main/xilem_web/LICENSE
//!
//! <!-- intra-doc-links go here -->
//!
//! <style>
//! .rustdoc-hidden { display: none; }
//! </style>
#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
// LINEBENDER LINT SET - v1
// See https://linebender.org/wiki/canonical-lints/
// These lints aren't included in Cargo.toml because they
// shouldn't apply to examples and tests
#![warn(unused_crate_dependencies)]
#![warn(clippy::print_stdout, clippy::print_stderr)]
// TODO: Remove any items listed as "Deferred"
#![cfg_attr(test, expect(clippy::print_stdout, reason = "Deferred: Noisy"))]
#![expect(
    rustdoc::broken_intra_doc_links,
    reason = "Deferred: Noisy. Tracked in https://github.com/linebender/xilem/issues/449"
)]
#![cfg_attr(not(debug_assertions), expect(unused, reason = "Deferred: Noisy"))]
#![expect(let_underscore_drop, reason = "Deferred: Noisy")]
#![expect(missing_debug_implementations, reason = "Deferred: Noisy")]
#![expect(unused_qualifications, reason = "Deferred: Noisy")]
#![expect(single_use_lifetimes, reason = "Deferred: Noisy")]
#![expect(clippy::exhaustive_enums, reason = "Deferred: Noisy")]
#![expect(clippy::match_same_arms, reason = "Deferred: Noisy")]
#![expect(clippy::cast_possible_truncation, reason = "Deferred: Noisy")]
#![expect(clippy::missing_assert_message, reason = "Deferred: Noisy")]
#![expect(clippy::return_self_not_must_use, reason = "Deferred: Noisy")]
#![expect(elided_lifetimes_in_paths, reason = "Deferred: Noisy")]
#![expect(clippy::use_self, reason = "Deferred: Noisy")]
// expect doesn't work here: https://github.com/rust-lang/rust/pull/130025
#![allow(missing_docs, reason = "We have many as-yet undocumented items")]
#![expect(unreachable_pub, reason = "Potentially controversial code style")]
#![expect(
    unnameable_types,
    reason = "Requires lint_reasons rustc feature for exceptions"
)]
#![expect(clippy::todo, reason = "We have a lot of 'real' todos")]
#![expect(clippy::missing_panics_doc, reason = "Can be quite noisy?")]
#![expect(
    clippy::shadow_unrelated,
    reason = "Potentially controversial code style"
)]
#![expect(clippy::allow_attributes, reason = "Deferred: Noisy")]
#![expect(clippy::allow_attributes_without_reason, reason = "Deferred: Noisy")]

use std::{any::Any, ops::Deref as _};

use web_sys::wasm_bindgen::JsCast;

/// The HTML namespace
pub const HTML_NS: &str = "http://www.w3.org/1999/xhtml";
/// The SVG namespace
pub const SVG_NS: &str = "http://www.w3.org/2000/svg";
/// The MathML namespace
pub const MATHML_NS: &str = "http://www.w3.org/1998/Math/MathML";

mod after_update;
mod app;
mod attribute_value;
mod context;
mod dom_helpers;
mod message;
mod one_of;
mod optional_action;
mod pod;
mod pointer;
mod templated;
mod text;
mod vec_splice;
mod vecmap;

pub mod concurrent;
pub mod diff;
pub mod elements;
pub mod events;
pub mod interfaces;
pub mod modifiers;
pub mod props;
pub mod svg;

pub use self::{
    after_update::{
        after_build, after_rebuild, before_teardown, AfterBuild, AfterRebuild, BeforeTeardown,
    },
    app::App,
    attribute_value::{AttributeValue, IntoAttributeValue},
    context::{MessageThunk, ViewCtx},
    dom_helpers::{document, document_body, get_element_by_id, input_event_target_value},
    message::{DynMessage, Message},
    optional_action::{Action, OptionalAction},
    pod::{AnyPod, Pod, PodFlags, PodMut},
    pointer::{Pointer, PointerDetails, PointerMsg},
};

pub use templated::{templated, Templated};

pub use xilem_core as core;

use core::{Adapt, AdaptThunk, AnyView, MapAction, MapState, MessageResult, View, ViewSequence};

/// A trait used for type erasure of [`DomNode`]s
/// It is e.g. used in [`AnyPod`]
pub trait AnyNode: AsRef<web_sys::Node> + 'static {
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<N: AsRef<web_sys::Node> + 'static> AnyNode for N {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// A trait to represent DOM nodes, which can optionally have associated `props` that are applied while building/rebuilding the views
pub trait DomNode: AnyNode {
    type Props: 'static;
    /// When this is called any accumulated (virtual) `props` changes are applied to the underlying DOM node.
    /// Where `props` stands for all kinds of modifiable properties, like attributes, styles, classes or more specific properties related to an element.
    fn apply_props(&self, props: &mut Self::Props, flags: &mut PodFlags);
}

/// A view which can have any [`DomView`] type, see [`AnyView`] for more details.
pub type AnyDomView<State, Action = ()> = dyn AnyView<State, Action, ViewCtx, AnyPod, DynMessage>;

/// The central [`View`] derived trait to represent DOM nodes in `xilem_web`, it's the base for all [`View`]s in `xilem_web`
pub trait DomView<State, Action = ()>:
    View<State, Action, ViewCtx, DynMessage, Element = Pod<Self::DomNode>>
{
    type DomNode: DomNode;

    /// Returns a boxed type erased [`AnyDomView`]
    ///
    /// # Examples
    /// ```
    /// use xilem_web::{elements::html::div, DomView};
    ///
    /// # fn view<State: 'static>() -> impl DomView<State> {
    /// div("a label").boxed()
    /// # }
    /// ```
    fn boxed(self) -> Box<AnyDomView<State, Action>>
    where
        State: 'static,
        Action: 'static,
        Self: Sized,
    {
        Box::new(self)
    }

    /// See [`adapt`](`core::adapt`)
    fn adapt<ParentState, ParentAction, ProxyFn>(
        self,
        f: ProxyFn,
    ) -> Adapt<ParentState, ParentAction, State, Action, ViewCtx, Self, DynMessage, ProxyFn>
    where
        State: 'static,
        Action: 'static,
        ParentState: 'static,
        ParentAction: 'static,
        Self: Sized,
        ProxyFn: Fn(
                &mut ParentState,
                AdaptThunk<State, Action, ViewCtx, Self, DynMessage>,
            ) -> MessageResult<ParentAction, DynMessage>
            + 'static,
    {
        core::adapt(self, f)
    }

    /// See [`after_build`](`after_update::after_build`)
    fn after_build<F>(self, callback: F) -> AfterBuild<State, Action, Self, F>
    where
        State: 'static,
        Action: 'static,
        Self: Sized,
        F: Fn(&Self::DomNode) + 'static,
    {
        after_build(self, callback)
    }

    /// See [`after_rebuild`](`after_update::after_rebuild`)
    fn after_rebuild<F>(self, callback: F) -> AfterRebuild<State, Action, Self, F>
    where
        State: 'static,
        Action: 'static,
        Self: Sized,
        F: Fn(&Self::DomNode) + 'static,
    {
        after_rebuild(self, callback)
    }

    /// See [`before_teardown`](`after_update::before_teardown`)
    fn before_teardown<F>(self, callback: F) -> BeforeTeardown<State, Action, Self, F>
    where
        State: 'static,
        Action: 'static,
        Self: Sized,
        F: Fn(&Self::DomNode) + 'static,
    {
        before_teardown(self, callback)
    }

    /// See [`map_state`](`core::map_state`)
    fn map_state<ParentState, F>(
        self,
        f: F,
    ) -> MapState<Self, F, ParentState, State, Action, ViewCtx, DynMessage>
    where
        State: 'static,
        ParentState: 'static,
        Self: Sized,
        F: Fn(&mut ParentState) -> &mut State + 'static,
    {
        core::map_state(self, f)
    }

    /// See [`map_action`](`core::map_action`)
    fn map_action<ParentAction, F>(self, f: F) -> MapAction<State, ParentAction, Action, Self, F>
    where
        State: 'static,
        ParentAction: 'static,
        Action: 'static,
        Self: Sized,
        F: Fn(&mut State, Action) -> ParentAction + 'static,
    {
        core::map_action(self, f)
    }
}

impl<V, State, Action, N> DomView<State, Action> for V
where
    V: View<State, Action, ViewCtx, DynMessage, Element = Pod<N>>,
    N: DomNode,
{
    type DomNode = N;
}

/// An ordered sequence of views, or sometimes also called fragment, it's used for `0..N` [`DomView`]s.
/// See [`ViewSequence`] for more technical details.
///
/// # Examples
///
/// ```
/// fn huzzah(clicks: i32) -> impl xilem_web::DomFragment<i32> {
///     (clicks >= 5).then_some("Huzzah, clicked at least 5 times")
/// }
/// ```
pub trait DomFragment<State, Action = ()>:
    ViewSequence<State, Action, ViewCtx, AnyPod, DynMessage>
{
}

impl<V, State, Action> DomFragment<State, Action> for V where
    V: ViewSequence<State, Action, ViewCtx, AnyPod, DynMessage>
{
}

impl DomNode for Box<dyn AnyNode> {
    type Props = Box<dyn Any>;

    fn apply_props(&self, _props: &mut Self::Props, _: &mut PodFlags) {
        // TODO this is probably not optimal, as misleading, this is only implemented for concrete (non-type-erased) elements
        // I do *think* it's necessary as method on the trait because of the Drop impl (and not having specialization there)
    }
}

impl AsRef<web_sys::Node> for Box<dyn AnyNode> {
    fn as_ref(&self) -> &web_sys::Node {
        self.deref().as_ref()
    }
}

impl DomNode for web_sys::Element {
    type Props = props::Element;

    fn apply_props(&self, props: &mut props::Element, flags: &mut PodFlags) {
        props.update_element(self, flags);
    }
}

impl DomNode for web_sys::Text {
    type Props = ();

    fn apply_props(&self, (): &mut (), _flags: &mut PodFlags) {}
}

impl DomNode for web_sys::HtmlInputElement {
    type Props = props::HtmlInputElement;

    fn apply_props(&self, props: &mut props::HtmlInputElement, flags: &mut PodFlags) {
        props.update_element(self, flags);
    }
}

pub trait FromWithContext<T>: Sized {
    fn from_with_ctx(value: T, ctx: &mut ViewCtx) -> Self;
}

impl<T> FromWithContext<T> for T {
    fn from_with_ctx(value: T, _ctx: &mut ViewCtx) -> Self {
        value
    }
}

// TODO specialize some of these elements, maybe via features?
macro_rules! impl_dom_node_for_elements {
    ($($ty:ident, )*) => {$(
        impl DomNode for web_sys::$ty {
            type Props = props::Element;

            fn apply_props(&self, props: &mut props::Element, flags: &mut PodFlags) {
                props.update_element(self, flags);
            }
        }

        impl FromWithContext<Pod<web_sys::Element>> for Pod<web_sys::$ty> {
            fn from_with_ctx(value: Pod<web_sys::Element>, _ctx: &mut ViewCtx) -> Self {
                Self {
                    node: value.node.unchecked_into(),
                    props: value.props,
                    flags: value.flags,
                }
            }
        }
    )*};
}

impl_dom_node_for_elements!(
    // Element,
    HtmlElement,
    HtmlAnchorElement,
    HtmlAreaElement,
    // HtmlBaseElement, TODO include metadata?
    // HtmlBodyElement, TODO include body element?
    HtmlBrElement,
    HtmlButtonElement,
    HtmlCanvasElement,
    HtmlDataElement,
    HtmlDataListElement,
    HtmlDetailsElement,
    HtmlDialogElement,
    // HtmlDirectoryElement, deprecated
    HtmlDivElement,
    HtmlDListElement,
    // HtmlUnknownElement, useful at all?
    HtmlEmbedElement,
    HtmlFieldSetElement,
    // HtmlFontElement, deprecated
    HtmlFormElement,
    // HtmlFrameElement, deprecated
    // HtmlFrameSetElement, deprecacted
    // HtmlHeadElement, TODO include metadata?
    HtmlHeadingElement,
    HtmlHrElement,
    // HtmlHtmlElement, TODO include metadata?
    HtmlIFrameElement,
    HtmlImageElement,
    // HtmlInputElement, has specialized impl
    HtmlLabelElement,
    HtmlLegendElement,
    HtmlLiElement,
    HtmlLinkElement,
    HtmlMapElement,
    HtmlMediaElement,
    HtmlAudioElement,
    HtmlVideoElement,
    HtmlMenuElement,
    // HtmlMenuItemElement, deprecated
    // HtmlMetaElement, TODO include metadata?
    HtmlMeterElement,
    HtmlModElement,
    HtmlObjectElement,
    HtmlOListElement,
    HtmlOptGroupElement,
    HtmlOptionElement,
    HtmlOutputElement,
    HtmlParagraphElement,
    // HtmlParamElement, deprecated
    HtmlPictureElement,
    HtmlPreElement,
    HtmlProgressElement,
    HtmlQuoteElement,
    HtmlScriptElement,
    HtmlSelectElement,
    HtmlSlotElement,
    HtmlSourceElement,
    HtmlSpanElement,
    // HtmlStyleElement, TODO include metadata?
    HtmlTableCaptionElement,
    HtmlTableCellElement,
    HtmlTableColElement,
    HtmlTableElement,
    HtmlTableRowElement,
    HtmlTableSectionElement,
    HtmlTemplateElement,
    HtmlTimeElement,
    HtmlTextAreaElement,
    // HtmlTitleElement, TODO include metadata?
    HtmlTrackElement,
    HtmlUListElement,
    SvgElement,
    SvgAnimationElement,
    SvgAnimateElement,
    SvgAnimateMotionElement,
    SvgAnimateTransformElement,
    SvgSetElement,
    SvgClipPathElement,
    SvgComponentTransferFunctionElement,
    SvgfeFuncAElement,
    SvgfeFuncBElement,
    SvgfeFuncGElement,
    SvgfeFuncRElement,
    SvgDescElement,
    SvgFilterElement,
    SvgGradientElement,
    SvgLinearGradientElement,
    SvgRadialGradientElement,
    SvgGraphicsElement,
    SvgDefsElement,
    SvgForeignObjectElement,
    SvgGeometryElement,
    SvgCircleElement,
    SvgEllipseElement,
    SvgLineElement,
    SvgPathElement,
    SvgPolygonElement,
    SvgPolylineElement,
    SvgRectElement,
    SvgImageElement,
    SvgSwitchElement,
    SvgTextContentElement,
    SvgTextPathElement,
    SvgTextPositioningElement,
    SvgTextElement,
    SvgtSpanElement,
    SvgUseElement,
    SvgaElement,
    SvggElement,
    SvgsvgElement,
    SvgMarkerElement,
    SvgMaskElement,
    SvgMetadataElement,
    SvgPatternElement,
    SvgScriptElement,
    SvgStopElement,
    SvgStyleElement,
    SvgSymbolElement,
    SvgTitleElement,
    SvgViewElement,
    SvgfeBlendElement,
    SvgfeColorMatrixElement,
    SvgfeComponentTransferElement,
    SvgfeCompositeElement,
    SvgfeConvolveMatrixElement,
    SvgfeDiffuseLightingElement,
    SvgfeDisplacementMapElement,
    SvgfeDistantLightElement,
    SvgfeDropShadowElement,
    SvgfeFloodElement,
    SvgfeGaussianBlurElement,
    SvgfeImageElement,
    SvgfeMergeElement,
    SvgfeMergeNodeElement,
    SvgfeMorphologyElement,
    SvgfeOffsetElement,
    SvgfePointLightElement,
    SvgfeSpecularLightingElement,
    SvgfeSpotLightElement,
    SvgfeTileElement,
    SvgfeTurbulenceElement,
    SvgmPathElement,
);
