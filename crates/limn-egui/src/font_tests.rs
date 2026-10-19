//! Exercise the actual egui font lookup without a window or a system font.

use super::*;

/// Whether a face of `family`'s chain maps `ch`. egui's own `has_glyph` says
/// no for a character of the face it takes its replacement glyph from, so
/// coverage is read from the faces themselves.
fn covers(ctx: &egui::Context, family: &egui::FontFamily, ch: char) -> bool {
    use skrifa::MetadataProvider;
    let definitions = ctx.fonts(|fonts| fonts.definitions().clone());
    definitions.families[family].iter().any(|name| {
        let data = &definitions.font_data[name];
        skrifa::FontRef::from_index(data.font.as_ref(), data.index)
            .is_ok_and(|face| face.charmap().map(ch).is_some_and(|id| id != skrifa::GlyphId::NOTDEF))
    })
}

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
fn hosts_can_extend_bundled_definitions_without_losing_fallbacks() {
    let mut definitions = font_definitions();
    let fallback = definitions.families[&egui::FontFamily::Proportional].clone();
    definitions.font_data.insert("host-face".into(), definitions.font_data["Ubuntu-Regular"].clone());
    definitions.families.get_mut(&egui::FontFamily::Proportional).unwrap().insert(0, "host-face".into());
    let ctx = egui::Context::default();
    ctx.set_fonts(definitions);
    let mut output = ctx.run_ui(egui::RawInput::default(), |_| {});
    output.textures_delta.clear();
    ctx.fonts_mut(|fonts| {
        let family = &fonts.definitions().families[&egui::FontFamily::Proportional];
        assert_eq!(family.first().map(String::as_str), Some("host-face"));
        assert_eq!(family[1..], fallback);
    });
    assert!(covers(&ctx, &egui::FontFamily::Proportional, '𝓡'));
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
        FontFamily::SansBoldItalic,
        FontFamily::Monospace,
    ] {
        let font = egui::FontId::new(16.0, egui_family(&ctx, family));
        for ch in sample.chars().filter(|ch| !ch.is_whitespace()).chain(
            (0x1d400..=0x1d7ff)
                .filter_map(char::from_u32)
                .filter(|ch| ch.is_alphanumeric()),
        ) {
            assert!(
                covers(&ctx, &font.family, ch),
                "{family:?} is missing U+{:04X} ({ch})",
                ch as u32
            );
        }
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
    assert!(limn::Fonts::bundled().fallbacks()[1].bytes().len() <= 400 * 1024);
    for family in [
        FontFamily::Sans,
        FontFamily::SansItalic,
        FontFamily::SansMedium,
        FontFamily::SansBold,
        FontFamily::SansBoldItalic,
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

#[test]
fn emoji_catalog_scalars_have_real_glyphs_in_every_family() {
    use skrifa::MetadataProvider;

    let ctx = context(true);
    let definitions = ctx.fonts(|fonts| fonts.definitions().clone());
    let samples = include_str!("emoji_samples.txt");
    assert_eq!(samples.lines().count(), 623);
    assert!(limn::Fonts::bundled().emoji_face().unwrap().bytes().len() <= 2 * 1024 * 1024);
    for family in [
        FontFamily::Sans,
        FontFamily::SansItalic,
        FontFamily::SansMedium,
        FontFamily::SansBold,
        FontFamily::SansBoldItalic,
        FontFamily::Monospace,
    ] {
        let names = &definitions.families[&egui_family(&ctx, family)];
        assert_eq!(names.last().map(String::as_str), Some("NotoEmoji"));
        let faces: Vec<_> = names
            .iter()
            .map(|name| {
                let data = &definitions.font_data[name];
                skrifa::FontRef::from_index(data.font.as_ref(), data.index).unwrap()
            })
            .collect();
        let mut missing = std::collections::BTreeSet::new();
        for ch in samples
            .chars()
            .filter(|ch| !ch.is_whitespace() && !matches!(ch, '\u{200d}' | '\u{fe0f}'))
        {
            if !faces.iter().any(|face| {
                face.charmap()
                    .map(ch)
                    .is_some_and(|id| id != skrifa::GlyphId::NOTDEF)
            }) {
                missing.insert(ch);
            }
        }
        assert!(
            missing.is_empty(),
            "{family:?} lacks {} emoji scalars: {missing:?}",
            missing.len()
        );
        let font = egui::FontId::new(24.0, egui_family(&ctx, family));
        for ch in "🙂🤣🦀🫡".chars() {
            assert!(
                !faces[..faces.len() - 1].iter().any(|face| face
                    .charmap()
                    .map(ch)
                    .is_some_and(|id| id != skrifa::GlyphId::NOTDEF)),
                "fixture must exercise a new glyph"
            );
            assert!(ctx.fonts_mut(|fonts| fonts.has_glyph(&font, ch)));
        }
    }
}

#[test]
fn emoji_fallback_preserves_existing_text_metrics() {
    use skrifa::MetadataProvider;

    let ctx = context(true);
    let mut previous = ctx.fonts(|fonts| fonts.definitions().clone());
    previous.font_data.remove("NotoEmoji");
    for names in previous.families.values_mut() {
        names.retain(|name| name != "NotoEmoji");
    }
    let baseline = egui::Context::default();
    baseline.set_fonts(previous.clone());
    baseline
        .run_ui(Default::default(), |_| {})
        .drop_without_applying_deltas();
    for family in [
        FontFamily::Sans,
        FontFamily::SansItalic,
        FontFamily::SansMedium,
        FontFamily::SansBold,
        FontFamily::SansBoldItalic,
        FontFamily::Monospace,
    ] {
        let font = egui::FontId::new(16.0, egui_family(&ctx, family));
        let faces: Vec<_> = previous.families[&font.family]
            .iter()
            .map(|name| {
                let data = &previous.font_data[name];
                skrifa::FontRef::from_index(data.font.as_ref(), data.index).unwrap()
            })
            .collect();
        for ch in include_str!("emoji_samples.txt")
            .chars()
            .chain("Wi 0123 RUNE".chars())
        {
            if !faces.iter().any(|face| {
                face.charmap()
                    .map(ch)
                    .is_some_and(|id| id != skrifa::GlyphId::NOTDEF)
            }) {
                continue;
            }
            assert_eq!(
                baseline.fonts_mut(|fonts| fonts.glyph_width(&font, ch)),
                ctx.fonts_mut(|fonts| fonts.glyph_width(&font, ch)),
                "{family:?} changed {ch}"
            );
        }
        // Only Noto Emoji draws emoji now that egui's own faces are not
        // installed, so text the other faces cover is what keeps its layout.
        for text in ["RUNE 0123 Wi", "𝖗𝖚𝖓𝖊 ✦ Lounge"]
        {
            let layout = |ctx: &egui::Context| {
                ctx.fonts_mut(|fonts| {
                    fonts
                        .layout_no_wrap(text.into(), font.clone(), egui::Color32::WHITE)
                        .rect
                        .size()
                })
            };
            assert_eq!(
                layout(&baseline),
                layout(&ctx),
                "{family:?} changed existing layout"
            );
        }
    }
}

#[test]
fn variation_selectors_and_supported_joined_emoji_have_one_painted_glyph() {
    let ctx = context(true);
    for family in [
        FontFamily::Sans,
        FontFamily::SansItalic,
        FontFamily::SansMedium,
        FontFamily::SansBold,
        FontFamily::SansBoldItalic,
        FontFamily::Monospace,
    ] {
        let font = egui::FontId::new(24.0, egui_family(&ctx, family));
        let painted = |text: &str| {
            ctx.fonts_mut(|fonts| {
                fonts
                    .layout_no_wrap(text.into(), font.clone(), egui::Color32::WHITE)
                    .rows
                    .iter()
                    .flat_map(|row| &row.glyphs)
                    .filter(|glyph| glyph.uv_rect.size.x > 0.0 && glyph.uv_rect.size.y > 0.0)
                    .map(|glyph| glyph.uv_rect)
                    .collect::<Vec<_>>()
            })
        };
        for text in ["❤️", "☀️", "🧑‍💻", "🏴‍☠️"] {
            assert_eq!(
                painted(text).len(),
                1,
                "{family:?} failed to compose {text}"
            );
        }
        // A single base glyph with the suffix dropped would also count as one.
        for (joined, base) in [("🧑‍💻", "🧑"), ("🏴‍☠️", "🏴")] {
            assert_ne!(
                painted(joined),
                painted(base),
                "{family:?} dropped the joined suffix"
            );
        }
    }
}

