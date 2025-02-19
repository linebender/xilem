# Reading Widget Properties

<!-- Copyright 2024 the Xilem Authors -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<div class="rustdoc-hidden">

> 💡 Tip
>
> This file is intended to be read in rustdoc.
> Use `cargo doc --open --package masonry --no-deps`.

</div>

<!-- TODO - Rewrite this chapter -->

**TODO - Add screenshots - see [#501](https://github.com/linebender/xilem/issues/501)**

Throughout the previous chapters, you may have noticed that most Widget methods take a `props: &PropertiesRef<'_>` or `props: &mut PropertiesMut<'_>` argument.
We haven't used these arguments so far, and you can build a robust widget set without them, but they're helpful for making your widgets more customizable and modular.


## What are Properties?

In Masonry, **Properties** (often abbreviated to **props**) are values of arbitrary static types stored alongside each widget.

In simpler terms, that means you can take any non-ref type (e.g. `struct RubberDuck(Color, String, Buoyancy);`) and associate a value of that type to any widget, including widgets of existing types (`Button`, `Checkbox`, `Textbox`, etc) or your own custom widget (`ColorRectangle`).

Code accessing the property will look like:

```rust,ignore
if let Some(ducky) = props.get::<RubberDuck>() {
    let (color, name, buoyancy) = ducky;
    // ...
}
```

### When to use Properties?

<!-- TODO - Mention event handling -->
<!-- I expect that properties will be used to share the same pointer event handling code between Button, SizedBox, Textbox, etc... -->

In practice, properties should mostly be used for styling.

Properties should be defined to represent self-contained values that a widget can have, that are expected to make sense for multiple types of widgets, and where code handling those values should be shared between widgets.

Some examples:

- `BackgroundColor`
- BorderColor
- Padding
- TextFont
- TextSize
- TextWeight

**TODO: Most of the properties cited above do *not* exist in Masonry's codebase. They should hopefully be added quickly.**

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

    fn paint(&mut self, ctx: &mut PaintCtx, props: &PropertiesRef<'_>, scene: &mut Scene) {
        let color = props.get::<BackgroundColor>().unwrap_or(masonry::palette::css::WHITE);
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
