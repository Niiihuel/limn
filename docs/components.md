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
| `Kbd` | Passive keycaps for a shortcut such as `Ctrl+Shift+P`. |

Button variants: `Default`, `Secondary`, `Outline`, `Ghost`, `Subtle`,
`Raised`, `Solid`, `Link`, `Success`, `Warning`, `Destructive`. `Subtle` is
the navigation row and keeps its label at the left; the filled variants center
it.

## Inputs

| Component | Purpose | Reports |
| --- | --- | --- |
| `TextField` | Single-line text. Optional password masking. | `InputChanged`, `Submitted` |
| `TextArea` | Multi-line text. Enter adds a line; Ctrl+Enter submits. | `InputChanged`, `Submitted` |
| `SearchField` | A text field with a magnifier and an optional shortcut hint. | `InputChanged`, `Submitted` |
| `Switch` | An on/off control. | `Clicked` |
| `DatePicker` | An ISO date field with a navigable month calendar. | `InputChanged`, and `Clicked` for navigation and days |
| `TimePicker` | A 24-hour `HH:MM` field with step controls. | `InputChanged`, and `Clicked` for the steps |
| `DateTimePicker` | The two above combined, each keeping its own state. | As above |
| `Composer` | A message composition surface. The host owns the text and the actions around it. | The actions of the controls the host places in it |

## Navigation

| Component | Purpose |
| --- | --- |
| `Tabs` | A row of tabs with a selection that slides to the active one. |
| `SegmentedControl` | Tabs in one rounded track, for choosing one of a few options. |
| `TabStrip`, `TabItem` | Folder tabs for open resources. Each `TabItem` has an icon, a title that fades, a tooltip and a slot for the host's close button; the host gives every tab its width. The strip scrolls sideways, keeps the active tab in view and reports `Clicked` with the tab's ID. `.each_tab` wraps a tab in something the host owns, such as a context menu. `TabStrip::header` is the 32-pixel bar that holds a strip and its tools (`.side_inset`, `.gap`); `TabStrip::surface` and `TabStrip::align` build label-only or icon-only tabs. The palette's `surface` is the active tab and the body, `raised` the bar, `hover` an inactive tab under the pointer. |
| `Breadcrumb` | A path. Segments with an action are links; the last is the current resource. Middle segments collapse into a menu when space is short. |
| `Stepper` | The steps of a short form: completed, active and pending. |
| `Collapsible` | A header that shows or hides content beneath it. |
| `TreeRow`, `TreeGuides` | Rows and indentation guides for file and project trees. |

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
| `Marker` | A one-line event in a conversation or log, optionally as a separator. |
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
| `Tooltip` | A hint for a control, after a short hover or at once with keyboard focus. |
| `Popover` | A floating slot anchored to any element, for arbitrary content. |
| `DropdownMenu` | A menu of actions with headings, separators, shortcuts and submenus. |
| `DropdownList` | A list for choosing one option, with the current one marked. |
| `ContextMenu` | A menu opened by a secondary click on its child. |
| `Modal` | A centered dialog for forms, with a title, a scrolling body and a footer. |
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

## Code

| Component | Purpose |
| --- | --- |
| `CodeEditor` | An editor view over a host-owned text buffer: selection, folding, diagnostics, breakpoints, optional Vim mode. |
| `CodeDiff` | A virtualized, read-only diff. |

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
