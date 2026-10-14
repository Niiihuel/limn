# Tabs

A row of tabs for switching between views, with a selection that slides to
the active one. `TabStrip` is the other kind: folder tabs for open resources,
such as files, that scroll sideways and open onto the body beneath them.

## Import

```rust
use limn::{Component, Icon, Tab, Tabs, TabsVariant};
```

For folder tabs:

```rust
use limn::{file_icon, Component, TabItem, TabStrip};
```

## Usage

Tabs are controlled: mark the active tab each frame and change it when a tab
reports a click. A `Tab` takes an action ID, a label and a width, which is
never less than 40.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    let tabs = vec![
        Tab::new("view=files", "Files", 120.0).icon(Icon::Folder).active(state.view == "files"),
        Tab::new("view=changes", "Changes", 120.0).icon(Icon::GitBranch).active(state.view == "changes"),
    ];
    Tabs::new(tabs, palette).id("views").render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if let Some(view) = id.strip_prefix("view=") {
            state.view = view.to_owned();
        }
    }
}
```

`id` names the strip for its selection animation. Set it when tabs are
inserted, removed or reordered; without it the strip is named after its
first tab.

## Underlined

```rust
Tabs::new(tabs, palette)
    .variant(TabsVariant::Underlined)
    .accent(theme.accent_text())
    .gap(16.0)
    .render()
```

`variant` chooses how the active tab is marked. `TabsVariant::Segmented`, the
default, fills it on a rounded track. `TabsVariant::Underlined` draws text tabs
with no track: a 2 px underline under the active tab, over a 1 px hairline in
the palette's `border`. The underline slides as the selection does. Both
variants have the same size, so one can replace the other without moving
anything around them. In a column the hairline runs the full width it is
given; in a row it is as wide as the tabs.

`accent` colors the active tab's label and the underline. Without it they take
the palette's `text`. Pass a color that reads as text, such as a theme's
`accent_text()`.

`gap` sets the space between two tabs, 2 by default, never below 0. The
underline spans its tab's whole width, so underlined tabs usually want a wider
gap and a width close to the label's. Set it before `fill_width`, which reads
it.

## Fill a width

```rust
Tabs::new(tabs, palette).fill_width(520.0).render()
```

`fill_width(total)` gives every tab the same width so the track is `total`
wide, whatever width each tab was built with.

## Height

`size` takes a `ControlSize` and `height` a number, never less than 24. The
default is the height of `ControlSize::Dense`.

## Disabled tabs

```rust
Tab::new("view=history", "History", 120.0).disabled()
```

A disabled tab is drawn muted and cannot be chosen. Marked active, it takes
no selection fill and no underline.

## Folder tabs

```rust
let items = vec![
    TabItem::new("file=0", "file-extent-0", file_icon("main.rs"), "main.rs", 112.0)
        .tooltip("crates/limn/src/main.rs")
        .active(state.file == 0)
        .close(close_button),
    TabItem::new("file=1", "file-extent-1", file_icon("lib.rs"), "lib.rs", 112.0)
        .active(state.file == 1),
];
let strip = TabStrip::new("files", items, palette).render();
TabStrip::header(strip, tools, width, palette).render()
```

A `TabItem` takes its ID, an extent ID, an icon, a title and a width. The
host gives every tab its width; the title fades when it does not fit. The
extent ID names the whole tab, close slot included, which is what the strip
scrolls into view. `TabStrip::new` takes a scroll ID that keeps the scroll
position between frames. The strip scrolls sideways when its tabs do not fit
and keeps the active tab in view. A click on a tab reports
`UiAction::Clicked` with the tab's ID.

`TabStrip::header(leading, trailing, width, palette)` is the 32 px bar that
holds the strip and, in `trailing`, the tools that stay outside the scrolling
part. `side_inset` sets the space at each edge of the bar, 6 by default, and
`gap` the space between the tabs and the tools, 8 by default.

The palette's `surface` paints the active tab, and must match the body under
the strip; `raised` paints the bar and `hover` an inactive tab under the
pointer.

## Close slot and title ink

`close` puts an element of the host's in the tab's trailing slot, 24 px
square. It should be 20 px square so its hover stays inside the tab's rounded
corner. The slot takes any element, so it can also hold a mark of unsaved
changes. Without one the slot stays empty.

```rust
TabItem::new("file=0", "file-extent-0", icon, "lib.rs", 112.0).title_ink(warning)
```

`title_ink` colors the title alone, active or not, for a state the host marks
by color, such as a file's status in version control. `tooltip` sets the
hint's text, which is the title by default, and `tooltip_placement` where it
opens, above the tab by default.

## A menu on each tab

```rust
TabStrip::new("files", items, palette).each_tab(|index, tab| {
    ContextMenu::new(format!("file-menu-{index}"), tab, entries(index), palette).render()
})
```

`each_tab` wraps each rendered tab, by position, in something the host owns.
The wrapper must keep the tab's size.

## Label-only and single tabs

```rust
let tab = TabStrip::surface("panel=output", label, active, 96.0, palette);
let tabs = TabStrip::align(Element::row(vec![tab]).size(Some(96.0), Some(TabStrip::TAB_HEIGHT)));
```

`TabStrip::surface` is the clickable surface of one tab around content of
your own, 24 px tall, for tabs that are only a label or an icon.
`TabStrip::align` sets a row of tabs on the bottom of the bar. For a group
that shows a single resource, `TabItem::render(palette)` gives the tab alone.

## Disabled folder tabs

```rust
TabItem::new("file=2", "file-extent-2", icon, "build.log", 112.0).enabled(false)
```

A disabled folder tab keeps its place, its tooltip and its close slot, with
icon and title in muted ink, and takes no pointer or focus. The close control
is the host's to leave out.

## Keyboard

With the focus on a tab, in either kind; a disabled tab is stepped over:

| Key | Effect |
| --- | --- |
| Left, Right | Chooses the tab before or after, wrapping at the ends |
| Home, End | Chooses the first or the last tab |

## Motion

The selection of `Tabs`, the fill or the underline, slides to the active tab
over the `move_ms` motion token. `animation_ms` takes another duration.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` | A tab is chosen by pointer or keyboard. The ID is the tab's. |

## Accessibility

A `TabItem` of a `TabStrip` is a tab named by its title, not by its tooltip nor
its close control, and says whether it is the selected one, the active tab. A
disabled tab is there and cannot be used. A tab built with `TabStrip::surface`
is named by the text in its content.

A `Tab` of `Tabs` is a tab named by its label, and says whether it is the
active one. A disabled tab is there and cannot be used.

## Limits

- The underline is 2 px and the hairline 1 px. They are not tokens.
- The underline spans the tab's width, which the host gives; tabs are not sized
  to their labels.
