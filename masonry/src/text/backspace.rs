// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Calc start of a backspace delete interval

use xi_unicode::*;

use crate::text::StringCursor;

/// Logic adapted from Android and
/// <https://github.com/xi-editor/xi-editor/pull/837>
/// See links present in that PR for upstream Android Source
/// Matches Android Logic as at 2024-05-10
#[allow(clippy::cognitive_complexity)]
fn backspace_offset(text: &str, start: usize) -> usize {
    #[derive(PartialEq)]
    enum State {
        Start,
        Lf,
        BeforeKeycap,
        BeforeVsAndKeycap,
        BeforeEmojiModifier,
        BeforeVsAndEmojiModifier,
        BeforeVs,
        BeforeEmoji,
        BeforeZwj,
        BeforeVsAndZwj,
        OddNumberedRis,
        EvenNumberedRis,
        InTagSequence,
        Finished,
    }
    let mut state = State::Start;

    let mut delete_code_point_count = 0;
    let mut last_seen_vs_code_point_count = 0;

    let mut cursor = StringCursor {
        text,
        position: start,
    };
    assert!(
        cursor.is_boundary(),
        "Backspace must begin at a valid codepoint boundary."
    );

    while state != State::Finished && cursor.pos() > 0 {
        let code_point = cursor.prev_codepoint().unwrap_or('0');

        match state {
            State::Start => {
                delete_code_point_count = 1;
                if code_point == '\n' {
                    state = State::Lf;
                } else if is_variation_selector(code_point) {
                    state = State::BeforeVs;
                } else if code_point.is_regional_indicator_symbol() {
                    state = State::OddNumberedRis;
                } else if code_point.is_emoji_modifier() {
                    state = State::BeforeEmojiModifier;
                } else if code_point.is_emoji_combining_enclosing_keycap() {
                    state = State::BeforeKeycap;
                } else if code_point.is_emoji() {
                    state = State::BeforeEmoji;
                } else if code_point.is_emoji_cancel_tag() {
                    state = State::InTagSequence;
                } else {
                    state = State::Finished;
                }
            }
            State::Lf => {
                if code_point == '\r' {
                    delete_code_point_count += 1;
                }
                state = State::Finished;
            }
            State::OddNumberedRis => {
                if code_point.is_regional_indicator_symbol() {
                    delete_code_point_count += 1;
                    state = State::EvenNumberedRis;
                } else {
                    state = State::Finished;
                }
            }
            State::EvenNumberedRis => {
                if code_point.is_regional_indicator_symbol() {
                    delete_code_point_count -= 1;
                    state = State::OddNumberedRis;
                } else {
                    state = State::Finished;
                }
            }
            State::BeforeKeycap => {
                if is_variation_selector(code_point) {
                    last_seen_vs_code_point_count = 1;
                    state = State::BeforeVsAndKeycap;
                } else {
                    if is_keycap_base(code_point) {
                        delete_code_point_count += 1;
                    }
                    state = State::Finished;
                }
            }
            State::BeforeVsAndKeycap => {
                if is_keycap_base(code_point) {
                    delete_code_point_count += last_seen_vs_code_point_count + 1;
                }
                state = State::Finished;
            }
            State::BeforeEmojiModifier => {
                if is_variation_selector(code_point) {
                    last_seen_vs_code_point_count = 1;
                    state = State::BeforeVsAndEmojiModifier;
                } else if code_point.is_emoji_modifier_base() {
                    delete_code_point_count += 1;
                    state = State::BeforeEmoji;
                } else {
                    state = State::Finished;
                }
            }
            State::BeforeVsAndEmojiModifier => {
                if code_point.is_emoji_modifier_base() {
                    delete_code_point_count += last_seen_vs_code_point_count + 1;
                }
                state = State::Finished;
            }
            State::BeforeVs => {
                if code_point.is_emoji() {
                    delete_code_point_count += 1;
                    state = State::BeforeEmoji;
                } else {
                    if !is_variation_selector(code_point) {
                        //TODO: UCharacter.getCombiningClass(codePoint) == 0
                        delete_code_point_count += 1;
                    }
                    state = State::Finished;
                }
            }
            State::BeforeEmoji => {
                if code_point.is_zwj() {
                    state = State::BeforeZwj;
                } else {
                    state = State::Finished;
                }
            }
            State::BeforeZwj => {
                if code_point.is_emoji() {
                    delete_code_point_count += 2;
                    state = if code_point.is_emoji_modifier() {
                        State::BeforeEmojiModifier
                    } else {
                        State::BeforeEmoji
                    };
                } else if is_variation_selector(code_point) {
                    last_seen_vs_code_point_count = 1;
                    state = State::BeforeVsAndZwj;
                } else {
                    state = State::Finished;
                }
            }
            State::BeforeVsAndZwj => {
                if code_point.is_emoji() {
                    delete_code_point_count += last_seen_vs_code_point_count + 2;
                    last_seen_vs_code_point_count = 0;
                    state = State::BeforeEmoji;
                } else {
                    state = State::Finished;
                }
            }
            State::InTagSequence => {
                if code_point.is_tag_spec_char() {
                    delete_code_point_count += 1;
                } else if code_point.is_emoji() {
                    delete_code_point_count += 1;
                    state = State::Finished;
                } else {
                    delete_code_point_count = 1;
                    state = State::Finished;
                }
            }
            State::Finished => {
                break;
            }
        }
    }

    cursor.set(start);
    for _ in 0..delete_code_point_count {
        let _ = cursor.prev_codepoint();
    }
    cursor.pos()
}

