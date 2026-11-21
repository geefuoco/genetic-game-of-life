//! Undo and redo of edit strokes within one Generation.

use crate::allele_tree::AlleleTree;
use crate::genetics::Genotype;
use std::collections::HashMap;

/// What a Cell holds: its Genotype while live, and the Birth Generation of
/// its Organism.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CellState {
    pub(crate) genotype: Option<Genotype>,
    pub(crate) born: u64,
}

/// The changes one stroke made. Undoing swaps them back into the World, which
/// leaves the stroke holding what redoing needs, and the reverse.
#[derive(Debug, Clone, Default)]
pub(crate) struct Stroke {
    /// Each changed Cell's index and its other state: before the stroke while
    /// it can be undone, after it while it can be redone.
    pub(crate) cells: HashMap<usize, CellState>,
    /// The other Allele Tree, if the stroke added brush Alleles.
    pub(crate) tree: Option<AlleleTree>,
}

impl Stroke {
    fn is_empty(&self) -> bool {
        self.cells.is_empty() && self.tree.is_none()
    }
}

/// The strokes made since the Generation last advanced: the one still open,
/// those that can be undone, and those that can be redone.
#[derive(Debug, Clone, Default)]
pub(crate) struct EditHistory {
    open: Stroke,
    undo: Vec<Stroke>,
    redo: Vec<Stroke>,
}

impl EditHistory {
    /// Records that the open stroke changed the Cell at `index` from
    /// `before`, discarding the strokes that could be redone.
    pub(crate) fn record_cell(&mut self, index: usize, before: CellState) {
        self.redo.clear();
        self.open.cells.entry(index).or_insert(before);
    }

    /// Records the Allele Tree as it was before the open stroke first added
    /// a brush Allele, discarding the strokes that could be redone.
    pub(crate) fn record_tree(&mut self, before: &AlleleTree) {
        self.redo.clear();
        self.open.tree.get_or_insert_with(|| before.clone());
    }

    /// Closes the open stroke, so the next edit starts a new one.
    pub(crate) fn end_stroke(&mut self) {
        if !self.open.is_empty() {
            self.undo.push(std::mem::take(&mut self.open));
        }
    }

    /// The latest stroke that can be undone, closing the open stroke first.
    pub(crate) fn take_undo(&mut self) -> Option<Stroke> {
        self.end_stroke();
        self.undo.pop()
    }

    /// The latest undone stroke, closing the open stroke first.
    pub(crate) fn take_redo(&mut self) -> Option<Stroke> {
        self.end_stroke();
        self.redo.pop()
    }

    /// Makes `stroke`, just undone, the next to redo.
    pub(crate) fn push_redo(&mut self, stroke: Stroke) {
        self.redo.push(stroke);
    }

    /// Makes `stroke`, just redone, the next to undo.
    pub(crate) fn push_undo(&mut self, stroke: Stroke) {
        self.undo.push(stroke);
    }

    pub(crate) fn can_undo(&self) -> bool {
        !self.open.is_empty() || !self.undo.is_empty()
    }

    pub(crate) fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }
}
