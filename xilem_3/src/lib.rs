//! An unproven extension of Xilem.

use std::{marker::PhantomData, sync::Arc};

use xilem_core::{View, ViewPathTracker};

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
trait Component<AppState> {
    type Element;
    type View: View<AppState, Action, ViewCtx, Element = Self::Element>;
    type HyperState;
    fn create(self, state: &mut AppState) -> (Self::View, Self::HyperState);
    fn update(self, hyper_state: &mut Self::HyperState, app_state: &mut AppState) -> Self::View;
}

impl<F, AppState, ResultComponent> Component<AppState> for F
where
    // TODO: We can have extra hooks in the arguments here :eyes:
    F: FnOnce(&mut AppState) -> ResultComponent,
    ResultComponent: Component<AppState>,
{
    type Element = ResultComponent::Element;
    type View = ResultComponent::View;
    type HyperState = ResultComponent::HyperState;

    fn create(self, state: &mut AppState) -> (Self::View, Self::HyperState) {
        let component = (self)(state);
        component.create(state)
    }

    fn update(
        // &mut self,
        self,
        // previous: Option<&mut Self>,
        hyper_state: &mut Self::HyperState,
        app_state: &mut AppState,
    ) -> Self::View {
        let component = (self)(app_state);
        component.update(hyper_state, app_state)
    }
}

trait WidgetView<State>:
    View<State, Action, ViewCtx, Element = Pod<Self::Widget>> + Send + Sync
{
    type Widget: ?Sized;
}

struct Pod<T: ?Sized> {
    val: PhantomData<T>,
}

struct ExampleAppState;

// fn app_logic(state: &mut ExampleAppState) -> impl HyperView<ExampleAppState> {}

struct Memoized<Data, NewData, AppState, Component, InitData, ResultHyper>
where
    // TODO: Should these be FnOnce?
    InitData: Fn(&mut AppState) -> Data,
    NewData: Fn(&mut AppState, &mut Data) -> bool,
    Component: FnOnce(&mut AppState, &Data) -> ResultHyper,
{
    memoize: NewData,
    init_data: InitData,
    component: Component,
    #[expect(clippy::type_complexity, reason = "PhantomData matches where clauses.")]
    phantom: PhantomData<(
        fn(&mut AppState, Data) -> Data,
        fn(&mut AppState, &Data) -> ResultHyper,
    )>,
}

impl<Data, NewData, AppState, Memo, Comp, InitData> Component<AppState>
    for Memoized<Data, NewData, AppState, Memo, InitData, Comp>
where
    InitData: Fn(&mut AppState) -> Data,
    NewData: Fn(&mut AppState, &mut Data) -> bool,
    Memo: Fn(&mut AppState, &Data) -> Comp,
    Comp: Component<AppState>,
{
    type Element = Comp::Element;
    type View = Arc<Comp::View>;
    type HyperState = (Data, Arc<Comp::View>, Comp::HyperState);

    fn create(self, app_state: &mut AppState) -> (Self::View, Self::HyperState) {
        let data = (self.init_data)(app_state);
        let component = (self.component)(app_state, &data);
        let (view, child_state) = component.create(app_state);
        let view = Arc::new(view);
        (view.clone(), (data, view, child_state))
    }

    fn update(self, hyper_state: &mut Self::HyperState, app_state: &mut AppState) -> Self::View {
        let (data, stored_view, hyper_state) = hyper_state;
        if (self.memoize)(app_state, data) {
            // TODO: We could optimisitically store the old version of `stored_view`,
            // so as to reuse this allocation (if `Arc::get_mut` doesn't error).
            let hyper = (self.component)(app_state, data);
            let view = hyper.update(hyper_state, app_state);
            *stored_view = Arc::new(view);
        }
        stored_view.clone()
    }
}
