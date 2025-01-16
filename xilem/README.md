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

<!-- We use cargo-rdme to update the README with the contents of lib.rs.
To edit the following section, update it in lib.rs, then run:
cargo rdme --workspace-project=color --heading-base-level=0
Full documentation at https://github.com/orium/cargo-rdme -->

<!-- Intra-doc links used in lib.rs should be evaluated here.
See https://linebender.org/blog/doc-include/ for related discussion. -->

[`flex`]: https://docs.rs/xilem/latest/xilem/view/fn.flex.html
[`grid`]: https://docs.rs/xilem/latest/xilem/view/fn.grid.html
[`sized box`]: https://docs.rs/xilem/latest/xilem/view/fn.sized_box.html
[`button`]: https://docs.rs/xilem/latest/xilem/view/fn.button.html
[`checkbox`]: https://docs.rs/xilem/latest/xilem/view/fn.checkbox.html
[`image`]: https://docs.rs/xilem/latest/xilem/view/fn.image.html
[`label`]: https://docs.rs/xilem/latest/xilem/view/fn.label.html
[`portal`]: https://docs.rs/xilem/latest/xilem/view/fn.portal.html
[`progress bar`]: https://docs.rs/xilem/latest/xilem/view/fn.progress_bar.html
[`prose`]: https://docs.rs/xilem/latest/xilem/view/fn.prose.html
[`spinner`]: https://docs.rs/xilem/latest/xilem/view/fn.spinner.html
[`task`]: https://docs.rs/xilem/latest/xilem/view/fn.task.html
[`textbox`]: https://docs.rs/xilem/latest/xilem/view/fn.textbox.html
[`variable label`]: https://docs.rs/xilem/latest/xilem/view/fn.variable_label.html
[`zstack`]: https://docs.rs/xilem/latest/xilem/view/fn.zstack.html
[weight]: https://docs.rs/parley/latest/parley/style/struct.FontWeight.html

<!-- cargo-rdme start -->

`Xilem` is a UI toolkit. It combines ideas from `Flutter`, `SwiftUI`, and `Elm`.
Like all of these, it uses lightweight view objects, diffing them to provide
minimal updates to a retained UI. Like `SwiftUI`, it is strongly typed. For more
details on `Xilem`'s reactive architecture see `Xilem`: an [architecture for UI in Rust].

`Xilem`'s reactive layer is built on top of a wide array of foundational Rust UI projects, e.g.:

* Widgets are provided by [Masonry], which is a fork of the now discontinued `Druid` UI toolkit.
* Rendering is provided by [Vello], a high performance GPU compute-centric 2D renderer.
* GPU compute infrastructure is provided by wgpu.
* Text support is provided by [Parley], [Fontique], [swash], and [skrifa].
* Accessibility is provided by [AccessKit].
* Window handling is provided by [winit].

`Xilem` can currently be considered to be in an alpha state. Lots of things need improvements.

### Example
The simplest app looks like this:
```rust
use winit::error::EventLoopError;
use xilem::view::{button, flex, label};
use xilem::{EventLoop, WidgetView, Xilem};

#[derive(Default, Debug)]
struct AppState {
    num: i32,
}

fn app_logic(data: &mut AppState) -> impl WidgetView<AppState> {
    flex((label(format!("{}", data.num)), button("increment", |data: &mut AppState| data.num+=1)))
}

fn main() -> Result<(), EventLoopError> {
    let app = Xilem::new(AppState::default(), app_logic);
    app.run_windowed(EventLoop::with_user_event(), "Counter".into())?;
    Ok(())
}
```
More examples available [here](https://github.com/linebender/xilem/tree/main/xilem/examples).

### View elements
The primitives your `Xilem` app’s view tree will generally be constructed from:
- [`flex`]: layout defines how items will be arranged in rows or columns.
- [`grid`]: layout divides a window into regions and defines the relationship
  between inner elements in terms of size and position.
- [`lens`]: an adapter which allows using a component which only uses one field
  of the current state.
- [`map action`]: provides a message that the parent view has to handle
  to update the state.
- [`adapt`]: the most flexible but also most verbose way to modularize the views
  by state and action.
- [`sized box`]: forces its child to have a specific width and/or height.
- [`button`]: basic button element.
- [`checkbox`]: an element which can be in checked and unchecked state.
- [`image`]: displays the bitmap `image`.
- [`label`]: a non-interactive text element.
- [`portal`]: a view which puts `child` into a scrollable region.
- [`progress bar`]: progress bar element.
- [`prose`]: displays immutable text which can be selected within.
- [`spinner`]: can be used to display that progress is happening on some process.
- [`task`]: launch a task which will run until the view is no longer in the tree.
- [`textbox`]: The textbox widget displays text which can be edited by the user.
- [`variable label`]: displays non-editable text, with a variable [weight].
- [`zstack`]: an element that lays out its children on top of each other.

[architecture for UI in Rust]: https://raphlinus.github.io/rust/gui/2022/05/07/ui-architecture.html
[winit]: https://crates.io/crates/winit
[Druid]: https://crates.io/crates/druid
[Masonry]: https://crates.io/crates/masonry
[Vello]: https://crates.io/crates/vello
[Parley]: https://crates.io/crates/parley
[Fontique]: https://crates.io/crates/fontique
[swash]: https://crates.io/crates/swash
[skrifa]: https://crates.io/crates/skrifa
[AccessKit]: https://crates.io/crates/accesskit
[`flex`]: https://docs.rs/xilem/latest/xilem/view/flex/
[`grid`]: https://docs.rs/xilem/latest/xilem/view/grid/
[`lens`]: core::lens
[`map state`]: core::map_state
[`map action`]: core::map_action
[`adapt`]: core::adapt
[`sized box`]: https://docs.rs/xilem/latest/xilem/view/sized_box/
[`button`]: https://docs.rs/xilem/latest/xilem/view/button/
[`checkbox`]: https://docs.rs/xilem/latest/xilem/view/checkbox/
[`image`]: https://docs.rs/xilem/latest/xilem/view/image/
[`label`]: https://docs.rs/xilem/latest/xilem/view/label/
[`portal`]: https://docs.rs/xilem/latest/xilem/view/portal/
[`progress bar`]: https://docs.rs/xilem/latest/xilem/view/progress_bar/
[`prose`]: https://docs.rs/xilem/latest/xilem/view/prose/
[`spinner`]: https://docs.rs/xilem/latest/xilem/view/spinner/
[`task`]: https://docs.rs/xilem/latest/xilem/view/task/
[`textbox`]: https://docs.rs/xilem/latest/xilem/view/textbox/
[`variable label`]: https://docs.rs/xilem/latest/xilem/view/variable_label/
[`zstack`]: https://docs.rs/xilem/latest/xilem/view/zstack/
[weight]: masonry::FontWeight 

<!-- cargo-rdme end -->

## Minimum supported Rust Version (MSRV)

This version of Xilem has been verified to compile with **Rust 1.82** and later.

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

[Xilem: an architecture for UI in Rust]: https://raphlinus.github.io/rust/gui/2022/05/07/ui-architecture.html
[Rust code of conduct]: https://www.rust-lang.org/policies/code-of-conduct
