use super::{caption, section, Cx, Section};
use limn::{Component, Element, Markdown, MarkdownLabels, MarkdownLimits, Text};

const COMPLETE: &str = "# Review summary\n\n**The session** was fixed in `src/main.rs`; details are in the [guide](https://example.com/docs) and at https://github.com/rust-lang/rust.\n\n## Steps\n\n1. Build the project\n2. Run the tests\n   - [x] Unit tests\n   - [ ] Integration tests\n- A standalone item with text long enough to wrap onto two lines within the reading column.\n\n> A quote keeps its bar and background even when it spans several lines of the column.\n\n```rust\nfn main() {\n    println!(\"hello\");\n}\n```\n\n```\nno language\n```\n\n---\n\n### Closing\n\nText *with emphasis* and a forced  \nline break.";

const STREAMING: &str = "The result arrives in parts.\n\n- First item already **complete**\n- Second item with an unfinished link https://example.com/doc\n\nThe last line is still **open and contains `code";

const SIZED: &str = "## Release notes\n\nThe **build** passes in `ci.yml`; see the [log](https://example.com/log).\n\n1. Update the lockfile\n2. Tag the release\n\n> Ship it on Friday.\n\n```sh\ncargo publish\n```";

const CHAT: &str = "**Bold**, *italic*, ***both***, __underlined__, ~~struck through~~ and __**marks inside marks**__.\n-# Subtext: small and quiet, as a footnote under a message.";

const PLAIN: &str = "**This is not bold**: check https://github.com/rust-lang/rust and https://example.com/docs, not `src/main.rs`.";

fn link(cx: &Cx, written: &str) -> Element {
    let palette = cx.palette;
    let address = Markdown::web_link(written);
    Element::row(vec![
        Element::icon(Markdown::link_icon(written)).foreground(palette.muted),
        Text::new(written, palette).mono().muted().render(),
        Text::new(address.as_deref().unwrap_or("does not open"), palette).mono().truncate().render().flex(1.0),
    ])
    .gap(8.0)
    .size(Some(cx.width.min(560.0)), None)
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let width = cx.width.min(560.0);
    let labels = MarkdownLabels {
        code_fallback: "CODE",
        truncated_note: "Long text: preview limited to the first blocks.",
        ..MarkdownLabels::DEFAULT
    };
    let page = Markdown::source_page(COMPLETE, 0);
    vec![
        section(cx, "Complete Markdown", vec![
            caption(cx, "Headings, lists, quote, code, rule, and links, with labels supplied by the host"),
            Markdown::new(COMPLETE, palette)
                .width(width)
                .labels(labels)
                .link_action_prefix("markdown-complete:")
                .render()
                .element,
        ]),
        section(cx, "Chat marks", vec![
            caption(cx, "__underline__, ~~strikethrough~~, ***bold italic*** and -# subtext lines, as chat clients write them"),
            Markdown::new(CHAT, palette)
                .width(width)
                .link_action_prefix("markdown-chat:")
                .render()
                .element,
        ]),
        section(cx, "Text size", vec![
            caption(cx, ".font_size(12.0): headings, code, list markers and line heights keep their proportion to the body"),
            Markdown::new(SIZED, palette)
                .width(width)
                .font_size(12.0)
                .link_action_prefix("markdown-small:")
                .render()
                .element,
            caption(cx, "14 px, the default"),
            Markdown::new(SIZED, palette)
                .width(width)
                .link_action_prefix("markdown-default:")
                .render()
                .element,
            caption(cx, ".font_size(18.0)"),
            Markdown::new(SIZED, palette)
                .width(width)
                .font_size(18.0)
                .link_action_prefix("markdown-large:")
                .render()
                .element,
        ]),
        section(cx, "Streaming", vec![
            caption(cx, ".streaming(true): the unfinished tail is styled, but its address does not open yet"),
            Markdown::new(STREAMING, palette)
                .width(width)
                .streaming(true)
                .link_action_prefix("markdown-streaming:")
                .render()
                .element,
        ]),
        section(cx, "Limits", vec![
            caption(cx, ".limits set to 3 blocks: the rest is available in the source view"),
            Markdown::new(COMPLETE, palette)
                .width(width)
                .labels(labels)
                .limits(MarkdownLimits { source_chars: 20_000, blocks: 3 })
                .link_action_prefix("markdown-limited:")
                .render()
                .element,
            caption(cx, "220 px column"),
            Markdown::new(COMPLETE, palette)
                .width(220.0)
                .labels(labels)
                .limits(MarkdownLimits { source_chars: 20_000, blocks: 6 })
                .link_action_prefix("markdown-narrow:")
                .render()
                .element
                .size(Some(220.0), None),
        ]),
        section(cx, "Plain text with links", vec![
            caption(cx, ".plain_with_autolinks(21.0): only complete addresses become links"),
            Markdown::new(PLAIN, palette)
                .plain_with_autolinks(21.0)
                .link_action_prefix("markdown-plain:")
                .render()
                .element
                .size(Some(width), None),
        ]),
        section(cx, "Addresses for the host", vec![
            caption(cx, "Markdown::web_link normalizes or rejects; Markdown::link_icon supplies the glyph"),
            link(cx, "https://GitHub.com/limn"),
            link(cx, "https://example.com/docs"),
            link(cx, "file:///etc/passwd"),
        ]),
        section(cx, "Paginated source", vec![
            caption(cx, "Markdown::source_page(..).render(width, palette)"),
            page.render(width, palette),
        ]),
    ]
}
