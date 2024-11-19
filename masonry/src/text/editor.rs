// Copyright 2020 the Xilem Authors and the Parley Authors
// SPDX-License-Identifier: Apache-2.0

// We need to be careful with contributions to this file, as we want them to get back to Parley.

//! Import of Parley's `PlainEditor` as the version in Parley is insufficient for our needs.

use core::{cmp::PartialEq, default::Default, fmt::Debug};

use accesskit::{Node, NodeId, TreeUpdate};
use parley::layout::LayoutAccessibility;
use parley::{
    layout::{
        cursor::{Cursor, Selection, VisualMode},
        Affinity, Alignment, Layout, Line,
    },
    style::Brush,
    FontContext, LayoutContext, Rect,
};
use std::{borrow::ToOwned, string::String, vec::Vec};

use super::styleset::StyleSet;

#[derive(Copy, Clone, Debug)]
pub enum ActiveText<'a> {
    /// The selection is empty and the cursor is a caret; this is the text of the cluster it is on.
    FocusedCluster(Affinity, &'a str),
    /// The selection contains this text.
    Selection(&'a str),
}

/// Opaque representation of a generation.
///
/// Obtained from [`PlainEditor::generation`].
// Overflow handling: the generations are only compared,
// so wrapping is fine. This could only fail if exactly
// `u32::MAX` generations happen between drawing
// operations. This is implausible and so can be ignored.
#[derive(PartialEq, Eq, Default, Clone, Copy)]
pub struct Generation(u32);

impl Generation {
    /// Make it not what it currently is.
    pub(crate) fn nudge(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}

/// Basic plain text editor with a single default style.
#[derive(Clone)]
pub struct PlainEditor<T>
where
    T: Brush + Clone + Debug + PartialEq + Default,
{
    default_style: StyleSet<T>,
    buffer: String,
    layout: Layout<T>,
    layout_access: LayoutAccessibility,
    selection: Selection,
    cursor_mode: VisualMode,
    width: Option<f32>,
    scale: f32,
    // Simple tracking of when the layout needs to be updated
    // before it can be used for `Selection` calculations or
    // for drawing.
    // Not all operations on `PlainEditor` need to operate on a
    // clean layout, and not all operations trigger a layout.
    layout_dirty: bool,
    // TODO: We could avoid redoing the full text layout if linebreaking or
    // alignment were unchanged
    // linebreak_dirty: bool,
    // alignment_dirty: bool,
    alignment: Alignment,
    generation: Generation,
}

impl<T> PlainEditor<T>
where
    T: Brush,
{
    pub fn new(font_size: f32) -> Self {
        Self {
            default_style: StyleSet::new(font_size),
            buffer: Default::default(),
            layout: Default::default(),
            layout_access: Default::default(),
            selection: Default::default(),
            cursor_mode: Default::default(),
            width: None,
            scale: 1.0,
            layout_dirty: false,
            alignment: Alignment::Start,
            // We don't use the `default` value to start with, as our consumers
            // will choose to use that as their initial value, but will probably need
            // to redraw if they haven't already.
            generation: Generation(1),
        }
    }
}

/// The argument passed to the callback of [`PlainEditor::transact`],
/// on which the caller performs operations.
pub struct PlainEditorTxn<'a, T>
where
    T: Brush + Clone + Debug + PartialEq + Default,
{
    editor: &'a mut PlainEditor<T>,
    font_cx: &'a mut FontContext,
    layout_cx: &'a mut LayoutContext<T>,
}

impl<T> PlainEditorTxn<'_, T>
where
    T: Brush + Clone + Debug + PartialEq + Default,
{
    // --- MARK: Forced relayout ---
    /// Insert at cursor, or replace selection.
    pub fn insert_or_replace_selection(&mut self, s: &str) {
        self.editor
            .replace_selection(self.font_cx, self.layout_cx, s);
    }

    /// Delete the selection.
    pub fn delete_selection(&mut self) {
        self.insert_or_replace_selection("");
    }

    /// Delete the selection or the next cluster (typical ‘delete’ behavior).
    pub fn delete(&mut self) {
        if self.editor.selection.is_collapsed() {
            let range = self.editor.selection.focus().text_range();
            if !range.is_empty() {
                let start = range.start;
                self.editor.buffer.replace_range(range, "");
                self.update_layout();
                self.editor
                    .set_selection(self.editor.cursor_at(start).into());
            }
        } else {
            self.delete_selection();
        }
    }

    /// Delete the selection or up to the next word boundary (typical ‘ctrl + delete’ behavior).
    pub fn delete_word(&mut self) {
        let start = self.editor.selection.focus().text_range().start;
        if self.editor.selection.is_collapsed() {
            let end = self
                .editor
                .cursor_at(start)
                .next_word(&self.editor.layout)
                .index();

            self.editor.buffer.replace_range(start..end, "");
            self.update_layout();
            self.editor
                .set_selection(self.editor.cursor_at(start).into());
        } else {
            self.delete_selection();
        }
    }

    /// Delete the selection or the previous cluster (typical ‘backspace’ behavior).
    pub fn backdelete(&mut self) {
        let end = self.editor.selection.focus().text_range().start;
        if self.editor.selection.is_collapsed() {
            if let Some(start) = self
                .editor
                .selection
                .focus()
                .cluster_path()
                .cluster(&self.editor.layout)
                .map(|x| {
                    if self.editor.selection.focus().affinity() == Affinity::Upstream {
                        Some(x)
                    } else {
                        x.previous_logical()
                    }
                })
                .and_then(|c| c.map(|x| x.text_range().start))
            {
                self.editor.buffer.replace_range(start..end, "");
                self.update_layout();
                self.editor
                    .set_selection(self.editor.cursor_at(start).into());
            }
        } else {
            self.delete_selection();
        }
    }

    /// Delete the selection or back to the previous word boundary (typical ‘ctrl + backspace’ behavior).
    pub fn backdelete_word(&mut self) {
        let end = self.editor.selection.focus().text_range().start;
        if self.editor.selection.is_collapsed() {
            let start = self
                .editor
                .selection
                .focus()
                .previous_word(&self.editor.layout)
                .text_range()
                .start;

            self.editor.buffer.replace_range(start..end, "");
            self.update_layout();
            self.editor
                .set_selection(self.editor.cursor_at(start).into());
        } else {
            self.delete_selection();
        }
    }

    // --- MARK: Cursor Movement ---
    /// Move the cursor to the cluster boundary nearest this point in the layout.
    pub fn move_to_point(&mut self, x: f32, y: f32) {
        self.refresh_layout();
        self.editor
            .set_selection(Selection::from_point(&self.editor.layout, x, y));
    }

    /// Move the cursor to a byte index.
    ///
    /// No-op if index is not a char boundary.
    pub fn move_to_byte(&mut self, index: usize) {
        if self.editor.buffer.is_char_boundary(index) {
            self.refresh_layout();
            self.editor
                .set_selection(self.editor.cursor_at(index).into());
        }
    }

    /// Move the cursor to the start of the buffer.
    pub fn move_to_text_start(&mut self) {
        self.editor.set_selection(self.editor.selection.move_lines(
            &self.editor.layout,
            isize::MIN,
            false,
        ));
    }

    /// Move the cursor to the start of the physical line.
    pub fn move_to_line_start(&mut self) {
        self.editor
            .set_selection(self.editor.selection.line_start(&self.editor.layout, false));
    }

    /// Move the cursor to the end of the buffer.
    pub fn move_to_text_end(&mut self) {
        self.editor.set_selection(self.editor.selection.move_lines(
            &self.editor.layout,
            isize::MAX,
            false,
        ));
    }

    /// Move the cursor to the end of the physical line.
    pub fn move_to_line_end(&mut self) {
        self.editor
            .set_selection(self.editor.selection.line_end(&self.editor.layout, false));
    }

    /// Move up to the closest physical cluster boundary on the previous line, preserving the horizontal position for repeated movements.
    pub fn move_up(&mut self) {
        self.editor.set_selection(
            self.editor
                .selection
                .previous_line(&self.editor.layout, false),
        );
    }

    /// Move down to the closest physical cluster boundary on the next line, preserving the horizontal position for repeated movements.
    pub fn move_down(&mut self) {
        self.editor
            .set_selection(self.editor.selection.next_line(&self.editor.layout, false));
    }

    /// Move to the next cluster left in visual order.
    pub fn move_left(&mut self) {
        self.editor
            .set_selection(self.editor.selection.previous_visual(
                &self.editor.layout,
                self.editor.cursor_mode,
                false,
            ));
    }

    /// Move to the next cluster right in visual order.
    pub fn move_right(&mut self) {
        self.editor.set_selection(self.editor.selection.next_visual(
            &self.editor.layout,
            self.editor.cursor_mode,
            false,
        ));
    }

    /// Move to the next word boundary left.
    pub fn move_word_left(&mut self) {
        self.editor.set_selection(
            self.editor
                .selection
                .previous_word(&self.editor.layout, false),
        );
    }

    /// Move to the next word boundary right.
    pub fn move_word_right(&mut self) {
        self.editor
            .set_selection(self.editor.selection.next_word(&self.editor.layout, false));
    }

    /// Select the whole buffer.
    pub fn select_all(&mut self) {
        self.editor.set_selection(
            Selection::from_index(&self.editor.layout, 0, Affinity::default()).move_lines(
                &self.editor.layout,
                isize::MAX,
                true,
            ),
        );
    }

    /// Collapse selection into caret.
    pub fn collapse_selection(&mut self) {
        self.editor.set_selection(self.editor.selection.collapse());
    }

    /// Move the selection focus point to the start of the buffer.
    pub fn select_to_text_start(&mut self) {
        self.editor.set_selection(self.editor.selection.move_lines(
            &self.editor.layout,
            isize::MIN,
            true,
        ));
    }

    /// Move the selection focus point to the start of the physical line.
    pub fn select_to_line_start(&mut self) {
        self.editor
            .set_selection(self.editor.selection.line_start(&self.editor.layout, true));
    }

    /// Move the selection focus point to the end of the buffer.
    pub fn select_to_text_end(&mut self) {
        self.editor.set_selection(self.editor.selection.move_lines(
            &self.editor.layout,
            isize::MAX,
            true,
        ));
    }

    /// Move the selection focus point to the end of the physical line.
    pub fn select_to_line_end(&mut self) {
        self.editor
            .set_selection(self.editor.selection.line_end(&self.editor.layout, true));
    }

    /// Move the selection focus point up to the nearest cluster boundary on the previous line, preserving the horizontal position for repeated movements.
    pub fn select_up(&mut self) {
        self.editor.set_selection(
            self.editor
                .selection
                .previous_line(&self.editor.layout, true),
        );
    }

    /// Move the selection focus point down to the nearest cluster boundary on the next line, preserving the horizontal position for repeated movements.
    pub fn select_down(&mut self) {
        self.editor
            .set_selection(self.editor.selection.next_line(&self.editor.layout, true));
    }

    /// Move the selection focus point to the next cluster left in visual order.
    pub fn select_left(&mut self) {
        self.editor
            .set_selection(self.editor.selection.previous_visual(
                &self.editor.layout,
                self.editor.cursor_mode,
                true,
            ));
    }

    /// Move the selection focus point to the next cluster right in visual order.
    pub fn select_right(&mut self) {
        self.editor.set_selection(self.editor.selection.next_visual(
            &self.editor.layout,
            self.editor.cursor_mode,
            true,
        ));
    }

    /// Move the selection focus point to the next word boundary left.
    pub fn select_word_left(&mut self) {
        self.editor.set_selection(
            self.editor
                .selection
                .previous_word(&self.editor.layout, true),
        );
    }

    /// Move the selection focus point to the next word boundary right.
    pub fn select_word_right(&mut self) {
        self.editor
            .set_selection(self.editor.selection.next_word(&self.editor.layout, true));
    }

    /// Select the word at the point.
    pub fn select_word_at_point(&mut self, x: f32, y: f32) {
        self.refresh_layout();
        self.editor
            .set_selection(Selection::word_from_point(&self.editor.layout, x, y));
    }

    /// Select the physical line at the point.
    pub fn select_line_at_point(&mut self, x: f32, y: f32) {
        self.refresh_layout();
        let focus = *Selection::from_point(&self.editor.layout, x, y)
            .line_start(&self.editor.layout, true)
            .focus();
        self.editor
            .set_selection(Selection::from(focus).line_end(&self.editor.layout, true));
    }

    /// Move the selection focus point to the cluster boundary closest to point.
    pub fn extend_selection_to_point(&mut self, x: f32, y: f32) {
        self.refresh_layout();
        // FIXME: This is usually the wrong way to handle selection extension for mouse moves, but not a regression.
        self.editor.set_selection(
            self.editor
                .selection
                .extend_to_point(&self.editor.layout, x, y),
        );
    }

    /// Move the selection focus point to a byte index.
    ///
    /// No-op if index is not a char boundary.
    pub fn extend_selection_to_byte(&mut self, index: usize) {
        if self.editor.buffer.is_char_boundary(index) {
            self.refresh_layout();
            self.editor.set_selection(
                self.editor
                    .selection
                    .maybe_extend(self.editor.cursor_at(index), true),
            );
        }
    }

    /// Select a range of byte indices
    ///
    /// No-op if either index is not a char boundary.
    pub fn select_byte_range(&mut self, start: usize, end: usize) {
        if self.editor.buffer.is_char_boundary(start) && self.editor.buffer.is_char_boundary(end) {
            self.refresh_layout();
            self.editor.set_selection(
                Selection::from(self.editor.cursor_at(start))
                    .maybe_extend(self.editor.cursor_at(end), true),
            );
        }
    }

    pub fn select_from_accesskit(&mut self, selection: &accesskit::TextSelection) {
        self.refresh_layout();
        if let Some(selection) = Selection::from_access_selection(
            selection,
            &self.editor.layout,
            &self.editor.layout_access,
        ) {
            self.editor.set_selection(selection);
        }
    }

    // --- MARK: Internal helpers ---
    fn update_layout(&mut self) {
        self.editor.update_layout(self.font_cx, self.layout_cx);
    }

    fn refresh_layout(&mut self) {
        self.editor.refresh_layout(self.font_cx, self.layout_cx);
    }
}

impl<T> PlainEditor<T>
where
    T: Brush + Clone + Debug + PartialEq + Default,
{
    /// Run a series of [`PlainEditorTxn`] methods.
    ///
    /// This is a utility shorthand around [`transaction`](Self::transaction);
    pub fn transact<R>(
        &mut self,
        font_cx: &mut FontContext,
        layout_cx: &mut LayoutContext<T>,
        callback: impl FnOnce(&mut PlainEditorTxn<'_, T>) -> R,
    ) -> R {
        let mut txn = self.transaction(font_cx, layout_cx);
        callback(&mut txn)
    }

    /// Run a series of [`PlainEditorTxn`] methods, updating the layout
    /// if necessary.
    ///
    /// This is a utility shorthand to simplify methods which require the editor
    /// and the provided contexts.
    pub fn transaction<'txn>(
        &'txn mut self,
        font_cx: &'txn mut FontContext,
        layout_cx: &'txn mut LayoutContext<T>,
    ) -> PlainEditorTxn<'txn, T> {
        PlainEditorTxn {
            editor: self,
            font_cx,
            layout_cx,
        }
    }

    /// Make a cursor at a given byte index
    fn cursor_at(&self, index: usize) -> Cursor {
        // TODO: Do we need to be non-dirty?
        // FIXME: `Selection` should make this easier
        if index >= self.buffer.len() {
            Cursor::from_index(
                &self.layout,
                self.buffer.len().saturating_sub(1),
                Affinity::Upstream,
            )
        } else {
            Cursor::from_index(&self.layout, index, Affinity::Downstream)
        }
    }

    fn replace_selection(
        &mut self,
        font_cx: &mut FontContext,
        layout_cx: &mut LayoutContext<T>,
        s: &str,
    ) {
        // TODO: Do we need to be non-dirty?
        let range = self.selection.text_range();
        let start = range.start;
        if self.selection.is_collapsed() {
            self.buffer.insert_str(start, s);
        } else {
            self.buffer.replace_range(range, s);
        }

        self.update_layout(font_cx, layout_cx);
        self.set_selection(self.cursor_at(start.saturating_add(s.len())).into());
    }

    /// Update the selection, and nudge the `Generation` if something other than `h_pos` changed.
    fn set_selection(&mut self, new_sel: Selection) {
        if new_sel.focus() != self.selection.focus() || new_sel.anchor() != self.selection.anchor()
        {
            self.generation.nudge();
        }

        self.selection = new_sel;
    }

    /// Get either the contents of the current selection, or the text of the cluster at the caret.
    pub fn active_text(&self) -> ActiveText {
        if self.selection.is_collapsed() {
            let range = self
                .selection
                .focus()
                .cluster_path()
                .cluster(&self.layout)
                .map(|c| c.text_range())
                .unwrap_or_default();
            ActiveText::FocusedCluster(self.selection.focus().affinity(), &self.buffer[range])
        } else {
            ActiveText::Selection(&self.buffer[self.selection.text_range()])
        }
    }

    /// Get rectangles representing the selected portions of text.
    pub fn selection_geometry(&self) -> Vec<Rect> {
        self.selection.geometry(&self.layout)
    }

    /// Get a rectangle representing the current caret cursor position.
    pub fn selection_strong_geometry(&self, size: f32) -> Option<Rect> {
        self.selection.focus().strong_geometry(&self.layout, size)
    }

    pub fn selection_weak_geometry(&self, size: f32) -> Option<Rect> {
        self.selection.focus().weak_geometry(&self.layout, size)
    }

    /// Get the lines from the `Layout`.
    pub fn lines(&self) -> impl Iterator<Item = Line<T>> + '_ + Clone {
        self.layout.lines()
    }

    /// Borrow the text content of the buffer.
    pub fn text(&self) -> &str {
        &self.buffer
    }

    /// Get the current `Generation` of the layout, to decide whether to draw.
    ///
    /// You should store the generation the editor was at when you last drew it, and then redraw
    /// when the generation is different (`Generation` is [`PartialEq`], so supports the equality `==` operation).
    pub fn generation(&self) -> Generation {
        self.generation
    }

    /// Get the full read-only details from the layout
    pub fn layout(
        &mut self,
        font_cx: &mut FontContext,
        layout_cx: &mut LayoutContext<T>,
    ) -> &Layout<T> {
        self.refresh_layout(font_cx, layout_cx);
        &self.layout
    }

    /// Get the full read-only details from the layout, if valid.
    pub fn get_layout(&self) -> Option<&Layout<T>> {
        if self.layout_dirty {
            None
        } else {
            Some(&self.layout)
        }
    }

    /// Get the (potentially invalid) details from the layout.
    pub fn layout_raw(&self) -> &Layout<T> {
        &self.layout
    }

    /// Replace the whole text buffer.
    pub fn set_text(&mut self, is: &str) {
        self.buffer.clear();
        self.buffer.push_str(is);
        self.layout_dirty = true;
    }

    /// Set the width of the layout.
    // TODO: If this is infinite, is the width used for alignnment the min width?
    pub fn set_width(&mut self, width: Option<f32>) {
        self.width = width;
        self.layout_dirty = true;
    }

    /// Set the alignment of the layout.
    pub fn set_alignment(&mut self, alignment: Alignment) {
        self.alignment = alignment;
        self.layout_dirty = true;
    }

    /// Set the scale for the layout.
    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale;
        self.layout_dirty = true;
    }

    /// Set the default style for the layout.
    pub fn edit_styles(&mut self) -> &mut StyleSet<T> {
        self.layout_dirty = true;
        &mut self.default_style
    }

    /// Update the layout if it is dirty.
    fn refresh_layout(&mut self, font_cx: &mut FontContext, layout_cx: &mut LayoutContext<T>) {
        if self.layout_dirty {
            self.update_layout(font_cx, layout_cx);
        }
    }

    /// Update the layout.
    fn update_layout(&mut self, font_cx: &mut FontContext, layout_cx: &mut LayoutContext<T>) {
        let mut builder = layout_cx.ranged_builder(font_cx, &self.buffer, self.scale);
        for prop in self.default_style.inner().values() {
            builder.push_default(prop.to_owned());
        }
        builder.build_into(&mut self.layout, &self.buffer);
        self.layout.break_all_lines(self.width);
        self.layout.align(self.width, self.alignment);
        self.selection = self.selection.refresh(&self.layout);
        self.layout_dirty = false;
        self.generation.nudge();
    }

    pub fn accessibility(
        &mut self,
        update: &mut TreeUpdate,
        node: &mut Node,
        next_node_id: impl FnMut() -> NodeId,
        x_offset: f64,
        y_offset: f64,
    ) {
        self.layout_access.build_nodes(
            &self.buffer,
            &self.layout,
            update,
            node,
            next_node_id,
            x_offset,
            y_offset,
        );
        if let Some(selection) = self
            .selection
            .to_access_selection(&self.layout, &self.layout_access)
        {
            node.set_text_selection(selection);
        }
        node.add_action(accesskit::Action::SetTextSelection);
    }
}
