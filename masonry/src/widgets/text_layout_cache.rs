// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A cache of text layouts, shared by the text widgets.

use smallvec::SmallVec;

use crate::core::{BrushIndex, StyleSet};
use crate::parley::{FontContext, Layout, LayoutContext};
use crate::{TextAlign, TextAlignOptions};

/// Text layout computation inputs and output.
pub(crate) struct TextLayout {
    /// Computed text layout.
    pub(crate) layout: Layout<BrushIndex>,
    /// Max advance value that was used when calculating this layout.
    max_advance: Option<f32>,
    /// Text alignment of this layout.
    alignment: TextAlign,
    /// Text alignment width of this layout.
    alignment_width: f32,
    /// Last use timestamp for cache eviction purposes.
    last_used: u8,
}

impl TextLayout {
    /// Text layout width differences less than 0.01 pixels can be considered equal.
    const EPSILON: f32 = 0.01;

    /// Creates a new [`TextLayout`] with the specified `max_advance` constraint and `timestamp`.
    fn new(max_advance: Option<f32>, timestamp: u8) -> Self {
        Self {
            layout: Layout::new(),
            max_advance,
            alignment: TextAlign::Start,
            alignment_width: -1., // Not aligned yet
            last_used: timestamp,
        }
    }

    /// Discards this layout, making it an obvious choice for cache eviction.
    fn discard(&mut self) {
        // Mark it as least recently used.
        self.last_used = 0;
        // Set a fake unlikely-to-be-seen max advance so it won't get any use.
        self.max_advance = Some(-1.);
    }

    /// Align the text layout.
    ///
    /// This method ensures that alignment only happens when the inputs have changed.
    pub(crate) fn align(&mut self, alignment: TextAlign, alignment_width: f32) {
        if self.alignment == alignment
            && (self.alignment_width - alignment_width).abs() < Self::EPSILON
        {
            return;
        }
        self.alignment = alignment;
        self.alignment_width = alignment_width;
        self.layout.align(
            Some(self.alignment_width),
            self.alignment,
            TextAlignOptions::default(),
        );
    }

    /// Returns `true` if this layout would be the result for `max_advance`.
    ///
    /// For example:
    ///
    /// `compute_layout(max_advance == 10) => layout.width == 8`
    /// is also valid for `max_advance == 9`.
    ///
    /// This check assumes that Parley does greedy line-breaking,
    /// which it does at the time of writing this.
    fn satisfies(&self, max_advance: Option<f32>) -> bool {
        // Check if the specified constraint is compatible with this layout's constraint.
        self.max_advance.is_none_or(|layout_max_advance| {
            max_advance.is_some_and(|max_advance| {
                layout_max_advance - max_advance + Self::EPSILON >= 0.
            })
        }) &&
        // Check if the computed layout fits into the specified constraint.
        max_advance.is_none_or(|max_advance| {
            max_advance - self.layout.width() + Self::EPSILON >= 0.
        })
    }

    /// Returns `true` if this layout was more constrained than `max_advance`.
    fn more_constrained(&self, max_advance: Option<f32>) -> bool {
        self.max_advance.is_some_and(|layout_max_advance| {
            max_advance.is_none_or(|max_advance| layout_max_advance < max_advance)
        })
    }

    /// Returns `true` if the layouts are equal.
    ///
    /// That is if they have the same number of line breaks with the same reason at the same places.
    fn equals(&self, other: &Self) -> bool {
        if self.layout.len() != other.layout.len() {
            return false;
        }
        let mut a = self.layout.lines();
        let mut b = other.layout.lines();
        loop {
            match (a.next(), b.next()) {
                (None, None) => return true,
                (Some(a_line), Some(b_line)) => {
                    if a_line.break_reason() != b_line.break_reason() {
                        return false;
                    }
                    if a_line.text_range() != b_line.text_range() {
                        return false;
                    }
                }
                _ => return false,
            }
        }
    }
}

/// A cache of [`TextLayouts`](TextLayout) for one piece of text.
///
/// Entries are keyed by their `max_advance` constraint, so within a layout pass
/// measurements at different constraints and the final layout can share entries.
/// The cache must be [cleared](Self::clear) whenever text, styles, or fonts change.
pub(crate) struct TextLayoutCache {
    /// Cached layouts.
    layouts: Vec<TextLayout>,
    /// Time tracking for cache usage.
    cache_time: u8,
    /// The currently active layout index.
    ///
    /// `usize::MAX` works well for none, as it will be overwritten before its used
    /// for layout access, but will be read as-is during cache eviction.
    /// During which any value larger than the cache capacity will be ignored.
    active_layout: usize,
}

impl TextLayoutCache {
    /// Total number of text layouts to cache.
    ///
    /// Must be at least `2`, to allow for one active layout and one speculative one.
    /// Must be less than `u8::MAX` because it's also used as the cache time reset value.
    const CACHE_CAPACITY: usize = 5;

    /// Creates a new empty cache.
    pub(crate) fn new() -> Self {
        Self {
            layouts: Vec::new(),
            cache_time: 0,
            active_layout: usize::MAX,
        }
    }

