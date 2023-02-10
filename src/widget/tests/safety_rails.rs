// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

use smallvec::smallvec;

use crate::testing::{ModularWidget, TestHarness};
use crate::widget::Flex;
use crate::*;

fn make_parent_widget<W: Widget>(child: W) -> ModularWidget<WidgetPod<W>> {
    let child = WidgetPod::new(child);
    ModularWidget::new(child)
        .event_fn(move |child, ctx, event, env| {
            child.on_event(ctx, event, env);
        })
        .lifecycle_fn(move |child, ctx, event, env| child.lifecycle(ctx, event, env))
        .layout_fn(move |child, ctx, bc, env| {
            let size = child.layout(ctx, bc, env);
            ctx.place_child(child, Point::ZERO, env);
            size
        })
        .paint_fn(move |child, ctx, env| {
            child.paint(ctx, env);
        })
        .children_fn(|child| smallvec![child.as_dyn()])
}

// TODO - recurse command?

#[should_panic(expected = "not visited in method event")]
#[test]
fn check_forget_to_recurse_event() {
    let widget = make_parent_widget(Flex::row()).event_fn(|_child, _ctx, _event, _| {
        // We forget to call child.on_event();
    });

    let mut harness = TestHarness::create(widget);
    harness.mouse_move(Point::ZERO);
}

#[should_panic(expected = "not visited in method lifecycle")]
#[test]
fn check_forget_to_recurse_lifecycle() {
    let widget = make_parent_widget(Flex::row()).lifecycle_fn(|_child, _ctx, _event, _| {
        // We forget to call child.lifecycle();
    });

    let _harness = TestHarness::create(widget);
}

#[should_panic(expected = "before receiving WidgetAdded.")]
#[test]
fn check_forget_to_recurse_widget_added() {
    let widget = make_parent_widget(Flex::row()).lifecycle_fn(|child, ctx, event, env| {
        if let LifeCycle::WidgetAdded = event {
            // We forget to call child.lifecycle();
            ctx.skip_child(child);
        } else {
            child.lifecycle(ctx, event, env);
        }
    });

    let _harness = TestHarness::create(widget);
}

#[should_panic(expected = "not visited in method layout")]
#[test]
fn check_forget_to_recurse_layout() {
    let widget = make_parent_widget(Flex::row()).layout_fn(|_child, _ctx, _, _| {
        // We forget to call child.layout();
        Size::ZERO
    });

    let _harness = TestHarness::create(widget);
}

#[should_panic(expected = "missing call to place_child method for child widget")]
#[test]
fn check_forget_to_call_place_child() {
    let widget = make_parent_widget(Flex::row()).layout_fn(|child, ctx, bc, env| {
        // We call child.layout(), but forget place_child
        child.layout(ctx, bc, env)
    });

    let _harness = TestHarness::create(widget);
}

#[should_panic(expected = "not visited in method paint")]
#[test]
fn check_forget_to_recurse_paint() {
    let widget = make_parent_widget(Flex::row()).paint_fn(|_child, _ctx, _| {
        // We forget to call child.paint();
    });

    let mut harness = TestHarness::create(widget);
    harness.render();
}

// ---

// TODO - allow non-recurse in some cases

