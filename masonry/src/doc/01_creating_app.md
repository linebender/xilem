**TODO - Add screenshots - see [#501](https://github.com/linebender/xilem/issues/501)**

This tutorial explains how to build a simple Masonry app, step by step.
Though it isn't representative of how we expect Masonry to be used, it does cover the basic architecture.

The app we'll create is identical to the to-do-list example shown in the README.

## The Widget tree

Let's start with the `main()` function.

```rust
fn main() {
    const VERTICAL_WIDGET_SPACING: f64 = 20.0;

    use masonry::widget::{Button, Flex, Portal, RootWidget, Textbox};

    let main_widget = Portal::new(
        Flex::column()
            .with_child(
                Flex::row()
                    .with_flex_child(Textbox::new(""), 1.0)
                    .with_child(Button::new("Add task")),
            )
            .with_spacer(VERTICAL_WIDGET_SPACING),
    );
    let main_widget = RootWidget::new(main_widget);

    // ...

    masonry::event_loop_runner::run(
        // ...
        main_widget,
        // ...
    )
    .unwrap();
}
```

First we create our initial widget hierarchy.
We're trying to build a simple to-do list app, so our root widget is a scrollable area (`Portal`) with a vertical list (`Flex`), whose first row is a horizontal list (`Flex`) containing a text field (`Textbox`) and an "Add task" button (`Button`).

We wrap it in a `RootWidget`, whose main purpose is to include a `Window` node in the accessibility tree.

At the end of the main function, we pass the root widget to the `event_loop_runner::run` function.
That function starts the main event loop, which runs until the user closes the window.
During the course of the event loop, the widget tree will be displayed, and updated as the user interacts with the app.


## The `Driver`

To handle user interactions, we need to implement the `AppDriver` trait:

```rust
trait AppDriver {
    fn on_action(&mut self, ctx: &mut DriverCtx<'_>, widget_id: WidgetId, action: Action);
}
```

Every time the user interacts with the app in a meaningful way (clicking a button, entering text, etc), the `on_action` method is called.

That method gives our app a `DriverCtx` context, which we can use to access the root widget, and a `WidgetId` identifying the widget that emitted the action.

We create a `Driver` struct to store a very simple app's state, and we implement the `AppDriver` trait for it:

```rust
use masonry::app_driver::{AppDriver, DriverCtx};
use masonry::{Action, WidgetId};
use masonry::widget::{Label};

struct Driver {
    next_task: String,
}

impl AppDriver for Driver {
    fn on_action(&mut self, ctx: &mut DriverCtx<'_>, _widget_id: WidgetId, action: Action) {
        match action {
            Action::ButtonPressed(_) => {
                let mut root: WidgetMut<RootWidget<Portal<Flex>>> = ctx.get_root();
                let mut portal = root.child_mut();
                let mut flex = portal.child_mut();
                flex.add_child(Label::new(self.next_task.clone()));
            }
            Action::TextChanged(new_text) => {
                self.next_task = new_text.clone();
            }
            _ => {}
        }
    }
}
```

In `on_action`, we handle the two possible actions:

- `TextChanged`: Update the text of the next task.
- `ButtonPressed`: Add a task to the list.

Because our widget tree only has one button and one textbox, there is no possible ambiguity as to which widget emitted the event, so we can ignore the `_widget_id` argument.

When handling `ButtonPressed`:

- `ctx.get_root()` returns a `WidgetMut<RootWidget<...>>`.
- `root.child_mut()` returns a `WidgetMut<Portal<...>>` for the `Portal`.
- `portal.child_mut()` returns a `WidgetMut<Flex>` for the `Flex`.

A `WidgetMut<...>` is a smart reference type which lets us modify the widget tree.
It's set up to automatically propagate update flags and update internal state when dropped.

We use `WidgetMut<Flex>::add_child()` to add a new `Label` with the text of our new task to our list.

In our main function, we create a `Driver` and pass it to `event_loop_runner::run`:

```rust
    // ...

    let driver = Driver {
        next_task: String::new(),
    };

    // ...

    masonry::event_loop_runner::run(
        // ...
        main_widget,
        driver,
    )
    .unwrap();
```

## Bringing it all together

The last step is to create our Winit window and start our main loop.

```rust
    use masonry::dpi::LogicalSize;
    use winit::window::Window;

    let window_attributes = Window::default_attributes()
        .with_title("To-do list")
        .with_resizable(true)
        .with_min_inner_size(LogicalSize::new(400.0, 400.0));

    masonry::event_loop_runner::run(
        masonry::event_loop_runner::EventLoop::with_user_event(),
        window_attributes,
        main_widget,
        driver,
    )
    .unwrap();
```

Our complete program therefore looks like this:

```rust
fn main() {
    const VERTICAL_WIDGET_SPACING: f64 = 20.0;

    use masonry::widget::{Button, Flex, Portal, RootWidget, Textbox};

    let main_widget = Portal::new(
        Flex::column()
            .with_child(
                Flex::row()
                    .with_flex_child(Textbox::new(""), 1.0)
                    .with_child(Button::new("Add task")),
            )
            .with_spacer(VERTICAL_WIDGET_SPACING),
    );
    let main_widget = RootWidget::new(main_widget);

    use masonry::app_driver::{AppDriver, DriverCtx};
    use masonry::{Action, WidgetId};
    use masonry::widget::{Label};

    struct Driver {
        next_task: String,
    }

    impl AppDriver for Driver {
        fn on_action(&mut self, ctx: &mut DriverCtx<'_>, _widget_id: WidgetId, action: Action) {
            match action {
                Action::ButtonPressed(_) => {
                    let mut root: WidgetMut<RootWidget<Portal<Flex>>> = ctx.get_root();
                    let mut portal = root.child_mut();
                    let mut flex = portal.child_mut();
                    flex.add_child(Label::new(self.next_task.clone()));
                }
                Action::TextChanged(new_text) => {
                    self.next_task = new_text.clone();
                }
                _ => {}
            }
        }
    }

    let driver = Driver {
        next_task: String::new(),
    };

use masonry::dpi::LogicalSize;
    use winit::window::Window;

    let window_attributes = Window::default_attributes()
        .with_title("To-do list")
        .with_resizable(true)
        .with_min_inner_size(LogicalSize::new(400.0, 400.0));

    masonry::event_loop_runner::run(
        masonry::event_loop_runner::EventLoop::with_user_event(),
        window_attributes,
        main_widget,
        driver,
    )
    .unwrap();
}
```

All the Masonry examples follow this structure:

- An initial widget tree.
- A struct implementing `AppDriver` to handle user interactions.
- A Winit window and event loop.

Some examples also define custom Widgets, but you can build an interactive app with Masonry's base widget set.
Doing so isn't *quite* recommended: most users will probably want to use a higher layer on top of Masonry.


## Higher layers

The above example isn't representative of how we expect Masonry to be used.

In practice, we expect most implementations of `AppDriver` to be GUI frameworks built on top of Masonry and using it to back their own abstractions.

Currently, the only public framework built with Masonry is Xilem, though we hope others will develop as Masonry matures.

Most of this documentation is written to help developers trying to build such a framework.
