# Accordion

Several titled items one under another, each of which opens to show its
content. Use it for a list of questions, or for the groups of a settings
page, where a reader wants one or two parts at a time.

## Import

```rust
use limn::{Accordion, AccordionItem, Component};
```

## Usage

An accordion is controlled: pass whether each item is open every frame, and
change your own state when a header reports a click. It takes an ID, the
items and the palette; an item takes the ID its header reports, a title and
its content.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    Accordion::new(
        "faq",
        vec![
            AccordionItem::new("shipping", "How long does shipping take?", shipping(state))
                .open(state.open.as_deref() == Some("shipping")),
            AccordionItem::new("returns", "Can I return an order?", returns(state))
                .open(state.open.as_deref() == Some("returns")),
        ],
        palette,
    )
    .render()
}
```

The content is in the window only while its item is open.

## One open at a time

Keep the ID of the open item, `open: Option<String>`. A click on another
item replaces it; a click on the open one closes it.

```rust
fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if matches!(id.as_str(), "shipping" | "returns") {
            state.open = (state.open.as_deref() != Some(id.as_str())).then(|| id.clone());
        }
    }
}
```

## Several open

Keep the IDs of the open items, `open: HashSet<String>`, and pass
`.open(state.open.contains("shipping"))` for each. A click flips only the
item it was on.

```rust
fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if matches!(id.as_str(), "shipping" | "returns") && !state.open.remove(id) {
            state.open.insert(id.clone());
        }
    }
}
```

The accordion does not know which rule you follow. Any other rule, such as
one item that is always open, is a few lines of the same kind.

## Bordered

```rust
Accordion::new("settings", items, palette).bordered(true)
```

| | Drawn as |
| --- | --- |
| Plain, the default | A list with a hairline between the items and nothing around them. |
| `.bordered(true)` | One surface: the palette's `surface` inside its `border`, with the radius of a card, and the same hairlines between the items. |

## Icon and description

```rust
AccordionItem::new("alerts", "Notifications", content)
    .icon(Icon::Bell)
    .description("3 channels on")
```

`.icon` puts an icon before the title, in the palette's muted color.
`.description` puts muted text beside the title, on its line. In a narrow
header the title is cut with an ellipsis first, then the description; the
chevron keeps its place at the trailing edge.

## Sizes

```rust
Accordion::new("settings", items, palette).size(ControlSize::Compact)
```

A header is 8 pixels higher than a control of its size, and its title has the
text size of a button of that size.

| Size | Header | Title |
| --- | --- | --- |
| `Dense` | 32 | 12 |
| `Compact` | 36 | 12 |
| `Regular`, the default | 40 | 13 |
| `Spacious` | 44 | 14 |

## Content

The content is any element: text, a form, another accordion. It is set 12
pixels in from each side, as the title is, with 12 pixels under it.

## Accordion or Collapsible

| Use | When |
| --- | --- |
| `Accordion` | Several parts of one thing, of equal weight, in a list: questions, the groups of a settings page. The items share their size, their hairlines and, bordered, one outline. |
| [`Collapsible`](collapsible.md) | One header alone in a flow of other content: the steps of a tool call, a list of changed files in a conversation. It is compact, fills as one block while open, and has what a row of activity needs: a loading spinner, a thinking state, a status ink and a footer that closes it. |

Both open and close their content with the same [`Collapse`](../animation.md#content-in-the-flow-that-leaves).

## Disabled

```rust
AccordionItem::new("owner", "Owner", content).open(true).disabled()
AccordionItem::new("plan", "Plan", content).enabled(state.can_edit)

Accordion::new("settings", items, palette).disabled()
```

A disabled item cannot be opened or closed. Its header keeps its place and
its size with a muted title, takes no click and no focus, and reports
nothing. The item stays open or closed as you say, and what is inside an
open one can still be used. A disabled accordion disables every item,
whatever each one says; `.enabled(true)` leaves each as it says it is.

## Keyboard

| Key | Effect |
| --- | --- |
| Tab, Shift+Tab | Moves to the next or the previous header, and through the controls inside an open item. The content of a closed item is not a stop. |
| Enter, Space | Reports the focused header's `Clicked`. |

## Motion

An item's content opens from zero height and fades in, and closes in place
while its last paint fades; the items below move with it. It is the
library's `Collapse`: a spring that takes one and a half
`tokens().motion.move_ms`. Content that is closing takes no pointer and no
focus from the moment the item is closed. The chevron does not turn: it
points right on a closed item and down on an open one, and changes at once.
The fill under the pointer cross-fades over `tokens().motion.hover_ms`.
Under reduced motion everything stands at once.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` | A header is activated by pointer, Enter or Space, whether its item is open or closed. `id` is that item's. |

## Accessibility

Each header is told to the platform as a button named by its title, with
whether it is expanded, and with its description if it has one. A disabled
header is told as disabled. `.accessibility_label("Settings")` on the
accordion tells it as a group of that name, known by the accordion's ID;
without one the platform is told of the headers alone.

## Limits

- The description stands beside the title, not under it; a header is one line.
- A title does not wrap: one that does not fit is cut with an ellipsis.
- The IDs of the items must be unique in the window. Each item derives
  `{id}:body` from its own for the slot its content opens in.
