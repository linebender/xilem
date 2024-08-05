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

use core::{
    Adapt, AdaptThunk, AnyElement, AnyView, MapAction, MapState, MessageResult, SuperElement, View,
    ViewElement,
};
use std::any::Any;
use wasm_bindgen::UnwrapThrowExt;
use web_sys::wasm_bindgen::JsCast;

/// The HTML namespace
pub const HTML_NS: &str = "http://www.w3.org/1999/xhtml";
/// The SVG namespace
pub const SVG_NS: &str = "http://www.w3.org/2000/svg";
/// The MathML namespace
pub const MATHML_NS: &str = "http://www.w3.org/1998/Math/MathML";

mod after_update;
mod app;
mod attribute;
mod attribute_value;
mod class;
mod context;
mod element_props;
mod events;
mod message;
mod one_of;
mod optional_action;
mod pointer;
mod style;
mod text;
mod vec_splice;
mod vecmap;

pub mod concurrent;
pub mod elements;
pub mod interfaces;
pub mod svg;

pub use after_update::{
    after_build, after_rebuild, before_teardown, AfterBuild, AfterRebuild, BeforeTeardown,
};
pub use app::App;
pub use attribute::{Attr, Attributes, ElementWithAttributes, WithAttributes};
pub use attribute_value::{AttributeValue, IntoAttributeValue};
pub use class::{AsClassIter, Class, Classes, ElementWithClasses, WithClasses};
pub use context::ViewCtx;
pub use element_props::ElementProps;
pub use message::{DynMessage, Message};
pub use optional_action::{Action, OptionalAction};
pub use pointer::{Pointer, PointerDetails, PointerMsg};
pub use style::{style, ElementWithStyle, IntoStyles, Style, Styles, WithStyle};
pub use xilem_core as core;
use xilem_core::ViewSequence;

/// A trait used for type erasure of [`DomNode`]s
/// It is e.g. used in [`AnyPod`]
pub trait AnyNode: AsRef<web_sys::Node> + 'static {
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<N: AsRef<web_sys::Node> + Any> AnyNode for N {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// A trait to represent DOM nodes, which can optionally have associated `props` that are applied while building/rebuilding the views
pub trait DomNode<P>: AnyNode + 'static {
    fn apply_props(&self, props: &mut P);
}

/// Syntax sugar for adding a type bound on the `ViewElement` of a view, such that both, [`ViewElement`] and [`ViewElement::Mut`] have the same [`AsRef`] type
pub trait ElementAsRef<E>: for<'a> ViewElement<Mut<'a>: AsRef<E>> + AsRef<E> {}

impl<E, T> ElementAsRef<E> for T
where
    T: ViewElement + AsRef<E>,
    for<'a> T::Mut<'a>: AsRef<E>,
{
}

/// A view which can have any [`DomView`] type, see [`AnyView`] for more details.
pub type AnyDomView<State, Action = ()> = dyn AnyView<State, Action, ViewCtx, AnyPod, DynMessage>;

/// The central [`View`] derived trait to represent DOM nodes in `xilem_web`, it's the base for all [`View`]s in `xilem_web`
pub trait DomView<State, Action = ()>:
    View<State, Action, ViewCtx, DynMessage, Element = Pod<Self::DomNode, Self::Props>>
{
    type DomNode: DomNode<Self::Props>;
    type Props;

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
    fn after_build<F>(self, callback: F) -> AfterBuild<Self, F>
    where
        Self: Sized,
        F: Fn(&Self::DomNode),
    {
        after_build(self, callback)
    }

    /// See [`after_rebuild`](`after_update::after_rebuild`)
    fn after_rebuild<F>(self, callback: F) -> AfterRebuild<Self, F>
    where
        Self: Sized,
        F: Fn(&Self::DomNode),
    {
        after_rebuild(self, callback)
    }

    /// See [`before_teardown`](`after_update::before_teardown`)
    fn before_teardown<F>(self, callback: F) -> BeforeTeardown<Self, F>
    where
        Self: Sized,
        F: Fn(&Self::DomNode),
    {
        before_teardown(self, callback)
    }

    /// See [`map_state`](`core::map_state`)
    fn map_state<ParentState, F>(self, f: F) -> MapState<ParentState, State, Self, F>
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

impl<V, State, Action, W, P> DomView<State, Action> for V
where
    V: View<State, Action, ViewCtx, DynMessage, Element = Pod<W, P>>,
    W: DomNode<P>,
{
    type DomNode = W;
    type Props = P;
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

/// A container, which holds the actual DOM node, and associated props, such as attributes or classes.
/// These attributes are not directly set on the DOM node to avoid mutating or reading from the DOM tree unnecessarily, and to have more control over the whole update flow.
pub struct Pod<E, P> {
    pub node: E,
    pub props: P,
}

/// Type-erased [`Pod`], it's used for example as intermediate representation for children of a DOM node
pub type AnyPod = Pod<DynNode, Box<dyn Any>>;

impl<E: DomNode<P>, P: 'static> Pod<E, P> {
    pub fn into_dyn_node(node: E, mut props: P) -> AnyPod {
        node.apply_props(&mut props);
        Pod {
            node: DynNode {
                inner: Box::new(node),
            },
            props: Box::new(props),
        }
    }
}

impl<E: DomNode<P>, P: 'static> ViewElement for Pod<E, P> {
    type Mut<'a> = PodMut<'a, E, P>;
}

