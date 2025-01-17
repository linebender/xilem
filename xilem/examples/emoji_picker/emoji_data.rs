// Copyright 2013 the Xilem Authors and Cal Henderson
// SPDX-License-Identifier: MIT

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

use crate::{e, EmojiInfo};

/// Data adapted as a subset of <https://github.com/iamcal/emoji-data>
/// under the MIT License. Full license text can be found above in the source file.
pub(crate) const EMOJI: &[EmojiInfo] = &[
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
