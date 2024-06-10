// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use attribute::{Attr, WithAttributes};
use class::{AsClassIter, Class, WithClasses};
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
mod one_of;
mod optional_action;
mod text;
mod vec_splice;
mod vecmap;

pub use app::{App, ViewCtx};
pub use attribute_value::{AttributeValue, IntoAttributeValue};
pub use optional_action::{Action, OptionalAction};
pub use text::text;

type CowStr = std::borrow::Cow<'static, str>;

pub trait DomNode: AnyNode + 'static {
    type Props: 'static;

    fn update_node(&self, props: &mut Self::Props);
    // TODO maybe default impl?
    fn into_dyn_node(self, props: Self::Props) -> Pod<DynNode>;
}

// struct Cla;
// trait WithProps<E> {
//     fn update_props(&mut self, props: &mut E);
// }

// trait Classes {}
// impl<E: WithProps< Classes for Props<Cla> {}
// impl<E: Classes> Classes for Props<E> {}

// trait Props {
//     fn update<E>(&mut self, element: E, );
// }

// impl Classes

// pub trait DomNodeNew: AnyNode + 'static {
//     type Props<E: Props>: 'static;

//     fn update_node(&self, props: &mut Self::Props<E>);
//     // TODO maybe default impl?
//     fn into_dyn_node(self, props: Self::Props) -> Pod<DynNode>;
// }

// pub trait DomNodeNew: AsRef<Self::Node> + 'static {
//     type Node: AsRef<web_sys::Node>;
// }

// impl DomNodeNew for Pod<web_sys::Element> {
//     type Node = web_sys::Element;

// }

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

pub struct Pod<E: DomNode> {
    pub node: E,
    pub props: E::Props,
}

impl<E: DomNode> Pod<E> {
    pub fn into_dyn_node(node: E, props: E::Props) -> Pod<DynNode> {
        Pod {
            node: DynNode {
                inner: Box::new(node),
            },
            props: Box::new(props),
        }
    }
}

impl<E: DomNode> ViewElement for Pod<E> {
    type Mut<'a> = PodMut<'a, E>;
}

impl<E: DomNode> SuperElement<Pod<E>> for Pod<DynNode> {
    fn upcast(child: Pod<E>) -> Self {
        child.node.into_dyn_node(child.props)
    }

    fn with_downcast_val<R>(
        mut this: Self::Mut<'_>,
        f: impl FnOnce(PodMut<'_, E>) -> R,
    ) -> (Self::Mut<'_>, R) {
        let downcast = this.downcast();
        let ret = f(downcast);
        (this, ret)
    }
}

impl<E: DomNode> AnyElement<Pod<E>> for Pod<DynNode> {
    fn replace_inner(mut this: Self::Mut<'_>, child: Pod<E>) -> Self::Mut<'_> {
        Pod::<DynNode>::replace_inner(&mut this, child);
        this
    }
}

impl Pod<DynNode> {
    pub(crate) fn replace_inner<E: DomNode>(this: &mut PodMut<'_, DynNode>, node: Pod<E>) {
        this.node.inner = Box::new(node.node);
        *this.props = Box::new(node.props);
    }

    fn as_mut(&mut self, was_removed: bool) -> PodMut<'_, DynNode> {
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

impl DomNode for DynNode {
    type Props = Box<dyn Any>;

    fn update_node(&self, _props: &mut Self::Props) {
        // TODO this is probably not optimal, as misleading, this is only implemented for concrete (non-type-erased) elements
        // I do *think* it's necessary as method on the trait because of the Drop impl (and not having specialization there)
    }

    fn into_dyn_node(self, props: Self::Props) -> Pod<DynNode> {
        // Double wrapping necessary because otherwise downcast can fail
        Pod::into_dyn_node(self, props)
    }
}

pub struct PodMut<'a, E: DomNode> {
    // TODO no pub!
    pub node: &'a mut E,
    pub props: &'a mut E::Props,
    pub was_removed: bool,
}

impl PodMut<'_, DynNode> {
    fn downcast<E: DomNode>(&mut self) -> PodMut<'_, E> {
        PodMut {
            node: self.node.inner.as_any_mut().downcast_mut().unwrap(),
            props: self.props.downcast_mut().unwrap(),
            was_removed: false,
        }
    }
}

impl<E: DomNode> Drop for PodMut<'_, E> {
    fn drop(&mut self) {
        self.node.update_node(self.props);
    }
}

impl<T, E: AsRef<T> + DomNode> AsRef<T> for Pod<E> {
    fn as_ref(&self) -> &T {
        <E as AsRef<T>>::as_ref(&self.node)
    }
}

