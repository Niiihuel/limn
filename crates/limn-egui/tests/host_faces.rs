//! A host's own faces in egui's fonts are the faces limn's layout lays text
//! out in once `attach_fonts` runs. That changes `limn::fonts()` for the
//! whole process, so this file holds one test.

use std::sync::Arc;

use eframe::egui;
use egui::epaint::text::VariationCoords;
use limn::{Color, DrawCommand, Element, FontFamily, GlyphRun, Point, Size, TextLayout, TextStyle, UiTree};

/// Let egui take the fonts it was given.
fn settle(ctx: &egui::Context) {
    // This probe has no renderer to take the texture updates.
    ctx.run_ui(egui::RawInput::default(), |_| {}).textures_delta.clear();
}

fn runs(text: &str, family: FontFamily) -> Vec<GlyphRun> {
    TextLayout::new(text, TextStyle::new(14.0, family))
        .draw(Point::new(0.0, 0.0), Color::WHITE)
        .into_iter()
        .filter_map(|command| match command {
            DrawCommand::Glyphs(run) => Some(run),
            _ => None,
        })
        .collect()
}

/// The one run of `text`: where its face's bytes are, and its instance.
fn run(text: &str, family: FontFamily) -> (*const u8, Vec<i16>) {
    let runs = runs(text, family);
    assert_eq!(runs.len(), 1, "{text} in {family:?}");
    (runs[0].font.bytes().as_ptr(), runs[0].coords.to_vec())
}

