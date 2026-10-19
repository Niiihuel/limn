//! Scratch measurements for one immutable tree during one layout pass.
//! Temporary row/overlay trees use distinct scopes even if allocations reuse
//! the same address. Entries never survive into the next layout pass.

use super::{Element, ElementKind, Size};
use std::collections::HashMap;

const MAX_ENTRIES: usize = 16_384;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Key {
    scope: u64,
    node: usize,
    width: u32,
    height: u32,
}

#[derive(Default)]
pub(super) struct MeasurementCache {
    entries: HashMap<Key, Size>,
    #[cfg(test)]
    pub(super) disabled: bool,
    #[cfg(test)]
    pub(super) hits: std::cell::Cell<usize>,
}

impl MeasurementCache {
    pub(super) fn disabled(&self) -> bool {
        #[cfg(test)]
        {
            self.disabled
        }
        #[cfg(not(test))]
        {
            false
        }
    }

    pub(super) fn clear(&mut self) {
        self.entries.clear();
        #[cfg(test)]
        self.hits.set(0);
    }

    pub(super) fn get(&self, scope: u64, element: &Element, available: Size) -> Option<Size> {
        if self.disabled() {
            return None;
        }
        let value = self
            .entries
            .get(&Self::key(scope, element, available))
            .copied();
        #[cfg(test)]
        if value.is_some() {
            self.hits.set(self.hits.get() + 1);
        }
        value
    }

    pub(super) fn insert(
        &mut self,
        scope: u64,
        element: &Element,
        available: Size,
        measured: Size,
    ) {
        if !self.disabled() && self.entries.len() < MAX_ENTRIES {
            self.entries
                .insert(Self::key(scope, element, available), measured);
        }
    }

    fn key(scope: u64, element: &Element, available: Size) -> Key {
        Key {
            scope,
            node: element as *const Element as usize,
            width: available.width.to_bits(),
            height: available.height.to_bits(),
        }
    }
}

