// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry_core::core::ResizeDirection;
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
