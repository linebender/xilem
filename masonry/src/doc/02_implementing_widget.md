# Creating a new Widget

<!-- Copyright 2024 the Xilem Authors -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<div class="rustdoc-hidden">
> [!TIP]
>
> This file is intended to be read in rustdoc.
> Use `cargo doc --open --package masonry --no-deps`.
</div>

**TODO - Add screenshots - see [#501](https://github.com/linebender/xilem/issues/501)**

If you're building your own GUI framework on top of Masonry, or even a GUI app with specific needs, you'll want to specify your own widgets.

This tutorial explains how to create a simple leaf widget.


## The Widget trait

Widgets are types which implement the [`Widget`] trait.

This trait includes a set of methods that must be implemented to hook into Masonry's internals:

```rust,ignore
trait Widget {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent);
    fn on_text_event(&mut self, ctx: &mut EventCtx, event: &TextEvent);
    fn on_access_event(&mut self, ctx: &mut EventCtx, event: &AccessEvent);

    fn on_anim_frame(&mut self, ctx: &mut UpdateCtx, interval: u64);
    fn update(&mut self, ctx: &mut UpdateCtx, event: &Update);

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size;

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene);
    fn accessibility_role(&self) -> Role;
    fn accessibility(&mut self, ctx: &mut AccessCtx, node: &mut NodeBuilder);

    // ...
}
```

These methods are called by the framework at various points, with a `FoobarCtx` parameter giving information about the current widget (for example its size, position, or whether it's currently hovered).
The information accessible from the context argument depends on the method.

In the course of a frame, Masonry will run a series of passes over the widget tree, which will call these methods at different points:

- `on_pointer_event`, `on_text_event` and `on_access_event` are called once after a user-initiated event (like a mouse click or keyboard input).
- `on_anim_frame` is called once per frame for animated widgets.
- `update` is called many times during a frame, with various events reflecting changes in the widget's state (for instance, it gets or loses text focus).
- `layout` is called during Masonry's layout pass. It takes size constraints and returns the widget's desired size.
- `paint`, `accessibility_role` and `accessibility` are called roughly every frame for every widget, to allow them to draw to the screen and describe their structure to assistive technologies.

Most passes will skip most widgets by default.
For instance, the paint pass will only call a widget's `paint` method once, and then cache the resulting scene.
If your widget's appearance is changed by another method, you need to call `ctx.request_render()` to tell the framework to re-run the paint and accessibility passes.

Most context types include these methods for requesting future passes:

- `request_render()`
- `request_paint_only()`
- `request_accessibility_update()`
- `request_layout()`
- `request_anim_frame()`


## Widget mutation

In Masonry, widgets generally can't be mutated directly.
That is to say, even if you own a window, and even if that window holds a widget tree with a `Label` instance, you can't get a `&mut Label` directly from that window.

Instead, there are two ways to mutate `Label`:

- Inside a Widget method. Most methods (`on_pointer_event`, `update`, `layout`, etc) take a `&mut self` argument.
- Through a [`WidgetMut`] wrapper. So, to change your label's text, you will call `WidgetMut::<Label>::set_text()`. This helps Masonry make sure that internal metadata is propagated after every widget change.

As mentioned in the previous chapter, a `WidgetMut` is a smart reference type to the Widget tree.
Most Widgets will implement methods that let their users "project" a WidgetMut from a parent to its child.
For example, `WidgetMut<Portal<MyWidget>>` has a `get_child_mut()` method that returns a `WidgetMut<MyWidget>`.

So far, we've seen one way to get a WidgetMut: the [`DriverCtx::get_root()`] method in `AppDriver` implementations.
This methods returns a WidgetMut to the root widget, which you can then project into a WidgetMut reference to its descendants.

<!-- TODO - Change AppDriver trait to take a `&mut RenderRoot` instead, and rewrite above doc. -->

<!-- TODO - Mention edit_root_widget, edit_widget. -->

### Using WidgetMut in your custom Widget code

The WidgetMut type only has two fields, both public:

```rust,ignore
pub struct WidgetMut<'a, W: Widget> {
    pub ctx: MutateCtx<'a>,
    pub widget: &'a mut W,
}
```

`W` is your widget type. `MutateCtx` is yet another context type, with methods that let you get information about your widget and report that it changed in some ways.

If you want your widget to be mutable outside of its pass methods, you should write setter functions taking WidgetMut as a parameter.

These functions should modify the internal values of your widget, then set flags using `MutateCtx` depending on which values changed.
For instance, a `set_padding()` function should probably call `ctx.request_layout()`, whereas a `set_background_color()` function should probably call `ctx.request_render()` or `ctx.request_paint_only()`.


## Example widget: ColorRectangle

<!-- TODO - Interleave this with above documentation. -->

Let's implement a very simple widget: `ColorRectangle`.
This Widget has a size, a color, and emits a `ButtonPressed` action when the user left-clicks on it (on mouse press; we ignore mouse release).

First, let's create our struct:

```rust,ignore
use vello::kurbo::Size;
use vello::peniko::Color;

struct ColorRectangle {
    size: Size,
    color: Color,
}

impl ColorRectangle {
    fn new(size: Size, color: Color) -> Self {
        Self { size, color }
    }
}
```

This widget doesn't have children and doesn't really need to keep track of any transient state, so its definition is pretty simple.
Note that we store a size, and not a position: our widget's position is picked by its parent.

### Implementing the Widget trait

First we implement event methods:

```rust,ignore
use masonry::{
    Widget, EventCtx, PointerEvent, TextEvent, AccessEvent, Action
};

impl Widget for ColorRectangle {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        match event {
            PointerEvent::PointerDown(PointerButton::Primary, _) => {
                ctx.submit_action(Action::ButtonPressed(PointerButton::Primary));
            }
            _ => {},
        }
    }

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}

    fn on_access_event(&mut self, ctx: &mut EventCtx, event: &AccessEvent) {
            match event.action {
                accesskit::Action::Default => {
                    ctx.submit_action(Action::ButtonPressed(PointerButton::Primary));
                }
                _ => {}
            }
    }

    // ...
}
```

Here we've written a simple handler which filters pointer events for left clicks, and submits a [`ButtonPressed`] action.

We've also implemented the `on_access_event` method, which emulates the click behaviors for people using assistive technologies.

Next we can leave the `on_anim_frame` and `update` implementations empty:

```rust,ignore
use masonry::{
    UpdateCtx
};

impl Widget for ColorRectangle {
    // ...

    fn on_anim_frame(&mut self, _ctx: &mut UpdateCtx, _interval: u64) {}
    fn update(&mut self, _ctx: &mut UpdateCtx, _event: &Update) {}

    // ...
}
```

Next we implement layout:

```rust,ignore
use masonry::{
    LayoutCtx, BoxConstraints
};

impl Widget for ColorRectangle {
    // ...

    fn layout(&mut self, _ctx: &mut LayoutCtx, _bc: &BoxConstraints) -> Size {
        self.size
    }

    // ...
}
```

Our size is static, and doesn't depend on size constraints passed by our parent or context information like "the widget is currently hovered", so it can be written as a one-liner.

Next we write our render methods:

```rust,ignore
use masonry::{
    PaintCtx, AccessCtx
};
use vello::Scene;
use accesskit::{NodeBuilder, Role};

impl Widget for ColorRectangle {
    // ...

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        let rect = ctx.size().to_rect();
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            self.color,
            Some(Affine::IDENTITY),
            &rect,
        );
    }

    fn accessibility_role(&self) -> Role {
        Role::Button
    }

    fn accessibility(&mut self, ctx: &mut AccessCtx, node: &mut NodeBuilder) {
        node.set_default_action_verb(DefaultActionVerb::Click);
    }

    // ...
}
```

In our `paint` method, we're given a [`vello::Scene`] and paint a rectangle into it.

The rectangle's position is zero (because coordinates in our scenes are local to our widget), and its size is `ctx.size()`, which is the value returned by `layout()`; though we could also have used `self.size`.

Next we define our accessibility role.
Returning [`Role::Button`] means that screen readers will report our widget as a button, which roughly makes sense since it is clickable.

<!-- TODO - Add more detail about how you should choose your role. -->

In `accessibility`, we define a default action of `Click`, which is how we register our widget to be eligible for the `accesskit::Action::Default` event reported above.

<!-- TODO - Is that actually true? I'm not sure what set_default_action does. -->

We also write a `make_trace_span()` method, which is useful for debugging with the [tracing](https://docs.rs/tracing/latest/tracing/) framework.

```rust,ignore
use tracing::{trace_span, Span};

impl Widget for ColorRectangle {
    // ...

    fn make_trace_span(&self) -> Span {
        trace_span!("ColorRectangle")
    }

    // ...
}
```

And last, we stub in some additional methods:

```rust,ignore
use masonry::{
    RegisterCtx, WidgetId
};
use smallvec::SmallVec;

impl Widget for ColorRectangle {
    // ...

    fn register_children(&mut self, _ctx: &mut RegisterCtx) {}
    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        SmallVec::new()
    }
}
```

Don't worry about what they mean for now.

Finally, we want to define some setters for external users:

<!-- TODO - Rewrite once we've decided how WidgetMut should be implemented. -->

```rust,ignore
struct ColorRectangle {
    size: Size,
    color: Color,
}

impl WidgetMut<'_, ColorRectangle> {
    pub fn set_color(&mut self, color: Color) {
        self.widget.color = color;
        self.ctx.request_paint_only();
    }

    pub fn set_size(&mut self, size: Size) {
        self.widget.size = size;
        self.ctx.request_layout();
    }
}
```

By making ColorRectangle's fields private, and making it so the only way to mutate them is through a WidgetMut, we make it "watertight".
Our users can never find themselves in a situation where they forget to propagate invalidation flags, and end up with confusing bugs.


## Next up

This document was about how to create a simple leaf widget.

The next one is about creating a container widgets, and the complications it adds.

[`Widget`]: crate::Widget
[`WidgetMut`]: crate::widget::WidgetMut
[`DriverCtx::get_root()`]: crate::DriverCtx::get_root
[`ButtonPressed`]: crate::Action::ButtonPressed
[`vello::Scene`]: crate::vello::Scene
[`Role::Button`]: accesskit::Role::Button
