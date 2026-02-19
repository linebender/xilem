// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Default values used by various widgets in their paint methods.

#![allow(missing_docs, reason = "Names are self-explanatory.")]

use parley::{GenericFamily, LineHeight};

use crate::core::{DefaultProperties, StyleProperty, StyleSet};
use crate::layout::Length;
use crate::palette::css::DIM_GRAY;
use crate::peniko::Color;
use crate::properties::{
    ActiveBackground, Background, BarColor, BorderColor, BorderWidth, CaretColor, CheckmarkColor,
    CheckmarkStrokeWidth, ContentColor, CornerRadius, Dimensions, DisabledBackground,
    DisabledCheckmarkColor, DisabledContentColor, FocusedBorderColor, Gap, HoveredBorderColor,
    Padding, PlaceholderColor, SelectionColor, ThumbColor, ThumbRadius, ToggledBackground,
    TrackThickness, UnfocusedSelectionColor,
};
use crate::widgets::{
    Badge, Button, Checkbox, DisclosureButton, Divider, Flex, Grid, Label, ProgressBar,
    RadioButton, Spinner, Switch, TextArea, TextInput,
};

/// Default color for the app background.
///
/// If the app driver does some kind beginning-of-frame clearing,
/// it should clear with this color by default.
pub const BACKGROUND_COLOR: Color = Color::from_rgb8(0x1D, 0x1D, 0x1D);

pub const BORDER_WIDTH: f64 = 1.;

// Zync color variations from https://tailwindcss.com/docs/colors
pub const ZYNC_900: Color = Color::from_rgb8(0x18, 0x18, 0x1b);
pub const ZYNC_800: Color = Color::from_rgb8(0x27, 0x27, 0x2a);
pub const ZYNC_700: Color = Color::from_rgb8(0x3f, 0x3f, 0x46);
pub const ZYNC_600: Color = Color::from_rgb8(0x52, 0x52, 0x5b);
pub const ZYNC_500: Color = Color::from_rgb8(0x71, 0x71, 0x7a);

pub const ACCENT_COLOR: Color = Color::from_rgb8(0x3b, 0x7e, 0xe4);
pub const TEXT_COLOR: Color = Color::from_rgb8(0xf2, 0xf2, 0xf2);
pub const DISABLED_TEXT_COLOR: Color = Color::from_rgb8(0xa0, 0xa0, 0x9a);
pub const PLACEHOLDER_COLOR: Color = Color::from_rgba8(0xFF, 0xFF, 0xFF, 0x8F);
pub const TEXT_BACKGROUND_COLOR: Color = Color::from_rgb8(0x16, 0x16, 0x16);
pub const FOCUS_COLOR: Color = Color::from_rgb8(0xff, 0xff, 0xff);

// TODO: The following constants are not being used in properties
pub const TEXT_SIZE_NORMAL: f32 = 15.0;
pub const BASIC_WIDGET_HEIGHT: Length = Length::const_px(18.0);
pub const BORDERED_WIDGET_HEIGHT: f64 = 24.0;
pub const SCROLLBAR_COLOR: Color = Color::from_rgb8(0xff, 0xff, 0xff);
pub const SCROLLBAR_BORDER_COLOR: Color = Color::from_rgb8(0x77, 0x77, 0x77);
pub const SCROLLBAR_WIDTH: f64 = 8.;
pub const SCROLLBAR_PAD: f64 = 2.;
pub const SCROLLBAR_MIN_SIZE: f64 = 45.;
pub const SCROLLBAR_RADIUS: f64 = 5.;
pub const SCROLLBAR_EDGE_WIDTH: f64 = 1.;
pub const DEFAULT_GAP: Length = Length::const_px(10.0);
pub const DEFAULT_SPACER_LEN: Length = Length::const_px(10.0);
pub const WIDGET_CONTROL_COMPONENT_PADDING: Length = Length::const_px(4.0);

