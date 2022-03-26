use crate::testing::{Harness, ModularWidget};
use crate::widget::Flex;
use crate::*;
use smallvec::smallvec;

fn get_parent_widget<W: Widget>(child: W) -> ModularWidget<WidgetPod<W>> {
    let child = WidgetPod::new(child);
    ModularWidget::new(child)
        .event_fn(move |child, ctx, event, env| {
            child.on_event(ctx, event, env);
        })
        .lifecycle_fn(move |child, ctx, event, env| child.lifecycle(ctx, event, env))
        .layout_fn(move |child, ctx, bc, env| {
            let size = child.layout(ctx, bc, env);
            child.set_origin(ctx, env, Point::ZERO);
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
    let widget = get_parent_widget(Flex::row()).event_fn(move |_child, _ctx, _event, _| {
        // We forget to call child.on_event();
    });

    let mut harness = Harness::create(widget);
    harness.mouse_move(Point::ZERO);
}

#[should_panic(expected = "not visited in method lifecycle")]
#[test]
fn check_forget_to_recurse_lifecycle() {
    let widget = get_parent_widget(Flex::row()).lifecycle_fn(move |_child, _ctx, _event, _| {
        // We forget to call child.lifecycle();
    });

    let _harness = Harness::create(widget);
}

#[should_panic(expected = "before receiving WidgetAdded.")]
#[test]
fn check_forget_to_recurse_widget_added() {
    let widget = get_parent_widget(Flex::row()).lifecycle_fn(move |child, ctx, event, env| {
        if let LifeCycle::WidgetAdded = event {
            // We forget to call child.lifecycle();
            ctx.skip_child(child);
        } else {
            child.lifecycle(ctx, event, env);
        }
    });

    let _harness = Harness::create(widget);
}

#[should_panic(expected = "not visited in method layout")]
#[test]
fn check_forget_to_recurse_layout() {
    let widget = get_parent_widget(Flex::row()).layout_fn(move |_child, _ctx, _, _| {
        // We forget to call child.layout();
        Size::ZERO
    });

    let _harness = Harness::create(widget);
}

#[should_panic(expected = "missing call to set_origin method for child widget")]
#[test]
fn check_forget_to_call_set_origin() {
    let widget = get_parent_widget(Flex::row()).layout_fn(move |child, ctx, bc, env| {
        // We call child.layout(), but forget set_origin
        child.layout(ctx, bc, env)
    });

    let _harness = Harness::create(widget);
}

#[should_panic(expected = "not visited in method paint")]
#[test]
fn check_forget_to_recurse_paint() {
    let widget = get_parent_widget(Flex::row()).paint_fn(move |_child, _ctx, _| {
        // We forget to call child.paint();
    });

    let mut harness = Harness::create(widget);
    harness.render();
}

// ---

// TODO - allow non-recurse in some cases

#[cfg(FALSE)]
#[test]
fn allow_non_recurse_event_handled() {
    let widget = get_parent_widget(Flex::row()).event_fn(move |_child, ctx, event, _| {
        // Event handled, we don't need to recurse
        ctx.set_handled();
    });

    let mut harness = Harness::create(widget);
    harness.mouse_move(Point::ZERO);
}

#[cfg(FALSE)]
#[test]
fn allow_non_recurse_cursor_oob() {
    let widget = get_parent_widget(Flex::row())
        .event_fn(move |child, ctx, event, env| {
            if !matches!(event, Event::MouseMove(_)) {
                child.on_event(ctx, event, env);
            }
        })
        .layout_fn(move |child, ctx, bc, env| {
            let _size = child.layout(ctx, bc, env);
            child.set_origin(ctx, env, Point::ZERO);
            Size::new(6000.0, 6000.0)
        });

    let mut harness = Harness::create(widget);
    harness.mouse_move(Point::new(5000.0, 5000.0));
}

#[cfg(FALSE)]
#[test]
fn allow_non_recurse_oob_paint() {
    let widget = get_parent_widget(Flex::row())
        .paint_fn(move |child, ctx, _| {
            // We forget to call child.paint();
        })
        .layout_fn(move |child, ctx, bc, env| {
            let _size = child.layout(ctx, bc, env);
            child.set_origin(ctx, env, Point::new(500.0, 500.0));
            Size::new(600.0, 600.0)
        });

    let mut harness = Harness::create_with_size(widget, Size::new(400.0, 400.0));
    harness.render();
}

// TODO - handle hidden items
// NOTE - All checks should use viewport

// ---

// TODO - expect better error message
#[should_panic]
#[test]
fn check_forget_children_changed() {
    pub const ADD_CHILD: Selector = Selector::new("druid-test.add-child");

    let child: Option<WidgetPod<Flex>> = None;
    let widget = ModularWidget::new(child)
        .event_fn(move |child, ctx, event, env| {
            if let Some(child) = child {
                child.on_event(ctx, event, env);
            }
            if let Event::Command(command) = event {
                if command.is(ADD_CHILD) {
                    *child = Some(WidgetPod::new(Flex::row()));
                }
            }
        })
        .lifecycle_fn(move |child, ctx, event, env| {
            if let Some(child) = child {
                child.lifecycle(ctx, event, env);
            }
        })
        .layout_fn(move |child, ctx, bc, env| {
            if let Some(child) = child {
                let size = child.layout(ctx, bc, env);
                child.set_origin(ctx, env, Point::ZERO);
                size
            } else {
                Size::ZERO
            }
        })
        .paint_fn(move |child, ctx, env| {
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

    let mut harness = Harness::create(widget);
    harness.submit_command(ADD_CHILD);
}
