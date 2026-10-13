# Diagram

A flowchart or a sequence diagram drawn by the engine from Mermaid source.
`Diagram` is an enum over the two kinds, `Flowchart` and `SequenceDiagram`,
for a host that is handed a source without knowing which it is.

## Import

```rust
use limn::{Component, Diagram};
```

`Flowchart`, `SequenceDiagram` and `DiagramViewer` are imported the same way
when a host uses them directly.

## Usage

`Diagram::parse` reads the source's header and returns `Diagram::Flowchart`
for a `flowchart` or a `graph` and `Diagram::Sequence` for a
`sequenceDiagram`. Anything it cannot draw is a `DiagramError`, for the host
to show the source instead.

```rust
fn view(source: &str, palette: ComponentPalette) -> Element {
    match Diagram::parse(source, palette) {
        Ok(diagram) => Element::panel(diagram.scrollable("build-diagram"))
            .padding(12.0)
            .background(palette.raised),
        Err(error) => Text::new(error.to_string(), palette).muted().render(),
    }
}
```

A diagram reports no actions. `render` is the diagram at its natural size;
`scrollable(id)` is described below.

## Scrolling

```rust
Diagram::parse(source, palette)?.scrollable("build-diagram")
```

`scrollable(id)` puts the diagram in a horizontal viewport that fills the
width it is given and pans when the diagram is wider, with a fade at a cut
edge. It returns the element, so it takes the place of `render`.

## Backdrop

```rust
Diagram::parse(source, palette)?.backdrop(palette.surface).scrollable("build-diagram")
```

`backdrop` names the fill the diagram sits on, which is painted behind the
labels on its lines. It is the palette's `raised` unless set, as inside a
card.

## Flowcharts

```rust
let chart = Flowchart::parse("flowchart LR\n    A[Source] --> B{Changed?}\n    B -->|Yes| C([Build])", palette)?;
```

`Flowchart::parse` reads one kind alone and has the same `backdrop`,
`scrollable` and `render`. The gallery's samples draw the directions `TD` and
`LR`; the nodes `A[rect]`, `A(rounded)`, `A([stadium])`, `A{decision}` and
`A((circle))`; the connectors `-->`, `-.->` and `==>`, with `-->|text|` or
`-- text -->` labels; and `subgraph … end` as a titled group.

## Sequence diagrams

```rust
let diagram = SequenceDiagram::parse("sequenceDiagram\n    Editor->>Preview: Changes the text\n    Preview-->>Editor: Redraws", palette)?;
```

`SequenceDiagram::parse` does the same for a `sequenceDiagram`. The gallery's
samples draw `participant` and `actor` with `as` aliases; messages with the
arrows `->>`, `-->>`, `-)`, `-x` and `--x`, to another participant or to the
sender itself; activations by `+` and `-` after the arrow; `Note right of` and
`Note over`; `loop`, `alt`/`else`, `opt`, `par`/`and`, `critical`/`option`,
`break` and `rect` blocks; `autonumber` and `title`.

[Components](../components.md) lists the whole subset each kind reads and its
limits.

## Limits

`Diagram::parse` refuses a source over 64 KiB. A source outside what its kind
supports is an error and is never guessed at.

## In Markdown

A closed `mermaid` fence in [Markdown](markdown.md) that `Diagram` can read is
drawn. One that is still open, or that it cannot read, stays a code block.

## A viewer over the window

```rust
DiagramViewer::new(base, "Build pipeline", "diagram-close", palette, diagram).render()
```

`DiagramViewer::new(base, title, close_id, palette, chart)` takes a `Diagram`,
a `Flowchart` or a `SequenceDiagram` and shows it at its natural size in a
panel that covers the window, over `base`. The host opens it by rendering it
and closes it when `close_id` is reported. `close_title` sets the hint of the
close control and `viewport(size)` the size the panel is fitted to, 900 by 620
unless set.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` with the viewer's `close_id` | A `DiagramViewer` is closed by its close control, Escape or the backdrop. |
