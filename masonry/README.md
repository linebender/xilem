<div align="center">

# Masonry

**A foundational framework for Rust GUI libraries.**

[![Latest published version.](https://img.shields.io/crates/v/masonry.svg)](https://crates.io/crates/masonry)
[![Documentation build status.](https://img.shields.io/docsrs/masonry.svg)](https://docs.rs/masonry)
[![Apache 2.0 license.](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](#license)

[![Linebender Zulip chat.](https://img.shields.io/badge/Linebender-%23masonry-blue?logo=Zulip)](https://xi.zulipchat.com/#narrow/stream/317477-masonry)
[![GitHub Actions CI status.](https://img.shields.io/github/actions/workflow/status/linebender/xilem/ci.yml?logo=github&label=CI)](https://github.com/linebender/xilem/actions)
[![Dependency staleness status.](https://deps.rs/crate/masonry/latest/status.svg)](https://deps.rs/crate/masonry)

</div>

[tracing_tracy]: https://crates.io/crates/tracing-tracy

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
use masonry::app_driver::{AppDriver, DriverCtx};
use masonry::dpi::LogicalSize;
use masonry::widget::{Button, Flex, Label, Portal, RootWidget, Textbox, WidgetMut};
use masonry::{Action, WidgetId};
use winit::window::Window;

const VERTICAL_WIDGET_SPACING: f64 = 20.0;

struct Driver {
    next_task: String,
}

impl AppDriver for Driver {
    fn on_action(&mut self, ctx: &mut DriverCtx<'_>, _widget_id: WidgetId, action: Action) {
        match action {
            Action::ButtonPressed(_) => {
                let mut root: WidgetMut<RootWidget<Portal<Flex>>> = ctx.get_root();
                let mut root = root.child_mut();
                let mut flex = root.child_mut();
                flex.add_child(Label::new(self.next_task.clone()));
            }
            Action::TextChanged(new_text) => {
                self.next_task = new_text.clone();
            }
            _ => {}
        }
    }
}

fn main() {
    let main_widget = Portal::new(
        Flex::column()
            .with_child(
                Flex::row()
                    .with_flex_child(Textbox::new(""), 1.0)
                    .with_child(Button::new("Add task")),
            )
            .with_spacer(VERTICAL_WIDGET_SPACING),
    );

    let window_size = LogicalSize::new(400.0, 400.0);
    let window_attributes = Window::default_attributes()
        .with_title("To-do list")
        .with_resizable(true)
        .with_min_inner_size(window_size);

    masonry::event_loop_runner::run(
        masonry::event_loop_runner::EventLoop::with_user_event(),
        window_attributes,
        RootWidget::new(main_widget),
        Driver {
            next_task: String::new(),
        },
    )
    .unwrap();
}
```

### Create feature flags

The following feature flags are available:

- `tracy`: Enables creating output for the [Tracy](https://github.com/wolfpld/tracy) profiler using [`tracing-tracy`][tracing_tracy].
  This can be used by installing Tracy and connecting to a Masonry with this feature enabled.

[winit]: https://crates.io/crates/winit
[Druid]: https://crates.io/crates/druid
[Xilem]: https://crates.io/crates/xilem

<!-- cargo-rdme end -->

## Minimum supported Rust Version (MSRV)

This version of Masonry has been verified to compile with **Rust 1.79** and later.

Future versions of Masonry might increase the Rust version requirement.
It will not be treated as a breaking change and as such can even happen with small patch releases.

<details>
<summary>Click here if compiling fails.</summary>

As time has passed, some of Masonry's dependencies could have released versions with a higher Rust requirement.
If you encounter a compilation issue due to a dependency and don't want to upgrade your Rust toolchain, then you could downgrade the dependency.

```sh
# Use the problematic dependency's name and version
cargo update -p package_name --precise 0.1.1
```

</details>

## Community

Discussion of Masonry development happens in the [Linebender Zulip](https://xi.zulipchat.com/), specifically the [#masonry channel](https://xi.zulipchat.com/#narrow/stream/317477-masonry).
All public content can be read without logging in.

Contributions are welcome by pull request. The [Rust code of conduct] applies.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache 2.0 license, shall be licensed as noted in the [License](#license) section, without any additional terms or conditions.

## License

Licensed under the Apache License, Version 2.0 ([LICENSE](LICENSE) or <http://www.apache.org/licenses/LICENSE-2.0>)

[Rust code of conduct]: https://www.rust-lang.org/policies/code-of-conduct
