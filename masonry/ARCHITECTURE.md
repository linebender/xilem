# ARCHITECTURE

Masonry is a framework that aims to provide the foundation for Rust GUI libraries.

Developers trying to write immediate-mode GUIs, Elm-architecture GUIs, functional reactive GUIs, etc, can import Masonry and get a platform to create windows (using Winit as a backend) each with a tree of widgets. Each widget has to implement the Widget trait that Masonry provides.


## High-level goals

Masonry has some opinionated design goals:

- **Be dumb.** As a general rule, Masonry doesn't do "algorithms". It has no reconciliation logic, no behind-the-scenes dataflow, no clever optimizations, etc. It tries to be efficient, but that efficiency comes from well-designed interfaces and well-placed abstraction boundaries. High-level logic should be implemented in downstream crates.
- **No mutability tricks.** Masonry uses no unsafe code and as few cells/mutexes as possible. It's designed to work within Rust's ownership system, not to bypass it. While it relies on the `tree_arena` crate in this repository which *does* perform some mutability tricks, the resulting usage patterns are still very rust-like.
- **Facilitate testing.** Masonry implements a `TestHarness` type that helps users write unit tests, including tests with simulated user interactions and screenshot tests. In general, every feature should be designed with easy-to-write high-reliability tests in mind.
- **Facilitate debugging.** GUI app bugs are often easy to fix, but extremely painful to track down. GUI framework bugs are worse. Masonry should facilitate reproducing bugs and pinpointing which bit of code they come from.
- **Provide reflection.** Masonry should help developers surface some of the inherent structure in GUI programs. It should provide tools out-of-the-box to get information about the widget tree, performance indicators, etc. It should also provide accessibility data out-of-the-box.


## Code layout

### `src/core/`

Most widget-related code, including the Widget trait, its context types, event types, and the WidgetRef, WidgetMut, and WidgetPod types.

#### `src/core/widget_state.rs`

Contains the WidgetState type, around which a lot of internal code is based.

WidgetState is one of the most important internal types in Masonry.
Understanding Masonry pass code will likely be easier if you read WidgetState documentation first.

### `src/app/`

Code for creating a Masonry app, including:

- `event_loop_runner.rs` - glue code between Masonry and winit.
- `render_root.rs` - Masonry's composition root. See **General architecture** section.

### `src/passes/`

Masonry's passes are computations that run on the entire widget tree (iff invalidation flags are set) once per frame.

`event.rs` and `update.rs` include a bunch of related passes. Every other file only includes one pass. `mod.rs` has a utility functions shared between multiple passes.

### `src/doc/`

Documentation for the entire crate. In other projects, this would be an `mdbook` doc, but we choose to directly inline the doc, so that `cargo test` runs on it.

### `src/testing/`

Contains the TestHarness type, various helper widgets for writing tests, and the snapshot testing code.

### `src/widgets/`

A list of basic widgets, each defined in a single file.


## Module organization principles