impl<T, E: AsRef<T> + DomNode> AsRef<T> for PodMut<'_, E> {
    fn as_ref(&self) -> &T {
        <E as AsRef<T>>::as_ref(self.node)
    }
}



impl DomNode for web_sys::Element {
    type Props = ElementProps;

    fn update_node(&self, props: &mut Self::Props) {
        props.update_element(self);
    }

    fn into_dyn_node(self, mut props: Self::Props) -> Pod<DynNode> {
        props.update_element(&self);
        Pod::into_dyn_node(self, props)
    }
}

impl DomNode for web_sys::Text {
    type Props = ();

    fn update_node(&self, _props: &mut Self::Props) {}

    fn into_dyn_node(self, props: Self::Props) -> Pod<DynNode> {
        Pod::into_dyn_node(self, props)
    }
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
        impl DomNode for web_sys::$ty {
            type Props = ElementProps;
            fn update_node(&self, props: &mut Self::Props) {
                props.update_element(self);
            }
            fn into_dyn_node(self, mut props: Self::Props) -> Pod<DynNode> {
                props.update_element(&self);
                Pod::into_dyn_node(self, props)
            }
        }

        // TODO make this more flexible... Right now a HtmlDivElement can't be a HtmlElement for example
        pub trait $ty<State, Action = ()>: DomView<State, Action, DomNode = web_sys::$ty> {}
        impl<State, Action, T: DomView<State, Action, DomNode = web_sys::$ty>> $ty<State, Action> for T {}

