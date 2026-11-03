// Copyright 2026 the Xilem Authors and the Parley Authors
// SPDX-License-Identifier: Apache-2.0

//! An [`accesskit`] integration for Parley text layouts and editors.
//!
//! Adapted from the `editor` example of Parley 0.12, which replaced Parley's
//! former built-in `accesskit` feature.

use std::collections::HashMap;

use accesskit::{
    Node, NodeId, Rect, Role, TextAlign, TextDirection, TextPosition, TextSelection, TreeUpdate,
};
use parley::editing::{Cursor, PlainEditor, PlainEditorDriver, Selection};
use parley::layout::{Affinity, Alignment, Cluster, ClusterPath, Layout, LineMetrics, Run, Style};
use parley::style::{Brush, FontStyle};
use skrifa::FontRef;
use skrifa::raw::TableProvider;
use skrifa::raw::types::NameId;

/// The maximum number of characters in a single `TextRun` node.
///
/// AccessKit's `word_starts` are `u8`s, so a span can't describe more characters than that.
const MAX_CHARACTERS_PER_SPAN: usize = u8::MAX as usize + 1;

/// Maps a Parley [`Layout`] onto a tree of AccessKit [`Role::TextRun`] nodes.
///
/// The same instance should be reused across accessibility passes for a given layout, so
/// that unchanged parts of the text keep their node IDs.
#[derive(Clone, Default, Debug)]
pub struct LayoutAccessibility {
    // Each span is a sequence of clusters, in logical order, that all have an identical
    // style. The nth span is given the nth ID from this list, which is kept across passes.
    span_ids: Vec<NodeId>,
    // Map from node ID to the path of the span's first cluster.
    span_paths_by_access_id: HashMap<NodeId, ClusterPath>,
    // Map from cluster path to the cluster's position within its span.
    span_positions_by_cluster_path: HashMap<ClusterPath, SpanPosition>,
}

/// The position of a cluster within its accessibility span.
#[derive(Copy, Clone, Debug)]
struct SpanPosition {
    /// The index of the span in logical order.
    span_index: usize,
    /// The index of the cluster within the span's characters.
    character_index: usize,
}

impl LayoutAccessibility {
    fn span_id(&self, span_index: usize) -> Option<NodeId> {
        self.span_ids.get(span_index).copied()
    }

