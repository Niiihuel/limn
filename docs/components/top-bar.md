# TopBar

The bar across the top of a window, laid out as a music app's: your
controls at the start, such as back and forward; a round Home button and a
wide search in the middle of the window; your controls at the end, such as
notifications, settings and the account. Use it as a window's first row.

## Import

```rust
use limn::{Component, TopBar};
```

## Usage

The bar is controlled: pass the query, the page shown and the window's width
each frame, and handle what it reports. It takes an ID, the query, a
placeholder and the palette.

```rust
fn view(state: &State, width: f32, palette: ComponentPalette) -> Element {
    TopBar::new("nav", &state.query, "What do you want to play?", palette)
        .width(width)
        .home_active(state.page == Page::Home)
        .browse_active(state.page == Page::Browse)
        .leading(vec![back, forward])
        .trailing(vec![bell, settings, avatar])
        .render()
}

fn update(state: &mut State, tree: &mut UiTree, action: &UiAction) {
    match action {
        UiAction::InputChanged { id, value } if id == "nav:search" => state.query = value.clone(),
        UiAction::Submitted { id, value } if id == "nav:search" => state.search(value),
        UiAction::Clicked { id } => match id.as_str() {
            "nav:home" => state.page = Page::Home,
            "nav:browse" => state.page = Page::Browse,
            "nav:search" => state.page = Page::Search,
            "nav:search:clear" => {
                state.query.clear();
                tree.set_input_value("nav:search", "");
                tree.focus("nav:search");
            }
            _ => {}
        },
        _ => {}
    }
}
```

The bar paints no ground of its own: put it on the window's color. It is
64 px tall, with 8 px around and between its controls.

## The search

The search is a pill 48 px tall on the palette's `raised` fill, lighter
under the pointer and outlined in the palette's `text` while it has the
focus. A magnifier starts it; a rule and a browse button end it. While the
query is not empty, a clear button stands before the rule. The search
reports as a [SearchField](search-field.md) with the ID `{id}:search` would:
`InputChanged`, `Submitted`, and `Clicked` with `{id}:search:clear`. The
field keeps the text typed in it, so after clearing the query give it the
new text with `UiTree::set_input_value`.

## A panel under the search

```rust
let bar = TopBar::new("nav", &state.query, "Search", palette);
let bar = if state.recent_open {
    bar.search_popover("recent:dismiss", recent_searches(state, palette))
} else {
    bar
};
```

`search_popover(dismiss_id, panel)` floats `panel` under the search from its
start edge, without taking the keyboard from the field, for the searches made
lately while it is focused and empty, or suggestions while it is typed in.
Pass it only while it should show. A press outside it or Escape reports
`Clicked` with `dismiss_id`. The round search of a narrow bar floats nothing.

## Width

```rust
TopBar::new("nav", &state.query, "Search", palette).width(window.width)
```

`width` is the width the bar is laid out at, normally the window's. The
search takes half of it, kept between 350 and 546 px. When that would be
400 px or less, at a window of 800 or less, the search is a round button
with a magnifier that reports `Clicked` with `{id}:search`, for the host to
open its search page. Without a width the search is 546 wide.

## Centering

Home and the search are centered in the bar, whatever the controls at each
end take. When centered they would meet the controls at either end, they
follow the controls at the start instead, 8 px after them, and the search
narrows to the room left before the controls at the end.

## Window controls

```rust
TopBar::new("nav", &state.query, "Search", palette).caption(72.0, 0.0)
```

`caption(left, right)` keeps that many logical pixels free at the start and
at the end for the window's own buttons, as macOS draws them at the start
and Windows at the end of a window without a title bar. The search stays
centered in the whole bar. A value that is not finite is ignored and a
negative one is zero.

## Home and Browse

```rust
TopBar::new("nav", &state.query, "Search", palette)
    .home_active(true)
    .home_label("Inicio")
    .browse_label("Explorar")
```

Home is a 48 px circle on the `raised` fill, lighter under the pointer. The
browse button is a 40 px circle at the end of the search. Each draws its
icon outlined and muted, and solid in the `text` color while
`home_active` or `browse_active` says its page is shown: `Icon::HomeFill`
and `Icon::GridFill`. `home_label` and `browse_label` set the hint and the
name of each, "Home" and "Browse" unless set.

## Your controls

`leading` and `trailing` take your elements, 8 px apart, each centered on
the bar's height. They are yours: the bar does not size, style or disable
them.

## A page that scrolls under it

A page's own header can fill in as the page scrolls under it, as a music
app's does under its top bar:

```rust
let header = Element::row(vec![title])
    .size(None, Some(64.0))
    .background(page_color)
    .sticky_top_background_fade_in(10.0, 80.0);
ScrollArea::new("page", Element::column(vec![header, banner, tracks])).render()
```

`sticky_top_background_fade_in(from, to)` pins the child at the top of its
scroll area as `sticky_top` does, and shows none of its `background` until
the page has scrolled `from` px, all of it by `to`: 10 to 80 for a page, 10
to 350 under a tall banner. The tree follows the offset as it lays out, so
you pass the same element every frame. See
[Layout](../layout.md#sticky-children).

## Disabled

```rust
TopBar::new("nav", &state.query, "Search", palette).enabled(state.online)
```

`disabled()` or `enabled(false)` draws Home and the search passive: they keep
their place and their query and take no pointer or focus. Your controls are
left as you give them.

## Keyboard

| Key | Effect |
| --- | --- |
| Tab | Moves through your leading controls, Home, the search, its clear and browse buttons, then your trailing controls |
| Enter | In the search, reports `Submitted` with the query |
| Enter, Space | On Home, the browse, clear or round search button, reports its click |

## Motion

The fills of Home, the search and its buttons cross-fade under the pointer
over `hover_ms`. A header's `sticky_top_background_fade_in` follows the
scroll, not a clock.

## Actions

| Action | When |
| --- | --- |
| `UiAction::InputChanged { id, value }` with `"{id}:search"` | The query changes. |
| `UiAction::Submitted { id, value }` with `"{id}:search"` | Enter is pressed in the search. |
| `UiAction::Clicked { id }` with `"{id}:home"` | Home is activated. |
| `UiAction::Clicked { id }` with `"{id}:browse"` | The browse button is activated. |
| `UiAction::Clicked { id }` with `"{id}:search:clear"` | The clear button is activated. |
| `UiAction::Clicked { id }` with `"{id}:search"` | The round search button of a narrow bar is activated. |

## Limits

- Icons are drawn at the library's 16 px in every control, where a music
  app draws 24 px.
- The search's size and whether it is a round button follow the width you
  pass, not the room your controls leave; that room only narrows it.
- The bar is one row: it does not wrap, and controls wider than the window
  overlap.
