//! The faces a host gave egui of its own, given to limn's layout as well, so
//! that text limn lays out is drawn in what egui draws: `attach_fonts`
//! takes them from egui's fonts, and `install_face` keeps them.

use std::sync::{Mutex, PoisonError};

use eframe::egui;
use limn::{Font, FontFamily, Fonts};

use crate::{SANS_BOLD, SANS_MEDIUM, SANS_SEMIBOLD};

/// limn's own faces, by the names `font_definitions` gives them in egui's.
const LIMN_FACES: [&str; 10] = [
    "Sans-Regular",
    "Sans-Medium",
    "Sans-Semibold",
    "Sans-Bold",
    "Hack",
    "Hack-Bold",
    "NotoSansMath-Regular",
    "NotoSansSymbols2-Regular",
    "NotoEmoji",
    "Display",
];

/// The faces limn was given, and those its layout was given on top of them
/// from egui's: kept apart, so `font_definitions` never gives egui back a
/// face of its own as one of limn's.
static LENT: Mutex<Option<(Fonts, Fonts)>> = Mutex::new(None);

/// The faces limn was given, without those taken from egui's.
pub(crate) fn given() -> Fonts {
    let current = limn::fonts();
    match &*LENT.lock().unwrap_or_else(PoisonError::into_inner) {
        Some((given, laid_out)) if *laid_out == current => given.clone(),
        // None were taken, or the host set the faces since: what it set is what it gives.
        _ => current,
    }
}

/// Lay limn's text out in `given` and the faces of the host's own in egui's
/// `definitions`.
pub(crate) fn attach(given: Fonts, definitions: &egui::FontDefinitions) {
    let current = limn::fonts();
    let fonts = with_egui_faces(&given, definitions, &current);
    if fonts != current {
        limn::set_fonts(fonts.clone());
    }
    *LENT.lock().unwrap_or_else(PoisonError::into_inner) = (fonts != given).then_some((given, fonts));
}

/// `given` with the faces of the host's own in `definitions`: the first face
/// of each interface family where it is not limn's, with the face `Sans` had
/// tried right behind it, as egui keeps it behind, and the bold face limn had
/// for display text without a face of its own, as egui draws it; and the
/// faces egui tries after limn's emoji face, as last resorts. A face limn already has in
/// `known`, from the same bytes at the same instance, is kept rather than made
/// again, so the text is not laid out again.
fn with_egui_faces(given: &Fonts, definitions: &egui::FontDefinitions, known: &Fonts) -> Fonts {
    // One face for every family egui draws from one, so each is laid out once.
    let mut made: Vec<Font> = Vec::new();
    let mut font = |name: &String| -> Option<Font> {
        let data = definitions.font_data.get(name)?;
        let instance: Vec<([u8; 4], f32)> =
            data.tweak.coords.as_ref().iter().map(|(tag, value)| (tag.to_be_bytes(), *value)).collect();
        // Both keep their bytes alive, so one address is one file.
        let same = |font: &&Font| {
            font.bytes().as_ptr() == data.bytes().as_ptr()
                && font.bytes().len() == data.bytes().len()
                && font.index() == data.index
                && font.variations() == instance
        };
        let mut kept = known.faces().map(|(_, font)| font).chain(known.fallbacks()).chain(known.last_resorts()).chain(&made);
        let font = match kept.find(same) {
            Some(font) => font.clone(),
            None => Font::from_shared(data.font.clone(), data.index).ok()?.with_variations(instance),
        };
        made.push(font.clone());
        Some(font)
    };
    let mut first = |family: egui::FontFamily| {
        let name = definitions.families.get(&family)?.first()?;
        if LIMN_FACES.contains(&name.as_str()) { None } else { font(name) }
    };
    let mut faces: Vec<(FontFamily, Font)> = [
        (FontFamily::Sans, egui::FontFamily::Proportional),
        (FontFamily::SansMedium, egui::FontFamily::Name(SANS_MEDIUM.into())),
        (FontFamily::SansSemibold, egui::FontFamily::Name(SANS_SEMIBOLD.into())),
        (FontFamily::SansBold, egui::FontFamily::Name(SANS_BOLD.into())),
    ]
    .into_iter()
    .filter_map(|(family, chain)| Some((family, first(chain)?)))
    .collect();
    // egui's display text starts with limn's bold face when limn has no display face: so does limn's.
    if let (true, None, Some(bold)) = (
        faces.iter().any(|(family, _)| *family == FontFamily::SansBold),
        given.face_of(FontFamily::Display),
        given.face_of(FontFamily::SansBold).or(given.face_of(FontFamily::Sans)),
    ) {
        faces.push((FontFamily::Display, bold.clone()));
    }
    let last: Vec<Font> = definitions
        .families
        .get(&egui::FontFamily::Proportional)
        .map(|chain| chain.iter().skip_while(|name| name.as_str() != "NotoEmoji").skip(1).filter_map(&mut font).collect())
        .unwrap_or_default();
    if faces.is_empty() && last.is_empty() {
        return given.clone();
    }
    let sans = faces.iter().find(|(family, _)| *family == FontFamily::Sans).map(|(_, font)| font);
    let behind = given.face_of(FontFamily::Sans).filter(|face| sans.is_some_and(|sans| sans != *face));
    let mut fonts = Fonts::new();
    for (family, face) in given.faces() {
        fonts = fonts.face(family, face.clone());
    }
    for fallback in behind.into_iter().chain(given.fallbacks()) {
        if !fonts.fallbacks().contains(fallback) {
            fonts = fonts.fallback(fallback.clone());
        }
    }
    if let Some(emoji) = given.emoji_face() {
        fonts = fonts.emoji(emoji.clone());
    }
    for resort in given.last_resorts().iter().chain(&last) {
        if !fonts.last_resorts().contains(resort) {
            fonts = fonts.last_resort(resort.clone());
        }
    }
    for (family, face) in faces {
        fonts = fonts.face(family, face);
    }
    fonts
}
