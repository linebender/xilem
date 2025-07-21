# Reading widget properties

<!-- Copyright 2024 the Xilem Authors -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<div class="rustdoc-hidden">

> 💡 Tip
>
> This file is intended to be read in rustdoc.
> Use `cargo doc --open --package masonry_winit --no-deps`.

</div>

<!-- TODO - Rewrite this chapter -->

**TODO - Add screenshots - see [#501](https://github.com/linebender/xilem/issues/501)**

Throughout the previous chapters, you may have noticed that most widget methods take a `props: &PropertiesRef<'_>` or `props: &mut PropertiesMut<'_>` argument.
We haven't used these arguments so far, and you can build a robust widget set without them, but they're helpful for making your widgets more customizable and modular.


## What are properties?

In Masonry, **properties** (often abbreviated to **props**) are values of arbitrary static types stored alongside each widget.

In simpler terms, that means you can create any non-reference type (e.g. `struct RubberDuck(Color, String, Buoyancy);`), give it the [`Property`] marker trait, and associate a value of that type to any widget, including widgets of existing types (`Button`, `Checkbox`, `TextInput`, etc) or your own custom widget (`ColorRectangle`).

Code accessing the property will look like:

```rust,ignore
let ducky = props.get::<RubberDuck>();
let RubberDuck(color, name, buoyancy) = ducky;
// ...
```

### Properties are only data

Properties are a way for widgets to store arbitrary state; they do not encode *behavior*.
For those familiar, properties are similar to the "Component" part of ECS.

In other words, adding a property to a widget will not change anything about how that widget is rendered *unless that widget has code specifically reading that property*.

Because arbitrary properties can be set on arbitrary widgets, that means it's perfectly possible to add a `BorderColor` property to a `MyBorderlessBox` widget.
This will not do anything, **not even log a warning message**.

We acknowledge that this may be a footgun in some cases, though we consider it an acceptable trade-off to keep the design simple.
We may reconsider this in the future.


### When to use properties?

<!-- TODO - Mention event handling -->
<!-- I expect that properties will be used to share the same pointer event handling code between Button, SizedBox, TextInput, etc... -->

In practice, properties should mostly be used for styling.

Properties should be defined to represent self-contained values that a widget can have, that are expected to make sense for multiple types of widgets, and where code handling those values should be shared between widgets.

Some examples:

- [`Background`]
- [`BorderColor`]
- [`BorderWidth`]
- [`Padding`]
- [`TextColor`]

Properties should *not* be used to represent an individual widget's state. The following should *not* be properties:

- Text contents.
- Cached values.
- A checkbox's status.

<!-- TODO - Mention properties as a unit of code sharing, once we have concrete examples of that. -->


## Using properties in `ColorRectangle`

With that in mind, let's rewrite our `ColorRectangle` widget to use properties:

```rust,ignore
use masonry::properties::BackgroundColor;

impl Widget for ColorRectangle {
    // ...

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        let color = props.get::<BackgroundColor>();
        let rect = ctx.size().to_rect();
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

## Setting properties in `WidgetMut`

The most idiomatic way to set properties is through `WidgetMut`:

```rust,ignore
let color_rectangle_mut: WidgetMut<ColorRectangle> = ...;

let bg = BackgroundColor { color: masonry::palette::css::BLUE };

color_rectangle_mut.insert_prop(bg);
```

This code will set the given rectangle's `BackgroundColor` (replacing the old one if there was one) to blue.

You can set as many properties as you want.
Properties are an associative map, where types are the keys.

But setting a property to a given value doesn't change anything by default, unless your widget code specifically reads that value and does something with it.

<!-- TODO - Mention "transform" property. -->

[`Property`]: crate::core::Property
[`Background`]: crate::properties::Background
[`BorderColor`]: crate::properties::BorderColor
[`BorderWidth`]: crate::properties::BorderWidth
[`Padding`]: crate::properties::Padding
[`TextColor`]: crate::properties::TextColor
