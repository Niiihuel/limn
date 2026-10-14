//! Exercise the actual egui font lookup without a window or a system font.

use super::*;

fn context(install: bool) -> egui::Context {
    let ctx = egui::Context::default();
    if install {
        install_fonts(&ctx);
    }
    let mut output = ctx.run_ui(egui::RawInput::default(), |_| {});
    output.textures_delta.clear();
    ctx
}

#[test]
fn mathematical_names_have_real_glyphs_in_every_limn_family() {
    let baseline = context(false);
    let ctx = context(true);
    let sample = "𝖗𝖚𝖓𝖊 𝓡𝓤𝓝𝓔 𝐑𝐔𝐍𝐄 𝟘𝟙𝟚 ℂℍℕℙℚℝℤℎℬℰℱℋℐℒℳℛ";
    let sans = egui::FontId::proportional(16.0);
    assert!(
        baseline.fonts_mut(|fonts| sample
            .chars()
            .filter(|ch| !ch.is_whitespace())
            .any(|ch| !fonts.has_glyph(&sans, ch))),
        "the fixture must exercise characters missing from the default fonts"
    );
    for family in [
        FontFamily::Sans,
        FontFamily::SansItalic,
        FontFamily::SansMedium,
        FontFamily::SansBold,
        FontFamily::Monospace,
    ] {
        let font = egui::FontId::new(16.0, egui_family(&ctx, family));
        ctx.fonts_mut(|fonts| {
            for ch in sample.chars().filter(|ch| !ch.is_whitespace()).chain(
                (0x1d400..=0x1d7ff)
                    .filter_map(char::from_u32)
                    .filter(|ch| ch.is_alphanumeric()),
            ) {
                assert!(
                    fonts.has_glyph(&font, ch),
                    "{family:?} is missing U+{:04X} ({ch})",
                    ch as u32
                );
            }
        });
    }
}

#[test]
fn installing_fallback_keeps_existing_ui_faces_and_monospace_metrics() {
    let baseline = context(false);
    let ctx = context(true);
    ctx.fonts(|fonts| {
        let families = &fonts.definitions().families;
        for (family, first) in [
            (egui::FontFamily::Proportional, "Ubuntu-Regular"),
            (egui::FontFamily::Name(SANS_MEDIUM.into()), "Ubuntu-Medium"),
            (egui::FontFamily::Name(SANS_BOLD.into()), "Ubuntu-Bold"),
        ] {
            assert_eq!(families[&family].first().map(String::as_str), Some(first));
        }
    });
    let font = egui::FontId::monospace(14.0);
    for ch in "Wi 01".chars() {
        assert_eq!(
            baseline.fonts_mut(|fonts| fonts.glyph_width(&font, ch)),
            ctx.fonts_mut(|fonts| fonts.glyph_width(&font, ch))
        );
    }
}
