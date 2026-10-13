# Changelog

Every release of limn, newest first. Versions before 1.0 may change the API
in any release.

## 0.1.39 — unreleased

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
