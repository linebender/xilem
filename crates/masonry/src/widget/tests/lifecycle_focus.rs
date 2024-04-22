// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

#![allow(unused)]

use std::cell::Cell;
use std::rc::Rc;

use smallvec::smallvec;

use crate::testing::{widget_ids, ModularWidget, ReplaceChild, TestHarness, TestWidgetExt as _};
use crate::widget::Flex;
use crate::*;

#[cfg(FALSE)]
const REQUEST_FOCUS: Selector<()> = Selector::new("masonry-test.request-focus");

struct FocusTaker;

#[cfg(FALSE)]
impl FocusTaker {
    fn new() -> impl Widget {
        Self::track(Default::default())
    }

    fn track(focused: Rc<Cell<bool>>) -> impl Widget {
        ModularWidget::new(focused)
            .event_fn(|_is_focused, ctx, event| {
                if let Event::Command(cmd) = event {
                    if cmd.is(REQUEST_FOCUS) {
                        ctx.request_focus();
                    }
                }
            })
            .status_change_fn(|is_focused, _ctx, event| {
                if let StatusChange::FocusChanged(focus) = event {
                    is_focused.set(*focus);
                }
            })
            .lifecycle_fn(|_is_focused, ctx, event| {
                if let LifeCycle::BuildFocusChain = event {
                    ctx.register_for_focus();
                }
            })
    }
}

#[cfg(FALSE)]
/// Check that a focus chain is correctly built initially..
#[test]
fn build_focus_chain() {
    let [id_1, id_2, id_3, id_4] = widget_ids();

    let widget = Flex::column()
        .with_child_id(FocusTaker::new(), id_1)
        .with_child_id(FocusTaker::new(), id_2)
        .with_child_id(FocusTaker::new(), id_3)
        .with_child_id(FocusTaker::new(), id_4);

    let harness = TestHarness::create(widget);

    // verify that we start out with four widgets registered for focus
    assert_eq!(harness.window().focus_chain(), &[id_1, id_2, id_3, id_4]);
}

#[cfg(FALSE)]
/// Check that focus changes trigger on_status_change
#[test]
fn focus_status_change() {
    let [id_1, id_2] = widget_ids();

    // we use these so that we can check that on_status_check was called
    let left_focus: Rc<Cell<bool>> = Default::default();
    let right_focus: Rc<Cell<bool>> = Default::default();
    assert_eq!(left_focus.get(), false);
    assert_eq!(right_focus.get(), false);

    let widget = Flex::row()
        .with_child_id(FocusTaker::track(left_focus.clone()), id_1)
        .with_child_id(FocusTaker::track(right_focus.clone()), id_2);

    let mut harness = TestHarness::create(widget);

    // nobody should have focus
    assert_eq!(left_focus.get(), false);
    assert_eq!(right_focus.get(), false);

    harness.submit_command(REQUEST_FOCUS.to(id_1));
    // check that left widget got "on_status_change" event.
    assert_eq!(left_focus.get(), true);
    assert_eq!(right_focus.get(), false);

    harness.submit_command(REQUEST_FOCUS.to(id_2));
    // check that left and right widget got "on_status_change" event.
    assert_eq!(left_focus.get(), false);
    assert_eq!(right_focus.get(), true);
}

#[cfg(FALSE)]
/// test that the last widget to request focus during an event gets it.
#[test]
fn take_focus() {
    let [id_1, id_2, id_3, id_4] = widget_ids();

    let widget = Flex::row()
        .with_child_id(FocusTaker::new(), id_1)
        .with_child_id(FocusTaker::new(), id_2)
        .with_child_id(FocusTaker::new(), id_3)
        .with_child_id(FocusTaker::new(), id_4);

    let mut harness = TestHarness::create(widget);

    // nobody should have focus
    assert_eq!(harness.window().focus, None);

    // this is sent to all widgets; the last widget to request focus should get it
    harness.submit_command(REQUEST_FOCUS);
    assert_eq!(harness.window().focus, Some(id_4));

    // this is sent to all widgets; the last widget to request focus should still get it
    harness.submit_command(REQUEST_FOCUS);
    assert_eq!(harness.window().focus, Some(id_4));
}

