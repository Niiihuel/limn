# Changelog

Every release of limn, newest first. Versions before 1.0 may change the API
in any release.

## Unreleased

- `SplitView` and `Pane`: a row or a column of panes with a `ResizeHandle` between each pair, each pane clipped and given a fraction of the room the handles leave (`Pane::fraction`) or what the others leave (`Pane::flex`, the default); `.handle_thickness(px)`. The handle after pane `i` reports the drag on `{id}:handle:{i}`, and `SplitView::resized(fractions, handle, delta_px, total_px, mins)` gives the host the new fractions, moving the two panes beside the handle and keeping every pane at its minimum; with a delta of 0 it fits them to the minimums after the split's length changes. On the gallery's SplitView page. See [SplitView](docs/components/split-view.md)
- `CodeDiff` marks changed words and folds unchanged lines. `word_ranges` tints the changed words of added and removed rows more strongly than the row; a row of the new kind `CodeDiffLineKind::Folded` shows its text, such as "Show 12 more lines", and reports `Clicked { id: "{id}:expand:{row}" }`; `colors(DiffColors)` sets the tints, markers and bars, its default the green and red drawn until now; `margin_bar(false)` hides the bar at the left edge of changed rows. New `DiffFileHeader::new(id, path, palette).stat(added, removed).collapsed(bool)`: a chevron, the path with the file's name emphasized and `+N −M`, reporting its ID. On the gallery's CodeDiff page. **Breaking** for exhaustive matches on `CodeDiffLineKind` and for `CodeDiff` literals. See [CodeDiff](docs/components/code-diff.md#changed-words)
- `TerminalView::focus_ring(visible)`: `false` draws no focus ring around the focused view, as xterm.js draws none, leaving the blinking cursor as the mark of the focus. A press still focuses it. On by default. See [TerminalView](docs/components/terminal-view.md#keyboard)
- `FontFamily::MonospaceBold`: the bold face of the code family, as wide as `Monospace`. `Fonts::bundled()` gives it Hack Bold, and limn-egui paints it; a host without it paints `Monospace`. `TerminalView` draws bold cells in it, the block cursor's character included. **Breaking** for exhaustive matches on `FontFamily`. See [TerminalView](docs/components/terminal-view.md)
- `TerminalView`: a terminal's screen and its scrollback, painted from a `TerminalScreen` of `TerminalCell`s whose colors the host's emulator already resolved; limn runs no shell and reads no escape sequences. Runs of one ink are drawn as one text over fills of their cells, inverse cells swap ink and fill, and `min_contrast(ratio)` lifts inks that do not read on their fill. A press or Tab gives it the focus under its ID, for the host to read the keys; dragging selects text across rows for Ctrl+C to report as `CopyRequested`; its block or bar cursor (`TerminalCursor`, `TerminalCursorShape`) blinks on the clock text fields' carets share, and holds still under reduced motion. `TerminalView::grid_size(size, font_size)` gives the columns and rows the host sizes its pseudo terminal to. New `HitKind::Focus`, the region of something that takes the focus and reports nothing. **Breaking** for exhaustive matches on `HitKind`. On the gallery's TerminalView page. See [TerminalView](docs/components/terminal-view.md)
- `SplitButton`: a primary action joined to a `DropdownMenu`, drawn as one 32 px button with 1 px between the halves. `SplitButton::new(id, label, menu_id, palette)` reports `Clicked` with `id` for the action and with `menu_id` for the chevron, each its own Tab stop; `.menu(entries, open, dismiss_id)` opens the menu below it, aligned to its end, and says the chevron is expanded; `.keys(&["Ctrl", "Enter"])` draws the shortcut in keycaps after the label, in the button's ink, and tells the platform as the action's description; `.variant(ButtonVariant)` colors both halves. `Labels::more_options` names the chevron. **Breaking** for `Labels` literals without `..`. On the gallery's SplitButton page. See [SplitButton](docs/components/split-button.md)
- `StatusDot`: a round mark in a status color, 6 px unless `.size(px)` says otherwise, such as green beside a session that is running. `.pulse(true)` eases its opacity from 1 to 0.3 and back every 1.2 s, every pulsing dot in a window in step, and holds it at full opacity under reduced motion. With `.id(id)` the platform knows it as a picture named by `.description(text)`. On the gallery's StatusDot page. See [StatusDot](docs/components/status-dot.md)
- `Progress::ring(diameter)`: a ring in place of the bar, its arc in `palette.text` covering the fraction clockwise from twelve o'clock over a circle in `palette.selected`, such as how much of a model's context is used. With an `id` it is the same progress indicator as the bar. On the gallery's Progress page. See [Progress](docs/components/progress.md#ring)
- `AlertDialog::soft_destructive(fill, border, ink)`, a pale confirm with a one-pixel outline; `cancel_variant(variant)`, so Cancel can be filled (`Secondary`) rather than outlined; `description_size(px)`, 12 unless set; and `closable(false)`, a dialog without the close button, which Cancel and Escape still dismiss. `Modal::closable(false)` likewise. The defaults are unchanged. See [AlertDialog](docs/components/dialog.md#alertdialog)
- `AlertDialog::soft_destructive(fill, border, ink)`, a pale confirm with a one-pixel outline; `cancel_variant(variant)`, so Cancel can be filled (`Secondary`) rather than outlined; `description_size(px)`, 12 unless set; and `closable(false)`, a dialog without the close button, which Cancel and Escape still dismiss. `Modal::closable(false)` likewise. `Modal::scrim(color)` and `title_gap(px)`, also on `AlertDialog`, set the backdrop's color and the space under the title; `AlertDialog::button_sizing(sizing)` the actions' geometry. The defaults are unchanged. See [AlertDialog](docs/components/dialog.md#alertdialog)
- `AlertDialog::soft_destructive(fill, border, ink)`, a pale confirm with a one-pixel outline; `cancel_variant(variant)`, so Cancel can be filled (`Secondary`) rather than outlined; `description_size(px)`, 12 unless set; and `closable(false)`, a dialog without the close button, which Cancel and Escape still dismiss. `Modal::closable(false)` likewise. `Modal::scrim(color)` and `title_gap(px)`, also on `AlertDialog`, set the backdrop's color and the space under the title; `AlertDialog::button_sizing(sizing)` the actions' geometry, and `description_line_height(px)` the description's leading. The defaults are unchanged. See [AlertDialog](docs/components/dialog.md#alertdialog)
- `ButtonVariant::Secondary` on a light palette darkens under the pointer and when pressed, rather than lightening toward the card it sits on until it reads as white (#F2F2F2 went to #FCFCFC). A dark palette still lightens. See [Button](docs/components/button.md#variants)
- `Tokens::shortcut_style`: `ShortcutStyle::Text` draws every menu's and `Kbd`'s shortcuts as one line of muted text, and a tooltip's in its style's `muted`, instead of keycaps; `Keycaps`, the default, is unchanged. **Breaking** for `Tokens` literals without `..`. See [Tokens](docs/tokens.md#shortcuts)
- `DropdownMenu::check_ink(color)` draws a selected row's check in that color, and `selected_fill(false)` leaves the selected row unfilled, its check alone marking it. Submenus take both. See [DropdownMenu](docs/components/dropdown-menu.md#style)
- `Composer::controls_at_last_line(true)`: with `inline_controls` and `auto_grow`, the actions stay beside the editor's last line, at the bottom of the growing row, rather than its first. See [Composer](docs/components/editor.md)
- `Markdown::code_syntax_colors(CodeSyntaxColors)`: the host's inks for fenced code, `text`, `keyword`, `string`, `number` and `comment`, and two it may turn on: `function`, a name before `(`, and `interpolation`, the braces of a Python f-string's `{…}` or a JavaScript template's `${…}`, whose contents are then drawn in `text`. `CodeSyntaxColors::for_palette` is the default and draws as before. `Markdown::inline_code_size(px)`: inline `code` at its own size in body text, such as 13 in a 16 px body, in the same proportion in headings. See [Markdown](docs/components/markdown.md#highlighted-code)
- `Markdown` draws code blocks as a chat app does when asked: `code_label(false)` drops the language line, `code_fit_content(true)` makes a block as wide as its code up to the reading width, and `code_copy_action_prefix(prefix)` puts a muted copy icon at its end, centered against a one-line block, whose action is the prefix followed by the code. Its hint is the new `MarkdownLabels::copy_code`. **Breaking** for `MarkdownLabels` literals without `..`. Defaults are unchanged. On the gallery's Markdown page, "Code as a chat app draws it". See [Markdown](docs/components/markdown.md#code-blocks-as-a-chat-app-draws-them)
- Formatted text laid out without `TextLayout` keeps its last letter when handed back the width it measured, as `TextLayout` already did: a line may run a hundredth of a pixel past its width
- `Markdown` colors fenced code by its language: comments, strings, keywords and numbers in Rust, JavaScript, TypeScript and TSX, Python, JSON, TOML, CSS, HTML and Markdown, read from the fence's label or an alias (`rs`, `js`, `ts`, `py`, `md`…). The line lexer runs synchronously while the tree is built, with or without the `syntax` feature. A fence with no language or an unknown one, code with nothing to color, and code over 64 KiB or 2,000 lines stay plain text as before. The inks follow the palette: the code views' inks on a dark one, with a lighter comment, and darker counterparts on a light one, each at least 4.5:1 on `Theme::DARK`'s raised fill and on light fills down to `rgb(220, 220, 220)`. The label, wrapping, sizes, line heights and the selected text are unchanged. See [Markdown](docs/components/markdown.md#highlighted-code)
- Without the `syntax` feature, `CodeLanguage::from_code_fence` reads the same labels and extensions as with it instead of always answering `Plain`, so code inside a fence of a Markdown file in a code view is colored by its language there too
- A wheel or trackpad scroll outside an open menu or popover closes it, as a press there does: it reports the dismiss ID with `UiAction::Clicked`, and in the same event scrolls or pans what is under the pointer. A scroll over the surface, a submenu included, scrolls it and keeps it open; a modal (`Modal`, `AlertDialog`, `BottomSheet`) stays open, and a context menu closes. Covers `Scroll`, `ScrollDirect` and `Scroll2D`. A popover made `popover_non_modal`, which a press outside does not close either, is left to its host. See [DropdownMenu](docs/components/dropdown-menu.md#actions) and [Popover](docs/components/popover.md#actions)
- Selecting in text fields and text areas: dragging selects from the press, Shift and a press extend the selection, a double press selects a word (or a run of spaces or a mark) and a triple press the line, or a single line's whole value. Ctrl and Left or Right move by word, and Ctrl and Backspace or Delete delete one. The arrows and Backspace step over a grapheme whole, so an emoji with its skin tone, or a CRLF break, is never split, and an arrow collapses a selection to the edge it points at. A password has no words: Ctrl and an arrow go to its ends and a double press selects it whole. The pointer gestures read the click count and Shift that limn-egui already sends. See [TextField](docs/components/text-field.md#pointer)
- Input methods in text fields and text areas, in a tree that lays its text out: what is being composed shows at the caret, underlined, in place of any selection, and joins the value (reporting `InputChanged`) only when it commits; a cancel leaves the value and the selection as they were, and an input method can delete around the caret. `UiTree::ime_area()` says where an input method belongs for the focused code editor or field, and whether it is a password, and `limn_egui::report_ime(&ctx, &tree, origin)` tells the platform each frame. Until now no limn field reported it, so egui kept input methods off for them. New `ImeArea`. See [TextField](docs/components/text-field.md#input-methods)
- Text fields and text areas are laid out in a `TextLayout` too, in a tree told to with `UiTree::set_text_layout(true)` (as `limn_egui::attach_fonts` does): their value is drawn as `DrawCommand::Glyphs`, and the caret, the selection, the wrapping, where a click puts the caret and where Up and Down take it all come from one layout of it. Up and Down keep the caret's column in pixels across shorter lines, End on a wrapped line stops before the space it hangs, a long value scrolls by pixels rather than whole characters, and a text area draws only the lines in view. A text area's highlights are spans of its text: an italic range is set in the italic face, and the caret and the wrapping follow it. See [TextArea](docs/components/text-area.md#highlights)
- Glyphs of the host's own: `TopBar::home_icons(rest, active)`, `browse_icons(rest, active)` and `search_icon(icon)`, the last for the field's magnifier and the round search of a narrow bar; `SideRail::library_icon(icon)` and `create_icon(icon)`. The glyphs drawn until now are the defaults. See [TopBar](docs/components/top-bar.md#home-and-browse) and [SideRail](docs/components/side-rail.md#head)
- `Tokens::tooltip`: one `TooltipStyle` for every tooltip in the process, whichever palette its control was built with, such as a #282828 body with white text and a transparent `border`, which draws no outline; `None`, the default, leaves each palette its own. **Breaking** for `Tokens` literals without `..`. See [Tokens](docs/tokens.md#tooltip)
- `DropdownMenu::scrollbar(appearance)` and `scrollbar_visibility(visibility)`: the options scroll with the host's bar, such as an overlay that shows under the pointer, and an overlay takes no width from the rows; submenus take both. Unless set, the reserved muted rail as before. See [DropdownMenu](docs/components/dropdown-menu.md#size-and-placement)
- `Element::formatted_text` is laid out in a `TextLayout` too, in a tree told to with `UiTree::set_text_layout(true)` (as `limn_egui::attach_fonts` does): `Markdown`, chat messages and every component made of formatted text are drawn as `DrawCommand::Glyphs`, and their spans are selected, hinted, focused and pressed from that layout. Text sits where it did, shaped rather than measured: spacing can differ by a pixel, and a line that mixes faces, such as one with inline code, sets them on one baseline. New `TextLayout::formatted(spans, paragraph)`, `max_lines(n)` and `span_bounds(index)`. With 400 paragraphs on screen the first frame takes 50 ms instead of 87, and limn's own pass is a little faster after it; but egui tessellates limn's glyph meshes more slowly than its own text (1.5 to 5 ms a frame instead of 1), so a frame where nothing moves can cost up to 2 ms more, and one while the window resizes about 1.5 times as long, its moved lines meshed again. limn-egui reuses a run's mesh without moving it between maps each frame. See [TextLayout](docs/components/text-layout.md#formatted-text)
- The cache of measured formatted text tells every face, line and hint apart: a span in `SansBold` and the same text in `Monospace` no longer evict each other
- Text laid out by limn keeps its last letter: a label that shrinks to fit in a row, such as a checkbox's, is handed back the width it measured by sums that can come out a rounding error short, and since words are broken between graphemes its last letter went to a hidden second line ("Compac"). A line may now run a hundredth of a pixel past its width
- Liquid glass is gone. **Breaking**: `GlassMaterial`, `Element::glass`, `NavigationBar::glass`, `DrawCommand::Glass` and limn-egui's `GlassRenderer` with its shader are removed, with the gallery's glass demo and the Liquid glass guide. A host that matches `DrawCommand::Glass` drops that arm. `NavigationBar` keeps its opaque surface; its gallery page is now `navigation-bar`. See [NavigationBar](docs/components/navigation-bar.md)
- `ScrollbarAppearance::thumb_radius(px)` and the `thumb_radius` field: the corner of the thumb and its track, round unless set, square at 0, as Spotify's desktop client draws it on Windows and Linux. See [ScrollArea](docs/components/scroll-area.md)
- Text laid out by limn breaks a word longer than its line between graphemes, as the measured text always has: a long address in `Element::text` no longer runs past its box
- `Element::push_on_hover(id, by)`: as `extend_on_hover`, and the element takes the room it reaches into, so what flows beside it gives way. `PeekPanel` now pushes the main area beside it instead of covering it, as a music app's closed panel does, on either side. See [PeekPanel](docs/components/peek-panel.md)
- The marks of text. `Markdown` reads `__underline__` (as chat clients write it), `~~strikethrough~~`, `***bold italic***` and `-# subtext` lines (0.8125 times the body, muted), and marks nest (`__**both**__`) but neither open nor close inside a `<…>` without spaces, such as a chat client's `<:emoji_name:id>`; `**strong**` is now drawn in the bold face and `*emphasis*` in the italic one, and `\~` escapes a tilde. A web address stops before a `|` at its end, which closes a table cell or a `||spoiler||`. `TextSpan::underline()` and `.strikethrough()` draw lines in the run's ink across its wrapped lines; `TextSpan::tooltip(title, palette)` shows a hint over a run after the hover delay (and on keyboard focus for a run with an action), a passive run staying selectable; `Element::max_lines(n)` cuts a formatted text to `n` lines ending in «…», which a selection does not copy. `TextHighlight::background` fills behind a range of a text area or the composer's editor, under the selection and the text, such as a mention drawn as its pill. New `FontFamily::SansBoldItalic` (the bold face slanted; `limn-egui` paints it), `TextHint` and `Style::max_lines`. **Breaking** for `TextSpan`, `TextHighlight` and `Style` literals without `..` and for exhaustive matches on `FontFamily`. On the gallery's Text page, "Marks and hints" and "A limit of lines"; on the Markdown page, "Chat marks"; on the TextArea page, a filled mention. See [Layout](docs/layout.md#marks-hints-and-a-limit-of-lines), [Markdown](docs/components/markdown.md#marks) and [TextArea](docs/components/text-area.md#highlights)
- `TopBar::search_popover(dismiss_id, panel)` floats a panel under the search from its start, without taking the keyboard from the field, such as the searches made lately. See [TopBar](docs/components/top-bar.md#a-panel-under-the-search)
- `SideRail` and `SideRailItem`: a sidebar's library folded into a 72 px column of 48 px pictures, each a button named by its title with a hint to its right that opens at once with the title over a muted second line; a 40 px head button that reports `{id}:expand`, an optional create button (`{id}:create`), `current(id)` and `playing(id)` marks, and items that scroll under the head. On the gallery's SideRail page. See [SideRail](docs/components/side-rail.md)
- `PeekPanel` and `Side`: a closed side panel as a 40 px strip at the window's edge, rounded where it faces the window, showing the panel's content at its open width behind a veil of 0.9. Under the pointer or the focus it reaches 12 px out on a spring that overshoots, the veil lifts and a chevron lights up; a click, Enter or Space reports its ID. Its content is only painted: its controls take no pointer, focus or reader there. On the gallery's PeekPanel page. See [PeekPanel](docs/components/peek-panel.md)
- `Element::extend_on_hover(id, by)`: under the pointer or the keyboard focus, an element's edge reaches out by `by` on a spring that overshoots a little and settles in three `move_ms`; its pointer target grows with it and its slot in the layout does not. See [Animation](docs/animation.md#an-edge-that-reaches-out-under-the-pointer)
- `Icon::ChevronLeft`, drawn with Bootstrap Icons 1.13.2 `chevron-left`. **Breaking** for exhaustive matches on `Icon`
- A `hover_region` that is itself a control counts as hovered while it has keyboard focus, so what it reveals shows for the keyboard too

- `Carousel`: a shelf of host-built items that scrolls sideways inside a page that scrolls down, without arrow buttons. `Carousel::new(id, items, palette)` with `.title` (24 px bold), `.eyebrow`, `.leading` (a picture held round in 48 × 48), `.trailing`, `.gap` (24 unless set) and `.inset`, so the first item lines up with the page's gutter while the row scrolls out to its edges. Edges with hidden items fade into the palette's `surface`, and its indicator shows only while it moves. See [Carousel](docs/components/carousel.md)
- `drag_to_pan` viewports glide: a drag let go with speed carries on at the pointer's speed over its last 100 ms (at most 8000 px/s) and slows as e^(-t/0.325 s) to rest, stopping at either end, by elapsed time; `animation_repaint_after` asks for every refresh while it glides. A press catches a gliding row, and the wheel, the trackpad or `set_horizontal_scroll_offset` take over from it. Under reduced motion a row stays where it was let go
- `drag_to_pan` viewports pan from a press on a button inside them too: after 4 px of sideways travel the press becomes a pan and the button is not clicked; a press that does not travel still clicks it. Draggable controls, sliders and fields keep their presses. Before, only a press on the viewport's own background panned
- Hints of their own: `Tooltip::delay_ms` (350 unless set; 0 opens at once, and the tree waits for each hint's own delay), `title_font_size` (12), `title_font_family` (`Sans`), `arrow` (on) and `background` (the palette's), kept as the new `TooltipOptions` on `ElementKind::Tooltip`; a one-line hint grows by what a larger title adds. **Breaking** for `ElementKind::Tooltip` patterns and literals without `..`. On the gallery's Tooltip page, "Delay, title, pointer and fill". See [Tooltip](docs/components/tooltip.md#delay-title-pointer-and-fill)
- `Badge::placement(BadgePlacement)` puts the mark in any corner of its child (`TopRight` unless set), and `height`, `outline(color, width)`, `ink`, `font_size` and `font_family` size and color it, such as a server's mentions: 17 px of white on red at the bottom right, ringed 3 px in the rail's color. A count keeps 3 px clear of a thicker ring. On the gallery's Badge page, "Corner and size". See [Badge](docs/components/badge.md#corner-and-size)
- `Element::small_icon(px)` draws a standalone icon under 16 px, down to `SMALL_ICON_SIZE` (8), and `Icon::Tree`, `FriedEgg`, `Gamepad`, `Car`, `Lightbulb` and `Flag`, drawn with unmodified Bootstrap Icons 1.13.2 (`tree`, `egg-fried`, `controller`, `car-front`, `lightbulb`, `flag`), name the categories of an emoji picker. **Breaking** for exhaustive matches on `Icon`. On the gallery's Icon page. See [Icon](docs/components/icon.md#smaller)
- `Slider::level(level, color)` draws a live level along the track in place of the fill, such as a microphone's input under the threshold the thumb sets; the platform is told only the value. On the gallery's Slider page. See [Slider](docs/components/slider.md#a-level-on-the-track)
- `Switch::track_size(width, height)`: a track of the host's own size, such as 50 × 26, the thumb 3 px inside it. On the gallery's Switch page. See [Switch](docs/components/switch.md#size)
- `Select::max_visible_items(n)`, `font_size(px)` and `ink(color)`: rows in view before the list scrolls, the choice's text size, and a 2 px border in that color while a field's list is open. A field's chevron now points up while its list is open. On the gallery's Select page. See [Select](docs/components/select.md#as-a-field)
- `ScrollbarVisibility::AutoHideUnlessHovered { hold_ms, fade_ms }`: as `AutoHide`, and also shown while the pointer is over the scroll surface, as a chat's history. **Breaking** for exhaustive matches on `ScrollbarVisibility`. On the gallery's ScrollArea page. See [ScrollArea](docs/components/scroll-area.md#scrollbar)
- Composer: with `inline_controls(true)` and `auto_grow` the row grows with the editor and each control stays beside its first line, and a disabled composer's placeholder wraps there; `editor_padding(px)` and `card_corner_radii(radii)`. `SuggestionItem::leading(element)`, `ink(color)` and `detail(text)`: a person's avatar, a role's color and a handle at the end of the line; an item without a description shows its line in the middle of the row. On the gallery's Editor and SuggestionList pages. See [Editor](docs/components/editor.md#controls-beside-a-growing-editor) and [SuggestionList](docs/components/suggestion-list.md#items)
- Pictures in a line of text: `TextSpan::image(raster)` paints a picture in place of a span's text, and `TextSpan::surface(id)` the host's surface (an animation's frames), each a square `INLINE_IMAGE_SCALE` (1.375) times the span's `font_size`, centered on its line and moved to the next line whole, such as a custom emoji in a message. The text, `:name:`, is not painted but is what a selection copies (the picture whole or not at all) and what accessibility reads; an `action` makes the picture a target. The formatted-text cache tells pictures apart. **Breaking** for `TextSpan` literals without `..`: the new public field `image: Option<InlineImage>`. On the gallery's Text page, "Inline pictures". See [Layout](docs/layout.md#pictures-in-a-line-of-text)

- One copy of each font: limn-egui no longer turns on eframe's `default_fonts`, and `install_fonts` gives egui only limn's faces, in the order limn lays text out in. Hack was bundled twice, and egui's Ubuntu Light, Noto Emoji and emoji icon faces are gone: the gallery's wasm is 6.59 MB gzipped, 454 KB less than before the text layout work began. **A host must call `install_fonts` before the first frame** (the guides always said so), and should drop `default_fonts` from its own eframe features to see the saving. Emoji in egui-drawn text now come from limn's Noto Emoji, and a markdown task's `○` is drawn in the text's weight
- `Fonts::emoji(font)` and `Fonts::emoji_face()`: the face for emoji, tried after every other face, so it never takes a symbol a text face has. `Fonts::bundled()` puts Noto Emoji there instead of among the fallbacks
- `UiTree::set_text_layout(true)` lays every `Element::text` out in a `TextLayout` and draws it as `DrawCommand::Glyphs`, selected and underlined from that layout, with its layout kept from one pass to the next and broken again without shaping when only its width changes. Off by default while the other components move to it; `limn_egui::attach_fonts` turns it on, so in its trees `Element::text` no longer gives `DrawCommand::Text`: a tool or test reading the text drawn reads `GlyphRun::text` or `Frame::text_layouts()`. Text sits where it did, within a physical pixel of rounding. 2000 labels lay out and draw about three times faster: 7 to 10 ms a frame instead of 25, 45 ms instead of 140 for the first
- `TextStyle::leading(Leading::Even | Leading::Below)`: where the glyphs stand in a line taller than they are. `TextLayout::centered(width)`. A layout's runs share their glyphs from one draw to the next, and limn-egui reuses the meshes of runs that stay in place, as egui reuses a galley
- `TextLayout`: text limn shapes, breaks into lines and places itself, with parley inside and none of it in limn's API. `TextStyle`, `wrapped(width)` without shaping again, `size`, `lines`, `caret`, `index_at`, `selection`, and `draw(origin, color)` for `DrawCommand::Glyphs(GlyphRun)`, which a backend draws without laying anything out. Kerning, ligatures, bidi and a fallback face per character. Components do not use it yet. See [TextLayout](docs/components/text-layout.md)
- `Fonts`, `Font`, `FontError`, `limn::fonts()` and `limn::set_fonts`: the faces text is laid out in, one set for the process. `Fonts::bundled()` behind the new default `bundled-fonts` feature; the font files moved from limn-egui to limn, with Hack added for `Monospace`
- limn-egui draws `DrawCommand::Glyphs` from an atlas of its own, rasterized as egui rasterizes its text, and gives egui the same font bytes limn lays text out in, without copying them
- `Element::drawing`: what the application draws itself, in the frame's own `DrawCommand`s, given the size it is laid out at. The tree places the commands, clips them to the element and keeps their clips balanced. See [Drawing](docs/components/drawing.md)
- Text widths are cached apart for every face: a label in `SansMedium`, `SansBold` or `SansItalic` could be given the width the same text had measured in `Monospace`, and so wrap or truncate at the wrong place
- `WindowControls`: the minimize, maximize (restore while maximized) and close buttons at the end of a title bar the app draws, as Windows places them, brought from Surco's own frame. `Labels` gains `minimize`, `maximize` and `restore`. See [WindowControls](docs/components/window-controls.md)
- `limn-egui`: a notch of the mouse wheel scrolls 120 px, as Chrome and Firefox do, instead of 30; touchpads stay one to one
- `CodeEditor` view options: `show_whitespace` marks spaces with `·` and tabs with `→`; `rulers` draws up to 16 column guides; `caret_style` takes a `CaretStyle` (`Line`, `Block`, `Underline`; Vim's Normal mode keeps its block); `caret_blink(false)` keeps the caret shown; `indent_guides(false)` and `highlight_current_line(false)` turn off what was always drawn. Defaults keep today's look
- `limn-editor::TextBuffer::trim_trailing_whitespace`: removes the spaces and tabs ending each line as one undo step, keeping line terminators and every selection's place in its line
- Bar, column and line charts whose values are all whole numbers get whole ticks: a count of 1 no longer shows a 0.5 tick
- `TopBar`: the bar across the top of a window, as a music app's. The host's controls at both ends (`.leading`, `.trailing`), with `.caption(left, right)` keeping room for the window's own buttons; a 48 px round Home button and a pill search centered in the window, or after the start controls when they would meet, narrowed to the room left. `.width(window)` sizes the search to half the window, between 350 and 546 px, and makes it a round magnifier button at 400 or less. The search ends in a rule and a browse button, with a clear button before them while there is a query; it reports as a `SearchField` with `{id}:search` would, and Home and Browse report `{id}:home` and `{id}:browse`, drawn solid with `.home_active` and `.browse_active`. `Icon::HomeFill`, the unmodified Bootstrap Icons 1.13.2 `house-fill`, is `Icon::Home.filled()`. **Breaking** for exhaustive matches on `Icon`. On the gallery's TopBar page. See [TopBar](docs/components/top-bar.md)
- `Element::sticky_top_background_fade_in(from, to)`: a `sticky_top` header whose background fills in as the page scrolls under it, none of it until `from` px of scroll and all of it by `to`, followed by the tree as it lays out. See [Layout](docs/layout.md#sticky-children)
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
