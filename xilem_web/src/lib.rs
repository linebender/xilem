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

use std::{
    any::Any,
    ops::{Deref as _, DerefMut as _},
};

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
mod dom_helpers;
mod element_props;
mod events;
mod message;
mod one_of;
mod optional_action;
mod pointer;
mod style;
#[cfg(feature = "hydration")]
mod templated;
mod text;
mod vec_splice;
mod vecmap;

pub mod concurrent;
pub mod elements;
pub mod interfaces;
pub mod svg;

pub use self::{
    after_update::{
        after_build, after_rebuild, before_teardown, AfterBuild, AfterRebuild, BeforeTeardown,
    },
    app::App,
    attribute::{Attr, Attributes, ElementWithAttributes, WithAttributes},
    attribute_value::{AttributeValue, IntoAttributeValue},
    class::{AsClassIter, Class, Classes, ElementWithClasses, WithClasses},
    context::{MessageThunk, ViewCtx},
    dom_helpers::{document, document_body, get_element_by_id, input_event_target_value},
    element_props::ElementProps,
    message::{DynMessage, Message},
    optional_action::{Action, OptionalAction},
    pointer::{Pointer, PointerDetails, PointerMsg},
    style::{style, ElementWithStyle, IntoStyles, Style, Styles, WithStyle},
};

#[cfg(feature = "hydration")]
pub use templated::{templated, Templated};

pub use xilem_core as core;

use xilem_core::{
    Adapt, AdaptThunk, AnyElement, AnyView, MapAction, MapState, MessageResult, SuperElement, View,
    ViewElement, ViewSequence,
};

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
    fn apply_props(&self, props: &mut Self::Props);
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

/// A container, which holds the actual DOM node, and associated props, such as attributes or classes.
///
/// These attributes are not directly set on the DOM node to avoid mutating or reading from the DOM tree unnecessarily, and to have more control over the whole update flow.
pub struct Pod<N: DomNode> {
    pub node: N,
    pub props: N::Props,
}

/// Type-erased [`Pod`], it's used for example as intermediate representation for children of a DOM node
pub type AnyPod = Pod<Box<dyn AnyNode>>;

impl<N: DomNode> Pod<N> {
    pub fn into_any_pod(node: N, mut props: N::Props) -> AnyPod {
        node.apply_props(&mut props);
        Pod {
            node: Box::new(node),
            props: Box::new(props),
        }
    }
}

impl DomNode for Box<dyn AnyNode> {
    type Props = Box<dyn Any>;

    fn apply_props(&self, _props: &mut Self::Props) {
        // TODO this is probably not optimal, as misleading, this is only implemented for concrete (non-type-erased) elements
        // I do *think* it's necessary as method on the trait because of the Drop impl (and not having specialization there)
    }
}

impl AsRef<web_sys::Node> for Box<dyn AnyNode> {
    fn as_ref(&self) -> &web_sys::Node {
        self.deref().as_ref()
    }
}

impl<N: DomNode> ViewElement for Pod<N> {
    type Mut<'a> = PodMut<'a, N>;
}

impl<N: DomNode> SuperElement<Pod<N>, ViewCtx> for AnyPod {
    fn upcast(_ctx: &mut ViewCtx, child: Pod<N>) -> Self {
        Pod::into_any_pod(child.node, child.props)
    }

    fn with_downcast_val<R>(
        mut this: Self::Mut<'_>,
        f: impl FnOnce(PodMut<'_, N>) -> R,
    ) -> (Self::Mut<'_>, R) {
        let downcast = this.downcast();
        let ret = f(downcast);
        (this, ret)
    }
}

impl<N: DomNode> AnyElement<Pod<N>, ViewCtx> for AnyPod {
    fn replace_inner(this: Self::Mut<'_>, mut child: Pod<N>) -> Self::Mut<'_> {
        child.node.apply_props(&mut child.props);
        if let Some(parent) = this.parent {
            parent
                .replace_child(child.node.as_ref(), this.node.as_ref())
                .unwrap_throw();
        }
        *this.node = Box::new(child.node);
        *this.props = Box::new(child.props);
        this
    }
}

impl AnyPod {
    fn as_mut<'a>(
        &'a mut self,
        parent: impl Into<Option<&'a web_sys::Node>>,
        was_removed: bool,
    ) -> PodMut<'a, Box<dyn AnyNode>> {
        PodMut::new(&mut self.node, &mut self.props, parent.into(), was_removed)
    }
}

/// The mutable representation of [`Pod`].
///
/// This is a container which contains info of what has changed and provides mutable access to the underlying element and its props
/// When it's dropped all changes are applied to the underlying DOM node
pub struct PodMut<'a, N: DomNode> {
    node: &'a mut N,
    props: &'a mut N::Props,
    parent: Option<&'a web_sys::Node>,
    was_removed: bool,
}

impl<'a, N: DomNode> PodMut<'a, N> {
    fn new(
        node: &'a mut N,
        props: &'a mut N::Props,
        parent: Option<&'a web_sys::Node>,
        was_removed: bool,
    ) -> PodMut<'a, N> {
        PodMut {
            node,
            props,
            parent,
            was_removed,
        }
    }
}

impl PodMut<'_, Box<dyn AnyNode>> {
    fn downcast<N: DomNode>(&mut self) -> PodMut<'_, N> {
        PodMut::new(
            self.node.deref_mut().as_any_mut().downcast_mut().unwrap(),
            self.props.downcast_mut().unwrap(),
            self.parent,
            false,
        )
    }
}

impl<N: DomNode> Drop for PodMut<'_, N> {
    fn drop(&mut self) {
        if !self.was_removed {
            self.node.apply_props(self.props);
        }
    }
}

impl<T, N: AsRef<T> + DomNode> AsRef<T> for Pod<N> {
    fn as_ref(&self) -> &T {
        <N as AsRef<T>>::as_ref(&self.node)
    }
}

impl<T, N: AsRef<T> + DomNode> AsRef<T> for PodMut<'_, N> {
    fn as_ref(&self) -> &T {
        <N as AsRef<T>>::as_ref(self.node)
    }
}

impl DomNode for web_sys::Element {
    type Props = ElementProps;

    fn apply_props(&self, props: &mut ElementProps) {
        props.update_element(self);
    }
}

impl DomNode for web_sys::Text {
    type Props = ();

    fn apply_props(&self, (): &mut ()) {}
}

// TODO specialize some of these elements, maybe via features?
macro_rules! impl_dom_node_for_elements {
    ($($ty:ident, )*) => {$(
        impl DomNode for web_sys::$ty {
            type Props = ElementProps;

            fn apply_props(&self, props: &mut ElementProps) {
                props.update_element(self);
            }
        }

        impl From<Pod<web_sys::Element>> for Pod<web_sys::$ty> {
            fn from(value: Pod<web_sys::Element>) -> Self {
                Self {
                    node: value.node.unchecked_into(),
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