    /// Clears the cache.
    ///
    /// Call this whenever text, styles, or fonts have changed.
    pub(crate) fn clear(&mut self) {
        self.layouts.clear();
        self.active_layout = usize::MAX;
    }

    /// Returns the cached layout at `idx`.
    pub(crate) fn get(&self, idx: usize) -> &TextLayout {
        &self.layouts[idx]
    }

    /// Marks the layout at `idx` as the active one, protecting it from cache eviction.
    ///
    /// The active layout is the one that gets painted, so it must stay cached
    /// even while speculative measurements churn through the other entries.
    pub(crate) fn set_active(&mut self, idx: usize) {
        self.active_layout = idx;
    }

    /// Returns the active layout.
    ///
    /// # Panics
    ///
    /// Panics if no active layout has been [set](Self::set_active) since the
    /// cache was created or [cleared](Self::clear).
    pub(crate) fn active(&self) -> &TextLayout {
        &self.layouts[self.active_layout]
    }

    /// Returns the active layout mutably.
    ///
    /// # Panics
    ///
    /// Panics if no active layout has been [set](Self::set_active) since the
    /// cache was created or [cleared](Self::clear).
    pub(crate) fn active_mut(&mut self) -> &mut TextLayout {
        &mut self.layouts[self.active_layout]
    }

    /// Increments and returns the cache timestamp.
    fn cache_time(&mut self) -> u8 {
        if self.cache_time == u8::MAX {
            // Compress all last_used timestamps
            let n = self.layouts.len();
            let mut idx: SmallVec<[usize; Self::CACHE_CAPACITY]> = (0..n).collect();
            idx.sort_unstable_by_key(|&i| self.layouts[i].last_used);
            for (rank, &i) in idx.iter().enumerate() {
                self.layouts[i].last_used = rank as u8;
            }
            self.cache_time = Self::CACHE_CAPACITY as u8;
        } else {
            self.cache_time += 1;
        }
        self.cache_time
    }

    /// Builds the text layout and breaks the text into lines.
    ///
    /// Backed by a cache layer.
    pub(crate) fn build_and_break(
        &mut self,
        font_ctx: &mut FontContext,
        layout_ctx: &mut LayoutContext<BrushIndex>,
        text: &str,
        styles: &StyleSet,
        max_advance: Option<f32>,
    ) -> usize {
        let timestamp = self.cache_time();

        // Check if the cache already has a suitable entry.
        // A suitable entry is one that was calculated with the same or larger constraint,
        // and resulted in a layout that still fits within this newly requested constraint.
        for (idx, layout) in self.layouts.iter_mut().enumerate() {
            if layout.satisfies(max_advance) {
                layout.last_used = timestamp;
                return idx;
            }
        }

        // No known compatible cache entry, so need to do text layout.
        let (mut idx, layout) = if self.layouts.len() < Self::CACHE_CAPACITY {
            // Create a new cache entry.
            self.layouts.push(TextLayout::new(max_advance, timestamp));
            (self.layouts.len() - 1, self.layouts.last_mut().unwrap())
        } else {
            // Repurpose the least recently used non-active cache entry.
            let (idx, layout) = self
                .layouts
                .iter_mut()
                .enumerate()
                .filter(|(idx, _)| *idx != self.active_layout)
                .min_by(|a, b| a.1.last_used.cmp(&b.1.last_used))
                .unwrap();
            layout.max_advance = max_advance;
            layout.last_used = timestamp;
            (idx, layout)
        };

        // TODO: Should we use a different scale?
        // See https://github.com/linebender/xilem/issues/1264
        let mut builder = layout_ctx.ranged_builder(font_ctx, text, 1.0, true);
        for prop in styles.inner().values() {
            builder.push_default(prop.to_owned());
        }
        builder.build_into(&mut layout.layout, text);

        layout.layout.break_all_lines(max_advance);

        // Check if the layout result matches an existing cache entry.
        // This happens when slightly increasing max_advance, as we can't then safely pre-identify
        // an existing cache entry because more text might fit inside this new larger constraint.
        // However, if it actually resulted in the same layout as with a slightly lower constraint,
        // then we don't want to have two cache entries with the same identical layout result.
        if let Some((equal_idx, _)) = self
            .layouts
            .iter()
            .enumerate()
            // Only those that are more constrained than the new constraint are viable.
            .filter(|(_, layout)| layout.more_constrained(max_advance))
            // We want the one that is closest to the new constraint.
            .max_by(|a, b| {
                // Because we only look at more constrained entries,
                // these are all guaranteed to be Option::Some.
                a.1.max_advance
                    .unwrap()
                    .total_cmp(&b.1.max_advance.unwrap())
            })
            // Make sure that it actually matches the new layout.
            .filter(|(_, layout)| layout.equals(&self.layouts[idx]))
        {
            // Though these two layouts are equal, we want to keep the older one.
            // Because the old one might be the currently active layout.
            let equal_layout = &mut self.layouts[equal_idx];
            // Mark the old layout as applicable up to this new constraint.
            equal_layout.max_advance = max_advance;
            equal_layout.last_used = timestamp;

            // Discard the new layout that we just created as it is a duplicate.
            let layout = &mut self.layouts[idx];
            layout.discard();

            // Return the updated old entry.
            idx = equal_idx;
        }

        idx
    }
}
