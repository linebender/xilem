# Xilem Masonry

<div align="center">

**Masonry frontend for the Xilem Rust UI framework**

<!-- TODO Add shields for crates.io and docs.rs -->
[![Apache 2.0 license.](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](#license)
\
[![Linebender Zulip chat.](https://img.shields.io/badge/Linebender-%23xilem-blue?logo=Zulip)](https://xi.zulipchat.com/#narrow/stream/354396-xilem)
[![GitHub Actions CI status.](https://img.shields.io/github/actions/workflow/status/linebender/xilem/ci.yml?logo=github&label=CI)](https://github.com/linebender/xilem/actions)
<!-- TODO Add shield for deps.rs -->

</div>

<!-- We use cargo-rdme to update the README with the contents of lib.rs.
To edit the following section, update it in lib.rs, then run:
cargo rdme --workspace-project=xilem_masonry
Full documentation at https://github.com/orium/cargo-rdme -->

<!-- Intra-doc links used in lib.rs should be evaluated here.
See https://linebender.org/blog/doc-include/ for related discussion. -->

[Xilem Core]: https://docs.rs/xilem_core/latest/xilem_core
[Masonry]: https://docs.rs/masonry/latest/masonry

<!-- markdownlint-disable MD053 -->
<!-- cargo-rdme start -->

An implementation of the Xilem architecture (through [Xilem Core][]) using [Masonry][] widgets as Xilem elements.

You probably shouldn't depend on this crate directly, unless you're trying to embed Xilem into a non-Winit platform.
See [Xilem][] or [Xilem Web][] instead.

[Xilem Core]: xilem_core
[Masonry]: masonry
[Xilem]: https://github.com/linebender/xilem/tree/main/xilem
[Xilem Web]: https://github.com/linebender/xilem/tree/main/xilem_web

<!-- cargo-rdme end -->
<!-- markdownlint-enable MD053 -->

## Minimum supported Rust Version (MSRV)

This version of Xilem Masonry has been verified to compile with **Rust 1.88** and later.

Future versions of Xilem Masonry might increase the Rust version requirement.
It will not be treated as a breaking change and as such can even happen with small patch releases.

## Community

Discussion of Xilem Masonry development happens in the [Linebender Zulip](https://xi.zulipchat.com/), specifically the [#xilem channel](https://xi.zulipchat.com/#narrow/stream/354396-xilem) or the [#masonry channel](https://xi.zulipchat.com/#narrow/channel/317477-masonry).
All public content can be read without logging in.

Contributions are welcome by pull request.
The [Rust code of conduct] applies.

## License

Licensed under the Apache License, Version 2.0 ([LICENSE] or <http://www.apache.org/licenses/LICENSE-2.0>)

[Rust code of conduct]: https://www.rust-lang.org/policies/code-of-conduct

<!-- Needs to be defined here for rustdoc's benefit -->
[LICENSE]: LICENSE