#[cfg(FALSE)]
#[test]
fn focus_updated_by_children_change() {
    let [id_1, id_2, id_3, id_4, id_5, id_6] = widget_ids();

    // this widget starts with a single child, and will replace them with a split
    // when we send it a command.
    let replacer = ReplaceChild::new(FocusTaker::new().with_id(id_4), move || {
        Flex::row()
            .with_child_id(FocusTaker::new(), id_5)
            .with_child_id(FocusTaker::new(), id_6)
    });

    let widget = Flex::row()
        .with_child_id(FocusTaker::new(), id_1)
        .with_child_id(FocusTaker::new(), id_2)
        .with_child_id(FocusTaker::new(), id_3)
        .with_child(replacer);

    let mut harness = TestHarness::create(widget);

    // verify that we start out with four widgets registered for focus
    assert_eq!(harness.window().focus_chain(), &[id_1, id_2, id_3, id_4]);

    // tell the replacer widget to swap its children
    harness.submit_command(REPLACE_CHILD);

    // verify that the two new children are registered for focus.
    assert_eq!(
        harness.window().focus_chain(),
        &[id_1, id_2, id_3, id_5, id_6]
    );
}

#[cfg(FALSE)]
#[test]
fn resign_focus_on_disable() {
    const CHANGE_DISABLED: Selector<bool> = Selector::new("masonry-test.change-disabled");

    fn make_container_widget(id: WidgetId, child: impl Widget) -> impl Widget {
        ModularWidget::new(WidgetPod::new_with_id(child, id))
            .event_fn(|child, ctx, event| {
                if let Event::Command(cmd) = event {
                    if let Some(disabled) = cmd.try_get(CHANGE_DISABLED) {
                        ctx.set_disabled(*disabled);
                        ctx.set_handled();
                        // TODO
                        //return;
                    }
                }
                child.on_event(ctx, event);
            })
            .lifecycle_fn(|child, ctx, event| {
                child.lifecycle(ctx, event);
            })
            .layout_fn(|child, ctx, bc| {
                let layout = child.layout(ctx, bc);
                ctx.place_child(child, Point::ZERO);
                layout
            })
            .children_fn(|child| smallvec![child.as_dyn()])
    }

    let [group_0, group_1, sub_group, focus_1, focus_2] = widget_ids();

    let root = Flex::row()
        .with_child_id(
            make_container_widget(sub_group, make_container_widget(focus_1, FocusTaker::new())),
            group_0,
        )
        .with_child_id(make_container_widget(focus_2, FocusTaker::new()), group_1);

    let mut harness = TestHarness::create(root);

    // Initial state -> Full focus chain, no focused widget
    assert_eq!(harness.window().focus_chain(), &[focus_1, focus_2]);
    assert_eq!(harness.window().focus, None);

    // Request focus to 2 -> Full focus chain, 2 is focused
    harness.submit_command(REQUEST_FOCUS.to(focus_2));
    assert_eq!(harness.window().focus_chain(), &[focus_1, focus_2]);
    assert_eq!(harness.window().focus, Some(focus_2));

    // Disable group 0 -> Remove 1 from focus chain, 2 is still focused
    harness.submit_command(CHANGE_DISABLED.with(true).to(group_0));
    assert_eq!(harness.window().focus_chain(), &[focus_2]);
    assert_eq!(harness.window().focus, Some(focus_2));

    // TODO - check that focus doesn't change if requested from a disabled widget
    //harness.submit_command(REQUEST_FOCUS.to(focus_1));

    // Disable group 1 -> Remove 2 from focus chain, no focused widget
    harness.submit_command(CHANGE_DISABLED.with(true).to(group_1));
    assert_eq!(harness.window().focus_chain(), &[]);
    assert_eq!(harness.window().focus, None);

    // Enable group 0 -> Add 1 to focus chain, no focused widget
    harness.submit_command(CHANGE_DISABLED.with(false).to(group_0));
    assert_eq!(harness.window().focus_chain(), &[focus_1]);
    assert_eq!(harness.window().focus, None);

    // Request focus to 1 -> 1 is focused
    harness.submit_command(REQUEST_FOCUS.to(focus_1));
    assert_eq!(harness.window().focus_chain(), &[focus_1]);
    assert_eq!(harness.window().focus, Some(focus_1));

    // Enable group 1 -> Full focus chain, 1 is still focused
    harness.submit_command(CHANGE_DISABLED.with(false).to(group_1));
    assert_eq!(harness.window().focus_chain(), &[focus_1, focus_2]);
    assert_eq!(harness.window().focus, Some(focus_1));

    // Disable group 0 -> Remove 1 from focus chain, no focused widget
    harness.submit_command(CHANGE_DISABLED.with(true).to(group_0));
    assert_eq!(harness.window().focus_chain(), &[focus_2]);
    assert_eq!(harness.window().focus, None);
}
