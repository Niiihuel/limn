use super::{caption, row, section, Cx, Section};
use limn::{AlertColors, AlertTone, Component, Element, Icon, IconButton, Separator, Text, TextRole};

const ROLES: [(TextRole, &str); 8] = [
    (TextRole::Display, "Display"),
    (TextRole::Heading, "Heading"),
    (TextRole::Body, "Body"),
    (TextRole::Label, "Label"),
    (TextRole::Caption, "Caption"),
    (TextRole::Small, "Small"),
    (TextRole::Micro, "Micro"),
    (TextRole::Code, "Code"),
];

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let width = cx.width.min(520.0);
    let long = "A title much longer than the column holding it, to show how it ends";
    let tool = |id: &str, icon: Icon| IconButton::new(id, icon, palette).render();
    let sample = |role: TextRole, name: &str| {
        Element::row(vec![
            Text::new(format!("{name} · {}", role.size()), palette)
                .muted()
                .render()
                .centered_y()
                .size(Some(96.0), None),
            Text::new("The quick brown fox jumps over the lazy dog", palette).role(role).render().centered_y(),
        ])
        .gap(12.0)
    };
    vec![
        section(
            cx,
            "Scale",
            std::iter::once(caption(cx, "Text takes its size from a TextRole and its color from the palette."))
                .chain(ROLES.iter().map(|(role, name)| sample(*role, name)))
                .collect(),
        ),
        section(cx, "Outside the scale", vec![
            caption(cx, ".font_size(px) for a size the host's design places between two roles"),
            Text::new("Small · 11", palette).role(TextRole::Small).muted().render(),
            Text::new(".font_size(11.5): between Small and Caption", palette).font_size(11.5).muted().render(),
            Text::new("Caption · 12", palette).muted().render(),
            Text::new("   42", palette).mono().font_size(10.5).muted().render(),
        ]),
        section(cx, "Ink and family", vec![
            Text::eyebrow("EYEBROW", palette).render(),
            Text::new("Primary text", palette).render(),
            Text::new("Secondary text with muted()", palette).muted().render(),
            Text::new("Semantic ink with ink()", palette).ink(AlertColors::for_tone(AlertTone::Error, palette).accent).render(),
            Text::new("cargo test -p limn", palette).mono().render(),
        ]),
        section(cx, "Weights", [
            ("Regular", limn::FontFamily::Sans),
            ("Medium", limn::FontFamily::SansMedium),
            ("Bold", limn::FontFamily::SansBold),
        ].into_iter().map(|(name, family)| {
            Element::text(format!("{name} · 175.6K of 200K tokens"))
                .font_family(family)
                .font_size(15.0)
                .foreground(palette.text)
        }).collect()),
        section(cx, "Gradient ink", vec![
            caption(cx, ".foreground_gradient(stops): a name in the colors of its role"),
            Element::text("Two colors, left to right")
                .font_family(limn::FontFamily::SansBold)
                .font_size(18.0)
                .foreground_gradient(vec![limn::Color::rgb(0xE0, 0x6C, 0x75), limn::Color::rgb(0xE5, 0xC0, 0x7B)]),
            Element::text("Three colors, holographic")
                .font_family(limn::FontFamily::SansBold)
                .font_size(18.0)
                .foreground_gradient(vec![
                    limn::Color::rgb(0xA9, 0xC9, 0xFF),
                    limn::Color::rgb(0xFF, 0xBB, 0xEC),
                    limn::Color::rgb(0xFF, 0xC3, 0xA0),
                ]),
            caption(cx, "Cut short, the gradient spans what is painted"),
            Element::truncated_text(long)
                .font_size(15.0)
                .foreground_gradient(vec![palette.muted, palette.text])
                .size(Some(width.min(240.0)), None),
        ]),
        section(cx, "Inline pictures", {
            let span = |text: &str, size: f32| limn::TextSpan::new(text, palette.text, size, limn::FontFamily::Sans);
            vec![
                caption(cx, "TextSpan::image(picture) and .surface(id): a square 1.375 times the text's size, painted in place of its text, which is what a selection copies and a screen reader reads"),
                Element::formatted_text(vec![
                    span("A custom emoji ", 16.0),
                    span(":dusk:", 16.0).image(super::media::picture(0)),
                    span(" in a line, one that moves ", 16.0),
                    span(":tide:", 16.0).surface(super::media::ANIMATION),
                    span(" on the host's surface, and the line wraps around them whole.", 16.0),
                ])
                .size(Some(width), None),
                caption(cx, "Alone, larger: a message that is only emoji, at 48 px"),
                Element::formatted_text(vec![
                    span(":forest:", 48.0 / limn::INLINE_IMAGE_SCALE).image(super::media::picture(1)),
                    span(" ", 48.0 / limn::INLINE_IMAGE_SCALE),
                    span(":dusk:", 48.0 / limn::INLINE_IMAGE_SCALE).image(super::media::picture(0)),
                ]),
            ]
        }),
        section(cx, "Long text", vec![
            Element::column(vec![
                caption(cx, "truncate() with tooltip()"),
                Text::new(long, palette).truncate().tooltip("typography-truncated").render(),
                caption(cx, "fade()"),
                Text::new(long, palette).fade(32.0).render(),
                caption(cx, "No options: the paragraph wraps"),
                Text::new(long, palette).line_height(20.0).render(),
            ])
            .gap(6.0)
            .size(Some(width.min(280.0)), None),
        ]),
        section(cx, "Soft focus", vec![
            Element::column(
                [0.0, 1.0, 2.0, 3.5]
                    .into_iter()
                    .enumerate()
                    .map(|(index, radius)| Text::new("The line that has not been sung yet", palette).font_size(20.0).render().soft_focus(format!("typography-soft:{index}"), radius))
                    .collect(),
            )
            .gap(8.0),
            caption(cx, "soft_focus: sharp, then at radii of 1, 2 and 3.5. The radius springs to its new value."),
        ]),
        section(cx, "Unicode fallback", std::iter::once(
            caption(cx, "Mathematical alphabets and decorative symbols: bundled Noto Math and Symbols 2 fallbacks; no system font needed."),
        ).chain([
            ("Regular", limn::FontFamily::Sans),
            ("Italic", limn::FontFamily::SansItalic),
            ("Medium", limn::FontFamily::SansMedium),
            ("Bold", limn::FontFamily::SansBold),
            ("Monospace", limn::FontFamily::Monospace),
        ].into_iter().map(|(name, family)| {
            Element::text(format!("{name} · 𝖗𝖚𝖓𝖊 · ╭─ ✦ Lounge ❖ ➜"))
                .font_family(family)
                .font_size(16.0)
                .foreground(palette.text)
        })).collect()),
        section(cx, "Monochrome emoji", std::iter::once(
            caption(cx, "Noto Emoji fills missing glyphs after the existing fonts; prior text keeps its face and width."),
        ).chain([
            ("Regular", limn::FontFamily::Sans),
            ("Italic", limn::FontFamily::SansItalic),
            ("Medium", limn::FontFamily::SansMedium),
            ("Bold", limn::FontFamily::SansBold),
            ("Monospace", limn::FontFamily::Monospace),
        ].into_iter().map(|(name, family)| {
            Element::text(format!("{name} · 🙂 🤣 🦀 🫡 🤔 🧑‍💻 🏴‍☠️"))
                .font_family(family)
                .font_size(20.0)
                .foreground(palette.text)
        })).chain([
            caption(cx, "Sequence limits: regional flags and some keycaps / joined flags can still use an earlier font."),
            Element::text("🇦🇷 · #️⃣ · 🏳️‍🌈").font_size(20.0).foreground(palette.muted),
        ]).collect()),
        section(cx, "Color emoji", vec![
            caption(cx, "limn_egui::set_color_emoji: the host's artwork over the glyphs; here synthetic discs for 😀, ❤️ and 👍🏽. Width, selection and copied text stay the font's."),
            Element::text("Text 😀 with ❤️ and 👍🏽, and ❤ without VS16").font_size(20.0).foreground(palette.text),
            Element::text("Italic 😀 · faded ❤️").font_family(limn::FontFamily::SansItalic).font_size(16.0).foreground(palette.muted),
        ]),
        section(cx, "Separator", vec![
            Element::column(vec![
                Text::new("Above", palette).render(),
                Separator::horizontal(palette).render(),
                Text::new("Below, with margin", palette).render(),
                Separator::horizontal(palette).margin(8.0).render(),
                Text::new("Fixed length", palette).render(),
                Separator::horizontal(palette).length(96.0).render(),
                Text::new("With ink()", palette).render(),
                Separator::horizontal(palette).ink(palette.muted).render(),
            ])
            .gap(8.0)
            .size(Some(width), None),
            row(vec![
                tool("typography-back", Icon::ArrowLeft),
                tool("typography-forward", Icon::ArrowRight),
                Separator::vertical(palette).length(16.0).centered_in(28.0).margin(4.0).render(),
                tool("typography-refresh", Icon::Refresh),
                Separator::vertical(palette).render(),
                tool("typography-archive", Icon::Archive),
            ]),
            caption(cx, "Vertical: centered in the toolbar with margin, and with no options (fills the height)."),
        ]),
    ]
}

