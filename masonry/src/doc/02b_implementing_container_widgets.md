
## Container widgets

Widgets can own other widgets.

Container widgets need to worry about a few more methods:

```rust
trait Widget {
    // ...

    fn register_children(&mut self, ctx: &mut RegisterCtx);
    fn layout(&mut self, ctx: &mut LayoutCtx) -> Size;
    fn compose(&mut self, ctx: &mut ComposeCtx);

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]>;
}
```

(Well, non-container widgets need to implement them too, but these implementations are trivial.)

Container widgets have the following additional constraints:

- Their `register_children` method must call `RegisterCtx::register_child` for each child.
- Their `layout` method must call `LayoutCtx::run_layout` and `LayoutCtx::place_child` for each child.
- Their `children_ids` method must return the IDs of all their children. That list is considered the "canonical" list of children by Masonry, and must match the children visited during `register_children` and `layout`.



- `register_children`, `layout` and `compose` are called when Masonry is trying to figure out the structure of the widget tree. They're especially relevant for container widgets, as give the framework information about their children.


    fn register_children(&mut self, ctx: &mut RegisterCtx);
    fn layout(&mut self, ctx: &mut LayoutCtx) -> Size;
    fn compose(&mut self, ctx: &mut ComposeCtx);


## Editing the widget tree

Widgets can be added and removed during event and rewrite passes *except* inside layout and register_children methods.

Not doing so is a logic error and may trigger debug assertions.

If you do want to add or remove a child during layout, you can always defer it with the `mutate_later` context method.
