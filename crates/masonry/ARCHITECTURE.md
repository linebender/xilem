# ARCHITECTURE

Masonry is a framework that aims to provide the foundation for Rust GUI libraries.

Developers trying to write immediate-mode GUIs, Elm-architecture GUIs, functional reactive GUIs, etc, can import Masonry and get a platform to create windows (using Glazier as a backend) each with a tree of widgets. Each widget has to implement the Widget trait that Masonry provides.

This crate was originally a fork of Druid that emerged from discussions I had with Raph Levien and Colin Rofls about what it would look like to turn Druid into a foundational library. This means the code looks very similar to Druid's code and has mostly the same dependencies and primitives.


## High-level goals

Masonry has some opinionated design goals:

- **Be dumb.** As a general rule, Masonry doesn't do "algorithms". It has no reconciliation logic, no behind-the-scenes dataflow, no clever optimizations, etc. It tries to be efficient, but that efficiency comes from well-designed interfaces and well-placed abstraction boundaries. High-level logic should be implemented in downstream crates.
- **No mutability tricks.** Masonry tries to use as little unsafe code and cells/mutexes as possible. It's designed to work within Rust's ownership system, not to bypass it.
- **Facilitate testing.** Masonry implements a `TestHarness` type that helps users write unit tests, including tests with simulated user interactions and screenshot tests. In general, every feature should be designed with easy-to-write high-reliability tests in mind.
- **Facilitate debugging.** GUI app bugs are often easy to fix, but extremely painful to track down. GUI framework bugs are worse. Masonry should facilitate reproducing bugs and pinpointing which bit of code they come from.
- **Provide reflection.** Masonry should help developers surface some of the inherent structure in GUI programs. It should provide tools out-of-the-box to get information about the widget tree, performance indicators, etc. It should also provide accessibility data out-of-the-box.


## Code layout

### `src/platform/`

Some platform-specific stuff, for interfacing with Glazier. Relatively empty.

### `src/testing/`

Contains the TestHarness type, various helper widgets for writing tests, and the snapshot testing code.

### `src/text/`

Contains text-handling code, for both displaying and editing text. Hasn't been maintained in a while, here be dragons.

### `src/widget/`

Contains widget-related items, including the Widget trait, and the WidgetRef, WidgetMut, and WidgetPod types.

Also includes a list of basic widgets, each defined in a single file.

### `src/app_root.rs`

The composition root of the framework. See **General architecture** section.

### `src/debug_logger.rs`, `src/debug_values.rs`

WIP logger to get record of widget passes. See issue #11.

## Module organization principles

