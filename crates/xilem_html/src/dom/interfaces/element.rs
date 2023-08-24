use std::borrow::Cow;

use wasm_bindgen::JsCast;

use crate::{
    dom::{attribute::Attr, event::EventListener},
    IntoAttributeValue, OptionalAction, View, ViewMarker,
};

// TODO should the options be its own function `on_event_with_options`,
// or should that be done via the builder pattern: `el.on_event().passive(false)`?
macro_rules! event_handler_mixin {
    ($(($fn_name:ident, $fn_name_options:ident, $event:expr, $web_sys_event_type:ident),)*) => {
    $(
        fn $fn_name<EH, OA>(
            self,
            handler: EH,
        ) -> EventListener<Self, web_sys::$web_sys_event_type, EH>
        where
            OA: OptionalAction<A>,
            EH: Fn(&mut T, web_sys::$web_sys_event_type) -> OA,
        {
            EventListener::new(self, $event, handler)
        }

        fn $fn_name_options<EH, OA>(
            self,
            handler: EH,
            options: gloo::events::EventListenerOptions,
        ) -> EventListener<Self, web_sys::$web_sys_event_type, EH>
        where
            OA: OptionalAction<A>,
            EH: Fn(&mut T, web_sys::$web_sys_event_type) -> OA,
        {
            EventListener::new_with_options(self, $event, handler, options)
        }
    )*
    };
}

