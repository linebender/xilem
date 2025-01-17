// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache 2.0

//! A simple emoji picker.

#![expect(clippy::shadow_unrelated, reason = "Idiomatic for Xilem users")]

use winit::error::EventLoopError;
use xilem::{
    core::map_state,
    palette,
    view::{button, flex, label, prose, sized_box, Axis},
    AnyWidgetView, Color, EventLoop, EventLoopBuilder, WidgetView, Xilem,
};

fn app_logic(data: &mut EmojiPagination) -> impl WidgetView<EmojiPagination> {
    flex((
        sized_box(flex(()).must_fill_major_axis(true)).height(50.), // Padding because of the info bar on Android
        flex((
            // TODO: Expose that this is a "zoom out" button accessibly
            button("🔍-", |data: &mut EmojiPagination| {
                data.size = (data.size + 1).min(5);
            }),
            // TODO: Expose that this is a "zoom in" button accessibly
            button("🔍+", |data: &mut EmojiPagination| {
                data.size = (data.size - 1).max(2);
            }),
        ))
        .direction(Axis::Horizontal),
        picker(data),
        map_state(
            paginate(
                data.start_index,
                (data.size * data.size) as usize,
                data.emoji.len(),
            ),
            |state: &mut EmojiPagination| &mut state.start_index,
        ),
        data.last_selected
            .map(|idx| label(format!("Selected: {}", data.emoji[idx].display)).text_size(40.)),
    ))
    .direction(Axis::Vertical)
}

fn picker(data: &mut EmojiPagination) -> impl WidgetView<EmojiPagination> {
    let mut result = vec![];
    // TODO: We should be able to use a grid view here, but that isn't implemented
    // We hack around it by making each item take up their proportion of the 400
    let dimensions = 400. / data.size as f64;
    for y in 0..data.size as usize {
        let mut row_contents = vec![];
        let row_idx = data.start_index + y * data.size as usize;
        for x in 0..data.size as usize {
            let idx = row_idx + x;
            let emoji = data.emoji.get(idx);
            // TODO: Use OneOf2
            let view: Box<AnyWidgetView<EmojiPagination>> = match emoji {
                Some(emoji) => {
                    let view = flex((
                        // TODO: Expose that this button corresponds to the label below to accessibility?
                        sized_box(button(emoji.display, move |data: &mut EmojiPagination| {
                            data.last_selected = Some(idx);
                        }))
                        .expand_width(),
                        sized_box(
                            prose(emoji.name)
                                .alignment(xilem::TextAlignment::Middle)
                                .brush(if data.last_selected.is_some_and(|it| it == idx) {
                                    // TODO: Ensure this selection indicator color is accessible
                                    // TODO: Expose selected state to accessibility tree
                                    palette::css::BLUE
                                } else {
                                    Color::WHITE
                                }),
                        )
                        .expand_width(),
                    ))
                    .must_fill_major_axis(true);
                    Box::new(view)
                }
                None => Box::new(flex(())),
            };
            row_contents.push(sized_box(view).width(dimensions).height(dimensions));
        }
        result.push(flex(row_contents).direction(Axis::Horizontal));
    }

    flex(result)
}

fn paginate(
    current_start: usize,
    count_per_page: usize,
    max_count: usize,
) -> impl WidgetView<usize> {
    let percentage = (current_start * 100) / max_count;

    flex((
        // TODO: Expose that this is a previous page button to accessibility
        button("<-", move |data| {
            *data = current_start.saturating_sub(count_per_page);
        }),
        label(format!("{percentage}%")),
        button("->", move |data| {
            let new_idx = current_start + count_per_page;
            if new_idx < max_count {
                *data = new_idx;
            }
        }),
    ))
    .direction(Axis::Horizontal)
}

struct EmojiPagination {
    size: u32,
    last_selected: Option<usize>,
    start_index: usize,
    emoji: Vec<EmojiInfo>,
}

fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let emoji = EmojiInfo::parse_file();
    let data = EmojiPagination {
        size: 4,
        last_selected: None,
        start_index: 0,
        emoji,
    };

    let app = Xilem::new(data, app_logic);
    app.run_windowed(event_loop, "First Example".into())
}

struct EmojiInfo {
    name: &'static str,
    display: &'static str,
}

impl EmojiInfo {
    /// Parse the supported emoji's information.
    fn parse_file() -> Vec<Self> {
        let mut lines = EMOJI_NAMES_CSV.lines();
        let first_line = lines.next();
        assert_eq!(
            first_line,
            Some("display,name"),
            "Probably wrong CSV-like file"
        );
        lines.flat_map(Self::parse_single).collect()
    }

    fn parse_single(line: &'static str) -> Option<Self> {
        let (display, name) = line.split_once(',')?;
        Some(Self { display, name })
    }
}

/// A subset of emoji data from <https://github.com/iamcal/emoji-data>, used under the MIT license.
/// Full details can be found in `xilem/resources/data/emoji_names/README.md` from
/// the workspace root.
const EMOJI_NAMES_CSV: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/data/emoji_names/emoji.csv",
));

// Boilerplate code: Identical across all applications which support Android

#[expect(clippy::allow_attributes, reason = "No way to specify the condition")]
#[allow(dead_code, reason = "False positive: needed in not-_android version")]
// This is treated as dead code by the Android version of the example, but is actually live
// This hackery is required because Cargo doesn't care to support this use case, of one
// example which works across Android and desktop
fn main() -> Result<(), EventLoopError> {
    run(EventLoop::with_user_event())
}
#[cfg(target_os = "android")]
// Safety: We are following `android_activity`'s docs here
#[expect(
    unsafe_code,
    reason = "We believe that there are no other declarations using this name in the compiled objects here"
)]
#[no_mangle]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    let mut event_loop = EventLoop::with_user_event();
    event_loop.with_android_app(app);

    run(event_loop).expect("Can create app");
}