    /// Push a `TextRun` node for every style span in `layout` onto `update`, as children
    /// of `parent_node`.
    ///
    /// `text` must be the source text that `layout` was built from. `x_offset` and
    /// `y_offset` are added to the layout-relative coordinates of every node.
    ///
    /// `next_node_id` is called to allocate IDs for spans that weren't present on the
    /// previous pass. `set_brush_properties` is called once per span, and should map the
    /// span's brush onto AccessKit's color and text decoration properties.
    pub fn build_nodes<B: Brush>(
        &mut self,
        text: &str,
        layout: &Layout<B>,
        update: &mut TreeUpdate,
        parent_node: &mut Node,
        mut next_node_id: impl FnMut() -> NodeId,
        x_offset: f64,
        y_offset: f64,
        set_brush_properties: impl Fn(&mut Node, &Style<B>),
    ) {
        self.span_paths_by_access_id.clear();
        self.span_positions_by_cluster_path.clear();
        // The number of spans started so far, which is also the logical index
        // of the next span to start.
        let mut spans_started = 0;
        let mut runs = Vec::new();
        let alignment = layout.alignment();

        for (line_index, line) in layout.lines().enumerate() {
            let metrics = line.metrics();
            // Defer adding each run node until we reach either the next run
            // or the end of the line, so that we can link runs within a line.
            let mut last_node: Option<(NodeId, Node)> = None;

            // Compute run offsets from left to right, then sort them into text order.
            runs.clear();
            runs.reserve(line.len());
            {
                let mut run_offset = metrics.offset;
                for run in line.runs() {
                    let advance = run.advance();
                    runs.push((run, run_offset));
                    run_offset += advance;
                }
            }
            runs.sort_by_key(|(r, _)| r.text_range().start);

            for (run, run_offset) in runs.drain(..) {
                let mut span_path = run_start_path(line_index, &run);
                let (mut id, mut node) = self.span_id_and_node(
                    &mut next_node_id,
                    spans_started,
                    &run,
                    alignment,
                    span_path,
                );
                spans_started += 1;
                if run.is_empty() {
                    // Record the start of empty runs so that a cursor in them can still be addressed.
                    self.span_positions_by_cluster_path.insert(
                        span_path,
                        SpanPosition {
                            span_index: spans_started - 1,
                            character_index: 0,
                        },
                    );
                }

                if let Some((last_id, mut last_node)) = last_node.take() {
                    link_spans(last_id, &mut last_node, id, &mut node);
                    add_span(update, parent_node, last_id, last_node);
                }

                let mut prev_style_index: Option<u16> = None;
                let mut span = SpanData::default();
                let mut span_offset = 0.0;

                for cluster in run.clusters() {
                    let style_index = cluster.style_index();
                    if let Some(prev_index) = prev_style_index
                        && (prev_index != style_index
                            || span.character_lengths.len() >= MAX_CHARACTERS_PER_SPAN)
                    {
                        prev_style_index = None;
                        let span_advance = span.advance;
                        std::mem::take(&mut span).finish(
                            &mut node,
                            x_offset,
                            y_offset,
                            metrics,
                            run_offset + span_offset,
                        );
                        span_offset += span_advance;
                        (id, node) = {
                            let (old_id, mut old_node) = (id, node);
                            span_path = cluster.path();
                            let (new_id, mut new_node) = self.span_id_and_node(
                                &mut next_node_id,
                                spans_started,
                                &run,
                                alignment,
                                span_path,
                            );
                            spans_started += 1;
                            link_spans(old_id, &mut old_node, new_id, &mut new_node);
                            add_span(update, parent_node, old_id, old_node);
                            (new_id, new_node)
                        };
                    }

                    if prev_style_index.is_none() {
                        prev_style_index = Some(style_index);
                        let style = cluster.style();
                        set_brush_properties(&mut node, style);
                        if let Some(locale) = &style.locale {
                            node.set_language(locale.as_str());
                        }
                    }

                    let cluster_text = &text[cluster.text_range()];
                    span.text.push_str(cluster_text);
                    if cluster.is_word_boundary() && !cluster.is_space_or_nbsp() {
                        span.word_starts.push(span.character_lengths.len() as _);
                    }
                    let character_index = span.character_lengths.len();
                    span.character_lengths.push(cluster_text.len() as _);
                    span.character_positions.push(span.advance);
                    span.character_widths.push(cluster.advance());
                    span.advance += cluster.advance();
                    self.span_positions_by_cluster_path.insert(
                        cluster.path(),
                        SpanPosition {
                            span_index: spans_started - 1,
                            character_index,
                        },
                    );
                }

                span.finish(
                    &mut node,
                    x_offset,
                    y_offset,
                    metrics,
                    run_offset + span_offset,
                );
                last_node = Some((id, node));
            }

            if let Some((id, node)) = last_node {
                add_span(update, parent_node, id, node);
            }
        }
    }

    /// Convert an AccessKit position within the nodes built by [`Self::build_nodes`] into
    /// a Parley [`Cursor`].
    ///
    /// Returns `None` if the position doesn't refer to one of those nodes.
    pub fn cursor_from_access_position<B: Brush>(
        &self,
        pos: &TextPosition,
        layout: &Layout<B>,
    ) -> Option<Cursor> {
        let span_path = self.span_paths_by_access_id.get(&pos.node)?;
        let run = span_path.run(layout)?;
        let index = run
            .clusters()
            .skip_while(|cluster| cluster.path() != *span_path)
            .nth(pos.character_index)
            .map(|cluster| cluster.text_range().start)
            .unwrap_or(layout.text_len());
        Some(Cursor::from_byte_index(layout, index, Affinity::Downstream))
    }

