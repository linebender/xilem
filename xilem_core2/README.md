
<div align="center">

# Xilem Core

</div>

<!-- Close the <div> opened in lib.rs for rustdoc, which hides the above title -->

</div>

<div align="center">

**Reactivity primitives for Rust**

[![Latest published version.](https://img.shields.io/crates/v/xilem_core.svg)](https://crates.io/crates/xilem_core)
[![Documentation build status.](https://img.shields.io/docsrs/xilem_core.svg)](https://docs.rs/xilem_core)
[![Apache 2.0 license.](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](#license)

[![Linebender Zulip chat.](https://img.shields.io/badge/Linebender-%23xilem-blue?logo=Zulip)](https://xi.zulipchat.com/#narrow/stream/354396-xilem)
[![GitHub Actions CI status.](https://img.shields.io/github/actions/workflow/status/linebender/xilem/ci.yml?logo=github&label=CI)](https://github.com/linebender/xilem/actions)
[![Dependency staleness status.](https://deps.rs/crate/xilem_core/latest/status.svg)](https://deps.rs/crate/xilem_core)

</div>

## Quickstart

## Crate feature flags

The following feature flags are available:

* `alloc` (enabled by default): Use the [`alloc`][] crate

## no_std support

Xilem Core supports running with `#![no_std]`, but does require an allocator to be available.
This is because message dispatching uses an open set of messages, which means that `Box<dyn Message>` (a thin wrapper around `Box<dyn Any>` with `Debug` support) must be used.

It is plausible that a version of the `View` trait could be created which does not require this boxing (such as by using a closed set of messages), but that is not provided by this library.
If you wish to use Xilem Core in environments where an allocator is not available, feel free to bring this up on [Zulip](#community).

<!-- MSRV will go here once we settle on that for this repository -->

<!-- We hide these elements when viewing in Rustdoc, because they're not expected to be present in crate level docs -->
<div class="rustdoc-hidden">

## Community

Discussion of Xilem Core development happens in the [Linebender Zulip](https://xi.zulipchat.com/), specifically in
[#xilem](https://xi.zulipchat.com/#narrow/stream/354396-xilem).
All public content can be read without logging in.

Contributions are welcome by pull request. The [Rust code of conduct][] applies.

## License

* Licensed under the Apache License, Version 2.0
  ([LICENSE] or <http://www.apache.org/licenses/LICENSE-2.0>)

</div>

[rust code of conduct]: https://www.rust-lang.org/policies/code-of-conduct

[LICENSE]: LICENSE
[`alloc`]: https://doc.rust-lang.org/stable/alloc/
