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

## Wrapping rows

A row with `.wrap()` moves a child that does not fit its width to the next
line, as a list of tags or role pills does:

```rust
Element::row(tags).gap(6.0).wrap()                 // lines 6 px apart
Element::row(tags).gap(6.0).line_gap(10.0).wrap()  // lines 10 px apart
```

| Rule | Effect |
| --- | --- |
| Width | Each child keeps its natural width, `gap` after the one before it on its line. |
| Lines | A line is as tall as its tallest child, and the others stretch to it unless sized, as in any row. `line_gap` is the space between lines; it is `gap` until set. |
| Height | The row is as tall as its lines, so what follows it in a column moves down. |
| Too wide | A child wider than the row has a line of its own, at the row's width: a text wraps there. |
| Order | Tab and assistive technology take the children in the order given, line after line, and the pointer finds each where it is drawn. |

A wrapping row needs a width to wrap at. In a column it has the column's.
In another row, give it `.flex(1.0)`, `.shrink_to_fit()` or `.max_width(w)`:
like any child of a row, it otherwise takes its natural width, which is all
of its children on one line. Unlike other content, a wrapping row is
re-wrapped at its `max_width`, and its height counts the lines it makes
there.

Limits: `flex` and `shrink_to_fit` on its children change nothing, and a line
starts at the left; it is not centered or justified. Only `Element::row`
wraps: `wrap` on another element does nothing.

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

A heavier text names its face: `FontFamily::SansMedium` or
`FontFamily::SansBold`. They are real faces that the host provides and each is
measured as itself, never a regular face drawn thicker. `limn-egui` ships
Ubuntu Medium and Bold for them; a host that sets its own fonts without those
faces paints the regular one. `FontFamily::SansItalic` is the regular face
slanted by the host.

## Containers

| Constructor | Use |
| --- | --- |
| `Element::panel(child)` | A box around one child: background, border, radius, padding. |
| `Element::clickable(id, child)` | A panel that is also one click target. |
| `Element::stack(children)` | Lays its children one over another in the same box, as large as the largest unless sized. They are painted in the order given, the last on top, inside whatever clips the stack, and where two take the pointer the later one has it. For a mark over a picture, or two faces of one spot that give way to each other under the pointer. |
| `Element::overlay(base, floating)` | Paints `floating` over `base` without affecting its layout. |
| `ScrollArea::new(id, child)` | A vertically scrolling viewport. |
| `ScrollArea::uniform_list(id, count, row_height, build_row)` | A virtualized list: only visible rows are built. |
| `ScrollArea::uniform_list(…).expansion(index, content)` | What row `index` opens under itself, at the height it takes: it scrolls with the rows, the rows after it move down, and the list stays virtual. One row at a time; a `Collapse` as `content` opens and closes it on the spring. The keyboard walks from the row into it and on to the next row. |
| `Element::horizontal_scroll(id, child)` | A horizontally scrolling strip. |

`.clip_children()` keeps a container's descendants, and their click targets,
inside its bounds.

`.top_background_fade(color, height)` and `.bottom_background_fade(color, height)`
paint a colour behind a container's children that starts at that edge and is
gone `height` further in: a tint behind a page's header, or under the last
lines of a panel. Paint only, and never taller than the container.

### Sticky children

A child of a column inside a scroll area can stay in view while the rows
after it scroll, like a section header.

| Builder | Effect |
| --- | --- |
| `.sticky_top()` | Once scrolled past the top of the viewport, the child is drawn there until the next sticky child of the same column pushes it out, or its column ends. |
| `.sticky_top_inset(px)` | Pins it that far below the top of the viewport. |
| `.sticky_top_max_height(px)` | While pinned, its bottom edge follows the scroll until `px` of height is left. What no longer fits is clipped and fades into the child's `background`. |
| `.sticky_top_fade(px)` | While pinned, the rows passing under it fade out over `px` below its bottom edge, in the color of the scroll area's `top_fade`. It grows in as the child pins and shortens as the next sticky child arrives. Paint only: no hit region, no layout. Nothing is drawn in a scroll area without a `top_fade`. |

The pinned child paints above the content passing under it and above the
scroll area's own fades, and takes clicks where it is drawn; its passive
parts block the rows beneath. The column's height and the scroll range do
not change. Give the child an opaque `background`, or the rows show through.

Limits: one child is pinned per column at a time. `reveal` does not account
for a pinned child, so pass `reveal_with_margin` its height when a control
could land under it. Uniform lists have no sticky rows.

### Scrolling from the host

The tree keeps each scroll area's offset. The host moves it with a request
that is resolved at the next layout, once sizes are known.

| Method | Effect |
| --- | --- |
| `tree.scroll_to_start(id)`, `tree.scroll_to_end(id)` | Jump to either end of the scroll area `id`. |
| `tree.scroll_to_index(id, index)` | Keep one row of a uniform list in view. |
| `tree.center(control_id)` | Bring a button or a clickable to the middle of the scroll area that holds it, travelling there by that area's scroll motion, or at once under reduced motion. For content that follows a clock, such as the line being sung. Near either end of the content it goes as far as the area scrolls; nothing happens when no control has that ID, and focus stays where it was. |
| `tree.reveal(control_id)` | Bring a button, a clickable or a field into view wherever it is: every scroll area around it scrolls just enough to show it whole, 12 px clear of the edge. Rows need no fixed height. Nothing moves when it is already in view, and nothing happens when no control has that ID. Focus stays where it was. |
| `tree.reveal_with_margin(control_id, margin)` | The same with a margin of the host's own, for a viewport whose edge is covered by a fade or a fixed header. A margin with no room shrinks until the control fits. |
| `tree.reveal_at_start(id, margin)` | Scroll so a control or an anchor starts `margin` under the top of its scroll area, also when it is already in view, which `reveal` leaves alone: how a table of contents or a rail of turns jumps to a section. The end of the content stops it short. |
| `element.anchor(id)`, `tree.anchor_offset(id)` | Name a passive element, such as a message or a heading, as a place to scroll to: `reveal(id)` then brings it into view like a control, and a pinned `sticky_top` anchor is revealed where its column has it, so the view returns to the start of what it pins over. `anchor_offset(id)` is how far its top was below the top of its scroll area at the last layout, negative once it has scrolled past: the last anchor at or above a line of the viewport is the part in view. The element has to be laid out by a column of the scroll area. |

`reveal` finds a control by its ID, so give the thing to reveal one: a row
that is not otherwise interactive can be an `Element::clickable`. Rows of a
uniform list exist only while in view; use `scroll_to_index` for those.

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
