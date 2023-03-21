//! Macros to generate all the different html events
//!
macro_rules! events {
    () => {};
    (($ty_name:ident, $builder_name:ident, $name:literal, $web_sys_ty:ty), $($rest:tt)*) => {
        event!($ty_name, $builder_name, $name, $web_sys_ty);
        events!($($rest)*);
    };
}

macro_rules! event {
    ($ty_name:ident, $builder_name:ident, $name:literal, $web_sys_ty:ty) => {
        pub struct $ty_name<V, F>(crate::OnEvent<$web_sys_ty, V, F>);

        pub fn $builder_name<V, F>(child: V, callback: F) -> $ty_name<V, F> {
            $ty_name(crate::on_event($name, child, callback))
        }

        impl<V, F> crate::view::ViewMarker for $ty_name<V, F> {}

        impl<T, A, V, F> crate::view::View<T, A> for $ty_name<V, F>
        where
            V: crate::view::View<T, A>,
            F: Fn(&mut T, &$crate::Event<$web_sys_ty, V::Element>) -> $crate::MessageResult<A>,
            V::Element: 'static,
        {
            type State = crate::event::OnEventState<V::State>;
            type Element = V::Element;

            fn build(
                &self,
                cx: &mut crate::context::Cx,
            ) -> (xilem_core::Id, Self::State, Self::Element) {
                self.0.build(cx)
            }

            fn rebuild(
                &self,
                cx: &mut crate::context::Cx,
                prev: &Self,
                id: &mut xilem_core::Id,
                state: &mut Self::State,
                element: &mut Self::Element,
            ) -> crate::ChangeFlags {
                self.0.rebuild(cx, &prev.0, id, state, element)
            }

            fn message(
                &self,
                id_path: &[xilem_core::Id],
                state: &mut Self::State,
                message: Box<dyn std::any::Any>,
                app_state: &mut T,
            ) -> xilem_core::MessageResult<A> {
                self.0.message(id_path, state, message, app_state)
            }
        }
    };
}

// event list from
// https://html.spec.whatwg.org/multipage/webappapis.html#idl-definitions
//
// I didn't include the events on the window, since we aren't attaching
// any events to the window in xilem_html

