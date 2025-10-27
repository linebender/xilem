# ARCHITECTURE
Xilem is a framework that aims to provide a performant and productive option for Rust GUI.

This document describes the high-level architecture of xilem. If you want to familirize yourself with the code base, you are just in the right place!

## High-level Goals
- **High Performance.** Lightweight state management, retained widget layer, massively parallel rendering backend (Vello), fast startup time, small binary size
- **Productive.** Offer an ergonomic API, code is concise (low Rust tax), idiomatic and UI components compose well
- **Rich 2D graphics model.**
- **Wide platform support.** Windows, macOS, Linux (Wayland/X11), Android (Google funded), iOS
- **Batteries-included.** Advanced text layout, IME, Accessibility, Animation, Styling

## Roadmap
<!-- TODO -->

## Bird's Eye View
The code can be roughly divided into 3 levels:
- Reactive Data Layer
- UI Layer
- Framework Layer

![The Xilem architectural overview](./docs/assets/xilem-architecture.svg)

### Reactive layer (`xilem_masonry`,  `xilem_web` and `xilem_core`)

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

### UI Layer (`masonry`)
The associated Elements of the `View` trait are either DOM nodes for `xilem_web`or implementations of the `Widget` trait.

### Framework Layer (`masonry`)

## Code Organisation
### `xilem_core`
Contains the `View` trait, and other general implementations. Is also contains the `DynMessage`, `MessageResult`, `Id` types and the tree-structrure tracking.

### `xilem_web/`
An implementation of Xilem running on the DOM.

### `masonry/`, `masonry_winit/`
See `ARCHITECTURE.md` file located under `masonry/doc`

## Screenshot tests

Multiple crates in this repository use screenshot tests to ensure the UI renders as expected.

Screenshots are all saved as PNG files in `screenshots/` folders at different places in the repo.
Because of this, we assume that any file matching `**/screenshots/*.png` is a saved screenshot.
