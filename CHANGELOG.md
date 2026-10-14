# Changelog

Every release of limn, newest first. Versions before 1.0 may change the API
in any release.

## Unreleased

- `ScrollArea::measured_list` and `Element::measured_list`: a virtualized list of rows of their own heights, such as the messages of a conversation. Rows are built only while in view and measured then; a key per row keeps its height, and holds the row at the top of the viewport still while rows above it are measured, change height or are added. `follow_end` (`Element::list_follow_end`) keeps the end in view as rows arrive while the list is scrolled there. `reveal` and `anchor_offset` reach what the rows in view hold
- `UiTree::visible_rows(id)`: the rows of a uniform or a measured list built at the last layout
- `ScalarTransition::shift`: moves a transition's start and end by the same amount, keeping its pace
- `UniformList` has a public `measured` field; code that builds the struct literally adds `measured: None`
- `TextArea::enter_submits`, `Composer::enter_submits` and `Element::text_area_enter_submits`: Enter sends and Shift+Enter breaks the line, as in a chat; Ctrl+Enter sends either way. Off by default
- Icons for conversations and calls: `Icon::MicrophoneMuted`, `Headphones`, `Emoji`, `Reply`, `At`, `Phone`, `HangUp`, `Screen` and `VideoOff`, drawn with Bootstrap Icons 1.13.2
- Icons for servers and settings: `Icon::Lock`, `Neutral`, `ChevronUp`, `Invite`, `Leave` and `DragHandle`, drawn with Bootstrap Icons 1.13.2 (`lock-fill`, `slash-lg`, `chevron-up`, `person-plus`, `box-arrow-left` and `grip-vertical`), with a section on the gallery's Icon page
- Markdown: `font_size` sets the body text, 14 unless set. Headings, inline and fenced code, list markers and their column, line heights and the space between blocks keep their proportion to it; a diagram's drawing keeps its own size. A tree cached at one size is not reused at another
- Tabs: `variant(TabsVariant::Underlined)`, text tabs with no track and an underline that slides to the active one over a hairline, the same size as the segmented tabs, with the same actions and keys; `accent` colors the active label and the underline, and `gap` sets the space between tabs
- Tabs: a disabled tab's label sits at the height of the enabled ones, where it stood higher
- `Tabs` has public `variant` and `accent` fields; code that builds the struct literally adds `variant: TabsVariant::Segmented, accent: None`
## 0.1.42 — unreleased

- `Element::wrap` and `Element::line_gap`: a row whose children move onto more lines when they do not fit its width, as tags or role pills do. The row is as tall as its lines; Tab, the pointer and assistive technology find each child where it is, in the order given
## 0.1.42 — unreleased

- Switch: `size(ControlSize)`, from 28×16 for `Dense` to 44×24 for `Spacious`; `variant(SwitchVariant::Accent)`, a light thumb on and off over a track of the palette's link color, or of `ink`, darkened where needed until the thumb reads on it at 3:1, as chat applications draw a switch; and `ink` for the track's color while it is on. A switch without them draws what it drew
- Slider: `marks()` draws a hairline at each stop, up to 100 stops, `mark_labels` names the stops over the track, each on one line, and `default_stop` shows one of them in the ink; the platform is told the label of the stop the value is on
- RadioOption: `icon`, between the circle and the label
## 0.1.42 — unreleased

- TextField: `icon`, an icon before the text inside the field, muted until the field has the focus, as a search field's magnifier; a disabled field keeps it in its place
- SearchField: `clearable(true)`, a button at the end of the field while it has a query, which reports `Clicked` with `"{id}:clear"`; the text stops before it, and disabled it stays in its place, passive. `Labels::clear_search` names it. A clearable field is as wide with a query as without, never under 80
- SearchField: a disabled field shows its shortcut hint only while it has no query, as the enabled one does
- Select: `variant(SelectVariant::Field)`, a trigger drawn as a text field, with its fill, border and height and the choice at its start, open or focused from the keyboard with a focused field's border; `size(ControlSize)` and `height(px)` for either variant. A select is drawn as it was unless told otherwise
- `Element::focus_border` on a control whose focus ring is off is shown while it has the keyboard focus, as its mark of that focus; a text input or area shows it on any focus, as before
## 0.1.42 — unreleased

