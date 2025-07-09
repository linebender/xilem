<div align="center">

# Masonry Winit

**A foundational framework for Rust GUI libraries**

[![Latest published version.](https://img.shields.io/crates/v/masonry_winit.svg)](https://crates.io/crates/masonry_winit)
[![Documentation build status.](https://img.shields.io/docsrs/masonry_winit.svg)](https://docs.rs/masonry_winit)
[![Apache 2.0 license.](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](#license)
\
[![Linebender Zulip chat.](https://img.shields.io/badge/Linebender-%23masonry-blue?logo=Zulip)](https://xi.zulipchat.com/#narrow/stream/317477-masonry)
[![GitHub Actions CI status.](https://img.shields.io/github/actions/workflow/status/linebender/xilem/ci.yml?logo=github&label=CI)](https://github.com/linebender/xilem/actions)
[![Dependency staleness status.](https://deps.rs/crate/masonry_winit/latest/status.svg)](https://deps.rs/crate/masonry_winit)

</div>

<!-- We use cargo-rdme to update the README with the contents of lib.rs.
To edit the following section, update it in lib.rs, then run:
cargo rdme --workspace-project=masonry_winit --heading-base-level=0
Full documentation at https://github.com/orium/cargo-rdme -->

<!-- Intra-doc links used in lib.rs should be evaluated here.
See https://linebender.org/blog/doc-include/ for related discussion. -->

<!-- cargo-rdme start -->

Masonry gives you a platform to create windows (using [winit] as a backend) each with a tree of widgets. It also gives you tools to inspect that widget tree at runtime, write unit tests on it, and generally have an easier time debugging and maintaining your app.

The framework is not opinionated about what your user-facing abstraction will be: you can implement immediate-mode GUI, the Elm architecture, functional reactive GUI, etc, on top of Masonry.

See [Xilem] as an example of reactive UI built on top of Masonry.

Masonry was originally a fork of [Druid] that emerged from discussions within the Linebender community about what it would look like to turn Druid into a foundational library.

Masonry can currently be considered to be in an alpha state.
Lots of things need improvements, e.g. text input is janky and snapshot testing is not consistent across platforms.

## Example

The to-do-list example looks like this:

```rust
use masonry_winit::app::{AppDriver, DriverCtx, WindowId};
use masonry::core::{Action, Widget, WidgetId, WidgetPod};
use masonry::dpi::LogicalSize;
use masonry::widgets::{Button, Flex, Label, Portal, TextInput};
use winit::window::Window;

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
        action: Action,
    ) {
        debug_assert_eq!(window_id, self.window_id, "unknown window");

        match action {
            Action::ButtonPressed(_) => {
                ctx.render_root(window_id).edit_root_widget(|mut root| {
                    let mut portal = root.downcast::<Portal<Flex>>();
                    let mut flex = Portal::child_mut(&mut portal);
                    Flex::add_child(&mut flex, Label::new(self.next_task.clone()));
                });
            }
            Action::TextChanged(new_text) => {
                self.next_task = new_text.clone();
            }
            _ => {}
        }
    }
}

fn main() {
    const VERTICAL_WIDGET_SPACING: f64 = 20.0;

    let main_widget = Portal::new(
        Flex::column()
            .with_child(
                Flex::row()
                    .with_flex_child(TextInput::new(""), 1.0)
                    .with_child(Button::new("Add task")),
            )
            .with_spacer(VERTICAL_WIDGET_SPACING),
    );

    let window_size = LogicalSize::new(400.0, 400.0);
    let window_attributes = Window::default_attributes()
        .with_title("To-do list")
        .with_resizable(true)
        .with_min_inner_size(window_size);

    let driver = Driver {
        next_task: String::new(),
        window_id: WindowId::next(),
    };
    masonry_winit::app::run(
        masonry_winit::app::EventLoop::with_user_event(),
        vec![(
            driver.window_id,
            window_attributes,
            WidgetPod::new(main_widget).erased(),
        )],
        driver,
    )
    .unwrap();
}
```

For more information, see [the documentation module](masonry::doc).

### Crate feature flags

The following feature flags are available:

- `tracy`: Enables creating output for the [Tracy](https://github.com/wolfpld/tracy) profiler using [`tracing-tracy`][tracing_tracy].
  This can be used by installing Tracy and connecting to a Masonry with this feature enabled.

### Debugging features

Masonry apps currently ship with two debugging features built in:
- A rudimentary widget inspector - toggled by F11 key.
- A debug mode painting widget layout rectangles - toggled by F12 key.

[winit]: https://crates.io/crates/winit
[Druid]: https://crates.io/crates/druid
[Xilem]: https://crates.io/crates/xilem
[tracing_tracy]: https://crates.io/crates/tracing-tracy

<!-- cargo-rdme end -->

## Minimum supported Rust Version (MSRV)

This version of Masonry Winit has been verified to compile with **Rust 1.88** and later.

Future versions of Masonry Winit might increase the Rust version requirement.
It will not be treated as a breaking change and as such can even happen with small patch releases.

<details>
<summary>Click here if compiling fails.</summary>

As time has passed, some of Masonry Winit's dependencies could have released versions with a higher Rust requirement.
If you encounter a compilation issue due to a dependency and don't want to upgrade your Rust toolchain, then you could downgrade the dependency.

```sh
# Use the problematic dependency's name and version
cargo update -p package_name --precise 0.1.1
```
</details>

## Community

Discussion of Masonry Winit development happens in the [Linebender Zulip](https://xi.zulipchat.com/), specifically the [#masonry channel](https://xi.zulipchat.com/#narrow/stream/317477-masonry).
All public content can be read without logging in.

Contributions are welcome by pull request.
The [Rust code of conduct] applies.

## License

Licensed under the Apache License, Version 2.0 ([LICENSE](LICENSE) or <http://www.apache.org/licenses/LICENSE-2.0>)

[Rust code of conduct]: https://www.rust-lang.org/policies/code-of-conduct
