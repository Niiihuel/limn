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
| `Button` | A labelled action. Eleven variants, several sizes down to the 22 px `ButtonSize::Mini`, optional leading and trailing icons, disabled and loading states, optional tooltip. `dispatch_as` reports another action while the button keeps its own ID. `ink` draws the label in a status color from the host, `gap` sets the icon-to-label space, and `radius` takes a `RadiusRole`; `round` makes an icon button a circle and a labelled one a pill, whatever the size. `sizing` takes a `ButtonSizing` profile (height, label size, padding, icon gap) in place of a named size, for a density the host owns; `IconButton` has `sizing`, `radius` and `round` too. |
| `IconButton` | A button whose icon is mandatory. Without a label it is square. |
| `TooltipSpec` | The hint of a `Button` or `IconButton`, passed with `tooltip_with`: title, description, shortcut, placement, palette, width, and `suppressed` to hide it while a menu is open. A disabled button keeps its hint. |
| `ActiveHover` | How an active button without a fill hovers: `Lift` lightens the selected fill, `Palette` keeps the palette's hover and selected colors. Set with `active_hover`. |
| `ButtonGroup` | Rendered buttons 6 px apart: a row, or a full-width column with `stacked`. `align_end` pushes them to the far edge, `leading` keeps one element at the near edge (first in the column), and `reverse_when_stacked` puts the primary action first among the actions in the column. `sizing` takes a `ButtonGroupSizing` with one gap for the row and one for the column; `ButtonGroupSizing::DEFAULT` is 6 and 6. |
| `Link` | A text action: link ink, underlined under the pointer and keyboard focus, no hover fill. |
| `Kbd` | Passive keycaps for a shortcut such as `Ctrl+Shift+P`. `.caption(text)` says what the keys do in a small muted caption after them, and `.alternative(keys)` adds another key after a slash (Enter / Tab): the hint in the footer of a palette or a suggestion list, 20 px high. |

Button variants: `Default`, `Secondary`, `Outline`, `Ghost`, `Subtle`,
`Raised`, `Solid`, `Link`, `Success`, `Warning`, `Destructive`. `Subtle` is
the navigation row and keeps its label at the left; the filled variants center
it.

## Inputs

| Component | Purpose | Reports |
| --- | --- | --- |
| `TextField` | Single-line text. Optional password masking; `.monospace(true)` for a path, an identifier or a number; `.height(px)` for a form whose inputs are taller than a `ControlSize` (never shorter than it). | `InputChanged`, `Submitted` |
| `TextArea` | Multi-line text. Enter adds a line; Ctrl+Enter submits. `.radius(RadiusRole)` rounds the box with another token. | `InputChanged`, `Submitted` |
| `SearchField` | A text field with a magnifier and an optional shortcut hint. | `InputChanged`, `Submitted` |
| `Switch` | An on/off control. | `Clicked` |
| `DatePicker` | An ISO date field with a navigable month calendar. | `InputChanged`, and `Clicked` for navigation and days |
| `TimePicker` | A 24-hour `HH:MM` field with step controls. | `InputChanged`, and `Clicked` for the steps |
| `DateTimePicker` | The two above combined, each keeping its own state. | As above |
| `Select` | A button showing the current choice that opens a `DropdownList` of the others. Controlled: the host passes the label, the options and whether it is open. `.sizing(SelectorSizing)` gives the trigger the profile's `min_width` and the list its `preferred_width`; `.width` and `.menu_width` override them, and without either the list is as wide as the trigger. `.menu_palette`, `.placement` (under the far edge by default), `.icon`, `.enabled`. | `Clicked` with its own ID, an option's action, or the dismiss ID |
| `FormField`, `FormFieldSizing` | A label over a control the host owns, which fills the field's width. `.hint` adds a muted line under the control, or between the label and the control with `.hint_above(true)`; `.error(text, ink)` adds a last line in the host's color. `FormFieldSizing` sets the label's role, whether it is muted, the least height of its line, the role of hints and error, and the gap. `.rows()` returns the field's lines instead of the rendered column, for a form that sets every line of every field in one column of its own and owns the gap. | Nothing of its own |
| `Composer` | A message composition surface. The host owns the text and the actions around it. `.header_strip(element, palette)` joins a 32 px strip of metadata to the top of the card, set in 8 from its sides and filled with that palette's surface; build its content `Composer::header_strip_width(width)` wide. | The actions of the controls the host places in it |
| `DictationButton`, `DictationState` | The round microphone of a composer. `Idle` is a ghost microphone; `Starting` and `Transcribing` show a `Spinner` and `Listening` a stop square that pulses as a `Skeleton` does, all on a filled circle and all still under reduced motion. The host owns the session and the transcript, and writes recognized text into the composer itself. `.live_palette(palette)` colors a live session (`surface` fills, `text` draws the mark), `.size(ControlSize)`, `.hover_ink`. `.enabled(false)` is a passive microphone in any state. | `Clicked` with its own ID in every state; the host toggles on it |