/// The gallery's color emoji: a disc of one color for each of three
/// clusters, drawn here rather than shipping anyone's artwork. `❤` without
/// VS16 asks for text presentation and keeps its glyph.
pub fn demo_emoji(cluster: &str) -> Option<&'static [u8]> {
    use std::sync::OnceLock;
    static DISCS: OnceLock<Vec<Vec<u8>>> = OnceLock::new();
    let discs = DISCS.get_or_init(|| {
        [[255, 204, 77], [221, 46, 68], [175, 126, 87]]
            .into_iter()
            .map(|[r, g, b]| {
                let disc = image::RgbaImage::from_fn(72, 72, |x, y| {
                    let (dx, dy) = (x as f32 - 35.5, y as f32 - 35.5);
                    let alpha = (33.0 - (dx * dx + dy * dy).sqrt()).clamp(0.0, 1.0);
                    image::Rgba([r, g, b, (alpha * 255.0) as u8])
                });
                let mut png = Vec::new();
                image::DynamicImage::ImageRgba8(disc)
                    .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
                    .expect("encode a disc");
                png
            })
            .collect()
    });
    let index = match cluster {
        "😀" => 0,
        "❤\u{fe0f}" => 1,
        "👍🏽" => 2,
        _ => return None,
    };
    Some(&discs[index])
}
