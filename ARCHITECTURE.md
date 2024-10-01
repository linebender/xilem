# ARCHITECTURE
Xilem is a framework that aims to provide a performant and productive option for Rust GUI.

This crates evolved from various experiments in the Linebender community (Druid, masonry, idiopath, lasagna, crochet, etc)

This document describes the high-level architecture of xilem. If you want to familirize yourself with the code base, you are just in the right place!

An evolution of the ideas implemented in xilem can be found in the following resources:
- [Data Oriented GUI (2018)](https://www.youtube.com/watch?v=4YTfxresvS8)
- [A Journey through incremental computation (2020)](https://www.youtube.com/watch?v=DSuX-LIAU-I)
- [Raph Levien on UI Frameworks (2021)](https://www.youtube.com/watch?v=PwuwG2-0n3I)
- [High Performance Rust UI (2022)](https://www.youtube.com/watch?v=zVUTZlNCb8U)
- [Ergonomic APIs for hard problems (2022)](https://www.youtube.com/watch?v=Phk0C-kLlho&t=2706s)
- [Xilem Vector Graphics (2023)](https://www.youtube.com/watch?v=XjbVnwBtVEk)
- [Announcing Masonry 0.1, and my vision for Rust UI (2023)](https://poignardazur.github.io/2023/02/02/masonry-01-and-my-vision-for-rust-ui/)
- [So you want to write a GUI framework (2021)](https://www.cmyr.net/blog/gui-framework-ingredients.html)
- [Rust GUI Infrastructure (2021)](https://www.cmyr.net/blog/rust-gui-infra.html)
- [Towards a unified theory of reactive UI (2019)](https://raphlinus.github.io/ui/druid/2019/11/22/reactive-ui.html)
- [Towards principled reactive UI (2020)](https://raphlinus.github.io/rust/druid/2020/09/25/principled-reactive-ui.html)
- [Xilem: an architecture for UI in Rust (2022)](https://raphlinus.github.io/rust/gui/2022/05/07/ui-architecture.html)
- [Advice for the next dozen Rust GUIs (2022)](https://raphlinus.github.io/rust/gui/2022/07/15/next-dozen-guis.html)

## High-level Goals
- **High Performance.** Lightweight state management, retained widget layer, massively parallel rendering backend (Vello), fast startup time, small binary size
- **Productive.** Offer an ergonomic API, code is concise (low Rust tax), idiomatic and UI components compose well
- **Rich 2D graphics model.**
- **Wide platform support.** Windows, MacOS, Linux (Wayland/X11), Android (Google funded), iOS
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
The most important construct at the reactive layer is the `View` trait. `View` implementations need to assign the associated `State` and `Element` and implement `build()`, `rebuild()` and `message()`. The `build()` is run only once (first run) to initialize the associated `State` create the associated `Element`of the view. The `rebuild()` is used to update the associated `State` and `Element` and it is what enables Xilem's reactivity. It is worth noting three special types of views the `Adapt`, `Memoize` and `AnyView` views. 

The `Adapt` node is used to adapt/convert from the parent data (often the `AppData`) to the child data (some subset of the `AppData`). The `Adapt` node can also adapt messages from the child scope to the parent scope and vice versa.

The `View` trees are always eagerly evaluated when the `app_logic()` is called. Even though the `View`'s are very lightweight when the tree becomes large it can create performance issues; the `Memoize` node prunes the `View` tree to improve performance. The generation of the subtree is postponed until the `build()`/`rebuild()` are executed. If none of the `AppData` dependencies change during a UI cycle neither the `View` subtree will be constructed nor the `rebuild()` will be called. The `View` subtree will only be constructed if any of the `AppData` dependencies change. The `Memoize` node should be used when the subtree is a pure function of the `AppData`.

Generally the Xilem `View` tree is statically typed. `AnyView` allows for the tree to be type erased to enable easy dynamic reconfiguration of the UI. 

### UI Layer (`masonry`)
The associated Elements of the `View` trait are either DOM nodes for `xilem_web`or implementations of the `Widget` trait.

### Framework Layer (`masonry`)

## Code Organisation
### `xilem_core`
Contains the `View` trait, and other general implementations. Is also contains the `Message`, `MessageResult`, `Id` types and the tree-structrure tracking.

### `xilem_web/`
An implementation of Xilem running on the DOM.

### `masonry/`
See `ARCHITECTURE.md` file located under `crates/masonry/doc`
