//! The scrolls a host asks of a scroll area or a list before a layout places
//! it: [`UiTree::scroll_to_end`] and [`UiTree::scroll_to_index`].

use super::*;

/// Requests by the ID of the scroll area they are for, each taken by the
/// first layout that places that area. A host may ask before the view that
/// holds the area is shown, as a list behind a loading state, so a request
/// waits for its area across layouts; one that waits is kept among the
/// `DORMANT_KEPT` asked for most recently, so IDs that never come do not pile
/// up. Spelled as a `HashSet` of IDs for `scroll_to_end`, and as a `HashMap`
/// of rows for `scroll_to_index`.
#[derive(Default)]
pub(super) struct ScrollRequests<V> {
    /// Each request, and when it was asked: a count of the requests made.
    pending: HashMap<String, (V, u64)>,
    asked: u64,
}

impl<V> ScrollRequests<V> {
    /// Asks `value` of `id`, in place of what was asked of it before, and
    /// forgets the request asked longest ago beyond the most recent.
    fn ask(&mut self, id: String, value: V) {
        self.asked += 1;
        self.pending.insert(id, (value, self.asked));
        if self.pending.len() > DORMANT_KEPT {
            let oldest = self.pending.iter().min_by_key(|(_, (_, asked))| *asked).map(|(id, _)| id.clone());
            if let Some(oldest) = oldest {
                self.pending.remove(&oldest);
            }
        }
    }
}

impl ScrollRequests<()> {
    /// Asks the area `id` to go to its end.
    pub(super) fn insert(&mut self, id: String) {
        self.ask(id, ());
    }

    /// Takes the request of the area `id`: whether there was one.
    pub(super) fn remove(&mut self, id: &str) -> bool {
        self.pending.remove(id).is_some()
    }
}

impl ScrollRequests<usize> {
    /// Asks the list `id` to bring its row `index` into view.
    pub(super) fn insert(&mut self, id: String, index: usize) {
        self.ask(id, index);
    }

    /// Takes the request of the list `id`: the row asked for.
    pub(super) fn remove(&mut self, id: &str) -> Option<usize> {
        self.pending.remove(id).map(|(index, _)| index)
    }
}

#[cfg(test)]
#[path = "scroll_request_tests.rs"]
mod tests;
