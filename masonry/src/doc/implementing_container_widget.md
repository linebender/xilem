# Creating a container widget

<!-- Copyright 2024 the Xilem Authors -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<div class="rustdoc-hidden">

> 💡 Tip
>
> This file is intended to be read in rustdoc.
> Use `cargo doc --open --package masonry_winit --no-deps`.

</div>

**TODO - Add screenshots - see [#501](https://github.com/linebender/xilem/issues/501)**

In the previous section we implemented a simple widget.
Our widget was overall pretty simple, the tutorial skipped over a few methods with a "we'll explain later" handwave.

However, in some cases you want to write a widget which "contains" other widgets.
You want these child widgets to receive events and be painted as well, as part of the widget hierarchy.

Note: If you only need a simple pass-through container that hosts exactly one child and you want to be able to dynamically swap that child at runtime, consider using [`Passthrough`](crate::widgets::Passthrough).
It forwards layout/paint/accessibility to its content and exposes helpers to replace or edit the hosted child.
For richer behavior (custom layout, chrome, etc.), continue with a dedicated container as described below.

To do so, you need to implement a container widget.
A container widget is still a type which implements the [`Widget`] trait.
It stores handles for its children using a type called [`WidgetPod`], and its `Widget` trait implementation is more complex.

As an example, let's write a `VerticalStack` widget, which lays out its children in a vertical line:

```rust,ignore
use masonry::core::{Widget, WidgetPod};

pub struct VerticalStack {
    children: Vec<WidgetPod<dyn Widget>>,
    gap: f64,
}

impl VerticalStack {
    pub fn new(gap: f64) -> Self {
        Self {
            children: Vec::new(),
            gap,
        }
    }
}
```

## Container-specific methods

A container widget needs to pay special attention to these methods:

```rust,ignore
trait Widget {
    // ...

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, cross_length: Option<f64>) -> f64;
    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, props: &PropertiesRef<'_>, size: Size);
    fn compose(&mut self, ctx: &mut ComposeCtx<'_>);

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>);
    fn children_ids(&self) -> ChildrenIds;
}
```

Let's go over them one by one.

### `layout`

Like with a leaf widget, the `measure` method must compute and return the length of the container widget.

Before that, it must call [`MeasureCtx::compute_length`] for each of its own children.
For a vertical stack, we want to sum these on the vertical axis and take the largest on the horizontal axis.

Then later in `layout`, it must call [`LayoutCtx::run_layout`] then [`LayoutCtx::place_child`] for each of its own children:

- `LayoutCtx::run_layout` recursively calls `Widget::layout` on the child.
  It takes a [`Size`] argument, which is the chosen size of the child.
- `LayoutCtx::place_child` sets the child's position relative to the container.

The `layout` method *must* iterate over all its children.
Not doing so is a logical bug.
When debug assertions are on, Masonry will actively try to detect cases where you forget to compute a child's layout and panic if it finds such a case.

For our `VerticalStack`, we'll lay out our children in a vertical line, with a gap between each child; we give each child an equal share of the available height:

```rust,ignore
use masonry::core::{LayoutCtx, MeasureCtx, PropertiesRef};
use masonry::kurbo::{Axis, Point, Size};
use masonry::layout::{LayoutSize, LenDef, LenReq, SizeDef};

impl Widget for VerticalStack {
    // ...

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<f64>,
    ) -> f64 {
        let (len_req, min_result) = match len_req {
            LenReq::MinContent | LenReq::MaxContent => (len_req, 0.),
            LenReq::FitContent(space) => (LenReq::MinContent, space),
        };

        let auto_size = SizeDef::req(axis, len_req);
        let context_size = LayoutSize::maybe(axis.cross(), cross_length);
        
        let mut length: f64 = 0.;
        for child in &mut self.children {
            let child_length = ctx.compute_length(child, auto_size, context_size, axis, cross_length);
            match axis {
                Axis::Horizontal => length = length.max(child_length),
                Axis::Vertical => length += child_length,
            }
        }

        if axis == Axis::Vertical {
            let gap_count = (self.children.len() - 1) as f64;
            length += gap_count * self.gap;
        }

        min_result.max(length)
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        _props: &PropertiesRef<'_>,
        size: Size,
    ) {
        let gap_count = (self.children.len() - 1) as f64;
        let total_child_vertical_space = size.height - self.gap * gap_count;
        let child_vertical_space = total_child_vertical_space / self.children.len() as f64;

        let width_def = LenDef::FitContent(size.width);
        let height_def = LenDef::FitContent(child_vertical_space.max(0.));
        let auto_size = SizeDef::new(width_def, height_def);
        let context_size = size.into();

        let mut y_offset = 0.0;
        for child in &mut self.children {
            let child_size = ctx.compute_size(child, auto_size, context_size);
            ctx.run_layout(child, child_size);
            ctx.place_child(child, Point::new(0.0, y_offset));

            y_offset += child_size.height + self.gap;
        }
    }

    // ...
}
```

There are a few things to note here:

- We refer to `LenReq::FitContent` to get the total space available to the container.
- We compute the height of the container by summing the heights of all children and adding the gap between them.
  We compute the width of the container by taking the largest child width.

### `compose`

The `compose` method is called during the compose pass, after layout.

The compose pass runs top-down and assigns transforms to children. Transform-only layout changes (e.g. scrolling) should request compose instead of requesting layout.

Compose is meant to be a cheaper way to position widgets than layout. Because the compose pass is more limited than layout, it's easier to recompute in many situations.

For instance, if a widget in a list changes size, its siblings and parents must be re-laid out to account for the change; whereas changing a given widget's transform only affects its children.

In the case of our `VerticalStack`, we don't implement any transform-only changes, so we don't need to do anything in compose:

```rust,ignore
use masonry::core::ComposeCtx;

impl Widget for VerticalStack {
    // ...

    fn compose(&mut self, _ctx: &mut ComposeCtx<'_>) {}
}
```

### `register_children` and `children_ids`

The `register_children` method must call [`RegisterCtx::register_child`] for each child:

```rust,ignore
use masonry::core::RegisterCtx;

impl Widget for VerticalStack {
    // ...

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        for child in &mut self.children {
            ctx.register_child(child);
        }
    }

    // ...
}
```

The `register_children` method is called to insert new children in your container into the widget tree.

You can request a call to that method by calling `ctx.children_changed()` on various context types.

The method *must* iterate over all its children.
Not doing so is a logical bug, and may also trigger debug assertions.

"All its children", in this context, means the children whose ids are returned by the `children_ids` method:

```rust,ignore
use masonry::core::ChildrenIds;

impl Widget for VerticalStack {
    // ...

    fn children_ids(&self) -> ChildrenIds {
        self.children.iter().map(|child| child.id()).collect()
    }
}
```

The `children_ids` method must return the IDs of all your container's children.
That list is considered the "canonical" list of children by Masonry, and must match the children visited during `register_children` and `layout`.
It should be stable across calls; anything that mutates the list `children_ids()` returns must also call `ctx.children_changed()`.


## Editing the widget tree

We've seen how to deal with the children of a container widget once they're already there.

But how do we add them in the first place?

Widgets will usually be added or removed through a [`WidgetMut`] wrapper.
Let's write `WidgetMut` methods for our `VerticalStack`:

```rust,ignore
use masonry::core::{NewWidget, WidgetMut};

impl VerticalStack {
    pub fn add_child(this: &mut WidgetMut<'_, Self>, child: NewWidget<dyn Widget>) {
        this.widget.children.push(child.to_pod());
        this.ctx.children_changed();
    }

    pub fn remove_child(this: &mut WidgetMut<'_, Self>, n: usize) {
        this.widget.children.remove(n);
        this.ctx.children_changed();
    }

    pub fn clear_children(this: &mut WidgetMut<'_, Self>) {
        this.widget.children.clear();
        this.ctx.children_changed();
    }
}
```

<!-- TODO - Explain what NewWidget is -->

If you want to add or remove a child during other passes, the simplest solution is to use the `mutate_self_later` context method.
That mutate takes a callback, and schedules it to be run with a `WidgetMut` wrapper to the current widget.


## Regular `Widget` methods

Now that we've implemented our container-specific methods, we should also implement the regular `Widget` methods.

In the case of our `VerticalStack`, all of them can be left empty:

```rust,ignore
use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, AccessEvent, EventCtx, NoAction, PaintCtx, PointerEvent, PropertiesRef, TextEvent,
    Update, UpdateCtx,
};
use masonry::vello::Scene;

impl Widget for VerticalStack {
    type Action = NoAction;

    fn on_pointer_event(&mut self, _ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, _event: &PointerEvent) {}
    fn on_text_event(&mut self, _ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, _event: &TextEvent) {}
    fn on_access_event(&mut self, _ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, _event: &AccessEvent) {}

    fn on_anim_frame(&mut self, _ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, _interval: u64) {}
    fn update(&mut self, _ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, _event: &Update) {}

    // ...

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _scene: &mut Scene) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    // ...
}
```

This might surprise you: shouldn't our container widget recurse these methods to its children?
Doesn't `VerticalStack::paint` need to call `paint` on its children, for instance?

It doesn't.

In Masonry, most passes are automatically propagated to children, and so container widgets do not need to *and cannot* call the pass methods on their children.

So for instance, if `VerticalStack::children_ids()` returns a list of three children, the paint pass will automatically call `paint` on all three children after `VerticalStack::paint()`.

Pass methods in container widgets should only implement the logic that is specific to the container itself.
For instance, a container widget with a background color should implement `paint` to draw the background.

[`Size`]: crate::kurbo::Size
[`Widget`]: crate::core::Widget
[`WidgetPod`]: crate::core::WidgetPod
[`WidgetMut`]: crate::core::WidgetMut
[`MeasureCtx::compute_length`]: crate::core::MeasureCtx::compute_length
[`LayoutCtx::place_child`]: crate::core::LayoutCtx::place_child
[`LayoutCtx::run_layout`]: crate::core::LayoutCtx::run_layout
[`RegisterCtx::register_child`]: crate::core::RegisterCtx::register_child
