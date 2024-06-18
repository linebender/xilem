use crate::{Mut, NoElement, View, ViewId, ViewPathTracker};

pub fn fork<Active, Alongside>(
    active_view: Active,
    alongside_view: Alongside,
) -> Fork<Active, Alongside> {
    Fork {
        active_view,
        alongside_view,
    }
}

pub struct Fork<Active, Alongside> {
    active_view: Active,
    alongside_view: Alongside,
}

impl<State, Action, Context, Active, Alongside> View<State, Action, Context>
    for Fork<Active, Alongside>
where
    Active: View<State, Action, Context>,
    Alongside: View<State, Action, Context, Element = NoElement>,
    Context: ViewPathTracker,
{
    type Element = Active::Element;

    type ViewState = (Active::ViewState, Alongside::ViewState);

    fn build(&self, ctx: &mut Context) -> (Self::Element, Self::ViewState) {
        let (NoElement, alongside_state) =
            ctx.with_id(ViewId::new(0), |ctx| self.alongside_view.build(ctx));
        let (element, active_state) =
            ctx.with_id(ViewId::new(1), |ctx| self.active_view.build(ctx));
        (element, (active_state, alongside_state))
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        (active_state, alongside_state): &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        ctx.with_id(ViewId::new(0), |ctx| {
            self.alongside_view
                .rebuild(&prev.alongside_view, alongside_state, ctx, ())
        });
        ctx.with_id(ViewId::new(1), |ctx| {
            self.active_view
                .rebuild(&prev.active_view, active_state, ctx, element)
        })
    }

    fn teardown(
        &self,
        (active_state, alongside_state): &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<'_, Self::Element>,
    ) {
        ctx.with_id(ViewId::new(0), |ctx| {
            self.alongside_view.teardown(alongside_state, ctx, ())
        });
        ctx.with_id(ViewId::new(1), |ctx| {
            self.active_view.teardown(active_state, ctx, element)
        });
    }

    fn message(
        &self,
        (active_state, alongside_state): &mut Self::ViewState,
        id_path: &[crate::ViewId],
        message: crate::DynMessage,
        app_state: &mut State,
    ) -> crate::MessageResult<Action> {
        let (first, id_path) = id_path
            .split_first()
            .expect("Id path has elements for Fork");
        match first.routing_id() {
            0 => self
                .active_view
                .message(active_state, id_path, message, app_state),
            1 => self
                .alongside_view
                .message(alongside_state, id_path, message, app_state),
            _ => unreachable!(),
        }
    }
}
