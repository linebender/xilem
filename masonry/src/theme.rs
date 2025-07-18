// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Default values used by various widgets in their paint methods.

#![allow(missing_docs, reason = "Names are self-explanatory.")]

use parley::{GenericFamily, LineHeight};

use crate::core::{DefaultProperties, StyleProperty, StyleSet};
use crate::peniko::Color;
use crate::properties::{
    ActiveBackground, Background, BorderColor, BorderWidth, CheckmarkColor, CheckmarkStrokeWidth,
    CornerRadius, DisabledBackground, DisabledCheckmarkColor, DisabledTextColor,
    HoveredBorderColor, Padding, TextColor,
};
use crate::widgets::{Button, Checkbox, Label, TextArea, TextInput};

pub const BORDER_WIDTH: f64 = 1.;

// Zync color variations from https://tailwindcss.com/docs/colors
pub const ZYNC_900: Color = Color::from_rgb8(0x18, 0x18, 0x1b);
pub const ZYNC_800: Color = Color::from_rgb8(0x27, 0x27, 0x2a);
pub const ZYNC_700: Color = Color::from_rgb8(0x3f, 0x3f, 0x46);
pub const ZYNC_600: Color = Color::from_rgb8(0x52, 0x52, 0x5b);
pub const ZYNC_500: Color = Color::from_rgb8(0x71, 0x71, 0x7a);

pub const ACCENT_COLOR: Color = Color::from_rgb8(0x3b, 0x7e, 0xe4);
pub const TEXT_COLOR: Color = Color::from_rgb8(0xf0, 0xf0, 0xea);
pub const DISABLED_TEXT_COLOR: Color = Color::from_rgb8(0xa0, 0xa0, 0x9a);

// TODO: The following constants are not being used in properties
pub const TEXT_SIZE_NORMAL: f32 = 15.0;
pub const BASIC_WIDGET_HEIGHT: f64 = 18.0;
pub const BORDERED_WIDGET_HEIGHT: f64 = 24.0;
pub const SCROLLBAR_COLOR: Color = Color::from_rgb8(0xff, 0xff, 0xff);
pub const SCROLLBAR_BORDER_COLOR: Color = Color::from_rgb8(0x77, 0x77, 0x77);
pub const SCROLLBAR_WIDTH: f64 = 8.;
pub const SCROLLBAR_PAD: f64 = 2.;
pub const SCROLLBAR_MIN_SIZE: f64 = 45.;
pub const SCROLLBAR_RADIUS: f64 = 5.;
pub const SCROLLBAR_EDGE_WIDTH: f64 = 1.;
pub const WIDGET_PADDING_VERTICAL: f64 = 10.0;
pub const WIDGET_PADDING_HORIZONTAL: f64 = 8.0;
pub const WIDGET_CONTROL_COMPONENT_PADDING: f64 = 4.0;

pub fn default_property_set() -> DefaultProperties {
    let mut properties = DefaultProperties::new();

    // Button
    properties.insert::<Button, _>(Padding::from_vh(6., 16.));
    properties.insert::<Button, _>(CornerRadius { radius: 6. });
    properties.insert::<Button, _>(BorderWidth {
        width: BORDER_WIDTH,
    });

    properties.insert::<Button, _>(Background::Color(ZYNC_800));
    properties.insert::<Button, _>(ActiveBackground(Background::Color(ZYNC_700)));
    properties.insert::<Button, _>(DisabledBackground(Background::Color(Color::BLACK)));
    properties.insert::<Button, _>(BorderColor { color: ZYNC_700 });
    properties.insert::<Button, _>(HoveredBorderColor(BorderColor { color: ZYNC_500 }));

    // Checkbox
    properties.insert::<Checkbox, _>(CornerRadius { radius: 4. });
    properties.insert::<Checkbox, _>(BorderWidth {
        width: BORDER_WIDTH,
    });

    properties.insert::<Checkbox, _>(Background::Color(ZYNC_800));
    properties.insert::<Checkbox, _>(ActiveBackground(Background::Color(ZYNC_700)));
    properties.insert::<Checkbox, _>(DisabledBackground(Background::Color(Color::BLACK)));
    properties.insert::<Checkbox, _>(BorderColor { color: ZYNC_700 });
    properties.insert::<Checkbox, _>(HoveredBorderColor(BorderColor { color: ZYNC_500 }));

    properties.insert::<Checkbox, _>(CheckmarkStrokeWidth { width: 2.0 });
    properties.insert::<Checkbox, _>(CheckmarkColor { color: TEXT_COLOR });
    properties.insert::<Checkbox, _>(DisabledCheckmarkColor(CheckmarkColor {
        color: DISABLED_TEXT_COLOR,
    }));

    // TextInput
    properties.insert::<TextInput, _>(Padding::from_vh(6., 12.));
    properties.insert::<TextInput, _>(CornerRadius { radius: 4. });
    properties.insert::<TextInput, _>(BorderWidth {
        width: BORDER_WIDTH,
    });

    properties.insert::<TextInput, _>(BorderColor { color: ZYNC_600 });

    // TextArea
    properties.insert::<TextArea<false>, _>(TextColor::new(TEXT_COLOR));
    properties.insert::<TextArea<false>, _>(DisabledTextColor(TextColor::new(DISABLED_TEXT_COLOR)));
    properties.insert::<TextArea<true>, _>(TextColor::new(TEXT_COLOR));
    properties.insert::<TextArea<true>, _>(DisabledTextColor(TextColor::new(DISABLED_TEXT_COLOR)));

    // Label
    properties.insert::<Label, _>(TextColor::new(TEXT_COLOR));
    properties.insert::<Label, _>(DisabledTextColor(TextColor::new(DISABLED_TEXT_COLOR)));

    properties
}

/// Applies the default text styles for Masonry into `styles`.
pub fn default_text_styles(styles: &mut StyleSet) {
    styles.insert(StyleProperty::LineHeight(LineHeight::FontSizeRelative(1.2)));
    styles.insert(GenericFamily::SystemUi.into());
}
