use core::marker::PhantomData;

use crate::{View, ViewMarker, ViewPathTracker};

pub fn lens<OuterState, Action, Context, Message, InnerState, MapState, InnerView, Component>(
    state: &mut OuterState,
    map: MapState,
    component: Component,
) -> Lens<InnerView, MapState, (OuterState, InnerState, Action, Context, Message)>
where
    MapState: Fn(&mut OuterState) -> &mut InnerState + Send + Sync + 'static,
    Component: FnOnce(&mut InnerState) -> InnerView,
    InnerView: View<InnerState, Action, Context, Message>,
    Context: ViewPathTracker,
{
    let mapped = map(state);
    let view = component(mapped);
    Lens {
        view,
        map,
        phantom: PhantomData,
    }
}

pub struct Lens<InnerView, MapState, Phantom> {
    view: InnerView,
    map: MapState,
    phantom: PhantomData<Phantom>,
}

impl<InnerView, MapState, Phantom> ViewMarker for Lens<InnerView, MapState, Phantom> {}
impl<OuterState, Action, Context, Message, InnerState, MapState, InnerView>
    View<OuterState, Action, Context, Message>
    for Lens<InnerView, MapState, (OuterState, InnerState, Action, Context, Message)>
where
    MapState: Fn(&mut OuterState) -> &mut InnerState + Send + Sync + 'static,
    InnerView: View<InnerState, Action, Context, Message>,
    Context: ViewPathTracker,
    OuterState: 'static,
    InnerState: 'static,
    Action: 'static,
    Context: 'static,
    Message: 'static,
{
    type Element = InnerView::Element;

    type ViewState = InnerView::ViewState;

    fn build(&self, ctx: &mut Context) -> (Self::Element, Self::ViewState) {
        self.view.build(ctx)
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: crate::Mut<'el, Self::Element>,
    ) -> crate::Mut<'el, Self::Element> {
        self.view.rebuild(&prev.view, view_state, ctx, element)
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: crate::Mut<'_, Self::Element>,
    ) {
        self.view.teardown(view_state, ctx, element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[crate::ViewId],
        message: Message,
        app_state: &mut OuterState,
    ) -> crate::MessageResult<Action, Message> {
        let inner_state = (self.map)(app_state);
        self.view.message(view_state, id_path, message, inner_state)
    }
}
