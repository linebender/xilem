<div align="center">

# Masonry Testing

**Headless Test Harness for Masonry**

[![Latest published version.](https://img.shields.io/crates/v/masonry_testing.svg)](https://crates.io/crates/masonry_testing)
[![Documentation build status.](https://img.shields.io/docsrs/masonry_testing.svg)](https://docs.rs/masonry_testing)
[![Apache 2.0 license.](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](#license)
\
[![Linebender Zulip chat.](https://img.shields.io/badge/Linebender-%23masonry-blue?logo=Zulip)](https://xi.zulipchat.com/#narrow/channel/317477-masonry)
[![GitHub Actions CI status.](https://img.shields.io/github/actions/workflow/status/linebender/xilem/ci.yml?logo=github&label=CI)](https://github.com/linebender/xilem/actions)
[![Dependency staleness status.](https://deps.rs/crate/masonry_testing/latest/status.svg)](https://deps.rs/crate/masonry_testing)

</div>

<!-- We use cargo-rdme to update the README with the contents of lib.rs.
To edit the following section, update it in lib.rs, then run:
cargo rdme --workspace-project=masonry_testing
Full documentation at https://github.com/orium/cargo-rdme -->

<!-- Intra-doc links used in lib.rs should be evaluated here.
See https://linebender.org/blog/doc-include/ for related discussion. -->

<!-- cargo-rdme start -->

Headless runner for testing [Masonry](https://docs.rs/masonry/latest/) applications.

The primary type from this crate is [`TestHarness`], which creates a host for any [Widget].
The widget can of course have children, which allows this crate to be used for testing entire applications.

The testing harness can:

- Simulate any external event which Masonry handles, including mouse movement, key presses, text input, accessibility events.
- Control the flow of time to the application (i.e. for testing animations).
- Take screenshots of the application, save these to a file, and ensure that these are up-to-date.
  See [Screenshots](#screenshots) for more details.

<!-- Masonry itself depends on Masonry Testing, so we can't use an intra-doc link here. -->
Testing in Masonry is also documented in the [Testing widgets in Masonry](https://docs.rs/masonry/latest/masonry/doc/doc_04_testing_widget/index.html)
chapter in Masonry's book.

This crate can be accessed for applications using Masonry as `masonry::testing`, if Masonry's `testing` feature is enabled.
For applications which are using only [Masonry Core](masonry_core), you should depend on `masonry_testing` directly.

## Screenshots

Tests using `TestHarness` can include snapshot steps by using the [`assert_render_snapshot`] screenshot.
This renders the application being tested, then compares it against the png file with the given name
from the `screenshots` folder (in the package being tested, i.e. adjacent to its `Cargo.toml` file).

Masonry Testing will update the reference file when the `MASONRY_TEST_BLESS` environment variable has a value of `1`.
This can be used if the file doesn't exist, or there's an expected difference.
The screenshots are losslessly compressed (using [oxipng]) and limited to a small maximum file size (this
limit has an escape hatch).
This ensures that the screenshots are small enough to embed in a git repository with limited risk
of clone times growing unreasonably.
UI screenshots compress well, so we expect this to be scalable.

For repositories hosted on GitHub, this scheme also allows for including screenshots of your app or
widgets in hosted documentation, although we haven't documented this publicly yet.

## Examples

For examples of this crate in use

- To test applications: see the tests in Masonry's examples.
- To test widgets: see the `tests` module in each widget in Masonry.

## Feature flags

The following crate [feature flags](https://doc.rust-lang.org/cargo/reference/features.html#dependency-features) are available:

- `default`: Enables the default features of [`masonry_core`][masonry_core].

[masonry_core]: https://crates.io/crates/masonry_core

<!-- cargo-rdme end -->

## Minimum supported Rust Version (MSRV)

This version of Masonry Testing has been verified to compile with **Rust 1.88** and later.

Future versions of Masonry Testing might increase the Rust version requirement.
It will not be treated as a breaking change and as such can even happen with small patch releases.

## Community

Discussion of Masonry Testing development happens in the [Linebender Zulip](https://xi.zulipchat.com/), specifically the [#masonry channel](https://xi.zulipchat.com/#narrow/channel/317477-masonry).
All public content can be read without logging in.

Contributions are welcome by pull request.
The [Rust code of conduct] applies.

## License

Licensed under the Apache License, Version 2.0 ([LICENSE](LICENSE) or <http://www.apache.org/licenses/LICENSE-2.0>)

[Rust code of conduct]: https://www.rust-lang.org/policies/code-of-conduct
