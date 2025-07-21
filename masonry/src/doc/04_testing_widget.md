# Testing widgets in Masonry

<!-- Copyright 2024 the Xilem Authors -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<div class="rustdoc-hidden">

> 💡 Tip
>
> This file is intended to be read in rustdoc.
> Use `cargo doc --open --package masonry_winit --no-deps`.

</div>

Masonry supports testing your UI in a headless setting, with similar capabilities to testing tools which use the [WebDriver](https://developer.mozilla.org/en-US/docs/Web/WebDriver) standard for web dev.

While sometimes you can go a long way keeping your business logic code in pure functions, the best practice is to test your GUI code as well.
You want to write "When I click on this button this widget should appear" into your codebase in a way CI can enforce mechanically.

Enabling this kind of testing is a core design goal of Masonry.

To demonstrate how testing works in Masonry, let's write a test suite for our `ColorRectangle` widget from two chapters ago.


## Creating the harness

First, let's write a test module with a first unit test:

```rust,ignore
// We place this block at the end of the file where we implemented ColorRectangle
#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;
    use masonry::testing::{widget_ids, TestHarness, TestWidgetExt};
    use masonry::theme::default_property_set;

    use super::*;

    #[test]
    fn simple_rect() {
        let [rect_id] = widget_ids();
        let widget = ColorRectangle::new(Size::new(20.0, 20.0), Color::BLUE).with_id(rect_id);

        let harness = TestHarness::create(default_property_set(), widget);

        assert_debug_snapshot!(harness.root_widget());

        // ...
    }
}
```

<!-- TODO - Rewrite this once we have a better way to assign ids to widgets. -->

First, we create a `ColorRectangle` with an arbitrary size and color.
We use `TestWidgetExt::with_id()` to assign it a pre-drawn id.
(As a side-effect, this also wraps our `ColorRectangle` in a [`WrapperWidget`].)

Then we instantiate the [`TestHarness`], with our (wrapped) `ColorRectangle` as the root.

We use `TestHarness::root_widget()` to get a [`WidgetRef`] to our root.

A [`WidgetRef`] is a rich reference to both a widget and its metadata.
We can use it to get a widget's origin, size, flags, etc.

It also has a `Debug` impl, which prints the widget's name and the sub-tree of all its children.
We use that debug impl with `insta::assert_debug_snapshot` to get an easy test checking the widget exists.

<!-- TODO - Remove reference to snapshot testing, replace with accessibility testing. -->


## Screenshot testing

So far our test isn't giving us much information.
We know that `ColorRectangle::new()` doesn't panic, and that's more or less it.

Let's add a visual test:

```rust,ignore
    // ...
    use masonry::assert_render_snapshot;

    #[test]
    fn simple_rect() {
        // ...

        assert_render_snapshot!(harness, "rect_blue_rectangle");
    }
```

The [`assert_render_snapshot!`] macro takes a snapshot name, renders the current state of the app, and stores the rendered image to `<CRATE-ROOT>/screenshots/<TEST-NAME>.png`.

The rendered screenshot is compared against an existing file checked in your project, and panics if the reference file is meaningfully different (with some tolerance for small pixel-by-pixel differences) or if there isn't one.

Adding screenshot tests lets you both check that your widget's `paint()` method runs correctly and explicitly track and check in your widget's visual changes into version control.

That way, if an internal change happens to affect how a widget is displayed, failing screenshot tests will force you to consider whether the visual change is deliberate or an error.

Note that, because all your unit tests and integration tests will send files to the same folder, you may want to use some kind of per-module namespacing scheme to avoid unwanted filename collisions.

<!-- TODO - Include screenshot. -->


## Simulating input

We can also use the harness as if we were a user interacting with a window.
The `TestHarness` types includes methods for mouse events, keyboard events, etc.

Let's create another snapshot test to check that our widget correctly changes color when the mouse hovers it:

```rust,ignore
    // ...

    #[test]
    fn hovered() {
        let [rect_id] = widget_ids();
        let widget = ColorRectangle::new(Size::new(20.0, 20.0), Color::BLUE).with_id(rect_id);

        let mut harness = TestHarness::create(widget);

        // Computes the rect's layout and sends an PointerEvent
        // placing the mouse at its center.
        harness.mouse_move_to(rect_id);
        assert_render_snapshot!(harness, "rect_hovered_rectangle");
    }
```

<!-- TODO - Include screenshot. -->


## Testing `WidgetMut`

In some cases, you may want to run tests where the widget tree is modified after its creation.

Like `RenderRoot`, `TestHarness` has methods that take a closure and, inside of that closure, give you a `WidgetMut` to a specific widget.

Let's add a test that changes a rectangle's color, then checks its visual appearance:

```rust,ignore
    // ...

    #[test]
    fn hovered() {
        let [rect_id] = widget_ids();
        let widget = ColorRectangle::new(Size::new(20.0, 20.0), Color::BLUE).with_id(rect_id);

        let mut harness = TestHarness::create(widget);

        // Computes the rect's layout and sends an PointerEvent
        // placing the mouse at its center.
        harness.mouse_move_to(rect_id);
        assert_render_snapshot!(harness, "rect_hovered_rectangle");
    }

    #[test]
    fn edit_rect() {
        let [rect_id] = widget_ids();
        let widget = ColorRectangle::new(Size::new(20.0, 20.0), Color::BLUE).with_id(rect_id);

        let mut harness = TestHarness::create(widget);

        harness.edit_widget(rect_id |mut rect| {
            let mut rect = rect.downcast::<ColorRectangle>();
            ColorRectangle::set_color(&mut rect, Size::new(50.0, 50.0));
            ColorRectangle::set_size(&mut rect, Color::RED);
        });

        assert_render_snapshot!(harness, "rect_big_red_rectangle");
    }
```

<!-- TODO - Include screenshot. -->


## Testing actions

The `TestHarness` is also capable of reading actions emitted by our widget with the `pop_action()` method.

Since our `WidgetRectangle` doesn't emit actions, let's look at a unit test for the [`Button`] widget instead:

```rust
    #[test]
    fn simple_button() {
        let [button_id] = widget_ids();
        let widget = Button::new("Hello").with_id(button_id);

        let mut harness = TestHarness::create(widget);

        // ...

        harness.mouse_click_on(button_id);
        assert_eq!(
            harness.pop_action::<ButtonPress>(),
            Some((ButtonPress(Some(PointerButton::Primary)), button_id))
        );
    }
```

Overall, this tutorial isn't an exhaustive list of the `TestHarness` API.

In general, `TestHarness` tries to implement methods matching every kind of behavior a user interacting with your app can have, using names that match the natural description of what the user does (e.g. `mouse_click_on`).

Read the [`TestHarness`] documentation for a full overview of its API.

[`Button`]: crate::widgets::Button
[`TestHarness`]: crate::testing::TestHarness
[`WrapperWidget`]: crate::testing::TestHarness
[`WidgetRef`]: crate::core::WidgetRef
[`assert_render_snapshot!`]: crate::testing::assert_render_snapshot
