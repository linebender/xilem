// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! This module imports widget code from other examples.
//!
//! When running unit tests, it will compare the rendered snapshots of the widgets
//! with the expected snapshots.

fn main() {
    println!("This example is only used to compile other examples.");
}

#[cfg(test)]
#[allow(dead_code, reason = "We don't need to run the main functions.")]
#[allow(missing_docs, reason = "Example code doesn't need docs.")]
#[path = "."]
pub mod others {
    pub mod calc_masonry;
    pub mod custom_widget;
    pub mod grid_masonry;
    pub mod simple_image;
    pub mod to_do_list;
}

// --- MARK: TESTS ---
#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;
    use masonry::assert_render_snapshot;
    use masonry::testing::TestHarness;

    use super::others::*;

    #[test]
    fn calc_masonry_screenshot() {
        let mut harness = TestHarness::create(calc_masonry::build_calc());
        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "calc_masonry_initial");

        // TODO - Test clicking buttons
    }

    #[test]
    fn custom_widget_screenshot() {
        let my_string = "Masonry + Vello".to_string();

        let mut harness = TestHarness::create(custom_widget::CustomWidget(my_string));
        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "custom_widget_initial");
    }

    #[test]
    fn grid_masonry_screenshot() {
        let mut harness = TestHarness::create(grid_masonry::make_grid(1.0));
        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "grid_masonry_initial");

        // TODO - Test clicking buttons
    }

    #[test]
    fn simple_image_screenshot() {
        let mut harness = TestHarness::create(simple_image::make_image());
        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "simple_image_initial");
    }

    #[test]
    fn to_do_list_screenshot() {
        let mut harness = TestHarness::create(to_do_list::make_widget_tree());
        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "to_do_list_initial");

        // TODO - Test clicking buttons
        // TODO - Test typing text
    }
}