pub fn default_property_set() -> DefaultProperties {
    let mut properties = DefaultProperties::new();

    // Badge
    properties.insert::<Badge, _>(Padding::from_vh(3., 5.));
    properties.insert::<Badge, _>(CornerRadius { radius: 999. });
    properties.insert::<Badge, _>(BorderWidth { width: 0. });
    properties.insert::<Badge, _>(Background::Color(ACCENT_COLOR));
    properties.insert::<Badge, _>(DisabledBackground(Background::Color(ZYNC_800)));
    properties.insert::<Badge, _>(BorderColor { color: ZYNC_700 });

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
    properties.insert::<Button, _>(FocusedBorderColor(BorderColor { color: FOCUS_COLOR }));

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
    properties.insert::<Checkbox, _>(FocusedBorderColor(BorderColor { color: FOCUS_COLOR }));

    properties.insert::<Checkbox, _>(CheckmarkStrokeWidth { width: 2.0 });
    properties.insert::<Checkbox, _>(CheckmarkColor { color: TEXT_COLOR });
    properties.insert::<Checkbox, _>(DisabledCheckmarkColor(CheckmarkColor {
        color: DISABLED_TEXT_COLOR,
    }));

    // DisclosureButton
    properties.insert::<DisclosureButton, _>(ContentColor::new(DIM_GRAY));
    properties.insert::<DisclosureButton, _>(Dimensions::fixed(
        Length::const_px(16.),
        Length::const_px(16.),
    ));
    properties.insert::<DisclosureButton, _>(Padding::all(4.));

    // Divider
    properties.insert::<Divider, _>(ContentColor::new(ZYNC_500));

    // Switch
    properties.insert::<Switch, _>(CornerRadius { radius: 10. }); // Full pill shape
    properties.insert::<Switch, _>(BorderWidth {
        width: BORDER_WIDTH,
    });

    properties.insert::<Switch, _>(Background::Color(ZYNC_700));
    properties.insert::<Switch, _>(ActiveBackground(Background::Color(ZYNC_600)));
    properties.insert::<Switch, _>(DisabledBackground(Background::Color(Color::BLACK)));
    properties.insert::<Switch, _>(ToggledBackground(Background::Color(ACCENT_COLOR)));
    properties.insert::<Switch, _>(BorderColor { color: ZYNC_700 });
    properties.insert::<Switch, _>(HoveredBorderColor(BorderColor { color: ZYNC_500 }));
    properties.insert::<Switch, _>(FocusedBorderColor(BorderColor { color: FOCUS_COLOR }));
    properties.insert::<Switch, _>(ThumbColor(Color::WHITE));
    properties.insert::<Switch, _>(ThumbRadius(8.0));
    properties.insert::<Switch, _>(TrackThickness(20.0));

    // Flex
    properties.insert::<Flex, _>(Gap::new(DEFAULT_GAP));

    // Grid
    properties.insert::<Grid, _>(Gap::ZERO);

    // TextInput
    properties.insert::<TextInput, _>(Padding::from_vh(6., 12.));
    properties.insert::<TextInput, _>(CornerRadius { radius: 4. });
    properties.insert::<TextInput, _>(BorderWidth {
        width: BORDER_WIDTH,
    });
    properties.insert::<TextInput, _>(BorderColor { color: ZYNC_600 });
    properties.insert::<TextInput, _>(FocusedBorderColor(BorderColor { color: FOCUS_COLOR }));
    properties.insert::<TextInput, _>(PlaceholderColor::new(PLACEHOLDER_COLOR));
    properties.insert::<TextInput, _>(CaretColor { color: TEXT_COLOR });
    properties.insert::<TextInput, _>(SelectionColor {
        color: ACCENT_COLOR,
    });
    properties.insert::<TextInput, _>(UnfocusedSelectionColor(SelectionColor {
        color: DISABLED_TEXT_COLOR,
    }));
    properties.insert::<TextInput, _>(Background::Color(TEXT_BACKGROUND_COLOR));
    properties.insert::<TextInput, _>(DisabledBackground(Background::Color(TEXT_BACKGROUND_COLOR)));

    // TextArea
    properties.insert::<TextArea<false>, _>(ContentColor::new(TEXT_COLOR));
    properties
        .insert::<TextArea<false>, _>(DisabledContentColor(ContentColor::new(DISABLED_TEXT_COLOR)));
    properties.insert::<TextArea<false>, _>(CaretColor { color: TEXT_COLOR });
    properties.insert::<TextArea<false>, _>(SelectionColor {
        color: ACCENT_COLOR,
    });
    properties.insert::<TextArea<false>, _>(UnfocusedSelectionColor(SelectionColor {
        color: DISABLED_TEXT_COLOR,
    }));
    properties.insert::<TextArea<true>, _>(ContentColor::new(TEXT_COLOR));
    properties
        .insert::<TextArea<true>, _>(DisabledContentColor(ContentColor::new(DISABLED_TEXT_COLOR)));
    properties.insert::<TextArea<true>, _>(CaretColor { color: TEXT_COLOR });
    properties.insert::<TextArea<true>, _>(SelectionColor {
        color: ACCENT_COLOR,
    });
    properties.insert::<TextArea<true>, _>(UnfocusedSelectionColor(SelectionColor {
        color: DISABLED_TEXT_COLOR,
    }));

    // Label
    properties.insert::<Label, _>(ContentColor::new(TEXT_COLOR));
    properties.insert::<Label, _>(DisabledContentColor(ContentColor::new(DISABLED_TEXT_COLOR)));

    // ProgressBar
    properties.insert::<ProgressBar, _>(CornerRadius { radius: 2. });
    properties.insert::<ProgressBar, _>(BorderWidth {
        width: BORDER_WIDTH,
    });

    properties.insert::<ProgressBar, _>(Background::Color(ZYNC_900));
    properties.insert::<ProgressBar, _>(BorderColor { color: ZYNC_800 });
    properties.insert::<ProgressBar, _>(BarColor(ACCENT_COLOR));

    // RadioButton
    properties.insert::<RadioButton, _>(BorderWidth {
        width: BORDER_WIDTH,
    });

    properties.insert::<RadioButton, _>(Background::Color(ZYNC_800));
    properties.insert::<RadioButton, _>(ActiveBackground(Background::Color(ZYNC_700)));
    properties.insert::<RadioButton, _>(DisabledBackground(Background::Color(Color::BLACK)));
    properties.insert::<RadioButton, _>(BorderColor { color: ZYNC_700 });
    properties.insert::<RadioButton, _>(HoveredBorderColor(BorderColor { color: ZYNC_500 }));
    properties.insert::<RadioButton, _>(FocusedBorderColor(BorderColor { color: FOCUS_COLOR }));

    properties.insert::<RadioButton, _>(CheckmarkColor { color: TEXT_COLOR });
    properties.insert::<RadioButton, _>(DisabledCheckmarkColor(CheckmarkColor {
        color: DISABLED_TEXT_COLOR,
    }));

    // Spinner
    properties.insert::<Spinner, _>(ContentColor::new(TEXT_COLOR));

    properties
}

/// Applies the default text styles for Masonry into `styles`.
pub fn default_text_styles(styles: &mut StyleSet) {
    styles.insert(StyleProperty::LineHeight(LineHeight::FontSizeRelative(1.2)));
    styles.insert(GenericFamily::SystemUi.into());
}

/// Set of default properties used in unit tests.
///
/// This lets us change default properties without having to reset all screenshots every time.
/// This should still be kept relatively close to `default_property_set()` so that screenshots look like end user apps.
#[cfg(test)]
pub(crate) fn test_property_set() -> DefaultProperties {
    let mut properties = default_property_set();

    const TEXT_COLOR: Color = Color::from_rgb8(0xf0, 0xf0, 0xea);
    properties.insert::<Label, _>(Padding::from_vh(0., 2.));
    properties.insert::<Checkbox, _>(CheckmarkColor { color: TEXT_COLOR });
    properties.insert::<TextArea<false>, _>(ContentColor::new(TEXT_COLOR));
    properties.insert::<TextArea<false>, _>(CaretColor { color: TEXT_COLOR });
    properties.insert::<Label, _>(ContentColor::new(TEXT_COLOR));
    properties.insert::<Spinner, _>(ContentColor::new(TEXT_COLOR));

    properties
}
