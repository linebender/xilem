# Concepts and definitions

<!-- Copyright 2024 the Xilem Authors -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<div class="rustdoc-hidden">

> 💡 Tip
>
> This file is intended to be read in rustdoc.
> Use `cargo doc --open --package masonry --no-deps` and open the `doc` module.

</div>

This section describes concepts mentioned by name elsewhere in the documentation and gives them a semi-formal definition for reference.

## Widget status

The notion of widget status is somewhat vague, but you can think of it as similar to [CSS pseudo-classes](https://developer.mozilla.org/en-US/docs/Web/CSS/Pseudo-classes).

Widget statuses are "things" managed by Masonry that affect how widgets are presented.
Statuses include:

- Being hovered.
- Being active.
- Having pointer capture.
- Having active text focus.
- Having inactive text focus.
- Being disabled.
- Being stashed.

When one of these statuses changes, the `update` method is called on the widget.
However, `update` can be called for reasons other than status changes.


## Hovered

A widget is "hovered" when a pointer is placed in the widget's hitbox, but not in the hitbox of its children.


## Pointer capture

When a user starts a pointer click on a widget, the widget can "capture" the pointer.

Pointer capture has a few implications:

- When a widget has captured a pointer, all events from that pointer will be sent to the widget, even if the pointer isn't in the widget's hitbox.
Conversely, no other widget can get events from the pointer (outside of bubbling).
- The "hovered" status of other widgets won't be updated even if the pointer is over them.
The hovered status of the capturing widget will be updated, meaning a widget that captured a pointer can still lose the "hovered" status.
- The pointer's cursor icon will be updated as if the pointer stayed over the capturing widget.
- If the widget loses pointer capture for some reason (e.g. the pointer is disconnected), the widget will get a [`Cancel`] event.

Masonry should guarantee that pointers can only be captured by one widget at a time.
Masonry should force the widget to lose pointer capture when some events occur; not just "mouse leave", but also `Tab` being pressed, the window losing focus, the widget being disabled, etc.

Examples of use cases for pointer capture include selecting text, dragging a slider, or long-pressing a button.


## Active

An "active" widget is one that the user is currently interacting with.

This is similar to the `:active` CSS pseudo-class, though Masonry doesn't guarantee it behaves the same.

Currently, a widget is determined to be active when it has pointer capture, though that definition may change in the future, either by making active status and pointer capture orthogonal, or by adding some pointer-agnostic interactions that will make a widget active, such as keyboard selection or accessibility inputs.

Interactive widgets (e.g. buttons) should have a way to indicate when they are active.


## Text focus

Focus marks whether a widget receives text events.

To give a simple example, when you click a text input, the text input gets focus: anything you type on your keyboard will be sent to that text input.

Focus will be changed:

- When users press the Tab key: Masonry will automatically pick the next widget in the tree that accepts focus [`Widget::accepts_focus`]. (If no widget is currently focused, its starting point will be the most recently clicked widget.)
- When users click outside the currently focused widget: Masonry will automatically remove focus.

Widgets that want to gain focus when clicked should call [`EventCtx::request_focus`] inside [`Widget::on_pointer_event`].
Other context types can also request focus.

If a widget gains or loses focus it will get a [`FocusChanged`] event.

Note that widgets without text-edition capabilities such as buttons and checkboxes can also get focus.
For instance, pressing space when a button is focused will trigger that button.

There are two types of focus: active and inactive focus.
Active focus is the default one; inactive focus is when the window your app runs in has lost focus itself.

In that case, we still mark the widget as focused, but with a different color to signal that e.g. typing on the keyboard won't actually affect it.

### Focus fallback

Masonry drivers have the option to give set a widget as the "focus fallback".

In that case, if no widget is focused, text events will get to the fallback widget instead.

The focus fallback isn't considered as "focused" and will not get [`FocusChanged`] events or be visually marked as focused.


## Disabled

A disabled widget is one which is made non-interactive, and should affect the state of the application.

For an example the decrease button of a counter of type `usize` should be disabled if the value is `0`.

A disabled widget cannot have active status, cannot get or keep text focus, and cannot get text events (except [`Ime::Disabled`]).
It cannot get pointer events (except [`PointerEvent::Cancel`]) or have hovered status either.
Its pointer icon will be the default one.

<!-- TODO: What about accessibility events? -->

(Note: The above is not how browsers handle disabled form inputs, but it matches how most frameworks handle disable widgets.)

While a widget is marked as disabled, all its children are automatically considered disabled.

Interactive widgets (e.g. buttons) should have a way to indicate when they are disabled, usually by showing a grayed-out appearance.


## Stashed

A stashed widget is one which is no longer "part of the logical tree", so to speak.

Stashed widgets can't receive keyboard or pointer events, don't get painted, aren't part of the accessibility tree, but should still keep some state.

The stereotypical stashed widget would be one inside a hidden tab in a "tab group" widget.

By contrast, widgets scrolled outside the viewport are **not** stashed: they can still get text events and are part of the accessibility tree.


## Interactivity

A widget is considered "interactive" if it can still get text and/or pointer events.
Stashed and disabled widget are non-interactive.


## Focus anchor

When the user presses `Tab` or `Shift+Tab`, Masonry will look for the closest sibling of the focus anchor which accepts focus, and focus it.

The focus anchor is generally either the focused widget, or the most recently clicked widget.

What happens if the focus anchor is removed from the tree or stashed/disabled is currently unspecified.

<!-- TODO - Ideally, the closest ancestor widget still interactive should become the new focus anchor in all three cases. -->

This behavior exists so that when the user clicks somewhere and then presses `Tab`, the focused widget is more likely to be close to whatever the user clicked.


## Properties / Props

All widgets have associated data of arbitrary types called "properties".
These properties are mostly used for styling and event handling.


## Box model

The box model refers to the following box hierarchy:

* Content-box - Contains only the widget's content.
* Border-box - Contains the widget's borders, padding, and its content-box.
* Paint-box - Contains the widget's painting, i.e. its border-box and any overflowing painting.
* Bounding-box - Contains the widget's and all of its descendants' clipped paint-boxes.

### Box lifecycle

The box lifecycle terms describe what stage a box is in during its journey from idea to painting.

1. **Preferred** - The size that a widget wishes to be and is the result of `LayoutCtx::compute_size`.
2. **Chosen** - The size that the parent of a widget ends up choosing for it and is given to `LayoutCtx::run_layout`.
3. **Layout** - The result of the chosen size being potentially adjusted to meet min/max constraints.
   For example, if the parent gave a size too small to even contain the child's borders and padding.
4. **Aligned** - Once a parent places its child to a specific position, that position will be aligned to the pixel grid.
   This alignment is done in the parent's border-box coordinate space using the child's layout border-box size.
5. **Effective** - The actual visual box that gets painted on the screen.
   This is the result when all of the transforms of the widget's tree branch are applied to its aligned box.

### Presence of descendants

Only the bounding-box is guaranteed to contain the widget's descendants.
The paint-box, border-box, and content-box may contain them only by chance.
As the descendants may be overflowing these bounds or a transformation may move them out completely.

### Bounding-box

We only calculate the effective variant of the bounding-box, i.e. where all transforms have been applied.
The effective bounding-box is a union of the widget's effective paint-box and the bounding-boxes of all of its descendants.
Additionally, these are clipped according to the per-widget clip rules.

This effective bounding-box in the window's coordinate space is used to determine which pointer events might affect either the widget or its descendants.

The bounding-boxes of the widget tree form a kind of "bounding volume hierarchy": when looking to find which widget a pointer is on, Masonry will automatically exclude any widget if the pointer is outside its bounding-box.

<!-- TODO - Include illustration. -->


## Coordinate spaces

All `Widget` method implementations operate in that widget's content-box coordinate space.
Which means that `(0, 0)` refers to the top-left point where padding ends and content begins.
This is easy to reason in for the widget specific operations.
The widget box can be assumed to be a simple rectangle and Masonry hides all the complicated transforms.

Internally Masonry also operates in the widget's border-box coordinate space, but this is generally hidden from widgets.
The difference compared to the content-box coordinate space is a simple border and padding based translation.

Finally there is the window's coordinate space.
Here all widgets have their transforms already applied so widget specific operations are complicated.
Generally you'll want to convert any window coordinate space geometry into the widget's content-box coordinate space.
Then easily operate on that geometry and finally convert the results back to the window's coordinate space.


## Clip shape

Widgets have a shape, usually one that matches their visual appearance, which has two purposes:

- Pointer events outside of that shape will not affect the pointer.
- If the widget is set to clip its contents, pointer events outside the clip shape won't affect the children either.
- If the widget is set to clip its contents, its scene and the children's scenes will be painted inside of the clip shape.

Currently, the clip shape is hardcoded to be the layout rect.

<!-- TODO: Rename to "widget shape" instead? -->
<!-- Need a better name. -->
<!-- TODO: Better integrate with box model documentation. -->


## Layers

A Masonry application is composed of layers.

Layers are top-level items in a [`RenderRoot`], drawn on top of each other.

There is always at least one layer, called the "base layer".
It's the one in which almost all content (buttons, texts, images) will be drawn.

Other layers can represent tooltips, menus, dialogs, etc.
They are created with a pre-set position and are drawn on top of the base layer.

### Adding a layer

<!-- TODO - Flesh this out -->

Most context methods have a `create_layer(layer_type, fallback_widget, pos)` method.

`layer_type` and `fallback_widget` are two redundant representations of the same layer:

- `layer_type` represents the layer's "semantic" content, as an enum with variants for common layer types.
- `fallback_widget` represents the layer's "visual" content, as a widget which should be drawn at the root of the new layer.

These two values are sent to the Masonry driver running the app; if the driver has built-in behavior for the given `layer_type`, this behavior will be used.
Otherwise, the driver will add a new layer to the current [`RenderRoot`] with `fallback_widget` as its root.


## Safety rails

When debug assertions are on, Masonry runs a bunch of checks every frame to make sure widget code doesn't have logical errors.

These checks are sometimes referred to as "safety rails".

Safety rails aren't guaranteed to run and may be disabled even in debug mode for performance reasons.
They should not be relied upon to check code correctness, but are meant to help you catch implementation errors early on during development.


## BiDi handling

Masonry currently doesn't have any special-case behavior for RTL (right-to-left) and vertical writing modes.

That means there is no easy way to set some "leading", "trailing", "inline", "block", etc, values and have them resolve to different directions depending on whether your audience uses European / Asian / other writing systems.

Handling writing modes is in-scope for Masonry in the long term, but is deferred for now.
We will probably need to implement other features before we can handle it properly, such as style cascading.


## Pixel snapping

Masonry currently handles pixel snapping for its widgets.

The basic idea is that when widgets are laid out, Masonry takes their reported sizes and positions, and rounds them to integer values, so that the drawn shapes line up with pixels.

This is done "at the end" of the layout pass, so to speak, so that widgets can lay themselves out assuming a floating point coordinate space, and without worrying about rounding errors.

The snapping is done in a way that preserves relations between widgets: if one widget ends precisely where another stops, Masonry will pick values so that their pixel-snapped layout rects have no gap or overlap.

<!-- TODO - Remove this note once https://github.com/linebender/xilem/issues/1264 is implemented. -->
**Note:** This may produce incorrect results with DPI scaling.
DPI-aware pixel snapping is a future feature.


[`Cancel`]: ui_events::pointer::PointerEvent::Cancel
[`PointerEvent::Cancel`]: ui_events::pointer::PointerEvent::Cancel
[`Ime::Disabled`]: crate::core::Ime::Disabled
[`FocusChanged`]: crate::core::Update::FocusChanged
[`Widget::accepts_focus`]: crate::core::Widget::accepts_focus
[`EventCtx::request_focus`]: crate::core::EventCtx::request_focus
[`Widget::on_pointer_event`]: crate::core::Widget::on_pointer_event
[`RenderRoot`]: crate::app::RenderRoot
