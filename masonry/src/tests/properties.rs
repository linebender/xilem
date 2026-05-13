// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::core::Widget as _;
use crate::kurbo::Rect;
use crate::layout::AsUnit;
use crate::palette::css::BLUE;
use crate::properties::{ContentColor, Dimensions, Gap, ObjectFit};
use crate::tests::assert_rect_approx_eq;
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

#[test]
fn object_fit_affine_stretch_maps_rect_to_rect() {
    let container = Rect::new(10., -20., 110., 30.);
    let content = Rect::new(-5., 10., 15., 20.);

    let transform = ObjectFit::Stretch.affine(container, content);

    assert_rect_approx_eq(
        "transformed",
        transform.transform_rect_bbox(content),
        container,
    );
}

#[test]
fn object_fit_affine_contain_handles_negative_origins() {
    let container = Rect::new(-30., -20., 70., 30.);
    let content = Rect::new(-10., -5., 10., 15.);

    let transform = ObjectFit::Contain.affine(container, content);

    assert_rect_approx_eq(
        "transformed",
        transform.transform_rect_bbox(content),
        Rect::new(-5., -20., 45., 30.),
    );
}
