# ARCHITECTURE

This repository holds the source code for the Xilem project and the Masonry project, including their sub-crates.

- Xilem is a family of high-level GUI frameworks. Xilem apps are written with idiomatic Rust code, with little to no reliance on macros and DSLs.
    - **`xilem_core`** includes the traits that define Xilem.
    - **`xilem_masonry`** is the natively compiled framework, built on Masonry.
    - **`xilem`** is a batteries-included wrapper for `xilem_masonry` using `winit` for platform support.
    - **`xilem_web`** is the web framework, built on the DOM.
- Masonry is a foundational framework for building high-level Rust GUI libraries.
    - **`masonry_core`** includes the base GUI engine.
    - **`masonry_testing`** includes a harness, helper macros and functions, etc, for testing apps builts with Masonry.
    - **`masonry`** includes a baseline set of widgets and properties, a default theme, unit tests for widgets, and unit tests for `masonry_core`.
    - **`masonry_winit`** is the winit backend.

See [xilem/ARCHITECTURE.md](./xilem/ARCHITECTURE.md) and [masonry/ARCHITECTURE.md](./masonry/ARCHITECTURE.md) for more details on each project.

This repo also holds `tree_arena`, a crate which implements a hierarchical container, which has some properties of a tree (given a mutable reference to a node, you can get disjoint references to its value and children), while allowing `O(1)` (in unsafe mode) access to any element.

```mermaid
graph TD
%%   subgraph Masonry
    masonry[Masonry]
    masonry_core[Masonry Core]
%%   end
    %% subgraph Masonry Drivers
        masonry_testing[Masonry Testing]
        masonry_winit[Masonry Winit]
        masonry_android_view[Masonry Android View]
    %% end
%%   subgraph Xilem
    xilem[Xilem]
    xilem_masonry[Xilem Masonry]
    xilem_core[Xilem Core]
    xilem_web[Xilem Web]
%%   end
%%   subgraph Vello
        vello[Vello]
        vello_encoding[Vello Encoding]
        vello_shaders[Vello Shaders]
        peniko[Peniko]
%%   end
%%   subgraph Parley
    parley[Parley]
    fontique[Fontique]
%%   end
tree_arena[Tree Arena]
%%   subgraph Font Infrastructure
    linebender_resource_handle[Linebender Resource Handle]
    skrifa["Skrifa (Google Fonts)"]
    swash[Swash]
    harfrust["HarfRust (HarfBuzz)"]
%%   end

color[Color]
kurbo[Kurbo]
web_sys["web-sys (wasm-bindgen)"]

wgpu["Wgpu (gfx-rs)"]

fontique --> linebender_resource_handle
masonry-->masonry_core
masonry-- (dev) -->masonry_testing
masonry_testing-->masonry_core

masonry_android_view --> masonry_core
masonry_winit --> masonry_core
harfrust --> skrifa
swash --> skrifa
parley --> swash
parley --> harfrust
parley --> fontique

peniko --> color
peniko --> kurbo

vello --> vello_shaders
vello_shaders --> vello_encoding
vello --> vello_encoding
vello_encoding --> skrifa
vello --> wgpu

vello --> peniko
vello --> linebender_resource_handle

xilem_web --> web_sys
xilem_web --> xilem_core
masonry_core -->  vello
masonry_core -->  parley
masonry_core -->  tree_arena

xilem_masonry --> xilem_core
xilem_masonry --> masonry

xilem --> xilem_masonry
xilem --> masonry_winit
```