    /// Convert a Parley [`Cursor`] into a position within the nodes built by
    /// [`Self::build_nodes`].
    ///
    /// Returns `None` if [`Self::build_nodes`] hasn't been called for this layout yet.
    pub fn cursor_to_access_position<B: Brush>(
        &self,
        cursor: Cursor,
        layout: &Layout<B>,
    ) -> Option<TextPosition> {
        if layout.text_len() == 0 {
            // If the text is empty, return the first node with a character index of 0.
            let run = layout.get(0)?.runs().next()?;
            let position = self
                .span_positions_by_cluster_path
                .get(&run_start_path(0, &run))?;
            return Some(TextPosition {
                node: self.span_id(position.span_index)?,
                character_index: 0,
            });
        }
        // Prefer the downstream cluster except at the end of the text, where we
        // choose the upstream cluster and add 1 to the character index.
        let (offset, path) = cursor
            .downstream_cluster(layout)
            .map(|cluster| (0, cluster.path()))
            .or_else(|| {
                cursor
                    .upstream_cluster(layout)
                    .map(|cluster| (1, cluster.path()))
            })?;
        // At the end of a layout ending with a newline, use the "phantom" run of the
        // trailing line so that AccessKit has correct visual geometry for the cursor.
        let (span_index, character_index) =
            if cursor.index() == layout.text_len() && ends_with_hard_line_break(layout) {
                let line_index = path.line_index() + 1;
                let run = layout.get(line_index)?.runs().next()?;
                let position = self
                    .span_positions_by_cluster_path
                    .get(&run_start_path(line_index, &run))?;
                (position.span_index, 0)
            } else {
                let position = self.span_positions_by_cluster_path.get(&path)?;
                (position.span_index, position.character_index + offset)
            };
        Some(TextPosition {
            node: self.span_id(span_index)?,
            character_index,
        })
    }

    /// Convert an AccessKit selection over the nodes built by [`Self::build_nodes`] into
    /// a Parley [`Selection`].
    ///
    /// Returns `None` if either endpoint doesn't refer to one of those nodes.
    pub fn selection_from_access_selection<B: Brush>(
        &self,
        selection: &TextSelection,
        layout: &Layout<B>,
    ) -> Option<Selection> {
        let anchor = self.cursor_from_access_position(&selection.anchor, layout)?;
        let focus = self.cursor_from_access_position(&selection.focus, layout)?;
        Some(Selection::new(anchor, focus))
    }

    /// Convert a Parley [`Selection`] into a selection over the nodes built by
    /// [`Self::build_nodes`].
    ///
    /// Returns `None` if [`Self::build_nodes`] hasn't been called for this layout yet.
    pub fn selection_to_access_selection<B: Brush>(
        &self,
        selection: &Selection,
        layout: &Layout<B>,
    ) -> Option<TextSelection> {
        let anchor = self.cursor_to_access_position(selection.anchor(), layout)?;
        let focus = self.cursor_to_access_position(selection.focus(), layout)?;
        Some(TextSelection { anchor, focus })
    }

    fn span_id_and_node<B: Brush>(
        &mut self,
        next_node_id: &mut impl FnMut() -> NodeId,
        span_index: usize,
        run: &Run<'_, B>,
        alignment: Option<Alignment>,
        span_path: ClusterPath,
    ) -> (NodeId, Node) {
        // Reuse the ID from the previous pass for the same span index, so that
        // merely updating the content of existing spans keeps node IDs stable.
        let id = match self.span_ids.get(span_index) {
            Some(id) => *id,
            None => {
                let id = next_node_id();
                self.span_ids.push(id);
                id
            }
        };
        self.span_paths_by_access_id.insert(id, span_path);
        let mut node = Node::new(Role::TextRun);
        node.set_text_direction(if run.is_rtl() {
            TextDirection::RightToLeft
        } else {
            TextDirection::LeftToRight
        });

        let font = run.font();
        if let Ok(font_ref) = FontRef::from_index(font.data.as_ref(), font.index)
            && let Ok(name) = font_ref.name()
        {
            for n in name.name_record().iter() {
                if n.name_id.get() == NameId::FAMILY_NAME {
                    if let Ok(string) = n.string(name.string_data()) {
                        node.set_font_family(string.to_string());
                    }
                    break;
                }
            }
        }
        node.set_font_size(run.font_size());
        let attrs = run.font_attrs();
        node.set_font_weight(attrs.weight.value());
        if matches!(attrs.style, FontStyle::Italic) {
            node.set_italic();
        }
        if let Some(align) = alignment {
            node.set_text_align(match align {
                Alignment::Start if run.is_rtl() => TextAlign::Right,
                Alignment::Start => TextAlign::Left,
                Alignment::End if run.is_rtl() => TextAlign::Left,
                Alignment::End => TextAlign::Right,
                Alignment::Left => TextAlign::Left,
                Alignment::Center => TextAlign::Center,
                Alignment::Right => TextAlign::Right,
                Alignment::Justify => TextAlign::Justify,
            });
        }

        (id, node)
    }
}

