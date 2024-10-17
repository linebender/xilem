(TODO - I'm rewriting this doc, shouldn't mention passes at all)

If you're building your own GUI framework on top of Masonry, or even a GUI app with specific needs, you'll probably want to invent your own widgets.

This documentation explains how.


## The Widget trait

Widgets are types which implement the `masonry::Widget` trait.

This trait includes a set of methods that must be implemented to hook into Masonry's internals:

```rust
trait Widget {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent);
    fn on_text_event(&mut self, ctx: &mut EventCtx, event: &TextEvent);
    fn on_access_event(&mut self, ctx: &mut EventCtx, event: &AccessEvent);

    fn on_anim_frame(&mut self, ctx: &mut UpdateCtx, interval: u64);
    fn update(&mut self, ctx: &mut UpdateCtx, event: &Update);

    fn layout(&mut self, ctx: &mut LayoutCtx) -> Size;

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene);
    fn accessibility(&mut self, ctx: &mut AccessCtx, node: &mut NodeBuilder);

    // ...
}
```

These methods are called by the framework at various points, with a `FoobarCtx` parameter giving information about the current widget (for example, its size, position, or whether it's currently hovered).
The information accessible from the context type depends on the method.

In the course of a frame, Masonry will run a series of passes over the widget tree, which will call these methods at different points:

- `on_pointer_event`, `on_text_event` and `on_access_event` are called once after a user-initiated event (like a mouse click or keyboard input).
- `on_anim_frame` is called once per frame for animated widgets.
- `update` is called many times during a frame, with various events reflecting changes in the widget's state (for instance, it gets or loses focus).
- `layout` is called during Masonry's layout pass. It takes size constraints and returns the widget's desired size.
- `paint`, `accessibility` are called roughly every frame for every widget, to allow them to draw to the screen and describe their structure to assistive technologies.

Most passes will skip most widgets by default.
For instance, the paint pass will only call a widget's `paint` method once, and then cache the resulting scene.
If your widget's appearance is changed by another method, you need to call `ctx.request_render()` to tell the framework to re-run the paint pass.

Most context types include these methods for requesting future passes:

- `request_render()`
- `request_paint_only()`
- `request_accessibility_update()`
- `request_layout()`
- `request_compose()`
- `request_anim_frame()`


## Widget mutation

In Masonry, widgets generally can't be mutated directly.
That is to say, even if you own a window, and even if that window holds a widget tree with a `Label` instance, you can't get a `&mut Label` directly from that window.

Instead, there are two ways to mutate `Label`:

- Inside a Widget method. Most methods (`on_pointer_event`, `update`, `layout`, etc) take `&mut self`.
- Through a `WidgetMut` wrapper. So, to change the label's text, you will call `WidgetMut<Label>::set_text()`. This helps Masonry make sure that internal metadata is propagated after every widget change.

TODO - edit_widget

TODO - mutate_later


## Example widget: ColorRectangle