events!(
    (OnAbort, on_abort, "abort", web_sys::Event),
    (OnAuxClick, on_auxclick, "auxclick", web_sys::PointerEvent),
    (
        OnBeforeInput,
        on_beforeinput,
        "beforeinput",
        web_sys::InputEvent
    ),
    (OnBeforeMatch, on_beforematch, "beforematch", web_sys::Event),
    (
        OnBeforeToggle,
        on_beforetoggle,
        "beforetoggle",
        web_sys::Event
    ),
    (OnBlur, on_blur, "blur", web_sys::FocusEvent),
    (OnCancel, on_cancel, "cancel", web_sys::Event),
    (OnCanPlay, on_canplay, "canplay", web_sys::Event),
    (
        OnCanPlayThrough,
        on_canplaythrough,
        "canplaythrough",
        web_sys::Event
    ),
    (OnChange, on_change, "change", web_sys::Event),
    (OnClick, on_click, "click", web_sys::MouseEvent),
    (OnClose, on_close, "close", web_sys::Event),
    (OnContextLost, on_contextlost, "contextlost", web_sys::Event),
    (
        OnContextMenu,
        on_contextmenu,
        "contextmenu",
        web_sys::PointerEvent
    ),
    (
        OnContextRestored,
        on_contextrestored,
        "contextrestored",
        web_sys::Event
    ),
    (OnCopy, on_copy, "copy", web_sys::Event),
    (OnCueChange, on_cuechange, "cuechange", web_sys::Event),
    (OnCut, on_cut, "cut", web_sys::Event),
    (OnDblClick, on_dblclick, "dblclick", web_sys::MouseEvent),
    (OnDrag, on_drag, "drag", web_sys::Event),
    (OnDragEnd, on_dragend, "dragend", web_sys::Event),
    (OnDragEnter, on_dragenter, "dragenter", web_sys::Event),
    (OnDragLeave, on_dragleave, "dragleave", web_sys::Event),
    (OnDragOver, on_dragover, "dragover", web_sys::Event),
    (OnDragStart, on_dragstart, "dragstart", web_sys::Event),
    (OnDrop, on_drop, "drop", web_sys::Event),
    (
        OnDurationChange,
        on_durationchange,
        "durationchange",
        web_sys::Event
    ),
    (OnEmptied, on_emptied, "emptied", web_sys::Event),
    (OnEnded, on_ended, "ended", web_sys::Event),
    (OnError, on_error, "error", web_sys::Event),
    (OnFocus, on_focus, "focus", web_sys::FocusEvent),
    (OnFocusIn, on_focusin, "focusin", web_sys::FocusEvent),
    (OnFocusOut, on_focusout, "focusout", web_sys::FocusEvent),
    (OnFormData, on_formdata, "formdata", web_sys::Event),
    (OnInput, on_input, "input", web_sys::InputEvent),
    (OnInvalid, on_invalid, "invalid", web_sys::Event),
    (OnKeyDown, on_keydown, "keydown", web_sys::KeyboardEvent),
    (OnKeyUp, on_keyup, "keyup", web_sys::KeyboardEvent),
    (OnLoad, on_load, "load", web_sys::Event),
    (OnLoadedData, on_loadeddata, "loadeddata", web_sys::Event),
    (
        OnLoadedMetadata,
        on_loadedmetadata,
        "loadedmetadata",
        web_sys::Event
    ),
    (OnLoadStart, on_loadstart, "loadstart", web_sys::Event),
    (OnMouseDown, on_mousedown, "mousedown", web_sys::MouseEvent),
    (
        OnMouseEnter,
        on_mouseenter,
        "mouseenter",
        web_sys::MouseEvent
    ),
    (
        OnMouseLeave,
        on_mouseleave,
        "mouseleave",
        web_sys::MouseEvent
    ),
    (OnMouseMove, on_mousemove, "mousemove", web_sys::MouseEvent),
    (OnMouseOut, on_mouseout, "mouseout", web_sys::MouseEvent),
    (OnMouseOver, on_mouseover, "mouseover", web_sys::MouseEvent),
    (OnMouseUp, on_mouseup, "mouseup", web_sys::MouseEvent),
    (OnPaste, on_paste, "paste", web_sys::Event),
    (OnPause, on_pause, "pause", web_sys::Event),
    (OnPlay, on_play, "play", web_sys::Event),
    (OnPlaying, on_playing, "playing", web_sys::Event),
    (OnProgress, on_progress, "progress", web_sys::Event),
    (OnRateChange, on_ratechange, "ratechange", web_sys::Event),
    (OnReset, on_reset, "reset", web_sys::Event),
    (OnResize, on_resize, "resize", web_sys::Event),
    (OnScroll, on_scroll, "scroll", web_sys::Event),
    (OnScrollEnd, on_scrollend, "scrollend", web_sys::Event),
    (
        OnSecurityPolicyViolation,
        on_securitypolicyviolation,
        "securitypolicyviolation",
        web_sys::Event
    ),
    (OnSeeked, on_seeked, "seeked", web_sys::Event),
    (OnSeeking, on_seeking, "seeking", web_sys::Event),
    (OnSelect, on_select, "select", web_sys::Event),
    (OnSlotChange, on_slotchange, "slotchange", web_sys::Event),
    (OnStalled, on_stalled, "stalled", web_sys::Event),
    (OnSubmit, on_submit, "submit", web_sys::Event),
    (OnSuspend, on_suspend, "suspend", web_sys::Event),
    (OnTimeUpdate, on_timeupdate, "timeupdate", web_sys::Event),
    (OnToggle, on_toggle, "toggle", web_sys::Event),
    (
        OnVolumeChange,
        on_volumechange,
        "volumechange",
        web_sys::Event
    ),
    (OnWaiting, on_waiting, "waiting", web_sys::Event),
    (
        OnWebkitAnimationEnd,
        on_webkitanimationend,
        "webkitanimationend",
        web_sys::Event
    ),
    (
        OnWebkitAnimationIteration,
        on_webkitanimationiteration,
        "webkitanimationiteration",
        web_sys::Event
    ),
    (
        OnWebkitAnimationStart,
        on_webkitanimationstart,
        "webkitanimationstart",
        web_sys::Event
    ),
    (
        OnWebkitTransitionEnd,
        on_webkittransitionend,
        "webkittransitionend",
        web_sys::Event
    ),
    (OnWheel, on_wheel, "wheel", web_sys::WheelEvent),
);