(Some of these principles aren't actually applied in the codebase yet. See issue #14 on Github.)

### Module structure

Virtually every module should be private. The only public modules should be inline module blocks that gather public-facing re-exports.

Most items should be exported from the root module, with no other public-facing export. This makes documentation more readable; readers don't need to click on multiple modules to find the item they're looking for.

There should be only three public modules:

- `widgets`
- `commands`
- `test_widgets`

Module files should not be `foobar/mod.rs`. Instead, they should be `_foobar.rs` (thus in the parent folder); the full name is for readability, the leading underscore is so these names appear first in file hierarchies.

### Imports

We should avoid `use super::xxx` imports, except for specific cases like unit tests.

Most imports in the code base should use the canonical top-level import.

### No prelude

Masonry should have no prelude. Examples and documentation should deliberately have a list of imports that cover everything users will need, so users can copy-paste these lists.


## General architecture

### Platform handlers

The composition root of Masonry is **AppRoot** and indirectly **AppRootInner** and **WindowRoot**. They are used by **MasonryWinHandler** and **MasonryAppHandler**. AppRoot, AppRootInner, and WindowRoot are in `src/app_root.rs`, MasonryWinHandler and MasonryAppHandler are in `src/platform/win_handler.rs`.

In more detail:

- **AppRootInner** is the real composition root. There is only a single one for any Masonry program. It calls WindowRoot methods in response to various events.
- **AppRoot** is the publicly exported root. It only owns a`Rc<RefCell<AppRootInner>>` and its methods mostly just do locking and call AppRootInner methods.
- Each window open by Glazier owns a **MasonryWinHandler**, which implements `glazier::WinHandler`. Each MasonryWinHandler holds an AppRoot.
- The global application managed by Glazier owns a **MasonryAppHandler** which implements `glazier::AppHandler`. That MasonryAppHandler holds an AppRoot. These types are almost exclusively used in MacOS apps.
- AppRootInner owns a collection of **WindowRoot**. Each of them stores the Masonry data (widget tree, event data, other metadata) of a single window.

To summarize, an application with N open windows will have:

- N instances of MasonryWinHandler.
- 1 instance of MasonryAppHandler.
- N + 1 instances of AppRoot, which are shared references to...
- 1 instance of AppRootInner, which stores a vec of...
- N instances of WindowRoot.

Additionally, each WindowRoot also stores a **WindowHandle**, which is a lightweight reference to the data structure created by Glazier to represent the window's platform-specific data (eg resource descriptors, rendering surface, etc).

The high-level control flow of a Masonry app's render loop is usually:

- The platform library (windows, x11, gtk, etc) runs some callbacks written in Glazier in response to user interactions, timers, or other events.
- The callbacks call MasonryWinHandler methods.
- Each method calls a single AppRoot method.
- That method calls some AppRootInner method.
- AppRootInner does a bunch of bookkeeping and calls WindowRoot methods.
- WindowRoot does a bunch of bookkeeping and calls the root WidgetPod's on_event/lifecycle/layout/paint methods.

#### WindowConfig vs WindowDescription vs WindowBuilder

WindowConfig, WindowDescription, and WindowBuilder have similar names and similar roles, so it might be a bit hard to tell them apart. A quick primer:

- **WindowConfig** includes a bunch of window-specific metadata. Things like the window size, whether it's maximized, etc. That is used when creating the window, and also to update it.
- **WindowDescription** is WindowConfig plus some Masonry-specific data, such as a WidgetPod of the root widget. It's only used when creating a new window.
- **WindowBuilder** is a type exported by Glazier. When creating a new window, you first instantiate a WindowBuilder and give it config options as well as a type-erased instance of the MasonryWinHandler that will get events for that window.


### Widget hierarchy and passes

The **Widget** trait is defined in `src/widget/widget.rs`. Most of the widget-related bookkeeping is done by the **WidgetPod** type defined in `src/widget/widget_pod.rs`.

A WidgetPod is the main way to store a child for container widgets. In general, the widget hierarchy looks like a tree of container widgets, with each container owning a WidgetPod or a Vec of WidgetPod or something similar. When a pass (on_event/lifecycle/layout/paint) is run on a window, the matching method is called on the root WidgetPod, which recurses to its widget, which calls the same method on each of its children.

Currently, container Widgets are encouraged to call pass methods on each of their children, even when the pass only concerns a single child (eg a click event where only one child is under the mouse); the filtering, if any, is done in WidgetPod.

The current passes are:

- **on_event:** Handles UX-related events, eg user interactions, timers, and IME updates. Widgets can declare these events as "handled" which has a bunch of semantic implications.
- **on_status_change:** TODO.
- **lifecycle:** Handles internal events, eg when the widget is marked as "disabled".
- **layout:** Given size constraints, return the widget's size. Container widgets first call their children's layout method, then set the position and layout date of each child.
- **paint** Paint the widget and its children.

The general pass order is "For each user event, call on_event once, then lifecycle a variable number of times, then schedule a paint. When the platform starts the paint, run layout, then paint".


### WidgetMut

In Masonry, widgets can't be mutated directly. All mutations go through a `WidgetMut` wrapper. So, to change a label's text, you might call `WidgetMut<Label>::set_text()`. This helps Masonry make sure that internal metadata is propagated after every widget change.

Generally speaking, to create a WidgetMut, you need a reference to the parent context that will be updated when the WidgetMut is dropped. That can be the WidgetMut of a parent, an EventCtx / LifecycleCtx, or the WindowRoot. In general, container widgets will have methods such that you can get a WidgetMut of a child from the WidgetMut of a parent.

WidgetMut gives direct mutable access to the widget tree. This can be used by GUI frameworks in their update method, and it can be used in tests to make specific local modifications and test their result.


### Tests

Masonry is designed to make unit tests easy to write, as if the test function were a mouse-and-keyboard user.

Testing is provided by the **TestHarness** type implemented in the `src/testing/harness.rs` file.

Ideally, the harness should provide ways to emulate absolutely every feature that Masonry apps can use. Besides the widget tree, that means keyboard events, mouse events, IME, timers, communication with background threads, animations, accessibility info, etc.

(TODO - Some of that emulation support is not implemented yet. See issue #12.)

Each widget has unit tests in its module and major features have modules with dedicated unit test suites. Ideally, we would like to achieve complete coverage within the crate.

#### Mock timers

For timers in particular, the framework does some special work to simulate the GUI environment.

The GlobalPassCtx types stores two timer handlers: **timers** and **mock_timer_queue**. The first one connects ids returned by the platform's timer creator to widget ids; the second one stores a list of timer values that have to be manually advanced by calling `TestHarness::move_timers_forward`.

When a widget calls `request_timer` in a normal running app, a normal timer is requested from the platform. When a widget calls `request_timer` from a simulated app inside a TestHarness, mock_timer_queue is used instead.

All this means you can have timer-based tests without *actually* having to sleep for the duration of the timer.