impl<E: DomNode<P>, P: 'static> SuperElement<Pod<E, P>> for AnyPod {
    fn upcast(child: Pod<E, P>) -> Self {
        Pod::into_dyn_node(child.node, child.props)
    }

    fn with_downcast_val<R>(
        mut this: Self::Mut<'_>,
        f: impl FnOnce(PodMut<'_, E, P>) -> R,
    ) -> (Self::Mut<'_>, R) {
        let downcast = this.downcast();
        let ret = f(downcast);
        (this, ret)
    }
}

impl<E: DomNode<P>, P: 'static> AnyElement<Pod<E, P>> for AnyPod {
    fn replace_inner(mut this: Self::Mut<'_>, child: Pod<E, P>) -> Self::Mut<'_> {
        Pod::replace_inner(&mut this, child);
        this
    }
}

impl AnyPod {
    pub(crate) fn replace_inner<E: DomNode<P>, P: 'static>(
        this: &mut PodMut<'_, DynNode, Box<dyn Any>>,
        node: Pod<E, P>,
    ) {
        this.parent
            .replace_child(node.node.as_ref(), this.node.as_ref())
            .unwrap_throw();
        this.node.inner = Box::new(node.node);
        *this.props = Box::new(node.props);
    }

    fn as_mut<'a>(
        &'a mut self,
        parent: &'a web_sys::Node,
        was_removed: bool,
    ) -> PodMut<'a, DynNode, Box<dyn Any>> {
        PodMut::new(&mut self.node, &mut self.props, parent, was_removed)
    }
}

/// A type erased DOM node, used in [`AnyPod`]
pub struct DynNode {
    inner: Box<dyn AnyNode>,
}

impl AsRef<web_sys::Node> for DynNode {
    fn as_ref(&self) -> &web_sys::Node {
        (*self.inner).as_ref()
    }
}

impl DomNode<Box<dyn Any>> for DynNode {
    fn apply_props(&self, _props: &mut Box<dyn Any>) {
        // TODO this is probably not optimal, as misleading, this is only implemented for concrete (non-type-erased) elements
        // I do *think* it's necessary as method on the trait because of the Drop impl (and not having specialization there)
    }
}

/// The mutable representation of [`Pod`].
/// This is a container which contains info of what has changed and provides mutable access to the underlying element and its props
/// When it's dropped all changes are applied to the underlying DOM node
pub struct PodMut<'a, E: DomNode<P>, P> {
    node: &'a mut E,
    props: &'a mut P,
    parent: &'a web_sys::Node,
    was_removed: bool,
}

impl<'a, E: DomNode<P>, P> PodMut<'a, E, P> {
    fn new(
        node: &'a mut E,
        props: &'a mut P,
        parent: &'a web_sys::Node,
        was_removed: bool,
    ) -> PodMut<'a, E, P> {
        PodMut {
            node,
            props,
            parent,
            was_removed,
        }
    }
}

impl PodMut<'_, DynNode, Box<dyn Any>> {
    fn downcast<E: DomNode<P>, P: 'static>(&mut self) -> PodMut<'_, E, P> {
        PodMut::new(
            self.node.inner.as_any_mut().downcast_mut().unwrap(),
            self.props.downcast_mut().unwrap(),
            self.parent,
            false,
        )
    }
}

impl<E: DomNode<P>, P> Drop for PodMut<'_, E, P> {
    fn drop(&mut self) {
        if !self.was_removed {
            self.node.apply_props(self.props);
        }
    }
}

impl<T, E: AsRef<T> + DomNode<P>, P> AsRef<T> for Pod<E, P> {
    fn as_ref(&self) -> &T {
        <E as AsRef<T>>::as_ref(&self.node)
    }
}

impl<T, E: AsRef<T> + DomNode<P>, P> AsRef<T> for PodMut<'_, E, P> {
    fn as_ref(&self) -> &T {
        <E as AsRef<T>>::as_ref(self.node)
    }
}

impl DomNode<ElementProps> for web_sys::Element {
    fn apply_props(&self, props: &mut ElementProps) {
        props.update_element(self);
    }
}

impl DomNode<()> for web_sys::Text {
    fn apply_props(&self, (): &mut ()) {}
}

/// Helper to get the HTML document body element
pub fn document_body() -> web_sys::HtmlElement {
    document().body().expect("HTML document missing body")
}

/// Helper to get the HTML document
pub fn document() -> web_sys::Document {
    let window = web_sys::window().expect("no global `window` exists");
    window.document().expect("should have a document on window")
}

/// Helper to get a DOM element by id
pub fn get_element_by_id(id: &str) -> web_sys::HtmlElement {
    document()
        .get_element_by_id(id)
        .unwrap()
        .dyn_into()
        .unwrap()
}

// TODO specialize some of these elements, maybe via features?
macro_rules! impl_dom_node_for_elements {
    ($($ty:ident, )*) => {$(
        impl DomNode<ElementProps> for web_sys::$ty {
            fn apply_props(&self, props: &mut ElementProps) {
                props.update_element(self);
            }
        }

        impl From<Pod<web_sys::Element, ElementProps>> for Pod<web_sys::$ty, ElementProps> {
            fn from(value: Pod<web_sys::Element, ElementProps>) -> Self {
                Self {
                    node: value.node.dyn_into().unwrap_throw(),
                    props: value.props,
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
    HtmlInputElement,
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
