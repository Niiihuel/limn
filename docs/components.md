# Components

Every component in the library, by category, with what it is for. For the
full list of builder methods on any of them, run `cargo doc -p limn --open`.
To see them running, open the [gallery](gallery.md).

## How every component works

```rust
let element = Component::new(/* required arguments */, palette)
    .option(value)   // optional builder methods
    .render();       // an Element, ready for the tree
```

- Required arguments come first: an ID for anything interactive, then its
  content, then the `ComponentPalette`.
- Components are **controlled**. You pass the current value each frame and
  handle the action the component reports. See
  [Concepts](concepts.md#who-owns-state).
- Unless noted, an interactive component reports `UiAction::Clicked { id }`.

## Actions

| Component | Purpose |
| --- | --- |
| `Button` | A labelled action. Eleven variants, several sizes down to the 22 px `ButtonSize::Mini`, optional leading and trailing icons, disabled and loading states, optional tooltip. `dispatch_as` reports another action while the button keeps its own ID. `ink` draws the label in a status color from the host, `gap` sets the icon-to-label space, and `radius` takes a `RadiusRole`. |
| `IconButton` | A button whose icon is mandatory. Without a label it is square. |
| `TooltipSpec` | The hint of a `Button` or `IconButton`, passed with `tooltip_with`: title, description, shortcut, placement, palette, width, and `suppressed` to hide it while a menu is open. A disabled button keeps its hint. |
| `ActiveHover` | How an active button without a fill hovers: `Lift` lightens the selected fill, `Palette` keeps the palette's hover and selected colors. Set with `active_hover`. |
| `ButtonGroup` | Rendered buttons 6 px apart: a row, or a full-width column with `stacked`. `align_end` pushes them to the far edge, `leading` keeps one element at the near edge (first in the column), and `reverse_when_stacked` puts the primary action first among the actions in the column. |
| `Link` | A text action: link ink, underlined under the pointer and keyboard focus, no hover fill. |
| `Kbd` | Passive keycaps for a shortcut such as `Ctrl+Shift+P`. `.caption(text)` says what the keys do in a small muted caption after them, and `.alternative(keys)` adds another key after a slash (Enter / Tab): the hint in the footer of a palette or a suggestion list, 20 px high. |

Button variants: `Default`, `Secondary`, `Outline`, `Ghost`, `Subtle`,
`Raised`, `Solid`, `Link`, `Success`, `Warning`, `Destructive`. `Subtle` is
the navigation row and keeps its label at the left; the filled variants center
it.

## Inputs

| Component | Purpose | Reports |
| --- | --- | --- |
| `TextField` | Single-line text. Optional password masking; `.monospace(true)` for a path, an identifier or a number. | `InputChanged`, `Submitted` |
| `TextArea` | Multi-line text. Enter adds a line; Ctrl+Enter submits. | `InputChanged`, `Submitted` |
| `SearchField` | A text field with a magnifier and an optional shortcut hint. | `InputChanged`, `Submitted` |
| `Switch` | An on/off control. | `Clicked` |
| `DatePicker` | An ISO date field with a navigable month calendar. | `InputChanged`, and `Clicked` for navigation and days |
| `TimePicker` | A 24-hour `HH:MM` field with step controls. | `InputChanged`, and `Clicked` for the steps |
| `DateTimePicker` | The two above combined, each keeping its own state. | As above |
| `Select` | A button showing the current choice that opens a `DropdownList` of the others. Controlled: the host passes the label, the options and whether it is open. `.sizing(SelectorSizing)` gives the trigger the profile's `min_width` and the list its `preferred_width`; `.width` and `.menu_width` override them, and without either the list is as wide as the trigger. `.menu_palette`, `.placement` (under the far edge by default), `.icon`, `.enabled`. | `Clicked` with its own ID, an option's action, or the dismiss ID |
| `FormField`, `FormFieldSizing` | A label over a control the host owns, which fills the field's width. `.hint` adds a muted line under the control, or between the label and the control with `.hint_above(true)`; `.error(text, ink)` adds a last line in the host's color. `FormFieldSizing` sets the label's role, whether it is muted, the least height of its line, the role of hints and error, and the gap. | Nothing of its own |
| `Composer` | A message composition surface. The host owns the text and the actions around it. `.header_strip(element, palette)` joins a 32 px strip of metadata to the top of the card, set in 8 from its sides and filled with that palette's surface; build its content `Composer::header_strip_width(width)` wide. | The actions of the controls the host places in it |

## Navigation

| Component | Purpose |
| --- | --- |
| `Tabs` | A row of tabs with a selection that slides to the active one. `.fill_width(total)` gives every tab the same width so the track is `total` wide. |
| `SegmentedControl` | Tabs in one rounded track, for choosing one of a few options. |
| `TabStrip`, `TabItem` | Folder tabs for open resources. Each `TabItem` has an icon, a title that fades, a tooltip and a slot for the host's close button; the host gives every tab its width. The strip scrolls sideways, keeps the active tab in view and reports `Clicked` with the tab's ID. `.each_tab` wraps a tab in something the host owns, such as a context menu. `TabStrip::header` is the 32-pixel bar that holds a strip and its tools (`.side_inset`, `.gap`); `TabStrip::surface` and `TabStrip::align` build label-only or icon-only tabs. The palette's `surface` is the active tab and the body, `raised` the bar, `hover` an inactive tab under the pointer. |
| `Breadcrumb` | A path. Segments with an action are links; the last is the current resource. Middle segments collapse into a menu when space is short. `Breadcrumb::from_path(id, path, palette)` splits a file path into its folders and the file with its icon; `.dirty(true)` marks unsaved changes, `.segment_action(|segment| ..)` lets the host make each segment a link, and `.item(..)` adds a segment after the file. |
| `Stepper` | The steps of a short form: completed, active and pending. |
| `Collapsible` | A header that shows or hides content beneath it. `.ink(color)` paints its icon, detail and chevron in a status color from the host. |
| `TreeRow`, `TreeGuides` | Rows and indentation guides for file and project trees. A row that folds takes `.count(n)`, small muted figures at its trailing edge, and `.disclosure(open)`, a muted chevron in a slot of its own; both sit before the `.trailing` element. |
| `Pager`, `PagerSizing` | Two arrows around a muted position label such as "3 / 12": `Pager::new(label, palette)` with `.previous(id, enabled)` and `.next(id, enabled)`. The host writes the label and says which ends can be taken; a disabled arrow is passive and keeps its hint. `.icons` changes the glyphs (up and down for a sequence that runs down the page), `.tooltips` takes a `TooltipSpec` for each arrow, `.label_width` holds the label in a fixed slot so the arrows do not move, and `.label_tooltip(id, spec)` spells the position out. `PagerSizing::DEFAULT` sets the arrows 4 from the label. |
| `MenuBar`, `MenuBarMenu` | The application menus across the top of a window: `MenuBar::new(menus, open_id, palette)` with one `MenuBarMenu::new(id, label, width, dismiss_id, entries)` per menu. The host owns which menu is open and every trigger's width; a trigger reports a click on its ID, fills with `palette.selected` while open, and its menu focuses the first enabled item. `.menu_palette` colors the menus, `.compact(id, dismiss_id, tooltip)` folds the bar into one button with the menus as submenus (`MenuBarMenu::compact_id` names each row), and `.width()` says how wide the bar is: 24 high, triggers 2 apart, 32 when compact. |
| `StatusBar`, `StatusBarItem`, `StatusBarItemSizing` | The bar at the foot of a window, 21 high with cells 4 apart. `StatusBarItem::new(id, width, palette)` takes `.icon`, `.label` and `.count` in the order they are shown, plus `.tooltip_with`, `.dispatch_as`, `.menu(entries, open, dismiss_id, menu_palette)` (opens upward, 256 wide), `.passive()` for information without an action, `.centered()`, `.ink(color)` for the glyphs and `.hosted(element)` for a cell the host wraps itself, such as a popover's anchor. `StatusBar::new(leading, trailing, palette).width(px)` always shows the leading items and keeps the trailing ones that fit, highest `.priority` first, in the order given; `.overflow(item)` is shown only while something was left out, `.trailing_budget(px)` replaces the bar's own reservation, and `.top_hairline(true)` adds a rule and the palette's surface for a bar that closes a panel. The host passes every width: the bar measures no text. Profiles: `LABEL` (glyph 12 wide, 4 from its label) and `COUNTS` (3 apart). |
| `ListRow` | One entry of a list under a single hover surface: an `.icon` centered in the row or a caller-owned `.leading` square, a title, a muted `.subtitle` (`.subtitle_mono` for a command or a path), `.meta` text or `.meta_element`s at the end of the title's line, a `.trailing` element and a muted `.trailing_icon` such as a chevron. It hovers to the palette's `hover` and presses to its `selected`, with no fill at rest unless `.filled()`. `.enabled(false)` renders a passive row with a muted title and no hit region; `.title_ink` and `.icon_ink` take a color the host owns. `.title_fade(true)` fades a title that does not fit instead of cutting it. `.hover_with(id)` keeps the row hovered while the pointer is over a trailing control, and `.trailing_action(id, control)` adds a control with a hit target of its own under one hover surface shared with the row; the hint then covers the row's own target alone. `.radius(RadiusRole)` rounds the fill and the hover (`Control` unless set). `.width` fixes the width and clips the content to it; `.tooltip_with` adds a hint. |
| `RowSizing` | The density of a `ListRow`: the two heights, padding, gaps, the icon, leading and trailing-icon columns, an optional `leading_height` that boxes the icon or the leading element from the top, the font sizes, and an optional box per line of copy. `RowSizing::COMPACT` (28, or 40 with a subtitle) and `DENSE` (24 / 36) are the presets; a `None` height follows the content. |
| `Card`, `CardDensity` | A raised surface that sets one group of content into the page: `Card::new(content, palette)` fills with the palette's `raised` under a hairline of its `border`. `.density` pads by 8 (`Compact`), 10 (`Regular`, the default) or 12 (`Roomy`); `.border(false)` drops the hairline; `.radius(RadiusRole)` rounds it (`Card` unless set); `.width` fixes the width. It is passive and follows its content's height. The host picks the fill by passing a palette whose `raised` is the color it wants. |
| `OutputBlock` | Read-only monospaced output of a command or a tool: `OutputBlock::new(id, text, palette)`. One clipped 17 px line per row, in a scroll area that builds only the rows in view; the text is cut to 48 KiB, 600 lines and 500 characters per line before it is shaped. `.max_height` bounds the lines before they scroll (200 unless set), `.ink(color)` paints them in a color the host owns, such as an error stream, `.truncation_note(text)` adds a muted line when text was cut, and `.width` fixes the width. The fill is the palette's `surface`. |
| `CodeBlock`, `CodeBlockSizing` | The labelled card that holds code inside prose: `CodeBlock::new(text, palette)` wraps monospaced code on the palette's `raised` fill inside a hairline. `.label(text)` adds a muted line above it, in the case the host wrote; `.spans(spans)` paints text the host's highlighter already colored; `.width` fixes the width. `CodeBlockSizing::CHAT` (a 10 px monospaced label over 12 px code) and `DOCUMENT` (an 11 px sans label over 13 px code) are the presets. |

## Feedback and status

| Component | Purpose |
| --- | --- |
| `Alert` | A persistent inline notice in one of five tones. |
| `Toast` | A brief card in the corner of the window. The host decides when it leaves. |
| `Progress` | A thin bar for work whose completed share is known. |
| `Spinner` | An indicator for work of unknown duration. |
| `Skeleton` | A pulsing placeholder in the shape of content that is loading. |
| `SkeletonGroup` | Placeholder rows for a list that is loading: `SkeletonGroup::rows(id, palette, count)`, with an optional leading icon square. `into_rows()` hands the rows to a host that places each one itself. |
| `Empty` | The state of a page, pane or result list with nothing in it: an icon, a title, a reason and one next step. It has no frame of its own and sits on whatever surface it is given. `.framed()` sets it in a card of its own: the palette's surface, a hairline border and the card radius. |
| `Notice` | A left-aligned inline notice standing where content would be: an optional muted icon, a title, a reason, a caller-owned `.action(element)` under the copy and a `.trailing(element)` after it. `.framed()` fills it with the palette's surface and the card radius, with no border. `.ink(color)` colors the title, for an error. `NoticeSizing::REGULAR` and `COMPACT` set its density; a profile with a `height` cuts each line of copy instead of wrapping it. |
| `Marker` | A one-line event in a conversation or log, optionally as a separator. `.ink(color)` paints it in a status color from the host, such as a failure. |
| `TextShimmer` | Text with a moving highlight, for work in progress. |
| `Chip` | A passive pill for a count or a label. `.outline()` draws the border only, with no fill. |
| `Avatar` | An account photo, or the initial of a name when there is none. |

Alert tones, shared by `Toast`: `Neutral`, `Info`, `Success`, `Warning`,
`Error`. The tone colors the icon. An `Error` alert also tints its
title and description; the other tones keep ordinary ink, so a page of
notices stays calm. An alert takes an optional action, placed at its top
right, and an optional dismiss button.

## Overlays

Floating surfaces. Each takes the element it is anchored to or laid over, and
an `open` flag the host controls. See [Animation](animation.md#what-animates)
for how they enter and leave.

| Component | Purpose |
| --- | --- |
| `Tooltip` | A hint for a control, after a short hover or at once with keyboard focus. `Tooltip::for_path` names a file over the directory that holds it. |
| `Popover` | A floating slot anchored to any element, for arbitrary content. |
| `DropdownMenu` | A menu of actions with headings, separators, shortcuts and submenus. `.header(element)` stays above the options and `.footer(element)` under them, below a hairline, while they scroll. Entries beyond items: `DropdownMenuEntry::Message(text)` (a muted sentence), `Loading { id, rows }` (skeleton rows) and `Error { id, text }` (a failure in a scroll area of 104). |
| `DropdownMenuItem` | One action. `.enabled(bool)` or `.disabled()`, `.tooltip_with(TooltipSpec)` for a hint. With `.description(text)` it is a two-line row at `SelectorSizing::detail_row_height` (40): icon, label over a muted detail, and a check column filled when `selected`; `.detail_mono(true)` sets the detail in monospace, `.status(text)` puts short text at the end of the detail line (`.status_width(px)` fixes its slot) and `.chevron()` ends the row with a chevron instead of the check, for a row that opens another view. |
| `DropdownMenuSection` | The heading of a group the user folds: `::new(id, label)` with `.icon`, `.count(n)` and `.collapsed(bool)`; it converts into a `DropdownMenuEntry` and reports `id` when activated. The host leaves the group's items out while it is collapsed. `.width(px)` fixes its width. |
| `MenuHeader` | The fixed head of a menu, passed to `DropdownMenu::header`: `.title(text)` or `.back(id, label)`, one square `.trailing(element)` tool, and `.search(id, query, placeholder)` for a compact search field under the row. `.width(px)` fixes its width. 28 high; the search field paints the palette's `raised`. |
| `MenuItem` | A menu row on its own, for a list the host lays out: icon, label and a shortcut or `.trailing` text, 26 high. `.description(text)` makes it the same two-line row as `DropdownMenuItem`, with `.detail_mono`, `.status`, `.status_width`, `.chevron` and `.selected(bool)`. `.enabled(false)` renders it passive. `.tooltip_with(TooltipSpec)` hints the row; `.trailing_action(id, icon, TooltipSpec)` adds a second 24 px action at the end, reached by Tab and not by the arrow keys, and `.trailing_dispatch_as(action)` makes it report another ID. |
| `PopoverPanel` | The content of a `Popover` that is more than a menu: `.header(element)`, `.body(element)` and `.footer(element)` stacked 4 px apart with hairlines between them. The body takes the height it needs and gives way first in a short window; `.body_max_height(px)` caps it, for a body that is a scroll area. The popover keeps placement, width and surface. |
| `SuggestionList` | Completions floating over an input without taking focus: `::new(anchor, id, items, selected_index, open, dismiss_id, palette)`. A `SuggestionItem` is `::new(id, label)` with `.icon`, `.description` and `.section(label)` to start a section. The host moves `selected_index`; rows are 42 high, at most 7 show, and only those in view are built. `.title`, `.empty(text)` for when there are no items, `.hints(elements)` (they stack under 300 px), `.note(text)`, `.width(available)` (the list is 160 to 352 wide), `.placement` and `.animation_ms(open, close)`. |
| `DropdownList` | A list for choosing one option, with the current one marked. |
| `ContextMenu` | A menu opened by a secondary click on its child. |
| `Modal` | A centered dialog for forms, with a title, a scrolling body and a footer. `Modal::fit_width(viewport_width, max)` is the width that keeps the window's margin clear, and `Modal::body_width(modal_width)` the width a body can count on: inside the padding and the rail reserved while it scrolls. |
| `AlertDialog` | A centered confirmation for consequential actions. |
| `ImageViewer` | A full-window image gallery. |

`Modal`, `AlertDialog`, `ImageViewer` and `Toast::layer` wrap the window's
root element, since they cover the whole window.

## Text

| Component | Purpose |
| --- | --- |
| `Text` | Passive text in a role of the type scale and a color of the palette: `.role`, `.muted`, `.ink` for a semantic color, `.mono`, `.truncate`, `.fade`, `.line_height`, and `.tooltip` to show the whole value of truncated text, wrapped. `Text::eyebrow` is the small monospaced label above a group. It takes no size in pixels. |

`TextRole` is the scale, in logical pixels:

| Role | Size | Family |
| --- | --- | --- |
| `Display` | 18 | Sans |
| `Heading` | 16 | Sans |
| `Body` | 14 | Sans |
| `Label` | 13 | Sans |
| `Caption` | 12 | Sans |
| `Small` | 11 | Sans |
| `Micro` | 10 | Sans |
| `Code` | 14 | Monospace |

## Layout and grouping

| Component | Purpose |
| --- | --- |
| `Separator` | A one-pixel rule in the border color. `Separator::horizontal` and `Separator::vertical` fill their axis unless given a `.length`; `.margin` adds space across the rule, `.centered_in` centers a vertical rule in a taller bar, and `.ink` takes a color the host owns. Margin and centering are off by default. |
| `ScrollArea` | A scrolling viewport, or a virtualized list of uniform rows. |
| `PageColumn` | A scrolling page whose content is a centered column: `PageColumn::new(scroll_id, available, palette)`. `.max_width` (960), `.min_width`, `.gutter(narrow, wide, breakpoint)` (14 and 28 at 640), `.gap` (10) and `.reserve_rail`, which keeps a scrollbar rail's width out of the column before the page overflows. Read `.content_width()` to size the content, then pass it with `.children`. It fills its parent. |
| `PageHeader`, `PageHeaderSizing` | The heading of a page: a title, a `.description`, and a `.leading` and a `.trailing` element centered on it. `.stacked` moves the trailing element under the copy, `.truncate` cuts both lines instead of wrapping them. The profile holds the two font sizes, the three gaps and an optional fixed height; `PageHeaderSizing::DEFAULT` is a settings page. |
| `SectionHeader`, `SectionSizing` | The heading of a section: a title, a `.count` chip right after it, a muted `.detail` and a `.trailing` element at the far end, and a `.disclosure` chevron before it. With `.action` the whole row is a button. `.width` fixes its width. Profiles: `TITLE` (a page section), `LIST` (a list heading) and `LABEL` (a quiet label in a side panel, hung from the top inset). |
| `PanelHeader`, `HeaderSizing` | The header row of a panel: a title cut to one line that takes the width left, `.trailing` tools after it, a `.back(id, tooltip)` arrow before it and a `.dismiss(id, tooltip)` close button at the end, both with hints that open downward. `.leading` puts an element in place of the title, such as tabs, a breadcrumb or a field. `.width` fixes the width. It paints no fill. Profiles: `SIDEBAR` (26 high), `PANE` (32, inset 4), `CARD` (28, inset 2, a body-sized title) and `POPOVER` (24); `center_tools: false` hangs the tools from the top inset instead of centering them. A rule between two groups of tools is a `Separator` with a length in the vector. |
| `SidebarPanel` | The body of a sidebar: its blocks in a column with an inset and a gap of 6, on the palette's `surface`. `.size(width, height)` fixes it. |
| `CardGrid` | Cards of equal width in rows of `.columns` (2), or in one column below the breakpoint: `CardGrid::new(width, breakpoint)`. Build each card at `.item_width()`, then pass them with `.items`. The gap is 8 and a short last row keeps its columns. |
| `Attachment` | A file or image card, with an optional remove action. |
| `PreferencesGroup`, `ActionRow` | A card of settings rows separated by hairlines, under a small muted label. Without a title it is just the card. A row takes an icon, a subtitle, a `.trailing` control such as a switch or a button, or a read-only `.value` in muted text, and can itself be an action. `.wrap_title` wraps the title instead of cutting it, `.align_top` starts the icon and the copy at the top, `.icon_muted` quiets the icon for a note, and `.stacked` moves the trailing element under the copy, except one with a fixed width of 64 or less, which stays beside it. |
| `ActionRowSizing` | The density of an `ActionRow`: the gap, the gap between title and subtitle, the icon column and the two minimum heights. `ActionRowSizing::DEFAULT` is the preferences row; pass another profile with `.sizing`. |

Any element can be placed without a component:

| Method | Effect |
| --- | --- |
| `element.centered_y()` | Centers it vertically in its parent's height. The wrapper takes over its width and flex, so it keeps its place in a row. |
| `element.centered_x()` | Centers it horizontally in its parent's width. |
| `element.centered()` | Centers it on both axes. |
| `element.clip_to(width, height)` | Holds it in a fixed box that does not flex and hides what overflows, while it keeps its own width. |

## Data

| Component | Purpose |
| --- | --- |
| `BarChart`, `ColumnChart` | Horizontal and vertical bar charts. |
| `HeatmapChart` | A grid of cells colored by value. |
| `ChartLegend` | A legend for any of the charts. |
| `DiffStat` | The lines a change adds and removes, as `+N` and `−M`. `.inks(added, removed)` takes the host's two colors; without them both counts are plain text. |

## Code

| Component | Purpose |
| --- | --- |
| `CodeEditor` | An editor view over a host-owned text buffer: selection, folding, diagnostics, breakpoints, optional Vim mode. |
| `CodeDiff` | A virtualized, read-only diff. |
| `FindBar`, `FindOptions`, `FindIds`, `FindLabels` | Find and replace for a `CodeEditor`, at most 520 wide. Controlled: the host passes the query, `.replacement`, `.replace_open`, `.options`, `.count_label`, `.has_matches`, `.has_current`, `.no_matches` and `.error`, and answers the actions of `FindIds` (derived from the bar's ID, or the host's own through `.ids`). `.labels` takes every string, `.ink` the failure color, `.replace_icons` the two replace glyphs and `.surface` a second palette for the bar under the controls. Below 480 wide, or with replace open, the options take their own row. `render()` gives the bar; `.attach(anchor)` floats it under the anchor without taking the pointer or the focus. `FindBar::height(width, replace_open, error)` is the height it lays out. |
| `CodeLanguage` | The languages with a grammar. `from_extension`, `from_path` and `from_code_fence` choose one; `comment_markers_for(path)` gives the markers that comment out a line, falling back on the file's name (`#` for shell, YAML, Nix, Ruby, Makefile and Dockerfile) when the language has none. Needs the `syntax` feature. |

## Icons

The standard iconography is [Bootstrap Icons](https://icons.getbootstrap.com/).
`Icon` names what an icon means, such as `Icon::Trash` or `Icon::Search`, and
the backend maps each to a Bootstrap icon. Every built-in icon is a Bootstrap
icon; `Icon::Brain`, which marks reasoning in progress, is drawn as
Bootstrap's light bulb because the set has no brain.

For artwork of your own, such as a brand mark, use `Icon::Custom(svg)` with
the source of an SVG drawn with `currentColor`. It is fitted to the icon's
square without distortion and tinted like any other icon:

```rust
const LOGO: &str = include_str!("../assets/logo.svg");
Element::icon(Icon::Custom(LOGO)).foreground(palette.text)
```

`icon.filled()` returns the solid form of an icon, for the chosen item of a
navigation list; an icon without one is returned unchanged.

`Element::icon(Icon::...)` paints a semantic icon at 16 px. `.large_icon(px)`
draws a standalone icon larger, for empty states and thumbnails.
`file_icon(path)`, `folder_icon(name, expanded)` and `language_icon(name)`
return the artwork for a file type.

## See also

- [Gallery and hot reload](gallery.md) to see and try each component.
- [Contributing a component](contributing.md) to add one.
