// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

pub use attribute::{Attr, WithAttributes};
pub use class::{AsClassIter, Class, WithClasses};
use element::ElementProps;
use std::any::Any;
use wasm_bindgen::UnwrapThrowExt;
use web_sys::wasm_bindgen::JsCast;

pub use xilem_core::{
    memoize, AnyElement, AnyView, AppendVec, DynMessage, MessageResult, Mut, OneOf2, OneOf2Ctx,
    SuperElement, View, ViewElement, ViewId, ViewPathTracker, ViewSequence,
};

/// The HTML namespace
pub const HTML_NS: &str = "http://www.w3.org/1999/xhtml";
/// The SVG namespace
pub const SVG_NS: &str = "http://www.w3.org/2000/svg";
/// The MathML namespace
pub const MATHML_NS: &str = "http://www.w3.org/1998/Math/MathML";

mod app;
mod attribute;
mod attribute_value;
mod class;
pub mod element;
pub mod elements;
mod events;
pub mod interfaces;
mod one_of;
mod optional_action;
mod text;
mod vec_splice;
mod vecmap;

pub use app::{App, ViewCtx};
pub use attribute_value::{AttributeValue, IntoAttributeValue};
pub use optional_action::{Action, OptionalAction};

pub trait AnyNode: AsRef<web_sys::Node> + 'static {
    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn as_node_ref(&self) -> &web_sys::Node;
}

impl<N: AsRef<web_sys::Node> + Any> AnyNode for N {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn as_node_ref(&self) -> &web_sys::Node {
        self.as_ref()
    }
}

pub trait DomNode<P>: AnyNode + 'static {
    fn apply_props(&self, props: &mut P);
}

pub trait ElementAsRef<E>: for<'a> ViewElement<Mut<'a>: AsRef<E>> + AsRef<E> {}

impl<E, T> ElementAsRef<E> for T
where
    T: ViewElement + AsRef<E>,
    for<'a> T::Mut<'a>: AsRef<E>,
{
}

pub type AnyDomView<State, Action = ()> =
    dyn AnyView<State, Action, ViewCtx, Pod<DynNode, Box<dyn Any>>>;

impl<V, State, Action, W, P> DomView<State, Action> for V
where
    V: View<State, Action, ViewCtx, Element = Pod<W, P>>,
    W: DomNode<P>,
{
    type DomNode = W;
    type Props = P;
}

pub trait DomView<State, Action = ()>:
    View<State, Action, ViewCtx, Element = Pod<Self::DomNode, Self::Props>>
{
    type DomNode: DomNode<Self::Props>;
    type Props;
}

pub struct Pod<E, P> {
    pub node: E,
    pub props: P,
}

impl<E: DomNode<P>, P: 'static> Pod<E, P> {
    pub fn into_dyn_node(node: E, props: P) -> Pod<DynNode, Box<dyn Any>> {
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

impl<E: DomNode<P>, P: 'static> SuperElement<Pod<E, P>> for Pod<DynNode, Box<dyn Any>> {
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

impl<E: DomNode<P>, P: 'static> AnyElement<Pod<E, P>> for Pod<DynNode, Box<dyn Any>> {
    fn replace_inner(mut this: Self::Mut<'_>, child: Pod<E, P>) -> Self::Mut<'_> {
        Pod::replace_inner(&mut this, child);
        this
    }
}

impl Pod<DynNode, Box<dyn Any>> {
    pub(crate) fn replace_inner<E: DomNode<P>, P: 'static>(
        this: &mut PodMut<'_, DynNode, Box<dyn Any>>,
        node: Pod<E, P>,
    ) {
        this.node.inner = Box::new(node.node);
        *this.props = Box::new(node.props);
    }

    fn as_mut(&mut self, was_removed: bool) -> PodMut<'_, DynNode, Box<dyn Any>> {
        PodMut {
            node: &mut self.node,
            props: &mut self.props,
            was_removed,
        }
    }
}

pub struct DynNode {
    inner: Box<dyn AnyNode>,
}

impl AsRef<web_sys::Node> for DynNode {
    fn as_ref(&self) -> &web_sys::Node {
        self.inner.as_node_ref()
    }
}

impl DomNode<Box<dyn Any>> for DynNode {
    fn apply_props(&self, _props: &mut Box<dyn Any>) {
        // TODO this is probably not optimal, as misleading, this is only implemented for concrete (non-type-erased) elements
        // I do *think* it's necessary as method on the trait because of the Drop impl (and not having specialization there)
    }
}

pub struct PodMut<'a, E: DomNode<P>, P> {
    // TODO no pub!
    node: &'a mut E,
    props: &'a mut P,
    was_removed: bool,
}

impl<'a, E: DomNode<P>, P> PodMut<'a, E, P> {
    fn new(node: &'a mut E, props: &'a mut P, was_removed: bool) -> PodMut<'a, E, P> {
        PodMut {
            node,
            props,
            was_removed,
        }
    }
}

impl PodMut<'_, DynNode, Box<dyn Any>> {
    fn downcast<E: DomNode<P>, P: 'static>(&mut self) -> PodMut<'_, E, P> {
        PodMut {
            node: self.node.inner.as_any_mut().downcast_mut().unwrap(),
            props: self.props.downcast_mut().unwrap(),
            was_removed: false,
        }
    }
}

impl<E: DomNode<P>, P> Drop for PodMut<'_, E, P> {
    fn drop(&mut self) {
        self.node.apply_props(self.props);
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

pub fn get_element_by_id(id: &str) -> web_sys::HtmlElement {
    document()
        .get_element_by_id(id)
        .unwrap()
        .dyn_into()
        .unwrap()
}

// TODO specialize some of these elements, maybe via features?
// TODO currently all trait interfaces are directly bound to the
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
