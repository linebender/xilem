// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#![expect(missing_docs, reason = "Bench crate")]

use divan::Bencher;
use masonry::core::{CollectionWidget, NewWidget};
use masonry::theme::default_property_set;
use masonry::widgets::{Flex, FlexParams, Label};
use masonry_testing::{TestHarness, TestHarnessParams};

// TODO - Runtime is currently dominated by wgpu setup overhead.
// TODO - `args = 100_000` panics.

#[divan::bench(args = [100, 1_000, 10_000])]
fn widget_list(bencher: Bencher<'_, '_>, children: u64) {
    let bencher = bencher.with_inputs(|| {
        let root_widget = NewWidget::new(Flex::column());
        TestHarness::create_with(
            default_property_set(),
            root_widget,
            TestHarnessParams::default(),
        )
    });

    bencher.bench_refs(|harness| {
        harness.edit_root_widget(|mut root| {
            for _ in 0..children {
                Flex::add(
                    &mut root,
                    NewWidget::new(Label::new("Hello")),
                    FlexParams::default(),
                );
            }
        });

        let _ = harness.render();
    });
}

// ---

fn main() {
    // Run registered benchmarks.
    divan::main();
}
