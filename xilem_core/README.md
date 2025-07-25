# Xilem Core

<div align="center">

**Reactivity primitives for Rust**

[![Latest published version.](https://img.shields.io/crates/v/xilem_core.svg)](https://crates.io/crates/xilem_core)
[![Documentation build status.](https://img.shields.io/docsrs/xilem_core.svg)](https://docs.rs/xilem_core)
[![Apache 2.0 license.](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](#license)
\
[![Linebender Zulip chat.](https://img.shields.io/badge/Linebender-%23xilem-blue?logo=Zulip)](https://xi.zulipchat.com/#narrow/stream/354396-xilem)
[![GitHub Actions CI status.](https://img.shields.io/github/actions/workflow/status/linebender/xilem/ci.yml?logo=github&label=CI)](https://github.com/linebender/xilem/actions)
[![Dependency staleness status.](https://deps.rs/crate/xilem_core/latest/status.svg)](https://deps.rs/crate/xilem_core)

</div>

<!-- We use cargo-rdme to update the README with the contents of lib.rs.
To edit the following section, update it in lib.rs, then run:
cargo rdme --workspace-project=xilem_core --heading-base-level=0
Full documentation at https://github.com/orium/cargo-rdme -->

<!-- Intra-doc links used in lib.rs should be evaluated here.
See https://linebender.org/blog/doc-include/ for related discussion. -->

[`alloc`]: https://doc.rust-lang.org/stable/alloc/
[`memoize`]: https://docs.rs/xilem_core/latest/xilem_core/views/memoize/fn.memoize.html
[`View`]: https://docs.rs/xilem_core/latest/xilem_core/view/trait.View.html
[Zulip]: #community

<!-- cargo-rdme start -->

Xilem Core provides primitives which are used by [Xilem][] (a cross-platform GUI toolkit) and [Xilem Web][] (a web frontend framework).
If you are using Xilem, [its documentation][xilem docs] will probably be more helpful for you. <!-- TODO: In the long-term, we probably also need a book? -->

Xilem apps will interact with some of the functions from this crate, in particular [`memoize`][].
Xilem apps which use custom widgets (and therefore must implement custom views), will implement the [`View`][] trait.

If you wish to implement the Xilem pattern in a different domain (such as for a terminal user interface), this crate can be used to do so.
Though, while Xilem Core should be able to support all kinds of domains, the crate prioritizes the ergonomics for users of Xilem.

## Hot reloading

Xilem Core does not currently include infrastructure to enable hot reloading, but this is planned.
The current proposal would split the application into two processes:

 - The app process, which contains the app state and create the views, which would be extremely lightweight and can be recompiled and restarted quickly.
 - The display process, which contains the widgets and would be long-lived, updating to match the new state of the view tree provided by the app process.

## Quickstart

<!-- TODO? -->

## `no_std` support

Xilem Core supports running with `#![no_std]`, but does use [`alloc`][] to be available.

It is plausible that this reactivity pattern could be used without allocation being required, but that is not provided by this package.
If you wish to use Xilem Core in environments where an allocator is not available, feel free to bring this up on [Zulip][Zulip].

[Xilem]: https://crates.io/crates/xilem
[Xilem Web]: https://crates.io/crates/xilem_web
[xilem docs]: https://docs.rs/xilem/latest/xilem/
[Zulip]: https://xi.zulipchat.com/#narrow/stream/354396-xilem

<!-- cargo-rdme end -->

## Minimum supported Rust Version (MSRV)

This version of Xilem Core has been verified to compile with **Rust 1.88** and later.

Future versions of Xilem Core might increase the Rust version requirement.
It will not be treated as a breaking change and as such can even happen with small patch releases.

## Community

Discussion of Xilem Core development happens in the [Linebender Zulip](https://xi.zulipchat.com/), specifically the [#xilem channel](https://xi.zulipchat.com/#narrow/stream/354396-xilem).
All public content can be read without logging in.

Contributions are welcome by pull request.
The [Rust code of conduct] applies.

## License

Licensed under the Apache License, Version 2.0 ([LICENSE] or <http://www.apache.org/licenses/LICENSE-2.0>)

[Rust code of conduct]: https://www.rust-lang.org/policies/code-of-conduct

<!-- Needs to be defined here for rustdoc's benefit -->
[LICENSE]: LICENSE
