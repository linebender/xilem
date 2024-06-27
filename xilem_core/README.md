
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

## no_std support

Xilem Core supports running with `#![no_std]`, but does require an allocator to be available.

It is plausible that this reactivity pattern could be used without allocation being required, but that is not provided by this package.
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

- Licensed under the Apache License, Version 2.0
  ([LICENSE] or <http://www.apache.org/licenses/LICENSE-2.0>)

</div>

[rust code of conduct]: https://www.rust-lang.org/policies/code-of-conduct

[LICENSE]: LICENSE
[Xilem]: https://crates.io/crates/xilem
[Xilem Web]: https://crates.io/crates/xilem_web
[xilem docs]: https://docs.rs/xilem/latest/xilem/
[`memoize`]: https://docs.rs/xilem_core/latest/xilem_core/views/memoize/fn.memoize.html
[`View`]: https://docs.rs/xilem_core/latest/xilem_core/view/trait.View.html
