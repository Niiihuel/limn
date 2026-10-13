# Kbd

Passive keycaps for a keyboard shortcut such as `Ctrl+Shift+P`, with an
optional caption that says what the keys do: the hints in the footer of a
palette or a suggestion list.

## Import

```rust
use limn::{Component, Kbd};
```

## Usage

A `Kbd` needs the keys and the palette. It draws one outlined keycap for each
key and takes no pointer or focus.

```rust
fn view(palette: ComponentPalette) -> Element {
    Kbd::new("Ctrl+Shift+P", palette).render()
}
```

## Notation

```rust
Kbd::new("Ctrl+K Ctrl+C", palette).render()
Kbd::new("Ctrl++", palette).render()
Kbd::new("↑↓", palette).render()
```

A plus joins the keys of one combination and a space separates the chords of
a sequence. A literal plus is written `Ctrl++`. The arrows `↑`, `↓`, `←` and
`→` are drawn as icons, so they do not depend on the font. Empty keys draw
nothing.

## Caption

```rust
Kbd::new("Esc", palette).caption("Close").render()
```

`caption` says what the keys do, in small muted text after them.

## Alternative

```rust
Kbd::new("Enter", palette).alternative("Tab").caption("Accept").render()
```

`alternative` adds another key that does the same, after a slash: Enter /
Tab. It can be called more than once.

## Menu accelerator

```rust
Kbd::new("Ctrl+K Ctrl+C", palette).menu().render()
```

`menu` draws the combination as plain muted text with no keycaps, the way a
menu row shows its accelerator.

## Width budget

```rust
Kbd::new("Ctrl+Shift+M", palette).max_width(80.0).render()
```

With `max_width`, a hint whose measured width does not fit is left out whole. It is
never cut, because a truncated combination would read as another shortcut.

The budget covers the complete hint: all alternatives, slash separators,
gaps and the caption. They disappear together, without leaving a caption
or slash on its own. A narrow parent row also omits the complete hint.
`menu` applies to every alternative.