#[test]
fn a_hosts_faces_in_egui_are_the_faces_limn_lays_its_text_out_in() {
    let bundled = limn::fonts();
    let ubuntu = bundled.face_of(FontFamily::Sans).unwrap().clone();
    let hack = bundled.face_of(FontFamily::Monospace).unwrap().clone();
    // Noto Emoji, a variable face (weight 300 to 700, 400 by default), is the
    // host's one file for its regular and bold weights; Hack is its medium.
    let file: Arc<dyn AsRef<[u8]> + Send + Sync> = Arc::new(bundled.emoji_face().unwrap().bytes().to_vec());
    let at = |weight: f32| egui::FontData {
        tweak: egui::FontTweak { coords: VariationCoords::new([(b"wght", weight)]), ..Default::default() },
        ..egui::FontData::from_blob(file.clone(), 0)
    };
    let medium: Arc<dyn AsRef<[u8]> + Send + Sync> = Arc::new(hack.bytes().to_vec());
    let faces = || limn_egui::UiFaces {
        regular: at(300.0),
        medium: Some(egui::FontData::from_blob(medium.clone(), 0)),
        semibold: None,
        bold: Some(at(700.0)),
    };
    let (host, host_medium) = ((*file).as_ref().as_ptr(), (*medium).as_ref().as_ptr());
    const LIGHTEST: i16 = -16384;
    const HEAVIEST: i16 = 16384;

    // Without faces of the host's own, limn's are as they were.
    let ctx = egui::Context::default();
    limn_egui::install_fonts(&ctx);
    settle(&ctx);
    let mut tree = UiTree::new(Element::column(vec![
        Element::text("🦀").font_family(FontFamily::Sans),
        Element::text("🦀").font_family(FontFamily::SansBold),
    ]));
    limn_egui::attach_fonts(&mut tree, &ctx);
    assert_eq!(limn::fonts(), bundled);
    assert_eq!(run("a", FontFamily::Sans).0, ubuntu.bytes().as_ptr());

    // Given to egui, then attached: limn's layout takes them, egui's bytes and all.
    ctx.set_fonts(limn_egui::font_definitions_with(faces()));
    settle(&ctx);
    limn_egui::attach_fonts(&mut tree, &ctx);
    let lent = limn::fonts();
    assert_eq!(lent.face_of(FontFamily::Sans).unwrap().variations(), [(*b"wght", 300.0)]);
    assert_eq!(run("🦀", FontFamily::Sans), (host, vec![LIGHTEST]));
    assert_eq!(run("🦀", FontFamily::SansItalic), (host, vec![LIGHTEST]), "regular, slanted");
    assert_eq!(run("a", FontFamily::SansMedium).0, host_medium);
    assert_eq!(run("🦀", FontFamily::SansBold), (host, vec![HEAVIEST]));
    assert_eq!(run("🦀", FontFamily::SansSemibold), (host, vec![HEAVIEST]), "no semibold: the bold, as CSS");
    // What the regular face lacks is drawn by the face behind it in egui too; code keeps its face.
    assert_eq!(run("a", FontFamily::Sans).0, ubuntu.bytes().as_ptr());
    assert_eq!(run("a", FontFamily::Monospace).0, hack.bytes().as_ptr());
    let ubuntu_bold = bundled.face_of(FontFamily::SansBold).unwrap().bytes().as_ptr();
    assert_eq!(run("a", FontFamily::Display).0, ubuntu_bold, "as egui's display text, limn's bold");

    // A tree's text is laid out, and painted, at each weight's instance.
    let frame = tree.layout(Size::new(200.0, 100.0));
    let laid: Vec<Vec<i16>> = frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Glyphs(run) => Some(run.coords.to_vec()),
            _ => None,
        })
        .collect();
    assert_eq!(laid, [vec![LIGHTEST], vec![HEAVIEST]]);
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| limn_egui::paint(ui, &frame));
    let quads: Vec<egui::Vec2> = output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Mesh(mesh) => Some(mesh.calc_bounds().size()),
            _ => None,
        })
        .collect();
    output.textures_delta.clear();
    assert_eq!(quads.len(), 2);
    assert!(quads[1].x > quads[0].x, "the heavier crab is drawn wider: {quads:?}");

    // Attached again, nothing changes; egui's own fonts never get the host's faces as limn's.
    limn_egui::attach_fonts(&mut tree, &ctx);
    assert_eq!(limn::fonts(), lent);
    let definitions = limn_egui::font_definitions();
    assert_eq!(definitions.font_data["Sans-Regular"].bytes().as_ptr(), ubuntu.bytes().as_ptr());
    assert_eq!(definitions.font_data["Sans-Regular"].tweak.coords, VariationCoords::default());

    // A face installed meanwhile joins them; neither is lost or doubled.
    limn_egui::install_face(&ctx, 3, hack.bytes().to_vec()).unwrap();
    settle(&ctx);
    limn_egui::attach_fonts(&mut tree, &ctx);
    let installed = limn::fonts();
    assert!(installed.face_of(FontFamily::Custom(3)).is_some());
    assert_eq!(installed.face_of(FontFamily::Sans), lent.face_of(FontFamily::Sans));
    assert_eq!(installed.fallbacks(), lent.fallbacks());
    assert_eq!(installed.fallbacks().len(), bundled.fallbacks().len() + 1);

    // A face the host puts behind every family, as one found for a script none draws.
    let mut found = ctx.fonts(|fonts| fonts.definitions().clone());
    let system: Arc<dyn AsRef<[u8]> + Send + Sync> = Arc::new(ubuntu.bytes().to_vec());
    found.font_data.insert("system".into(), egui::FontData::from_blob(system.clone(), 0).into());
    for family in found.families.values_mut() {
        family.push("system".into());
    }
    ctx.set_fonts(found);
    settle(&ctx);
    limn_egui::attach_fonts(&mut tree, &ctx);
    let resorts: Vec<*const u8> = limn::fonts().last_resorts().iter().map(|font| font.bytes().as_ptr()).collect();
    assert_eq!(resorts, [(*system).as_ref().as_ptr()]);
    assert_eq!(limn::fonts().face_of(FontFamily::Sans), lent.face_of(FontFamily::Sans));

    // Gone from egui's fonts, gone from limn's.
    ctx.set_fonts(limn_egui::font_definitions());
    settle(&ctx);
    limn_egui::attach_fonts(&mut tree, &ctx);
    let back = limn::fonts();
    assert_eq!(back.face_of(FontFamily::Sans), Some(&ubuntu));
    assert_eq!(back.fallbacks(), bundled.fallbacks());
    assert!(back.last_resorts().is_empty());
    assert!(back.face_of(FontFamily::Custom(3)).is_some(), "what limn was given stays");
    assert_eq!(back.face_of(FontFamily::Display), None);
    assert_eq!(run("🦀", FontFamily::SansBold), (bundled.emoji_face().unwrap().bytes().as_ptr(), vec![HEAVIEST]));
}