## Navigation

| Component | Purpose |
| --- | --- |
| `Tabs` | A row of tabs with a selection that slides to the active one. `.fill_width(total)` gives every tab the same width so the track is `total` wide. |
| `SegmentedControl` | Tabs in one rounded track, for choosing one of a few options. |
| `TabStrip`, `TabItem` | Folder tabs for open resources. Each `TabItem` has an icon, a title that fades, a tooltip and a 24-pixel slot that centers the host's close button, which should be 20 pixels square; the host gives every tab its width. `.title_ink` colors the title alone for a state the host marks by color, such as a file's status in version control; the slot takes any element, so a host can put a mark of unsaved changes there and swap it for the close button under the pointer. The strip scrolls sideways, keeps the active tab in view and reports `Clicked` with the tab's ID. `.each_tab` wraps a tab in something the host owns, such as a context menu. `TabStrip::header` is the 32-pixel bar that holds a strip and its tools (`.side_inset`, `.gap`); `TabStrip::surface` and `TabStrip::align` build label-only or icon-only tabs. The palette's `surface` is the active tab and the body, `raised` the bar, `hover` an inactive tab under the pointer. |
| `Breadcrumb` | A path. Segments with an action are links; the last is the current resource. Middle segments collapse into a menu when space is short. `Breadcrumb::from_path(id, path, palette)` splits a file path into its folders and the file with its icon; `.dirty(true)` marks unsaved changes, `.segment_action(|segment| ..)` lets the host make each segment a link, and `.item(..)` adds a segment after the file. |
| `Stepper` | The steps of a short form: completed, active and pending. |
| `Collapsible` | A header that shows or hides content beneath it. `.ink(color)` paints its icon, detail and chevron in a status color from the host. |
| `TreeRow`, `TreeGuides` | Rows and indentation guides for file and project trees. A row that folds takes `.count(n)`, small muted figures at its trailing edge, and `.disclosure(open)`, a muted chevron in a slot of its own; both sit before the `.trailing` element. |
| `Pager`, `PagerSizing` | Two arrows around a muted position label such as "3 / 12": `Pager::new(label, palette)` with `.previous(id, enabled)` and `.next(id, enabled)`. The host writes the label and says which ends can be taken; a disabled arrow is passive and keeps its hint. `.icons` changes the glyphs (up and down for a sequence that runs down the page), `.tooltips` takes a `TooltipSpec` for each arrow, `.label_width` holds the label in a fixed slot so the arrows do not move, and `.label_tooltip(id, spec)` spells the position out. `PagerSizing::DEFAULT` sets the arrows 4 from the label. |
| `MessageActions`, `MessageAction`, `MessageActionsVisibility`, `MessageActionDone` | The row of actions of a chat message: ghost icon buttons 24 high and 2 apart with muted ink, in the order they are added. `MessageActions::new(id, palette)` takes `.action(MessageAction::new(id, icon, title))`, `.caption(text)` for a time, a duration or a model name, and `.overflow(entries, open, title)` for a "···" menu of `DropdownMenuEntry`s, whose trigger reports `{id}:more` and which dismisses with `{id}:more:dismiss`. An action takes `.shortcut`, `.tooltip_with`, `.disabled(reason)` (passive, the reason under its title), `.done(bool)` (a check in place of the icon), `.active(bool)` for a toggle and `.dispatch_as`. `.align_end()` is for a user's message. `.visibility` is `Always`, `Hover` (painted while the pointer is over the `Element::hover_region` around the row, while one of its controls has keyboard focus or while its menu is open) or `Hidden` (room only); the row takes the same room in all three. The host owns the done state: keep a `MessageActionDone::new(id, now)`, pass `.shows(id, now)` to `done`, and repaint once after `.repaint_after(now)`. For a branch position, put a `Pager` beside it. |
| `MenuBar`, `MenuBarMenu` | The application menus across the top of a window: `MenuBar::new(menus, open_id, palette)` with one `MenuBarMenu::new(id, label, width, dismiss_id, entries)` per menu. The host owns which menu is open and every trigger's width; a trigger reports a click on its ID, fills with `palette.selected` while open, and its menu focuses the first enabled item. `.menu_palette` colors the menus, `.compact(id, dismiss_id, tooltip)` folds the bar into one button with the menus as submenus (`MenuBarMenu::compact_id` names each row), and `.width()` says how wide the bar is: 24 high, triggers 2 apart, 32 when compact. |
| `StatusBar`, `StatusBarItem`, `StatusBarItemSizing` | The bar at the foot of a window, 21 high with cells 4 apart. `StatusBarItem::new(id, width, palette)` takes `.icon`, `.label` and `.count` in the order they are shown, plus `.tooltip_with`, `.dispatch_as`, `.menu(entries, open, dismiss_id, menu_palette)` (opens upward, 256 wide), `.passive()` for information without an action, `.centered()`, `.ink(color)` for the glyphs and `.hosted(element)` for a cell the host wraps itself, such as a popover's anchor. `StatusBar::new(leading, trailing, palette).width(px)` always shows the leading items and keeps the trailing ones that fit, highest `.priority` first, in the order given; `.overflow(item)` is shown only while something was left out, `.trailing_budget(px)` replaces the bar's own reservation, and `.top_hairline(true)` adds a rule and the palette's surface for a bar that closes a panel. The host passes every width: the bar measures no text. Profiles: `LABEL` (glyph 12 wide, 4 from its label) and `COUNTS` (3 apart). |
| `ListRow` | One entry of a list under a single hover surface: an `.icon` centered in the row or a caller-owned `.leading` square, a title, a muted `.subtitle` (`.subtitle_mono` for a command or a path), `.meta` text or `.meta_element`s at the end of the title's line, a `.trailing` element and a muted `.trailing_icon` such as a chevron. It hovers to the palette's `hover` and presses to its `selected`, with no fill at rest unless `.filled()`. `.enabled(false)` renders a passive row with a muted title and no hit region; `.title_ink`, `.subtitle_ink` and `.icon_ink` take a color the host owns. `.title_fade(true)` fades a title that does not fit instead of cutting it. `.hover_with(id)` keeps the row hovered while the pointer is over a trailing control, and `.trailing_action(id, control)` adds a control with a hit target of its own under one hover surface shared with the row; the hint then covers the row's own target alone. `.radius(RadiusRole)` rounds the fill and the hover (`Control` unless set). `.outlined()` draws a hairline in the palette's border color around it, for a row that is a card of its own. `.width` fixes the width and clips the content to it, and `.copy_width` holds the copy at a width the host budgets instead of the width the row has left; `.tooltip_with` adds a hint. |
| `RowSizing` | The density of a `ListRow`: the two heights, padding, gaps, the icon, leading and trailing-icon columns, an optional `leading_height` that boxes the icon or the leading element from the top, the font sizes, and an optional box per line of copy. `RowSizing::COMPACT` (28, or 40 with a subtitle) and `DENSE` (24 / 36) are the presets; a `None` height follows the content. |
| `Card`, `CardDensity` | A raised surface that sets one group of content into the page: `Card::new(content, palette)` fills with the palette's `raised` under a hairline of its `border`. `.density` pads by 8 (`Compact`), 10 (`Regular`, the default) or 12 (`Roomy`), or not at all (`Flush`) for content whose rows run to the edge; `.border(false)` drops the hairline; `.radius(RadiusRole)` rounds it (`Card` unless set); `.width` fixes the width. It is passive and follows its content's height. The host picks the fill by passing a palette whose `raised` is the color it wants. |
| `OutputBlock` | Read-only monospaced output of a command or a tool: `OutputBlock::new(id, text, palette)`. One clipped 17 px line per row, in a scroll area that builds only the rows in view; the text is cut to 48 KiB, 600 lines and 500 characters per line before it is shaped. `.max_height` bounds the lines before they scroll (200 unless set), `.ink(color)` paints them in a color the host owns, such as an error stream, `.truncation_note(text)` adds a muted line when text was cut, and `.width` fixes the width. The fill is the palette's `surface`. |
| `CodeBlock`, `CodeBlockSizing` | The labelled card that holds code inside prose: `CodeBlock::new(text, palette)` wraps monospaced code on the palette's `raised` fill inside a hairline. `.label(text)` adds a muted line above it, in the case the host wrote; `.spans(spans)` paints text the host's highlighter already colored; `.width` fixes the width. `CodeBlockSizing::CHAT` (a 10 px monospaced label over 12 px code) and `DOCUMENT` (an 11 px sans label over 13 px code) are the presets. |
| `Flowchart`, `FlowchartGraph` | A flowchart drawn by the engine from the common subset of Mermaid's `flowchart`/`graph` syntax: `Flowchart::parse(source, palette)?` is the chart at its natural size and `.scrollable(id)` puts it in a horizontal viewport that pans, with an edge fade, when it is wider than its container. Directions `TD`/`TB`/`BT`/`LR`/`RL`; nodes `A[rect]`, `A(rounded)`, `A([stadium])`, `A{decision}`, `A((circle))` and bare ids; connectors `-->`, `---`, `-.->`, `==>` with `-->|text|` or `-- text -->` labels, chains and `A & B --> C`; `subgraph … end` as a titled box when the box holds no other node; `%%` comments; `style`, `classDef`, `class`, `linkStyle` and `click` are read and ignored. Anything else is a `DiagramError` (`FlowchartError` is the same type), for the host to show the source instead. Nodes are ranked by longest path and ordered by barycenter; labels wrap at 220. Nodes fill with the palette's `surface` inside a `border` hairline, connectors and their labels are `muted`, and `.backdrop(color)` names the fill the chart sits on (`raised` unless set). `FlowchartGraph::parse` and `.layout(measure, line_height)` give the graph and its geometry without painting. More than 120 nodes or 240 edges is an error; an edge from a node to itself is not drawn. |
| `SequenceDiagram`, `SequenceGraph` | A sequence diagram drawn by the engine from the common subset of Mermaid's `sequenceDiagram` syntax: `SequenceDiagram::parse(source, palette)?` is the diagram at its natural size and `.scrollable(id)` pans it like a flowchart. `participant` (a box) and `actor` (a figure), declared or named by a message, with `as` aliases; messages `->>`, `-->>`, `->`, `-->`, `-x`, `--x`, `-)`, `--)` with their text, to another participant or to the sender itself; activations by `activate`/`deactivate` and by `+`/`-` after the arrow; `Note left of`, `right of` and `over` one or two participants; `loop`, `alt`/`else`, `opt`, `par`/`and`, `critical`/`option` and `break` blocks with their labels, nested up to six deep, and `rect` as a highlighted region; `autonumber` (alone, with a start and a step, or `off`); `title`; `%%` comments. Keywords are read in any case. Anything else (`box`, `create`, `destroy`, `links`, two-headed arrows, a message without `:`, an `end` or a `deactivate` with nothing to close, an empty block) is a `DiagramError`, for the host to show the source instead. One lifeline per participant, in the order declared or first named, as far apart as the text between them needs: a message's text wraps at 240, a note at 180 and a name at 140, and no text, note or block meets a lifeline that is not its own. Every participant is drawn over its lifeline and again under it. Participants and activation bars fill with the palette's `surface` inside a `border` hairline, lifelines and blocks are `border`, arrows `muted` and what they say `text`; `.backdrop(color)` names the fill the diagram sits on (`raised` unless set). `SequenceGraph::parse` and `.layout(measure, line_height)` give the graph and its geometry without painting. More than 20 participants, 240 statements, 400 characters in one label or 32 KiB of source is an error. |
| `Diagram` | Either kind, for a host handed a source without knowing which it is: `Diagram::parse(source, palette)?` reads the header and gives a `Diagram::Flowchart` or a `Diagram::Sequence`, with the same `.scrollable(id)` and `.backdrop(color)`. A source over 64 KiB is an error. `DiagramViewer::new(base, title, close_id, palette, chart)` takes a `Diagram`, a `Flowchart` or a `SequenceDiagram` and shows it at its natural size in a panel over the window, panned in both directions; `close_id` is clicked by its close control, Escape and the backdrop. |

