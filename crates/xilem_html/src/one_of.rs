use wasm_bindgen::throw_str;

use crate::{ChangeFlags, Cx, Pod, View, ViewMarker, ViewSequence};

macro_rules! one_of {
    (
        #[doc = $first_doc_line:literal]
        $ident:ident { $( $vars:ident ),+ }
    ) => {
        #[doc = $first_doc_line]
        ///
        /// It is a statically-typed alternative to the type-erased `AnyView`.
        pub enum $ident<$($vars),+> {
            $($vars($vars),)+
        }

        impl<$($vars),+> AsRef<web_sys::Node> for $ident<$($vars),+>
        where
            $($vars: AsRef<web_sys::Node>,)+
        {
            fn as_ref(&self) -> &web_sys::Node {
                match self {
                    $( $ident::$vars(view) => view.as_ref(), )+
                }
            }
        }

        impl<VT, VA, $($vars),+> View<VT, VA> for $ident<$($vars),+>
        where $(
            $vars: View<VT, VA> + ViewMarker,
            $vars::Element: AsRef<web_sys::Node> + 'static,
        )+ {
            type State = $ident<$($vars::State),+>;
            type Element = $ident<$($vars::Element),+>;

            fn build(&self, cx: &mut Cx) -> (xilem_core::Id, Self::State, Self::Element) {
                match self {
                    $(
                        $ident::$vars(view) => {
                            let (id, state, el) = view.build(cx);
                            (id, $ident::$vars(state), $ident::$vars(el))
                        }
                    )+
                }
            }

            fn rebuild(
                &self,
                cx: &mut Cx,
                prev: &Self,
                id: &mut xilem_core::Id,
                state: &mut Self::State,
                element: &mut Self::Element,
            ) -> ChangeFlags {
                match (prev, self) {
                    $(
                        // Variant is the same as before
                        ($ident::$vars(prev_view), $ident::$vars(view)) => {
                            let ($ident::$vars(state), $ident::$vars(element)) = (state, element)
                            else {
                                throw_str(concat!(
                                    "invalid state/view in ", stringify!($ident), " (unreachable)",
                                ));
                            };
                            view.rebuild(cx, prev_view, id, state, element)
                        }
                        // Variant has changed
                        (_, $ident::$vars(view)) => {
                            let (new_id, new_state, new_element) = view.build(cx);
                            *id = new_id;
                            *state = $ident::$vars(new_state);
                            *element = $ident::$vars(new_element);
                            ChangeFlags::STRUCTURE
                        }
                    )+
                }
            }

            fn message(
                &self,
                id_path: &[xilem_core::Id],
                state: &mut Self::State,
                message: Box<dyn std::any::Any>,
                app_state: &mut VT,
            ) -> xilem_core::MessageResult<VA> {
                match self {
                    $(
                        $ident::$vars(view) => {
                            let $ident::$vars(state) = state else {
                                throw_str(concat!(
                                    "invalid state/view in", stringify!($ident), "(unreachable)",
                                ));
                            };
                            view.message(id_path, state, message, app_state)
                        }
                    )+
                }
            }
        }

        impl<VT, VA, $($vars),+> ViewSequence<VT, VA> for $ident<$($vars),+>
        where $(
            $vars: ViewSequence<VT, VA>,
        )+ {
            type State = $ident<$($vars::State),+>;

            fn build(&self, cx: &mut Cx, elements: &mut Vec<Pod>) -> Self::State {
                match self {
                    $(
                        $ident::$vars(view_sequence) => {
                            $ident::$vars(view_sequence.build(cx, elements))
                        }
                    )+
                }
            }

            fn rebuild(
                &self,
                cx: &mut Cx,
                prev: &Self,
                state: &mut Self::State,
                element: &mut xilem_core::VecSplice<Pod>,
            ) -> ChangeFlags {
                match (prev, self) {
                    $(
                        // Variant is the same as before
                        ($ident::$vars(prev_view), $ident::$vars(view_sequence)) => {
                            let $ident::$vars(state) = state else {
                                throw_str(concat!(
                                    "invalid state/view_sequence in ",
                                    stringify!($ident),
                                    " (unreachable)",
                                ));
                            };
                            view_sequence.rebuild(cx, prev_view, state, element)
                        }
                        // Variant has changed
                        (_, $ident::$vars(view_sequence)) => {
                            let new_state =
                                element.as_vec(|elements| view_sequence.build(cx, elements));
                            *state = $ident::$vars(new_state);
                            ChangeFlags::STRUCTURE
                        }
                    )+
                }
            }

            fn message(
                &self,
                id_path: &[xilem_core::Id],
                state: &mut Self::State,
                message: Box<dyn std::any::Any>,
                app_state: &mut VT,
            ) -> xilem_core::MessageResult<VA> {
                match self {
                    $(
                        $ident::$vars(view_sequence) => {
                            let $ident::$vars(state) = state else {
                                throw_str(concat!(
                                    "invalid state/view_sequence in ",
                                    stringify!($ident),
                                    " (unreachable)",
                                ));
                            };
                            view_sequence.message(id_path, state, message, app_state)
                        }
                    )+
                }
            }

            fn count(&self, state: &Self::State) -> usize {
                match self {
                    $(
                        $ident::$vars(view_sequence) => {
                            let $ident::$vars(state) = state else {
                                throw_str(concat!(
                                    "invalid state/view_sequence in ",
                                    stringify!($ident),
                                    " (unreachable)",
                                ));
                            };
                            view_sequence.count(state)
                        }
                    )+
                }
            }
        }

    };
}

one_of! {
    /// This view container can switch between two views.
    OneOf2 { A, B }
}
one_of! {
    /// This view container can switch between three views.
    OneOf3 { A, B, C }
}

one_of! {
    /// This view container can switch between four views.
    OneOf4 { A, B, C, D }
}

one_of! {
    /// This view container can switch between five views.
    OneOf5 { A, B, C, D, E }
}

one_of! {
    /// This view container can switch between six views.
    OneOf6 { A, B, C, D, E, F }
}

one_of! {
    /// This view container can switch between seven views.
    OneOf7 { A, B, C, D, E, F, G }
}

one_of! {
    /// This view container can switch between eight views.
    OneOf8 { A, B, C, D, E, F, G, H }
}
