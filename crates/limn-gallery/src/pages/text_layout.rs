use super::{caption, section, Cx, Section};
use limn::{AlertColors, AlertTone, Color, DrawCommand, Element, FontFamily, Leading, Point, Rect, TextLayout, TextStyle};

const SAMPLES: [(&str, FontFamily, f32); 5] = [
    ("Wavy AVATAR, fine type: 0.1.44 · ∑ 𝑥 ⌘ 😀", FontFamily::Sans, 14.0),
    ("Bold weight, set from its own face", FontFamily::SansBold, 14.0),
    ("Italic, slanted from the regular face", FontFamily::SansItalic, 14.0),
    ("fn main() { let answer = 0x2A; } ├── src", FontFamily::Monospace, 13.0),
    ("Interfaces, drawn in Rust.", FontFamily::Sans, 22.0),
];

const PARAGRAPH: &str = "limn shapes this paragraph itself, breaks it where the language allows, \
and places every glyph; the caret, the selection and what is painted come from that one layout.";

/// A layout drawn at its own size, from its top-left corner.
fn drawn(layout: TextLayout, ink: Color) -> Element {
    let size = layout.size();
    Element::drawing(move |_| layout.draw(Point::new(0.0, 0.0), ink)).size(Some(size.width), Some(size.height))
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let ink = palette.text;
    let accent = AlertColors::for_tone(AlertTone::Info, palette).accent;
    let width = cx.width.min(420.0);

    let mut families: Vec<Element> =
        SAMPLES.iter().map(|(text, family, size)| drawn(TextLayout::new(*text, TextStyle::new(*size, *family)), ink)).collect();
    families.push(caption(
        cx,
        "Each family and weight, laid out by limn and drawn from limn-egui's glyph atlas, as every Element::text is in a tree that attach_fonts set up.",
    ));

    // A line twice the text's height, outlined, so the spare room shows.
    let leading = |leading: Leading| {
        let layout = TextLayout::new(
            format!("{leading:?}"),
            TextStyle::new(14.0, FontFamily::Sans).line_height(36.0).leading(leading),
        );
        let line = Rect { width: 96.0, ..layout.lines().next().expect("one line").rect };
        Element::drawing(move |_| {
            let mut commands = vec![DrawCommand::StrokeRect { rect: line, color: palette.border, width: 1.0, radius: 0.0 }];
            commands.extend(layout.draw(Point::new(6.0, 0.0), ink));
            commands
        })
        .size(Some(96.0), Some(36.0))
    };

    let paragraph = TextLayout::new(PARAGRAPH, TextStyle::new(14.0, FontFamily::Sans)).wrapped(width);
    let (rule, size) = (palette.border, paragraph.size());
    let lined = paragraph.clone();
    let wrapped = Element::drawing(move |_| {
        let mut commands: Vec<DrawCommand> = lined
            .lines()
            .map(|line| DrawCommand::StrokeRect { rect: line.rect, color: rule, width: 1.0, radius: 0.0 })
            .collect();
        commands.extend(lined.draw(Point::new(0.0, 0.0), ink));
        commands
    })
    .size(Some(size.width), Some(size.height));

    let phrase = TextLayout::new("Select a word, then type after it.", TextStyle::new(16.0, FontFamily::Sans));
    let word = 9..13;
    let (marked, size) = (phrase.clone(), phrase.size());
    let selected = Element::drawing(move |_| {
        let mut commands: Vec<DrawCommand> = marked
            .selection(word.clone())
            .into_iter()
            .map(|rect| DrawCommand::FillRect { rect, color: Color { a: 70, ..accent }, radius: 2.0 })
            .collect();
        commands.extend(marked.draw(Point::new(0.0, 0.0), ink));
        let caret = marked.caret(word.end);
        commands.push(DrawCommand::FillRect { rect: Rect { width: 1.5, ..caret }, color: accent, radius: 0.0 });
        commands
    })
    .size(Some(size.width), Some(size.height));

    vec![
        section(cx, "Families", families),
        section(cx, "Leading", vec![
            super::row(vec![leading(Leading::Even), leading(Leading::Below)]),
            caption(cx, "TextStyle::leading: in a line taller than its glyphs, Even sets half the spare room above them, as CSS does, and Below sets them at the top, as limn's own text is set."),
        ]),
        section(cx, "Wrapping", vec![
            wrapped,
            caption(cx, "TextLayout::wrapped(width): broken where the language allows, each line's box drawn from TextLayout::lines()."),
        ]),
        section(cx, "Caret and selection", vec![
            selected,
            caption(cx, "TextLayout::selection(9..13) and TextLayout::caret(13): the same layout the glyphs come from, so they always line up."),
        ]),
    ]
}
