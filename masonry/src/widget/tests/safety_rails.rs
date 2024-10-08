// Copyright 2022 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use smallvec::smallvec;

use crate::testing::{ModularWidget, TestHarness, TestWidgetExt};
use crate::widget::Flex;
use crate::{LifeCycle, Point, PointerButton, Size, Widget, WidgetId, WidgetPod};

fn make_parent_widget<W: Widget>(child: W) -> ModularWidget<WidgetPod<W>> {
    let child = WidgetPod::new(child);
    ModularWidget::new(child)
        .register_children_fn(move |child, ctx| {
            ctx.register_child(child);
        })
        .layout_fn(move |child, ctx, bc| {
            let size = ctx.run_layout(child, bc);
            ctx.place_child(child, Point::ZERO);
            size
        })
        .children_fn(|child| smallvec![child.id()])
}

#[cfg(FALSE)]
#[should_panic(expected = "not visited in method on_text_event")]
#[test]
fn check_forget_to_recurse_text_event() {
    let widget = make_parent_widget(Flex::row()).text_event_fn(|_child, _ctx, _event| {
        // We forget to call child.on_text_event();
    });

    let mut harness = TestHarness::create(widget);
    harness.mouse_move(Point::ZERO);
}

#[cfg(FALSE)]
#[should_panic(expected = "not added in method lifecycle")]
#[test]
#[cfg_attr(
    not(debug_assertions),
    ignore = "This test doesn't work without debug assertions (i.e. in release mode). See https://github.com/linebender/xilem/issues/477"
)]
fn check_forget_to_recurse_lifecycle() {
    let widget = make_parent_widget(Flex::row()).lifecycle_fn(|_child, _ctx, _event| {
        // We forget to call child.lifecycle();
    });

    let _harness = TestHarness::create(widget);
}

#[cfg(FALSE)]
#[should_panic(expected = "before receiving WidgetAdded.")]
#[test]
fn check_forget_to_recurse_widget_added() {
    let widget = make_parent_widget(Flex::row()).lifecycle_fn(|child, ctx, event| {
        if let LifeCycle::WidgetAdded = event {
            // We forget to call child.lifecycle();
            ctx.skip_child(child);
        } else {
            child.lifecycle(ctx, event);
        }
    });

    let _harness = TestHarness::create(widget);
}

#[should_panic(expected = "did not call RegisterCtx::register_child()")]
#[test]
#[cfg_attr(
    not(debug_assertions),
    ignore = "This test doesn't work without debug assertions (i.e. in release mode). See https://github.com/linebender/xilem/issues/477"
)]
fn check_forget_register_child() {
    let widget = make_parent_widget(Flex::row()).register_children_fn(|_child, _ctx| {
        // We forget to call ctx.register_child();
    });

    let _harness = TestHarness::create(widget);
}

#[should_panic(expected = "in the list returned by children_ids")]
#[test]
#[cfg_attr(
    not(debug_assertions),
    ignore = "This test doesn't work without debug assertions (i.e. in release mode). See https://github.com/linebender/xilem/issues/477"
)]
fn check_register_invalid_child() {
    let widget = make_parent_widget(Flex::row()).register_children_fn(|child, ctx| {
        ctx.register_child(child);
        ctx.register_child(&mut WidgetPod::new(Flex::row()));
    });

    let _harness = TestHarness::create(widget);
}

#[should_panic(expected = "event does not allow pointer capture")]
#[test]
#[cfg_attr(
    not(debug_assertions),
    ignore = "This test doesn't work without debug assertions (i.e. in release mode). See https://github.com/linebender/xilem/issues/477"
)]
fn check_pointer_capture_outside_pointer_down() {
    let widget = ModularWidget::new(()).pointer_event_fn(|_, ctx, _event| {
        ctx.capture_pointer();
    });

    let mut harness = TestHarness::create(widget);
    harness.mouse_move((10.0, 10.0));
    harness.mouse_button_release(PointerButton::Primary);
}

