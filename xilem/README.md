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
* Widgets are provided by [Masonry], which is a fork of the now discontinued [Druid] UI toolkit.
* Rendering is provided by [Vello], a high performance GPU compute-centric 2D renderer.
* GPU compute infrastructure is provided by [wgpu].
* Text support is provided by [Parley], [Fontique], [Swash], and [Skrifa]. 
* Accessibility is provided by [AccessKit].
* Window handling is provided by [winit].

Xilem can currently be considered to be in an alpha state.
Lots of things need improvements.

## Minimum supported Rust Version (MSRV)

This version of Xilem has been verified to compile with **Rust 1.81** and later.

Future versions of Xilem might increase the Rust version requirement.
It will not be treated as a breaking change and as such can even happen with small patch releases.

<details>
<summary>Click here if compiling fails.</summary>

As time has passed, some of Xilem's dependencies could have released versions with a higher Rust requirement.
If you encounter a compilation issue due to a dependency and don't want to upgrade your Rust toolchain, then you could downgrade the dependency.

```sh
# Use the problematic dependency's name and version
cargo update -p package_name --precise 0.1.1
```

</details>

## Community

Discussion of Xilem development happens in the [Linebender Zulip](https://xi.zulipchat.com/), specifically the [#xilem channel](https://xi.zulipchat.com/#narrow/stream/354396-xilem).
All public content can be read without logging in.

Contributions are welcome by pull request. The [Rust code of conduct] applies.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache 2.0 license, shall be licensed as noted in the [License](#license) section, without any additional terms or conditions.

## License

Licensed under the Apache License, Version 2.0 ([LICENSE](LICENSE) or <http://www.apache.org/licenses/LICENSE-2.0>)

Some files used for examples are under different licenses:

* The font file (`RobotoFlex-Subset.ttf`) in `resources/fonts/roboto_flex/` is licensed solely as documented in that folder (and is not licensed under the Apache License, Version 2.0).
* The data file (`status.csv`) in `resources/data/http_cats_status/` is licensed solely as documented in that folder (and is not licensed under the Apache License, Version 2.0).

Note that these files are *not* distributed with the released crate.

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