/// Exposes a [`PlainEditor`] to assistive technologies.
///
/// This wraps [`LayoutAccessibility`] with the editor-specific text selection.
#[derive(Clone, Default, Debug)]
pub struct PlainEditorAccessibility {
    layout_access: LayoutAccessibility,
}

impl PlainEditorAccessibility {
    /// Perform an accessibility update for `editor`.
    ///
    /// `node` is the node for the text input itself; the spans are pushed onto `update`
    /// as its children. See [`LayoutAccessibility::build_nodes`] for the remaining
    /// arguments.
    ///
    /// Returns `None` if the editor's layout is not up to date.
    pub fn build_nodes<B: Brush>(
        &mut self,
        editor: &PlainEditor<B>,
        update: &mut TreeUpdate,
        node: &mut Node,
        next_node_id: impl FnMut() -> NodeId,
        x_offset: f64,
        y_offset: f64,
        set_brush_properties: impl Fn(&mut Node, &Style<B>),
    ) -> Option<()> {
        let layout = editor.try_layout()?;
        self.layout_access.build_nodes(
            editor.raw_text(),
            layout,
            update,
            node,
            next_node_id,
            x_offset,
            y_offset,
            set_brush_properties,
        );
        // The IME can ask for the caret to be hidden, in which case there is no
        // selection to report.
        if editor.is_cursor_visible() {
            if let Some(selection) = self
                .layout_access
                .selection_to_access_selection(editor.raw_selection(), layout)
            {
                node.set_text_selection(selection);
            }
        } else {
            node.clear_text_selection();
        }
        node.add_action(accesskit::Action::SetTextSelection);
        Some(())
    }

    /// Set the editor's selection from a selection over the nodes built by
    /// [`Self::build_nodes`].
    pub fn select_from_access_selection<B: Brush>(
        &self,
        driver: &mut PlainEditorDriver<'_, B>,
        selection: &TextSelection,
    ) {
        driver.refresh_layout();
        let selection = driver.editor.try_layout().and_then(|layout| {
            self.layout_access
                .selection_from_access_selection(selection, layout)
        });
        if let Some(selection) = selection {
            driver.set_selection(selection);
        }
    }
}

/// Returns `true` if the layout's text ends with a hard line break, meaning the layout has
/// a trailing "phantom" line that a cursor at the end of the text belongs to.
fn ends_with_hard_line_break<B: Brush>(layout: &Layout<B>) -> bool {
    layout
        .text_len()
        .checked_sub(1)
        .and_then(|index| Cluster::from_byte_index(layout, index))
        .is_some_and(|cluster| cluster.is_hard_line_break())
}

/// The path of `run`'s first cluster, used as the path of its first accessibility span.
///
/// Empty runs have no clusters, so they get a path that no cluster can have.
fn run_start_path<B: Brush>(line_index: usize, run: &Run<'_, B>) -> ClusterPath {
    run.clusters()
        .next()
        .map(|cluster| cluster.path())
        .unwrap_or_else(|| ClusterPath::new(line_index as u32, run.index() as u32, 0))
}

fn link_spans(prev_id: NodeId, prev: &mut Node, next_id: NodeId, next: &mut Node) {
    prev.set_next_on_line(next_id);
    next.set_previous_on_line(prev_id);
}

fn add_span(update: &mut TreeUpdate, parent_node: &mut Node, id: NodeId, node: Node) {
    update.nodes.push((id, node));
    parent_node.push_child(id);
}

/// The accumulated contents of a `TextRun` node.
#[derive(Default)]
struct SpanData {
    text: String,
    advance: f32,
    character_lengths: Vec<u8>,
    character_positions: Vec<f32>,
    character_widths: Vec<f32>,
    word_starts: Vec<u8>,
}

impl SpanData {
    fn finish(self, node: &mut Node, x_offset: f64, y_offset: f64, metrics: &LineMetrics, x: f32) {
        node.set_bounds(Rect {
            x0: x_offset + x as f64,
            y0: y_offset + metrics.content_block_min_coord as f64,
            x1: x_offset + (x + self.advance) as f64,
            y1: y_offset + metrics.content_block_max_coord as f64,
        });
        node.set_value(self.text);
        node.set_character_lengths(self.character_lengths);
        node.set_character_positions(self.character_positions);
        node.set_character_widths(self.character_widths);
        node.set_word_starts(self.word_starts);
    }
}
