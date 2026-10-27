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
| `SplitButton` | A primary action joined to a `DropdownMenu`: `SplitButton::new(id, label, menu_id, palette)` reports `id` for the action and `menu_id` for the chevron, each its own Tab stop. `.menu(entries, open, dismiss_id)` opens the menu below it, aligned to its end; `.keys(&["Ctrl", "Enter"])` draws the shortcut in keycaps after the label; `.variant` is a `Button`'s. |
| `ButtonGroup` | Rendered buttons 6 px apart: a row, or a full-width column with `stacked`. `align_end` pushes them to the far edge, `leading` keeps one element at the near edge (first in the column), and `reverse_when_stacked` puts the primary action first among the actions in the column. `sizing` takes a `ButtonGroupSizing` with one gap for the row and one for the column; `ButtonGroupSizing::DEFAULT` is 6 and 6. |
| `Toolbar`, `ToolbarItem` | A bar of commands in groups: `Toolbar::new(id, palette)` takes `.group(items)` in order, with a rule between two groups, and `.spacer()` to send the rest to the far end. It renders each `ToolbarItem` (`button`, `icon_button`, `toggle`, `icon_toggle`) as a ghost control at the bar's `.size(ControlSize)`, and places an `element` the host built. `.surface(bool)` and `.bordered(bool)` draw its own ground and outline, `.vertical()` lays it down a side, `.width(px)` fixes it and `.overflow(open, title)` moves what does not fit behind a menu, lowest `.priority` first. `.disabled()` disables every control it renders. Each control reports `Clicked` with its own ID; the menu's button reports `{id}:overflow_toggle` and its dismissal `{id}:overflow_dismiss`. |
| `Clipboard`, `ClipboardCopied` | A button that copies a link, a key or a path: `Clipboard::new(id, title, palette)` is a ghost icon button with the copy icon, muted ink and `title` as its hint. `.copied(bool)` shows a check in its place and in the same room; `.label(text)` adds text after the icon, `.icon(icon)` changes the one at rest, and `.variant` and `.size` are a button's. The host owns the clipboard and the state: on the click it copies the value, keeps a `ClipboardCopied::new(id, now)`, passes `.shows(id, now)` to `copied`, repaints once after `.repaint_after(now)` (1.5 s), and announces the copy with a `Toast`. |
| `Link` | A text action: link ink, underlined under the pointer and keyboard focus, no hover fill. |
| `Kbd` | Passive keycaps for a shortcut such as `Ctrl+Shift+P`. `.caption(text)` says what the keys do in a small muted caption after them, and `.alternative(keys)` adds another key after a slash (Enter / Tab): the hint in the footer of a palette or a suggestion list, 20 px high. |

Button variants: `Default`, `Secondary`, `Outline`, `Ghost`, `Subtle`,
`Raised`, `Solid`, `Link`, `Success`, `Warning`, `Destructive`. `Subtle` is
the navigation row and keeps its label at the left; the filled variants center
it.

## Inputs

| Component | Purpose | Reports |
| --- | --- | --- |
| `TextField` | Single-line text. Optional password masking; `.monospace(true)` for a path, an identifier or a number; `.height(px)` for a form whose inputs are taller than a `ControlSize` (never shorter than it); `.icon(Icon)` before the text, as a search field's magnifier. | `InputChanged`, `Submitted` |
| `TextArea` | Multi-line text. Enter adds a line; Ctrl+Enter submits. `.enter_submits(true)` sends on Enter and adds a line on Shift+Enter, as a chat does. `.radius(RadiusRole)` rounds the box with another token. | `InputChanged`, `Submitted` |
| `SearchField` | A text field with a magnifier and an optional shortcut hint. `.clearable(true)` adds a button at its end while it has a query; the host empties the query and the field. | `InputChanged`, `Submitted`, and `Clicked` with `"{id}:clear"` |
| `Switch`, `SwitchVariant` | An on/off control. `.size(ControlSize)` from 28×16 to 44×24, or `.track_size(width, height)` of the host's own; `SwitchVariant::Accent` keeps a light thumb on a track of the palette's link color, or of `.ink(color)`, darkened where needed for the thumb to read on it, as chat applications draw a switch; `.off_ink(color)` sets the track while off, as given. | `Clicked` |
| `Toggle`, `ToggleGroup`, `ToggleItem`, `ToggleVariant` | A button that stays pressed: a label, an icon or both, `Ghost` or `Outline`, in the four `ControlSize`s. A `ToggleGroup` joins several in one outlined row, of which any number may be pressed. The host owns each value. | `Clicked` with the toggle's own ID |
| `Checkbox` | A box that is checked or not, with an optional `.label` that is clicked with it and wraps in a narrow width, the box staying on its first line. `.indeterminate(true)` draws a dash for the parent of a partly chosen list. `.size(ControlSize)`, `.enabled`, and `.accessibility_label` for a box alone. | `Clicked` |
| `RadioGroup`, `RadioOption` | One choice among several: a circle for each option, with a dot in the chosen one and a label that is clicked with it. An option takes a muted `.description`, an `.icon` before its label, and can be disabled alone. Vertical unless `.horizontal()`; `.size(ControlSize)`, `.enabled(false)`. Left, Right, Home and End move the choice. | `Clicked`, with the chosen option's value as its ID |
| `NumberInput` | A field for a number joined to a button that lowers it and one that raises it. Controlled: the host passes the text, and `NumberInput::adjusted_value` turns a click into the next text within `.min`, `.max` and `.step`, written with `.decimals`. A button with nowhere to go is disabled. | `InputChanged`, and `Clicked` for the two buttons |
| `Slider` | A value dragged along a track, from 0 to 1: a seek position or a volume. Continuous unless `.steps(n)` gives it evenly spaced stops, which `.marks()` draws and `.mark_labels(..)` names, the `.default_stop(i)` in the ink. `.ink(color)` colors the fill and thumb, `.level(level, color)` draws a live level along the track in place of the fill, `.enabled(false)` shows the value without taking the pointer. The arrows move it by a twentieth or one stop; Home and End go to the ends. | `SliderChanged`, with `committed` false during a drag and true on release and from the keyboard |
| `SwatchPicker`, `Swatch` | One color among a few: a round swatch for each, named for the platform, and a ring around the chosen one in the text's ink. `.custom(color, name)` adds a swatch before the others under a pencil, for a color of the user's own, that opens a picker; `.columns(n)` breaks the swatches into rows; `.size(ControlSize)` sets the diameter from 20 to 32. The arrows, Home and End move the choice, as in a `RadioGroup`. | `Clicked`, with the chosen swatch's value as its ID, or `{id}:custom` |
| `ColorPicker`, `Hsv` | A color chosen by hand: a field of saturation across and value up, a bar of hues and a hex field. Controlled: the host keeps an `Hsv`, and `ColorPicker::color_after` turns any of its actions into the next one; `ColorPicker::hex` and `parse_hex` write and read `#rrggbb` and `#rgb`. `.width(px)`, 220 unless set; `.part_labels` names the three parts in the host's language. | `AreaSliderChanged` for `{id}:area`, `SliderChanged` for `{id}:hue`, `InputChanged` for `{id}:hex` |
| `DatePicker` | An ISO date field with a navigable month calendar. | `InputChanged`, and `Clicked` for navigation and days |
| `TimePicker` | A 24-hour `HH:MM` field with step controls. | `InputChanged`, and `Clicked` for the steps |
| `DateTimePicker` | The two above combined, each keeping its own state. | As above |
| `Select` | A button showing the current choice that opens a `DropdownList` of the others. Controlled: the host passes the label, the options and whether it is open. `.sizing(SelectorSizing)` gives the trigger the profile's `min_width` and the list its `preferred_width`; `.width` and `.menu_width` override them, and without either the list is as wide as the trigger. `.menu_palette`, `.placement` (under the far edge by default), `.icon`, `.enabled`. `.variant(SelectVariant::Field)` draws the trigger as a text field, the choice at its start; `.size(ControlSize)` and `.height(px)` as a text field takes them; open, its chevron points up and `.ink(color)` gives it a 2 px border. `.font_size(px)` and `.max_visible_items(n)`. | `Clicked` with its own ID, an option's action, or the dismiss ID |
| `FormField`, `FormFieldSizing` | A label over a control the host owns, which fills the field's width. `.hint` adds a muted line under the control, or between the label and the control with `.hint_above(true)`; `.error(text, ink)` adds a last line in the host's color. `FormFieldSizing` sets the label's role, whether it is muted, the least height of its line, the role of hints and error, and the gap. `.rows()` returns the field's lines instead of the rendered column, for a form that sets every line of every field in one column of its own and owns the gap. | Nothing of its own |
| `Composer` | A message composition surface. The host owns the text and the actions around it. `.enter_submits(true)` sends on Enter and adds a line on Shift+Enter. `.header_strip(element, palette)` joins a 32 px strip of metadata to the top of the card, set in 8 from its sides and filled with that palette's surface; build its content `Composer::header_strip_width(width)` wide. With `.inline_controls(true)` and `.auto_grow` the editor grows between its controls, which stay beside its first line, or its last with `.controls_at_last_line(true)`; `.editor_padding(px)` and `.card_corner_radii(radii)`. | The actions of the controls the host places in it |
| `DictationButton`, `DictationState` | The round microphone of a composer. `Idle` is a ghost microphone; `Starting` and `Transcribing` show a `Spinner` and `Listening` a stop square that pulses as a `Skeleton` does, all on a filled circle and all still under reduced motion. The host owns the session and the transcript, and writes recognized text into the composer itself. `.live_palette(palette)` colors a live session (`surface` fills, `text` draws the mark), `.size(ControlSize)`, `.hover_ink`. `.enabled(false)` is a passive microphone in any state. | `Clicked` with its own ID in every state; the host toggles on it |
| `DictationWaveform` | The level of what the microphone hears, beside the `DictationButton`. Controlled: the host samples the input and passes the recent levels, from 0 to 1 and oldest first; one every 80 ms fills the default width with two seconds. The newest stands at the trailing edge, the ones that do not fit are dropped from the oldest, and silence, or no level at all, is a row of dots. Each bar springs to its level. `.width(96.0)`, `.size(ControlSize)` for the height of a control of that size. A level that is not finite is drawn as silence. | None |