/// Calculate resulting offset for a backwards delete.
///
/// This involves complicated logic to handle various special cases that
/// are unique to backspace.
#[allow(clippy::trivially_copy_pass_by_ref)]
pub fn offset_for_delete_backwards(caret_position: usize, text: &impl AsRef<str>) -> usize {
    backspace_offset(text.as_ref(), caret_position)
}

#[cfg(test)]
mod tests {
    //! These tests originate from <https://github.com/xi-editor/xi-editor/pull/837>,
    //! with the logic itself originating from upstream Android.

    #[track_caller]
    fn assert_delete_backwards(input: &'static str, target: &'static str) {
        let result = super::offset_for_delete_backwards(input.len(), &input);
        if result != target.len() {
            panic!(
                "Backspacing got {:?}, expected {:?}. Index: got {result}, expected {target}",
                input.get(..result).unwrap_or("[INVALID RESULT INDEX]"),
                target
            );
        }
    }

    #[track_caller]
    fn assert_delete_backwards_seq(targets: &[&'static str]) {
        let mut ran = false;
        for val in targets.windows(2) {
            ran = true;
            assert_delete_backwards(val[0], val[1]);
        }
        if !ran {
            panic!("Didn't execute");
        }
    }

    #[test]
    #[should_panic(expected = "Backspacing got \"\", expected \"1\"")]
    fn assert_delete_backwards_invalid() {
        assert_delete_backwards("1", "1");
    }

    #[test]
    fn delete_combining_enclosing_keycaps() {
        // Including variation selector-18

        assert_delete_backwards("1\u{E0101}\u{20E3}", "");

        // multiple COMBINING ENCLOSING KEYCAP
        assert_delete_backwards_seq(&["1\u{20E3}\u{20E3}", "1\u{20E3}", ""]);

        // Isolated multiple COMBINING ENCLOSING KEYCAP
        assert_delete_backwards_seq(&["\u{20E3}\u{20E3}", "\u{20E3}", ""]);
    }

    #[test]
    fn delete_variation_selector_tests() {
        // Isolated variation selector

        assert_delete_backwards("\u{FE0F}", "");

        assert_delete_backwards("\u{E0100}", "");

        // Isolated multiple variation selectors
        assert_delete_backwards("\u{FE0F}\u{FE0F}", "\u{FE0F}");
        assert_delete_backwards("\u{FE0F}\u{E0100}", "\u{FE0F}");

        assert_delete_backwards("\u{E0100}\u{FE0F}", "\u{E0100}");
        assert_delete_backwards("\u{E0100}\u{E0100}", "\u{E0100}");

        // Multiple variation selectors
        assert_delete_backwards("#\u{FE0F}\u{FE0F}", "#\u{FE0F}");
        assert_delete_backwards("#\u{FE0F}\u{E0100}", "#\u{FE0F}");

        assert_delete_backwards("#\u{FE0F}", "");

        assert_delete_backwards("#\u{E0100}\u{FE0F}", "#\u{E0100}");
        assert_delete_backwards("#\u{E0100}\u{E0100}", "#\u{E0100}");

        assert_delete_backwards("#\u{E0100}", "");
    }

    #[test]
    fn delete_emoji_zwj_sequence_tests() {
        // U+200D is ZERO WIDTH JOINER
        assert_delete_backwards("\u{1F441}\u{200D}\u{1F5E8}", ""); // 👁‍🗨

        // U+FE0E is variation selector-15

        assert_delete_backwards("\u{1F441}\u{200D}\u{1F5E8}\u{FE0E}", "");
        // 👁‍🗨︎

        assert_delete_backwards("\u{1F469}\u{200D}\u{1F373}", "");
        // 👩‍🍳

        assert_delete_backwards("\u{1F487}\u{200D}\u{2640}", "");
        // 💇‍♀

        assert_delete_backwards("\u{1F487}\u{200D}\u{2640}\u{FE0F}", "");
        // 💇‍♀️

        assert_delete_backwards(
            "\u{1F468}\u{200D}\u{2764}\u{FE0F}\u{200D}\u{1F48B}\u{200D}\u{1F468}",
            "",
        );
        // 👨‍❤️‍💋‍👨

        // Emoji modifier can be appended to each emoji.

        assert_delete_backwards("\u{1F469}\u{1F3FB}\u{200D}\u{1F4BC}", "");
        // 👩🏻‍💼

        assert_delete_backwards(
            "\u{1F468}\u{1F3FF}\u{200D}\u{2764}\u{FE0F}\u{200D}\u{1F468}\u{1F3FB}",
            "",
        );
        // 👨🏿‍❤️‍👨🏻

        // End with ZERO WIDTH JOINER
        assert_delete_backwards_seq(&["\u{1F441}\u{200D}", "\u{1F441}", ""]); // 👁‍

        // Start with ZERO WIDTH JOINER
        assert_delete_backwards_seq(&["\u{200D}\u{1F5E8}", "\u{200D}", ""]);

        assert_delete_backwards_seq(&[
            "\u{FE0E}\u{200D}\u{1F5E8}",
            "\u{FE0E}\u{200D}",
            "\u{FE0E}",
            "",
        ]);

        // Multiple ZERO WIDTH JOINER
        assert_delete_backwards_seq(&[
            "\u{1F441}\u{200D}\u{200D}\u{1F5E8}",
            "\u{1F441}\u{200D}\u{200D}",
            "\u{1F441}\u{200D}",
            "\u{1F441}",
            "",
        ]);

        // Isolated multiple ZERO WIDTH JOINER
        assert_delete_backwards_seq(&["\u{200D}\u{200D}", "\u{200D}", ""]);
    }

    #[test]
    fn delete_flags_tests() {
        // Isolated regional indicator symbol

        assert_delete_backwards("\u{1F1FA}", "");

        // Odd numbered regional indicator symbols
        assert_delete_backwards_seq(&["\u{1F1FA}\u{1F1F8}\u{1F1FA}", "\u{1F1FA}\u{1F1F8}", ""]);

        // Incomplete sequence. (no tag_term: U+E007E)
        assert_delete_backwards_seq(&[
            "a\u{1F3F4}\u{E0067}b",
            "a\u{1F3F4}\u{E0067}",
            "a\u{1F3F4}",
            "a",
            "",
        ]);

        // No tag_base
        assert_delete_backwards_seq(&[
            "a\u{E0067}\u{E007F}b",
            "a\u{E0067}\u{E007F}",
            "a\u{E0067}",
            "a",
            "",
        ]);

        // Isolated tag chars
        assert_delete_backwards_seq(&[
            "a\u{E0067}\u{E0067}b",
            "a\u{E0067}\u{E0067}",
            "a\u{E0067}",
            "a",
            "",
        ]);

        // Isolated tab term.
        assert_delete_backwards_seq(&[
            "a\u{E007F}\u{E007F}b",
            "a\u{E007F}\u{E007F}",
            "a\u{E007F}",
            "a",
            "",
        ]);

        // Immediate tag_term after tag_base
        assert_delete_backwards_seq(&[
            "a\u{1F3F4}\u{E007F}\u{1F3F4}\u{E007F}b",
            "a\u{1F3F4}\u{E007F}\u{1F3F4}\u{E007F}",
            "a\u{1F3F4}\u{E007F}",
            "a",
            "",
        ]);
    }

    #[test]
    fn delete_emoji_modifier_tests() {
        // U+1F3FB is EMOJI MODIFIER FITZPATRICK TYPE-1-2.
        assert_delete_backwards_seq(&["\u{1F466}\u{1F3FB}", ""]);

        // Isolated emoji modifier
        assert_delete_backwards_seq(&["\u{1F3FB}", ""]);

        // Isolated multiple emoji modifier
        assert_delete_backwards_seq(&["\u{1F3FB}\u{1F3FB}", "\u{1F3FB}", ""]);

        // Multiple emoji modifiers
        assert_delete_backwards_seq(&["\u{1F466}\u{1F3FB}\u{1F3FB}", "\u{1F466}\u{1F3FB}", ""]);
    }

    #[test]
    fn delete_mixed_edge_cases_tests() {
        // COMBINING ENCLOSING KEYCAP + variation selector
        assert_delete_backwards_seq(&["1\u{20E3}\u{FE0F}", "1", ""]);

        // Variation selector + COMBINING ENCLOSING KEYCAP
        assert_delete_backwards_seq(&["\u{2665}\u{FE0F}\u{20E3}", "\u{2665}\u{FE0F}", ""]);

        // COMBINING ENCLOSING KEYCAP + ending with ZERO WIDTH JOINER
        assert_delete_backwards_seq(&["1\u{20E3}\u{200D}", "1\u{20E3}", ""]);

        // COMBINING ENCLOSING KEYCAP + ZERO WIDTH JOINER
        assert_delete_backwards_seq(&[
            "1\u{20E3}\u{200D}\u{1F5E8}",
            "1\u{20E3}\u{200D}",
            "1\u{20E3}",
            "",
        ]);

        // Start with ZERO WIDTH JOINER + COMBINING ENCLOSING KEYCAP
        assert_delete_backwards_seq(&["\u{200D}\u{20E3}", "\u{200D}", ""]);

        // ZERO WIDTH JOINER + COMBINING ENCLOSING KEYCAP
        assert_delete_backwards_seq(&[
            "\u{1F441}\u{200D}\u{20E3}",
            "\u{1F441}\u{200D}",
            "\u{1F441}",
            "",
        ]);

        // COMBINING ENCLOSING KEYCAP + regional indicator symbol
        assert_delete_backwards_seq(&["1\u{20E3}\u{1F1FA}", "1\u{20E3}", ""]);

        // Regional indicator symbol + COMBINING ENCLOSING KEYCAP
        assert_delete_backwards_seq(&["\u{1F1FA}\u{20E3}", "\u{1F1FA}", ""]);

        // COMBINING ENCLOSING KEYCAP + emoji modifier
        assert_delete_backwards_seq(&["1\u{20E3}\u{1F3FB}", "1\u{20E3}", ""]);

        // Emoji modifier + COMBINING ENCLOSING KEYCAP
        assert_delete_backwards_seq(&["\u{1F466}\u{1F3FB}\u{20E3}", "\u{1F466}\u{1F3FB}", ""]);

        // Variation selector + end with ZERO WIDTH JOINER
        assert_delete_backwards_seq(&["\u{2665}\u{FE0F}\u{200D}", "\u{2665}\u{FE0F}", ""]);

        // Variation selector + ZERO WIDTH JOINER

        assert_delete_backwards("\u{1F469}\u{200D}\u{2764}\u{FE0F}\u{200D}\u{1F469}", "");

        // Start with ZERO WIDTH JOINER + variation selector

        assert_delete_backwards("\u{200D}\u{FE0F}", "");

        // ZERO WIDTH JOINER + variation selector
        assert_delete_backwards_seq(&["\u{1F469}\u{200D}\u{FE0F}", "\u{1F469}", ""]);

        // Variation selector + regional indicator symbol
        assert_delete_backwards_seq(&["\u{2665}\u{FE0F}\u{1F1FA}", "\u{2665}\u{FE0F}", ""]);

        // Regional indicator symbol + variation selector

        assert_delete_backwards("\u{1F1FA}\u{FE0F}", "");

        // Variation selector + emoji modifier
        assert_delete_backwards_seq(&["\u{2665}\u{FE0F}\u{1F3FB}", "\u{2665}\u{FE0F}", ""]);

        // Emoji modifier + variation selector
        assert_delete_backwards_seq(&["\u{1F466}\u{1F3FB}\u{FE0F}", "\u{1F466}", ""]);

        // Start withj ZERO WIDTH JOINER + regional indicator symbol
        assert_delete_backwards_seq(&["\u{200D}\u{1F1FA}", "\u{200D}", ""]);

        // ZERO WIDTH JOINER + Regional indicator symbol
        assert_delete_backwards_seq(&[
            "\u{1F469}\u{200D}\u{1F1FA}",
            "\u{1F469}\u{200D}",
            "\u{1F469}",
            "",
        ]);

        // Regional indicator symbol + end with ZERO WIDTH JOINER
        assert_delete_backwards_seq(&["\u{1F1FA}\u{200D}", "\u{1F1FA}", ""]);

        // Regional indicator symbol + ZERO WIDTH JOINER

        assert_delete_backwards("\u{1F1FA}\u{200D}\u{1F469}", "");

        // Start with ZERO WIDTH JOINER + emoji modifier
        assert_delete_backwards_seq(&["\u{200D}\u{1F3FB}", "\u{200D}", ""]);

        // ZERO WIDTH JOINER + emoji modifier
        assert_delete_backwards_seq(&[
            "\u{1F469}\u{200D}\u{1F3FB}",
            "\u{1F469}\u{200D}",
            "\u{1F469}",
            "",
        ]);

        // Emoji modifier + end with ZERO WIDTH JOINER
        assert_delete_backwards_seq(&["\u{1F466}\u{1F3FB}\u{200D}", "\u{1F466}\u{1F3FB}", ""]);

        // Regional indicator symbol + Emoji modifier
        assert_delete_backwards_seq(&["\u{1F1FA}\u{1F3FB}", "\u{1F1FA}", ""]);

        // Emoji modifier + regional indicator symbol
        assert_delete_backwards_seq(&["\u{1F466}\u{1F3FB}\u{1F1FA}", "\u{1F466}\u{1F3FB}", ""]);

        // RIS + LF
        assert_delete_backwards_seq(&["\u{1F1E6}\u{000A}", "\u{1F1E6}", ""]);
    }
}