#[should_panic(expected = "event does not allow pointer capture")]
#[test]
#[cfg_attr(
    not(debug_assertions),
    ignore = "This test doesn't work without debug assertions (i.e. in release mode). See https://github.com/linebender/xilem/issues/477"
)]
fn check_pointer_capture_text_event() {
    let id = WidgetId::next();
    let widget = ModularWidget::new(())
        .lifecycle_fn(|_, ctx, event| {
            if let LifeCycle::WidgetAdded = event {
                ctx.register_for_focus();
            }
        })
        .text_event_fn(|_, ctx, _event| {
            ctx.capture_pointer();
        })
        .with_id(id);

    let mut harness = TestHarness::create(widget);
    harness.focus_on(Some(id));
    harness.keyboard_type_chars("a");
}

#[should_panic(expected = "not visited in method layout")]
#[test]
#[cfg_attr(
    not(debug_assertions),
    ignore = "This test doesn't work without debug assertions (i.e. in release mode). See https://github.com/linebender/xilem/issues/477"
)]
fn check_forget_to_recurse_layout() {
    let widget = make_parent_widget(Flex::row()).layout_fn(|_child, _ctx, _| {
        // We forget to call ctx.run_layout();
        Size::ZERO
    });

    let _harness = TestHarness::create(widget);
}

#[should_panic(expected = "missing call to place_child method for child widget")]
#[test]
#[cfg_attr(
    not(debug_assertions),
    ignore = "This test doesn't work without debug assertions (i.e. in release mode). See https://github.com/linebender/xilem/issues/477"
)]
fn check_forget_to_call_place_child() {
    let widget = make_parent_widget(Flex::row()).layout_fn(|child, ctx, bc| {
        // We call ctx.run_layout(), but forget place_child
        ctx.run_layout(child, bc)
    });

    let _harness = TestHarness::create(widget);
}

// ---

// TODO - allow non-recurse in some cases

#[cfg(FALSE)]
#[test]
fn allow_non_recurse_event_handled() {
    let widget = make_parent_widget(Flex::row())
        .pointer_event_fn(|_child, ctx, _event| {
            // Event handled, we don't need to recurse
            ctx.set_handled();
        })
        .text_event_fn(|_child, ctx, _event| {
            // Event handled, we don't need to recurse
            ctx.set_handled();
        });

    let mut harness = TestHarness::create(widget);
    harness.mouse_move(Point::ZERO);
}

#[cfg(FALSE)]
#[test]
fn allow_non_recurse_cursor_oob() {
    let widget = make_parent_widget(Flex::row())
        .pointer_event_fn(|child, ctx, event| {
            if !matches!(event, PointerEvent::PointerMove(_)) {
                child.on_pointer_event(ctx, event);
            }
        })
        .layout_fn(|child, ctx, bc| {
            let _size = ctx.run_layout(child, bc);
            ctx.place_child(child, Point::ZERO);
            Size::new(6000.0, 6000.0)
        });

    let mut harness = TestHarness::create(widget);
    harness.mouse_move(Point::new(5000.0, 5000.0));
}

#[cfg(FALSE)]
#[test]
fn allow_non_recurse_oob_paint() {
    let widget = make_parent_widget(Flex::row())
        .paint_fn(|_child, _ctx, _| {
            // We forget to call child.paint();
        })
        .layout_fn(|child, ctx, bc| {
            let _size = ctx.run_layout(child, bc);
            ctx.place_child(child, Point::new(500.0, 500.0));
            Size::new(600.0, 600.0)
        });

    let mut harness = TestHarness::create_with_size(widget, Size::new(400.0, 400.0));
    harness.render();
}

// ---

