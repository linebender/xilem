// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use core::fmt::Debug;

use crate::{MessageResult, NoElement, View, ViewMarker, ViewPathTracker};

/// A view which executes `once` exactly once.
///
/// `once` will be called only when the returned view is [built](View::build).
///
/// This is a [`NoElement`] view, and so should either be used in any sequence, or with [`fork`](crate::fork).
///
/// ## Examples
///
/// This can be useful for logging a value:
///
/// ```
/// # use xilem_core::{run_once, View, docs::{Fake as ViewCtx, DocsView as WidgetView}};
/// # struct AppData;
/// fn log_lifecycle(data: &mut AppData) -> impl WidgetView<AppData, ()> {
///     run_once(|| eprintln!("View constructed"))
/// }
/// ```
/// ## Capturing
///
/// This method cannot be used with a dynamic `once`.
/// That is, `once` cannot be a function pointer or capture any (non-zero sized) values.
/// You might otherwise expect the function to be reran when the captured values change, which is not the case.
/// [`run_once_raw`] is the same as `run_once`, but without this restriction.
///
/// // <https://doc.rust-lang.org/error_codes/E0080.html>
/// // Note that this error code is only checked on nightly
/// ```compile_fail,E0080
/// # use xilem_core::{run_once, View, docs::{DocsView as WidgetView}};
/// # struct AppData {
/// #    data: u32
/// # }
/// fn log_data(app: &mut AppData) -> impl WidgetView<AppData, ()> {
///     let val = app.data;
///     run_once(move || println!("{}", val))
/// }
/// # // We need to call the function to make the inline constant be evaluated
/// # let _ = log_data(&mut AppData { data: 10 });
/// ```
pub fn run_once<F>(once: F) -> RunOnce<F>
where
    F: Fn() + 'static,
{
    const {
        assert!(
            core::mem::size_of::<F>() == 0,
            "`run_once` will not be ran again when its captured variables are updated.\n\
            To ignore this warning, use `run_once_raw`."
        );
    };
    RunOnce { once }
}

/// A view which executes `once` exactly once.
///
/// This is [`run_once`] without the capturing rules.
/// See [`run_once`] for full documentation.
pub fn run_once_raw<F>(once: F) -> RunOnce<F>
where
    F: Fn() + 'static,
{
    RunOnce { once }
}

/// The view type for [`run_once`].
///
/// This is a [`NoElement`] view.
pub struct RunOnce<F> {
    once: F,
}

impl<F> ViewMarker for RunOnce<F> {}
impl<F, State, Action, Context, Message> View<State, Action, Context, Message> for RunOnce<F>
where
    Context: ViewPathTracker,
    F: Fn() + 'static,
    // TODO: Work out what traits we want to require `Message`s to have
    Message: Debug,
{
    type Element = NoElement;

    type ViewState = ();

    fn build(&self, _: &mut Context) -> (Self::Element, Self::ViewState) {
        (self.once)();
        (NoElement, ())
    }

    fn rebuild<'el>(
        &self,
        _: &Self,
        (): &mut Self::ViewState,
        _: &mut Context,
        (): crate::Mut<'el, Self::Element>,
    ) -> crate::Mut<'el, Self::Element> {
        // Nothing to do
    }

    fn teardown(
        &self,
        (): &mut Self::ViewState,
        _: &mut Context,
        _: crate::Mut<'_, Self::Element>,
    ) {
        // Nothing to do
    }

    fn message(
        &self,
        (): &mut Self::ViewState,
        _: &[crate::ViewId],
        message: Message,
        _: &mut State,
    ) -> MessageResult<Action, Message> {
        // Nothing to do
        panic!("Message should not have been sent to a `RunOnce` View: {message:?}");
    }
}
