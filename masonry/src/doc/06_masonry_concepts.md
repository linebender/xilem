# Concepts and definitions

<!-- Copyright 2024 the Xilem Authors -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<div class="rustdoc-hidden">

> 💡 Tip
>
> This file is intended to be read in rustdoc.
> Use `cargo doc --open --package masonry --no-deps`.

</div>

This section describes concepts mentioned by name elsewhere in the documentation and gives them a semi-formal definition for reference.

## Widget status

The notion of widget status is somewhat vague, but you can think of it as similar to [CSS pseudo-classes](https://developer.mozilla.org/en-US/docs/Web/CSS/Pseudo-classes).

Widget statuses are "things" managed by Masonry that affect how widgets are presented.
Statuses include:

- Being hovered.
- Having pointer capture.
- Having active text focus.
- Having inactive text focus.
- Being disabled.
- Being stashed.

When one of these statuses changes, the `update` method is called on the widget.
However, `update` can be called for reasons other than status changes.


## Pointer capture

When a user starts a pointer click on a widget, the widget can "capture" the pointer.

Pointer capture has a few implications:

- When a widget has captured a pointer, all events from that pointer will be sent to the widget, even if the pointer isn't in the widget's hitbox.
Conversely, no other widget can get events from the pointer (outside of bubbling).
- The "hovered" status of other widgets won't be updated even if the pointer is over them.
The hovered status of the capturing widget will be updated, meaning a widget that captured a pointer can still lose the "hovered" status.
- The pointer's cursor icon will be updated as if the pointer stayed over the capturing widget.
- If the widget loses pointer capture for some reason (e.g. the pointer is disconnected), the Widget will get a [`PointerLost`] event.

Masonry should guarantee that pointers can only be captured by one widget at a time.
Masonry should force the widget to lose pointer capture when some events occur; not just MouseLeave, but also `Tab` being pressed, the window losing focus, the widget being disabled, etc.

Examples of use cases for pointer capture include selecting text, dragging a slider, or long-pressing a button.


## Text focus

Focus marks whether a widget receives text events.

To give a simple example, when you click a textbox, the textbox gets focus: anything you type on your keyboard will be sent to that textbox.

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


## Disabled

A disabled widget is one which is visibly marked as non-interactive.

It is usually grayed out, and can't receive pointer or text events.


## Stashed

A stashed widget is one which is no longer "part of the logical tree", so to speak.

Stashed widgets can't receive keyboard or pointer events, don't get painted, aren't part of the accessibility tree, but should still keep some state.

The stereotypical stashed widget would be one inside a hidden tab in a "tab group" widget.

By contrast, widgets scrolled outside the viewport are **not** stashed: they can still get text events and are part of the accessibility tree.


## Interactivity

A widget is considered "interactive" if it can still get text and/or pointer events.
Stashed and disabled widget are non-interactive.


## Properties / Props

All widgets have associated data of arbitrary types called "properties".
These properties are mostly used for styling and event handling.

See [Reading Widget Properties](crate::doc::doc_04b_widget_properties) for more info.


## Bounding rect

A widget's bounding rect is a window-space axis-aligned rectangle inside of which pointer events might affect either the widget or its descendants.

In general, the bounding rect is a union or a widget's layout rect and the bounding rects of all its descendants.

The bounding rects of the widget tree form a kind of "bounding volume hierarchy": when looking to find which widget a pointer is on, Masonry will automatically exclude any widget if the pointer is outside its bounding rect.

<!-- TODO - Include illustration. -->

<!-- TODO - Add section about clip paths and pointer detection. -->


## Layout rect

Previous versions of Masonry had a concept of a widget's "layout rect", composed of its self-declared size and the position attributed by its parent.

However, given that widgets can have arbitrary transforms, the concept of an axis-aligned layout rect doesn't really make sense anymore.


## Safety rails

When debug assertions are on, Masonry runs a bunch of checks every frame to make sure widget code doesn't have logical errors.

These checks are sometimes referred to as "safety rails".

Safety rails aren't guaranteed to run and may be disabled even in debug mode for performance reasons.
They should not be relied upon to check code correctness, but are meant to help you catch implementation errors early on during development.

[`PointerLost`]: crate::core::PointerEvent::PointerLost
[`FocusChanged`]: crate::core::Update::FocusChanged
[`Widget::accepts_focus`]: crate::core::Widget::accepts_focus
[`EventCtx::request_focus`]: crate::core::EventCtx::request_focus
[`Widget::on_pointer_event`]: crate::core::Widget::on_pointer_event
