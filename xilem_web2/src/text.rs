use xilem_core::{Mut, OrphanView, AsOrphanView, View};

use crate::{Pod, ViewCtx};

pub struct Text<T>(T);

// // Due to new limitations of the orphan rule in xilem_core a new type wrapper is necessary here
// pub fn text<T>(text: T) -> Text<T> {
//     Text(text)
// }

// strings -> text nodes
macro_rules! impl_string_view {
    ($ty:ty) => {
        impl<State, Action> View<State, Action, ViewCtx> for Text<$ty> {
            type Element = Pod<web_sys::Text>;

            type ViewState = ();

            fn build(&self, _ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
                let pod = Pod {
                    node: web_sys::Text::new_with_data(&self.0).unwrap(),
                    props: (),
                };
                (pod, ())
            }

            fn rebuild<'a>(
                &self,
                prev: &Self,
                (): &mut Self::ViewState,
                _ctx: &mut ViewCtx,
                element: <Self::Element as xilem_core::ViewElement>::Mut<'a>,
            ) -> <Self::Element as xilem_core::ViewElement>::Mut<'a> {
                if prev.0 != self.0 {
                    element.node.set_data(&self.0);
                }
                element
            }

            fn teardown(
                &self,
                _view_state: &mut Self::ViewState,
                _ctx: &mut ViewCtx,
                _element: Mut<'_, Pod<web_sys::Text>>,
            ) {
            }

            fn message(
                &self,
                _view_state: &mut Self::ViewState,
                _id_path: &[xilem_core::ViewId],
                message: xilem_core::DynMessage,
                _app_state: &mut State,
            ) -> xilem_core::MessageResult<Action> {
                xilem_core::MessageResult::Stale(message)
            }
        }

        impl<State, Action> AsOrphanView<$ty, State, Action> for ViewCtx {
            type V = Text<$ty>;

            fn as_view(value: &$ty) -> Self::V {
                Text(value)
                // text(value)
            }
        }
    };
}

macro_rules! impl_string_orphan_view {
    ($ty:ty) => {
        impl<State, Action> OrphanView<$ty, State, Action> for ViewCtx {
            type Element = Pod<web_sys::Text>;

            type ViewState = ();

            fn build(view: &$ty, _ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
                let pod = Pod {
                    node: web_sys::Text::new_with_data(view).unwrap(),
                    props: (),
                };
                (pod, ())
            }

            fn rebuild<'a>(
                new: &$ty,
                prev: &$ty,
                (): &mut Self::ViewState,
                _ctx: &mut ViewCtx,
                element: Mut<'a, Self::Element>,
            ) -> Mut<'a, Self::Element> {
                if prev != new {
                    element.node.set_data(new);
                }
                element
            }

            fn teardown(
                _view: &$ty,
                _view_state: &mut Self::ViewState,
                _ctx: &mut ViewCtx,
                _element: Mut<'_, Pod<web_sys::Text>>,
            ) {
            }

            fn message(
                _view: &$ty,
                _view_state: &mut Self::ViewState,
                _id_path: &[xilem_core::ViewId],
                message: xilem_core::DynMessage,
                _app_state: &mut State,
            ) -> xilem_core::MessageResult<Action> {
                xilem_core::MessageResult::Stale(message)
            }
        }
    };
}

// own?
impl_string_orphan_view!(String);
impl_string_view!(&'static str);
// impl_string_view!(std::borrow::Cow<'static, str>);

macro_rules! impl_to_string_view {
    ($ty:ty) => {
        impl<State, Action> View<State, Action, ViewCtx> for Text<$ty> {
            type Element = Pod<web_sys::Text>;

            type ViewState = ();

            fn build(&self, _ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
                let pod = Pod {
                    node: web_sys::Text::new_with_data(&self.0.to_string()).unwrap(),
                    props: (),
                };
                (pod, ())
            }

            fn rebuild<'a>(
                &self,
                prev: &Self,
                (): &mut Self::ViewState,
                _ctx: &mut ViewCtx,
                element: <Self::Element as xilem_core::ViewElement>::Mut<'a>,
            ) -> <Self::Element as xilem_core::ViewElement>::Mut<'a> {
                if prev.0 != self.0 {
                    element.node.set_data(&self.0.to_string());
                }
                element
            }

            fn teardown(
                &self,
                _view_state: &mut Self::ViewState,
                _ctx: &mut ViewCtx,
                _element: Mut<'_, Pod<web_sys::Text>>,
            ) {
            }

            fn message(
                &self,
                _view_state: &mut Self::ViewState,
                _id_path: &[xilem_core::ViewId],
                message: xilem_core::DynMessage,
                _app_state: &mut State,
            ) -> xilem_core::MessageResult<Action> {
                xilem_core::MessageResult::Stale(message)
            }
        }

        impl<State, Action> AsOrphanView<$ty, State, Action> for ViewCtx {
            type V = Text<$ty>;

            fn as_view(value: &$ty) -> Self::V {
                // text(*value)
                Text(*value)
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
