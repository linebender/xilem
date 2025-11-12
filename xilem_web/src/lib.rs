// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

// After you edit the crate's doc comment, run this command, then check README.md for any missing links
// cargo rdme --workspace-project=xilem_web

//! This is a prototype implementation of the Xilem architecture (through [Xilem Core][]) using DOM elements as Xilem elements (unfortunately the two concepts have the same name).
//!
//! # Quickstart
//!
//! The easiest way to start, is to use [Trunk][] within some of the examples (see the `web_examples/` directory).
//! Run `trunk serve`, then navigate the browser to the link provided (usually <http://localhost:8080>).
//!
//! ## Example
//!
//! A minimal example to run an application with `xilem_web`:
//!
//! ```rust,no_run
//! use xilem_web::{
//!     document_body,
//!     elements::html::{button, div, p},
//!     interfaces::{Element as _, HtmlDivElement},
//!     App,
//!     core::Edit,
//! };
//!
//! fn app_logic(clicks: &mut u32) -> impl HtmlDivElement<Edit<u32>> + use<> {
//!     div((
//!         button(format!("clicked {clicks} times")).on_click(|clicks: &mut u32, _event| *clicks += 1),
//!         (*clicks >= 5).then_some(p("Huzzah, clicked at least 5 times")),
//!     ))
//! }
//!
//! pub fn main() {
//!     let clicks = 0;
//!     App::new(document_body(), clicks, app_logic).run();
//! }
//! ```
//!
//! [Trunk]: https://trunkrs.dev/
//! [Xilem Core]: xilem_core

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
// TODO: Remove any items listed as "Deferred"
#![cfg_attr(test, expect(clippy::print_stdout, reason = "Deferred: Noisy"))]
#![expect(missing_debug_implementations, reason = "Deferred: Noisy")]
#![expect(clippy::cast_possible_truncation, reason = "Deferred: Noisy")]
#![expect(clippy::missing_assert_message, reason = "Deferred: Noisy")]
#![expect(missing_docs, reason = "We have many as-yet undocumented items")]
#![expect(unreachable_pub, reason = "Potentially controversial code style")]
#![expect(
    unnameable_types,
    reason = "Requires lint_reasons rustc feature for exceptions"
)]
#![expect(clippy::todo, reason = "We have a lot of 'real' todos")]

use std::any::Any;
use std::ops::Deref as _;

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

pub use self::after_update::{
    AfterBuild, AfterRebuild, BeforeTeardown, after_build, after_rebuild, before_teardown,
};
pub use self::app::App;
pub use self::attribute_value::{AttributeValue, IntoAttributeValue};
pub use self::context::{MessageThunk, ViewCtx};
pub use self::core::DynMessage;
pub use self::dom_helpers::{document, document_body, get_element_by_id, input_event_target_value};
pub use self::optional_action::{Action, OptionalAction};
pub use self::pod::{AnyPod, Pod, PodFlags, PodMut};
pub use self::pointer::{Pointer, PointerDetails, PointerMsg};

pub use templated::{Templated, templated};

pub use xilem_core as core;

use core::{AnyView, Arg, View, ViewArgument, ViewSequence};

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
pub type AnyDomView<State, Action = ()> = dyn AnyView<State, Action, ViewCtx, AnyPod>;

/// The central [`View`] derived trait to represent DOM nodes in `xilem_web`, it's the base for all [`View`]s in `xilem_web`
pub trait DomView<State: ViewArgument, Action = ()>:
    View<State, Action, ViewCtx, Element = Pod<Self::DomNode>>
{
    type DomNode: DomNode;

    /// Returns a boxed type erased [`AnyDomView`]
    ///
    /// # Examples
    /// ```
    /// use xilem_web::{elements::html::div, DomView, core::ViewArgument};
    ///
    /// # fn view<State: ViewArgument>() -> impl DomView<State> {
    /// div("a label").boxed()
    /// # }
    /// ```
    fn boxed(self) -> Box<AnyDomView<State, Action>>
    where
        Action: 'static,
        Self: Sized,
    {
        Box::new(self)
    }

    /// See [`after_build`](`after_update::after_build`)
    fn after_build<F>(self, callback: F) -> AfterBuild<State, Action, Self, F>
    where
        Action: 'static,
        Self: Sized,
        F: Fn(&Self::DomNode) + 'static,
    {
        after_build(self, callback)
    }

    /// See [`after_rebuild`](`after_update::after_rebuild`)
    fn after_rebuild<F>(self, callback: F) -> AfterRebuild<State, Action, Self, F>
    where
        Action: 'static,
        Self: Sized,
        F: Fn(&Self::DomNode) + 'static,
    {
        after_rebuild(self, callback)
    }

    /// See [`before_teardown`](`after_update::before_teardown`)
    fn before_teardown<F>(self, callback: F) -> BeforeTeardown<State, Action, Self, F>
    where
        Action: 'static,
        Self: Sized,
        F: Fn(&Self::DomNode) + 'static,
    {
        before_teardown(self, callback)
    }
}

impl<V, State: ViewArgument, Action, N> DomView<State, Action> for V
where
    V: View<State, Action, ViewCtx, Element = Pod<N>>,
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
/// # use xilem_web::core::Edit;
/// fn huzzah(clicks: i32) -> impl xilem_web::DomFragment<Edit<i32>> {
///     (clicks >= 5).then_some("Huzzah, clicked at least 5 times")
/// }
/// ```
pub trait DomFragment<State: ViewArgument, Action = ()>:
    ViewSequence<State, Action, ViewCtx, AnyPod>
{
}

impl<V, State: ViewArgument, Action> DomFragment<State, Action> for V where
    V: ViewSequence<State, Action, ViewCtx, AnyPod>
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