#[cfg(FALSE)]
#[test]
fn allow_non_recurse_event_handled() {
    let widget = make_parent_widget(Flex::row()).event_fn(|_child, ctx, event, _| {
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
        .event_fn(|child, ctx, event, env| {
            if !matches!(event, Event::MouseMove(_)) {
                child.on_event(ctx, event, env);
            }
        })
        .layout_fn(|child, ctx, bc, env| {
            let _size = child.layout(ctx, bc, env);
            ctx.place_child(child, Point::ZERO, env);
            Size::new(6000.0, 6000.0)
        });

    let mut harness = TestHarness::create(widget);
    harness.mouse_move(Point::new(5000.0, 5000.0));
}

#[cfg(FALSE)]
#[test]
fn allow_non_recurse_oob_paint() {
    let widget = make_parent_widget(Flex::row())
        .paint_fn(|child, ctx, _| {
            // We forget to call child.paint();
        })
        .layout_fn(|child, ctx, bc, env| {
            let _size = child.layout(ctx, bc, env);
            ctx.place_child(child, Point::new(500.0, 500.0), env);
            Size::new(600.0, 600.0)
        });

    let mut harness = TestHarness::create_with_size(widget, Size::new(400.0, 400.0));
    harness.render();
}

#[test]
fn allow_non_recurse_cursor_stashed() {
    let widget = make_parent_widget(Flex::row())
        .event_fn(|child, ctx, event, env| {
            ctx.set_stashed(child, true);

            if !matches!(event, Event::MouseMove(_)) {
                child.on_event(ctx, event, env);
            }
        })
        .layout_fn(|_child, _ctx, _bc, _env| Size::ZERO);

    let mut harness = TestHarness::create(widget);
    harness.mouse_move(Point::new(5000.0, 5000.0));
}

#[test]
fn allow_non_recurse_stashed_paint() {
    let widget = make_parent_widget(Flex::row())
        .event_fn(|child, ctx, _event, _| {
            ctx.set_stashed(child, true);
        })
        .layout_fn(|_child, _ctx, _bc, _env| Size::ZERO)
        .paint_fn(|_child, _ctx, _env| {
            // We don't call child.paint();
        });

    let mut harness = TestHarness::create_with_size(widget, Size::new(400.0, 400.0));
    harness.render();
}

// ---

#[should_panic(expected = "children changed")]
#[test]
fn check_forget_children_changed() {
    pub const ADD_CHILD: Selector = Selector::new("masonry-test.add-child");

    let child: Option<WidgetPod<Flex>> = None;
    let widget = ModularWidget::new(child)
        .event_fn(|child, ctx, event, env| {
            if let Some(child) = child {
                child.on_event(ctx, event, env);
            }
            if let Event::Command(command) = event {
                if command.is(ADD_CHILD) {
                    *child = Some(WidgetPod::new(Flex::row()));
                }
            }
        })
        .lifecycle_fn(|child, ctx, event, env| {
            if let Some(child) = child {
                child.lifecycle(ctx, event, env);
            }
        })
        .layout_fn(|child, ctx, bc, env| {
            if let Some(child) = child {
                let size = child.layout(ctx, bc, env);
                ctx.place_child(child, Point::ZERO, env);
                size
            } else {
                Size::ZERO
            }
        })
        .paint_fn(|child, ctx, env| {
            if let Some(child) = child {
                child.paint(ctx, env);
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
    let widget = make_parent_widget(Flex::row()).event_fn(|child, ctx, event, env| {
        child.on_event(ctx, event, env);
    });

    let mut harness = TestHarness::create(widget);
    harness.mouse_move(Point::ZERO);
}

#[cfg(FALSE)]
#[should_panic]
#[test]
fn check_recurse_lifecycle_twice() {
    let widget = make_parent_widget(Flex::row()).lifecycle_fn(|child, ctx, event, env| {
        child.lifecycle(ctx, event, env);
    });

    let _harness = TestHarness::create(widget);
}

#[cfg(FALSE)]
#[should_panic]
#[test]
fn check_recurse_layout_twice() {
    let widget = make_parent_widget(Flex::row()).layout_fn(|child, ctx, bc, env| {
        let size = child.layout(ctx, bc, env);
        ctx.place_child(child, Point::ZERO, env);
        size
    });

    let _harness = TestHarness::create(widget);
}

#[cfg(FALSE)]
#[should_panic]
#[test]
fn check_recurse_paint_twice() {
    let widget = make_parent_widget(Flex::row()).paint_fn(|child, ctx, env| {
        child.paint(ctx, env);
    });

    let mut harness = TestHarness::create(widget);
    harness.render();
}

// ---

#[should_panic(expected = "trying to compute layout of stashed widget")]
#[test]
fn check_layout_stashed() {
    let widget = make_parent_widget(Flex::row())
        .event_fn(|child, ctx, _event, _| {
            ctx.set_stashed(child, true);
        })
        .layout_fn(|child, ctx, bc, env| {
            let size = child.layout(ctx, bc, env);
            ctx.place_child(child, Point::ZERO, env);
            size
        });

    let mut harness = TestHarness::create(widget);
    harness.mouse_move(Point::ZERO);
}

#[should_panic(expected = "trying to paint stashed widget")]
#[test]
fn check_paint_stashed() {
    let widget = make_parent_widget(Flex::row())
        .event_fn(|child, ctx, _event, _| {
            ctx.set_stashed(child, true);
        })
        .layout_fn(|_child, _ctx, _bc, _env| Size::ZERO)
        .paint_fn(|child, ctx, env| {
            child.paint(ctx, env);
        });

    let mut harness = TestHarness::create(widget);
    harness.mouse_move(Point::ZERO);
    harness.render();
}

// ---

// TODO - For now, paint_rect is automaticall computed, so there's no way this test fails.
#[cfg(FALSE)]
#[should_panic(expected = "doesn't contain paint_rect")]
#[test]
fn check_paint_rect_includes_children() {
    use crate::widget::Label;
    let widget = make_parent_widget(Label::new("Hello world")).layout_fn(|child, ctx, bc, env| {
        let _size = child.layout(ctx, bc, env);
        ctx.place_child(child, Point::ZERO, env);
        Size::ZERO
    });

    let mut harness = TestHarness::create(widget);
    harness.mouse_move(Point::ZERO);
    harness.render();
}
