use parley::layout::cursor::Selection;
use parley::layout::Affinity;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Movement {
    Backspace, // Backspace may behave differently than LeftArrow for emoji.
    Grapheme(Affinity),
    Word(Affinity),
    Line(Affinity),
    ParagraphStart,
    ParagraphEnd,
    LineUp(usize),
    LineDown(usize),
    DocumentStart,
    DocumentEnd,
}

impl Movement {
    pub fn apply_to(&self, selection: &Selection, extend: bool) -> Selection {
        match self {
            Movement::Backspace => {

            },
            Movement::Grapheme(direction) => {
                if direction == Affinity::Downstream {
                    selection.previous_visual(layout, mode, extend)
                }
            },
            Movement::Word(direction) => {

            },
            Movement::Line(direction) => {

            },
            Movement::ParagraphStart => {

            },
            Movement::ParagraphEnd => {

            },
            Movement::LineUp(_) => {

            },
            Movement::LineDown(_) => {

            },
            Movement::DocumentStart => {

            },
            Movement::DocumentEnd => {

            },
        }
    }
}

self.selection.is_collapsed
self.selection.focus
self.selection.text_range
self.selection.refresh
self.selection.is_collapsed
self.selection.text_range
self.selection.is_collapsed
self.selection.text_range
self.selection.focus
self.selection.previous_word
self.selection.previous_visual
self.selection.next_word
self.selection.previous_line
self.selection.next_line
self.selection.line_start
self.selection.line_end
self.selection.is_collapsed
self.selection.focus
self.selection.refresh
self.selection.is_collapsed
self.selection.focus
self.selection.focus
self.selection.extend_to_point
self.selection.is_collapsed
self.selection.text_range
self.selection.geometry_with
self.selection.focus
self.selection.focus