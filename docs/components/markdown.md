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
checked lists, quotes, subtext lines, fenced code and rules, with strong,
emphasis, bold italic, underline, strikethrough, inline code and web links.
Tables, images, HTML and nested quotes stay text.
[Components](../components.md) has the full reference.

## Marks

| Written | Drawn |
| --- | --- |
| `**strong**` | The bold face (`FontFamily::SansBold`), in `colors.strong` |
| `*emphasis*` | The italic face, in `colors.emphasis` |
| `***both***` | `FontFamily::SansBoldItalic`, in `colors.strong` |
| `__underline__` | A line under the text, as chat clients write it; CommonMark reads it as strong |
| `~~strikethrough~~` | A line through the text |
| `` `code` `` | Monospace over `colors.code_fill` |
| `-# subtext` | A line of its own, 0.8125 times the body's size, in `colors.muted` |

Marks nest: `__**this**__` and `**__this__**` are both bold and
underlined, and `~~*this*~~` is struck italic. A mark closes at the first
copy of its delimiter, so it never holds itself, and neither opens nor
closes inside a `<…>` without spaces, such as an autolink or a chat
client's `<:emoji_name:id>`. `\~` writes a tilde, as
`\*` and `\_` write theirs. A web address stops before a `|` at its end,
which closes a table cell or a `||spoiler||` around it.

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
ignored, and the rest is kept between 8 and 96. In a column too narrow for a
large size, a list gives up its nesting first and then the width of its marker
column, so an item's text always starts inside the column.

`code_font_size(size)` sets fenced code independently of the body, in logical
pixels. Without it, code keeps its existing proportion to the body. Invalid
numbers and nonpositive sizes are ignored; valid sizes stay within 1 to 96.
`inline_code_size(size)` sets inline `code` in body text, in logical pixels,
such as 13 in a 16 px body; in a heading or a subtext line it keeps that
proportion to the text around it.
`code_word_wrap(false)` preserves each code source line in a horizontal
viewport; prose continues to wrap. Both settings take effect in cached and
streaming previews. A standalone `CodeBlock` offers the same controls as
`font_size(size)` and `word_wrap(bool)`; its label keeps its original size.
Give the preview a stable `.id(message_id)` to preserve code viewport offsets
when rebuilding it each frame. Each code fence then receives its own ID within
that message, including identical fences. Standalone blocks take `.id(block_id)`.
Anonymous instances have independent temporary identities.

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
the line under a cut preview. `copy_code` is the hint of a code block's copy
icon. The other fields label a drawn diagram and its actions.

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

## Highlighted code

````rust
let source = "```rust\nfn main() {\n    println!(\"hi\"); // greet\n}\n```";
Markdown::new(source, palette).render()
````

A fence in a language limn's line lexer knows, such as `rust`, `js`, `ts`,
`py`, `json`, `toml`, `css`, `html` or `md`, has its comments, strings,
keywords and numbers colored, with or without the `syntax` feature. The inks
follow the palette, dark or light, not `MarkdownColors`. A fence with no
language or another one stays plain, and so does code longer than 64 KiB or
2,000 lines, so a long message is not held up. The full list of labels is in
[Components](../components.md#highlighted-code).

```rust
Markdown::new(source, palette).code_syntax_colors(CodeSyntaxColors {
    text: Color::rgb(0x19, 0x19, 0x19),
    string: Color::rgb(0x1E, 0x9E, 0x3C),
    function: Some(Color::rgb(0x00, 0x73, 0xE6)),
    interpolation: Some(Color::rgb(0xCD, 0x20, 0x54)),
    ..CodeSyntaxColors::for_palette(palette)
})
```

`code_syntax_colors` replaces those inks: `text` (names, operators,
punctuation, and plain code), `keyword`, `string`, `number` and `comment`.
`function`, when set, colors a name followed by `(`, such as `print`;
`interpolation`, when set, colors the braces of a Python f-string's `{…}` and a
JavaScript template's `${…}` and draws what they hold in `text`. Both are
`None` in `CodeSyntaxColors::for_palette`, which leaves those in `text` and in
`string`. An interpolation is found only in a string that opens on its line.

## Code blocks as a chat app draws them

````rust
let palette = ComponentPalette { raised: Color::WHITE, border: Color::rgb(0xE5, 0xE5, 0xE5), ..palette };
Markdown::new("```python\nprint(\"hi\")\n```", palette)
    .code_label(false)
    .code_fit_content(true)
    .code_copy_action_prefix("copy_code:")
    .code_font_size(13.0)
    .render()
````

A code block is a card on the palette's `raised` fill with a 1 px `border`, the
card radius (8) and 12 px of padding; pass a palette with other `raised` and
`border` to change them. `code_label(false)` drops the language line above the
code. `code_fit_content(true)` makes each block as wide as its code and padding,
up to the reading width, where longer code wraps. `code_copy_action_prefix(prefix)`
puts a muted copy icon at the end of each block, hinted with `labels.copy_code`
and centered against a one-line block; its action is the prefix followed by the
code exactly, which the host puts on the clipboard. Inline `code` is drawn in
`colors.code_ink` over `colors.code_fill`.

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
| `UiAction::Clicked { id }` with `id` = the code copy prefix and the code | A code block's copy icon is activated. |