        impl From<Pod<web_sys::Element>> for Pod<web_sys::$ty> {
            fn from(value: Pod<web_sys::Element>) -> Self {
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

pub trait ElementAsRef<E>: for<'a> ViewElement<Mut<'a>: AsRef<E>> + AsRef<E> {}

impl<E, T> ElementAsRef<E> for T
where
    T: ViewElement + AsRef<E>,
    for<'a> T::Mut<'a>: AsRef<E>,
{
}

macro_rules! event_handler_mixin {
    ($(($event_ty: ident, $fn_name:ident, $event:expr, $web_sys_event_type:ident),)*) => {
    $(
        fn $fn_name<Callback, OA>(
            self,
            handler: Callback,
        ) -> events::$event_ty<Self, State, Action, Callback>
        where
            Self: Sized,
            Self::Element: AsRef<web_sys::Element>,
            OA: OptionalAction<Action>,
            Callback: Fn(&mut State, web_sys::$web_sys_event_type) -> OA,
        {
            events::$event_ty::new(self, handler)
        }
    )*
    };
}

// TODO, not working yet
pub type AnyDomView<State, Action = ()> = dyn AnyView<State, Action, ViewCtx, Pod<DynNode>>;

impl<V, State, Action, W> DomView<State, Action> for V
where
    V: View<State, Action, ViewCtx, Element = Pod<W>>,
    W: DomNode,
{
    type DomNode = W;
}

pub trait DomView<State, Action = ()>:
    View<State, Action, ViewCtx, Element = Pod<Self::DomNode>>
{
    type DomNode: DomNode;

    fn attr(
        self,
        name: impl Into<CowStr>,
        value: impl IntoAttributeValue,
    ) -> Attr<Self, State, Action>
    where
        Self: Sized,
        Self::Element: WithAttributes,
        // The following bound would be more correct, but the current trait solver is not capable enough
        // (the new trait solver is able to do handle this though...)
        // but the bound above is enough for the API for now
        // Self::Element: ElementWithAttributes,
    {
        Attr::new(self, name.into(), value.into_attr_value())
    }

    fn class<AsClasses: AsClassIter>(
        self,
        as_classes: AsClasses,
    ) -> Class<Self, AsClasses, State, Action>
    where
        Self: Sized,
        Self::Element: WithClasses,
        // The following bound would be more correct, but the current trait solver is not capable enough
        // (the new trait solver is able to do handle this though...)
        // but the bound above is enough for the API for now
        // Self::Element: ElementWithClasses,
    {
        Class::new(self, as_classes)
    }

    fn on<Event, Callback, OA>(
        self,
        event: impl Into<CowStr>,
        handler: Callback,
    ) -> events::OnEvent<Self, State, Action, Event, Callback>
    where
        Self::Element: AsRef<web_sys::Element>,
        Event: JsCast + 'static,
        OA: OptionalAction<Action>,
        Callback: Fn(&mut State, Event) -> OA,
        Self: Sized,
    {
        events::OnEvent::new(self, event, handler)
    }

    // event list from
    // https://html.spec.whatwg.org/multipage/webappapis.html#idl-definitions
    //
    // I didn't include the events on the window, since we aren't attaching
    // any events to the window in xilem_web
    event_handler_mixin!(
        (OnAbort, on_abort, "abort", Event),
        (OnAuxClick, on_auxclick, "auxclick", PointerEvent),
        (OnBeforeInput, on_beforeinput, "beforeinput", InputEvent),
        (OnBeforeMatch, on_beforematch, "beforematch", Event),
        (OnBeforeToggle, on_beforetoggle, "beforetoggle", Event),
        (OnBlur, on_blur, "blur", FocusEvent),
        (OnCancel, on_cancel, "cancel", Event),
        (OnCanPlay, on_canplay, "canplay", Event),
        (OnCanPlayThrough, on_canplaythrough, "canplaythrough", Event),
        (OnChange, on_change, "change", Event),
        (OnClick, on_click, "click", MouseEvent),
        (OnClose, on_close, "close", Event),
        (OnContextLost, on_contextlost, "contextlost", Event),
        (OnContextMenu, on_contextmenu, "contextmenu", PointerEvent),
        (
            OnContextRestored,
            on_contextrestored,
            "contextrestored",
            Event
        ),
        (OnCopy, on_copy, "copy", Event),
        (OnCueChange, on_cuechange, "cuechange", Event),
        (OnCut, on_cut, "cut", Event),
        (OnDblClick, on_dblclick, "dblclick", MouseEvent),
        (OnDrag, on_drag, "drag", Event),
        (OnDragEnd, on_dragend, "dragend", Event),
        (OnDragEnter, on_dragenter, "dragenter", Event),
        (OnDragLeave, on_dragleave, "dragleave", Event),
        (OnDragOver, on_dragover, "dragover", Event),
        (OnDragStart, on_dragstart, "dragstart", Event),
        (OnDrop, on_drop, "drop", Event),
        (OnDurationChange, on_durationchange, "durationchange", Event),
        (OnEmptied, on_emptied, "emptied", Event),
        (OnEnded, on_ended, "ended", Event),
        (OnError, on_error, "error", Event),
        (OnFocus, on_focus, "focus", FocusEvent),
        (OnFocusIn, on_focusin, "focusin", FocusEvent),
        (OnFocusOut, on_focusout, "focusout", FocusEvent),
        (OnFormData, on_formdata, "formdata", Event),
        (OnInput, on_input, "input", Event),
        (OnInvalid, on_invalid, "invalid", Event),
        (OnKeyDown, on_keydown, "keydown", KeyboardEvent),
        (OnKeyUp, on_keyup, "keyup", KeyboardEvent),
        (OnLoad, on_load, "load", Event),
        (OnLoadedData, on_loadeddata, "loadeddata", Event),
        (OnLoadedMetadata, on_loadedmetadata, "loadedmetadata", Event),
        (OnLoadStart, on_loadstart, "loadstart", Event),
        (OnMouseDown, on_mousedown, "mousedown", MouseEvent),
        (OnMouseEnter, on_mouseenter, "mouseenter", MouseEvent),
        (OnMouseLeave, on_mouseleave, "mouseleave", MouseEvent),
        (OnMouseMove, on_mousemove, "mousemove", MouseEvent),
        (OnMouseOut, on_mouseout, "mouseout", MouseEvent),
        (OnMouseOver, on_mouseover, "mouseover", MouseEvent),
        (OnMouseUp, on_mouseup, "mouseup", MouseEvent),
        (OnPaste, on_paste, "paste", Event),
        (OnPause, on_pause, "pause", Event),
        (OnPlay, on_play, "play", Event),
        (OnPlaying, on_playing, "playing", Event),
        (OnProgress, on_progress, "progress", Event),
        (OnRateChange, on_ratechange, "ratechange", Event),
        (OnReset, on_reset, "reset", Event),
        (OnResize, on_resize, "resize", Event),
        (OnScroll, on_scroll, "scroll", Event),
        (OnScrollEnd, on_scrollend, "scrollend", Event),
        (
            OnSecurityPolicyViolation,
            on_securitypolicyviolation,
            "securitypolicyviolation",
            Event
        ),
        (OnSeeked, on_seeked, "seeked", Event),
        (OnSeeking, on_seeking, "seeking", Event),
        (OnSelect, on_select, "select", Event),
        (OnSlotChange, on_slotchange, "slotchange", Event),
        (OnStalled, on_stalled, "stalled", Event),
        (OnSubmit, on_submit, "submit", Event),
        (OnSuspend, on_suspend, "suspend", Event),
        (OnTimeUpdate, on_timeupdate, "timeupdate", Event),
        (OnToggle, on_toggle, "toggle", Event),
        (OnVolumeChange, on_volumechange, "volumechange", Event),
        (OnWaiting, on_waiting, "waiting", Event),
        (OnWheel, on_wheel, "wheel", WheelEvent),
    );
}
