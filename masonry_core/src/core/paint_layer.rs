// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

/// How the current widget subtree should be represented in the current paint pass.
///
/// This controls how Masonry records the widget subtree into the current
/// [`VisualLayerPlan`](crate::app::VisualLayerPlan). Current hosts still flatten
/// these scene layers back together, so changing this does not yet change runtime
/// presentation behavior.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PaintLayerMode {
    /// Paint into the current scene layer.
    #[default]
    Inline,
    /// Record this widget subtree as an isolated scene layer.
    ///
    /// The subtree still paints in normal painter order. If nested isolated scene
    /// layers occur, Masonry will split the surrounding scene as needed to preserve
    /// that order in the flattened visual-layer plan.
    IsolatedScene,
    /// Record this widget subtree as an external placeholder layer.
    ///
    /// Current hosts do not realize these placeholders yet; compatibility consumers
    /// simply skip them while flattening scene content. This mode exists so the core
    /// paint model can represent external boundaries before host integration lands.
    External,
}
