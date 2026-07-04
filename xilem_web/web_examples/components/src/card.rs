// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use xilem_web::{DomFragment, core::map_action, elements::html, interfaces::Element};

// The `CardAction` is a composition of
// the variations of the card component
// (only `Toggle` here so far)
// and the generic actions `A` of the child.
pub(crate) enum Action<A> {
    Toggle,
    Child(A),
}

impl<A> xilem_web::Action for Action<A> {}

// This view is generic about its child and its actions.
// It also has no state of its own,
// but only communicates with the parents via the [`CardAction`].
pub(crate) fn view<State, Child, ChildAction>(
    title: &'static str,
    collapsed: bool,
    content: Child,
) -> impl Element<State, Action<ChildAction>>
where
    Child: DomFragment<State, ChildAction>,
    State: 'static,
    ChildAction: 'static,
{
    let content = map_action(
        html::div(content)
            .class("content")
            .class(collapsed.then_some("hidden")),
        |_, msg| Action::Child(msg),
    );

    html::div((
        html::h3(title)
            .class("title")
            .on_click(|_, _| Action::Toggle),
        content,
    ))
    .class("card")
}
