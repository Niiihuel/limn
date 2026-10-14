# Markdown

Renders a bounded subset of Markdown as an ordinary element tree. Use it for
text that arrives as Markdown, such as a message, including while it is still
being written.

## Import

```rust
use limn::{Markdown, MarkdownLabels, MarkdownLimits};
```

`Markdown` has a `render` of its own, so `Component` is not needed.

## Usage

`render` returns a `MarkdownPreview`: `element` goes in the tree, and
`limited` says the source was cut. A link reports `UiAction::Clicked` with an
ID that starts with the link prefix.

```rust
fn view(state: &State, width: f32, palette: ComponentPalette) -> Element {
    Markdown::new(&state.message, palette)
        .width(width)
        .link_action_prefix("open:")
        .render()
        .element
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if let Some(link) = id.strip_prefix("open:") {
            // link is the block hash, the run index and the URL, joined by colons.
            if let Some(url) = link.splitn(3, ':').nth(2) {
                state.open_in_browser(url);
            }
        }
    }
}
```

The subset is headings of three levels, paragraphs, bulleted, numbered and
checked lists, quotes, fenced code and rules, with strong, emphasis, inline
code and web links. Tables, images, HTML and nested quotes stay text.
[Components](../components.md) has the full reference.

## Width

```rust
Markdown::new(source, palette).width(560.0).render()
```

`width` is the reading column, kept between 48 and 1,200. A width that is not
finite reads as 600.

## Text size

```rust
Markdown::new(source, palette).font_size(state.text_size).render()
```

`font_size` is the size of body text, 14 unless set. Headings, inline and
fenced code, list markers and their column, line heights, the space between
blocks and the note under a cut preview keep their proportion to it: at 21,
everything is half as large again. A drawn diagram keeps its own size; only
the words on its card follow. A size that is not finite and positive is
ignored, and the rest is kept between 8 and 96.

## Streaming

```rust
Markdown::new(&state.partial, palette).streaming(true).render()
```

With `streaming(true)` the source is still being written: an unclosed
delimiter at its end already styles its text, a web address there has no
action until it ends, and the blocks above the last one are reused from the
previous render.

## Links

```rust
Markdown::new(source, palette).link_action_prefix("open:").render()
```

`link_action_prefix` is the start of every link action; `markdown_link:`
unless set. The prefix is a `&'static str`.

`Markdown::web_link(value)` gives one address in the normalized form the
renderer would open, or `None` for one it refuses. `Markdown::link_icon(address)`
gives the glyph its link wears. `Markdown::web_links(source)` lists the
complete web addresses in a text with their ranges.

## Labels

```rust
let labels = MarkdownLabels {
    code_fallback: "CODE",
    truncated_note: "Long text: showing the first blocks only.",
    ..MarkdownLabels::DEFAULT
};
Markdown::new(source, palette).labels(labels).render()
```

`labels` replaces the renderer's own strings with the host's language:
`code_fallback` labels a code block with no language and `truncated_note` is
the line under a cut preview. The other fields label a drawn diagram and its
actions.

## Limits

```rust
Markdown::new(source, palette)
    .limits(MarkdownLimits { source_chars: 20_000, blocks: 3 })
    .render()
```

`limits` sets how much of the source is rendered: `source_chars` and `blocks`.
A limit of zero reads as one. When the source is cut, `limited` is true on the
preview.

## Colors

`colors(MarkdownColors)` replaces the colors derived from the palette.
`MarkdownColors::for_palette(palette)` is the default to start from.

## Literal text with links

```rust
Markdown::new(source, palette).plain_with_autolinks(21.0).render()
```

`plain_with_autolinks(line_height)` keeps the source literal, as one paragraph
with that line height: only complete HTTP(S) addresses are styled and become
links. Nothing is cached or cut.

## Diagrams

A closed `mermaid` fence that [Diagram](diagram.md) can read is drawn instead
of shown as code. `diagram_action_prefix(prefix)` gives every diagram an
action that switches between the drawing and its source: the prefix followed
by the diagram's ID as sixteen hexadecimal digits. The host keeps the IDs it
was asked to show as source and passes them to `diagram_sources`.

## Source by pages

```rust
let page = Markdown::source_page(source, 0);
page.render(width, palette)
```

`Markdown::source_page(source, start_byte)` returns a `MarkdownSourcePage`: a
page of the exact source as `text`, with `previous_byte` and `next_byte` for
the pages around it. Its `render(width, palette)` shows it as an element.

## Long lists

Hold `Markdown::begin_frame(scope, width)` while building a long list of
messages; see the notes on the cache in [Components](../components.md).

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` with `id` = the link prefix, then the block hash, the run index and the URL, joined by colons | A link is activated. |
| `UiAction::Clicked { id }` with `id` = the diagram prefix and sixteen hexadecimal digits | The switch on a diagram or on its source is activated. |
