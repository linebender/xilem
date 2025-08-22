<div align="center">

# Masonry

**A foundational framework for Rust GUI libraries**

[![Latest published version.](https://img.shields.io/crates/v/masonry.svg)](https://crates.io/crates/masonry)
[![Documentation build status.](https://img.shields.io/docsrs/masonry.svg)](https://docs.rs/masonry)
[![Apache 2.0 license.](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](#license)
\
[![Linebender Zulip chat.](https://img.shields.io/badge/Linebender-%23masonry-blue?logo=Zulip)](https://xi.zulipchat.com/#narrow/stream/317477-masonry)
[![GitHub Actions CI status.](https://img.shields.io/github/actions/workflow/status/linebender/xilem/ci.yml?logo=github&label=CI)](https://github.com/linebender/xilem/actions)
[![Dependency staleness status.](https://deps.rs/crate/masonry/latest/status.svg)](https://deps.rs/crate/masonry)

</div>

<!-- We use cargo-rdme to update the README with the contents of lib.rs.
To edit the following section, update it in lib.rs, then run:
cargo rdme --workspace-project=masonry
Full documentation at https://github.com/orium/cargo-rdme -->

<!-- Intra-doc links used in lib.rs are evaluated here.
See https://linebender.org/blog/doc-include/ for related discussion. -->

[vello]: https://crates.io/crates/vello
[vello::wgpu]: https://crates.io/crates/wgpu
[parley]: https://crates.io/crates/parley
[accesskit]: https://crates.io/crates/accesskit
[tracing]: https://crates.io/crates/tracing

<!-- cargo-rdme start -->

Masonry is a foundational framework for building GUI libraries in Rust.

The developers of Masonry are developing [Xilem], a reactive UI library built on top of Masonry.
Masonry's API is geared towards creating GUI libraries; if you are creating an application, we recommend also considering Xilem.

Masonry gives you a platform-independent manager, which owns and maintains a widget tree.
It also gives you tools to inspect that widget tree at runtime, write unit tests on it, and generally have an easier time debugging and maintaining your app.

The framework is not opinionated about what your user-facing abstraction will be: you can implement immediate-mode GUI, the Elm architecture, functional reactive GUI, etc., on top of Masonry.

It *is* opinionated about its internals: things like text focus, pointer interactions and accessibility events are often handled in a centralized way.

Masonry is built on top of:

- [Vello][vello] and [wgpu][vello::wgpu] for 2D graphics.
- [Parley][parley] for the text stack.
- [AccessKit][accesskit] for plugging into accessibility APIs.

Masonry can be used with any windowing library which allows the window content to be rendered using `wgpu`.
There are currently two backends for using Masonry to create operating system windows:

- [masonry_winit] for most platforms.
- `masonry_android_view` for Android. This can currently be found in the [Android View repository](https://github.com/rust-mobile/android-view),
  and is not yet generally usable.

<!-- TODO: Document that Masonry is a set of baseline widgets and properties built on Masonry core, which can also be used completely independently -->

## Example

The to-do-list example looks like this, using `masonry_winit` as the backend:

```rust
use masonry::core::{ErasedAction, NewWidget, Widget, WidgetId, WidgetPod};
use masonry::dpi::LogicalSize;
use masonry::properties::types::{Length, AsUnit};
use masonry::theme::default_property_set;
use masonry::widgets::{Button, ButtonPress, Flex, Label, Portal, TextAction, TextInput};
use masonry_winit::app::{AppDriver, DriverCtx, NewWindow, WindowId};
use masonry_winit::winit::window::Window;

struct Driver {
    next_task: String,
    window_id: WindowId,
}

impl AppDriver for Driver {
    fn on_action(
        &mut self,
        window_id: WindowId,
        ctx: &mut DriverCtx<'_, '_>,
        _widget_id: WidgetId,
        action: ErasedAction,
    ) {
        debug_assert_eq!(window_id, self.window_id, "unknown window");

        if action.is::<ButtonPress>() {
            ctx.render_root(window_id).edit_root_widget(|mut root| {
                let mut portal = root.downcast::<Portal<Flex>>();
                let mut flex = Portal::child_mut(&mut portal);
                Flex::add_child(&mut flex, Label::new(self.next_task.clone()).with_auto_id());
            });
        } else if action.is::<TextAction>() {
            let action = *action.downcast::<TextAction>().unwrap();
            match action {
                TextAction::Changed(new_text) => {
                    self.next_task = new_text.clone();
                }
                _ => {}
            }
        }
    }
}

fn main() {
    const WIDGET_SPACING: Length = Length::const_px(5.0);

    let main_widget = Portal::new(
        Flex::column()
            .with_child(NewWidget::new(
                Flex::row()
                    .with_flex_child(TextInput::new("").with_auto_id(), 1.0)
                    .with_child(
                        Button::new(
                            Label::new("Add task").with_auto_id()
                        ).with_auto_id()
                    ),
            ))
            .with_spacer(WIDGET_SPACING)
            .with_auto_id(),
    );

    let window_size = LogicalSize::new(400.0, 400.0);
    let window_attributes = masonry_winit::winit::window::WindowAttributes::default()
        .with_title("To-do list")
        .with_resizable(true)
        .with_min_inner_size(window_size);

    let driver = Driver {
        next_task: String::new(),
        window_id: WindowId::next(),
    };
    let event_loop = masonry_winit::app::EventLoop::builder()
        .build()
        .unwrap();
    masonry_winit::app::run_with(
        event_loop,
        vec![NewWindow::new_with_id(
            driver.window_id,
            window_attributes,
            NewWidget::new(main_widget).erased(),
        )],
        driver,
        default_property_set(),
    )
    .unwrap();
}
```

## Feature flags

The following crate [feature flags](https://doc.rust-lang.org/cargo/reference/features.html#dependency-features) are available:

- `tracy`: Enables creating output for the [Tracy](https://github.com/wolfpld/tracy) profiler using [`tracing-tracy`][tracing_tracy].
  This can be used by installing Tracy and connecting to a Masonry with this feature enabled.
- `testing`: Re-exports the test harness from [`masonry_testing`].

## Debugging features

Masonry apps currently ship with several debugging features built in:

- A rudimentary widget inspector - toggled by the F11 key.
- A debug mode painting widget layout rectangles - toggled by the F12 key.
- Automatic registration of a [tracing] subscriber, which outputs to the console and to a file in the dev profile.

[masonry_winit]: https://crates.io/crates/masonry_winit
[Xilem]: https://github.com/linebender/xilem/tree/main/xilem
[tracing_tracy]: https://crates.io/crates/tracing-tracy

<!-- cargo-rdme end -->

## Minimum supported Rust Version (MSRV)

This version of Masonry has been verified to compile with **Rust 1.88** and later.

Future versions of Masonry might increase the Rust version requirement.
It will not be treated as a breaking change and as such can even happen with small patch releases.

## Community

Discussion of Masonry development happens in the [Linebender Zulip](https://xi.zulipchat.com/), specifically the [#masonry channel](https://xi.zulipchat.com/#narrow/stream/317477-masonry).
All public content can be read without logging in.

Contributions are welcome by pull request.
The [Rust code of conduct] applies.

## License

Licensed under the Apache License, Version 2.0 ([LICENSE](LICENSE) or <http://www.apache.org/licenses/LICENSE-2.0>)

[Rust code of conduct]: https://www.rust-lang.org/policies/code-of-conduct
