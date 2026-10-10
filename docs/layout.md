# Layout

How to arrange and size elements. Layout in limn is rows and columns with
flexible children, which covers nearly every interface without a constraint
solver.

## Rows and columns

```rust
Element::row(vec![a, b, c]).gap(8.0)      // left to right
Element::column(vec![a, b, c]).gap(8.0)   // top to bottom
```

`gap` is the space between children. `padding` is the space inside an
element's edges, the same on all four sides.

Along its direction a container gives each child its natural size. Across its
direction it **stretches** each child to fill: children of a row are as tall
as the row, and children of a column are as wide as the column. A child with
an explicit size in that direction keeps it.

## Sizing a child

| Builder | Effect |
| --- | --- |
| `.size(Some(w), Some(h))` | Fixed width and height. Pass `None` to leave one natural. |
| `.min_size(w, h)` | Never smaller than this. |
| `.max_width(w)` | Caps the natural width. Content wider than this is not re-wrapped; pair with `.clip_children()` or a truncating text. |
| `.flex(n)` | Takes a share of the space left over, in proportion to `n`. |
| `.shrink_to_fit()` | Keeps its natural width while there is room, and gives it up when there is not. |

### Sharing spare space: `flex`

Children with `.flex(n)` divide whatever the fixed children leave:

```rust
Element::row(vec![
    sidebar.size(Some(220.0), None), // fixed
    content.flex(1.0),               // the rest
])
```

`Element::spacer().flex(1.0)` is an empty flexible child. Use it to push
things apart or to one side:

```rust
Element::row(vec![title, Element::spacer().flex(1.0), close_button])
```

### Giving up width: `shrink_to_fit`

A child with neither `flex` nor `shrink_to_fit` always takes its natural
width, even if that pushes its siblings out of the row. Mark it
`.shrink_to_fit()` to let it wrap or truncate instead:

```rust
Element::row(vec![
    rule.min_size(16.0, 0.0).flex(1.0),
    label.shrink_to_fit(),               // natural width, wraps if too long
    rule.min_size(16.0, 0.0).flex(1.0),
])
```

When several children can shrink, they give up width in order: the first one
shrinks down to its `min_size`, then the next, and only if that is still not
enough do they shrink below their minimums. Flexible siblings keep their
minimum widths throughout.

## Centering

There is no alignment property. Center a child by surrounding it with flexible
spacers.

```rust
// Horizontally: a row with a spacer on each side.
Element::row(vec![Element::spacer().flex(1.0), child, Element::spacer().flex(1.0)])

// Vertically: a column with a spacer above and below.
Element::column(vec![Element::spacer().flex(1.0), child, Element::spacer().flex(1.0)])
```

The vertical form matters inside rows. Because a row stretches its children,
an icon next to a taller label sits at the top unless it is wrapped this way.

`.center_content()` is narrower than its name suggests: it centers the text of
a text element or the label of a button horizontally within that element.

## Text

| Constructor | Behavior when it does not fit |
| --- | --- |
| `Element::text(s)` | Wraps onto more lines. |
| `Element::truncated_text(s)` | One line, ending in an ellipsis. |
| `Element::faded_text(s, fade_width)` | One line, fading out at the right edge. |
| `Element::formatted_text(spans)` | Wraps; each span has its own color, size and optional action. |

Text is monospace by default. UI text sets `.font_family(FontFamily::Sans)`
and a size with `.font_size(px)`.

## Containers

| Constructor | Use |
| --- | --- |
| `Element::panel(child)` | A box around one child: background, border, radius, padding. |
| `Element::clickable(id, child)` | A panel that is also one click target. |
| `Element::overlay(base, floating)` | Paints `floating` over `base` without affecting its layout. |
| `ScrollArea::new(id, child)` | A vertically scrolling viewport. |
| `ScrollArea::uniform_list(id, count, row_height, build_row)` | A virtualized list: only visible rows are built. |
| `Element::horizontal_scroll(id, child)` | A horizontally scrolling strip. |

`.clip_children()` keeps a container's descendants, and their click targets,
inside its bounds.

## The root

The root element is given the whole window. To keep a component at its own
size in a test or a small demo, put it in a row or column next to a flexible
spacer.

## Padding on buttons

A button's `padding` is horizontal. Its label and icon are always centered in
its full height, so a short button with deep padding does not lose its
content.

## See also

- [Concepts](concepts.md) for how elements become a frame.
- [Tokens](tokens.md) for the corner radii containers use.
