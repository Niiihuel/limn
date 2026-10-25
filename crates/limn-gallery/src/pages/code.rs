use super::{caption, row, section, Cx, Section};
use limn::{
    Button, ButtonVariant, CodeDiff, CodeEditor, EditorController, CodeDiffLine, CodeDiffLineKind, CodeLanguage, Component, DiffFileHeader,
    Element, RadiusRole,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

type Rows = Arc<[CodeDiffLine]>;

thread_local! {
    /// A diff colors its rows once for each allocation it is given, so a demo
    /// hands it the same one frame after frame and another only when the
    /// number of its rows changes.
    static ROWS: RefCell<HashMap<&'static str, Rows>> = RefCell::default();
}

/// The demo's buffer: one for the life of the gallery, as a host keeps its own.
fn editor() -> EditorController {
    static EDITOR: std::sync::OnceLock<EditorController> = std::sync::OnceLock::new();
    const TEXT: &str = "/// Greets whoever it is given.\nfn greet(name: &str) -> String {\n    let name = name.trim();\n    if name.is_empty() {\n        return \"Hello.\".to_owned();\n    }\n    format!(\"Hello, {name}.\")\n}\n\nfn main() {\n    println!(\"{}\", greet(\"limn\"));\n}\n";
    EDITOR.get_or_init(|| EditorController::new(limn_editor::TextBuffer::new(TEXT).expect("a short text fits"))).clone()
}

fn kept(name: &'static str, count: usize, build: impl FnOnce() -> Vec<CodeDiffLine>) -> Rows {
    ROWS.with(|rows| {
        let mut rows = rows.borrow_mut();
        match rows.get(name) {
            Some(kept) if kept.len() == count => kept.clone(),
            _ => {
                let built: Rows = build().into();
                rows.insert(name, built.clone());
                built
            }
        }
    })
}

/// A change to one function: a space before a line both sides have, `-`
/// before one only the old side has and `+` before one only the new side has.
const PATCH: [(char, &str); 12] = [
    (' ', "/// The sum of the even numbers up to `limit`."),
    ('-', "fn total(limit: u32) -> u32 {"),
    ('-', "    let mut sum = 0;"),
    ('-', "    for n in 1..=limit {"),
    ('-', "        if n % 2 == 0 {"),
    ('-', "            sum += n;"),
    ('-', "        }"),
    ('-', "    }"),
    ('-', "    sum"),
    ('+', "pub fn total(limit: u32) -> u32 {"),
    ('+', "    (1..=limit).filter(|n| n % 2 == 0).sum() // one pass, no state"),
    (' ', "}"),
];

/// Rows from marked lines, numbered on each side from `first`.
fn rows(patch: impl IntoIterator<Item = (char, String)>, first: usize) -> Vec<CodeDiffLine> {
    let (mut old, mut new) = (first, first);
    let number =|counter: &mut usize| {
        *counter += 1;
        *counter - 1
    };
    patch
        .into_iter()
        .map(|(mark, text)| {
            let kind = match mark {
                '+' => CodeDiffLineKind::Added,
                '-' => CodeDiffLineKind::Removed,
                _ => CodeDiffLineKind::Context,
            };
            CodeDiffLine {
                kind,
                old_line: (kind != CodeDiffLineKind::Added).then(|| number(&mut old)),
                new_line: (kind != CodeDiffLineKind::Removed).then(|| number(&mut new)),
                text,
            }
        })
        .collect()
}

/// As tall as `count` rows, which are 20 px each inside 12 px of padding.
fn height_of(count: usize) -> f32 {
    count as f32 * 20.0 + 12.0
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let width = cx.width.min(620.0);
    let framed = |diff: CodeDiff| {
        Element::panel(diff.palette(palette).render())
            .radius(RadiusRole::Surface.px())
            .clip_children()
            .size(Some(width), None)
    };
    let patch = || kept("patch", PATCH.len(), || rows(PATCH.map(|(mark, text)| (mark, text.to_owned())), 41));
    let long = kept("long", 60, || {
        rows((0..60).map(|row| (if row % 7 == 3 { '+' } else { ' ' }, format!("let value_{row} = compute({row});"))), 1)
    });
    let arrived = cx.state.choice("code-diff-tail", "6").parse::<usize>().unwrap_or(6).clamp(6, 500);
    let tail = kept("tail", arrived, || {
        rows((0..arrived).map(|row| (' ', format!("test suite::case_{:04} ... ok", row + 1))), 1)
    });
    // One changed word on each side, and the unchanged lines around them folded until asked for.
    // A click flips the flag named by its ID: the folded row's, and the header's.
    let unfolded = cx.state.flag("code-diff-fold:expand:0");
    let review = kept(if unfolded { "review-open" } else { "review" }, if unfolded { 9 } else { 4 }, || {
        let mut lines = rows(
            [
                ('-', "const GREETING: &str = \"Hello\";"),
                ('+', "const GREETING: &str = \"Hi\";"),
            ]
            .map(|(mark, text)| (mark, text.to_owned())),
            7,
        );
        let around = rows((1..7).map(|n| (' ', format!("// line {n}"))), 1);
        if unfolded {
            lines.splice(0..0, around);
        } else {
            lines.insert(0, CodeDiffLine {
                kind: CodeDiffLineKind::Folded,
                old_line: None,
                new_line: None,
                text: "Show 6 more lines".into(),
            });
        }
        lines.push(rows([(' ', "fn main() {}".to_owned())], 8).remove(0));
        lines
    });
    let words: Arc<[Vec<std::ops::Range<usize>>]> = review
        .iter()
        .map(|line| match line.kind {
            CodeDiffLineKind::Removed => vec![24..29],
            CodeDiffLineKind::Added => vec![24..26],
            _ => Vec::new(),
        })
        .collect::<Vec<_>>()
        .into();
    let header_collapsed = cx.state.flag("code-diff-file");
    let outline = |id: String, label: &str| Button::new(id, label, palette).variant(ButtonVariant::Outline).render();
    vec![
        section(cx, "Code editor", vec![
            caption(cx, "An editor draws a limn_editor::TextBuffer that the host owns, behind an EditorController it keeps between frames."),
            // Named for a screen reader as the file it shows.
            CodeEditor::new("code-editor", editor())
                .palette(palette)
                .language(CodeLanguage::Rust)
                .height(220.0)
                .render()
                .accessibility_label("greet.rs"),
        ]),
        section(cx, "Code diff", vec![
            caption(cx, "The rows the host gives it, with the old and the new line number and a column of marks. The language colors comments, strings, numbers and keywords."),
            framed(CodeDiff::new("code-diff", patch()).language(CodeLanguage::Rust).height(height_of(PATCH.len()))),
            caption(cx, "single_gutter(true): one column of numbers and no column of marks."),
            framed(
                CodeDiff::new("code-diff-single", patch())
                    .language(CodeLanguage::Rust)
                    .single_gutter(true)
                    .height(height_of(PATCH.len())),
            ),
            caption(cx, "focusable(true), with 60 rows in 152 px: a click or Tab focuses it, and the wheel or its scrollbar moves it. Only the rows in view are built."),
            framed(CodeDiff::new("code-diff-long", long).language(CodeLanguage::Rust).focusable(true).height(152.0)),
            caption(cx, "follow_tail(true): it stays at the last row as rows arrive, until the reader scrolls up."),
            row(vec![
                outline(format!("code-diff-tail={}", arrived + 1), "Append a row"),
                outline(format!("code-diff-tail={}", arrived + 20), "Append 20"),
                outline("code-diff-tail=6".to_owned(), "Reset"),
            ]),
            framed(CodeDiff::new("code-diff-tail", tail).single_gutter(true).follow_tail(true).height(height_of(5))),
            caption(cx, "DiffFileHeader over a diff with word_ranges: the changed words take a stronger tint, and a Folded row asks the host for the lines it hides."),
            Element::column(vec![
                DiffFileHeader::new("code-diff-file", "src/greeting.rs", palette)
                .stat(1, 1)
                .collapsed(header_collapsed)
                .render(),
                if header_collapsed {
                    Element::spacer().size(None, Some(0.0))
                } else {
                    CodeDiff::new("code-diff-fold", review.clone())
                        .word_ranges(words)
                        .palette(palette)
                        .language(CodeLanguage::Rust)
                        .height(height_of(review.len()))
                        .render()
                },
            ])
            .radius(RadiusRole::Surface.px())
            .clip_children()
            .size(Some(width), None),
            caption(cx, "No rows."),
            framed(CodeDiff::new("code-diff-empty", Vec::new()).height(height_of(2))),
        ]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_are_numbered_on_the_side_they_belong_to() {
        let built = rows(PATCH.map(|(mark, text)| (mark, text.to_owned())), 41);
        let numbers: Vec<_> = built.iter().map(|line| (line.old_line, line.new_line)).collect();
        assert_eq!(numbers[0], (Some(41), Some(41)));
        assert_eq!((numbers[1], numbers[8]), ((Some(42), None), (Some(49), None)));
        assert_eq!((numbers[9], numbers[10]), ((None, Some(42)), (None, Some(43))));
        assert_eq!(numbers[11], (Some(50), Some(44)));
    }
}