## Navigation

| Component | Purpose |
| --- | --- |
| `NavigationBar`, `NavigationWorkspace`, `NavigationAction` | A floating pill of workspace dots, clock/date and status actions. The selected workspace springs between dots; controls keep tooltips, keyboard focus and host-owned actions. Narrow bars compact labels and scroll horizontally. See [NavigationBar](components/navigation-bar.md). |
| `Tabs`, `TabsVariant` | A row of tabs with a selection that slides to the active one. `.fill_width(total)` gives every tab the same width so the track is `total` wide. `.variant(TabsVariant::Underlined)` draws text tabs with no track, a 2-pixel underline under the active one and a hairline in the palette's `border` under all of them; `.accent` colors the active label and the underline, `.gap` the space between tabs. |
| `SegmentedControl` | Tabs in one rounded track, for choosing one of a few options. |
| `TabStrip`, `TabItem` | Folder tabs for open resources. Each `TabItem` has an icon, a title that fades, a tooltip and a 24-pixel slot that centers the host's close button, which should be 20 pixels square; the host gives every tab its width. `.title_ink` colors the title alone for a state the host marks by color, such as a file's status in version control; the slot takes any element, so a host can put a mark of unsaved changes there and swap it for the close button under the pointer. The strip scrolls sideways, keeps the active tab in view and reports `Clicked` with the tab's ID. `.each_tab` wraps a tab in something the host owns, such as a context menu. `TabStrip::header` is the 32-pixel bar that holds a strip and its tools (`.side_inset`, `.gap`); `TabStrip::surface` and `TabStrip::align` build label-only or icon-only tabs. The palette's `surface` is the active tab and the body, `raised` the bar, `hover` an inactive tab under the pointer. |
| `Breadcrumb` | A path. Segments with an action are links; the last is the current resource. Middle segments collapse into a menu when space is short. `Breadcrumb::from_path(id, path, palette)` splits a file path into its folders and the file with its icon; `.dirty(true)` marks unsaved changes, `.segment_action(|segment| ..)` lets the host make each segment a link, and `.item(..)` adds a segment after the file. |
| `Stepper`, `Step` | The steps of a short form: `Stepper::new(steps, active_index, palette)` with one `Step::new(id, label)` per step, which takes `.description(text)` and `.icon(icon)`. Each step is a round badge 24 across with its glyph, or its number without one, a rule 2 thick from it to the next badge, and its label under it; every step but the last takes an equal share of `.width`. The steps up to the active one have the badge filled in the text ink, and so are the rules between them; the ones to come are quiet and lift under the pointer, and the fills cross-fade when the step changes. `.vertical()` stacks the steps with the label and the description beside each badge and the rules running down. A click anywhere on a step reports its ID; the host owns the index. |
| `Collapsible` | A header that shows or hides content beneath it. While open, the header and the content read as one filled block, and the content opens and closes in place. `.ink(color)` paints its icon, detail and chevron in a status color from the host. |
| `Accordion`, `AccordionItem` | Several titled items one under another, each of which opens under its header: `Accordion::new(id, items, palette)` with one `AccordionItem::new(id, title, content)` per item, which takes `.open(bool)`, `.icon(icon)`, a muted `.description(text)` beside the title and `.disabled()`. The host owns which items are open, so one at a time and several at a time are both its rule; a header reports `Clicked` with its item's ID. `.bordered(true)` draws one outlined surface in place of a plain list with hairlines, and `.size(ControlSize)` sets the header's height and text. The content opens and closes with the library's `Collapse`, and that of a closed item is not in the window. |
| `TreeRow`, `TreeGuides` | Rows and indentation guides for file and project trees. A row that folds takes `.count(n)`, small muted figures at its trailing edge, and `.disclosure(open)`, a muted chevron in a slot of its own; both sit before the `.trailing` element. |
| `Pager`, `PagerSizing` | Two arrows around a muted position label such as "3 / 12": `Pager::new(label, palette)` with `.previous(id, enabled)` and `.next(id, enabled)`. The host writes the label and says which ends can be taken; a disabled arrow is passive and keeps its hint. `.icons` changes the glyphs (up and down for a sequence that runs down the page), `.tooltips` takes a `TooltipSpec` for each arrow, `.label_width` holds the label in a fixed slot so the arrows do not move, and `.label_tooltip(id, spec)` spells the position out. `PagerSizing::DEFAULT` sets the arrows 4 from the label. |
| `Pagination` | Numbered pages between a previous and a next arrow: `Pagination::new(id, page, pages, palette)`, pages counting from 1. The first and the last page always show, with the current one, `.siblings(n)` neighbours on each side (1 unless set) and an ellipsis where pages are left out; the row keeps the same cells on every page. Each button reports `Clicked` with `{id}={page}`, which `Pagination::page_of(action_id, id)` reads back. `.compact()` is the arrows around "3 / 12"; `.size(ControlSize)`, `.labels(previous, next, page)` for the words, `.disabled()`. See [Pagination](components/pagination.md) for when to use it and when a `Pager`. |
| `MessageActions`, `MessageAction`, `MessageActionsVisibility`, `MessageActionDone` | The row of actions of a chat message: ghost icon buttons 24 high and 2 apart with muted ink, in the order they are added. `MessageActions::new(id, palette)` takes `.action(MessageAction::new(id, icon, title))`, `.caption(text)` for a time, a duration or a model name, and `.overflow(entries, open, title)` for a "···" menu of `DropdownMenuEntry`s, whose trigger reports `{id}:more` and which dismisses with `{id}:more:dismiss`. An action takes `.shortcut`, `.tooltip_with`, `.disabled(reason)` (passive, the reason under its title), `.done(bool)` (a check in place of the icon), `.active(bool)` for a toggle and `.dispatch_as`. `.align_end()` is for a user's message. `.visibility` is `Always`, `Hover` (painted while the pointer is over the `Element::hover_region` around the row, while one of its controls has keyboard focus or while its menu is open) or `Hidden` (room only); the row takes the same room in all three. The host owns the done state: keep a `MessageActionDone::new(id, now)`, pass `.shows(id, now)` to `done`, and repaint once after `.repaint_after(now)`. For a branch position, put a `Pager` beside it. |
| `ChatMinimap`, `ChatMinimapMark` | A conversation's turns as a rail of short marks beside it: `ChatMinimap::new(id, marks, palette)` with one `ChatMinimapMark::new(id, title).preview(text)` per turn, both cut to one line. The mark under the pointer or the keyboard focus grows from 8 to 26 px and four neighbours on each side step down to rest, each on a spring that keeps its speed when the pointer moves on; that mark shows its title and preview in a tooltip at its side with no wait. `.active(id)` marks the turn in view, longer and in the text ink. A click reports the mark's ID; Up, Down, Home and End move the focus along the rail and report the mark they reach, without wrapping. Marks are 10 px apart and sit closer when the rail gets less height than they ask for. For another kind of long view, build the `MarkRail` yourself and pass it to `Element::mark_rail`. |
| `MenuBar`, `MenuBarMenu` | The application menus across the top of a window: `MenuBar::new(menus, open_id, palette)` with one `MenuBarMenu::new(id, label, width, dismiss_id, entries)` per menu. The host owns which menu is open and every trigger's width; a trigger reports a click on its ID, fills with `palette.selected` while open, and its menu focuses the first enabled item. `.menu_palette` colors the menus, `.compact(id, dismiss_id, tooltip)` folds the bar into one button with the menus as submenus (`MenuBarMenu::compact_id` names each row), and `.width()` says how wide the bar is: 24 high, triggers 2 apart, 32 when compact. |
| `WindowControls` | The minimize, maximize and close buttons at the end of a title bar the app draws itself, as Windows places them: `WindowControls::new(id, palette)`, `.maximized(bool)` shows restore in the middle, `.height(px)` makes them as tall as the bar (32 by default). Each is 46 wide (`WindowControls::WIDTH` is all three); close turns red under the pointer. They report `{id}:minimize`, `{id}:maximize` and `{id}:close` (`minimize_id`, `maximize_id`, `close_id`); the host answers with its backend's window commands. |
| `StatusBar`, `StatusBarItem`, `StatusBarItemSizing` | The bar at the foot of a window, 21 high with cells 4 apart. `StatusBarItem::new(id, width, palette)` takes `.icon`, `.label` and `.count` in the order they are shown, plus `.tooltip_with`, `.dispatch_as`, `.menu(entries, open, dismiss_id, menu_palette)` (opens upward, 256 wide), `.passive()` for information without an action, `.centered()`, `.ink(color)` for the glyphs and `.hosted(element)` for a cell the host wraps itself, such as a popover's anchor. `StatusBar::new(leading, trailing, palette).width(px)` always shows the leading items and keeps the trailing ones that fit, highest `.priority` first, in the order given; `.overflow(item)` is shown only while something was left out, `.trailing_budget(px)` replaces the bar's own reservation, and `.top_hairline(true)` adds a rule and the palette's surface for a bar that closes a panel. The host passes every width: the bar measures no text. Profiles: `LABEL` (glyph 12 wide, 4 from its label) and `COUNTS` (3 apart). `.strong()` sets the label in the text ink, for the one value the bar is read for, such as the branch; `.tinted_icon(icon, color)` adds a glyph in a color of its own whatever the cell's ink, such as a severity's; `StatusBarItem::divider(id, palette)` is a passive hairline one pixel wide and 11 tall between two groups of cells. |
| `TopBar` | The bar across the top of a window, as a music app's: `TopBar::new(id, query, placeholder, palette)` with the host's `.leading` and `.trailing` controls at the ends, and a 48 px round Home button and a pill search centered in the window between them, or after the start controls when centered they would meet, narrowed to the room left. `.width(window)` sizes the search to half of it, between 350 and 546, and makes it a round button at 400 or less; `.caption(left, right)` keeps room for the window's own buttons; `.home_active` and `.browse_active` draw `Icon::HomeFill` and `Icon::GridFill`. The search ends in a rule and a browse button, with a clear button before them while there is a query. It reports `InputChanged` and `Submitted` with `{id}:search`, and `Clicked` with `{id}:home`, `{id}:browse`, `{id}:search:clear`, and `{id}:search` for the round search. 64 px tall. See [TopBar](components/top-bar.md). | `.search_popover(dismiss_id, panel)` floats a panel under the search, such as recent searches.
| `ListRow` | One entry of a list under a single hover surface: an `.icon` centered in the row or a caller-owned `.leading` square, a title, a muted `.subtitle` (`.subtitle_mono` for a command or a path), `.meta` text or `.meta_element`s at the end of the title's line, a `.trailing` element and a muted `.trailing_icon` such as a chevron. It hovers to the palette's `hover` and presses to its `selected`, with no fill at rest unless `.filled()`. `.enabled(false)` renders a passive row with a muted title and no hit region; `.title_ink`, `.subtitle_ink` and `.icon_ink` take a color the host owns. `.title_fade(true)` fades a title that does not fit instead of cutting it. `.hover_with(id)` keeps the row hovered while the pointer is over a trailing control, and `.trailing_action(id, control)` adds a control with a hit target of its own under one hover surface shared with the row; the hint then covers the row's own target alone. `.radius(RadiusRole)` rounds the fill and the hover (`Control` unless set). `.outlined()` draws a hairline in the palette's border color around it, for a row that is a card of its own. `.width` fixes the width and clips the content to it, and `.copy_width` holds the copy at a width the host budgets instead of the width the row has left; `.tooltip_with` adds a hint. |
| `ListRowSizing` | The density of a `ListRow`: the two heights, padding, gaps, the icon, leading and trailing-icon columns, an optional `leading_height` that boxes the icon or the leading element from the top, the font sizes, and an optional box per line of copy. `ListRowSizing::COMPACT` (28, or 40 with a subtitle) and `DENSE` (24 / 36) are the presets; a `None` height follows the content. |
| `Card`, `CardDensity` | A raised surface that sets one group of content into the page: `Card::new(content, palette)` fills with the palette's `raised` under a hairline of its `border`. `.density` pads by 8 (`Compact`), 10 (`Regular`, the default) or 12 (`Roomy`), or not at all (`Flush`) for content whose rows run to the edge; `.border(false)` drops the hairline; `.radius(RadiusRole)` rounds it (`Card` unless set); `.width` fixes the width. It is passive and follows its content's height. The host picks the fill by passing a palette whose `raised` is the color it wants. |
| `MediaCard` | A track as a card to hand around: its record sleeve beside its title over a muted `.artist`, at the end of the card, over a `.tint` the host takes from the artwork (the palette's `raised` without one) that darkens towards the foot so the copy reads on any color. `MediaCard::new(id, title, palette)` takes the picture with `.artwork(RasterImage)`, cropped to the sleeve's square, and an element for the top corner at the end with `.mark(element)`. The sleeve is the control and reports a click on the card's ID; the host answers by playing or pausing and passing `.playing(bool)`. While it plays the record is out of its sleeve and turning, a turn every three seconds, and stopped it keeps the angle it reached; under the pointer it comes out as an invitation. The sleeve and its record are `Element::record`, for a host that wants them alone. `.width` (325, never under 220) fixes the width and a long title fades without widening it; `.enabled(false)` renders a passive card with its record in. With reduced motion the record shows out and holds still. |
| `MediaTile` | A tile of a grid or a shelf of albums and artists under a single hover surface: square artwork over a one-line title and a muted `.subtitle`, both cut with an ellipsis. `MediaTile::new(id, title, palette)` takes a caller-owned picture with `.artwork(element)` and holds it in a square of the tile's `.width` (160 unless set, never under 48) with the `Card` radius; a picture of another shape fills the square when its `RasterImage` is marked `.cover()`, which crops it around its center, and is otherwise shown whole inside it. Without one it draws a muted music note on the palette's `selected`, as for a cover that has not loaded. `.round(true)` makes the artwork a circle and centers the copy, for an artist rather than an album. It hovers to the palette's `hover` and presses to its `selected` across artwork and copy, and a click anywhere on it reports its ID. `.enabled(false)` renders a passive tile with a muted title and no hit region. Its height follows the copy. |
| `Carousel` | A shelf of host-built items, such as `MediaTile`s, that scrolls sideways inside a page that scrolls down, without arrow buttons: `Carousel::new(id, items, palette)`. A drag anywhere on it pans it (a press on an item becomes a drag after 4 px sideways and then clicks nothing; a press that does not travel clicks the item), a released drag glides on and slows to rest, and a trackpad's sideways scroll or Shift with the wheel pans it while the wheel scrolls the page. Optional `.title` (24 px bold), `.eyebrow` (a muted line over it), `.leading` (a picture held round in 48 × 48) and `.trailing` (such as a "Show all" link); `.gap` (24 unless set) and `.inset`, the space before the first item and around the heading, so the row lines up with the page's gutter and still scrolls to its edges. Edges with hidden items fade into the palette's `surface`; an indicator shows while it moves. Under reduced motion a released row stays where it was let go. |
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
| `Progress` | A thin bar for work whose completed share is known. `.flat()` gives it square ends, the fill and the rest side by side as two pieces, for the meter of a dense list; the fill is `palette.text` and the rest `palette.selected`, so a host names a status color in the palette. `.ring(diameter)` draws a ring in place of the bar, its arc covering the fraction from twelve o'clock: a gauge such as a model's context used. |
| `Spinner` | An indicator for work of unknown duration. |
| `StatusDot` | A round mark in a status color, 6 px unless `.size(px)` says otherwise. `.pulse(true)` eases its opacity from 1 to 0.3 and back every 1.2 s, every pulsing dot in step, and holds it at 1 under reduced motion. With `.id(id)` the platform knows it as a picture named by `.description(text)`. |
| `Skeleton` | A pulsing placeholder in the shape of content that is loading. |
| `SkeletonGroup` | Placeholder rows for a list that is loading: `SkeletonGroup::rows(id, palette, count)`, with an optional leading icon square. `into_rows()` hands the rows to a host that places each one itself. |
| `Empty` | The state of a page, pane or result list with nothing in it: an icon, a title, a reason and one next step. It has no frame of its own and sits on whatever surface it is given. `.framed()` sets it in a card of its own: the palette's surface, a hairline border and the card radius. |
| `Notice` | A left-aligned inline notice standing where content would be: an optional muted icon, a title, a reason, a caller-owned `.action(element)` under the copy and a `.trailing(element)` after it. `.stacked()` sets the icon over the copy instead of beside it. `.framed()` fills it with the palette's surface and the card radius, with no border. `.ink(color)` colors the title, for an error. `NoticeSizing::REGULAR` and `COMPACT` set its density; a profile with a `height` cuts each line of copy instead of wrapping it. |
| `Marker` | A one-line event in a conversation or log, optionally as a separator. `.ink(color)` paints it in a status color from the host, such as a failure. |
| `TextShimmer` | Text with a moving highlight, for work in progress. |
| `Collapse` | A slot in the flow that closes when its content leaves: `Collapse::new(id, content)` with `content: Option<Element>`, rendered every frame. When the content stops being passed the slot closes on a spring, in one and a half `move_ms`, while the last paint fades, clipped to what is left and taking no pointer input, and what follows glides up. `.expand()` also opens it from zero on the same spring when content arrives in a slot rendered empty. With reduced motion both happen at once. Keep it out of a column's `gap`, or the gap around the empty slot stays; put the spacing inside the content. `Element::animate_height(id, expand)` is the same for a host's own element whose height becomes zero. See [Animation](animation.md#content-in-the-flow-that-leaves). |
| `RollingNumber` | A number whose digits roll to their new values: `RollingNumber::new(id, text, palette)`. Each digit is a column of 0 to 9 behind a one-line window, known by its place from the right; it rolls up or down over three `move_ms`, and shows the new value at once under reduced motion. `.color`, `.font_size`, `.font_family`. `Element::animate_slide(id, y)` is the movement alone. |
| `Ticker` | A fixed-height, clipped slot for the latest of a series of lines, such as the current step of running work: `Ticker::new(id, key, line)`. When `key` changes the new line rises in from below while the one it replaces moves up and fades, over `move_ms`; with reduced motion it swaps. `.height` sets the slot (20 unless set), which never changes, so content below does not move. `Element::animate_replace(id, key)` is the same movement for a host that clips the slot itself. |
| `FramedCard` | A card that stands on a frame a shade apart from it, with a foot on the frame for a status and an action that belong to the whole card. `FramedCard::new(palette)`, then `.section(element)` for content that keeps to the card's inset, `.rows(element)` for a row or a block of rows whose fill reaches past the inset on both sides, `.space(px)`, `.foot(element)` and `.width(px)`. A row's fill keeps the same margin from the card's edge on every side and its corner follows the card's. `FramedCard::row(id, content, actions, palette)` builds one row with the hover fill its controls share, and `FramedCard::block(id, row, listed, palette)` a row and what it opens to list, read as one block while open. It has no surface of its own: the host gives it one with rounded corners and no padding. `.flat()` is for a host whose surface already is the card, such as a popover: no frame and no card are drawn and the parts keep their insets from the host's edge; `.into_parts()` then gives the body and the foot apart, for a host that scrolls one and pins the other. `ContextWindow` is built on it and has the same `.flat()` and `.parts()`. |
| `ContextWindow` | What fills a model's context window: the figure in use, a field of cells shared out among the shares, one row per share with its count, and a foot with a status line and an action. `ContextWindow::new(id, title, shares, capacity, palette, tones)`; a `ContextShare` has a color, optional `items` it opens to list and an optional `ContextShareAction` (`Offer`, `Busy`, `Done`). `.unmeasured()` lists a share the host could not measure: a dash stands where its count would be, also in the figure while it is in focus, and it takes no cells. The host owns every figure: when one changes the counts roll, the cells of a share that shrinks empty from its end and the shares after it follow. An item that is removed closes where it was, and the percentage fades to the colour of its new state. `.focused(share)` makes one share stand out, lending the figure its count while the others recede; pass `ContextWindow::share_under(id, tree.hovered_id().unwrap_or_default())`. `.expanded`, `.caption`, `.status` / `.status_done`, `.status_lead` (the words the status opens with, which read stronger), `.badge`, `.header_icon`, `.footer_action`, `.width` (320 unless set) and `.format` set the rest. It draws its own card on a frame a shade apart, with the foot on the frame, so the host gives it a surface with rounded corners and no padding. A row's hover fill reaches past its content on both sides, and an open share reads as one block with what it lists. Actions: `"{id}:share:{share}"`, `"{id}:action:{share}"`, `"{id}:remove:{share}:{item}"`. |
| `Element::rolling_number` | A figure that counts from the value it showed under the same ID to the one it is given, written by a `fn(f32) -> String` on every frame and laid out as it reads, so what follows it keeps to its side. Style it as a text. With reduced motion it shows the new value at once. |
| `Element::unit_grid` | A field of equal cells shared out among parts in reading order (`UnitGrid`: columns, rows, cell, gap, parts, the color of an empty cell and an optional part in focus). Each part's count is kept by ID and moves to a new one, so a part that shrinks gives its cells up from its end; with a part in focus the others recede towards the empty color. |
| `Element::level_bars` | Levels from 0 to 1 as a row of bars, as many as fit the width it is given (`LevelBars`: the levels oldest first, the width of a bar, the gap, the ink of a bar and of a dot). The newest level takes the last slot; a slot without one is a dot. Each slot's height is kept by ID and springs to a new level. `DictationWaveform` is this with a palette. |
| `Chip` | An inline pill for a count, a label or a state. `.outline()` draws the border only, with no fill. `.tone(AlertTone)` colors it as an alert of that tone colors its icon, a tint under a label that reads on it, and `.color(color)` does the same with a color of the host's. `.size(ControlSize)` sets height and text together, `.height` the height alone. `.icon(icon)` or `.dot()` stands before the label in its color. It is passive unless `.remove_action(id)` adds a close button that reports `Clicked` with that ID, named `Labels::remove` and the label; `.disabled()` / `.enabled(bool)` mutes the chip and makes the button passive. |
| `Avatar` | An account photo, or the initial of a name when there is none. |
| `AvatarGroup` | Several avatars that overlap in a row, the first on top, each with a ring in the surface color: `AvatarGroup::new(avatars, palette)`. `.limit(n)` shows the first `n` and counts the rest in a circle that reads `+N`; `.size` sets every diameter, `.overlap` the share one covers of the next (a quarter unless set) and `.ring` the ring's color on another ground. Passive; one image for the platform with `.id` and `.accessibility_label`. |
| `Badge` | A passive mark over the top right corner of another element: `Badge::new(child, palette)` with `.count(n)` (`.max(n)`, 99 unless set, above which it reads `99+`; zero shows nothing unless `.show_zero(true)`) or `.dot()`. `.color` sets its fill, `.placement(BadgePlacement)` its corner, `.height`, `.outline(color, width)`, `.ink`, `.font_size` and `.font_family` its size and look, and `.description` what it says of the control under it to a screen reader. The child keeps its layout size, hit region, focus and actions. |

Alert tones, shared by `Toast`: `Neutral`, `Info`, `Success`, `Warning`,
`Error`. The tone colors an alert's icon and a toast's title, which has
no icon. An `Error` alert also tints its
title and description; the other tones keep ordinary ink, so a page of
notices stays calm. An alert takes an optional action, placed at its top
right, and an optional dismiss button.

## Overlays

Floating surfaces. Each takes the element it is anchored to or laid over, and
an `open` flag the host controls. See [Animation](animation.md#what-animates)
for how they enter and leave.

| Component | Purpose |
| --- | --- |
| `Tooltip` | A hint for a control, after a short hover or at once with keyboard focus. `.delay_ms`, `.title_font_size`, `.title_font_family`, `.arrow` and `.background` change when it opens and how it looks. `Tooltip::for_path` names a file over the directory that holds it. When the control it describes is inside a larger anchor, such as a link in a paragraph, the hint points at that control, and at the part of it under the pointer when it runs over several lines. |
| `Popover` | A floating slot anchored to any element, for arbitrary content. On the element, `.popover_gap(px)` sets its distance from the anchor (5 unless set); 0 under a hairline anchor makes it stand in for the row that follows (an anchor of no height is not placed inside a scroll area). |
| `DropdownMenu` | A menu of actions with headings, separators, shortcuts and submenus, which nest: a submenu stays open while the pointer or the focus is in one nested in it. `.at(point)` opens it with its corner on a point of the window, such as where a secondary click was, instead of against its anchor; the arrow keys reach it wherever the focus is, and a secondary click outside closes it, as one outside any popover does. `.header(element)` stays above the options and `.footer(element)` under them, below a hairline, while they scroll. Entries beyond items: `DropdownMenuEntry::Message(text)` (a muted sentence), `Loading { id, rows }` (skeleton rows) and `Error { id, text }` (a failure in a scroll area of 104). |
| `DropdownMenuItem` | One action. `.enabled(bool)` or `.disabled()`, `.tooltip_with(TooltipSpec)` for a hint. `.checkbox(checked)` and `.radio(checked)` make it a check box or a radio item of the menu: a box or a circle at the end of the row, said to the platform as such and on or off, still reporting its ID. With `.description(text)` it is a two-line row as tall as the menu's `sizing` says (40 without one) that lines up with the menu's one-line rows: icon, label over a muted detail, such as why a disabled item cannot be chosen, and a check column filled when `selected`, or a chevron with a `.submenu`; `.detail_mono(true)` sets the detail in monospace, `.status(text)` puts short text at the end of the detail line (`.status_width(px)` fixes its slot) and `.chevron()` ends the row with a chevron instead of the check, for a row that opens another view. `.truncate()` cuts a long label to one line in a contextual menu, where it would otherwise wrap. |
| `DropdownMenuSection` | The heading of a group the user folds: `::new(id, label)` with `.icon`, `.count(n)` and `.collapsed(bool)`; it converts into a `DropdownMenuEntry` and reports `id` when activated. The host leaves the group's items out while it is collapsed. `.width(px)` fixes its width. |
| `MenuHeader` | The fixed head of a menu, passed to `DropdownMenu::header`: `.title(text)` or `.back(id, label)`, one square `.trailing(element)` tool, and `.search(id, query, placeholder)` for a compact search field under the row. `.width(px)` fixes its width. 28 high; the search field paints the palette's `raised`. |
| `MenuItem` | A menu row on its own, for a list the host lays out: icon, label and a shortcut or `.trailing` text, 26 high. `.description(text)` makes it the same two-line row as `DropdownMenuItem`, with `.detail_mono`, `.status`, `.status_width`, `.chevron` and `.selected(bool)`. `.enabled(false)` renders it passive. `.tooltip_with(TooltipSpec)` hints the row; `.trailing_action(id, icon, TooltipSpec)` adds a second 24 px action at the end, reached by Tab and not by the arrow keys, and `.trailing_dispatch_as(action)` makes it report another ID. With `.trailing_action`, the row answers the pointer over either target with one fill behind both, and the action adds a fill of its own, a shade over the row's. |
| `PopoverPanel`, `PopoverPanelSizing` | The content of a `Popover` that is more than a menu: `.header(element)`, `.body(element)` and `.footer(element)` stacked 4 px apart with hairlines between them. The body takes the height it needs and gives way first in a short window; `.body_max_height(px)` caps it, for a body that is a scroll area. The popover keeps placement, width and surface. `.sizing(PopoverPanelSizing { gap, padding, header_rule })` sets another rhythm: `DEFAULT` is 4, 0 and a hairline under the header. |
| `SuggestionList` | Completions floating over an input without taking focus: `::new(anchor, id, items, selected_index, open, dismiss_id, palette)`. A `SuggestionItem` is `::new(id, label)` with `.icon` or `.leading(element)`, `.description`, `.ink`, `.detail(text)` at the end of its line and `.section(label)` to start a section. The host moves `selected_index`; rows are 42 high, at most 7 show, and only those in view are built. `.title`, `.empty(text)` for when there are no items, `.hints(elements)` (they stack under 300 px), `.note(text)`, `.width(available)` (the list is 160 to 352 wide), `.placement` and `.animation_ms(open, close)`. |
| `DropdownList` | A list for choosing one option, with the current one marked. |
| `ContextMenu` | A menu opened by a secondary click or the menu key on its child. Its items are actions in one list; for submenus, check boxes, radio items or reasons, the host opens a `DropdownMenu` with `.at(point)`. |
| `Modal` | A centered dialog for forms, with a title, a scrolling body and a footer. `Modal::fit_width(viewport_width, max)` is the width that keeps the window's margin clear, and `Modal::body_width(modal_width)` the width a body can count on: inside the padding and the rail reserved while it scrolls. |
| `BottomSheet` | A controlled bottom-edge dialog with a 44-point header/close target, scrolling body, fixed footer and optional safe-area padding. `BottomSheetDrag` interprets its in-place downward dismissal gesture. See [BottomSheet](components/bottom-sheet.md). |
| `AlertDialog` | A centered confirmation for consequential actions. `.alternative(id, label)` adds a second way to go ahead between Cancel and the confirm action, for the same step without its consequential part. |
| `ImageViewer` | A full-window image gallery. `.zoom(level)` shows the picture that many times larger than its fit to the window (1 to `ImageViewer::MAX_ZOOM`), panned like a diagram: the wheel moves it down, a drag or a sideways gesture moves it across. `.zoom_actions(out, fit, in)` adds the three controls to the header; the host owns the level and answers them. |
| `CoachMark` | A teaching bubble with a caret towards a control that already exists, named by its ID, and a ring around it. It reports close, next, back and skip; the rest of the window takes no input while it shows. See [Guided tours](tours.md). |
| `Tour`, `TourStep` | A sequence of coach marks with a step counter. The host owns the current step; the bubble travels from one target to the next. |

`Modal`, `BottomSheet`, `AlertDialog`, `ImageViewer`, `CoachMark`, `Tour` and `Toast::layer`
wrap the window's root element, since they cover the whole window.

## Text

| Component | Purpose |
| --- | --- |
| `Text` | Passive text in a role of the type scale and a color of the palette: `.role`, `.muted`, `.ink` for a semantic color, `.mono`, `.truncate`, `.fade`, `.line_height`, and `.tooltip` to show the whole value of truncated text, wrapped. `Text::eyebrow` is the small monospaced label above a group. `.font_size(px)` is for a step the host's design has between two roles, such as 10.5; prefer a role. |
| `Markdown` | Markdown source as wrapped prose with links, lists, quotes and code blocks; see [Markdown](#markdown). |
| `TextLayout` | Text limn shapes, breaks into lines and places itself: `TextLayout::new(text, TextStyle::new(size, family))`, `.wrapped(width)` to break it again without shaping, `.size()`, `.lines()`, `.caret(index)`, `.index_at(point)`, `.selection(range)`, and `.draw(origin, color)` for the `DrawCommand::Glyphs` a backend paints. `TextStyle::leading` places the glyphs in a tall line; `.centered(width)` centers each line. The faces come from `limn::set_fonts(Fonts)`, `Fonts::bundled()` by default. A tree with `set_text_layout(true)`, as `limn_egui::attach_fonts` makes it, lays every `Element::text` out in one; the other components still measure through the backend. |

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
three levels, paragraphs, bulleted, numbered and checked lists, quotes,
subtext lines (`-# `), fenced code and rules, with strong, emphasis, bold
italic, underline (`__x__`), strikethrough (`~~x~~`), inline code and web
links. Use it for text that arrives as Markdown, such as a message,
including while it is still being written.

### Example

```rust
let preview = Markdown::new(source, palette).width(reading_width).render();
// preview.element goes in the tree; preview.limited says the source was cut.
```

### Options

| Builder | Effect |
| --- | --- |
| `.width(px)` | The reading column, kept between 48 and 1,200; 600 unless set or when not finite. |
| `.font_size(px)` | The body text, 14 unless set; headings, code, list markers, line heights and the space between blocks keep their proportion to it. Kept between 8 and 96; a size that is not finite and positive is ignored. |
| `.colors(MarkdownColors)` | The ten inks and fills: `text`, `strong`, `emphasis`, `link`, `code_ink`, `code_fill`, `muted`, `success`, `panel`, `border`. The default, `MarkdownColors::for_palette`, reads the palette and paints strong text like body text, in the bold face. |
| `.streaming(bool)` | The source is still growing: an unclosed delimiter at its end already styles its text, a half-written fence is held back, and a web address at the end has no action until it is delimited. |
| `.link_action_prefix(text)` | The start of every link action; `markdown_link:` unless set. |
| `.code_syntax_colors(CodeSyntaxColors)` | The inks of fenced code: `text`, `keyword`, `string`, `number`, `comment`, and, when set, `function` (a name before `(`) and `interpolation` (an f-string's or a template's braces). `CodeSyntaxColors::for_palette` unless set. |
| `.inline_code_size(px)` | Inline `code` in body text, the body size unless set; in a heading or subtext it keeps that proportion. |
| `.code_label(bool)` | The language line above fenced code; shown unless set to `false`. |
| `.code_fit_content(bool)` | Each code block as wide as its code and padding, up to the reading width; the whole width unless set. |
| `.code_copy_action_prefix(text)` | A copy icon at the end of each code block whose action is the prefix followed by the code; none unless set. |
| `.labels(MarkdownLabels)` | `code_fallback`, the label of a code block with no language; `truncated_note`, the muted line under a cut preview; `diagram`, the label of a drawn diagram; `show_source` and `show_diagram`, the action that switches a diagram to its source and back; `expand`, the action that opens a diagram over the window; and `copy_code`, the hint of a code block's copy icon. English unless set. |
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
`CodeBlockSizing::CHAT` on the palette given to `new`, its code colored as
described under [Highlighted code](#highlighted-code). With another
`font_size`, each of these numbers but the paddings, the bar and the radii is
multiplied by `font_size / 14`, and so is the text of a code block and of a
diagram's card.

### Highlighted code

A fence whose language the line lexer knows has its comments, strings,
keywords and numbers colored; the rest of the code keeps the palette's
`text`. The language is read from the fence's first word with
`CodeLanguage::from_code_fence`, in any case:

| Language | Fence labels |
| --- | --- |
| Rust | `rust`, `rs` |
| JavaScript | `javascript`, `js`, `jsx`, `mjs`, `cjs`, `node`, `nodejs` |
| TypeScript | `typescript`, `ts`, and `tsx` for TSX |
| Python | `python`, `python3`, `py`, `pyi` |
| JSON | `json`, `jsonc` |
| TOML | `toml` |
| CSS | `css`, `scss` |
| HTML | `html`, `htm`, `xml`, `svg` |
| Markdown | `md`, `markdown` and their variants |

The lexer runs while the tree is built, with or without the `syntax`
feature, so the colors are there on the first frame. On a dark palette the
inks are the ones `CodeEditor` and `CodeDiff` give the same four kinds, with
a lighter comment; on a light one, judged by the palette's `border` as
`ComponentPalette::link` is, they are darker counterparts. Each is at least
4.5:1 on dark fills up to `rgb(53, 53, 56)`, the raised fill of
`Theme::DARK`, and on light ones down to `rgb(220, 220, 220)`. They do not follow
`MarkdownColors`. Code stays plain text, as before, when its fence names no
language or one not in the table, when nothing in it takes a color, and when
it is longer than 64 KiB or 2,000 lines. The copied and announced text is the
code as written.

### Notes

- The component stores nothing. Rendered trees are kept by an engine cache on
  the UI thread: 64 complete sources within 4 MB, and the last two streaming
  ones within 512 KB, so an append reuses every block above the one that
  changed. A tree is reused only for an equal source, width, text size, palette,
  colors, labels, prefix and limits.
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
| `Element::drawing` | What the application draws itself, in the frame's own commands: `Element::drawing(|size| commands)` is given the size it is laid out at and returns `DrawCommand`s placed from its top-left corner. The tree moves them into place, clips them to the box, and keeps the drawing's own clips balanced. Give it a size: alone it takes no height. Passive; wrap it in `Element::clickable` to press it. |
| `ResizeHandle` | The strip between two panes that the pointer drags to move their boundary: `ResizeHandle::vertical(id, palette)` stands between two columns and fills their row's height, `ResizeHandle::horizontal` lies between two rows. `.thickness` (8, never under 2) is the room it takes and its pointer target, so the gap between two panes can be the handle itself. It draws nothing at rest, fills with the palette's `hover` under the pointer and with `selected` while held, and the pointer takes the column- or row-resize shape over it and through the drag (`Element::cursor`, which a host applies with `limn_egui::set_cursor(ctx, tree.cursor())` after the layout). It is controlled: it reports `DragStarted`, `DragMoved` and `Dropped` or `DragCancelled` on its ID with the pointer's position, and the host adds the pointer's travel to the size the pane had when the drag began and keeps it within its own limits. It has no keyboard resize. |
| `SplitView` | A row or a column of panes with a `ResizeHandle` between each pair: `SplitView::row(id, panes, palette)` or `SplitView::column`, each `Pane::new(element)` with `.fraction(f)` (its share of the room the handles leave) or `.flex()` (what the others leave), clipped to its pane; `.handle_thickness(px)`. It is controlled: the handle after pane `i` reports the drag on `{id}:handle:{i}`, and the pure `SplitView::resized(fractions, handle, delta_px, total_px, mins)` gives the host the new fractions, keeping the panes at their minimums. See [SplitView](components/split-view.md). |
| `ScrollArea` | A scrolling viewport, or a virtualized list of uniform rows or of rows measured at their own heights (`measured_list`, with `follow_end` for a conversation). The host scrolls it through the tree: `scroll_to_start`, `scroll_to_end`, `scroll_to_index` for a uniform list, and `reveal(control_id)` or `reveal_with_margin` to bring any control inside it into view; see [Layout](layout.md#scrolling-from-the-host). A child of the column inside it can stay pinned at the top while its section scrolls: `.sticky_top()`, `.sticky_top_inset`, `.sticky_top_max_height`, `.sticky_top_fade`, and `.sticky_top_background_fade_in(from, to)` for a header whose background fills in as the page scrolls under it; see [Layout](layout.md#sticky-children). |
| `PageColumn` | A scrolling page whose content is a centered column: `PageColumn::new(scroll_id, available, palette)`. `.max_width` (960), `.min_width`, `.gutter(narrow, wide, breakpoint)` (14 and 28 at 640), `.gap` (10) and `.reserve_rail`, which keeps a scrollbar rail's width out of the column before the page overflows. `.tint_scrollbar(false)` leaves the scrollbar the scroll area's own color instead of the palette's muted. Read `.content_width()` to size the content, then pass it with `.children`. It fills its parent. |
| `PageHeader`, `PageHeaderSizing` | The heading of a page: a title, a `.description`, and a `.leading` and a `.trailing` element centered on it. `.stacked` moves the trailing element under the copy, `.trailing_top` keeps it on the title's line instead of centering it, `.truncate` cuts both lines instead of wrapping them. The profile holds the two font sizes, the three gaps and an optional fixed height; `PageHeaderSizing::DEFAULT` is a settings page. |
| `SectionHeader`, `SectionHeaderSizing` | The heading of a section: a title, a `.count` chip right after it, a muted `.detail` and a `.trailing` element at the far end, and a `.disclosure` chevron before it. With `.action` the whole row is a button. `.width` fixes its width and `.truncate` cuts the title to one line. Profiles: `TITLE` (a page section), `LIST` (a list heading) and `LABEL` (a quiet label in a side panel, hung from the top inset). |
| `PanelHeader`, `PanelHeaderSizing` | The header row of a panel: a title cut to one line that takes the width left, `.trailing` tools after it, a `.back(id, tooltip)` arrow before it and a `.dismiss(id, tooltip)` close button at the end, both with hints that open downward. `.leading` puts an element in place of the title, such as tabs, a breadcrumb or a field. `.width` fixes the width. It paints no fill. Profiles: `SIDEBAR` (26 high), `PANE` (32, inset 4), `CARD` (28, inset 2, a body-sized title) and `POPOVER` (24); `center_tools: false` hangs the tools from the top inset instead of centering them. A rule between two groups of tools is a `Separator` with a length in the vector. |
| `SidebarPanel` | The body of a sidebar: its blocks in a column with an inset and a gap of 6, on the palette's `surface`. `.size(width, height)` fixes it. |
| `SideRail`, `SideRailItem` | A sidebar's library folded into a column 72 wide (`SideRail::WIDTH`): one 48 px picture for each item (`.picture`, `.round` for a person), whose hint opens to its right at once with its title and a muted `.subtitle`. `.expand(tooltip)` heads it with a 40 px button that reports `{id}:expand`, `.create(tooltip)` adds one that reports `{id}:create`, `.current(id)` keeps the page shown filled and `.playing(id)` marks what plays. Its items scroll under the head. No text is drawn; each item is a button named by its title. |
| `PeekPanel`, `Side` | What is left of a closed side panel: a strip 40 wide (`.width`) at the window's edge (`.side`, right unless set), rounded where it faces the window, that shows the panel's content laid out at `.content_width` behind a veil of 0.9. Under the pointer or the focus it reaches 12 px further out on a spring that overshoots, the veil lifts and its chevron lights up. It reports a click, Enter or Space on its ID; the host opens the panel. Name it with `.accessibility_label`. |
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
| `Element::row(children).wrap()` | Moves the children that do not fit the row's width onto more lines, each as tall as its tallest child, and grows with them: tags, role pills. `.line_gap(px)` spaces the lines, `gap` until set. See [Layout](layout.md#wrapping-rows). |

## Data

| Component | Purpose |
| --- | --- |
| `Table`, `TableColumn`, `TableRow`, `TableSizing`, `TableSort` | Rows of cells under a header of column labels: columns that sort, resize, hide and drop out of a narrow table, and a header that stays pinned while the rows scroll; see [Table](#table). |
| `LineChart`, `AreaChart`, `PieChart`, `RadarChart` | Lines over the points of an axis, with the area under them filled or not; a pie or a ring of slices; and series as shapes over axes that spread from a center. Drawn in as they appear, and read by the pointer with the same tooltip as the bar charts. |
| `BarChart`, `ColumnChart` | Bars over a value axis, lying or standing: one for each datum, or several `ChartSeries` side by side or stacked, with a gradient if asked. They grow as the chart appears, and under the pointer a category is lit and listed in a tooltip that follows it. |
| `HeatmapChart` | A grid of cells colored by value. |
| `ChartLegend` | A legend for any of the charts. |
| `DescriptionList`, `DescriptionItem` | Pairs of a label and its value, for the details of one thing: `DescriptionList::new(items, palette)` with `DescriptionItem::new(label, text)` or `::with(label, element)` for a chip, a link or a copy button, which keeps its own behaviour. The muted label stands beside its value in a column of `.label_width` (120), or over it with `.stacked()`; `.columns(n)` (1 to 4) with `.span(n)` on an item, `.bordered(true)` for an outlined surface ruled between rows, columns and label and value, `.dividers(false)` for a plain list without rules, `.size(ControlSize)`, `.width`. Passive: no action, no tab stop, no node of its own. See [DescriptionList](components/description-list.md). |
| `DiffStat` | The lines a change adds and removes, as `+N` and `−M`. `.inks(added, removed)` takes the host's two colors; without them both counts are plain text. |

## Table

Rows of cells that line up under a header of column labels, for a list the
user sorts, filters and reshapes, such as the tracks of a playlist. The host
owns the rows and every choice; the table lays them out and reports what was
asked.

### Example

```rust
let text = |value: &str| Text::new(value, palette).truncate().render();
let columns = vec![
    TableColumn::fixed("index", "#", 24.0).align_end(),
    TableColumn::flexible("title", "Title", 160.0).sortable(),
    TableColumn::fixed("album", "Album", state.album_width)
        .sortable()
        .resizable(80.0)
        .optional(state.album_shown)
        .hide_below(460.0),
    TableColumn::fixed("length", "Length", 56.0).icon(Icon::Clock).sortable().align_end(),
];
let rows = state
    .tracks
    .iter()
    .map(|track| {
        let cells = vec![text(&track.number), text(&track.title), text(&track.album), text(&track.length)];
        TableRow::new(format!("track:{}", track.id), cells).selected(track.id == state.playing)
    })
    .collect();
let table = Table::new("tracks", columns, width, palette)
    .rows(rows)
    .sort(state.sort.clone())
    .sticky_header()
    .sizing(TableSizing::ROOMY)
    .render();
```

`crates/limn-gallery/src/pages/table.rs` is a whole host: it filters, sorts,
keeps the widths and toggles the columns.

### Options

| Builder | Effect |
| --- | --- |
| `Table::new(id, columns, width, palette)` | A header alone. `width` is the width the table is given: it decides which columns fit and how wide the flexible ones are. |
| `.rows(rows)` | The rows to show, in order. The host filters and sorts them. |
| `.sort(Option<TableSort>)` | The sorted column: its label takes the text color and an arrow after it, up for ascending and down for descending. |
| `.sticky_header()` | Pins the header at the top of the scroll area around the table while its rows scroll under it, filled with the palette's `surface`. |
| `.columns_menu(open, title)` | Adds a button at the end of the header, with `title` as its hint, that opens the list of optional columns. Rows leave its slot empty. |
| `.menu_palette(palette)` | Colors the columns menu and the header's context menu; the table's own palette unless set. |
| `.radius(RadiusRole)` | Rounds the fill of a row; `Control` unless set. |
| `.sizing(TableSizing)` | The density: both heights, the gap between rows, the side inset, the gap between columns, the role of the labels and the gap before the arrow. `COMPACT` (rows of 28 under a header of 28) unless set; `ROOMY` is 56 under 36, for rows with artwork and two lines. |
| `.into_parts()` | In place of `.render()`: the header and one element per row, for a host that wraps each row in a `ContextMenu` or a `hover_region`. Stack them in one column: the first row `header_gap` under the header, the rows `row_gap` apart. |

A column:

| Builder | Effect |
| --- | --- |
| `TableColumn::fixed(key, label, width)` | A column `width` wide. `key` names it in the actions. |
| `TableColumn::flexible(key, label, min_width)` | A column that shares the width the fixed ones leave, never under its minimum. Several share it equally. |
| `.icon(icon)` | Shows a glyph in the header. The label becomes its hint and stays its name in the columns menu. |
| `.sortable()` | Makes the header a button. |
| `.resizable(min_width)` | Lets the boundary beside a fixed column be dragged; a short rule in the header marks it. The column is never drawn under `min_width`. A flexible column ignores it. |
| `.align_end()` | Sets the label and the cells against the end of the column, for figures. |
| `.optional(shown)` | Lists the column in the columns menu, checked while shown. A column that is not shown is left out. |
| `.hide_below(width)` | Leaves the column out while the table is narrower than `width`. |

A row:

| Builder | Effect |
| --- | --- |
| `TableRow::new(id, cells)` | One cell per column, in the order the columns were given, those not shown included. A missing cell is empty. |
| `.selected(bool)` | Fills the row with the palette's `selected`: the chosen row, or the one that plays. |
| `.controls(ids)` | The controls inside its cells. The row stays lit while the pointer is over one of them. |

### Actions

| Action | When |
| --- | --- |
| `Clicked` on `{id}:sort:{key}` | The header of a sortable column is activated. `TableSort::toggled(current, key)` gives the sort it leaves: ascending, then descending, then none. |
| `DragStarted`, `DragMoved`, then `Dropped` or `DragCancelled`, on `{id}:resize:{key}` | The boundary of a resizable column is dragged, as a `ResizeHandle` reports it. Enter on a focused boundary reports `Clicked` on the same ID, which the host may ignore. The host adds the pointer's travel to the width the column had when the drag began and passes the new width to `TableColumn::fixed`. |
| `Clicked` on `{id}:column:{key}` | An optional column is toggled, in the columns menu or in the header's context menu. |
| `Clicked` on `{id}:columns`, and on `{id}:columns:dismiss` | The columns button is activated; its menu is dismissed. |
| `Clicked` on a row's ID | The row is activated. A control in a cell reports its own ID instead. |

A column before the first flexible one has its handle on its far edge and
grows as the pointer goes right. A column after it has the handle on its near
edge and grows as the pointer goes left. The flexible column takes up the
change, so the boundary follows the pointer.

### States

- **Sorted.** The label of the sorted column is in the text color, with its
  arrow. The others are muted, and one that sorts takes the text color under
  the pointer.
- **Selected.** The row is filled with the palette's `selected`, at rest and
  under the pointer.
- **Empty.** With no rows the table is its header. Put an `Empty` under it.
- **Narrow.** Columns leave at their `hide_below`. If what is left still does
  not fit, resizable columns give width up in order down to their minimum,
  then every column shrinks alike. Nothing is painted outside the table.

### Keyboard

| Key | Effect |
| --- | --- |
| Tab | Moves through the sortable headers, the handles and the columns button, then the rows and the controls in their cells. |
| Enter, Space | Activates the header, the button, the row or the control in focus. |

A boundary has no keyboard resize.

### Motion

A row's fill and a label's ink answer the pointer over `hover_ms`. The
columns menu enters and leaves as any menu does.

### Notes

- Pass the width the table really gets. A scroll area that keeps a rail for
  its bar gives its content that much less: subtract the rail (14 unless
  set), or give the scroll area `ScrollbarAppearance::overlay()`.
- A cell hugs its content at the start of its column, or at its end, and
  gives width up when the column is narrower: a `Text` with `.truncate()` is
  cut with an ellipsis. A cell the host made `.flex(1.0)` fills its column.
  Cells are centered in the row's height and clipped to the row.
- A resizable column is drawn as wide as the host says only while that fits,
  so keep what you store within limits of your own; the table does not report
  the width it drew.
- A sticky header pins while its table is in view and leaves with the
  table's last row, so a page can hold more under the table.
- Every row is built on every frame, and rows out of view are not painted.
  There is no virtualized table.

## Code

| Component | Purpose |
| --- | --- |
| `CodeEditor` | An editor view over a host-owned text buffer: selection, folding, diagnostics, breakpoints, optional Vim mode. |
| `CodeDiff` | A virtualized, read-only diff: changed words, folded rows that ask the host for their lines, configurable `DiffColors`. |
| `DiffFileHeader` | The bar above a file's diff: a chevron that reports its ID, the path with the name emphasized, and `+N −M`. |
| `TerminalView`, `TerminalScreen`, `TerminalCell`, `TerminalCursor`, `TerminalCursorShape` | A terminal's screen and scrollback, painted from cells the host's emulator resolved: `TerminalView::new(id, screen, palette)` with `.font_size`, `.background` and `.min_contrast`. It takes the focus under `id` for the host to read the keys, selects text by dragging for `CopyRequested`, blinks its block or bar cursor on the caret clock, and scrolls as `{id}:scroll`. `TerminalView::grid_size(size, font_size)` gives the columns and rows that fit. |
| `FindBar`, `FindOptions`, `FindIds`, `FindLabels` | Find and replace for a `CodeEditor`, at most 520 wide. Controlled: the host passes the query, `.replacement`, `.replace_open`, `.options`, `.count_label`, `.has_matches`, `.has_current`, `.no_matches` and `.error`, and answers the actions of `FindIds` (derived from the bar's ID, or the host's own through `.ids`). `.labels` takes every string, `.ink` the failure color, `.replace_icons` the two replace glyphs and `.surface` a second palette for the bar under the controls. Below 480 wide, or with replace open, the options take their own row. `render()` gives the bar; `.attach(anchor)` floats it under the anchor without taking the pointer or the focus. `FindBar::height(width, replace_open, error)` is the height it lays out. |
| `CodeLanguage` | The languages with a grammar. `from_extension`, `from_path` and `from_code_fence` choose one; `comment_markers_for(path)` gives the markers that comment out a line, falling back on the file's name (`#` for shell, YAML, Nix, Ruby, Makefile and Dockerfile) when the language has none. These need the `syntax` feature; without it the enum keeps its variants and `from_code_fence`, which reads the same labels. |

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

To draw the semantic icons from another set, give the backend a pack: a
function from an `Icon` to the SVG that draws it, or `None` to leave that one
to Bootstrap. Call it once, when the window is created:

```rust
fn lucide(icon: Icon) -> Option<&'static str> {
    match icon {
        Icon::Search => Some(include_str!("../assets/lucide/search.svg")),
        Icon::Trash => Some(include_str!("../assets/lucide/trash-2.svg")),
        _ => None,
    }
}

limn_egui::set_icon_pack(lucide);
```

The artwork must draw with `currentColor`, as `Icon::Custom` does, and is
fitted to the icon's square. Components keep asking for `Icon::Search`; only
what it looks like changes.

`icon.filled()` returns the solid form of an icon, for the chosen item of a
navigation list; an icon without one is returned unchanged.

`Element::icon(Icon::...)` paints a semantic icon at 16 px. `.large_icon(px)`
draws a standalone icon larger, for empty states and thumbnails, and
`.small_icon(px)` smaller, down to 8, such as a mark over another icon.
`file_icon(path)`, `folder_icon(name, expanded)` and `language_icon(name)`
return the artwork for a file type.

## See also

- [Gallery and hot reload](gallery.md) to see and try each component.
- [Contributing a component](contributing.md) to add one.