- New components: `SwatchPicker` and `Swatch` (one color among a few, a ring around the chosen one, and an optional custom swatch that opens a picker) and `ColorPicker` with `Hsv` (a field of saturation and value, a bar of hues and a hex field; `ColorPicker::color_after` turns its actions into the next color, `hex` and `parse_hex` write and read it), each with a page, a guide and the swatches in the controls harness
- `Element::area_slider` and `UiAction::AreaSliderChanged`: a slider over an area, dragged in two directions and stepped with all four arrows
- `Element::horizontal_gradient` and `vertical_gradient`: evenly spaced colors across an element, its corners rounded
- Gallery: a field shows the text a slider leads to, as it does after a click
- Labels: `saturation_brightness`, `hue` and `hex`, what a color picker calls its parts to the platform

## 0.1.41 — 2026-10-13

- New components: `Accordion` and `AccordionItem`, `Pagination` (numbered pages that keep their place as the page changes), `DescriptionList` and `DescriptionItem`, `AvatarGroup` and `Toolbar` (groups of commands with rules between them, and a menu for what does not fit)
- Chip: `tone` and `color`, `size`, `icon` and `dot`, and `remove_action` with `enabled`/`disabled`; a plain chip draws what it drew
- Button: a disabled button of a fixed width keeps its label in the middle, where the enabled one has it
- Documentation site: a component is marked "New" in the release it comes in and the one after, from the `since` of its gallery page
- New components: `Checkbox` (with an indeterminate state), `RadioGroup` and `RadioOption`, `Toggle` and `ToggleGroup`, `NumberInput` (a field with buttons that step it, between a minimum and a maximum) and `Badge` (a count or a dot over the corner of something else), each with a page, a guide and a place in the controls harness
- Accessibility: `Access::mixed` for a check box that is neither on nor off, published as such
- `Element::every_arrow`: a tab strip that Up and Down step as Left and Right do, which a radio group uses
- HeatmapChart: its cells are a plot the tree draws (`Element::heat_plot`). The columns come in one after another as the chart appears; the cell under the pointer grows over its neighbours while the others recede, and says what it is in the charts' tooltip, which follows the pointer

## 0.1.40 — 2026-10-13

- Charts: `LineChart` and `AreaChart`, `PieChart` with `donut` and `center`, and `RadarChart`, drawn in as they appear and read by the pointer with the charts' own tooltip, which stays inside the window; `Element::line_plot`, `pie_plot` and `radar_plot` for a chart of the host's own, and `DrawCommand::FillMesh` for a backend to draw
- Charts: `BarChart` and `ColumnChart` are drawn by the tree. Several series with `ChartSeries`, side by side or `stacked()`, negative values, `gradient(true)`, rounded tips, a dashed grid and a value axis with round ticks; the bars grow from the zero line as the chart appears, and under the pointer a band lights the category, the others recede and a tooltip of the chart's own follows the pointer with a row for each series
- Charts, changed: a `ChartDatum` keeps a negative value, a column chart no longer scrolls sideways, `ChartDatum::tooltip` is the name of the bar's row in the tooltip, and `ColumnChart::integer_ticks` is deprecated
- `Element::bar_plot` with `BarPlot`, `BarSeries` and `BarValue` for a chart of the host's own, and `DrawCommand::FillHorizontalGradient` for a backend to draw
- Gallery: a picker's field shows the day chosen or the time stepped; it kept the text it had
- TimePicker: a card under the field with a column for each unit: the arrow that raises it, large digits, the arrow that lowers it and its name; the actions it reports are unchanged
- DatePicker: a day's number stands in the middle of its cell, so the fill of the chosen day is around it
- A background fade follows its element's rounded corners: a `MediaCard` showed square corners at its foot
- `Radius::ROUND`: controls as pills, and surfaces, cards and panels to match
- Kbd: a caption and the slash between alternatives stand on the keycaps' middle line; they hung from the top
- Gallery: one section or one variant of a page alone, as `&section=` and `&variant=` in its address, centered in what it is given
- Documentation site: a new landing, every example of a component on a card of its own with its code, branches in the sidebar, a search opened with Ctrl+K, one header on every page, and no gallery page

## 0.1.39 — 2026-10-13

The first release on crates.io.

