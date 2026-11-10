use crate::{Element, FontFamily, FontMetrics, Size, UiTree};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[test]
fn unchanged_text_reuses_measurements_across_frames_and_rebuilt_roots() {
    let text = "La respuesta crece, pero estas palabras del historial no cambian.";
    let root = || {
        Element::text(text)
            .font_family(FontFamily::Sans)
            .size(Some(180.0), None)
    };
    let mut tree = UiTree::new(root());
    let calls = Arc::new(AtomicUsize::new(0));
    let measured = calls.clone();
    tree.set_text_measurer(move |text, size, _| {
        measured.fetch_add(1, Ordering::Relaxed);
        text.chars().count() as f32 * size * 0.5
    });
    let viewport = Size::new(180.0, 300.0);
    tree.layout(viewport);
    let initial = calls.load(Ordering::Relaxed);
    assert!(initial > 0);
    for _ in 0..12 {
        tree.set_root(root());
        tree.layout(viewport);
    }
    assert_eq!(calls.load(Ordering::Relaxed), initial);
    // A new font source must invalidate old measurements even at the same size.
    let replacement = calls.clone();
    tree.set_text_measurer(move |text, size, _| {
        replacement.fetch_add(1, Ordering::Relaxed);
        text.chars().count() as f32 * size
    });
    tree.layout(viewport);
    assert!(calls.load(Ordering::Relaxed) > initial);
}

#[test]
fn metric_changes_and_removing_a_measurer_invalidate_retained_widths() {
    let mut tree = UiTree::new(Element::text("abcdefghij").font_family(FontFamily::Sans));
    let viewport = Size::new(65.0, 300.0);
    tree.set_sans_metrics(FontMetrics {
        char_width: 4.0,
        line_height: 18.0,
    });
    let narrow_font = tree.layout(viewport).text_layouts()[0].runs.len();
    tree.set_sans_metrics(FontMetrics {
        char_width: 12.0,
        line_height: 18.0,
    });
    let wide_font = tree.layout(viewport).text_layouts()[0].runs.len();
    assert!(wide_font > narrow_font);
    tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 2.0);
    let custom = tree.layout(viewport).text_layouts()[0].runs.len();
    assert!(custom < wide_font);
    tree.clear_text_measurer();
    let fallback = tree.layout(viewport).text_layouts()[0].runs.len();
    assert_eq!(fallback, wide_font);
}
