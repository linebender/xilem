**TODO - This is copy-pasted from ARCHITECTURE.md, needs to be edited.**

### WidgetMut

In Masonry, widgets can't be mutated directly. All mutations go through a `WidgetMut` wrapper. So, to change a label's text, you might call `WidgetMut<Label>::set_text()`. This helps Masonry make sure that internal metadata is propagated after every widget change.

Generally speaking, to create a WidgetMut, you need a reference to the parent context that will be updated when the WidgetMut is dropped. That can be the WidgetMut of a parent, an EventCtx / LifecycleCtx, or the WindowRoot. In general, container widgets will have methods such that you can get a WidgetMut of a child from the WidgetMut of a parent.

WidgetMut gives direct mutable access to the widget tree. This can be used by GUI frameworks in their update method, and it can be used in tests to make specific local modifications and test their result.


**TODO - This is copy-pasted from the pass spec RFC, needs to be edited.**

## Editing the widget tree

Widgets can be added and removed during event and rewrite passes *except* inside layout and register_children methods.

Not doing so is a logic error and may trigger debug assertions.

If you do want to add or remove a child during layout, you can always defer it with the `mutate_later` context method.


## Widget methods and context types

Widgets are types which implement the `masonry::Widget` trait.

This trait includes a set of methods that must be implemented to hook into the different passes listed above:

```rust
// Exact signatures may differ
trait Widget {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent);
    fn on_text_event(&mut self, ctx: &mut EventCtx, event: &TextEvent);
    fn on_access_event(&mut self, ctx: &mut EventCtx, event: &AccessEvent);

    fn register_children(&mut self, ctx: &mut RegisterCtx);
    fn update(&mut self, ctx: &mut UpdateCtx, event: &UpdateEvent);
    fn layout(&mut self, ctx: &mut LayoutCtx) -> Size;
    fn compose(&mut self, ctx: &mut ComposeCtx);

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene);
    fn accessibility(&mut self, ctx: &mut AccessCtx);

    // ...
}
```

These methods all take a given context type as a parameter.
Methods aside, `WidgetMut` references can provide a `MutateCtx` context.
`WidgetRef` references can provide a `QueryCtx` context, which is used in some read-only methods.

Those context types have many methods, some shared, some unique to a given pass.
There are too many to document here, but we can lay out some general principles:

- Render passes should be pure and can be skipped occasionally, therefore their context types (`PaintCtx` and `AccessCtx`) can't set invalidation flags or send signals.
- The `layout` and `compose` passes lay out all widgets, which are transiently invalid during the passes, therefore `LayoutCtx`and `ComposeCtx` cannot access the size and position of the `self` widget.
They can access the layout of children if they have already been laid out.
- For the same reason, `LayoutCtx`and `ComposeCtx` cannot create a `WidgetRef` reference to a child.
- `MutateCtx`, `EventCtx` and `UpdateCtx` can let you add and remove children.
- `RegisterCtx` can't do anything except register children.
- `QueryCtx` provides read-only information about the widget.

