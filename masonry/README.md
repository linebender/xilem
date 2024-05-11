<div align="center">

# Masonry

**Foundational framework for Rust GUI libraries**

[![Latest published version.](https://img.shields.io/crates/v/masonry.svg)](https://crates.io/crates/masonry)
[![Documentation build status.](https://img.shields.io/docsrs/masonry.svg)](https://docs.rs/masonry)
[![Apache 2.0 license.](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](#license)

[![Linebender Zulip chat.](https://img.shields.io/badge/Linebender-%23masonry-blue?logo=Zulip)](https://xi.zulipchat.com/#narrow/stream/317477-masonry)
[![GitHub Actions CI status.](https://img.shields.io/github/actions/workflow/status/linebender/xilem/ci.yml?logo=github&label=CI)](https://github.com/linebender/xilem/actions)
[![Dependency staleness status.](https://deps.rs/crate/masonry/latest/status.svg)](https://deps.rs/crate/masonry)

</div>

Masonry gives you a platform to create a window (using [winit] as a backend) with a tree of widgets.
It also gives you tools to inspect that widget tree at runtime, write unit tests on it, and generally have an easier time debugging and maintaining your app.

The framework is not opinionated about what your user-facing abstraction will be: you can implement immediate-mode GUI, the Elm architecture, functional reactive GUI, etc, on top of Masonry.
See [Xilem] as an example of reactive UI built on top of Masonry.

Masonry was originally a fork of [Druid] that emerged from discussions with Druid authors Raph Levien and Colin Rofls about what it would look like to turn Druid into a foundational library.

Masonry can currently be considered to be in an alpha state.
Lots of things need improvements, e.g. text input is janky and snapshot testing is not consistent across platforms.

## Community

Discussion of Masonry development happens in the [Linebender Zulip](https://xi.zulipchat.com/), specifically the [#masonry stream](https://xi.zulipchat.com/#narrow/stream/317477-masonry).
All public content can be read without logging in.

Contributions are welcome by pull request. The [Rust code of conduct] applies.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache 2.0 license, shall be licensed as noted in the [License](#license) section, without any additional terms or conditions.

## License

Licensed under the Apache License, Version 2.0 ([LICENSE](LICENSE) or <http://www.apache.org/licenses/LICENSE-2.0>)

[Xilem]: https://crates.io/crates/xilem
[Druid]: https://crates.io/crates/druid
[winit]: https://crates.io/crates/winit
[Rust code of conduct]: https://www.rust-lang.org/policies/code-of-conduct
