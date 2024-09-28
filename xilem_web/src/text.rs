// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "hydration")]
use wasm_bindgen::JsCast;
use xilem_core::{Mut, OrphanView};

use crate::{DynMessage, Pod, ViewCtx};

// strings -> text nodes
macro_rules! impl_string_view {
    ($ty:ty) => {
        impl<State, Action> OrphanView<$ty, State, Action, DynMessage> for ViewCtx {
            type OrphanElement = Pod<web_sys::Text>;

            type OrphanViewState = ();

            fn orphan_build(
                view: &$ty,
                #[cfg_attr(not(feature = "hydration"), allow(unused_variables))] ctx: &mut ViewCtx,
            ) -> (Self::OrphanElement, Self::OrphanViewState) {
                #[cfg(feature = "hydration")]
                let node = if ctx.is_hydrating() {
                    ctx.hydrate_node().unwrap().unchecked_into()
                } else {
                    web_sys::Text::new_with_data(view).unwrap()
                };
                #[cfg(not(feature = "hydration"))]
                let node = web_sys::Text::new_with_data(view).unwrap();
                (Pod { node, props: () }, ())
            }

            fn orphan_rebuild<'a>(
                new: &$ty,
                prev: &$ty,
                (): &mut Self::OrphanViewState,
                _ctx: &mut ViewCtx,
                element: Mut<'a, Self::OrphanElement>,
            ) -> Mut<'a, Self::OrphanElement> {
                if prev != new {
                    element.node.set_data(new);
                }
                element
            }

            fn orphan_teardown(
                _view: &$ty,
                _view_state: &mut Self::OrphanViewState,
                _ctx: &mut ViewCtx,
                _element: Mut<'_, Pod<web_sys::Text>>,
            ) {
            }

            fn orphan_message(
                _view: &$ty,
                _view_state: &mut Self::OrphanViewState,
                _id_path: &[xilem_core::ViewId],
                message: DynMessage,
                _app_state: &mut State,
            ) -> xilem_core::MessageResult<Action, DynMessage> {
                xilem_core::MessageResult::Stale(message)
            }
        }
    };
}

impl_string_view!(&'static str);
impl_string_view!(String);
impl_string_view!(std::borrow::Cow<'static, str>);

macro_rules! impl_to_string_view {
    ($ty:ty) => {
        impl<State, Action> OrphanView<$ty, State, Action, DynMessage> for ViewCtx {
            type OrphanElement = Pod<web_sys::Text>;

            type OrphanViewState = ();

            fn orphan_build(
                view: &$ty,
                #[cfg_attr(not(feature = "hydration"), allow(unused_variables))] ctx: &mut ViewCtx,
            ) -> (Self::OrphanElement, Self::OrphanViewState) {
                #[cfg(feature = "hydration")]
                let node = if ctx.is_hydrating() {
                    ctx.hydrate_node().unwrap().unchecked_into()
                } else {
                    web_sys::Text::new_with_data(&view.to_string()).unwrap()
                };
                #[cfg(not(feature = "hydration"))]
                let node = web_sys::Text::new_with_data(&view.to_string()).unwrap();
                (Pod { node, props: () }, ())
            }

            fn orphan_rebuild<'a>(
                new: &$ty,
                prev: &$ty,
                (): &mut Self::OrphanViewState,
                _ctx: &mut ViewCtx,
                element: Mut<'a, Self::OrphanElement>,
            ) -> Mut<'a, Self::OrphanElement> {
                if prev != new {
                    element.node.set_data(&new.to_string());
                }
                element
            }

            fn orphan_teardown(
                _view: &$ty,
                _view_state: &mut Self::OrphanViewState,
                _ctx: &mut ViewCtx,
                _element: Mut<'_, Pod<web_sys::Text>>,
            ) {
            }

            fn orphan_message(
                _view: &$ty,
                _view_state: &mut Self::OrphanViewState,
                _id_path: &[xilem_core::ViewId],
                message: DynMessage,
                _app_state: &mut State,
            ) -> xilem_core::MessageResult<Action, DynMessage> {
                xilem_core::MessageResult::Stale(message)
            }
        }
    };
}

// Allow numbers to be used directly as a view
impl_to_string_view!(f32);
impl_to_string_view!(f64);
impl_to_string_view!(i8);
impl_to_string_view!(u8);
impl_to_string_view!(i16);
impl_to_string_view!(u16);
impl_to_string_view!(i32);
impl_to_string_view!(u32);
impl_to_string_view!(i64);
impl_to_string_view!(u64);
impl_to_string_view!(u128);
impl_to_string_view!(isize);
impl_to_string_view!(usize);
