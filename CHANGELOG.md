# Changelog

Every release of limn, newest first. Versions before 1.0 may change the API
in any release.

## Unreleased

- `Carousel`: a shelf of host-built items that scrolls sideways inside a page that scrolls down, without arrow buttons. `Carousel::new(id, items, palette)` with `.title` (24 px bold), `.eyebrow`, `.leading` (a picture held round in 48 × 48), `.trailing`, `.gap` (24 unless set) and `.inset`, so the first item lines up with the page's gutter while the row scrolls out to its edges. Edges with hidden items fade into the palette's `surface`, and its indicator shows only while it moves. See [Carousel](docs/components/carousel.md)
- `drag_to_pan` viewports glide: a drag let go with speed carries on at the pointer's speed over its last 100 ms (at most 8000 px/s) and slows as e^(-t/0.325 s) to rest, stopping at either end, by elapsed time; `animation_repaint_after` asks for every refresh while it glides. A press catches a gliding row, and the wheel, the trackpad or `set_horizontal_scroll_offset` take over from it. Under reduced motion a row stays where it was let go
- `drag_to_pan` viewports pan from a press on a button inside them too: after 4 px of sideways travel the press becomes a pan and the button is not clicked; a press that does not travel still clicks it. Draggable controls, sliders and fields keep their presses. Before, only a press on the viewport's own background panned
- `CodeEditor` view options: `show_whitespace` marks spaces with `·` and tabs with `→`; `rulers` draws up to 16 column guides; `caret_style` takes a `CaretStyle` (`Line`, `Block`, `Underline`; Vim's Normal mode keeps its block); `caret_blink(false)` keeps the caret shown; `indent_guides(false)` and `highlight_current_line(false)` turn off what was always drawn. Defaults keep today's look
- `limn-editor::TextBuffer::trim_trailing_whitespace`: removes the spaces and tabs ending each line as one undo step, keeping line terminators and every selection's place in its line
- Bar, column and line charts whose values are all whole numbers get whole ticks: a count of 1 no longer shows a 0.5 tick
- `limn-dnd::WaylandDataOwner` and `FileDropEndpoint`: an explicit owner routes registered Wayland surfaces through one worker and one data device per seat. Independent endpoint retirement preserves old terminal events; Detached releases its surface permit, while owner Closed still requires cleanup and join. Four worker and four surface permits are global, with the existing bounded codec, Copy decisions and registry budget. Default attachment stays DnD-only; Primary and stock-SDK clipboard backend integration remain pending. The existing native receiver demo is unchanged
- Opt-in `WaylandDataOwner::attach_owned_with_text_clipboard`, `reserve_text_paste`, `TextPasteRequest`, `TextClipboardLimits` and `ClipboardError`: bounded incoming Selection text with explicit ACK, strict UTF-8/RAW results, preserved CRLF and valid empty text. Text shares the Copy-drop incoming reservation; success is published after reader-watcher removal and consumed once or expired by TTL. Cancel/Drop leave admitted reader/body cleanup on the worker. The gallery illustrates the lifecycle synthetically, with no OS clipboard access. Private-server coverage is complemented by a public-API smoke on private headless Weston 15.0.1/Pixman with a fake seat: real configure/focus, RAW UTF-8/CRLF and empty one-shot results, and Detached/Closed with the host reader alive. This covers only that compositor/configuration; SDK/UI/RUNE and Hyprland integration remain unverified, and a full clipboard backend remains pending. See [File drops and text](docs/file-drop.md#opt-in-selection-text)
- `limn-dnd::FileDropReceiver`: an unsafe owned, receive-only Linux/Wayland API with bounded URI-list validation, correlated Started/Ready/Ended events, explicit Copy decisions, context epochs and nonblocking closure. Receivers and legacy FileDnd share four main-worker/surface reservations and 64 non-refundable registry attempts; receivers use one off-UI reaper. Local finish success does not acknowledge compositor receipt. See [File drops](docs/file-drop.md) for the lifetime and SDK progress limits
- **Breaking:** `FileDnd::attach` is now unsafe: borrowed display/surface resources and coordinated host reading must survive its synchronous Drop/join. Legacy outgoing and per-drop reader behavior remains outside the receive-only hardening **Breaking:** legacy attaches now share the four drop-worker slots and the 64-registry budget with receivers; `attach` returns `None` once either is used up.
- Standalone native receive demo: `cargo run -p limn-gallery --bin file_drop`, with explicit Accept/Reject, a 30 s demo decision window, metadata-only display and a close handshake. A private nested Weston run verified two fixture basenames, explicit Accept Copy/Reject and normal demo closure, without opening file contents or sending anything. Header, target and Close stay fixed while details/logs scroll; three UiTree tests cover hit testing and the 14 px target-width regression. The observed Hyprland host clipboard routing conflict remains unresolved; shared-device host integration and other combinations remain unverified

- Add indexed input: `limn_egui::events_indexed` returns `IndexedUiEvent { source_index, event }` for the same post-normalization viewport snapshot. Hosts can interleave untranslated events and read Key/Alt metadata from their source; frame-derived file metadata has no index and follows every indexed event, keeping the file order of `events`. See [Indexed input](docs/backend.md#indexed-input).
- Input compatibility: `events` and the indexed API share a converter that deduplicates only forward, contiguous, non-overlapping Enter/Text and clipboard Key/payload pairs. An unknown intervening event breaks a pair. Initial pointer state, double/triple-click detection and unpaired Copy/Cut Shift remain whole-frame compatibility data; touch translation is unchanged.
- Gallery: `captured_paste --ordered-input` demonstrates the two synthetic navigation/marker orders using the visible destinations; `--small` selects 600 × 580. Verification and captures remain synthetic and separate from native clipboard integration.

- Add the captured-paste API contract: `UiTree::capture_paste_target`, `invalidate_paste_targets`, opaque single-use `PasteTarget`, and `UiEvent::PasteCaptured`. Delayed text replies remain tied to observed tree/focus/editable state, including IME and Vim mode changes; accepted empty or failed insertions consume the capture, while busy/pending-layout attempts do not. This does not provide clipboard I/O, an owner, ACK/completion or native clipboard integration. See [Captured paste targets](docs/captured-paste.md).
- Add opaque `limn_editor::EditStateToken` and `TextBuffer::edit_state_token` for buffer instance, text revision and primary/secondary selection state. Shared-buffer selection changes and ABA invalidate it; no-op setters preserve it. Validate and edit under the same nonblocking buffer guard.
- API compatibility: `UiEvent::PasteCaptured` is a new variant for exhaustive matches to handle. Legacy `PasteTo` remains available with its previous ID/focus semantics; the new capture has no serialization contract and retains no text or editor resources.

- Gallery: `captured_paste` example checks synthetic Unicode delivery, stale invalidation and empty-once replay in a TextArea and CodeEditor. It includes limn Run/Close/theme/corner controls, `--verify`, dark/light and square variants, and `--screenshot` to a new PNG. It does not request clipboard I/O; eframe may initialize its default backend. Native visual QA is separate.
- Pills and circles are true stadiums at every scale. egui stores radii as whole points and `limn-egui` truncated them, leaving a flat strip at each end of a pill whose half height was not a whole point; radii now round, and one within half a point of half the shorter side saturates so epaint clamps it to the pixel-snapped rectangle. The tree also clamps every painted radius to the rectangle it rounds
- `clip_children` and scroll viewports on a rounded element clip their descendants to the rounded shape, so a child flush with or pushed past a corner no longer paints it square. The tree emits the new `DrawCommand::RoundClip { rect, radii }` after the clip's `PushClip` (the rectangle is unchanged, so a backend may ignore it); `limn-egui` cuts fills, corner fills and images to the clip and rounds the corners that reach its arcs, concentric with them
- `Element::round` rounds the ends fully at whatever size the element is laid out; `Button::round` and `IconButton::round` use it, so a round control given another height after it is built stays a pill
- `ScalarSpring::launch(value, motion, now)` sets a spring off at the speed an ease-out over its settling time starts with instead of from standstill, and takes the new motion atomically. Tab indicators, coach marks, `animate_height` slots and paced springs use it, so they move on the first frame after a change (14% to over 25% of the way) and no longer jump when their pace changes mid-flight
- `DismissDrag::velocity(now)` reads the recent downward speed, to hand a released drag to `ScalarSpring::release`
- `Element::animate_scale(id, factor)` and `animate_scale_about(id, factor, anchor)` paint an element larger or smaller on a spring without changing layout, hit targets or line breaks; reduced motion changes it at once
- `UiTree::set_scroll_offset(id, offset)` positions a vertical scroll area from the host, as `set_horizontal_scroll_offset` does sideways
- `Tabs::radius(r)` rounds the selection by `r` and the segmented track by 2 more, concentric, a capsule from half the height; `Tabs::font_size(size)` sets the labels. `Tabs` adds the public `radius` and `font_size` fields
- `ActionRow::title_size`, `subtitle_size` and `radius`; `PreferencesGroup::title_size`, `description_size` and `radius`
- `limn-egui` decodes a thumbnail for the size it is drawn at times pixels per point, in steps of 128 up to 1024 (`set_image_thumbnail_side`), in place of a fixed 384; `set_full_image_cache_entries` sets how many full-resolution textures are kept

- `Element::shrink_height_to_fit`: opt-in height shrinking for a non-flex child of a column. Short content keeps its natural height; a bounded scroll area can yield room to a fixed footer without covering it. Marked children yield in order, minimums first and then toward zero if necessary; flexible siblings keep their minimum heights and share the rest by weight. Rows, unmarked columns and the width-only `shrink_to_fit` keep their behavior. `Style` adds public `shrink_height`, false by default; code building `Style` literally adds that field. The ScrollArea gallery shows short, tall and constrained content
- `Composer::drop_target()` opts the enabled editor into file drops using its existing ID, without adding a hit region or changing focus. The host owns validation and file access; disabled composers never accept drops. The Editor gallery records the count without opening files.
- File hover and drop targets respect floating surfaces, context menus and modals, keeping files from reaching covered background controls.
- The egui bridge delivers native files after the frame's ordinary input, allowing hosts to revalidate an editor after a click or key changes the view.
- NumberInput: `button_placement(NumberInputButtonPlacement::Split)` places decrease and increase on either side of a centered editable value, with matching caret, selection and pointer geometry. The default `End` placement and existing actions, sizes and bounds are unchanged
- Passive scroll anchors retain current layout geometry inside clipped static branches, including the first deep layout and changes to width or font size. `UiTree::anchor_offset` no longer loses nested anchors when their container leaves the viewport; those hidden branches still produce no painting or hit targets.
- `limn_egui::set_icon_rasterizer` installs an optional UI-thread renderer for native icon masks, using the existing bounded texture cache and tint. Missing or malformed masks fall back to SVG; file-type artwork preserves its colors. Replacing or removing the renderer refreshes cached icon textures
- `Element::fade_insets(top, bottom)` positions edge fade bands outside opaque floating bars without changing scroll range, layout or pointer targets. Fade heights describe the visible transition beside the bars; zero insets preserve existing rendering. `Style` adds the public `fade_insets` field, defaulting to `(0.0, 0.0)`.
- Horizontal viewports stop their pointer pan on `PointerLeave`, preserving the current offset when a touch host takes over scrolling or cancels the gesture. A later touch no longer resumes the old grab and jumps the content back

## 0.1.43 — 2026-10-15

- CodeEditor and CodeDiff: `font_size` keeps text, row geometry, scrolling and pointer selection at the same size; `diff_backgrounds(false)` hides row tints while preserving change markers. Both add public `font_size` and `diff_backgrounds` fields for hosts building their structs literally
- CodeBlock: `font_size` overrides code without changing its label; `word_wrap(false)` preserves source lines in a horizontal viewport. Markdown exposes independent fenced-code settings through `code_font_size` and `code_word_wrap`, included in its render cache key. Set `id` on a block or preview to preserve horizontal offsets when rebuilding it; equal fences and cached previews keep separate viewport identities
- Anonymous code viewport state is discarded when replacing the visual tree, keeping repeated renders bounded; explicitly named code viewports and other scroll state keep their offsets
- `limn_egui::font_definitions()` exposes bundled faces and their fallbacks so hosts can prepend system fonts before calling `Context::set_fonts`; `install_fonts` uses these same definitions
- Native raster images use two decode workers per UI thread instead of a thread per image, with a nonblocking queue bounded by 32 sources and 16 MiB of encoded data. `limn_egui::clear_image_cache()` releases textures and queued sources; late results from the old generation are discarded. The ImageViewer gallery has a cache reload control
- `limn_egui::install_fonts`: Noto Emoji 3.002 monochrome fallback, appended after every existing face. A 623-entry emoji fixture verifies scalar coverage in all five families, with existing glyph widths preserved. The unmodified variable font adds 1,982,596 bytes under OFL-1.1; provenance, license and synthetic Text gallery examples are included. Regional flags and some combined sequences remain limited by font selection; this does not add color emoji
- `limn_egui::install_fonts`: Noto Sans Symbols 2 v2.008 fallback for monochrome decorative symbols such as `✦`, `❖` and `➜`. Every family preserves its existing glyph priority. The unmodified OpenType face adds 404,576 bytes under OFL-1.1; full license, release source and hashes are included, with synthetic examples in the Text gallery
- `limn_egui::install_fonts`: bundled Noto Sans Math fallback for mathematical alphabets such as `𝖗𝖚𝖓𝖊` and `𝓡𝓤𝓝𝓔`, in every limn font family. Existing UI faces and egui fallbacks keep their priority. The unmodified font adds about 1 MiB under SIL OFL-1.1; its license and pinned provenance are included. The Text gallery shows synthetic samples

## 0.1.42 — 2026-10-14

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
- `Element::wrap` and `Element::line_gap`: a row whose children move onto more lines when they do not fit its width, as tags or role pills do. The row is as tall as its lines; Tab, the pointer and assistive technology find each child where it is, in the order given
- Switch: `size(ControlSize)`, from 28×16 for `Dense` to 44×24 for `Spacious`; `variant(SwitchVariant::Accent)`, a light thumb on and off over a track of the palette's link color, or of `ink`, darkened where needed until the thumb reads on it at 3:1, as chat applications draw a switch; and `ink` for the track's color while it is on. A switch without them draws what it drew
- Slider: `marks()` draws a hairline at each stop, up to 100 stops, `mark_labels` names the stops over the track, each on one line, and `default_stop` shows one of them in the ink; the platform is told the label of the stop the value is on
- RadioOption: `icon`, between the circle and the label
- TextField: `icon`, an icon before the text inside the field, muted until the field has the focus, as a search field's magnifier; a disabled field keeps it in its place
- SearchField: `clearable(true)`, a button at the end of the field while it has a query, which reports `Clicked` with `"{id}:clear"`; the text stops before it, and disabled it stays in its place, passive. `Labels::clear_search` names it. A clearable field is as wide with a query as without, never under 80
- SearchField: a disabled field shows its shortcut hint only while it has no query, as the enabled one does
- Select: `variant(SelectVariant::Field)`, a trigger drawn as a text field, with its fill, border and height and the choice at its start, open or focused from the keyboard with a focused field's border; `size(ControlSize)` and `height(px)` for either variant. A select is drawn as it was unless told otherwise
- `Element::focus_border` on a control whose focus ring is off is shown while it has the keyboard focus, as its mark of that focus; a text input or area shows it on any focus, as before
- New components: `SwatchPicker` and `Swatch` (one color among a few, a ring around the chosen one, and an optional custom swatch that opens a picker) and `ColorPicker` with `Hsv` (a field of saturation and value, a bar of hues and a hex field; `ColorPicker::color_after` turns its actions into the next color, `hex` and `parse_hex` write and read it), each with a page, a guide and the swatches in the controls harness
- `Element::area_slider` and `UiAction::AreaSliderChanged`: a slider over an area, dragged in two directions and stepped with all four arrows
- `Element::horizontal_gradient` and `vertical_gradient`: evenly spaced colors across an element, its corners rounded
- Gallery: a field shows the text a slider leads to, as it does after a click
- Labels: `saturation_brightness`, `hue` and `hex`, what a color picker calls its parts to the platform
- DropdownMenu: `DropdownMenuItem::checkbox` and `radio`, items drawn with a box or a circle at the end of the row and told to the platform as a menu's check box or radio item (`Role::MenuItemCheckBox` and `Role::MenuItemRadio`), on or off
- DropdownMenu: a two-line item lines up with the menu's one-line rows in every style: it is as tall as the menu's `sizing` says and has their sides, icon slot, label size, gap and check and chevron columns, where it had the catalogs' metrics whatever the menu; one with a `description` and a `submenu` ends with a chevron and opens it; a disabled one says why on its second line; and in a menu with no `selected` item it no longer says it is not selected
- DropdownMenu: a submenu stays open while the pointer or the focus is in a submenu nested in it, and the arrow keys walk in and out of every level; it closed as the pointer went into the nested one. The pointer on another submenu's row closes the ones the keyboard opened, where both stayed open
- DropdownMenu: `at(point)` opens the menu at a point of the window, as a context menu at the secondary click, with every kind of item; `Element::popover_at` places any popover so
- A secondary click outside an open popover or menu reports its dismiss ID, as a primary one does; it left the popover open

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