- `CodeEditor` without the `syntax` feature, as in a browser, colors its buffer with the line lexer; it panicked on the first frame
- Everything public in the five library crates has a doc comment, fields and variants included; the crates carry `#![warn(missing_docs)]`
- Accessibility, read from AT-SPI as a screen reader gets it (`tools/atspi/run.sh`): `limn_egui::name_window` names the window; a date or time picker's field is called "Date" or "Time", or what `accessibility_label` says, and a day by its whole date; `Spinner::decorative` keeps a spinner inside a control out of the list, and the library's own nested spinners use it
- Labels: the texts that were still fixed now come from `set_labels`: the pickers' placeholders, an alert's default titles, the find bar, Markdown's block labels, the code review hints and the editor's status line; `MarkdownLabels::default()` follows them
- `limn-language`: every error message is in English
- `Element`: `padding`, `gap`, `flex`, `size`, `min_size` and `radius` ignore a number that is not finite; `border` after `dashed_border` is solid again
- `ChartLegend` cuts its labels to the width it is given
- `CodeEditor::tab_width` takes up to 32 columns, as `indent_style` does
- `Composer::compact` never did anything and is deprecated
- `limn-language`: a parse worker no longer spins when its queue's lock is poisoned
- Gallery pages and guides for `DatePicker`, `TimePicker`, `DateTimePicker`, the charts, `CodeEditor`, `CodeDiff`, `Attachment`, `Avatar`, `ImageViewer`, `TreeRow`, `FramedCard`, `ScrollArea`, `Skeleton` and `TextShimmer`
- Tests: one harness holds 51 interactive controls to the same facts (one action for a click, for Enter and for Space; reached by Tab; inert, in place and announced as such when disabled), and `Attachment`, `Avatar`, `ChartLegend`, `Link` and `MenuSeparator` have tests of their own
- MenuBar: a disabled menu's accessibility node covers its whole trigger, not only its label
- Keyboard: Page Up and Page Down scroll what the wheel scrolls, the image viewer steps with the arrow keys, and a popover takes them with `Element::popover_arrows`; `limn_egui::prefers_reduced_motion` reads the browser's preference for less motion
- Renamed, with the old names kept and marked deprecated: `RowSizing` is `ListRowSizing`, `HeaderSizing` is `PanelHeaderSizing`, `SectionSizing` is `SectionHeaderSizing`, `Link::color` is `Link::ink`, `Text::size` is `Text::font_size`
- Accessibility: every frame lists its controls for the platform (`Frame::access`) with a role, a name and a state; `Element::accessible` and `accessibility_label` say what the tree cannot work out, and `limn-egui` publishes it through AccessKit and carries out what a screen reader asks with `apply_access_requests`
- Sixteen more components can be disabled: `Link`, `Clipboard`, `DismissButton`, `Marker`, `Collapsible`, `DatePicker`, `TimePicker`, `DateTimePicker`, `Step`, `ActionRow`, `ResizeHandle`, `TabItem`, `MenuBarMenu`, `SuggestionItem`, `BreadcrumbItem` and `Composer`
- Every component that can be disabled takes both `disabled()` and `enabled(bool)`: twelve had only one
- Sizes that are not finite are ignored by `Tab::new`, `Tabs::height`, `SegmentedControl::height`, `CodeEditor::height`, `CodeDiff::height`, `TreeRow::width`, `MenuSeparator::new` and `Toast::layer`
- Hot reload is part of the library: `limn::hot` (`connect`, `call`, `on_patch`, `remember`, `restore`) and `hot_shape!`, which notices a patch that changes a state's fields, behind the `hot` feature, and `limn_egui::repaint_on_patch`; the gallery uses them
- Gallery: English labels, captions, menus, tooltips and sample content throughout
- Kbd: width budgets cover alternatives and captions together; narrow rows omit the whole hint
- Built-in labels default to English and can be replaced with `set_labels`; `Labels::SPANISH` preserves the earlier Spanish copy
- Toast cards can use explicit IDs; breadcrumb search hints accept a custom shortcut
- Documentation site: live components, native Mermaid diagrams and guides for every component
- Documentation site: a new interactive landing, separate design-system foundations and patterns, and preview/code tabs
- Docs: liquid glass, where its numbers come from and what is ours
- Gallery: a page for glass, with a bar to drag over six backdrops
- Glass: a material that refracts the scene behind a surface, and a navigation bar on it
- License limn under MIT or Apache-2.0
- Table: room under the header's line, and a rule where a column is dragged
- Gallery: a page for the table
- Table: rows of cells under a header that sorts, resizes and hides columns
- A background fade from the top edge
- Decode images off the UI thread
- Stack: children one over another, and a layer that hides under the pointer
- Level bars rise fast and fall back slowly
- Soft focus for text and a smooth centring in a scroll area
- A resize handle stays where the host keeps the boundary
- DictationWaveform: the microphone's level as bars on springs
- Modal: the width of a body that fits without scrolling
- Credit the card's arrangement
- A record that comes out of its sleeve and turns, and a card around it
- The pointer takes the resize shape over a resize handle
- A resize handle
- A media tile, and images that fill their box
- A sliders icon
- A slider, and the icons a media player needs

