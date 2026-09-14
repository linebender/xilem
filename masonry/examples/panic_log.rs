// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Demonstrates the panic-log buffering: a widget panics inside its `layout`
//! method (which runs during the layout pass, inside `run_rewrite_passes`).
//! The panic-buffer tracing subscriber flushes that frame's events to a file
//! in `$TMPDIR` named `masonry-{ts}-{pid}-panic.log`.

#![cfg_attr(not(test), windows_subsystem = "windows")]

use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, AccessEvent, ChildrenIds, ErasedAction, EventCtx, LayoutCtx, MeasureCtx, NewWidget,
    NoAction, PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, TextEvent, Widget,
    WidgetId,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Size};
use masonry::layout::LenReq;
use masonry::theme::default_property_set;
use masonry_winit::app::{AppDriver, DriverCtx, NewWindow, WindowId};
use masonry_winit::winit::window::Window;
use tracing::{Span, info, trace_span};

struct Driver;

impl AppDriver for Driver {
    fn on_action(
        &mut self,
        _window_id: WindowId,
        _ctx: &mut DriverCtx<'_, '_>,
        _widget_id: WidgetId,
        _action: ErasedAction,
    ) {
    }
}

/// A leaf widget that emits a tracing event and then panics in `layout`,
/// which runs during the layout pass inside `run_rewrite_passes`.
struct PanicWidget;

impl Widget for PanicWidget {
    type Action = NoAction;

    fn on_pointer_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &PointerEvent,
    ) {
    }

    fn on_text_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &TextEvent,
    ) {
    }

    fn on_access_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &AccessEvent,
    ) {
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        _axis: Axis,
        _len_req: LenReq,
        _cross_length: Option<f64>,
    ) -> f64 {
        0.0
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {
        info!("PanicWidget::layout called — about to simulate a widget bug");
        panic!("simulated widget bug during layout pass");
    }

    fn paint(
        &mut self,
        _ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        _painter: &mut Painter<'_>,
    ) {
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("PanicWidget", id = id.trace())
    }
}

fn main() {
    let window_attributes = Window::default_attributes().with_title("Panic-log demo");

    masonry_winit::app::run(
        vec![NewWindow::new(
            window_attributes,
            NewWidget::new(PanicWidget).erased(),
        )],
        Driver,
        default_property_set(),
    )
    .unwrap();
}

// --- MARK: TESTS

#[cfg(test)]
mod tests {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    use masonry_testing::TestHarness;

    use super::*;

    /// Drives the panic widget through the layout pass synchronously and
    /// asserts that the panic-buffer tracing layer wrote a panic log file
    /// containing the in-pass tracing event.
    ///
    /// The harness installs the panic hook indirectly via
    /// `try_init_test_tracing`. `AssertUnwindSafe` is sound because the
    /// harness is dropped (not reused) before any state is read back.
    #[test]
    fn panic_log_file_is_written() {
        // Snapshot existing `masonry-*-panic.log` files before the run so we
        // can identify the new one by set difference. Filename-only matching
        // (e.g. by PID suffix) is unreliable: PIDs are recycled and a stale
        // file from a prior process with the same PID would collide.
        let pid_suffix = format!("-{}-panic.log", std::process::id());
        let snapshot = panic_log_paths(&pid_suffix);

        let result = catch_unwind(AssertUnwindSafe(|| {
            let mut harness =
                TestHarness::create(default_property_set(), NewWidget::new(PanicWidget));
            harness.render();
        }));
        assert!(
            result.is_err(),
            "expected the layout-pass panic to propagate"
        );

        let new_paths: Vec<_> = panic_log_paths(&pid_suffix)
            .into_iter()
            .filter(|p| !snapshot.contains(p))
            .collect();
        assert_eq!(
            new_paths.len(),
            1,
            "expected exactly one new masonry-*{pid_suffix} file in $TMPDIR; got {new_paths:?}"
        );

        let log_path = &new_paths[0];
        let contents = std::fs::read_to_string(log_path).unwrap();

        // Remove the file before asserting on its contents, so a failing
        // assertion doesn't leave a stale artifact in $TMPDIR.
        let _ = std::fs::remove_file(log_path);

        assert!(
            contents.contains("about to simulate a widget bug"),
            "panic log should contain the buffered tracing event; got:\n{contents}"
        );
    }

    fn panic_log_paths(pid_suffix: &str) -> std::collections::HashSet<std::path::PathBuf> {
        std::fs::read_dir(std::env::temp_dir())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().ends_with(pid_suffix))
            .map(|e| e.path())
            .collect()
    }
}