pub(super) fn fills_available_width(kind: &ElementKind) -> bool {
    matches!(
        kind,
        ElementKind::Column(_)
            | ElementKind::Stack(_)
            | ElementKind::Row(_)
            | ElementKind::TabStrip { .. }
            | ElementKind::Panel(_)
            | ElementKind::Clickable { .. }
            | ElementKind::Overlay { .. }
            | ElementKind::Popover { .. }
            | ElementKind::Tooltip { .. }
            | ElementKind::HorizontalScroll { .. }
            | ElementKind::Scroll { .. }
            | ElementKind::UniformList(_)
            | ElementKind::CodeEditor(_)
            | ElementKind::CodeDiff(_)
            | ElementKind::EffortSlider { .. }
            | ElementKind::Skeleton { .. }
            | ElementKind::Spinner { .. }
            | ElementKind::ProgressRing { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_measurements_separate_constraints_and_temporary_tree_lifetimes() {
        let element = Element::text("First temporary row");
        let mut cache = MeasurementCache::default();
        let available = Size::new(200.0, 100.0);
        cache.insert(1, &element, available, Size::new(80.0, 20.0));
        assert_eq!(
            cache.get(1, &element, available),
            Some(Size::new(80.0, 20.0))
        );
        assert_eq!(
            cache.get(2, &element, available),
            None,
            "same address in a new scope must miss"
        );
        assert_eq!(cache.get(1, &element, Size::new(100.0, 100.0)), None);
        assert_eq!(cache.get(1, &element, Size::new(200.0, 50.0)), None);
        for scope in 2..MAX_ENTRIES as u64 * 2 {
            cache.insert(scope, &element, available, available);
        }
        assert_eq!(cache.entries.len(), MAX_ENTRIES);
        cache.clear();
        assert!(cache.get(1, &element, available).is_none());
    }

    #[test]
    fn cached_measurements_match_uncached_frames_with_overlays_virtual_rows_and_resize() {
        use crate::{Color, FontFamily, Point, TextSpan, UiEvent, UiTree};
        let row = |index| {
            Element::row(vec![
                Element::text("variable ".repeat(index + 1)),
                Element::button(format!("row:{index}"), "Abrir"),
            ])
            .gap(7.0)
        };
        let root = Element::column(vec![
            Element::panel(Element::column(vec![
                Element::text("Esta cabecera tiene tamaño fijo y contenido natural extenso."),
                Element::text("No cambia sus límites al medirla de nuevo."),
            ]))
            .size(Some(300.0), Some(48.0))
            .min_size(310.0, 40.0),
            Element::uniform_list("rows", 30, 26.0, row).size(None, Some(110.0)),
            Element::panel(Element::column(vec![
                Element::text("Natural paragraph measured by its parent and then placed."),
                Element::row(vec![Element::text("Nested"), Element::text("content")]).gap(4.0),
            ]))
            .padding(5.0),
            Element::overlay(
                Element::row(vec![
                    Element::text("Base corta"),
                    Element::button("base", "Base"),
                ])
                .gap(5.0),
                Element::column(vec![
                    Element::text("Flotante con texto diferente y más ancho"),
                    Element::button("floating", "Flotante"),
                ]),
            )
            .size(None, Some(65.0)),
            Element::formatted_text(vec![TextSpan::new(
                "Texto con ñ y 👩🏽‍💻 que cambia de líneas al reducir el ancho.",
                Color::rgb(240, 240, 240),
                14.0,
                FontFamily::Sans,
            )]),
            Element::text_area("draft", "uno\r\ndos", "Escribe").text_area_auto_grow(32.0, 120.0),
        ])
        .gap(6.0);
        let mut cached = UiTree::new(root.clone());
        let mut uncached = UiTree::new(root);
        uncached.measurement_cache.get_mut().disabled = true;
        for tree in [&mut cached, &mut uncached] {
            tree.set_reduced_motion(true);
        }
        let now = web_time::Instant::now();
        for width in [360.0, 180.0, 420.0] {
            let size = Size::new(width, 600.0);
            assert_eq!(cached.layout_at(size, now), uncached.layout_at(size, now));
            for tree in [&mut cached, &mut uncached] {
                tree.handle_event_at(
                    UiEvent::ScrollDirect {
                        position: Point::new(40.0, 80.0),
                        delta_y: -65.0,
                    },
                    now,
                );
                tree.set_input_value(
                    "draft",
                    "Una entrada nueva con muchas más palabras y líneas.\nSegunda línea.\nTercera.",
                );
            }
            assert_eq!(cached.layout_at(size, now), uncached.layout_at(size, now));
        }
        assert!(cached.measurement_cache.borrow().hits.get() > 0);
    }

    #[test]
    fn tooltip_wait_requests_its_deadline_and_actual_animation_keeps_fast_repaint() {
        use super::super::{UiTree, TOOLTIP_HOVER_DELAY};
        use web_time::{Duration, Instant};
        let now = Instant::now();
        let mut tree = UiTree::new(Element::spacer());
        tree.tooltip_triggers.insert("hint".into(), TOOLTIP_HOVER_DELAY);
        tree.tooltip_hover_since = Some(("hint".into(), now));
        assert_eq!(
            tree.animation_repaint_after_at(now),
            Some(TOOLTIP_HOVER_DELAY)
        );
        let elapsed = Duration::from_millis(120);
        assert_eq!(
            tree.animation_repaint_after_at(now + elapsed),
            Some(TOOLTIP_HOVER_DELAY - elapsed)
        );
        assert_eq!(
            tree.animation_repaint_after_at(now + TOOLTIP_HOVER_DELAY),
            None
        );
        tree.spinner_started.insert("busy".into(), now);
        assert_eq!(
            tree.animation_repaint_after_at(now),
            Some(Duration::from_millis(16))
        );
        tree.spinner_started.clear();
        tree.pressed = Some("hint".into());
        assert_eq!(tree.animation_repaint_after_at(now), None);
    }
}