## Feedback and status

| Component | Purpose |
| --- | --- |
| `Alert` | A persistent inline notice in one of five tones. |
| `Toast` | A brief card in the corner of the window. The host decides when it leaves. |
| `Progress` | A thin bar for work whose completed share is known. `.flat()` gives it square ends and no track under the fill, for the meter of a dense list; the fill is `palette.text` and the rest `palette.selected`, so a host names a status color in the palette. |
| `Spinner` | An indicator for work of unknown duration. |
| `Skeleton` | A pulsing placeholder in the shape of content that is loading. |
| `SkeletonGroup` | Placeholder rows for a list that is loading: `SkeletonGroup::rows(id, palette, count)`, with an optional leading icon square. `into_rows()` hands the rows to a host that places each one itself. |
| `Empty` | The state of a page, pane or result list with nothing in it: an icon, a title, a reason and one next step. It has no frame of its own and sits on whatever surface it is given. `.framed()` sets it in a card of its own: the palette's surface, a hairline border and the card radius. |
| `Notice` | A left-aligned inline notice standing where content would be: an optional muted icon, a title, a reason, a caller-owned `.action(element)` under the copy and a `.trailing(element)` after it. `.stacked()` sets the icon over the copy instead of beside it. `.framed()` fills it with the palette's surface and the card radius, with no border. `.ink(color)` colors the title, for an error. `NoticeSizing::REGULAR` and `COMPACT` set its density; a profile with a `height` cuts each line of copy instead of wrapping it. |
| `Marker` | A one-line event in a conversation or log, optionally as a separator. `.ink(color)` paints it in a status color from the host, such as a failure. |
| `TextShimmer` | Text with a moving highlight, for work in progress. |
| `Collapse` | A slot in the flow that closes when its content leaves: `Collapse::new(id, content)` with `content: Option<Element>`, rendered every frame. When the content stops being passed the slot closes over `exit_ms` while the last paint fades, clipped to what is left and taking no pointer input, and what follows glides up. `.expand()` also opens it from zero over `enter_ms` when content arrives in a slot rendered empty. With reduced motion both happen at once. Keep it out of a column's `gap`, or the gap around the empty slot stays; put the spacing inside the content. `Element::animate_height(id, expand)` is the same for a host's own element whose height becomes zero. See [Animation](animation.md#content-in-the-flow-that-leaves). |
| `Ticker` | A fixed-height, clipped slot for the latest of a series of lines, such as the current step of running work: `Ticker::new(id, key, line)`. When `key` changes the new line rises in from below while the one it replaces moves up and fades, over `move_ms`; with reduced motion it swaps. `.height` sets the slot (20 unless set), which never changes, so content below does not move. `Element::animate_replace(id, key)` is the same movement for a host that clips the slot itself. |
| `ContextWindow` | What fills a model's context window: the figure in use, a field of cells shared out among the shares, one row per share with its count, and a foot with a status line and an action. `ContextWindow::new(id, title, shares, capacity, palette, tones)`; a `ContextShare` has a color, optional `items` it opens to list and an optional `ContextShareAction` (`Offer`, `Busy`, `Done`). The host owns every figure: when one changes the counts roll, the cells of a share that shrinks empty from its end and the shares after it follow. `.focused(share)` makes one share stand out, lending the figure its count while the others recede; pass `ContextWindow::share_under(id, tree.hovered_id())`. `.expanded`, `.caption`, `.status` / `.status_done`, `.badge`, `.footer_action`, `.width` (320 unless set) and `.format` set the rest. It has no surface of its own, so it sits in a popover or a card. Actions: `"{id}:share:{share}"`, `"{id}:action:{share}"`, `"{id}:remove:{share}:{item}"`. |
| `Element::rolling_number` | A figure that counts from the value it showed under the same ID to the one it is given, written by a `fn(f32) -> String` on every frame and laid out as it reads, so what follows it keeps to its side. Style it as a text. With reduced motion it shows the new value at once. |
| `Element::unit_grid` | A field of equal cells shared out among parts in reading order (`UnitGrid`: columns, rows, cell, gap, parts, the color of an empty cell and an optional part in focus). Each part's count is kept by ID and moves to a new one, so a part that shrinks gives its cells up from its end; with a part in focus the others recede towards the empty color. |
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
| `Popover` | A floating slot anchored to any element, for arbitrary content. On the element, `.popover_gap(px)` sets its distance from the anchor (5 unless set); 0 under a hairline anchor makes it stand in for the row that follows (an anchor of no height is not placed inside a scroll area). |
| `DropdownMenu` | A menu of actions with headings, separators, shortcuts and submenus. `.header(element)` stays above the options and `.footer(element)` under them, below a hairline, while they scroll. Entries beyond items: `DropdownMenuEntry::Message(text)` (a muted sentence), `Loading { id, rows }` (skeleton rows) and `Error { id, text }` (a failure in a scroll area of 104). |
| `DropdownMenuItem` | One action. `.enabled(bool)` or `.disabled()`, `.tooltip_with(TooltipSpec)` for a hint. With `.description(text)` it is a two-line row at `SelectorSizing::detail_row_height` (40): icon, label over a muted detail, and a check column filled when `selected`; `.detail_mono(true)` sets the detail in monospace, `.status(text)` puts short text at the end of the detail line (`.status_width(px)` fixes its slot) and `.chevron()` ends the row with a chevron instead of the check, for a row that opens another view. `.truncate()` cuts a long label to one line in a contextual menu, where it would otherwise wrap. |
| `DropdownMenuSection` | The heading of a group the user folds: `::new(id, label)` with `.icon`, `.count(n)` and `.collapsed(bool)`; it converts into a `DropdownMenuEntry` and reports `id` when activated. The host leaves the group's items out while it is collapsed. `.width(px)` fixes its width. |
| `MenuHeader` | The fixed head of a menu, passed to `DropdownMenu::header`: `.title(text)` or `.back(id, label)`, one square `.trailing(element)` tool, and `.search(id, query, placeholder)` for a compact search field under the row. `.width(px)` fixes its width. 28 high; the search field paints the palette's `raised`. |
| `MenuItem` | A menu row on its own, for a list the host lays out: icon, label and a shortcut or `.trailing` text, 26 high. `.description(text)` makes it the same two-line row as `DropdownMenuItem`, with `.detail_mono`, `.status`, `.status_width`, `.chevron` and `.selected(bool)`. `.enabled(false)` renders it passive. `.tooltip_with(TooltipSpec)` hints the row; `.trailing_action(id, icon, TooltipSpec)` adds a second 24 px action at the end, reached by Tab and not by the arrow keys, and `.trailing_dispatch_as(action)` makes it report another ID. |
| `PopoverPanel`, `PopoverPanelSizing` | The content of a `Popover` that is more than a menu: `.header(element)`, `.body(element)` and `.footer(element)` stacked 4 px apart with hairlines between them. The body takes the height it needs and gives way first in a short window; `.body_max_height(px)` caps it, for a body that is a scroll area. The popover keeps placement, width and surface. `.sizing(PopoverPanelSizing { gap, padding, header_rule })` sets another rhythm: `DEFAULT` is 4, 0 and a hairline under the header. |
| `SuggestionList` | Completions floating over an input without taking focus: `::new(anchor, id, items, selected_index, open, dismiss_id, palette)`. A `SuggestionItem` is `::new(id, label)` with `.icon`, `.description` and `.section(label)` to start a section. The host moves `selected_index`; rows are 42 high, at most 7 show, and only those in view are built. `.title`, `.empty(text)` for when there are no items, `.hints(elements)` (they stack under 300 px), `.note(text)`, `.width(available)` (the list is 160 to 352 wide), `.placement` and `.animation_ms(open, close)`. |
| `DropdownList` | A list for choosing one option, with the current one marked. |
| `ContextMenu` | A menu opened by a secondary click on its child. |
| `Modal` | A centered dialog for forms, with a title, a scrolling body and a footer. `Modal::fit_width(viewport_width, max)` is the width that keeps the window's margin clear, and `Modal::body_width(modal_width)` the width a body can count on: inside the padding and the rail reserved while it scrolls. |
| `AlertDialog` | A centered confirmation for consequential actions. `.alternative(id, label)` adds a second way to go ahead between Cancel and the confirm action, for the same step without its consequential part. |
| `ImageViewer` | A full-window image gallery. |
| `CoachMark` | A teaching bubble with a caret towards a control that already exists, named by its ID, and a ring around it. It reports close, next, back and skip; the rest of the window takes no input while it shows. See [Guided tours](tours.md). |
| `Tour`, `TourStep` | A sequence of coach marks with a step counter. The host owns the current step; the bubble travels from one target to the next. |

`Modal`, `AlertDialog`, `ImageViewer`, `CoachMark`, `Tour` and `Toast::layer`
wrap the window's root element, since they cover the whole window.

## Text

| Component | Purpose |
| --- | --- |
| `Text` | Passive text in a role of the type scale and a color of the palette: `.role`, `.muted`, `.ink` for a semantic color, `.mono`, `.truncate`, `.fade`, `.line_height`, and `.tooltip` to show the whole value of truncated text, wrapped. `Text::eyebrow` is the small monospaced label above a group. `.size(px)` is for a step the host's design has between two roles, such as 10.5; prefer a role. |
| `Markdown` | Markdown source as wrapped prose with links, lists, quotes and code blocks; see [Markdown](#markdown). |

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

## Markdown

Renders a bounded subset of Markdown as an ordinary element tree: headings of
three levels, paragraphs, bulleted, numbered and checked lists, quotes, fenced
code and rules, with strong, emphasis, inline code and web links. Use it for
text that arrives as Markdown, such as a message, including while it is still
being written.

### Example

```rust
let preview = Markdown::new(source, palette).width(reading_width).render();
// preview.element goes in the tree; preview.limited says the source was cut.
```

### Options

| Builder | Effect |
| --- | --- |
| `.width(px)` | The reading column, kept between 48 and 1,200; 600 unless set or when not finite. |
| `.colors(MarkdownColors)` | The ten inks and fills: `text`, `strong`, `emphasis`, `link`, `code_ink`, `code_fill`, `muted`, `success`, `panel`, `border`. The default, `MarkdownColors::for_palette`, reads the palette and paints strong text like body text. |
| `.streaming(bool)` | The source is still growing: an unclosed delimiter at its end already styles its text, a half-written fence is held back, and a web address at the end has no action until it is delimited. |
| `.link_action_prefix(text)` | The start of every link action; `markdown_link:` unless set. |
| `.labels(MarkdownLabels)` | `code_fallback`, the label of a code block with no language; `truncated_note`, the muted line under a cut preview; `diagram`, the label of a drawn diagram; and `show_source` and `show_diagram`, the action that switches a diagram to its source and back. English unless set. |
| `.diagram_action_prefix(text)` | Gives every drawn diagram that action; none unless set. |
| `.diagram_sources(&[u64])` | The diagrams, by the id in their action, to show as source. |
| `.diagram()` | Instead of `.render()`: the source is what one `mermaid` fence holds, and the result is the card that fence is drawn as, with the labels, prefix and sources set, or `None` when it is not a diagram the engine draws. For a host that renders its documents itself. |
| `.limits(MarkdownLimits)` | `source_chars` and `blocks` rendered: 20,000 and 160 unless set. |
| `.plain_with_autolinks(line_height)` | Keeps the source literal in one paragraph and only turns complete HTTP(S) addresses into links. Not cached and not cut. |

`Markdown::source_page(source, start_byte)` returns a `MarkdownSourcePage`: at
most 20,000 characters of the exact source, with the byte offsets of the pages
before and after it. `.render(width, palette)` shows it as monospaced rows.
`Markdown::web_links(source)` lists the complete web addresses in a text with
their character ranges, for a host that highlights them in an editor.
`Markdown::web_link(value)` gives one address in the normalized form the
renderer would open, or `None`, and `Markdown::link_icon(address)` the glyph
its link wears, for a host that draws a link of its own or receives an action.

### Actions

| Action | When |
| --- | --- |
| `Clicked { id }` with `id` = prefix, block hash, run index and URL, joined by colons | A link is activated. Only `http` and `https` destinations with a host and no credentials become links; anything else stays text. |

| `Clicked { id }` with `id` = the diagram prefix and the diagram's id as sixteen hexadecimal digits | The switch on a diagram or on its source is activated. The host keeps the ids it was asked to show as source and passes them to `.diagram_sources`. |

A closed `mermaid` fence that `Diagram` can read, a flowchart or a sequence
diagram, is drawn in a card like a code block's. A fence still being written,
another kind of Mermaid diagram or a source with an error stays a code block.
With a diagram prefix the card also carries an expand action, the prefix,
`expand:` and the same sixteen digits: `Markdown::diagram_source(id)` gives
the source of one of the last 64 diagrams drawn, for the host to parse and
show in a `DiagramViewer`.

The ID depends on the block's text and the link's position in it, so the same
block rendered twice gives the same IDs. A host that shows a source more than
once rewrites the span actions after rendering.

### Measurements

Body text is 14 on a 22 line; headings are 22, 18 and 16 at 1.4. A heading
has 20 above it and 8 below; list items are 4 apart and other blocks 14. A
list indents 14 per level with a 22 marker column. A quote is a padded panel
with the surface radius and a 2 px bar; a code block is a `CodeBlock` in
`CodeBlockSizing::CHAT` on the palette given to `new`.

### Notes

- The component stores nothing. Rendered trees are kept by an engine cache on
  the UI thread: 64 complete sources within 4 MB, and the last two streaming
  ones within 512 KB, so an append reuses every block above the one that
  changed. A tree is reused only for an equal source, width, palette, colors,
  labels, prefix and limits.
- Hold `Markdown::begin_frame(scope, width)` while building a long list of
  messages, so that a miss cannot evict the trees of the rows still to come.
  A new scope or width empties the cache.
- Labels and the link prefix are `&'static str`.
- There is no bold or italic face: strong and emphasis are colors.
- Tables, images, HTML and nested quotes are not supported and stay text.

## Layout and grouping

| Component | Purpose |
| --- | --- |
| `Separator` | A one-pixel rule in the border color. `Separator::horizontal` and `Separator::vertical` fill their axis unless given a `.length`; `.margin` adds space across the rule, `.centered_in` centers a vertical rule in a taller bar, and `.ink` takes a color the host owns. Margin and centering are off by default. |
| `ScrollArea` | A scrolling viewport, or a virtualized list of uniform rows. The host scrolls it through the tree: `scroll_to_start`, `scroll_to_end`, `scroll_to_index` for a uniform list, and `reveal(control_id)` or `reveal_with_margin` to bring any control inside it into view; see [Layout](layout.md#scrolling-from-the-host). A child of the column inside it can stay pinned at the top while its section scrolls: `.sticky_top()`, `.sticky_top_inset`, `.sticky_top_max_height`, `.sticky_top_fade`; see [Layout](layout.md#sticky-children). |
| `PageColumn` | A scrolling page whose content is a centered column: `PageColumn::new(scroll_id, available, palette)`. `.max_width` (960), `.min_width`, `.gutter(narrow, wide, breakpoint)` (14 and 28 at 640), `.gap` (10) and `.reserve_rail`, which keeps a scrollbar rail's width out of the column before the page overflows. `.tint_scrollbar(false)` leaves the scrollbar the scroll area's own color instead of the palette's muted. Read `.content_width()` to size the content, then pass it with `.children`. It fills its parent. |
| `PageHeader`, `PageHeaderSizing` | The heading of a page: a title, a `.description`, and a `.leading` and a `.trailing` element centered on it. `.stacked` moves the trailing element under the copy, `.trailing_top` keeps it on the title's line instead of centering it, `.truncate` cuts both lines instead of wrapping them. The profile holds the two font sizes, the three gaps and an optional fixed height; `PageHeaderSizing::DEFAULT` is a settings page. |
| `SectionHeader`, `SectionSizing` | The heading of a section: a title, a `.count` chip right after it, a muted `.detail` and a `.trailing` element at the far end, and a `.disclosure` chevron before it. With `.action` the whole row is a button. `.width` fixes its width and `.truncate` cuts the title to one line. Profiles: `TITLE` (a page section), `LIST` (a list heading) and `LABEL` (a quiet label in a side panel, hung from the top inset). |
| `PanelHeader`, `HeaderSizing` | The header row of a panel: a title cut to one line that takes the width left, `.trailing` tools after it, a `.back(id, tooltip)` arrow before it and a `.dismiss(id, tooltip)` close button at the end, both with hints that open downward. `.leading` puts an element in place of the title, such as tabs, a breadcrumb or a field. `.width` fixes the width. It paints no fill. Profiles: `SIDEBAR` (26 high), `PANE` (32, inset 4), `CARD` (28, inset 2, a body-sized title) and `POPOVER` (24); `center_tools: false` hangs the tools from the top inset instead of centering them. A rule between two groups of tools is a `Separator` with a length in the vector. |
| `SidebarPanel` | The body of a sidebar: its blocks in a column with an inset and a gap of 6, on the palette's `surface`. `.size(width, height)` fixes it. |
| `CardGrid` | Cards of equal width in rows of `.columns` (2), or in one column below the breakpoint: `CardGrid::new(width, breakpoint)`. Build each card at `.item_width()`, then pass them with `.items`. The gap is 8 and a short last row keeps its columns. |
| `Attachment` | A file or image card, with an optional remove action. |
| `PreferencesGroup`, `ActionRow` | A card of settings rows separated by hairlines, under a small muted label. Without a title it is just the card. A row takes an icon, a subtitle, a `.trailing` control such as a switch or a button, or a read-only `.value` in muted text, and can itself be an action. `.wrap_title` wraps the title instead of cutting it, `.align_top` starts the icon and the copy at the top, `.trailing_top` keeps the trailing element on the first line instead of centering it, `.icon_muted` quiets the icon for a note, and `.stacked` moves the trailing element under the copy, except one with a fixed width of 64 or less, which stays beside it. |
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
