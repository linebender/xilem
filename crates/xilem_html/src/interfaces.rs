use crate::{View, ViewMarker};
use std::borrow::Cow;

use gloo::events::EventListenerOptions;
use wasm_bindgen::JsCast;

use crate::{
    events::{self, OnEvent},
    Attr, IntoAttributeValue, OptionalAction,
};

pub(crate) mod sealed {
    pub trait Sealed {}
}

// TODO should the options be its own function `on_event_with_options`,
// or should that be done via the builder pattern: `el.on_event().passive(false)`?
macro_rules! event_handler_mixin {
    ($(($event_ty: ident, $fn_name:ident, $event:expr, $web_sys_event_type:ident),)*) => {
    $(
        fn $fn_name<EH, OA>(self, handler: EH) -> events::$event_ty<T, A, Self, EH>
        where
            OA: OptionalAction<A>,
            EH: Fn(&mut T, web_sys::$web_sys_event_type) -> OA,
        {
            $crate::events::$event_ty::new(self, handler)
        }
    )*
    };
}

pub trait Element<T, A = ()>: View<T, A> + ViewMarker + sealed::Sealed
where
    Self: Sized,
{
    fn on<E, EH, OA>(self, event: impl Into<Cow<'static, str>>, handler: EH) -> OnEvent<Self, E, EH>
    where
        E: JsCast + 'static,
        OA: OptionalAction<A>,
        EH: Fn(&mut T, E) -> OA,
        Self: Sized,
    {
        OnEvent::new(self, event, handler)
    }

    fn on_with_options<E, EH, OA>(
        self,
        event: impl Into<Cow<'static, str>>,
        handler: EH,
        options: EventListenerOptions,
    ) -> OnEvent<Self, E, EH>
    where
        E: JsCast + 'static,
        OA: OptionalAction<A>,
        EH: Fn(&mut T, E) -> OA,
        Self: Sized,
    {
        OnEvent::new_with_options(self, event, handler, options)
    }

    // TODO should the API be "functional" in the sense, that new attributes are wrappers around the type,
    // or should they modify the underlying instance (e.g. via the following methods)?
    // The disadvantage that "functional" brings in, is that elements are not modifiable (i.e. attributes can't be simply added etc.)
    // fn attrs(&self) -> &Attributes;
    // fn attrs_mut(&mut self) -> &mut Attributes;

    /// Set an attribute on this element.
    ///
    /// # Panics
    ///
    /// If the name contains characters that are not valid in an attribute name,
    /// then the `View::build`/`View::rebuild` functions will panic for this view.
    fn attr(
        self,
        name: impl Into<Cow<'static, str>>,
        value: impl IntoAttributeValue,
    ) -> Attr<T, A, Self> {
        Attr {
            element: self,
            name: name.into(),
            value: value.into_attribute_value(),
            phantom: std::marker::PhantomData,
        }
    }

    // TODO should some methods extend some properties automatically,
    // instead of overwriting the (possibly set) inner value
    // or should there be (extra) "modifier" methods like `add_class` and/or `remove_class`
    fn class(self, class: impl Into<Cow<'static, str>>) -> Attr<T, A, Self> {
        self.attr("class", class.into())
    }

    // event list from
    // https://html.spec.whatwg.org/multipage/webappapis.html#idl-definitions
    //
    // I didn't include the events on the window, since we aren't attaching
    // any events to the window in xilem_html
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
        (OnInput, on_input, "input", InputEvent),
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

macro_rules! dom_interface_macro_and_trait_definitions {
    // $dollar workaround for not yet stabilized feature 'macro_metavar_expr'
    ($dollar:tt, $($dom_interface:ident : $super_dom_interface: ident $body: tt),*) => {
        $(
        pub trait $dom_interface<T, A = ()>: $super_dom_interface<T, A> $body
        )*

        /// Execute $mac which is a macro, that takes at least the $dom_interface:ident as parameter for all interface relatives.
        /// For example for_all_dom_interface_relatives(HtmlVideoElement, my_mac, ...) invocates my_mac!(HtmlVideoElement, ...), my_mac!(HtmlMediaElement, ...), my_mac!(HtmlElement, ...) and my_mac!(Element, ...)
        /// It optionally passes arguments given to for_all_dom_interface_relatives! to $mac!
        macro_rules! for_all_dom_interface_relatives {
            // base case, Element is the root interface for all kinds of DOM interfaces
            (Element, $mac:ident $dollar($body_:tt)*) => {
                $mac!(Element $dollar($body_)*);
            };
            $(($dom_interface, $mac:ident $dollar($body_:tt)*) => {
                $mac!($dom_interface $dollar($body_)*);
                $crate::interfaces::for_all_dom_interface_relatives!($super_dom_interface, $mac $dollar($body_)*);
             };)*
        }

        pub(crate) use for_all_dom_interface_relatives;

        /// Execute $mac which is a macro, that takes at least the $dom_interface:ident as parameter, for all dom interfaces.
        /// It optionally passes arguments given to for_all_dom_interfaces! to $mac!
        macro_rules! for_all_dom_interfaces {
            ($mac:ident $dollar($body_:tt)*) => {
                $mac!(Element $dollar($body_)*);
                $($mac!($dom_interface $dollar($body_)*);)*
            }
        }

        pub(crate) use for_all_dom_interfaces;
    };
}

dom_interface_macro_and_trait_definitions!($,
    HtmlElement : Element {},
    HtmlAnchorElement : HtmlElement {},
    HtmlAreaElement : HtmlElement {},
    HtmlAudioElement : HtmlMediaElement {},
    HtmlBaseElement : HtmlElement {},
    HtmlBodyElement : HtmlElement {},
    HtmlBrElement : HtmlElement {},
    HtmlButtonElement : HtmlElement {},
    HtmlCanvasElement : HtmlElement {
        fn width(self, value: u32) -> Attr<T, A, Self> {
            self.attr("width", value)
        }
        fn height(self, value: u32) -> Attr<T, A, Self> {
            self.attr("height", value)
        }
    },
    HtmlDataElement : HtmlElement {},
    HtmlDataListElement : HtmlElement {},
    HtmlDetailsElement : HtmlElement {},
    HtmlDialogElement : HtmlElement {},
    HtmlDirectoryElement : HtmlElement {},
    HtmlDivElement : HtmlElement {},
    HtmlDListElement : HtmlElement {},
    HtmlUnknownElement : HtmlElement {},
    HtmlEmbedElement : HtmlElement {},
    HtmlFieldSetElement : HtmlElement {},
    HtmlFontElement : HtmlElement {},
    HtmlFormElement : HtmlElement {},
    HtmlFrameElement : HtmlElement {},
    HtmlFrameSetElement : HtmlElement {},
    HtmlHeadElement : HtmlElement {},
    HtmlHeadingElement : HtmlElement {},
    HtmlHrElement : HtmlElement {},
    HtmlHtmlElement : HtmlElement {},
    HtmlIFrameElement : HtmlElement {},
    HtmlImageElement : HtmlElement {},
    HtmlInputElement : HtmlElement {},
    HtmlLabelElement : HtmlElement {},
    HtmlLegendElement : HtmlElement {},
    HtmlLiElement : HtmlElement {},
    HtmlLinkElement : HtmlElement {},
    HtmlMapElement : HtmlElement {},
    HtmlMediaElement : HtmlElement {},
    HtmlMenuElement : HtmlElement {},
    HtmlMenuItemElement : HtmlElement {},
    HtmlMetaElement : HtmlElement {},
    HtmlMeterElement : HtmlElement {},
    HtmlModElement : HtmlElement {},
    HtmlObjectElement : HtmlElement {},
    HtmlOListElement : HtmlElement {},
    HtmlOptGroupElement : HtmlElement {},
    HtmlOptionElement : HtmlElement {},
    HtmlOutputElement : HtmlElement {},
    HtmlParagraphElement : HtmlElement {},
    HtmlParamElement : HtmlElement {},
    HtmlPictureElement : HtmlElement {},
    HtmlPreElement : HtmlElement {},
    HtmlProgressElement : HtmlElement {},
    HtmlQuoteElement : HtmlElement {},
    HtmlScriptElement : HtmlElement {},
    HtmlSelectElement : HtmlElement {},
    HtmlSlotElement : HtmlElement {},
    HtmlSourceElement : HtmlElement {},
    HtmlSpanElement : HtmlElement {},
    HtmlStyleElement : HtmlElement {},
    HtmlTableCaptionElement : HtmlElement {},
    HtmlTableCellElement : HtmlElement {},
    HtmlTableColElement : HtmlElement {},
    HtmlTableElement : HtmlElement {},
    HtmlTableRowElement : HtmlElement {},
    HtmlTableSectionElement : HtmlElement {},
    HtmlTemplateElement : HtmlElement {},
    HtmlTimeElement : HtmlElement {},
    HtmlTextAreaElement : HtmlElement {},
    HtmlTitleElement : HtmlElement {},
    HtmlTrackElement : HtmlElement {},
    HtmlUListElement : HtmlElement {},
    HtmlVideoElement : HtmlMediaElement {
        fn width(self, value: u32) -> Attr<T, A, Self> {
            self.attr("width", value)
        }
        fn height(self, value: u32) -> Attr<T, A, Self> {
            self.attr("height", value)
        }
    }
);
