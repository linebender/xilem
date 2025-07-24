// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! This module imports widget code from most of the examples.
//!
//! When running integration tests, it will compare the rendered snapshots of the widgets
//! with the expected snapshots.

#[cfg(test)]
#[allow(dead_code, reason = "We don't need to run the main functions.")]
#[allow(missing_docs, reason = "Example code doesn't need docs.")]
#[path = "../examples"]
pub mod others {
    pub mod calc_masonry;
    pub mod custom_widget;
    pub mod grid_masonry;
    pub mod simple_image;
    pub mod to_do_list;
}

#[cfg(test)]
mod tests {
    use super::*;
    use masonry::core::Widget;
    use masonry::kurbo::Size;
    use masonry::theme::default_property_set;
    use masonry_testing::{TestHarness, TestHarnessParams};

    // A series of tests to check that various widgets (the ones used in examples)
    // can handle being laid out and painted with a size of zero.

    const PARAMS_ZERO_SIZE: TestHarnessParams = {
        let mut params = TestHarnessParams::DEFAULT;
        params.window_size = Size::ZERO;
        params
    };

    #[test]
    fn zero_size_calc_masonry() {
        let mut harness = TestHarness::create_with(
            default_property_set(),
            others::calc_masonry::build_calc(),
            PARAMS_ZERO_SIZE,
        );
        let _ = harness.render();
    }

    #[test]
    fn zero_size_custom_widget() {
        let mut harness = TestHarness::create_with(
            default_property_set(),
            others::custom_widget::CustomWidget("Foobar".to_string()).with_auto_id(),
            PARAMS_ZERO_SIZE,
        );
        let _ = harness.render();
    }

    #[test]
    fn zero_size_grid_masonry() {
        let mut harness = TestHarness::create_with(
            default_property_set(),
            others::grid_masonry::make_grid(1.0).with_auto_id(),
            PARAMS_ZERO_SIZE,
        );
        let _ = harness.render();
    }

    #[test]
    fn zero_size_simple_image() {
        let mut harness = TestHarness::create_with(
            default_property_set(),
            others::simple_image::make_image().with_auto_id(),
            PARAMS_ZERO_SIZE,
        );
        let _ = harness.render();
    }

    #[test]
    fn zero_size_to_do_list() {
        let mut harness = TestHarness::create_with(
            default_property_set(),
            others::to_do_list::make_widget_tree().with_auto_id(),
            PARAMS_ZERO_SIZE,
        );
        let _ = harness.render();
    }
}