(Some of these principles aren't actually applied in the codebase yet. See <https://github.com/linebender/xilem/issues/367>.)

### Module structure

The public module hierarchy should be relatively flat. This makes documentation more readable; readers don't need to click on multiple modules to find the item they're looking for, and maintainers don't need to open multiple layers of folders to find a file.

### Imports

We should avoid `use super::xxx` imports, except for specific cases like unit tests.

Most imports in the code base should use the canonical top-level import.

### No prelude

Masonry should have no prelude. Examples and documentation should deliberately have a list of imports that cover everything users will need, so users can copy-paste these lists.


## General architecture

### Platform handlers

The composition roots of Masonry are:

- **RenderRoot**, which owns the widget tree.
- The **AppDriver** trait, which owns the business logic.
- The **run_with** function in `event_loop_runner.rs`.

The high-level control flow of a Masonry app's render loop is usually:

- The platform library (windows, macos, x11, etc) runs some callbacks written in Winit in response to user interactions, timers, or other events.
- Winit calls some RenderRoot method.
- RenderRoot runs a series of passes (events, updates, paint, accessibility, etc).
- Throughout those passes, RenderRootSignal values are pushed to a queue which winit can query.
- winit may change its internals or schedule another frame based on those signals.


### Widget hierarchy and passes

The **Widget** trait is defined in `src/widget/widget.rs`. Most of the widget-related bookkeeping is done in the passes defined in `src/passes`.

A **WidgetPod** is the main way to store a child for container widgets. In general, the widget hierarchy looks like a tree of container widgets, with each container owning a WidgetPod or a Vec of WidgetPod or something similar.

When RenderRoot runs a pass (on_xxx_event/update_xxx/paint), it usually iterates over the widget tree in depth-first pre-order, and calls the matching method is called on the root WidgetPod, which recurses to its widget, which calls the same method on each of its children.

There is one exception to this pattern: the layout pass requires widgets to "manually" recurse to their children.

The current passes are:

- **mutate:** Runs a list of callbacks with mutable access to the widget tree. These callbacks can either be passed by the event loop, or pushed to a queue by widgets during the other passes.
- **on_xxx_event:** Handles UX-related events, e.g. clicks, text entered, IME updates and accessibility input. Widgets can declare these events as "handled" which has a bunch of semantic implications.
- **anim:** Do updates related to an animation frame.
- **update:** Handles internal changes to some widgets, e.g. when the widget is marked as "disabled" or Masonry detects that a widget is hovered by a pointer.
- **layout:** Given size constraints, return the widget's size. Container widgets first call `LayoutCtx::run_layout` on their children, then set the position of each child.
- **compose:** Computes the global transform/origin for every widget.
- **paint** Paint every widget.
- **accessibility:** Compute every widget's node in the accessibility tree.

See [masonry/src/doc/05_pass_system.md] for details.


### WidgetMut

In Masonry, widgets can't be mutated directly. All mutations go through a `WidgetMut` wrapper. So, to change a label's text, you might call `WidgetMut<Label>::set_text()`. This helps Masonry make sure that internal metadata is propagated after every widget change.

In general, there's three ways to get a WidgetMut:

- From a WidgetMut to a parent widget.
- As an argument to the callback passed to `RenderRoot::edit_widget()`.
- As an argument to a callback pushed to the mutate pass.

In most cases, the WidgetMut holds a reference to a WidgetState that will be updated when the WidgetMut is dropped.

WidgetMut gives direct mutable access to the widget tree. This can be used by GUI frameworks in their tree update methods, and it can be used in tests to make specific local modifications and test their result.


### Tests

Masonry is designed to make unit tests easy to write, as if the test function were a mouse-and-keyboard user.

Testing is provided by the **TestHarness** type implemented in the `src/testing/harness.rs` file.

Ideally, the harness should provide ways to emulate absolutely every feature that Masonry apps can use. Besides the widget tree, that means keyboard events, mouse events, IME, timers, communication with background threads, animations, accessibility info, etc.

(TODO - Some of that emulation support is not implemented yet. See <https://github.com/linebender/xilem/issues/369>.)

Each widget has unit tests in its module and major features have modules with dedicated unit test suites. Ideally, we would like to achieve complete coverage within the crate.

#### Screenshot tests

TODO - mention kompari

TestHarness can render a widget tree, save the result to an image, and compare the image to a stored snapshot. This lets us check that (1) our widgets' paint methods don't panic and (2) changes don't introduce accidental regression in their visual appearance.

The screenshots are stored using git LFS, which adds some minor complications but avoids the overhead of committing files directly to Git.

We include some of the screenshots in the documentation; because `docs.rs` doesn't have access to LFS files, we use the `include_screenshot!` to instead link to `https://media.githubusercontent.com` when building doc for `docs.rs`.

## VS Code markers

Masonry uses VS Code markers to help users browse code with the minimap:

https://code.visualstudio.com/docs/getstarted/userinterface#_minimap

These markers look like this:

```rust
// --- MARK: MARKER NAME ---
```

By convention, we write them in all caps with three dashes. Markers don't need to follow strict naming conventions, but their names should be a shorthand for the area of the code they're in. Names should be short enough not to overflow the VS Code minimap.

Small files shouldn't have markers, except for files following a general template (widget implementations, view implementations). Generally files should have between 50 and 200 lines between markers. If a file has any markers, it should have enough split the file into distinct regions.
