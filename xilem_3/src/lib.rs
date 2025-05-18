//! An unproven extension of Xilem.

use std::{marker::PhantomData, sync::Arc};

use xilem_core::{View, ViewPathTracker};

struct AppState;

struct ViewCtx {}

impl ViewPathTracker for ViewCtx {
    fn push_id(&mut self, _: xilem_core::ViewId) {
        todo!()
    }

    fn pop_id(&mut self) {
        todo!()
    }

    fn view_path(&mut self) -> &[xilem_core::ViewId] {
        todo!()
    }
}

// pub trait HyperView<State, Action = ()>:
//     for <'a> View< State, Action, ViewCtx, ViewState = Hyper<AppState,>,Element = Hyper<State>> + Send + Sync

// {

//     type Widget: ?Sized;
// }

struct Action;

// TODO: Maybe this type should be renamed View.
/// Functionality which runs on the main UI thread with your app's state.
pub trait HyperView<State> {
    type Element;
    type View: View<State, Action, ViewCtx, Element = Self::Element>;
    type HyperState;
    fn build(
        // &mut self,
        self,
        // previous: Option<&mut Self>,
        hyper_state: &mut Self::HyperState,
        app_state: &mut State,
    ) -> Self::View;
}

pub trait WidgetView<State>:
    View<State, Action, ViewCtx, Element = Pod<Self::Widget>> + Send + Sync
{
    type Widget: ?Sized;
}

struct Pod<T: ?Sized> {
    val: PhantomData<T>,
}

fn app_logic(state: &mut AppState) -> impl HyperView<AppState> {}

struct Memoized<Data, NewData, AppState, Component, InitData, ResultHyper>
where
    // TODO: Should these be FnOnce?
    InitData: Fn(&mut AppState) -> Data,
    NewData: Fn(&mut AppState, &mut Data) -> bool,
    Component: Fn(&mut AppState, &Data) -> ResultHyper,
{
    memoize: NewData,
    init_data: InitData,
    component: Component,
    phantom: PhantomData<(
        fn(&mut AppState, Data) -> Data,
        fn(&mut AppState, &Data) -> ResultHyper,
    )>,
}

impl<Data, NewData, AppState, Component, InitData, ResultHyper, ResultView> HyperView<AppState>
    for Memoized<Data, NewData, AppState, Component, InitData, ResultHyper>
where
    InitData: Fn(&mut AppState) -> Data,
    NewData: Fn(&mut AppState, &mut Data) -> bool,
    Component: Fn(&mut AppState, &Data) -> ResultHyper,
    ResultHyper: HyperView<AppState, View = ResultView>,
    ResultView: View<AppState, Action, ViewCtx>,
    ResultHyper::HyperState: Default,
{
    type Element = ResultView::Element;
    type View = Arc<ResultView>;
    type HyperState = Option<(Data, Arc<ResultView>, ResultHyper::HyperState)>;

    fn build(
        self,
        // &mut self,
        // previous: Option<&mut Self>,
        hyper_state: &mut Self::HyperState,
        app_state: &mut AppState,
    ) -> Self::View {
        match hyper_state {
            Some((data, stored_view, hyper_state)) => {
                if (self.memoize)(app_state, data) {
                    // TODO: We could optimisitically store the old version of `stored_view`,
                    // so as to reuse this allocation if `Arc::get_mut` doesn't error.
                    let hyper = (self.component)(app_state, &data);
                    let view = hyper.build(hyper_state, app_state);
                    *stored_view = Arc::new(view);
                }
                stored_view.clone()
            }
            None => {
                let data = (self.init_data)(app_state);
                let hyper = (self.component)(app_state, &data);
                let mut child_state = ResultHyper::HyperState::default();
                let view = hyper.build(&mut child_state, app_state);
                let view = Arc::new(view);
                *hyper_state = Some((data, view.clone(), child_state));
                return view;
            }
        }
    }
}
