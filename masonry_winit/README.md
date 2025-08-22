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
cargo rdme --workspace-project=masonry_winit
Full documentation at https://github.com/orium/cargo-rdme -->

<!-- Intra-doc links used in lib.rs should be evaluated here.
See https://linebender.org/blog/doc-include/ for related discussion. -->

<!-- TODO: Standardise on docs.rs or crates.io pages here? -->

[winit]: https://crates.io/crates/winit

<!-- cargo-rdme start -->

This is the [Winit][winit] backend for the [Masonry] GUI framework.

See [Masonry's documentation] for more details, examples and resources.

## Example

```rust
use masonry::core::{ErasedAction, NewWidget, Widget, WidgetId, WidgetPod};
use masonry::dpi::LogicalSize;
use masonry::theme::default_property_set;
use masonry_winit::app::{AppDriver, DriverCtx, NewWindow, WindowId};
use masonry_winit::winit::window::Window;

struct Driver {
    // ...
}

impl AppDriver for Driver {
    fn on_action(
        &mut self,
        window_id: WindowId,
        ctx: &mut DriverCtx<'_, '_>,
        widget_id: WidgetId,
        action: ErasedAction,
    ) {
        // ...
    }
}

fn main() {
    let main_widget = {
        // ...
    };

    let window_size = LogicalSize::new(400.0, 400.0);
    let window_attributes = masonry_winit::winit::window::WindowAttributes::default()
        .with_title("My Masonry App")
        .with_resizable(true)
        .with_min_inner_size(window_size);

    let driver = Driver {
        // ...
    };
    let event_loop = masonry_winit::app::EventLoop::builder()
        .build()
        .unwrap();
    masonry_winit::app::run_with(
        event_loop,
        vec![NewWindow::new(
            window_attributes,
            NewWidget::new(main_widget).erased(),
        )],
        driver,
        default_property_set(),
    )
    .unwrap();
}
```

(See the Masonry documentation for more detailed examples.)

[Masonry's documentation]: https://docs.rs/masonry
[Masonry]: https://crates.io/crates/masonry

<!-- cargo-rdme end -->

## Minimum supported Rust Version (MSRV)

This version of Masonry Winit has been verified to compile with **Rust 1.88** and later.

Future versions of Masonry Winit might increase the Rust version requirement.
It will not be treated as a breaking change and as such can even happen with small patch releases.

## Community

Discussion of Masonry Winit development happens in the [Linebender Zulip](https://xi.zulipchat.com/), specifically the [#masonry channel](https://xi.zulipchat.com/#narrow/stream/317477-masonry).
All public content can be read without logging in.

Contributions are welcome by pull request.
The [Rust code of conduct] applies.

## License

Licensed under the Apache License, Version 2.0 ([LICENSE](LICENSE) or <http://www.apache.org/licenses/LICENSE-2.0>)

[Rust code of conduct]: https://www.rust-lang.org/policies/code-of-conduct
