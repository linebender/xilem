// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A simple emoji picker.

use masonry::properties::types::AsUnit;
use winit::error::EventLoopError;
use xilem::core::map_state;
use xilem::style::Style as _;
use xilem::view::{
    Axis, FlexExt, FlexSpacer, GridExt, button, flex, flex_row, grid, label, prose, sized_box,
};
use xilem::{
    Color, EventLoop, EventLoopBuilder, TextAlign, WidgetView, WindowOptions, Xilem, palette,
};

fn app_logic(data: &mut EmojiPagination) -> impl WidgetView<EmojiPagination> + use<> {
    flex((
        FlexSpacer::Fixed(50.px()), // Padding because of the info bar on Android
        flex_row((
            // TODO: Expose that this is a "zoom out" button accessibly
            button("🔍-", |data: &mut EmojiPagination| {
                data.size = (data.size + 1).min(5);
            }),
            // TODO: Expose that this is a "zoom in" button accessibly
            button("🔍+", |data: &mut EmojiPagination| {
                data.size = (data.size - 1).max(2);
            }),
        )),
        picker(data).flex(1.0),
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
        FlexSpacer::Fixed(10.px()),
    ))
    .direction(Axis::Vertical)
    .must_fill_major_axis(true)
}

fn picker(data: &mut EmojiPagination) -> impl WidgetView<EmojiPagination> + use<> {
    let mut grid_items = vec![];
    'outer: for y in 0..data.size as usize {
        let row_idx = data.start_index + y * data.size as usize;
        for x in 0..data.size as usize {
            let idx = row_idx + x;
            let emoji = data.emoji.get(idx);
            let Some(emoji) = emoji else {
                // There are no more emoji, no point still looping
                break 'outer;
            };
            let view = flex((
                // TODO: Expose that this button corresponds to the label below for accessibility?
                sized_box(button(
                    label(emoji.display).text_size(200.0 / data.size as f32),
                    move |data: &mut EmojiPagination| {
                        data.last_selected = Some(idx);
                    },
                ))
                .expand_width(),
                sized_box(
                    prose(emoji.name)
                        .text_alignment(TextAlign::Center)
                        .text_color(if data.last_selected.is_some_and(|it| it == idx) {
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
            grid_items.push(view.grid_pos(x.try_into().unwrap(), y.try_into().unwrap()));
        }
    }

    grid(
        grid_items,
        data.size.try_into().unwrap(),
        data.size.try_into().unwrap(),
    )
    .spacing(10.px())
    .padding(20.0)
}

fn paginate(
    current_start: usize,
    count_per_page: usize,
    max_count: usize,
) -> impl WidgetView<usize> {
    let current_end = (current_start + count_per_page).min(max_count);
    let percentage_start = (current_start * 100) / max_count;
    let percentage_end = (current_end * 100) / max_count;

    flex_row((
        // TODO: Expose that this is a previous page button to accessibility
        button(label("⬅️").text_size(24.0), move |data| {
            *data = current_start.saturating_sub(count_per_page);
        })
        .disabled(current_start == 0),
        label(format!("{percentage_start}% - {percentage_end}%")),
        button(label("➡️").text_size(24.0), move |data| {
            let new_idx = current_start + count_per_page;
            if new_idx < max_count {
                *data = new_idx;
            }
        })
        .disabled(current_end == max_count),
    ))
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

    let app = Xilem::new_simple(data, app_logic, WindowOptions::new("Emoji picker"));
    app.run_in(event_loop)
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
    run(EventLoop::builder())
}
#[cfg(target_os = "android")]
// Safety: We are following `android_activity`'s docs here
#[expect(
    unsafe_code,
    reason = "We believe that there are no other declarations using this name in the compiled objects here"
)]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    let mut event_loop = EventLoop::builder();
    event_loop.with_android_app(app);

    run(event_loop).expect("Can create app");
}
