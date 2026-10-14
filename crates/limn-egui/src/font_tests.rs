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

#[test]
fn decorative_symbols_have_real_glyphs_in_every_limn_family() {
    use skrifa::MetadataProvider;

    let ctx = context(true);
    let sample = "◆◇●○■□▪▫▸▹▾▿⌜⌟├─│╭╰═║⟡✦✧❖➜→←⇢➤⤷⎯⏤⠿⣿";
    let definitions = ctx.fonts(|fonts| fonts.definitions().clone());
    assert!(include_bytes!("../assets/fonts/NotoSansSymbols2-Regular.otf").len() <= 400 * 1024);
    for family in [
        FontFamily::Sans,
        FontFamily::SansItalic,
        FontFamily::SansMedium,
        FontFamily::SansBold,
        FontFamily::Monospace,
    ] {
        let font = egui::FontId::new(16.0, egui_family(&ctx, family));
        let names = &definitions.families[&font.family];
        let math = names
            .iter()
            .position(|name| name == "NotoSansMath-Regular")
            .unwrap();
        assert_eq!(names[math + 1], "NotoSansSymbols2-Regular");
        let covers = |name: &str, ch| {
            let data = &definitions.font_data[name];
            let face = skrifa::FontRef::from_index(data.font.as_ref(), data.index).unwrap();
            face.charmap()
                .map(ch)
                .is_some_and(|glyph| glyph != skrifa::GlyphId::NOTDEF)
        };
        for ch in sample.chars() {
            // has_glyph compares face IDs in egui 0.36: a valid glyph on the
            // replacement face (e.g. box drawing) incorrectly returns false.
            // Check the installed chain's actual charmaps for the full fixture.
            assert!(
                names.iter().any(|name| covers(name, ch)),
                "{family:?} is missing U+{:04X} ({ch})",
                ch as u32
            );
        }
        assert!(
            !names
                .iter()
                .filter(|name| *name != "NotoSansSymbols2-Regular")
                .any(|name| covers(name, '✦')),
            "the baseline must lack our regression glyph"
        );
        // This scalar is absent from every previous face (asserted above),
        // so has_glyph also verifies egui actually resolves our new fallback.
        assert!(
            ctx.fonts_mut(|fonts| fonts.has_glyph(&font, '✦')),
            "{family:?} must resolve the installed symbol fallback"
        );
    }
}
