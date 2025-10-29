# ARCHITECTURE

Xilem is a family of high-level GUI frameworks.
Xilem apps are written with idiomatic Rust code, with little to no reliance on macros and DSLs.


## General architecture

The most important thing about Xilem is that it is a *reactive* architecture:

- After every change, user-provided functions are called to generate a **view tree**, a lightweight representation of the app's UI.
- The new view tree is compared against the previous view tree.
- Based on the differences, the back-end creates an updates a retained **element tree**.
  Elements are added, removed or mutated to match the current view tree.
    - In `xilem`, the element tree is the Masonry widget tree.
    - in `xilem_web`, the element tree is the DOM.

This architecture is strongly inspired by React, Elm and SwiftUI, which work on similar principles.

A Xilem app's code will usually be made of functions called **components** that create and return subsets of the view tree.
A component function might look like this pseudo-code:

```rust
fn list_item(item: &ItemData) -> impl View<...> {
    row((
        image(item.image_url),
        text(item.text),
        button("Do thing").on_click(|...| do_thing(item.id)),
    ))
}
```


## High-level Goals

- **Good Performance:** Xilem aims for the "performant by default" class of Rust software. 
  Everything isn't always optimized, but it uses sober data structures and high-performance dependencies (Vello, Parley, wgpu).
  Static typing gives us very efficient diffing of view trees by default.
- **Productive:** Xilem offers a high-level API, designed to bypass the borrowing and state management issues that retained GUIs suffer from in Rust.
- **Idiomatic:** Xilem apps are low on macros or DSLs.
  Writing a component is about creating and composing view objects, no special syntax needed.
- **Batteries-included:** Through Masonry, Xilme includes advanced text layout, IME, accessibility trees and styling.
- **Wide platform support:** Xilem supports the web through `xilem_web` and desktop/mobile platforms through `masonry` and `winit`.


## Code layout

The Xilem project includes these crates:

- **`xilem_core`:** Includes the core traits such as `View`, `ViewElement`, `ViewSequence`, `ElementSlice`, and many others. Also include some generic implementations for these traits (e.g. for Box, Arc, tuples, etc).
- **`xilem_web`:** Web backend for Xilem. Depends on `xilem_core` and `wasm-bindgen`.
- **`xilem`:** Natively compiled backend for Xilem. Depends on `xilem_core`, `masonry` and `masonry_winit`.


## Writing Xilem code

**TODO - This section needs a rewrite.**

Your main interaction with the framework is through the `app_logic()`. Like Elm, the `app_logic()` contains centralized state. On each cycle (meaning, roughly, on each high-level UI interaction such as a button click), the framework calls a closure, giving it mutable access to the `AppData`, and the return value is a `View` tree. This `View` tree is fairly short-lived; it is used to render the UI, possibly dispatch some events, and be used as a reference for diffing by the next cycle, at which point it is dropped. You pass the `app_logic()` to the framework (represented by `Xilem`) and then use it to create the window and run the UI.

```rust
struct AppData {
    count: u32,
}

fn app_logic(data: &mut AppData) -> impl View<AppData, (), Element = impl Widget> {
    let count = data.count
    let button_label = if count == 1 {
        "clicked 1 time".to_string()
    } else {
        format!("clicked {count} times")
    };
    button(button_label, |data: &mut AppData| data.count +=1)
}

fn main() {
    let data = AppData {
      count: 0
    }
    let app = Xilem::new(data, app_logic);
    app.run_windowed("Application Example".into()).unwrap();
}
```
The most important construct at the reactive layer is the `View` trait. `View` implementations need to assign the associated `State` and `Element` and implement `build()`, `rebuild()` and `message()`. The `build()` is run only once (first run) to initialize the associated `State` create the associated `Element`of the view. The `rebuild()` is used to update the associated `State` and `Element` and it is what enables Xilem's reactivity. It is worth noting three special types of views the `lens`, `Memoize` and `AnyView` views. 

The `lens` view is used to adapt/convert from the parent data (often the `AppData`) to the child data (some subset of the `AppData`).

The `View` trees are always eagerly evaluated when the `app_logic()` is called. Even though the `View`'s are very lightweight when the tree becomes large it can create performance issues; the `Memoize` node prunes the `View` tree to improve performance. The generation of the subtree is postponed until the `build()`/`rebuild()` are executed. If none of the `AppData` dependencies change during a UI cycle neither the `View` subtree will be constructed nor the `rebuild()` will be called. The `View` subtree will only be constructed if any of the `AppData` dependencies change. The `Memoize` node should be used when the subtree is a pure function of the `AppData`.

Generally the Xilem `View` tree is statically typed. `AnyView` allows for the tree to be type erased to enable easy dynamic reconfiguration of the UI. 
