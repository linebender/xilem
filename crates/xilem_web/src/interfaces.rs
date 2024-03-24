use crate::{class::Class, style::Style, Pointer, PointerMsg, View, ViewMarker};
use std::{borrow::Cow, marker::PhantomData};

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
        fn $fn_name<EH, OA>(self, handler: EH) -> events::$event_ty<Self, T, A, EH>
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
    fn on<E, EH, OA>(
        self,
        event: impl Into<Cow<'static, str>>,
        handler: EH,
    ) -> OnEvent<Self, T, A, E, EH>
    where
        E: JsCast + 'static,
        OA: OptionalAction<A>,
        EH: Fn(&mut T, E) -> OA,
        Self: Sized,
    {
        OnEvent::new(self, event, handler)
    }

    fn on_with_options<Ev, EH, OA>(
        self,
        event: impl Into<Cow<'static, str>>,
        handler: EH,
        options: EventListenerOptions,
    ) -> OnEvent<Self, T, A, Ev, EH>
    where
        Ev: JsCast + 'static,
        OA: OptionalAction<A>,
        EH: Fn(&mut T, Ev) -> OA,
        Self: Sized,
    {
        OnEvent::new_with_options(self, event, handler, options)
    }

    fn pointer<F: Fn(&mut T, PointerMsg)>(self, f: F) -> Pointer<Self, T, A, F> {
        crate::pointer::pointer(self, f)
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
    ) -> Attr<Self, T, A> {
        Attr {
            element: self,
            name: name.into(),
            value: value.into_attr_value(),
            phantom: std::marker::PhantomData,
        }
    }

    /// Add a class to the wrapped element.
    ///
    /// If multiple classes are added, all will be applied to the element.
    fn class(self, class: impl Into<Cow<'static, str>>) -> Class<Self, T, A> {
        self.class_opt(Some(class))
    }

    /// Add an optional class to the wrapped element.
    ///
    /// If multiple classes are added, all will be applied to the element.
    fn class_opt(self, class: Option<impl Into<Cow<'static, str>>>) -> Class<Self, T, A> {
        Class {
            element: self,
            class_name: class.map(Into::into),
            phantom: PhantomData,
        }
    }

    /// Set a style attribute
    fn style(
        self,
        name: impl Into<Cow<'static, str>>,
        value: impl Into<Cow<'static, str>>,
    ) -> Style<Self, T, A> {
        self.style_opt(name, Some(value))
    }

    /// Set a style attribute
    fn style_opt(
        self,
        name: impl Into<Cow<'static, str>>,
        value: Option<impl Into<Cow<'static, str>>>,
    ) -> Style<Self, T, A> {
        Style {
            element: self,
            name: name.into(),
            value: value.map(Into::into),
            phantom: PhantomData,
        }
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

// base case for ancestor macros, do nothing, because the body is in all the child interface macros...
#[allow(unused_macros)]
macro_rules! for_all_element_ancestors {
    ($($_:tt)*) => {};
}
#[allow(unused_imports)]
pub(crate) use for_all_element_ancestors;

macro_rules! dom_interface_macro_and_trait_definitions_impl {
    ($interface:ident {
        methods: $_methods_body:tt,
        child_interfaces: {
            $($child_interface:ident {
                methods: $child_methods_body:tt,
                child_interfaces: $child_interface_body: tt
            },)*
        }
    }) => {
        paste::paste! {
            $(
                pub trait $child_interface<T, A = ()>: $interface<T, A> $child_methods_body

                /// Execute $mac which is a macro, that takes $dom_interface:ident (<optional macro parameters>) as match arm for all interfaces that
                #[doc = concat!("`", stringify!($child_interface), "`")]
                /// inherits from
                macro_rules! [<for_all_ $child_interface:snake _ancestors>] {
                    ($mac:path, $extra_params:tt) => {
                        $mac!($interface, $extra_params);
                        $crate::interfaces::[<for_all_ $interface:snake _ancestors>]!($mac, $extra_params);
                    };
                }
                pub(crate) use [<for_all_ $child_interface:snake _ancestors>];
            )*
        }
        paste::paste! {
            /// Execute $mac which is a macro, that takes $dom_interface:ident (<optional macro parameters>) as match arm for all interfaces that inherit from
            #[doc = concat!("`", stringify!($interface), "`")]
            #[allow(unused_macros)]
            macro_rules! [<for_all_ $interface:snake _descendents>] {
                ($mac:path, $extra_params:tt) => {
                    $(
                        $mac!($child_interface, $extra_params);
                        $crate::interfaces::[<for_all_ $child_interface:snake _ descendents>]!($mac, $extra_params);
                    )*
                };
            }
            #[allow(unused_imports)]
            pub(crate) use [<for_all_ $interface:snake _descendents>];
        }

        $(
            $crate::interfaces::dom_interface_macro_and_trait_definitions_impl!(
                $child_interface {
                    methods: $child_methods_body,
                    child_interfaces: $child_interface_body
                }
            );
        )*
    };
}

pub(crate) use dom_interface_macro_and_trait_definitions_impl;

/// Recursively generates trait and macro definitions for all interfaces, defined below
/// The macros that are defined with this macro are functionally composing a macro which is invoked for all ancestor and descendent interfaces of a given interface
/// For example `for_all_html_video_element_ancestors!($mac, ())` invokes $mac! for the interfaces `HtmlMediaElement`, `HtmlElement` and `Element`
/// And `for_all_html_media_element_descendents` is run for the interfaces `HtmlAudioElement` and `HtmlVideoElement`
macro_rules! dom_interface_macro_and_trait_definitions {
    ($($interface:ident $interface_body:tt,)*) => {
        $crate::interfaces::dom_interface_macro_and_trait_definitions_impl!(
            Element {
                methods: {},
                child_interfaces: {$($interface $interface_body,)*}
            }
        );
        macro_rules! for_all_dom_interfaces {
            ($mac:path, $extra_params:tt) => {
                $mac!(Element, $extra_params);
                $crate::interfaces::for_all_element_descendents!($mac, $extra_params);
            };
        }
        pub(crate) use for_all_dom_interfaces;
    }
}

macro_rules! impl_dom_interfaces_for_ty_helper {
    ($dom_interface:ident, ($ty:ident, <$($additional_generic_var:ident,)*>, <$($additional_generic_var_on_ty:ident,)*>, {$($additional_generic_bounds:tt)*})) => {
        $crate::interfaces::impl_dom_interfaces_for_ty_helper!($dom_interface, ($ty, $dom_interface, <$($additional_generic_var,)*>, <$($additional_generic_var_on_ty,)*>, {$($additional_generic_bounds)*}));
    };
    ($dom_interface:ident, ($ty:ident, $bound_interface:ident, <$($additional_generic_var:ident,)*>, <$($additional_generic_var_on_ty:ident,)*>, {$($additional_generic_bounds:tt)*})) => {
        impl<E, T, A, $($additional_generic_var,)*> $crate::interfaces::$dom_interface<T, A> for $ty<E, T, A, $($additional_generic_var_on_ty,)*>
        where
            E: $crate::interfaces::$bound_interface<T, A>,
            $($additional_generic_bounds)*
        {
        }
    };
}

pub(crate) use impl_dom_interfaces_for_ty_helper;

/// Implement DOM interface traits for the given type and all descendent DOM interfaces,
/// such that every possible method defined on the underlying element is accessible via typing
/// The requires the type of signature Type<E, T, A>, whereas T is the AppState type, A, is Action, and E is the underlying Element type that is composed
/// It additionally accepts generic vars (vars: <vars>) that is added on the impl<E, T, A, <vars>>, and vars_on_ty (Type<E, T, A, <vars_on_ty>>) and additional generic typebounds
macro_rules! impl_dom_interfaces_for_ty {
    ($dom_interface:ident, $ty:ident) => {
        $crate::interfaces::impl_dom_interfaces_for_ty!($dom_interface, $ty, vars: <>, vars_on_ty: <>, bounds: {});
    };
    ($dom_interface:ident, $ty:ident, vars: <$($additional_generic_var:ident,)*>, vars_on_ty: <$($additional_generic_var_on_ty:ident,)*>, bounds: {$($additional_generic_bounds:tt)*}) => {
        paste::paste! {
            $crate::interfaces::[<for_all_ $dom_interface:snake _ancestors>]!(
                $crate::interfaces::impl_dom_interfaces_for_ty_helper,
                ($ty, $dom_interface, <$($additional_generic_var,)*>, <$($additional_generic_var_on_ty,)*>, {$($additional_generic_bounds)*})
            );
            $crate::interfaces::impl_dom_interfaces_for_ty_helper!($dom_interface, ($ty, $dom_interface, <$($additional_generic_var,)*>, <$($additional_generic_var_on_ty,)*>, {$($additional_generic_bounds)*}));
            $crate::interfaces::[<for_all_ $dom_interface:snake _descendents>]!(
                $crate::interfaces::impl_dom_interfaces_for_ty_helper,
                ($ty, <$($additional_generic_var,)*>, <$($additional_generic_var_on_ty,)*>, {$($additional_generic_bounds)*})
            );
        }
    };
}

pub(crate) use impl_dom_interfaces_for_ty;

dom_interface_macro_and_trait_definitions!(
    HtmlElement {
        methods: {},
        child_interfaces: {
            HtmlAnchorElement { methods: {}, child_interfaces: {} },
            HtmlAreaElement { methods: {}, child_interfaces: {} },
            // HtmlBaseElement { methods: {}, child_interfaces: {} }, TODO include metadata?
            // HtmlBodyElement { methods: {}, child_interfaces: {} }, TODO include body element?
            HtmlBrElement { methods: {}, child_interfaces: {} },
            HtmlButtonElement { methods: {}, child_interfaces: {} },
            HtmlCanvasElement {
                methods: {
                    fn width(self, value: u32) -> Attr<Self, T, A> {
                        self.attr("width", value)
                    }
                    fn height(self, value: u32) -> Attr<Self, T, A> {
                        self.attr("height", value)
                    }
                },
                child_interfaces: {}
            },
            HtmlDataElement { methods: {}, child_interfaces: {} },
            HtmlDataListElement { methods: {}, child_interfaces: {} },
            HtmlDetailsElement { methods: {}, child_interfaces: {} },
            HtmlDialogElement { methods: {}, child_interfaces: {} },
            // HtmlDirectoryElement { methods: {}, child_interfaces: {} }, deprecated
            HtmlDivElement { methods: {}, child_interfaces: {} },
            HtmlDListElement { methods: {}, child_interfaces: {} },
            // HtmlUnknownElement { methods: {}, child_interfaces: {} }, useful at all?
            HtmlEmbedElement { methods: {}, child_interfaces: {} },
            HtmlFieldSetElement { methods: {}, child_interfaces: {} },
            // HtmlFontElement { methods: {}, child_interfaces: {} }, deprecated
            HtmlFormElement { methods: {}, child_interfaces: {} },
            // HtmlFrameElement { methods: {}, child_interfaces: {} }, deprecated
            // HtmlFrameSetElement { methods: {}, child_interfaces: {} }, deprecacted
            // HtmlHeadElement { methods: {}, child_interfaces: {} }, TODO include metadata?
            HtmlHeadingElement { methods: {}, child_interfaces: {} },
            HtmlHrElement { methods: {}, child_interfaces: {} },
            // HtmlHtmlElement { methods: {}, child_interfaces: {} }, TODO include metadata?
            HtmlIFrameElement { methods: {}, child_interfaces: {} },
            HtmlImageElement { methods: {}, child_interfaces: {} },
            HtmlInputElement { methods: {}, child_interfaces: {} },
            HtmlLabelElement { methods: {}, child_interfaces: {} },
            HtmlLegendElement { methods: {}, child_interfaces: {} },
            HtmlLiElement { methods: {}, child_interfaces: {} },
            HtmlLinkElement { methods: {}, child_interfaces: {} },
            HtmlMapElement { methods: {}, child_interfaces: {} },
            HtmlMediaElement {
                methods: {},
                child_interfaces: {
                    HtmlAudioElement { methods: {}, child_interfaces: {} },
                    HtmlVideoElement {
                        methods: {
                            fn width(self, value: u32) -> Attr<Self,T, A> {
                                self.attr("width", value)
                            }
                            fn height(self, value: u32) -> Attr<Self, T, A> {
                                self.attr("height", value)
                            }
                        },
                        child_interfaces: {}
                    },
                }
            },
            HtmlMenuElement { methods: {}, child_interfaces: {} },
            // HtmlMenuItemElement { methods: {}, child_interfaces: {} }, deprecated
            // HtmlMetaElement { methods: {}, child_interfaces: {} }, TODO include metadata?
            HtmlMeterElement { methods: {}, child_interfaces: {} },
            HtmlModElement { methods: {}, child_interfaces: {} },
            HtmlObjectElement { methods: {}, child_interfaces: {} },
            HtmlOListElement { methods: {}, child_interfaces: {} },
            HtmlOptGroupElement { methods: {}, child_interfaces: {} },
            HtmlOptionElement { methods: {}, child_interfaces: {} },
            HtmlOutputElement { methods: {}, child_interfaces: {} },
            HtmlParagraphElement { methods: {}, child_interfaces: {} },
            // HtmlParamElement { methods: {}, child_interfaces: {} }, deprecated
            HtmlPictureElement { methods: {}, child_interfaces: {} },
            HtmlPreElement { methods: {}, child_interfaces: {} },
            HtmlProgressElement { methods: {}, child_interfaces: {} },
            HtmlQuoteElement { methods: {}, child_interfaces: {} },
            HtmlScriptElement { methods: {}, child_interfaces: {} },
            HtmlSelectElement { methods: {}, child_interfaces: {} },
            HtmlSlotElement { methods: {}, child_interfaces: {} },
            HtmlSourceElement { methods: {}, child_interfaces: {} },
            HtmlSpanElement { methods: {}, child_interfaces: {} },
            // HtmlStyleElement { methods: {}, child_interfaces: {} }, TODO include metadata?
            HtmlTableCaptionElement { methods: {}, child_interfaces: {} },
            HtmlTableCellElement { methods: {}, child_interfaces: {} },
            HtmlTableColElement { methods: {}, child_interfaces: {} },
            HtmlTableElement { methods: {}, child_interfaces: {} },
            HtmlTableRowElement { methods: {}, child_interfaces: {} },
            HtmlTableSectionElement { methods: {}, child_interfaces: {} },
            HtmlTemplateElement { methods: {}, child_interfaces: {} },
            HtmlTimeElement { methods: {}, child_interfaces: {} },
            HtmlTextAreaElement { methods: {}, child_interfaces: {} },
            // HtmlTitleElement { methods: {}, child_interfaces: {} }, TODO include metadata?
            HtmlTrackElement { methods: {}, child_interfaces: {} },
            HtmlUListElement { methods: {}, child_interfaces: {} },
        }
    },
    SvgElement {
        methods: {},
        child_interfaces: {
            SvgAnimationElement {
                methods: {},
                child_interfaces: {
                    SvgAnimateElement { methods: {}, child_interfaces: {} },
                    SvgAnimateMotionElement { methods: {}, child_interfaces: {} },
                    SvgAnimateTransformElement { methods: {}, child_interfaces: {} },
                    SvgSetElement { methods: {}, child_interfaces: {} },
                }
            },
            SvgClipPathElement { methods: {}, child_interfaces: {} },
            SvgComponentTransferFunctionElement {
                methods: {},
                child_interfaces: {
                    SvgfeFuncAElement { methods: {}, child_interfaces: {} },
                    SvgfeFuncBElement { methods: {}, child_interfaces: {} },
                    SvgfeFuncGElement { methods: {}, child_interfaces: {} },
                    SvgfeFuncRElement { methods: {}, child_interfaces: {} },
                }
            },
            SvgDescElement { methods: {}, child_interfaces: {} },
            SvgFilterElement { methods: {}, child_interfaces: {} },
            SvgGradientElement {
                methods: {},
                child_interfaces: {
                    SvgLinearGradientElement { methods: {}, child_interfaces: {} },
                    SvgRadialGradientElement { methods: {}, child_interfaces: {} },
                }
            },
            SvgGraphicsElement {
                methods: {},
                child_interfaces: {
                    SvgDefsElement { methods: {}, child_interfaces: {} },
                    SvgForeignObjectElement { methods: {}, child_interfaces: {} },
                    SvgGeometryElement {
                        methods: {
                            fn stroke(self, brush: impl Into<peniko::Brush>, style: peniko::kurbo::Stroke) -> crate::svg::Stroke<Self, T, A> {
                                crate::svg::stroke(self, brush, style)
                            }
                        },
                        child_interfaces: {
                            SvgCircleElement {
                                methods: {
                                    fn fill(self, brush: impl Into<peniko::Brush>) -> crate::svg::Fill<Self, T, A> {
                                        crate::svg::fill(self, brush)
                                    }
                                },
                                child_interfaces: {}
                            },
                            SvgEllipseElement {
                                methods: {
                                    fn fill(self, brush: impl Into<peniko::Brush>) -> crate::svg::Fill<Self, T, A> {
                                        crate::svg::fill(self, brush)
                                    }
                                },
                                child_interfaces: {}
                            },
                            SvgLineElement { methods: {}, child_interfaces: {} },
                            SvgPathElement {
                                methods: {
                                    fn fill(self, brush: impl Into<peniko::Brush>) -> crate::svg::Fill<Self, T, A> {
                                        crate::svg::fill(self, brush)
                                    }
                                },
                                child_interfaces: {}
                            },
                            SvgPolygonElement {
                                methods: {
                                    fn fill(self, brush: impl Into<peniko::Brush>) -> crate::svg::Fill<Self, T, A> {
                                        crate::svg::fill(self, brush)
                                    }
                                },
                                child_interfaces: {}
                            },
                            SvgPolylineElement {
                                methods: {
                                    fn fill(self, brush: impl Into<peniko::Brush>) -> crate::svg::Fill<Self, T, A> {
                                        crate::svg::fill(self, brush)
                                    }
                                },
                                child_interfaces: {}
                            },
                            SvgRectElement {
                                methods: {
                                    fn fill(self, brush: impl Into<peniko::Brush>) -> crate::svg::Fill<Self, T, A> {
                                        crate::svg::fill(self, brush)
                                    }
                                },
                                child_interfaces: {}
                            },
                        }
                    },
                    SvgImageElement { methods: {}, child_interfaces: {} },
                    SvgSwitchElement { methods: {}, child_interfaces: {} },
                    SvgTextContentElement {
                        methods: {
                            fn fill(self, brush: impl Into<peniko::Brush>) -> crate::svg::Fill<Self, T, A> {
                                crate::svg::fill(self, brush)
                            }
                            fn stroke(self, brush: impl Into<peniko::Brush>, style: peniko::kurbo::Stroke) -> crate::svg::Stroke<Self, T, A> {
                                crate::svg::stroke(self, brush, style)
                            }
                        },
                        child_interfaces: {
                            SvgTextPathElement { methods: {}, child_interfaces: {} },
                            SvgTextPositioningElement {
                                methods: {},
                                child_interfaces: {
                                    SvgTextElement { methods: {}, child_interfaces: {} },
                                    SvgtSpanElement { methods: {}, child_interfaces: {} },
                                }
                            },
                        }
                    },
                    SvgUseElement { methods: {}, child_interfaces: {} },
                    SvgaElement { methods: {}, child_interfaces: {} },
                    SvggElement {
                        methods: {
                            fn fill(self, brush: impl Into<peniko::Brush>) -> crate::svg::Fill<Self, T, A> {
                                crate::svg::fill(self, brush)
                            }
                            fn stroke(self, brush: impl Into<peniko::Brush>, style: peniko::kurbo::Stroke) -> crate::svg::Stroke<Self, T, A> {
                                crate::svg::stroke(self, brush, style)
                            }
                        },
                        child_interfaces: {}
                    },
                    SvgsvgElement { methods: {}, child_interfaces: {} },
                }
            },
            SvgMarkerElement { methods: {}, child_interfaces: {} },
            SvgMaskElement { methods: {}, child_interfaces: {} },
            SvgMetadataElement { methods: {}, child_interfaces: {} },
            SvgPatternElement { methods: {}, child_interfaces: {} },
            SvgScriptElement { methods: {}, child_interfaces: {} },
            SvgStopElement { methods: {}, child_interfaces: {} },
            SvgStyleElement { methods: {}, child_interfaces: {} },
            SvgSymbolElement { methods: {}, child_interfaces: {} },
            SvgTitleElement { methods: {}, child_interfaces: {} },
            SvgViewElement { methods: {}, child_interfaces: {} },
            SvgfeBlendElement { methods: {}, child_interfaces: {} },
            SvgfeColorMatrixElement { methods: {}, child_interfaces: {} },
            SvgfeComponentTransferElement { methods: {}, child_interfaces: {} },
            SvgfeCompositeElement { methods: {}, child_interfaces: {} },
            SvgfeConvolveMatrixElement { methods: {}, child_interfaces: {} },
            SvgfeDiffuseLightingElement { methods: {}, child_interfaces: {} },
            SvgfeDisplacementMapElement { methods: {}, child_interfaces: {} },
            SvgfeDistantLightElement { methods: {}, child_interfaces: {} },
            SvgfeDropShadowElement { methods: {}, child_interfaces: {} },
            SvgfeFloodElement { methods: {}, child_interfaces: {} },
            SvgfeGaussianBlurElement { methods: {}, child_interfaces: {} },
            SvgfeImageElement { methods: {}, child_interfaces: {} },
            SvgfeMergeElement { methods: {}, child_interfaces: {} },
            SvgfeMergeNodeElement { methods: {}, child_interfaces: {} },
            SvgfeMorphologyElement { methods: {}, child_interfaces: {} },
            SvgfeOffsetElement { methods: {}, child_interfaces: {} },
            SvgfePointLightElement { methods: {}, child_interfaces: {} },
            SvgfeSpecularLightingElement { methods: {}, child_interfaces: {} },
            SvgfeSpotLightElement { methods: {}, child_interfaces: {} },
            SvgfeTileElement { methods: {}, child_interfaces: {} },
            SvgfeTurbulenceElement { methods: {}, child_interfaces: {} },
            SvgmPathElement { methods: {}, child_interfaces: {} },
        }
    },
);

// Core View implementations

impl<ParentT, ParentA, ChildT, ChildA, V, F> sealed::Sealed
    for crate::Adapt<ParentT, ParentA, ChildT, ChildA, V, F>
{
}
impl<ParentT, ChildT, V, F> sealed::Sealed for crate::AdaptState<ParentT, ChildT, V, F> {}

macro_rules! impl_dom_traits_for_adapt_views {
    ($dom_interface:ident, ()) => {
        impl<ParentT, ParentA, ChildT, ChildA, V, F> $dom_interface<ParentT, ParentA>
            for crate::Adapt<ParentT, ParentA, ChildT, ChildA, V, F>
        where
            V: $dom_interface<ChildT, ChildA>,
            F: Fn(
                &mut ParentT,
                crate::AdaptThunk<ChildT, ChildA, V>,
            ) -> xilem_core::MessageResult<ParentA>,
        {
        }
        impl<ParentT, ChildT, A, V, F> $dom_interface<ParentT, A>
            for crate::AdaptState<ParentT, ChildT, V, F>
        where
            V: $dom_interface<ChildT, A>,
            F: Fn(&mut ParentT) -> &mut ChildT,
        {
        }
    };
}
for_all_dom_interfaces!(impl_dom_traits_for_adapt_views, ());
