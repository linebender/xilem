// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

// Most of this file is copy-pasted directly from
// https://github.com/DioxusLabs/blitz/blob/main/packages/blitz-shell/src/convert_events.rs
// Should be removed once https://github.com/rust-windowing/winit/pull/4026 is merged.

use masonry::core::{Ime, ResizeDirection};
use winit::event::Ime as WinitIme;
use winit::window::ResizeDirection as WinitResizeDirection;

pub(crate) fn masonry_resize_direction_to_winit(dir: ResizeDirection) -> WinitResizeDirection {
    match dir {
        ResizeDirection::East => WinitResizeDirection::East,
        ResizeDirection::North => WinitResizeDirection::North,
        ResizeDirection::NorthEast => WinitResizeDirection::NorthEast,
        ResizeDirection::NorthWest => WinitResizeDirection::NorthWest,
        ResizeDirection::South => WinitResizeDirection::South,
        ResizeDirection::SouthEast => WinitResizeDirection::SouthEast,
        ResizeDirection::SouthWest => WinitResizeDirection::SouthWest,
        ResizeDirection::West => WinitResizeDirection::West,
    }
}

pub(crate) fn winit_ime_to_masonry(event: WinitIme) -> Ime {
    match event {
        WinitIme::Enabled => Ime::Enabled,
        WinitIme::Disabled => Ime::Disabled,
        WinitIme::Preedit(text, cursor) => Ime::Preedit(text, cursor),
        WinitIme::Commit(text) => Ime::Commit(text),
    }
}
