// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Default values used by various widgets in their paint methods.

#![allow(missing_docs, reason = "Names are self-explanatory.")]

use crate::core::DefaultProperties;
use crate::kurbo::Insets;
use crate::peniko::Color;
use crate::properties::types::Gradient;
use crate::properties::{Background, BorderColor, BorderWidth, CornerRadius, Padding};
use crate::widgets::Button;

// Colors are from https://sashat.me/2017/01/11/list-of-20-simple-distinct-colors/
// They're picked for visual distinction and accessibility (99 percent)

pub const WINDOW_BACKGROUND_COLOR: Color = Color::from_rgb8(0x29, 0x29, 0x29);
pub const TEXT_COLOR: Color = Color::from_rgb8(0xf0, 0xf0, 0xea);
pub const DISABLED_TEXT_COLOR: Color = Color::from_rgb8(0xa0, 0xa0, 0x9a);
pub const PLACEHOLDER_COLOR: Color = Color::from_rgb8(0x80, 0x80, 0x80);
pub const PRIMARY_LIGHT: Color = Color::from_rgb8(0x5c, 0xc4, 0xff);
pub const PRIMARY_DARK: Color = Color::from_rgb8(0x00, 0x8d, 0xdd);
pub const PROGRESS_BAR_RADIUS: f64 = 4.;
pub const BACKGROUND_LIGHT: Color = Color::from_rgb8(0x3a, 0x3a, 0x3a);
pub const BACKGROUND_DARK: Color = Color::from_rgb8(0x31, 0x31, 0x31);
pub const FOREGROUND_LIGHT: Color = Color::from_rgb8(0xf9, 0xf9, 0xf9);
pub const FOREGROUND_DARK: Color = Color::from_rgb8(0xbf, 0xbf, 0xbf);
pub const DISABLED_FOREGROUND_LIGHT: Color = Color::from_rgb8(0x89, 0x89, 0x89);
pub const DISABLED_FOREGROUND_DARK: Color = Color::from_rgb8(0x6f, 0x6f, 0x6f);
pub const BUTTON_DARK: Color = Color::BLACK;
pub const BUTTON_LIGHT: Color = Color::from_rgb8(0x21, 0x21, 0x21);
pub const DISABLED_BUTTON_DARK: Color = Color::from_rgb8(0x28, 0x28, 0x28);
pub const DISABLED_BUTTON_LIGHT: Color = Color::from_rgb8(0x38, 0x38, 0x38);
pub const BUTTON_BORDER_RADIUS: f64 = 4.;
pub const BUTTON_BORDER_WIDTH: f64 = 2.;
pub const BORDER_DARK: Color = Color::from_rgb8(0x3a, 0x3a, 0x3a);
pub const BORDER_LIGHT: Color = Color::from_rgb8(0xa1, 0xa1, 0xa1);
pub const SELECTED_TEXT_BACKGROUND_COLOR: Color = Color::from_rgb8(0x43, 0x70, 0xA8);
pub const SELECTED_TEXT_INACTIVE_BACKGROUND_COLOR: Color = Color::from_rgb8(0x74, 0x74, 0x74);
pub const SELECTION_TEXT_COLOR: Color = Color::from_rgb8(0x00, 0x00, 0x00);
pub const CURSOR_COLOR: Color = Color::WHITE;
pub const TEXT_SIZE_NORMAL: f32 = 15.0;
pub const TEXT_SIZE_LARGE: f32 = 24.0;
pub const BASIC_WIDGET_HEIGHT: f64 = 18.0;
pub const WIDE_WIDGET_WIDTH: f64 = 100.;
pub const BORDERED_WIDGET_HEIGHT: f64 = 24.0;
pub const TEXTBOX_BORDER_RADIUS: f64 = 2.;
pub const TEXTBOX_BORDER_WIDTH: f64 = 1.;
pub const TEXTBOX_INSETS: Insets = Insets::new(4.0, 4.0, 4.0, 4.0);
pub const SCROLLBAR_COLOR: Color = Color::from_rgb8(0xff, 0xff, 0xff);
pub const SCROLLBAR_BORDER_COLOR: Color = Color::from_rgb8(0x77, 0x77, 0x77);
pub const SCROLLBAR_MAX_OPACITY: f64 = 0.7;
pub const SCROLLBAR_FADE_DELAY: u64 = 1500;
pub const SCROLLBAR_WIDTH: f64 = 8.;
pub const SCROLLBAR_PAD: f64 = 2.;
pub const SCROLLBAR_MIN_SIZE: f64 = 45.;
pub const SCROLLBAR_RADIUS: f64 = 5.;
pub const SCROLLBAR_EDGE_WIDTH: f64 = 1.;
pub const WIDGET_PADDING_VERTICAL: f64 = 10.0;
pub const WIDGET_PADDING_HORIZONTAL: f64 = 8.0;
pub const WIDGET_CONTROL_COMPONENT_PADDING: f64 = 4.0;

static DEBUG_COLOR: &[Color] = &[
    Color::from_rgb8(230, 25, 75),
    Color::from_rgb8(60, 180, 75),
    Color::from_rgb8(255, 225, 25),
    Color::from_rgb8(0, 130, 200),
    Color::from_rgb8(245, 130, 48),
    Color::from_rgb8(70, 240, 240),
    Color::from_rgb8(240, 50, 230),
    Color::from_rgb8(250, 190, 190),
    Color::from_rgb8(0, 128, 128),
    Color::from_rgb8(230, 190, 255),
    Color::from_rgb8(170, 110, 40),
    Color::from_rgb8(255, 250, 200),
    Color::from_rgb8(128, 0, 0),
    Color::from_rgb8(170, 255, 195),
    Color::from_rgb8(0, 0, 128),
    Color::from_rgb8(128, 128, 128),
    Color::from_rgb8(255, 255, 255),
    Color::from_rgb8(0, 0, 0),
];

/// A color used for debug painting.
///
/// The same color is always returned given the same id, usually the id of a widget.
/// When painting a widget, [`PaintCtx::debug_color`][crate::core::PaintCtx::debug_color] is typically used instead.
pub fn get_debug_color(id: u64) -> Color {
    let color_num = id as usize % DEBUG_COLOR.len();
    DEBUG_COLOR[color_num]
}

pub fn default_property_set() -> DefaultProperties {
    let mut properties = DefaultProperties::new();

    properties.insert::<Button, _>(BorderColor { color: BORDER_DARK });
    properties.insert::<Button, _>(BorderWidth {
        width: BUTTON_BORDER_WIDTH,
    });
    properties.insert::<Button, _>(CornerRadius {
        radius: BUTTON_BORDER_RADIUS,
    });
    // NOTE: these padding values are chosen to match the existing look of TextBox;
    // they should be reevaluated at some point.
    properties.insert::<Button, _>(Padding::from_vh(2., 8.));
    properties.insert::<Button, _>(Background::Gradient(
        Gradient::new_linear(0.0).with_stops([BUTTON_LIGHT, BUTTON_DARK]),
    ));

    // TODO - Add default Padding to RootWidget?

    properties
}
