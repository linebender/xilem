// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0 AND MIT

//! A simple emoji picker.
//! It is expected that the Emoji in this example may not render.
//! This is because Vello does not support any kinds of bitmap fonts.
//!
//! Note that the MIT license is needed because of the emoji data.
//! Everything except for the [`EMOJI`] constant is Apache 2.0 licensed.

#![expect(clippy::shadow_unrelated, reason = "Idiomatic for Xilem users")]

use winit::error::EventLoopError;
use xilem::{
    core::map_state,
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
                EMOJI.len(),
            ),
            |state: &mut EmojiPagination| &mut state.start_index,
        ),
        data.last_selected
            .map(|idx| label(format!("Selected: {}", EMOJI[idx].display)).text_size(40.)),
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
            let emoji = EMOJI.get(idx);
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
                                    Color::BLUE
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
}

fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let data = EmojiPagination {
        size: 4,
        last_selected: None,
        start_index: 0,
    };

    let app = Xilem::new(data, app_logic);
    app.run_windowed(event_loop, "First Example".into())
}

struct EmojiInfo {
    name: &'static str,
    display: &'static str,
}

const fn e(display: &'static str, name: &'static str) -> EmojiInfo {
    EmojiInfo { name, display }
}

// Data adapted from https://github.com/iamcal/emoji-data
// under the MIT License. Full license text included below this item
const EMOJI: &[EmojiInfo] = &[
    e("😁", "grinning face with smiling eyes"),
    e("😂", "face with tears of joy"),
    e("😃", "smiling face with open mouth"),
    e("😄", "smiling face with open mouth and smiling eyes"),
    e("😅", "smiling face with open mouth and cold sweat"),
    e("😆", "smiling face with open mouth and tightly-closed eyes"),
    e("😇", "smiling face with halo"),
    e("😈", "smiling face with horns"),
    e("😉", "winking face"),
    e("😊", "smiling face with smiling eyes"),
    e("😋", "face savouring delicious food"),
    e("😌", "relieved face"),
    e("😍", "smiling face with heart-shaped eyes"),
    e("😎", "smiling face with sunglasses"),
    e("😏", "smirking face"),
    e("😐", "neutral face"),
    e("😑", "expressionless face"),
    e("😒", "unamused face"),
    e("😓", "face with cold sweat"),
    e("😔", "pensive face"),
    e("😕", "confused face"),
    e("😖", "confounded face"),
    e("😗", "kissing face"),
    e("😘", "face throwing a kiss"),
    e("😙", "kissing face with smiling eyes"),
    e("😚", "kissing face with closed eyes"),
    e("😛", "face with stuck-out tongue"),
    e("😜", "face with stuck-out tongue and winking eye"),
    e("😝", "face with stuck-out tongue and tightly-closed eyes"),
    e("😞", "disappointed face"),
    e("😟", "worried face"),
    e("😠", "angry face"),
    e("😡", "pouting face"),
    e("😢", "crying face"),
    e("😣", "persevering face"),
    e("😤", "face with look of triumph"),
    e("😥", "disappointed but relieved face"),
    e("😦", "frowning face with open mouth"),
    e("😧", "anguished face"),
    e("😨", "fearful face"),
    e("😩", "weary face"),
    e("😪", "sleepy face"),
    e("😫", "tired face"),
    e("😬", "grimacing face"),
    e("😭", "loudly crying face"),
    e("😮‍💨", "face exhaling"),
    e("😮", "face with open mouth"),
    e("😯", "hushed face"),
    e("😰", "face with open mouth and cold sweat"),
    e("😱", "face screaming in fear"),
    e("😲", "astonished face"),
    e("😳", "flushed face"),
    e("😴", "sleeping face"),
    e("😵‍💫", "face with spiral eyes"),
    e("😵", "dizzy face"),
    e("😶‍🌫️", "face in clouds"),
    e("😶", "face without mouth"),
    e("😷", "face with medical mask"),
    e("😸", "grinning cat face with smiling eyes"),
    e("😹", "cat face with tears of joy"),
    e("😺", "smiling cat face with open mouth"),
    e("😻", "smiling cat face with heart-shaped eyes"),
    e("😼", "cat face with wry smile"),
    e("😽", "kissing cat face with closed eyes"),
    e("😾", "pouting cat face"),
    e("😿", "crying cat face"),
    e("🙀", "weary cat face"),
    e("🙁", "slightly frowning face"),
    e("🙂‍↔️", "head shaking horizontally"),
    e("🙂‍↕️", "head shaking vertically"),
    e("🙂", "slightly smiling face"),
    e("🙃", "upside-down face"),
    e("🙄", "face with rolling eyes"),
    e("🙅‍♀️", "woman gesturing no"),
    e("🙅‍♂️", "man gesturing no"),
    e("🙅", "face with no good gesture"),
    e("🙆‍♀️", "woman gesturing ok"),
    e("🙆‍♂️", "man gesturing ok"),
    e("🙆", "face with ok gesture"),
    e("🙇‍♀️", "woman bowing"),
    e("🙇‍♂️", "man bowing"),
    e("🙇", "person bowing deeply"),
    e("🙈", "see-no-evil monkey"),
    e("🙉", "hear-no-evil monkey"),
    e("🙊", "speak-no-evil monkey"),
    e("🙋‍♀️", "woman raising hand"),
    e("🙋‍♂️", "man raising hand"),
    e("🙋", "happy person raising one hand"),
    e("🙌", "person raising both hands in celebration"),
    e("🙍‍♀️", "woman frowning"),
    e("🙍‍♂️", "man frowning"),
    e("🙍", "person frowning"),
    e("🙎‍♀️", "woman pouting"),
    e("🙎‍♂️", "man pouting"),
    e("🙎", "person with pouting face"),
    e("🙏", "person with folded hands"),
    e("🚀", "rocket"),
    e("🚁", "helicopter"),
];

// The MIT License (MIT)
//
// Copyright (c) 2013 Cal Henderson
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

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
