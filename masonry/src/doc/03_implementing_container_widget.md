# Creating a container Widget

**TODO - Add screenshots - see [#501](https://github.com/linebender/xilem/issues/501)**

In the previous section we implemented a simple widget.
Our widget was overall pretty simple, the tutorial skipped over a few methods with a "we'll explain later" handwave.

However, in some cases you want to write a widget which "contains" other widgets.
You want these child widgets to receive events and be painted as well, as part of the widget hierarchy.

To do so, you need to implement a container widget.
A container widget is still a type which implements the `Widget` trait.
It stores handles to its children using a type called `WidgetPod`, and its `Widget` trait implementation is more complex.

As an example, let's write a `VerticalStack` widget, which lays out its children in a vertical line:

```ignore
struct VerticalStack {
    children: Vec<WidgetPod<Box<dyn Widget>>>,
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

```ignore
trait Widget {
    // ...

    fn layout(&mut self, ctx: &mut LayoutCtx) -> Size;
    fn compose(&mut self, ctx: &mut ComposeCtx);

    fn register_children(&mut self, ctx: &mut RegisterCtx);
    fn children_ids(&self) -> SmallVec<[WidgetId; 16]>;
}
```

Let's go over them one by one.

### `layout`

Like with a leaf widget, the `layout` method must compute and return the size of the container widget.

Before that, it must call `LayoutCtx::run_layout` then `LayoutCtx::place_child` for each of its own children:

- `LayoutCtx::run_layout` recursively calls `Widget::layout` on the child. It takes a `BoxConstraints` argument, which represents how much space the parent "gives" to the child.
- `LayoutCtx::place_child` sets the child's position relative to the container.

Generally, containers first get the size of all their children, then use that information and the parent constraints to both compute their own size and spread the children within the available space.

The `layout` method *must* iterate over all its children.
Not doing so is a logical bug.
When debug assertions are on, Masonry will actively try to detect cases where you forget to compute a child's layout and panic if it finds such a case.

For our `VerticalStack`, we'll lay out our children in a vertical line, with a gap between each child; we give each child an equal share of the available height:

```ignore
use masonry::{
    LayoutCtx, BoxConstraints
};

impl Widget for VerticalStack {
    // ...

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        let total_width = bc.max().height;
        let total_height = bc.max().height;
        let total_child_height = total_height - self.gap * (self.children.len() - 1) as f64;
        let child_height = total_child_height / self.children.len() as f64;

        let mut y_offset = 0.0;
        for child in &self.children {
            let child_bc = BoxConstraints::new(Size::new(0., 0.), Size::new(total_width, child_height));

            let child_size = ctx.run_layout(child, &child_bc);
            ctx.place_child(child, Point::new(0.0, y_offset));

            y_offset += child_size.height + self.gap;
        }

        let total_height = y_offset - self.gap;
        Size::new(total_width, total_height)
    }

    // ...
}
```

There are a few things to note here:

- We use `bc.max()` to get the total space available to the container.
- Our children get maximum constraints based on their share of the available space, and a minimum of 0.
- We compute the size of each child, and use the computed height to get the offset for the next child. That height might smaller **or greater** than the child's available height.
- We compute the height of the container by summing the heights of all children and adding the gap between them. That total height might be smaller **or greater** than the container's available height.
- We return the total size of the container.


### `compose`

The `compose` method is called during the compose pass, after layout.

The compose pass runs top-down and assigns transforms to children. Transform-only layout changes (e.g. scrolling) should request compose instead of requesting layout.

Compose is meant to be a cheaper way to position widgets than layout. Because the compose pass is more limited than layout, it's easier to recompute in many situations.

For instance, if a widget in a list changes size, its siblings and parents must be re-laid out to account for the change; whereas changing a given widget's transform only affects its children.

In the case of our `VerticalStack`, we don't implement any transform-only changes, so we don't need to do anything in compose:

```ignore
use masonry::{
    LayoutCtx, BoxConstraints
};

impl Widget for VerticalStack {
    // ...

    fn compose(&mut self, _ctx: &mut ComposeCtx) {}
}
```

### `register_children` and `children_ids`

The `register_children` method must call `RegisterCtx::register_child` for each child:

```ignore
use masonry::{
    Widget, RegisterCtx
};

impl Widget for VerticalStack {
    // ...

    fn register_children(&mut self, ctx: &mut RegisterCtx) {
        for child in &self.children {
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

```ignore
impl Widget for VerticalStack {
    // ...

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
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

Widgets will usually be added or removed through a `WidgetMut` wrapper.
Let's write WidgetMut methods for our `VerticalStack`:

```ignore
impl WidgetMut<'_, VerticalStack> {
    pub fn add_child(&mut self, child: WidgetPod<Box<dyn Widget>>) {
        self.widget.children.push(child);
        self.ctx.children_changed();
    }

    pub fn remove_child(&mut self, n: usize) {
        self.widget.children.remove(n);
        self.ctx.children_changed();
    }

    pub fn clear_children(&mut self) {
        self.widget.children.clear();
        self.ctx.children_changed();
    }
}
```

If you want to add or remove a child during other passes, the simplest solution is to use the `mutate_self_later` context method.
That mutate takes a callback, and schedules it to be run with a `WidgetMut` wrapper to the current widget.


## Regular `Widget` methods

Now that we've implemented our container-specific methods, we should also implement the regular `Widget` methods.

In the case of our `VerticalStack`, all of them can be left empty:

```ignore
impl Widget for VerticalStack {
    fn on_pointer_event(&mut self, _ctx: &mut EventCtx, _event: &PointerEvent) {}
    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}
    fn on_access_event(&mut self, _ctx: &mut EventCtx, _event: &AccessEvent) {}

    fn on_anim_frame(&mut self, _ctx: &mut UpdateCtx, _interval: u64) {}
    fn update(&mut self, _ctx: &mut UpdateCtx, _event: &Update) {}

    // ...

    fn paint(&mut self, _ctx: &mut PaintCtx, _scene: &mut Scene) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx, _node: &mut NodeBuilder) {}

    // ...
}
```

This might surprise you: shouldn't our container widget recurse these methods to its children?
Doesn't `VerticalStack::paint` need to call `paint` on its children, for instance?

It doesn't.

In Masonry, most passes are automatically propagated to children, without container widgets having to implement code iterating over their children.

So for instance, if `VerticalStack::children_ids()` returns a list of three children, the paint pass will automatically call `paint` on all three children after `VerticalStack::paint()`.

So various methods in container widgets should only implement the logic that is specific to the container itself.
For instance, a container widget with a background color should implement `paint` to draw the background.
