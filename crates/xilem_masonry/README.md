<div align="center">

# Xilem

**An experimental Rust architecture for reactive UI**

[![Latest published version.](https://img.shields.io/crates/v/xilem.svg)](https://crates.io/crates/xilem)
[![Documentation build status.](https://img.shields.io/docsrs/xilem.svg)](https://docs.rs/xilem)
[![Apache 2.0 license.](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](#license)

[![Linebender Zulip chat.](https://img.shields.io/badge/Linebender-%23xilem-blue?logo=Zulip)](https://xi.zulipchat.com/#narrow/stream/354396-xilem)
[![GitHub Actions CI status.](https://img.shields.io/github/actions/workflow/status/linebender/xilem/ci.yml?logo=github&label=CI)](https://github.com/linebender/xilem/actions)
[![Dependency staleness status.](https://deps.rs/crate/xilem/latest/status.svg)](https://deps.rs/crate/xilem)

</div>

Xilem is a UI toolkit.
It combines ideas from Flutter, SwiftUI, and Elm.
Like all of these, it uses lightweight view objects, diffing them to provide minimal updates to a retained UI.
Like SwiftUI, it is strongly typed.
For more details on Xilem's reactive architecture see [Xilem: an architecture for UI in Rust].

Xilem's reactive layer is built on top of a wide array of foundational Rust UI projects, e.g.:
* Widgets are provided by [Masonry], which is a fork of [Druid].
* Rendering is provided by [Vello], a high performance GPU compute-centric 2D renderer.
* GPU compute infrastructure is provided by [wgpu].
* Text support is provided by [Parley], [Fontique], [Swash], and [Skrifa]. 
* Accessibility is provided by [AccessKit].
* Window handling is provided by [winit].

Xilem can currently be considered to be in an alpha state.
Lots of things need improvements.

## Community

Discussion of Xilem development happens in the [Linebender Zulip](https://xi.zulipchat.com/), specifically the [#xilem stream](https://xi.zulipchat.com/#narrow/stream/354396-xilem).
All public content can be read without logging in.

Contributions are welcome by pull request. The [Rust code of conduct] applies.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache 2.0 license, shall be licensed as noted in the [License](#license) section, without any additional terms or conditions.

## License

Licensed under the Apache License, Version 2.0 ([LICENSE](LICENSE) or <http://www.apache.org/licenses/LICENSE-2.0>)

[Masonry]: https://crates.io/crates/masonry
[Druid]: https://crates.io/crates/druid
[Vello]: https://crates.io/crates/vello
[wgpu]: https://crates.io/crates/wgpu
[Parley]: https://crates.io/crates/parley
[Fontique]: https://crates.io/crates/fontique
[Swash]: https://crates.io/crates/swash
[Skrifa]: https://crates.io/crates/skrifa
[AccessKit]: https://crates.io/crates/accesskit
[winit]: https://crates.io/crates/winit
[Xilem: an architecture for UI in Rust]: https://raphlinus.github.io/rust/gui/2022/05/07/ui-architecture.html
[Rust code of conduct]: https://www.rust-lang.org/policies/code-of-conduct