#[cfg(FALSE)]
#[should_panic(expected = "children changed")]
#[test]
fn check_forget_children_changed() {
    pub const ADD_CHILD: Selector = Selector::new("masonry-test.add-child");

    let child: Option<WidgetPod<Flex>> = None;
    let widget = ModularWidget::new(child)
        .event_fn(|child, ctx, event| {
            if let Some(child) = child {
                child.on_event(ctx, event);
            }
            if let Event::Command(command) = event {
                if command.is(ADD_CHILD) {
                    *child = Some(WidgetPod::new(Flex::row()));
                }
            }
        })
        .lifecycle_fn(|child, ctx, event| {
            if let Some(child) = child {
                child.lifecycle(ctx, event);
            }
        })
        .layout_fn(|child, ctx, bc| {
            if let Some(child) = child {
                let size = ctx.run_layout(child, bc);
                ctx.place_child(child, Point::ZERO);
                size
            } else {
                Size::ZERO
            }
        })
        .paint_fn(|child, ctx, scene| {
            if let Some(child) = child {
                child.paint(ctx, scene);
            }
        })
        .children_fn(|child| {
            if let Some(child) = child {
                smallvec![child.as_dyn()]
            } else {
                smallvec![]
            }
        });

    let mut harness = TestHarness::create(widget);
    harness.submit_command(ADD_CHILD);
}

// ---

#[cfg(FALSE)]
#[should_panic]
#[test]
fn check_recurse_event_twice() {
    let widget = make_parent_widget(Flex::row()).pointer_event_fn(|child, ctx, event| {
        child.on_pointer_event(ctx, event);
        child.on_pointer_event(ctx, event);
    });

    let mut harness = TestHarness::create(widget);
    harness.mouse_move(Point::ZERO);
}

#[cfg(FALSE)]
#[should_panic]
#[test]
fn check_recurse_lifecycle_twice() {
    let widget = make_parent_widget(Flex::row()).lifecycle_fn(|child, ctx, event| {
        child.lifecycle(ctx, event);
        child.lifecycle(ctx, event);
    });

    let _harness = TestHarness::create(widget);
}

#[cfg(FALSE)]
#[should_panic]
#[test]
fn check_recurse_layout_twice() {
    let widget = make_parent_widget(Flex::row()).layout_fn(|child, ctx, bc| {
        let size = ctx.run_layout(child, bc);
        let _ = ctx.run_layout(child, bc);
        ctx.place_child(child, Point::ZERO);
        size
    });

    let _harness = TestHarness::create(widget);
}

#[cfg(FALSE)]
#[should_panic]
#[test]
fn check_recurse_paint_twice() {
    let widget = make_parent_widget(Flex::row()).paint_fn(|child, ctx, scene| {
        child.paint(ctx, scene);
        child.paint(ctx, scene);
    });

    let mut harness = TestHarness::create(widget);
    harness.render();
}

// ---

#[should_panic(expected = "trying to compute layout of stashed widget")]
#[test]
fn check_layout_stashed() {
    let widget = make_parent_widget(Flex::row())
        .lifecycle_fn(|child, ctx, event| {
            if matches!(event, LifeCycle::WidgetAdded) {
                ctx.set_stashed(child, true);
            }
        })
        .layout_fn(|child, ctx, bc| {
            let size = ctx.run_layout(child, bc);
            ctx.place_child(child, Point::ZERO);
            size
        });

    let mut harness = TestHarness::create(widget);
    harness.mouse_move(Point::ZERO);
}

// ---

// TODO - For now, paint_rect is automatically computed, so there's no way this test fails.
#[cfg(FALSE)]
#[should_panic(expected = "doesn't contain paint_rect")]
#[test]
fn check_paint_rect_includes_children() {
    use crate::widget::Label;
    let widget = make_parent_widget(Label::new("Hello world")).layout_fn(|child, ctx, bc| {
        let _size = ctx.run_layout(child, bc);
        ctx.place_child(child, Point::ZERO);
        Size::ZERO
    });

    let mut harness = TestHarness::create(widget);
    harness.mouse_move(Point::ZERO);
    harness.render();
}
