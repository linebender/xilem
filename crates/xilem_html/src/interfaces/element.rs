use std::borrow::Cow;

use wasm_bindgen::JsCast;

use crate::{Attr, AttributeValue, IntoAttributeValue, OnEvent, OptionalAction};

// TODO should the options be its own function `on_event_with_options`,
// or should that be done via the builder pattern: `el.on_event().passive(false)`?
macro_rules! event_handler_mixin {
    ($(($fn_name:ident, $event:expr, $web_sys_event_type:ident),)*) => {
    $(
        fn $fn_name<EH, OA>(
            self,
            handler: EH,
        ) -> OnEvent<Self, web_sys::$web_sys_event_type, EH>
        where
            OA: OptionalAction<A>,
            EH: Fn(&mut T, web_sys::$web_sys_event_type) -> OA,
        {
            OnEvent::new(self, $event, handler)
        }
    )*
    };
}

use super::Node;
// TODO should Node or even EventTarget have the super trait View instead?
pub trait Element<T, A = ()>: Node<T, A>
where
    Self: Sized,
{
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
    ) -> Attr<Self> {
        Attr {
            element: self,
            name: name.into(),
            value: value.into_attribute_value(),
        }
    }

    // TODO should some methods extend some properties automatically,
    // instead of overwriting the (possibly set) inner value
    // or should there be (extra) "modifier" methods like `add_class` and/or `remove_class`
    fn class(self, class: impl Into<Cow<'static, str>>) -> Attr<Self> {
        self.attr("class", AttributeValue::String(class.into()))
    }

    // event list from
    // https://html.spec.whatwg.org/multipage/webappapis.html#idl-definitions
    //
    // I didn't include the events on the window, since we aren't attaching
    // any events to the window in xilem_html
    event_handler_mixin!(
        (on_abort, "abort", Event),
        (on_auxclick, "auxclick", PointerEvent),
        (on_beforeinput, "beforeinput", InputEvent),
        (on_beforematch, "beforematch", Event),
        (on_beforetoggle, "beforetoggle", Event),
        (on_blur, "blur", FocusEvent),
        (on_cancel, "cancel", Event),
        (on_canplay, "canplay", Event),
        (on_canplaythrough, "canplaythrough", Event),
        (on_change, "change", Event),
        (on_click, "click", MouseEvent),
        (on_close, "close", Event),
        (on_contextlost, "contextlost", Event),
        (on_contextmenu, "contextmenu", PointerEvent),
        (on_contextrestored, "contextrestored", Event),
        (on_copy, "copy", Event),
        (on_cuechange, "cuechange", Event),
        (on_cut, "cut", Event),
        (on_dblclick, "dblclick", MouseEvent),
        (on_drag, "drag", Event),
        (on_dragend, "dragend", Event),
        (on_dragenter, "dragenter", Event),
        (on_dragleave, "dragleave", Event),
        (on_dragover, "dragover", Event),
        (on_dragstart, "dragstart", Event),
        (on_drop, "drop", Event),
        (on_durationchange, "durationchange", Event),
        (on_emptied, "emptied", Event),
        (on_ended, "ended", Event),
        (on_error, "error", Event),
        (on_focus, "focus", FocusEvent),
        (on_focusin, "focusin", FocusEvent),
        (on_focusout, "focusout", FocusEvent),
        (on_formdata, "formdata", Event),
        (on_input, "input", InputEvent),
        (on_invalid, "invalid", Event),
        (on_keydown, "keydown", KeyboardEvent),
        (on_keyup, "keyup", KeyboardEvent),
        (on_load, "load", Event),
        (on_loadeddata, "loadeddata", Event),
        (on_loadedmetadata, "loadedmetadata", Event),
        (on_loadstart, "loadstart", Event),
        (on_mousedown, "mousedown", MouseEvent),
        (on_mouseenter, "mouseenter", MouseEvent),
        (on_mouseleave, "mouseleave", MouseEvent),
        (on_mousemove, "mousemove", MouseEvent),
        (on_mouseout, "mouseout", MouseEvent),
        (on_mouseover, "mouseover", MouseEvent),
        (on_mouseup, "mouseup", MouseEvent),
        (on_paste, "paste", Event),
        (on_pause, "pause", Event),
        (on_play, "play", Event),
        (on_playing, "playing", Event),
        (on_progress, "progress", Event),
        (on_ratechange, "ratechange", Event),
        (on_reset, "reset", Event),
        (on_resize, "resize", Event),
        (on_scroll, "scroll", Event),
        (on_scrollend, "scrollend", Event),
        (on_securitypolicyviolation, "securitypolicyviolation", Event),
        (on_seeked, "seeked", Event),
        (on_seeking, "seeking", Event),
        (on_select, "select", Event),
        (on_slotchange, "slotchange", Event),
        (on_stalled, "stalled", Event),
        (on_submit, "submit", Event),
        (on_suspend, "suspend", Event),
        (on_timeupdate, "timeupdate", Event),
        (on_toggle, "toggle", Event),
        (on_volumechange, "volumechange", Event),
        (on_waiting, "waiting", Event),
        (on_wheel, "wheel", WheelEvent),
    );
}

impl<T, A, E: Element<T, A>> Element<T, A> for Attr<E> {}

impl<T, A, E, Ev, F, OA> Element<T, A> for OnEvent<E, Ev, F>
where
    F: Fn(&mut T, Ev) -> OA,
    E: Element<T, A>,
    Ev: JsCast + 'static,
    OA: OptionalAction<A>,
{
}
