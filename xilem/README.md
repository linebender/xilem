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
[crate::view::text_input]: https://docs.rs/xilem/latest/xilem/view/fn.text_input.html
[crate::view::zstack]: https://docs.rs/xilem/latest/xilem/view/fn.zstack.html
[masonry::parley]: https://docs.rs/parley/latest/parley
[masonry::vello::wgpu]: https://docs.rs/wgpu/latest/wgpu
[masonry::vello]: https://docs.rs/vello/latest/vello/
[xilem_core]: https://docs.rs/parley_core/latest/xilem_core
[xilem_examples]: ./examples/

<!-- markdownlint-disable MD053 -->
<!-- cargo-rdme start -->

Xilem is a UI toolkit. It combines ideas from `Flutter`, `SwiftUI`, and `Elm`.
Like all of these, it uses lightweight view objects, diffing them to provide
minimal updates to a retained UI. Like `SwiftUI`, it is strongly typed.

The talk *[Xilem: Let's Build High Performance Rust UI](https://www.youtube.com/watch?v=OvfNipIcRiQ)* by Raph Levien
was presented at the RustNL conference in 2024, and gives a video introduction to these ideas.
Xilem is implemented as a reactive layer on top of [Masonry][masonry], a widget toolkit which is developed alongside Xilem.
Masonry itself is built on top of a wide array of foundational Rust UI projects:

* Rendering is provided by [Vello][masonry::vello], a high performance GPU compute-centric 2D renderer.
* GPU compute infrastructure is provided by [wgpu][masonry::vello::wgpu].
* Text layout is provided by [Parley][masonry::parley].
* Accessibility is provided by [AccessKit][] ([docs][accesskit_docs]).
* Window handling is provided by [winit][].

Xilem can currently be considered to be in an alpha state. Lots of things need improvements (including this documentation!).

There is also a [blog post][xilem_blog] from when Xilem was first introduced.

## Example

A simple incrementing counter application looks like:

```rust
use winit::error::EventLoopError;
use xilem::view::{button, flex, label};
use xilem::{EventLoop, WindowOptions, WidgetView, Xilem};

#[derive(Default)]
struct Counter {
    num: i32,
}

fn app_logic(data: &mut Counter) -> impl WidgetView<Counter> + use<> {
    flex((
        label(format!("{}", data.num)),
        button("increment", |data: &mut Counter| data.num += 1),
    ))
}

fn main() -> Result<(), EventLoopError> {
    let app = Xilem::new_simple(Counter::default(), app_logic, WindowOptions::new("Counter app"));
    app.run_in(EventLoop::builder())?;
    Ok(())
}
```

A key feature of Xilem's architecture is that the application's state, in this case `Counter`, is an arbitrary `'static` Rust type.
In this example, `app_logic` is the root component, which creates the view value it returns.
This, in turn, leads to corresponding Masonry widgets being created, in this case a button and a label.
When the button is pressed, the number will be incremented, and then `app_logic` will be re-ran.
The returned view will be compared with its previous value, which will minimally update the contents of these widgets.
As the `num` field's value has changed, the `label`'s formatted text will be different.
This means that the label widget's text will be updated, updating the value displayed to the user.
In this case, because the button is the same, it will not be updated.

More examples can be found [in the repository][xilem_examples].

**Note: The linked examples are for the `main` branch of Xilem. If you are using a released version, please view the examples in the tag for that release.**

## Reactive layer

The core concepts of the reactive layer are explained in [Xilem Core][xilem_core].

## View elements

The primitives your `Xilem` app’s view tree will generally be constructed from:

* [`flex`][crate::view::flex]: defines how items will be arranged in a row or column
* [`grid`][crate::view::grid]: divides a window into regions and defines the relationship
  between inner elements in terms of size and position
* [`sized_box`][crate::view::sized_box]: forces its child to have a specific width and/or height
* [`split`][crate::view::split]: contains two views splitting the area either vertically or horizontally which can be resized.
* [`button`][crate::view::button]: basic button element
* [`image`][crate::view::image]: displays a bitmap image
* [`portal`][crate::view::portal]: a scrollable region
* [`progress_bar`][crate::view::progress_bar]: progress bar element
* [`prose`][crate::view::prose]: displays immutable, selectable text
* [`text_input`][crate::view::text_input]: allows text to be edited by the user
* [`task`][crate::view::task]: launch an async task which will run until the view is no longer in the tree
* [`zstack`][crate::view::zstack]: an element that lays out its children on top of each other

You should also expect to use the adapters from Xilem Core, including:

* [`lens`][crate::core::lens]: an adapter for using a component from a field of the current state.
* [`memoize`][crate::core::memoize]: allows you to avoid recreating views you know won't have changed, based on a key.

## Precise Capturing

Throughout Xilem you will find usage of `+ use<>` in return types, which is the Rust syntax for [Precise Capturing](https://doc.rust-lang.org/stable/std/keyword.use.html#precise-capturing).
This is new syntax in the 2024 edition, and so it might be unfamiliar.
Here's a snippet from the Xilem examples:

```rust
fn app_logic(data: &mut EmojiPagination) -> impl WidgetView<EmojiPagination> + use<> {
   // ...
}
```

The precise capturing syntax in this case indicates that the returned view does not make use of the lifetime of `data`.
This is required because the view types in Xilem must be `'static`, but as of the 2024 edition, when `impl Trait` is used
for return types, Rust assumes that the return value will use the parameter's lifetimes.
That is a simplifying assumption for most Rust code, but this is mismatched with how Xilem works.

[accesskit_docs]: masonry::accesskit
[AccessKit]: https://accesskit.dev/
[Druid]: https://crates.io/crates/druid
[Fontique]: https://crates.io/crates/fontique
[Masonry]: https://crates.io/crates/masonry
[Parley]: https://crates.io/crates/parley
[skrifa]: https://crates.io/crates/skrifa
[swash]: https://crates.io/crates/swash
[Vello]: https://crates.io/crates/vello
[winit]: https://crates.io/crates/winit
[xilem_blog]: https://raphlinus.github.io/rust/gui/2022/05/07/ui-architecture.html
[xilem_examples]: https://github.com/linebender/xilem/tree/main/xilem/examples

<!-- cargo-rdme end -->
<!-- markdownlint-enable MD053 -->

## Minimum supported Rust Version (MSRV)

This version of Xilem has been verified to compile with **Rust 1.88** and later.

Future versions of Xilem might increase the Rust version requirement.
It will not be treated as a breaking change and as such can even happen with small patch releases.

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
