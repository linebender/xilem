// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::core::palette::css::ORANGE;
use crate::core::{NewWidget, PropertySet, layout::AsUnit as _};
use crate::properties::{ContentColor, Dimensions, Gap};

#[test]
fn widget_new_properties() {
    let harness = TestHarness::new();

    let widget = Button::new("").prepare();

    // Set Dimensions, Gap, ContentColor to some values.
    let widget = widget
        .with_props(Dimensions::STRETCH)
        .with_props(Gap::new(10.px()))
        .with_props(ContentColor::new(ORANGE));

    let props = &new_widget.properties;
    assert_eq!(props.get::<Dimensions>(), Dimensions::STRETCH);
    assert_eq!(props.get::<Gap>(), Gap::new(10.px()));
    assert_eq!(props.get::<ContentColor>(), ContentColor::new(ORANGE));

    // Override Dimensions and Gap but not ContentColor.
    let widget = widget.with_props((Dimensions::MIN, Gap::ZERO));

    let props = &new_widget.properties;
    assert_eq!(props.get::<Dimensions>(), Dimensions::MIN);
    assert_eq!(props.get::<Gap>(), Gap::ZERO);
    assert_eq!(props.get::<ContentColor>(), ContentColor::new(ORANGE));
}
