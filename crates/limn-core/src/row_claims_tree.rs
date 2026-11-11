//! The controls the rows of a virtual list take the focus with. The walk for
//! focus claims sees a list as one marker, since its rows are built only when
//! placed: each row is walked here, as it is placed, and an ID it takes twice
//! is noted in [`Frame::duplicate_ids`].

use super::*;

impl LayoutBuilder<'_> {
    /// Collects into `focusables`, in tree order, the controls a row of a
    /// virtual list takes the focus with; `seen` is the row's own. A control
    /// that takes an ID another took (in this row, in a row placed before it
    /// in this pass, or in the tree the walk for claims met) is noted in the
    /// frame's duplicates, unless it is inert. Each pass starts afresh, so a
    /// row placed again by a pass that repeats is not counted twice.
    pub(super) fn collect_row_focus(
        &mut self,
        row: &Element,
        focusables: &mut Vec<(String, HitKind)>,
        seen: &mut crate::FastSet<String>,
    ) {
        if self.inert {
            return collect_focus_order(row, focusables, seen);
        }
        // One table for the pass, as big as the rows in view usually need:
        // growing it from empty cost more than the checks.
        if self.row_claims.capacity() == 0 {
            self.row_claims.reserve(64);
        }
        let mut claims = Claims {
            twice: std::mem::take(&mut self.frame.duplicate_ids),
            rows: Some((self.claimed, &mut self.row_claims)),
        };
        collect_focus_claims(row, focusables, seen, &mut claims);
        self.frame.duplicate_ids = claims.twice;
    }
}

#[cfg(test)]
#[path = "row_claims_tests.rs"]
mod tests;