## 0.1.38 — 2026-10-13

- The stepper is a row of badges joined by rules, or a column of them

## 0.1.37 — 2026-10-13

- A control or an anchor can be scrolled to the start of its view

## 0.1.36 — 2026-10-13

- A status cell can stand out, color one glyph and be divided from the next

## 0.1.35 — 2026-10-13

- A control can be an anchor too

## 0.1.34 — 2026-10-13

- A passive element can be a place to scroll to

## 0.1.33 — 2026-10-13

- A conversation minimap and a clipboard button

## 0.1.32 — 2026-10-12

- A menu row's action has a hover of its own inside the row's

## 0.1.31 — 2026-10-12

- A row of a uniform list opens content under itself

## 0.1.30 — 2026-10-12

- A full-screen icon

## 0.1.29 — 2026-10-12

- The image viewer zooms and pans

## 0.1.28 — 2026-10-12

- Slots open and close on a spring, and a framed card's frame has corners of its own

## 0.1.27 — 2026-10-12

- A share's action lights up over its row, and a flat card leaves room under its last row

## 0.1.26 — 2026-10-12

- A hint points at the control it describes, not at the middle of what wraps it

## 0.1.25 — 2026-10-12

- A framed card can stand flat on a host that already is the card, and its foot's action answers the pointer

## 0.1.24 — 2026-10-12

- A framed card keeps its rows apart, so two fills never touch

## 0.1.23 — 2026-10-12

- The context window card lists a share that was not measured without a count

## 0.1.22 — 2026-10-12

- A collapsible reads as one block and opens in place; a framed card's fills follow its corners

## 0.1.21 — 2026-10-12

- FramedCard: the card on a frame, its roomy rows and its foot, as a piece of its own

## 0.1.20 — 2026-10-12

- The context window card stands on a frame, with roomier rows and its foot outside the card

## 0.1.19 — 2026-10-12

- Heavier faces for text, rows that close where they were, and ink that fades to a new colour

## 0.1.18 — 2026-10-12

- A context window card whose figures count and whose cells empty by themselves

## 0.1.17 — 2026-10-12

- A host-fed surface takes the part of a frame that changed and uploads only that

## 0.1.16 — 2026-10-12

- Sequence diagrams are drawn natively from Mermaid sources, in Markdown, over the window and on their own

## 0.1.15 — 2026-10-11

- A host-fed surface shows frames one to one with the display, in one texture

## 0.1.14 — 2026-10-11

- Flowcharts: straight connectors to a neighbour a few pixels off, drag to pan with a position bar, an expand action and a full-window DiagramViewer

## 0.1.13 — 2026-10-11

- Flowcharts are drawn natively from Mermaid sources, in Markdown and on their own

## 0.1.12 — 2026-10-11

- Keycaps follow Code OSS: an outline, a firmer bottom edge and a plus between the keys of one combination

## 0.1.11 — 2026-10-11

- A pinned child with a fade also covers what would scroll above it and through its corners

## 0.1.10 — 2026-10-11

- Text area: a scrollbar while it scrolls or is pointed at, and slanted highlights for text not yet committed

## 0.1.9 — 2026-10-11

- Select opens the standard menu: compact rows and a check on the choice

## 0.1.8 — 2026-10-11

- A shared hover surface stays lit in the gap between its actions

## 0.1.7 — 2026-10-11

- DictationButton: a composer's microphone, with a spinner while it starts or transcribes and a pulsing stop while it listens

## 0.1.6 — 2026-10-11

- A control hovered on a lit shared hover surface is lifted one step above it
- MessageActions: the row of actions of a chat message, revealed by hovering it
- CoachMark and Tour: a teaching bubble anchored to a control by its ID

## 0.1.5 — 2026-10-11

- Toast: copy wraps and the card grows instead of cutting it with an ellipsis
- TabItem::title_ink: a tab's title takes the ink of a state the host marks by color
- Button::round and Icon::StopFill: a circular send or stop button
- Element::sticky_top_fade: the rows under a pinned child fade out below it

## 0.1.4 — 2026-10-11

- Tooltip items can be icons alone; a press passes through a hint's passive anchor

## 0.1.3 — 2026-10-10

- Collapse and Element::animate_height: a slot in the flow that closes when its content leaves

## 0.1.2 — 2026-10-10

