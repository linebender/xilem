# Creating a new Widget

<!-- Copyright 2024 the Xilem Authors -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<div class="rustdoc-hidden">

> 💡 Tip
>
> This file is intended to be read in rustdoc.
> Use `cargo doc --open --package masonry_winit --no-deps`.

</div>

**TODO - Add screenshots - see [#501](https://github.com/linebender/xilem/issues/501)**

If you're building your own GUI framework on top of Masonry, or even a GUI app with specific needs, you'll want to specify your own widgets.

This tutorial explains how to create a simple leaf widget.


## The Widget trait

Widgets are types which implement the [`Widget`] trait:

```rust,ignore
trait Widget {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, _props: &mut PropertiesMut<'_>, event: &PointerEvent);
    fn on_text_event(&mut self, ctx: &mut EventCtx, _props: &mut PropertiesMut<'_>, event: &TextEvent);
    fn on_access_event(&mut self, ctx: &mut EventCtx, _props: &mut PropertiesMut<'_>, event: &AccessEvent);

    fn on_anim_frame(&mut self, ctx: &mut UpdateCtx, _props: &mut PropertiesMut<'_>, interval: u64);
    fn update(&mut self, ctx: &mut UpdateCtx, _props: &mut PropertiesMut<'_>, event: &Update);

    fn layout(&mut self, ctx: &mut LayoutCtx, _props: &mut PropertiesMut<'_>, bc: &BoxConstraints) -> Size;

    fn paint(&mut self, ctx: &mut PaintCtx, _props: &PropertiesRef<'_>, scene: &mut Scene);
    fn accessibility_role(&self) -> Role;
    fn accessibility(&mut self, ctx: &mut AccessCtx, _props: &PropertiesRef<'_>, node: &mut Node);

    // ...
}
```

In the course of a frame, Masonry will run a series of passes over the widget tree, which will call these methods at different points:

- `on_pointer_event`, `on_text_event` and `on_access_event` are called once after a user-initiated event (like a mouse click or keyboard input).
- `on_anim_frame` is called once per frame for animated widgets.
- `update` is called many times during a frame, with various events reflecting changes in the widget's state (for instance, it gets or loses text focus).
- `layout` is called during Masonry's layout pass. It takes size constraints and returns the widget's desired size.
- `paint`, `accessibility_role` and `accessibility` are called roughly every frame for every widget, to allow them to draw to the screen and describe their structure to assistive technologies.


## Our example widget: `ColorRectangle`

<!-- TODO - Create color_rectangle.rs file as a template for this. -->

Let's implement a very simple widget named `ColorRectangle`.
This widget has a size, a color, and will notify Masonry when the user left-clicks on it (on mouse press; we'll ignore mouse release).

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
Note that we store a size, and not a position: our widget's position is managed by its parent.


### Event methods

First we implement event methods:

```rust,ignore
use masonry_winit::core::{
    Widget, EventCtx, PointerEvent, TextEvent, AccessEvent, Action
};

impl Widget for ColorRectangle {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        match event {
            PointerEvent::PointerDown(PointerButton::Primary, _) => {
                ctx.submit_action(Action::ButtonPressed(PointerButton::Primary));
            }
            _ => {},
        }
    }

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _props: &mut PropertiesMut<'_>, _event: &TextEvent) {}

    fn on_access_event(&mut self, ctx: &mut EventCtx, _props: &mut PropertiesMut<'_>, event: &AccessEvent) {
            match event.action {
                accesskit::Action::Click => {
                    ctx.submit_action(Action::ButtonPressed(PointerButton::Primary));
                }
                _ => {}
            }
    }

    // ...
}
```

We handle pointer events and accessibility events the same way: we check the event type, and if it's a left-click, we submit an action.

Submitting an action lets Masonry know that a semantically meaningful event has occurred; Masonry will call `AppDriver::on_action()` with the action before the end of the frame.
This lets higher-level frameworks like Xilem react to UI events, like a button being pressed.

Implementing `on_access_event` lets us emulate click behaviors for people using assistive technologies.

We don't handle any text events.


### Animation and update

Since our widget isn't animated and doesn't react to changes in status, we can leave the `on_anim_frame` and `update` implementations empty:

```rust,ignore
use masonry_winit::core::{
    UpdateCtx, Update,
};

impl Widget for ColorRectangle {
    // ...

    fn on_anim_frame(&mut self, _ctx: &mut UpdateCtx, _props: &mut PropertiesMut<'_>, _interval: u64) {}
    fn update(&mut self, _ctx: &mut UpdateCtx, _props: &mut PropertiesMut<'_>, _event: &Update) {}

    // ...
}
```

### Layout

Next we implement layout:

```rust,ignore
use masonry_winit::core::{
    LayoutCtx, BoxConstraints
};

impl Widget for ColorRectangle {
    // ...

    fn layout(&mut self, _ctx: &mut LayoutCtx, _props: &mut PropertiesMut<'_>, bc: &BoxConstraints) -> Size {
        bc.constrain(self.size)
    }

    // ...
}
```

Our size is static, and doesn't perform complex operations like line-breaking, or depend on context information like "the widget is currently hovered", so it can be written as a one-liner.

`BoxConstraints` are the minimum and maximum size our parent expects this widget to respect. It's possible to ignore them, but we choose not to.
We return our stored size, clamped between the min and max constraints.

### Render methods

Next we write our render methods:

```rust,ignore
use masonry_winit::core::{
    PaintCtx, AccessCtx
};
use vello::Scene;
use accesskit::{Node, Role};

impl Widget for ColorRectangle {
    // ...

    fn paint(&mut self, ctx: &mut PaintCtx, _props: &PropertiesRef<'_>, scene: &mut Scene) {
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

    fn accessibility(&mut self, ctx: &mut AccessCtx, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.add_action(accesskit::Action::Click);
    }

    // ...
}
```

In our `paint` method, we're given a [`vello::Scene`] and paint a rectangle into it.

The rectangle's origin is zero (because coordinates in our scene are local to our widget), and its size is `ctx.size()`, which is the value returned by `layout()`.

Next we define our accessibility role.
Returning [`Role::Button`] means that screen readers will report our widget as a button, which roughly makes sense since it is clickable.

In `accessibility`, we're given a mutable reference to a node from the accessibility tree representing the current widget, pre-filled with some information about it.

We edit that node to mark that our widget accepts the `accesskit::Action::Click` event.

#### Writing accessibility code

Masonry's support of accessibility APIs is based on the [accesskit](https://github.com/AccessKit/accesskit) crate.
For more info on which role `accessibility_role` should return and how the accessibility node should be edited, consult the accesskit documentation.

... Well, in theory.
In practice, accesskit is currently sparsely documented ([see accesskit#402](https://github.com/AccessKit/accesskit/issues/402)), and some values are subject to interpretation.
AccessKit is inspired by Chromium's accessibility API, which is undocumented, and by the ARIA standard.

Pragmatically, if you're not sure about what a certain value means or how to implement a certain feature, your recourses are:

- Read the accesskit documentation.
- Read the ARIA documentation.
- Look at the egui and Masonry widget implementations.
- Look at the accesskit platform bindings code.
- Ask your question on the Linebender zulip, in [the #accessibility channel](https://xi.zulipchat.com/#narrow/channel/359997-accessibility/).


### Other methods

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
use masonry_winit::core::{
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


## Context arguments

The methods of the [`Widget`] trait take `***Ctx` context arguments, which you can use to get information about the current widget.
For instance, our `paint()` method used [`PaintCtx::size()`] to get the widget's size.
The information accessible from the context argument depends on the method.

### Status flags

All context types have getters to check some status information:

- `is_hovered()`
- `has_pointer_capture()`
- `is_focused()`
- `is_disabled()`
- `is_stashed()`

See the ["Concepts and definitions"](06_masonry_concepts.md#widget-status) documentation for more information on what they mean.

### Requesting passes

Most passes will skip most widgets by default.

For example, the animate pass will only run on widgets running an animation, and the paint pass will only call a widget's `paint` method once, after which it caches the resulting scene.

If your widget's appearance is changed by another method, you need to call `ctx.request_render()` to tell the framework to re-run the paint and accessibility passes.

Most context types include these methods for requesting future passes:

- `request_render()`
- `request_paint_only()`
- `request_accessibility_update()`
- `request_layout()`
- `request_anim_frame()`


### Using context in `ColorRectangle`

To show how context types are used in practice, let's add a feature to `ColorRectangle`: the widget will now be painted in white when hovered.

First, we update our paint method:

```rust,ignore
impl Widget for ColorRectangle {
    // ...

    fn paint(&mut self, ctx: &mut PaintCtx, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        let rect = ctx.size().to_rect();
        let color = if ctx.is_hovered() {
            Color::WHITE
        } else {
            self.color
        };
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            color,
            Some(Affine::IDENTITY),
            &rect,
        );
    }

    // ...
}
```

Next, we need to detect hover changes so the paint method actually gets called.

Let's re-implement the `update` method:

```rust,ignore
impl Widget for ColorRectangle {
    // ...

    fn update(&mut self, ctx: &mut UpdateCtx, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::HoveredChanged(_) => {
                ctx.request_render();
            }
            _ => {}
        }
    }

    // ...
}
```

## Properties

Most of the methods we've listed take a `props: &mut PropertiesMut<'_>` argument.

We won't cover properties in this chapter.
See [Reading Widget Properties](crate::doc::doc_04b_widget_properties) for more info.


## Widget mutation

In Masonry, widgets generally can't be mutated directly.
That is to say, even if you own a window, and even if that window holds a widget tree with a `Label` instance, you can't get a `&mut Label` directly from that window.

Instead, there are two ways to mutate `Label`:

- Inside a Widget method. Most methods (`on_pointer_event`, `update`, `layout`, etc) take a `&mut self` argument.
- Through a [`WidgetMut`] wrapper. So, to change your label's text, you will call [`Label::set_text()`], which takes a [`WidgetMut`] argument. This helps Masonry make sure that internal metadata is propagated after every widget change.

As mentioned in the previous chapter, a `WidgetMut` is a smart reference type to the Widget tree.
Most Widgets will implement methods that let their users "project" a WidgetMut from a parent to its child.
For example, `WidgetMut<Portal<MyWidget>>` has a `get_child_mut()` method that returns a `WidgetMut<MyWidget>`.

So far, we've seen one way to get a WidgetMut: the [`RenderRoot::edit_root_widget()`] method.
This methods returns a WidgetMut to the root widget, which you can then project into a WidgetMut reference to its descendants.

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


### Mutating `ColorRectangle`

Let's define some setters for `ColorRectangle`:

```rust,ignore
struct ColorRectangle {
    size: Size,
    color: Color,
}

impl ColorRectangle {
    pub fn set_color(this: &mut WidgetMut<'_, Self>, color: Color) {
        this.widget.color = color;
        this.ctx.request_paint_only();
    }

    pub fn set_size(this: &mut WidgetMut<'_, Self>, size: Size) {
        this.widget.size = size;
        this.ctx.request_layout();
    }
}
```

By making ColorRectangle's fields private, and making it so the only way to mutate them is through a WidgetMut, we make it "watertight".
Our users can never find themselves in a situation where they forget to propagate invalidation flags, and end up with confusing bugs.


## Next up

This document was about how to create a simple leaf widget.

The next one is about creating a container widgets, and the complications it adds.

[`Widget`]: crate::core::Widget
[`WidgetMut`]: crate::core::WidgetMut
[`PaintCtx::size()`]: crate::core::PaintCtx::size
[`UpdateCtx::request_paint_only()`]: crate::core::UpdateCtx::request_paint_only
[`ButtonPressed`]: crate::core::Action::ButtonPressed
[`vello::Scene`]: crate::vello::Scene
[`Role::Button`]: accesskit::Role::Button
[`RenderRoot::edit_root_widget()`]: crate::app::RenderRoot::edit_root_widget
[`Label::set_text()`]: https://docs.rs/masonry_widgets/latest/masonry_widgets/widgets/struct.Label.html#method.set_text
