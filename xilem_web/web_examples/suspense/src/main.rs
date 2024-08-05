// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use gloo_timers::future::TimeoutFuture;
use xilem_web::{
    concurrent::await_once,
    core::{fork, one_of::Either},
    document_body,
    elements::html::{h1, p},
    interfaces::Element,
    App,
};

fn app_logic(view_has_resolved: &mut bool) -> impl Element<bool> {
    let view = if !*view_has_resolved {
        Either::A(p("This will be replaced soon"))
    } else {
        Either::B(h1("The time has come for fanciness"))
    };
    fork(
        // note how the `Class` view is applied to either the p or the h1 element
        view.class(view_has_resolved.then_some("blink")),
        await_once(
            |_| TimeoutFuture::new(5000),
            |view_has_resolved: &mut bool, _| {
                *view_has_resolved = true;
            },
        ),
    )
}

pub fn main() {
    console_error_panic_hook::set_once();
    App::new(document_body(), false, app_logic).run();
}
