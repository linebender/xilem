**TODO - Add screenshots - see [#501](https://github.com/linebender/xilem/issues/501)**

This tutorial covers the to-do-list example shown in the README, and uses it as a support to explain the basic Masonry architecture.

## Building a Masonry app

The first thing our code does after imports is to define a `Driver` which implements the `AppDriver` trait:

```rust
struct Driver {
    next_task: String,
}

impl AppDriver for Driver {
    fn on_action(&mut self, ctx: &mut DriverCtx<'_>, _widget_id: WidgetId, action: Action) {
        match action {
            Action::ButtonPressed(_) => {
                let mut root: WidgetMut<RootWidget<Portal<Flex>>> = ctx.get_root();
                let mut root = root.get_element();
                let mut flex = root.child_mut();
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

The AppDriver implementation has access to the root app state.
Its methods are called whenever an "action" is emitted by the app.
Actions are user interactions with semantic meaning, like "click a Button" or "change the text in a Textbox".

In our case, we change our `next_task` text when text is entered in a textbox, and we add a new line to the list when a button is clicked.

Because our button has a single textbox and a single button, there is no possible ambiguity as to which widget emitted the event, so we can ignore the `_widget_id` argument.

Next is the main function:

```rust
fn main() {
    const VERTICAL_WIDGET_SPACING: f64 = 20.0;

    let main_widget = Portal::new(
        Flex::column()
            .with_child(
                Flex::row()
                    .with_flex_child(Textbox::new(""), 1.0)
                    .with_child(Button::new("Add task")),
            )
            .with_spacer(VERTICAL_WIDGET_SPACING),
    );

    // ...
```

First we create our initial widget hierarchy.
`Portal` is a scrollable area, `Flex` is a container laid out with the flexbox algorithm, `Textbox` and `Button` are self-explanatory.

```rust
    // ...

    let window_size = LogicalSize::new(400.0, 400.0);
    let window_attributes = Window::default_attributes()
        .with_title("To-do list")
        .with_resizable(true)
        .with_min_inner_size(window_size);

    masonry::event_loop_runner::run(
        masonry::event_loop_runner::EventLoop::with_user_event(),
        window_attributes,
        RootWidget::new(main_widget),
        Driver {
            next_task: String::new(),
        },
    )
    .unwrap();
}
```

Finally, we create our Winit window and start our main loop.

Not that we separately pass the widget tree, the `AppDriver` and an `EventLoopBuilder` to the `run` function.

Once we call that function, the event loop runs until the user closes the program.


## The Masonry architecture

The above example isn't representative of how we expect Masonry to be used.

The code creates a Masonry app directly, and implements its own `AppDriver`.
In practice, we expect most implementations of `AppDriver` to be GUI frameworks built on top of Masonry, and using it to back their own abstractions.

Currently, the only public framework built with Masonry is Xilem, though we hope others will develop as Masonry matures.

Most of this documentation is written to help developers trying to build such a framework.