- Context menus may grow to 480 px before they scroll
- Element::popover_gap: say that an anchor needs height inside a scroll area
- AlertDialog::alternative and Element::popover_gap
- Element::sticky_top: a column child that stays at the top of its scroll area
- UiTree::reveal: bring a control into view in scroll areas of rows of any height
- Ticker: a fixed-height slot whose line rises in as the one before leaves

## 0.1.1 — 2026-10-10

- TabItem: center the close control in its slot so a 20 px one stays inside the tab
- DropdownMenu: a custom row spans the same width as its neighbours

## 0.1.0 — 2026-10-10

- Text::size, TextArea::radius, Markdown::web_link and Markdown::link_icon
- FormField::rows, PopoverPanelSizing, TextField::height, Progress::flat and DropdownMenuItem::truncate
- PageHeader::trailing_top, SectionHeader::truncate, Notice::stacked and three ListRow builders
- Button::sizing, ButtonGroupSizing, ActionRow::trailing_top and RadiusRole::Square
- Markdown: the cache key carries the radius tokens
- Markdown: bounded Markdown prose with links, a source pager and an engine cache
- CardDensity::Flush: a card whose rows run to its edge
- FindBar: every public field and builder says what it is
- FindBar, Breadcrumb::from_path, Composer::header_strip and status inks
- StatusBar, StatusBarItem and MenuBar: the chrome of an IDE window
- MenuHeader and PopoverPanel lose the builders nothing uses; the gallery shows status_width
- Menu slots, MenuHeader, PopoverPanel, SuggestionList and Kbd::caption
- FormField, Select, Modal widths, Tabs::fill_width and TextField::monospace
- Card and CodeBlock: doc comments on the constructors and the sizing fields
- Card, OutputBlock and CodeBlock: a raised surface and two read-only monospace blocks
- Gallery: panel headers sit in an outline on the surface, so the separator between tools shows
- PanelHeader, HeaderSizing, SidebarPanel and Pager: the head of a panel
- Gallery: the folded sidebar header has a neutral folder name
- ListRow: faded titles, a hover peer, trailing actions and a radius role; TreeRow: count and disclosure
- Tooltip::for_path ignores a trailing separator; ListRow tests cover icon_ink and subtitle_mono
- ListRow, RowSizing, DiffStat and Tooltip::for_path: the row of a list
- Notice: a negative fixed height is clamped like every other profile number
- Notice and NoticeSizing: the left-aligned inline notice
- Gallery: the page column demo shows its wide and its narrow state
- PageColumn, PageHeader, SectionHeader and CardGrid: the shell of a page
- ActionRowSizing, ActionRow behaviour builders, Chip::outline and Empty::framed
- ButtonGroup: leading stays first when stacked; only the actions reverse
- ButtonGroup, ButtonSize::Mini and Button ink, gap and radius
- Gallery: label-only, icon-only, single and context-menu folder tabs
- TabStrip, TabItem and TabStripHeader: folder tabs for open resources
- TooltipSpec, ActiveHover, dispatch_as and Icon::Minus for toolbar buttons
- Scheme and ControlColors: a doc comment on every field
- Scheme of explicit roles, Icon::filled, SkeletonGroup and neutral copy
- Text::tooltip wraps the whole value instead of cutting it at one line
- Separator, Text, centering helpers and RadiusRole
- Toast: larger, with its close button centered
- PreferencesGroup: a quiet label over a plain card
- Git icon for branches; preference rows return to the quiet hairline
- Empty: the frameless blank slate from openADE
- Bring over openADE's flat keycaps and nested scroll reveal
- Icons: follow openADE's Bootstrap choices; add bell and filled variants
- Icons: play and pause
- Rename the library to limn
- Editor: bring over the newer editor from openADE
- Every built-in icon is a Bootstrap icon
- Move assistant-specific components out of the library
- Documentation in English under docs/
- Gallery: every component, hot reload and page capture
- Backend: Bootstrap icons, Ubuntu Regular and a counter example
- Tokens, shared motion, new components and layout fixes
- Renderer: configurable image cache size
- Large standalone icons, and the shared search field in the breadcrumb
- Breadcrumb: search button and an editable search field mode
- Breadcrumb segments use the shared rounded hover
- One interaction rule: Theme::interactive and Theme::active
- Add Theme: a palette derived from three colors
- Let the host read and set a horizontal scroll's offset
- Add attach_fonts to wire a tree to egui's fonts
- Add Wayland file drag and drop and a persistent drag ghost
- Extract lienzo UI framework from openADE
