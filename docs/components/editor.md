# Editing tools

The parts an editing surface puts around its text: a find and replace bar
(`FindBar`), the path of the open file (`Breadcrumb::from_path`), a message
composer with a strip of metadata (`Composer`), its microphone
(`DictationButton`, `DictationWaveform`), and a status ink on a `Marker` and
a [Collapsible](collapsible.md). "Editor" is not one type; this page names
each one.

## Import

```rust
use limn::{
    Breadcrumb, Component, Composer, DictationButton, DictationState, DictationWaveform,
    FindBar, FindIds, FindLabels, FindOptions, Marker,
};
```

## Usage

`FindBar` is controlled: the host searches, and the bar shows the query, the
options and the count. It takes an ID, the query and the palette. Its fields
report `InputChanged` and its buttons `Clicked`, each with an ID derived from
the bar's, as `{id}:query`, `{id}:next` and so on.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    FindBar::new("find", &state.query, palette)
        .count_label(state.count_label())
        .has_matches(state.matches > 0)
        .has_current(state.current.is_some())
        .no_matches(!state.query.is_empty() && state.matches == 0)
        .render()
}

fn update(state: &mut State, action: &UiAction) {
    match action {
        UiAction::InputChanged { id, value } if id == "find:query" => state.search(value),
        UiAction::Clicked { id } => match id.as_str() {
            "find:next" => state.next_match(),
            "find:previous" => state.previous_match(),
            "find:dismiss" => state.close_find(),
            _ => {}
        },
        _ => {}
    }
}
```

`has_matches` enables previous, next and replace all. `has_current` says the
selection is one of the matches and enables replacing it. `no_matches` says
the search finished and found nothing. The host formats `count_label`, such
as `2 / 14`.

## Replace and options

```rust
FindBar::new("find", &state.query, palette)
    .replacement(Some(&state.replacement))
    .replace_open(state.replace_open)
    .options(FindOptions { case: state.case, regex: state.regex, ..FindOptions::default() })
    .render()
```

`replace_open(true)` shows the replace row, whose field reports
`InputChanged` with `{id}:replacement`. `FindOptions` says which of the four
toggles show as on: `case`, `word`, `regex` and `in_selection`. Each toggle
reports a click and the host flips its own value. The bar takes at most 520
of `width`; below 480, or with the replace row open, the options and the
count take a row of their own.

## Errors, ink and strings

```rust
FindBar::new("find", &state.query, palette)
    .error(state.regex_error.as_deref())
    .ink(danger)
    .labels(FindLabels { query: "Search the file".into(), ..FindLabels::default() })
    .render()
```

`error` adds a last row saying why the query could not be searched. `ink` is
the host's failure color, for that row and for the count when `no_matches`
is set. `FindLabels` holds every string the bar shows: the placeholders, the
tooltips and the shortcuts named in them; the default is English. The bar
shows the shortcuts and the host handles them. `ids(FindIds { .. })` uses IDs
of the host's own in place of the derived ones, `replace_icons` changes the
two replace glyphs, and `surface` takes a second palette for the bar under
the controls.

## Floating the find bar

```rust
FindBar::new("find", &state.query, palette).attach(editor)
```

`attach(anchor)` floats the bar under the end of the anchor. It takes neither
the pointer nor the focus from the document, so the host closes it.
`FindBar::height(width, replace_open, error)` is the height the bar lays
out, so a host can keep what it covers in view.

## A file's path

```rust
Breadcrumb::from_path("file-path", "crates/limn/src/find_bar.rs", palette)
    .dirty(state.unsaved)
    .render()
```

The path of the open file, one segment per folder and the file last. See
[Breadcrumb](breadcrumb.md) for its links and for a symbol after the file.

## Composer

```rust
Composer::new("composer", &state.draft, "Describe a task", width, palette)
    .pill_controls(add_button, send_button)
    .auto_grow(28.0, 120.0)
    .header_strip(strip, strip_palette)
    .render()
