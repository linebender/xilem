// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::core::{MessageCtx, MessageResult, Mut, View, ViewMarker};
use crate::{Pod, ViewCtx, WidgetView};

use masonry::widgets;

/// An element which holds radio buttons.
///
/// # Example
/// ```ignore
/// use xilem::view::{flex_row, radio_button, radio_group};
///
/// #[derive(Debug, PartialEq, Eq)]
/// enum Fruit {
///     Banana,
///     Apple,
///     Lime,
/// }
///
/// struct State {
///     fruit: Fruit,
/// }
///
/// // ...
///
/// radio_group(flex_row((
///    radio_button("Banana", app_state.fruit == Fruit::Banana, |app_state: &mut State| {
///         app_state.fruit = Fruit::Banana;
///    }),
///    radio_button("Apple", app_state.fruit == Fruit::Apple, |app_state: &mut State| {
///         app_state.fruit = Fruit::Apple;
///    }),
///    radio_button("Lime", app_state.fruit == Fruit::Lime, |app_state: &mut State| {
///         app_state.fruit = Fruit::Lime;
///    }),
/// )))
/// ```
pub fn radio_group<State, Action, V>(child: V) -> RadioGroup<V>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    RadioGroup { child }
}

/// The [`View`] created by [`radio_group`] from a bool value and a callback.
///
/// See `radio_group` documentation for more context.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct RadioGroup<V> {
    child: V,
}

impl<V> ViewMarker for RadioGroup<V> {}
impl<State, Action, V> View<State, Action, ViewCtx> for RadioGroup<V>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    type Element = Pod<widgets::RadioGroup>;
    type ViewState = V::ViewState;

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let (child, child_state) = self.child.build(ctx, app_state);
        let widget = widgets::RadioGroup::new(child.new_widget);

        (ctx.create_pod(widget), child_state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        let mut child = widgets::RadioGroup::child_mut(&mut element);
        self.child
            .rebuild(&prev.child, view_state, ctx, child.downcast(), app_state);
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        let mut child = widgets::RadioGroup::child_mut(&mut element);
        self.child.teardown(view_state, ctx, child.downcast());
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        let mut child = widgets::RadioGroup::child_mut(&mut element);
        self.child
            .message(view_state, message, child.downcast(), app_state)
    }
}
