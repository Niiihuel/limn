//! What the layout keeps for an element from one layout to the next: a
//! spring's value, a slot's height, an ink on its way to a new color. One
//! store holds every kind, by the element's ID and the kind's type, so a new
//! kind needs no field of the tree, no borrow of the layout and no sweep of
//! its own. An entry lasts while layouts touch it; the sweep after a layout
//! forgets what that layout did not touch.

use std::any::{Any, TypeId};
use crate::FastMap;
use web_time::Instant;

struct Kept<T> {
    value: T,
    /// The clock of the last layout that touched it.
    touched: Instant,
}

/// The entries of one kind, whatever the kind is.
trait Kind {
    fn sweep(&mut self, now: Instant);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: 'static> Kind for FastMap<String, Kept<T>> {
    fn sweep(&mut self, now: Instant) {
        self.retain(|_, kept| kept.touched == now);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Default)]
pub(super) struct StateStore {
    kinds: FastMap<TypeId, Box<dyn Kind>>,
}

impl StateStore {
    fn kind<T: 'static>(&self) -> Option<&FastMap<String, Kept<T>>> {
        self.kinds.get(&TypeId::of::<T>()).and_then(|kind| kind.as_any().downcast_ref())
    }

    fn kind_mut<T: 'static>(&mut self) -> &mut FastMap<String, Kept<T>> {
        self.kinds
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(FastMap::<String, Kept<T>>::default()))
            .as_any_mut()
            .downcast_mut()
            .expect("a kind is stored under its own type")
    }

    /// The state of kind `T` kept under `id`, made by `init` the first time.
    /// Touching it keeps it through the sweep after the layout at `now`.
    pub(super) fn touch<T: 'static>(&mut self, id: &str, now: Instant, init: impl FnOnce() -> T) -> &mut T {
        let kind = self.kind_mut::<T>();
        // The ID is copied only for a new entry.
        if !kind.contains_key(id) {
            kind.insert(id.to_owned(), Kept { value: init(), touched: now });
        }
        let kept = kind.get_mut(id).expect("inserted above");
        kept.touched = now;
        &mut kept.value
    }

    /// The state of kind `T` under `id`, if there is one, kept through the
    /// sweep after the layout at `now`.
    pub(super) fn keep<T: 'static>(&mut self, id: &str, now: Instant) -> Option<&mut T> {
        let kept = self.kind_mut::<T>().get_mut(id)?;
        kept.touched = now;
        Some(&mut kept.value)
    }

    /// The state of kind `T` under `id`, without keeping it.
    pub(super) fn get<T: 'static>(&self, id: &str) -> Option<&T> {
        self.kind::<T>()?.get(id).map(|kept| &kept.value)
    }

    /// As [`Self::get`], to change it, still without keeping it.
    pub(super) fn get_mut<T: 'static>(&mut self, id: &str) -> Option<&mut T> {
        self.kind_mut::<T>().get_mut(id).map(|kept| &mut kept.value)
    }

    /// The clock of the last layout that touched the state of kind `T` under `id`.
    pub(super) fn touched<T: 'static>(&self, id: &str) -> Option<Instant> {
        self.kind::<T>()?.get(id).map(|kept| kept.touched)
    }

    /// Every state of kind `T`.
    pub(super) fn values<T: 'static>(&self) -> impl Iterator<Item = &T> {
        self.kind::<T>().into_iter().flat_map(|kind| kind.values().map(|kept| &kept.value))
    }

    /// The IDs that keep a state of kind `T`.
    pub(super) fn ids<T: 'static>(&self) -> impl Iterator<Item = &str> {
        self.kind::<T>().into_iter().flat_map(|kind| kind.keys().map(String::as_str))
    }

    /// Every state of kind `T`, to change.
    pub(super) fn values_mut<T: 'static>(&mut self) -> impl Iterator<Item = &mut T> {
        self.kind_mut::<T>().values_mut().map(|kept| &mut kept.value)
    }

    /// Forget every state the layout at `now` did not touch.
    pub(super) fn sweep(&mut self, now: Instant) {
        for kind in self.kinds.values_mut() {
            kind.sweep(now);
        }
    }

    /// Forget every state.
    pub(super) fn clear(&mut self) {
        self.kinds.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    struct Height(f32);
    struct Ink(u8);

    #[test]
    fn a_state_lasts_while_layouts_touch_it_and_kinds_do_not_meet() {
        let mut store = StateStore::default();
        let first = Instant::now();
        *store.touch("row", first, || Height(0.0)) = Height(12.0);
        store.touch("row", first, || Ink(1));
        assert_eq!(store.touch("row", first, || Height(99.0)).0, 12.0, "made once");
        assert_eq!(store.get::<Ink>("row").map(|ink| ink.0), Some(1), "one ID, two kinds");
        store.sweep(first);
        assert!(store.get::<Height>("row").is_some(), "touched by the layout that swept");

        let second = first + Duration::from_millis(16);
        store.touch("row", second, || Height(0.0));
        assert_eq!(store.get::<Ink>("row").map(|ink| ink.0), Some(1), "a look does not keep it");
        assert_eq!(store.touched::<Height>("row"), Some(second));
        store.sweep(second);
        assert_eq!(store.get::<Height>("row").map(|height| height.0), Some(12.0));
        assert!(store.get::<Ink>("row").is_none(), "untouched by that layout, forgotten");
        assert_eq!(store.values::<Height>().count(), 1);
        store.clear();
        assert_eq!(store.values::<Height>().count(), 0);
    }
}
