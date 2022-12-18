// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

#![allow(unused_imports)]

// TODO
// Honestly, these tests are a pain in the ass to visualize
// I wouldn't mind a better way to write/read them.

use super::*;
#[cfg(FALSE)]
#[test]
fn aspect_ratio_tight_constraints() {
    let id = WidgetId::next();
    let (width, height) = (400., 400.);
    let aspect = AspectRatioBox::<()>::new(Label::new("hello!"), 1.0)
        .with_id(id)
        .fix_width(width)
        .fix_height(height)
        .center();

    let (window_width, window_height) = (600., 600.);

    Harness::create_simple((), aspect, |harness| {
        harness.set_initial_size(Size::new(window_width, window_height));
        harness.send_initial_events();
        harness.just_layout();
        let state = harness.get_state(id);
        assert_eq!(state.layout_rect().size(), Size::new(width, height));
    });
}

#[cfg(FALSE)]
#[test]
fn aspect_ratio_infinite_constraints() {
    let id = WidgetId::next();
    let (width, height) = (100., 100.);
    let label = Label::new("hello!").fix_width(width).height(height);
    let aspect = AspectRatioBox::<()>::new(label, 1.0)
        .with_id(id)
        .scroll()
        .center();

    let (window_width, window_height) = (600., 600.);

    Harness::create_simple((), aspect, |harness| {
        harness.set_initial_size(Size::new(window_width, window_height));
        harness.send_initial_events();
        harness.just_layout();
        let state = harness.get_state(id);
        assert_eq!(state.layout_rect().size(), Size::new(width, height));
    });
}

#[cfg(FALSE)]
#[test]
fn aspect_ratio_tight_constraint_on_width() {
    let id = WidgetId::next();
    let label = Label::new("hello!");
    let aspect = AspectRatioBox::<()>::new(label, 2.0)
        .with_id(id)
        .fix_width(300.)
        .center();

    let (window_width, window_height) = (600., 50.);

    Harness::create_simple((), aspect, |harness| {
        harness.set_initial_size(Size::new(window_width, window_height));
        harness.send_initial_events();
        harness.just_layout();
        let state = harness.get_state(id);
        assert_eq!(state.layout_rect().size(), Size::new(300., 50.));
    });
}

#[cfg(FALSE)]
#[test]
fn aspect_ratio() {
    let id = WidgetId::next();
    let label = Label::new("hello!");
    let aspect = AspectRatioBox::<()>::new(label, 2.0)
        .with_id(id)
        .center()
        .center();

    let (window_width, window_height) = (1000., 1000.);

    Harness::create_simple((), aspect, |harness| {
        harness.set_initial_size(Size::new(window_width, window_height));
        harness.send_initial_events();
        harness.just_layout();
        let state = harness.get_state(id);
        assert_eq!(state.layout_rect().size(), Size::new(1000., 500.));
    });
}
