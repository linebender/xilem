<div align="center">

# Xilem

**An experimental Rust architecture for reactive UI**

[![Latest published version.](https://img.shields.io/crates/v/xilem.svg)](https://crates.io/crates/xilem)
[![Documentation build status.](https://img.shields.io/docsrs/xilem.svg)](https://docs.rs/xilem)
[![Apache 2.0 license.](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](#license)
\
[![Linebender Zulip chat.](https://img.shields.io/badge/Linebender-%23xilem-blue?logo=Zulip)](https://xi.zulipchat.com/#narrow/stream/354396-xilem)
[![GitHub Actions CI status.](https://img.shields.io/github/actions/workflow/status/linebender/xilem/ci.yml?logo=github&label=CI)](https://github.com/linebender/xilem/actions)
[![Dependency staleness status.](https://deps.rs/crate/xilem/latest/status.svg)](https://deps.rs/crate/xilem)

</div>

<!-- We use cargo-rdme to update the README with the contents of lib.rs.
To edit the following section, update it in lib.rs, then run:
cargo rdme --workspace-project=xilem --heading-base-level=0
Full documentation at https://github.com/orium/cargo-rdme -->

<!-- Intra-doc links used in lib.rs should be evaluated here.
See https://linebender.org/blog/doc-include/ for related discussion. -->

[accesskit_docs]: https://docs.rs/accesskit/latest/accesskit
[crate::core::lens]: https://docs.rs/xilem_core/latest/xilem_core/fn.lens.html
[crate::core::memoize]: https://docs.rs/xilem_core/latest/xilem_core/fn.memoize.html
[crate::view::button]: https://docs.rs/xilem/latest/xilem/view/fn.button.html
[crate::view::flex]: https://docs.rs/xilem/latest/xilem/view/fn.flex.html
[crate::view::grid]: https://docs.rs/xilem/latest/xilem/view/fn.grid.html
[crate::view::image]: https://docs.rs/xilem/latest/xilem/view/fn.image.html
[crate::view::portal]: https://docs.rs/xilem/latest/xilem/view/fn.portal.html
[crate::view::progress_bar]: https://docs.rs/xilem/latest/xilem/view/fn.progress_bar.html
[crate::view::prose]: https://docs.rs/xilem/latest/xilem/view/fn.prose.html
[crate::view::sized_box]: https://docs.rs/xilem/latest/xilem/view/fn.sized_box.html
[crate::view::split]: https://docs.rs/xilem/latest/xilem/view/fn.split.html
[crate::view::task]: https://docs.rs/xilem/latest/xilem/view/fn.task.html
[crate::view::textbox]: https://docs.rs/xilem/latest/xilem/view/fn.textbox.html
[crate::view::zstack]: https://docs.rs/xilem/latest/xilem/view/fn.zstack.html
[masonry::parley]: https://docs.rs/parley/latest/parley
[masonry::vello::wgpu]: https://docs.rs/wgpu/latest/wgpu
[masonry::vello]: https://docs.rs/vello/latest/vello/
[xilem_core]: https://docs.rs/parley_core/latest/xilem_core
[xilem_examples]: ./examples/

<!-- markdownlint-disable MD053 -->
<!-- cargo-rdme start -->

`xilem_masonry` provides Xilem views for the Masonry backend.

Xilem is a portable, native UI framework written in Rust.
See [the Xilem documentation](https://docs.rs/xilem/latest/xilem/)
for details.

[Masonry](masonry) is a foundational library for writing native GUI frameworks.

Xilem's architecture uses lightweight view objects, diffing them to provide minimal
updates to a retained UI.

`xilem_masonry` uses Masonry's widget tree as the retained UI.

<!-- cargo-rdme end -->
<!-- markdownlint-enable MD053 -->

## Minimum supported Rust Version (MSRV)

This version of Xilem has been verified to compile with **Rust 1.86** and later.

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

Contributions are welcome by pull request.
The [Rust code of conduct] applies.

## License

Licensed under the Apache License, Version 2.0 ([LICENSE](LICENSE) or <http://www.apache.org/licenses/LICENSE-2.0>)

Some files used for examples are under different licenses:

* The font file (`RobotoFlex-Subset.ttf`) in `resources/fonts/roboto_flex/` is licensed solely as documented in that folder (and is not licensed under the Apache License, Version 2.0).
* The data file (`status.csv`) in `resources/data/http_cats_status/` is licensed solely as documented in that folder (and is not licensed under the Apache License, Version 2.0).
* The data file (`emoji.csv`) in `resources/data/emoji_names/` is licensed solely as documented in that folder (and is not licensed under the Apache License, Version 2.0).

Note that these files are *not* distributed with the released crate.

[Rust code of conduct]: https://www.rust-lang.org/policies/code-of-conduct