use super::Node;
pub trait Element<T, A = ()>: Node + View<T, A> + ViewMarker
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
    fn attr<K, V>(self, name: K, value: V) -> Attr<Self>
    where
        K: Into<Cow<'static, str>>,
        V: IntoAttributeValue,
    {
        Attr {
            element: self,
            name: name.into(),
            value: value.into_attribute_value(),
        }
    }

    fn class<V>(self, class: V) -> Attr<Self>
    where
        V: IntoAttributeValue,
    {
        self.attr("class", class)
    }

    // event list from
    // https://html.spec.whatwg.org/multipage/webappapis.html#idl-definitions
    //
    // I didn't include the events on the window, since we aren't attaching
    // any events to the window in xilem_html
    event_handler_mixin!(
        (on_abort, on_abort_with_options, "abort", Event),
        (
            on_auxclick,
            on_auxclick_with_options,
            "auxclick",
            PointerEvent
        ),
        (
            on_beforeinput,
            on_beforeinput_with_options,
            "beforeinput",
            InputEvent
        ),
        (
            on_beforematch,
            on_beforematch_with_options,
            "beforematch",
            Event
        ),
        (
            on_beforetoggle,
            on_beforetoggle_with_options,
            "beforetoggle",
            Event
        ),
        (on_blur, on_blur_with_options, "blur", FocusEvent),
        (on_cancel, on_cancel_with_options, "cancel", Event),
        (on_canplay, on_canplay_with_options, "canplay", Event),
        (
            on_canplaythrough,
            on_canplaythrough_with_options,
            "canplaythrough",
            Event
        ),
        (on_change, on_change_with_options, "change", Event),
        (on_click, on_click_with_options, "click", MouseEvent),
        (on_close, on_close_with_options, "close", Event),
        (
            on_contextlost,
            on_contextlost_with_options,
            "contextlost",
            Event
        ),
        (
            on_contextmenu,
            on_contextmenu_with_options,
            "contextmenu",
            PointerEvent
        ),
        (
            on_contextrestored,
            on_contextrestored_with_options,
            "contextrestored",
            Event
        ),
        (on_copy, on_copy_with_options, "copy", Event),
        (on_cuechange, on_cuechange_with_options, "cuechange", Event),
        (on_cut, on_cut_with_options, "cut", Event),
        (
            on_dblclick,
            on_dblclick_with_options,
            "dblclick",
            MouseEvent
        ),
        (on_drag, on_drag_with_options, "drag", Event),
        (on_dragend, on_dragend_with_options, "dragend", Event),
        (on_dragenter, on_dragenter_with_options, "dragenter", Event),
        (on_dragleave, on_dragleave_with_options, "dragleave", Event),
        (on_dragover, on_dragover_with_options, "dragover", Event),
        (on_dragstart, on_dragstart_with_options, "dragstart", Event),
        (on_drop, on_drop_with_options, "drop", Event),
        (
            on_durationchange,
            on_durationchange_with_options,
            "durationchange",
            Event
        ),
        (on_emptied, on_emptied_with_options, "emptied", Event),
        (on_ended, on_ended_with_options, "ended", Event),
        (on_error, on_error_with_options, "error", Event),
        (on_focus, on_focus_with_options, "focus", FocusEvent),
        (on_focusin, on_focusin_with_options, "focusin", FocusEvent),
        (
            on_focusout,
            on_focusout_with_options,
            "focusout",
            FocusEvent
        ),
        (on_formdata, on_formdata_with_options, "formdata", Event),
        (on_input, on_input_with_options, "input", InputEvent),
        (on_invalid, on_invalid_with_options, "invalid", Event),
        (
            on_keydown,
            on_keydown_with_options,
            "keydown",
            KeyboardEvent
        ),
        (on_keyup, on_keyup_with_options, "keyup", KeyboardEvent),
        (on_load, on_load_with_options, "load", Event),
        (
            on_loadeddata,
            on_loadeddata_with_options,
            "loadeddata",
            Event
        ),
        (
            on_loadedmetadata,
            on_loadedmetadata_with_options,
            "loadedmetadata",
            Event
        ),
        (on_loadstart, on_loadstart_with_options, "loadstart", Event),
        (
            on_mousedown,
            on_mousedown_with_options,
            "mousedown",
            MouseEvent
        ),
        (
            on_mouseenter,
            on_mouseenter_with_options,
            "mouseenter",
            MouseEvent
        ),
        (
            on_mouseleave,
            on_mouseleave_with_options,
            "mouseleave",
            MouseEvent
        ),
        (
            on_mousemove,
            on_mousemove_with_options,
            "mousemove",
            MouseEvent
        ),
        (
            on_mouseout,
            on_mouseout_with_options,
            "mouseout",
            MouseEvent
        ),
        (
            on_mouseover,
            on_mouseover_with_options,
            "mouseover",
            MouseEvent
        ),
        (on_mouseup, on_mouseup_with_options, "mouseup", MouseEvent),
        (on_paste, on_paste_with_options, "paste", Event),
        (on_pause, on_pause_with_options, "pause", Event),
        (on_play, on_play_with_options, "play", Event),
        (on_playing, on_playing_with_options, "playing", Event),
        (on_progress, on_progress_with_options, "progress", Event),
        (
            on_ratechange,
            on_ratechange_with_options,
            "ratechange",
            Event
        ),
        (on_reset, on_reset_with_options, "reset", Event),
        (on_resize, on_resize_with_options, "resize", Event),
        (on_scroll, on_scroll_with_options, "scroll", Event),
        (on_scrollend, on_scrollend_with_options, "scrollend", Event),
        (
            on_securitypolicyviolation,
            on_securitypolicyviolation_with_options,
            "securitypolicyviolation",
            Event
        ),
        (on_seeked, on_seeked_with_options, "seeked", Event),
        (on_seeking, on_seeking_with_options, "seeking", Event),
        (on_select, on_select_with_options, "select", Event),
        (
            on_slotchange,
            on_slotchange_with_options,
            "slotchange",
            Event
        ),
        (on_stalled, on_stalled_with_options, "stalled", Event),
        (on_submit, on_submit_with_options, "submit", Event),
        (on_suspend, on_suspend_with_options, "suspend", Event),
        (
            on_timeupdate,
            on_timeupdate_with_options,
            "timeupdate",
            Event
        ),
        (on_toggle, on_toggle_with_options, "toggle", Event),
        (
            on_volumechange,
            on_volumechange_with_options,
            "volumechange",
            Event
        ),
        (on_waiting, on_waiting_with_options, "waiting", Event),
        (on_wheel, on_wheel_with_options, "wheel", WheelEvent),
    );
}

impl<T, A, E: Element<T, A>> Element<T, A> for Attr<E> {}

impl<T, A, E, Ev, F, OA> Element<T, A> for EventListener<E, Ev, F>
where
    F: Fn(&mut T, Ev) -> OA,
    E: Element<T, A>,
    Ev: JsCast + 'static,
    OA: OptionalAction<A>,
{
}
