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

<!-- Image link used in lib.rs. -->
[to-do-screenshot]: ./screenshots/example_to_do_list_initial.png

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
use masonry::core::{ErasedAction, NewWidget, Properties, Widget, WidgetId, WidgetTag};
use masonry::dpi::LogicalSize;
use masonry::peniko::color::AlphaColor;
use masonry::properties::Padding;
use masonry::properties::types::Length;
use masonry::theme::default_property_set;
use masonry::widgets::{Button, ButtonPress, Flex, Label, Portal, TextAction, TextArea, TextInput};
use masonry_winit::app::{AppDriver, DriverCtx, NewWindow, WindowId};
use masonry_winit::winit::window::Window;

const TEXT_INPUT_TAG: WidgetTag<TextInput> = WidgetTag::new("text-input");
const LIST_TAG: WidgetTag<Flex> = WidgetTag::new("list");
const WIDGET_SPACING: Length = Length::const_px(5.0);

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
            let render_root = ctx.render_root(window_id);

            render_root.edit_widget_with_tag(TEXT_INPUT_TAG, |mut text_input| {
                let mut text_area = TextInput::text_mut(&mut text_input);
                TextArea::reset_text(&mut text_area, "");
            });
            render_root.edit_widget_with_tag(LIST_TAG, |mut list| {
                let child = Label::new(self.next_task.clone()).with_auto_id();
                Flex::add_child(&mut list, child);
            });
        } else if action.is::<TextAction>() {
            let action = action.downcast::<TextAction>().unwrap();
            match *action {
                TextAction::Changed(new_text) => {
                    self.next_task = new_text.clone();
                }
                TextAction::Entered(_) => {}
            }
        }
    }
}

/// Return initial to-do-list without items.
pub fn make_widget_tree() -> NewWidget<impl Widget> {
    let text_input = NewWidget::new_with_tag(
        TextInput::new("").with_placeholder("ex: 'Do the dishes', 'File my taxes', ..."),
        TEXT_INPUT_TAG,
    );
    let button = NewWidget::new(Button::with_text("Add task"));

    let list = Flex::column()
        .with_child(NewWidget::new_with_props(
            Flex::row()
                .with_flex_child(text_input, 1.0)
                .with_child(button),
            Properties::new().with(Padding::all(WIDGET_SPACING.get())),
        ))
        .with_spacer(WIDGET_SPACING);

    NewWidget::new(Portal::new(NewWidget::new_with_tag(list, LIST_TAG)))
}

fn main() {
    let window_size = LogicalSize::new(400.0, 400.0);
    let window_attributes = Window::default_attributes()
        .with_title("To-do list")
        .with_resizable(true)
        .with_min_inner_size(window_size);
    let driver = Driver {
        next_task: String::new(),
        window_id: WindowId::next(),
    };

    let event_loop = masonry_winit::app::EventLoop::with_user_event()
        .build()
        .unwrap();
    masonry_winit::app::run_with(
        event_loop,
        vec![
            NewWindow::new_with_id(
                driver.window_id,
                window_attributes,
                make_widget_tree().erased(),
            )
            .with_base_color(AlphaColor::from_rgb8(2, 6, 23)),
        ],
        driver,
        default_property_set(),
    )
    .unwrap();
}
```

Running this will open a window that looks like this:

![Screenshot of the to-do-list example][to-do-screenshot]

## Feature flags

The following crate [feature flags](https://doc.rust-lang.org/cargo/reference/features.html#dependency-features) are available:

- `tracy`: Enables creating output for the [Tracy](https://github.com/wolfpld/tracy) profiler using [`tracing-tracy`][tracing_tracy].
  This can be used by installing Tracy and connecting to a Masonry with this feature enabled.
- `testing`: Re-exports the test harness from [`masonry_testing`].

## Debugging features

Masonry apps currently ship with several debugging features built in:

- A rudimentary widget inspector - toggled by the F11 key.
- A debug mode painting widget layout rectangles - toggled by the F12 key.
- Optional automatic registration of a [tracing] subscriber, which outputs to the console and to a file in the dev profile.

If you want to use your own subscriber, simply set it before starting masonry - in this case masonry will not set a subscriber.

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
