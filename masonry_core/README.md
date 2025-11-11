<div align="center">

# Masonry Core

**Foundational headless GUI engine of Masonry**

[![Latest published version.](https://img.shields.io/crates/v/masonry_core.svg)](https://crates.io/crates/masonry_core)
[![Documentation build status.](https://img.shields.io/docsrs/masonry_core.svg)](https://docs.rs/masonry_core)
[![Apache 2.0 license.](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](#license)
\
[![Linebender Zulip chat.](https://img.shields.io/badge/Linebender-%23masonry-blue?logo=Zulip)](https://xi.zulipchat.com/#narrow/channel/317477-masonry)
[![GitHub Actions CI status.](https://img.shields.io/github/actions/workflow/status/linebender/xilem/ci.yml?logo=github&label=CI)](https://github.com/linebender/xilem/actions)
[![Dependency staleness status.](https://deps.rs/crate/masonry_core/latest/status.svg)](https://deps.rs/crate/masonry_core)

</div>

<!-- We use cargo-rdme to update the README with the contents of lib.rs.
To edit the following section, update it in lib.rs, then run:
cargo rdme --workspace-project=masonry_core
Full documentation at https://github.com/orium/cargo-rdme -->

<!-- Intra-doc links used in lib.rs should be evaluated here.
See https://linebender.org/blog/doc-include/ for related discussion. -->
[ui_events]: https://crates.io/crates/ui-events
[vello]: https://crates.io/crates/vello
[accesskit]: https://crates.io/crates/accesskit

[core::Widget::Action]: https://docs.rs/masonry_core/latest/masonry_core/core/widget/trait.Widget.html#associatedtype.Action
[core::Widget]: https://docs.rs/masonry_core/latest/masonry_core/core/widget/trait.Widget.html
[core::WidgetMut]: https://docs.rs/masonry_core/latest/masonry_core/core/widget_mut/struct.WidgetMut.html
[doc::pass_system]: https://docs.rs/masonry_core/latest/masonry_core/doc/internals_01_pass_system/index.html

<!-- markdownlint-disable MD053 -->
<!-- cargo-rdme start -->

Masonry Core provides the base GUI engine for Masonry.

Masonry's widgets are implemented in the Masonry crate, which re-exports this crate as `masonry::core`.
Most users who wish to use Masonry for creating applications (and UI libraries) should
prefer to depend on Masonry directly (i.e. the `masonry` crate).
[Masonry's documentation] can be found on docs.rs.

Masonry Core provides:

- [`Widget`][core::Widget], the trait for GUI widgets in Masonry.
- Event handling and bubbling, using types from [`ui-events`][ui_events] for interoperability.
- Communication between parent and child widgets for layout.
- Compositing of widget's content (to be rendered using [Vello][vello]).
- Creation of accessibility trees using [Accesskit][accesskit].
- APIs for widget manipulation (such as [`WidgetMut`][core::WidgetMut]).
- The [`Action`][core::Widget::Action] mechanism by which widgets send events to the application.

Details of many of these can be found in the [Pass System][doc::pass_system] article.

If you're writing a library in the Masonry ecosystem, you should depend on `masonry_core`
directly where possible (instead of depending on `masonry`).
This will allow applications using your library to have greater compilation parallelism.
Cases where this apply include:

- Writing an alternative driver for Masonry (alike to [Masonry Winit][]).
- Witing a library containing one or more custom widget (such as a 2d mapping widget).

Masonry Core can also be used by by applications wishing to not use Masonry's provided
set of widgets, so as to have more control.
This can be especially useful if you wish to exactly match the appearance of an existing library,
or enforce following a specific design guide, which Masonry's widgets may not always allow.
Masonry Core provides a useful shared set of functionality to implement alternative widget libraries.
Note that Masonry Core is currently focused primarily on the main Masonry crate itself, as we're
not aware of any projects using Masonry Core as described in this paragraph.

## Feature flags

The following crate [feature flags](https://doc.rust-lang.org/cargo/reference/features.html#dependency-features) are available:

- `default`: Enables the default features of [Vello][vello].
- `tracy`: Enables creating output for the [Tracy](https://github.com/wolfpld/tracy) profiler using [`tracing-tracy`][tracing_tracy].
  This can be used by installing Tracy and connecting to a Masonry with this feature enabled.

[Masonry's documentation]: https://docs.rs/masonry/latest/
[Masonry Winit]: https://docs.rs/masonry_winit/latest/
[tracing_tracy]: https://crates.io/crates/tracing-tracy

<!-- cargo-rdme end -->
<!-- markdownlint-enable MD053 -->

## Minimum supported Rust Version (MSRV)

This version of Masonry Core has been verified to compile with **Rust 1.88** and later.

Future versions of Masonry Core might increase the Rust version requirement.
It will not be treated as a breaking change and as such can even happen with small patch releases.

## Community

Discussion of Masonry Core development happens in the [Linebender Zulip](https://xi.zulipchat.com/), specifically the [#masonry channel](https://xi.zulipchat.com/#narrow/channel/317477-masonry).
All public content can be read without logging in.

Contributions are welcome by pull request.
The [Rust code of conduct] applies.

## License

Licensed under the Apache License, Version 2.0 ([LICENSE] or <http://www.apache.org/licenses/LICENSE-2.0>)

[Rust code of conduct]: https://www.rust-lang.org/policies/code-of-conduct

<!-- Needs to be defined here for rustdoc's benefit -->
[LICENSE]: LICENSE
