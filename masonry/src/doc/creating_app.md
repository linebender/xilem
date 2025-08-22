# Building a "To-Do List" app

<!-- Copyright 2024 the Xilem Authors -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<div class="rustdoc-hidden">

> 💡 Tip
>
> This file is intended to be read in rustdoc.
> Use `cargo doc --open --package masonry_winit --no-deps`.

</div>


**TODO - Add screenshots - see [#501](https://github.com/linebender/xilem/issues/501)**

This tutorial explains how to build a simple Masonry app, step by step.
Though it isn't representative of how we expect Masonry to be used, it does cover the basic architecture.

The app we'll create is identical to the to-do-list example shown in the README.

## Dependencies

In this tutorial, we'll create a Masonry app, running in a Winit window, with some common widgets provided by Masonry.

This means you should add some dependencies to your project:

```sh
cargo add masonry
cargo add masonry_winit
```


## The widget tree

Let's start with the `main()` function.

```rust,ignore
fn main() {
    const VERTICAL_WIDGET_SPACING: f64 = 20.0;

    use masonry::widgets::{Button, Flex, Portal, TextInput};

    let main_widget = Portal::new(
        Flex::column()
            .with_child(
                Flex::row()
                    .with_flex_child(TextInput::new(""), 1.0)
                    .with_child(Button::new("Add task")),
            )
            .with_spacer(VERTICAL_WIDGET_SPACING),
    );

    // ...

    masonry_winit::app::run(
        // ...
        main_widget,
        // ...
    )
    .unwrap();
}
```

First we create our initial widget hierarchy.
We're trying to build a simple to-do list app, so our root widget is a scrollable area ([`Portal`]) with a vertical list ([`Flex`]), whose first row is a horizontal list (`Flex` again) containing a text field ([`TextInput`]) and an "Add task" button ([`Button`]).

At the end of the main function, we pass the root widget to the `event_loop_runner::run` function.
That function starts the main event loop, which runs until the user closes the window.
During the course of the event loop, the widget tree will be displayed, and updated as the user interacts with the app.

## The `Driver`

To handle user interactions, we need to implement the `AppDriver` trait:

```rust,ignore
trait AppDriver {
    fn on_action(&mut self, window_id: WindowId, ctx: &mut DriverCtx<'_>, widget_id: WidgetId, action: ErasedAction);
}
```

Every time the user interacts with the app in a meaningful way (clicking a button, entering text, etc), the widget the user interacted with will emit an action, and the framework will call `AppDriver::on_action()`.
These actions are type-erased in the [`ErasedAction`] type.
Each widget documents which action types it will emit, and in which circumstances.

That method gives our app a `DriverCtx` context and a window id, which we can use to access the widget tree, and a [`WidgetId`] identifying the widget that emitted the action.

We create a `Driver` struct to store a very simple app's state, and we implement the `AppDriver` trait for it:

```rust,ignore
use masonry::core::{Action, WidgetId};
use masonry::widgets::Label;
# use masonry::widgets::{Button, Flex, Portal, TextInput};
use masonry_winit::app::{AppDriver, DriverCtx};

struct Driver {
    next_task: String,
}

impl AppDriver for Driver {
    fn on_action(
        &mut self,
        window_id: WindowId,
        ctx: &mut DriverCtx<'_, '_>,
        _widget_id: WidgetId,
        action: ErasedAction,
    ) {
        if action.is::<ButtonPress>() {
            ctx.render_root(window_id).edit_root_widget(|mut root| {
                let mut portal = root.downcast::<Portal<Flex>>();
                let mut flex = Portal::child_mut(&mut portal);
                Flex::add_child(&mut flex, Label::new(self.next_task.clone()));
            });
        } else if action.is::<TextAction>() {
            let action: TextAction = *action.downcast().unwrap();
            match action {
                TextAction::Changed(new_text) => self.next_task = new_text.clone(),
                _ => {}
            }
        }
    }
}
```

In `on_action`, we handle the two possible actions:

- `TextAction::Changed`: Update the text of the next task.
- `ButtonPress`: Add a task to the list.

Because our widget tree only has one button and one text input, there is no possible ambiguity as to which widget emitted the event, so we can ignore the `WidgetId` argument.

When handling `ButtonPress`:

- `ctx.render_root()` returns a reference to the `RenderRoot`, which owns the widget tree and all the associated visual state.
- `RenderRoot::edit_root_widget()` takes a closure; that closure takes a `WidgetMut<dyn Widget>` which we call `root`. Once the closure returns, `RenderRoot` runs some passes to update the app's internal states.
- `root.downcast::<...>()` returns a `WidgetMut<Portal<Flex>>`.
- `Portal::child_mut()` returns a `WidgetMut<Flex>`.

A [`WidgetMut`] is a smart reference type which lets us modify the widget tree.
It's set up to automatically propagate update flags and update internal state when dropped.

We use [`Flex::add_child()`][add_child] to add a new `Label` with the text of our new task to our list.

In our main function, we create a `Driver` and pass it to `event_loop_runner::run`:

```rust,ignore
    // ...

    let driver = Driver {
        next_task: String::new(),
    };

    // ...

    masonry_winit::app::run(
        // ...
        main_widget,
        driver,
    )
    .unwrap();
```

## Bringing it all together

The last step is to create our Winit window and start our main loop.

```rust,ignore
    use masonry::dpi::LogicalSize;
    use masonry_winit::winit::window::Window;

    let window_attributes = masonry_winit::winit::window::WindowAttributes::default()
        .with_title("To-do list")
        .with_resizable(true)
        .with_min_inner_size(LogicalSize::new(400.0, 400.0));

    masonry_winit::app::run(
        masonry_winit::app::EventLoop::builder(),
        window_attributes,
        main_widget,
        driver,
    )
    .unwrap();
```

Our complete program therefore looks like this:

```rust,ignore
fn main() {
    const VERTICAL_WIDGET_SPACING: f64 = 20.0;

    use masonry::widgets::{Button, Flex, Portal, TextInput};

    let main_widget = Portal::new(
        Flex::column()
            .with_child(
                Flex::row()
                    .with_flex_child(TextInput::new(""), 1.0)
                    .with_child(Button::new("Add task")),
            )
            .with_spacer(VERTICAL_WIDGET_SPACING),
    );

    use masonry::core::{ErasedAction, WidgetId};
    use masonry::widgets::Label;
    use masonry_winit::app::{AppDriver, DriverCtx};

    struct Driver {
        next_task: String,
    }

    impl AppDriver for Driver {
        fn on_action(
            &mut self,
            window_id: WindowId,
            ctx: &mut DriverCtx<'_, '_>,
            _widget_id: WidgetId,
            action: ErasedAction,
        ) {
            if action.is::<ButtonPress>() {
                ctx.render_root(window_id).edit_root_widget(|mut root| {
                    let mut portal = root.downcast::<Portal<Flex>>();
                    let mut flex = Portal::child_mut(&mut portal);
                    Flex::add_child(&mut flex, Label::new(self.next_task.clone()));
                });
            } else if action.is::<TextAction>() {
                let action: TextAction = *action.downcast().unwrap();
                match action {
                    TextAction::Changed(new_text) => self.next_task = new_text.clone(),
                    _ => {}
                }
            }
        }
    }

    let driver = Driver {
        next_task: String::new(),
    };

    use masonry::dpi::LogicalSize;
    use masonry_winit::winit::window::Window;

    let window_attributes = masonry_winit::winit::window::WindowAttributes::default()
        .with_title("To-do list")
        .with_resizable(true)
        .with_min_inner_size(LogicalSize::new(400.0, 400.0));

    # return;

    masonry_winit::app::run(
        masonry_winit::app::EventLoop::builder(),
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

Some examples also define custom widgets, but you can build an interactive app with Masonry's base widget set, though it's not Masonry's intended use.


## Higher layers

The above example isn't representative of how we expect Masonry to be used.

In practice, we expect most implementations of `AppDriver` to be GUI frameworks built on top of Masonry and using it to back their own abstractions.

Currently, the only public framework built with Masonry is Xilem, though we hope others will develop as Masonry matures.

Most of this documentation is written to help developers trying to build such a framework.

[`Portal`]: crate::widgets::Portal
[`Flex`]: crate::widgets::Flex
[`TextInput`]: crate::widgets::TextInput
[`Button`]: crate::widgets::Button

[`ErasedAction`]: crate::core::ErasedAction
[`WidgetId`]: crate::core::WidgetId
[`WidgetMut`]: crate::core::WidgetMut
[add_child]: crate::widgets::Flex::add_child
