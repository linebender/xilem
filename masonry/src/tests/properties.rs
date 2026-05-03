// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::core::Widget as _;
use crate::layout::AsUnit;
use crate::palette::css::BLUE;
use crate::properties::{ContentColor, Dimensions, Gap};
use crate::widgets::Button;

#[test]
fn widget_new_properties() {
    let widget = Button::with_text("").prepare();

    // Set Dimensions, Gap, ContentColor to some values.
    let widget = widget
        .with_props(Dimensions::STRETCH)
        .with_props(Gap::new(10.px()))
        .with_props(ContentColor::new(BLUE));

    let props = &widget.properties;
    assert_eq!(props.get::<Dimensions>(), Some(&Dimensions::STRETCH));
    assert_eq!(props.get::<Gap>(), Some(&Gap::new(10.px())));
    assert_eq!(props.get::<ContentColor>(), Some(&ContentColor::new(BLUE)));

    // Override Dimensions and Gap but not ContentColor.
    let widget = widget.with_props((Dimensions::MIN, Gap::ZERO));

    let props = &widget.properties;
    assert_eq!(props.get::<Dimensions>(), Some(&Dimensions::MIN));
    assert_eq!(props.get::<Gap>(), Some(&Gap::ZERO));
    assert_eq!(props.get::<ContentColor>(), Some(&ContentColor::new(BLUE)));
}