```

`Composer` is a card around a multi-line editor. It takes the editor's ID,
the text, a placeholder, a width and the palette, and the host owns the text
through `InputChanged` for the editor's ID. `pill_controls` puts the editor
above a row with one element at its start and one at its end; the host builds
both and handles their actions. `auto_grow` grows the editor with its
wrapped lines between two heights, and `editor_height` fixes it instead.
`header_strip` joins a strip of metadata to the top of the card, set in from
its sides and filled with that palette's `surface`. Build the strip's content
`Composer::header_strip_width(width)` wide.

### Controls beside a growing editor

```rust
Composer::new("composer", &state.draft, "Message #general", width, palette)
    .pill_controls(add_button, emoji_button)
    .inline_controls(true)
    .auto_grow(32.0, 284.0)
    .editor_padding(6.0)
    .card_corner_radii(CornerRadii::bottom(8.0))
    .render()
```

`inline_controls(true)` puts the editor between the two controls. Without
`auto_grow` the row is 28 tall and the editor one line, for a compact
surface the host swaps for the full-width editor once a draft is entered.
With `auto_grow` the row grows with the editor, and each control stays
centered beside its first line, `auto_grow`'s least height, as a chat's
composer keeps them while a message gets longer. `controls_at_last_line(true)`
keeps them beside its last line instead, at the bottom of the growing row. A disabled composer's
placeholder wraps there too, so a long reason not to write stays whole.

`editor_padding` is the room between the editor's edge and its text: 8 in
the card, 6 in the pill and 5 with inline controls unless set.
`card_corner_radii` rounds the card's corners one by one in place of
`card_radius`, such as square top corners under a strip of the host's joined
above it.

### Files from another application

Add `.drop_target()` to opt the enabled editor into external file drops.
`UiAction::FilesDropped` uses the editor's existing ID as its `target` and
preserves the list of paths. It does not submit the draft, change its text or
move focus. Disabled composers accept no drops, even with this option set.
Hover/drop resolution stays inside a modal while it is open.

The host validates the complete list, permissions and current account before
opening files in its own service. The component and event bridge never read
their contents. A native backend needs a pointer position to resolve a target;
the operating system's last reported position can lag during external drags.
The Editor gallery demonstrates the action by retaining only the file count.

### Disabled

```rust
Composer::new("composer", &state.draft, "Describe a task", width, palette).enabled(!state.sending).render()
```

A disabled composer shows its draft, or its placeholder, as muted text in the
editor's place and takes no pointer, focus or typing. The elements the host
passes in, such as the controls, the footer and the attachments, are the
host's to disable.

## Dictation

```rust
DictationButton::new("dictate", state.dictation, palette).render()
DictationWaveform::new("levels", &state.levels, palette).width(48.0).render()
```

`DictationButton` is the composer's round microphone. Its state is a
`DictationState`: `Idle` offers to start a session, `Starting` and
`Transcribing` show a spinner, and `Listening` shows a stop square. Every
state reports `Clicked` with the one ID, so the host toggles on it; the host
owns the session and the transcript. `live_palette` colors a live session:
its `surface` fills the circle and its `text` draws the mark. `size` takes a
`ControlSize`, and `enabled(false)` is a passive microphone in any state.

`DictationWaveform` draws the levels the host sampled, from 0 to 1 and oldest
first, with the newest at the trailing edge. Silence, or no level at all, is
a row of dots. It is 96 wide by default and reports nothing.

## Status ink

```rust
Marker::new("request", "Model request failed", palette).ink(danger).render()
```

A `Marker` is a one-line event in a conversation or a log. `ink` paints its
label, its icon and its spinner in a status color from the host, in place of
the palette's muted. `Collapsible` takes the same `ink`.

## Actions

| Action | When |
| --- | --- |
| `UiAction::InputChanged { id, value }` | The find bar's query or replacement changes, or the composer's text. |
| `UiAction::Clicked { id }` | A button of the find bar is activated: an option, the replace toggle, previous, next, replace, replace all or close. |
| `UiAction::Clicked { id }` | The dictation button is activated, in any state. |

## Accessibility

The composer's editor is a multiline field named by its placeholder, with the
draft as its value; disabled, it is still there as disabled. The find bar's
buttons are named by their `FindLabels`: the case, word, regular expression
and selection options say whether they are toggled, the replace toggle whether
it is expanded, and a button that cannot be used is disabled. The count is a
status and the error an alert.
