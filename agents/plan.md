# Plan: stable, then renamed

limn takes a new name and opens to contributors once it is stable. This page
says what stable means here, what is done, and the order of what is left. It
is the one page here that changes every week: the change that moves its
facts changes the page.

Last brought up to date: 2026-11-10.

## How work lands now

Until the new repository opens, there are no pull requests.

- One line of work: the branch `next`. Everything written so far is in it.
- One topic, one commit, on `next`. The five commands of
  [`AGENTS.md`](../AGENTS.md#4-commands) pass before the commit.
- `next` is pushed to the old repository as a backup, and nothing else is.
- R17 and the pull-request half of [the workflow](workflow.md) apply from the
  day the new repository opens. Every other rule applies now.

## What stable means

| # | Condition | Today |
| --- | --- | --- |
| 1 | One line of work. No feature waits behind it in a branch, and applications build on it or on a tag. | The line exists: `next` holds every branch. Each application builds and passes on `next` in a branch of its own; none is merged or was run. |
| 2 | Every [invariant](invariants.md) is **Held**. | Seven are not: 1, 2, 3, 5, 9, 12 and 13. |
| 3 | What a host or a backend compiles against is settled: every breaking change known today has landed. | No. See stage B. |
| 4 | An application can build a design of its own on the engine, and change nothing in it. | Partly. The engine is a crate of its own; it still holds some design. See stage C. |
| 5 | A new control edits no file that another change is likely to edit. | Better: a component is a file of `limn-components`. A change to the layout still meets `tree.rs`. |
| 6 | CI holds every rule a machine can hold, on every platform limn claims. | Partly. No check of formatting; Linux only; two tests of `limn-dnd` fail now and then. |
| 7 | A release is cut by following a page. | No such page. |

## Done

| Change | Where |
| --- | --- |
| A layout keeps every measurement it makes: no cap on the `MeasurementCache`. | #194 |
| `Frame::duplicate_ids`; state out of sight kept for the 64 most recent IDs; `with_tokens`. | #194 |
| One layout a frame: a focus asked for early waits for the layout that adds its control. | #194 |
| `StateStore`: fourteen maps of `UiTree` became one store with one sweep. | #195 |
| An `Element` is 688 bytes, down from 1,248, under a test that keeps it there. | #196 |
| The layout's tables hash with foldhash; text widths are kept by generation. A third less time. | #197 |
| Measuring changes no state, asserted in debug. | `next` (was #198) |
| These pages, and `layout_bench`. | `next` |
| `CHANGELOG.md` merges by union: two branches that add a line both keep theirs. | `next` |
| The eleven open features, on the core as it is now: the code look, `centered_y`, Markdown's line height, a modal's title, drag handles, the floating surfaces and what was stacked on them (looping icons, a hint with a picture, a host's faces, a radio's accent), and liquid glass. | `next` (were #179, #180, #182, #186 to #193) |
| The engine and the design in crates of their own: `limn-core`, `limn-components`, and `limn` over both. | `next`; stage C, its first step |
| `tree.rs` split along its seams, from 23,860 lines to 13,458, with no change of behaviour. | `next`; stage D.1 |
| What the tree paints by itself takes its look from outside: the focus ring, a coach mark's ring and caret, scrollbars, a tooltip's content, the context menu's view. With nothing given, each is drawn as before. | `next`; stage C.1 |

What the features needed to stand on the new core, so the next one knows:

- State they kept in maps of the tree went to the `StateStore`: an icon's
  motion, a sheet's spring, the tab indicator's lift and material.
- Two tests of a host's faces failed without the default features: they asked
  for text laid out by limn, which paints nothing without the bundled faces.
- The gallery's glass page read its bar before the input, for a loop that
  laid out twice. It reads it before the view is described.

Checked on 2026-11-10, after the cut, the split of `tree.rs` and stage C.1:
the five commands pass (2,380 tests; 1,874 without the default features).
Every name `limn` exported before the cut, it exports now. `layout_bench`
lays out as fast as before the cut: 1.10 to 1.12 ms for 1,000 rows against
1.11 to 1.15, and 7.9 to 8.1 ms for 4,000 against 7.8 to 8.0. Each change of
stage C.1 was compared, draw command by draw command, with what the engine
drew before it, over the cases its own page of notes lists. Not checked:
the gallery running, so nothing here says the windows look as they did.

## Order of work

The order is by what blocks what. A stage does not wait for every item of the
one before it, only for those it names.

### A. One line of work

1. ~~Bring every branch onto the core as it is.~~ Done: `next`.
2. **Point the applications at `next`.** Done in a branch of each, not
   merged: all three build and pass their own suites. Two needed only the
   new paths and `..` in one pattern of `DrawCommand::Icon`. The third, 315
   commits behind, needed no change to compile, and keeps its text on the
   faces it gives egui with `set_text_layout(false)`; see the faces in
   stage B. Left to each application: its CI still fetches the old line of
   limn, and none was run, so nothing was seen.
3. **Close the old pull requests** without merging, once the applications
   build: their work is in `next`.

### B. Settle the contracts

Each of these breaks a host or a backend, so each lands before limn calls
itself stable.

| Item | Closes | What it takes |
| --- | --- | --- |
| Widget commands leave `DrawCommand`: `Spinner` and `ProgressRing` become polylines; `Record` and `EffortSlider` are composed in the core from the commands there are. | Invariant 9 | Finding which shape is missing first (a turned image, for `Record`). Looked at in the gallery: a test does not see it. |
| `Glass`, `GlassSelection` and `TextEffect`, which the features added, are read against R8. | Invariant 9 | Not looked at yet. |
| One shape for a press: position, button, modifiers, click count. `PointerDown`, `PointerDownModified` and `PointerDownEditor` stay as deprecated spellings of it (R14). | A debt | Every test writes `UiEvent::PointerDown(point)`, so the old form must keep compiling. |
| A key the tree derives is a type a host cannot write. | Invariant 3 | `extend/`, `slide/`, `scale/`, `__sheet:` and `popover_surface:` first. |
| An ID taken twice is loud in a debug build. | Invariants 2, 12 | *Needs a decision*: a panic in debug stops an application that works today. |
| What is public by accident: the fields of `Element` and `Style`. | Condition 3 | An audit; then a check of the public API in CI. |
| A host's own faces reach limn's text layout. Done: `attach_fonts` gives `limn::fonts()` the host's faces in egui's fonts (`egui_faces.rs` in `limn-egui`): the first face of each interface family, at its `FontTweak::coords` (`Font::with_variations`), and the faces after limn's emoji face as last resorts (`Fonts::last_resort`). Left: a face a host puts elsewhere in egui's chains, or first in `Monospace`, is egui's alone; a variable face given without coordinates is drawn by limn at its family's weight and by egui at its default; of a `.ttc`, limn may lay out another face of the file than the one named (read from fontique, not tested); a host that calls `limn::set_fonts` after `attach_fonts` took faces keeps them past egui's; with `set_color_emoji`, a picture is fitted to the room of the shaped glyphs in limn's layout and to egui's room for the characters in measured text, so at 16 px `👍🏽` is 19.2 px wide on the first and 16.3 on the second, a keycap 17.6 and 16, and a cluster limn takes from Noto Sans Symbols 2 (`❤️`, keycaps) makes its line 27.2 px tall at 16 px where a measured line stays 19.0 (measured by painting both paths in a probe, not kept as a test); a run with a picture builds its glyph mesh again each frame (`paint_glyphs`). | Found by an application | Each left item is small; the `.ttc` one wants a test with a collection, which the repository has none of. A host's search for missing characters reads `DrawCommand::Glyphs` too: the host's change. |

Small gaps, each a commit of its own:

- ~~`scroll_to_end` and `scroll_to_index` keep a request for an ID that is
  never laid out (invariant 5).~~ Done: a waiting request is kept for the 64
  IDs asked of most recently. It still outlives a layout that does not place
  its area, because two of the applications rely on that: a search's results
  behind their loading state, a conversation behind a pane grown over it.
- `sheet_detents`, `sheet_frames` and `popover_order` are maps of the tree
  that came with liquid glass, from before R3. They belong in the store.
- ~~`Frame::duplicate_ids` does not see the rows of a virtual list (invariant
  2).~~ Done: the rows in view are checked as they are placed.
- The code editor's `EditorView::update` reads the wall clock (invariant 11).
- Durations written as constants in `tree.rs`: a popover's change of view,
  the effort slider (one of its three in `slider_tree.rs`), a tooltip's delay
  (R11).
- `crates/limn-gallery/examples/captured_paste.rs` lays out after each event
  (R6).
- What a tooltip's content floats by itself, a popover or an overlay, is
  placed outside the inert subtree and could take the pointer
  (`Element::inert`).
- A press on a separator or a disabled row of a context menu closes it.
- `ScrollbarAppearance` has public fields, constructors (`Default`,
  `overlay`, `reserved`, `pan_bar`) and builders for all of them but
  `min_thumb_length` and the four opacities, which only a struct literal
  sets: two applications do, over `..ScrollbarAppearance::overlay()`, as
  `limn-components` does for other fields. A field added later breaks no
  literal with `..`. `#[non_exhaustive]` would forbid those literals, so it
  follows only once those five fields have builders and the applications
  use them. (`CoachAnchor` is done: a constructor, builders and
  `#[non_exhaustive]`.)

### C. The engine apart from the design

The aim: limn is an engine anyone builds a design on, as GPUI is, and the
design that ships with it is one design among those that could be. An
application with a look of its own depends on `limn-core`, uses none of
`limn-components`, and changes nothing in the engine to get there.

**Done, the first step: the cut.**

| Crate | Holds | May depend on |
| --- | --- | --- |
| `limn-core` | The engine: elements, layout, events, retained state, text, the frame. | Nothing of the design. |
| `limn-components` | The design: 106 modules, every component the engine does not name. | `limn-core`, through what it makes public. |
| `limn` | Both, re-exported: `use limn::Button` and every import an application has still work. | Both. |

- The compiler holds the line now: a component cannot reach inside the
  engine (R22).
- What the components did reach for was opened as seams, `#[doc(hidden)]
  pub` (R23). `seams_do_not_grow` holds their number: 65 on 2026-11-11,
  down from 86 at the cut. Before it there were none to count, because
  nothing stopped a component from reaching in.
- The engine's tests stand on the engine alone: 738 of them. The 47 that
  need a component are tests of `limn-components` now, and none was lost.

**Left: what the engine still holds of the design.** Each item ends with the
engine offering a general piece and the look moving to `limn-components`.
In order, by how much of a host's own design each one blocks:

1. **What the tree paints by itself**: tooltips, the context menu
   (`context_menu.rs`, `action_style.rs`), scrollbars, the focus ring, a
   coach mark's ring and caret. They take their colors from
   `ComponentPalette` and their measures from constants. A host cannot
   restyle them without a change to the engine. Each takes its look from
   what the element carries, given by the component that asks for it.
   - The focus ring and a coach mark's ring and caret are done, as a `Ring` in
     `Tokens::focus_ring`, `Element::focus_ring_style` and `CoachAnchor`.
   - Scrollbars are done: a vertical bar's buttons, arrows and track and the
     pan bar of `drag_to_pan` take their look from `ScrollbarAppearance`.
     Left: the thumb of a text area (`paint_laid_out_area`) and the code
     editor's own bars (`code_editor.rs`).
   - A tooltip's content can be the component's own (`Tooltip::content`,
     placed by `paint_tooltip` in `tooltip_tree.rs`); still hard-wired are
     the built-in layout's measures, text hints (`TextSpan::tooltip`), the
     code editor's diagnostics and the mark rail's hint.
   - The context menu takes a view (`ContextMenu::view`,
     `UiTree::set_context_menu_view`); still hard-wired: its default look,
     which lives in the engine (`ContextMenuView::default_element`), its
     entrance and its 8-pixel margins in the window.
2. **The controls in `components.rs`**: `Button`, `IconButton`, `Kbd` and
   `Tooltip`, with `spinner.rs` and `scroll_area.rs`. They, and the controls
   that have moved, are builders over kinds the engine paints
   (`ElementKind::Button`, `TextInput`, `TabStrip`). The engine keeps the
   behaviour (press, focus, caret, selection) on a kind with no look; the
   look becomes a component.
   - Done: `Link`, `TextField`, `TextArea`, `SearchField`, `Switch`, `Tabs`,
     `SegmentedControl` and `Popover`, which no engine code built, are
     components of `limn-components`, in a file each, with their tests. The
     helpers only they used went with them, and five seams closed with them.
   - Left: `Button` and `IconButton`, which the code editor's review bar
     builds; `Kbd`, which the context menu builds; `Tooltip`, which the code
     editor's fold and breakpoint controls build; and the kinds themselves.
3. **The widgets in `ElementKind`**: the code editor and its diff
   (`code_editor*.rs`, `code_diff.rs`, `code_syntax.rs`), `Flowchart`,
   `SequenceDiagram` and `diagram.rs`, the plots, `Record`, `EffortSlider`,
   `Sketch`, `Scribble`, `ZoomView`. First a drawing that can say its own
   size and take the pointer; then each becomes a component over it and
   leaves the enum.
4. **Looks**: liquid glass and iOS sheets (`glass.rs`, `liquid.rs`,
   `sheet_geometry.rs`), the words of built-in controls (`labels.rs`), an
   icon set (`file_icons.rs`), and `ComponentPalette` itself, which is the
   design's palette and lives in the engine because the engine paints with
   it. The engine keeps the material a backend renders; the look goes.
5. **The seams.** Each one is a component a host could not have written.
   They close as the items above land: a seam becomes a documented, general
   piece, or its user moves beside it.
6. **One ID the engine reads**: `code_block:auto:`, which a code block
   makes for itself (`is_anonymous_code_scroll_id`, against R2).

### D. Open the core

These move code. Nothing waits behind them now, so they can start.

1. **Split `tree.rs` along its seams**, with no change of behaviour, one move
   per commit, each into a child module of `tree`, as `zoom_view_tree.rs` is.
   Done, from 23,860 lines to about 13,500: its tests (`tree_tests.rs`),
   `DrawCommand` (`draw_command.rs`), `UiEvent`, `UiAction` and `Key`
   (`events.rs`), `Frame` and hit testing (`frame.rs`), the state of text
   inputs (`input_state.rs`), formatted text (`formatted_text.rs`), painting
   tooltips (`tooltip_tree.rs`), scrollbars and sliders (`scrollbar_tree.rs`,
   `slider_tree.rs`), iOS sheets (`sheet_tree.rs`). Left: liquid glass,
   untouched; and what of those seams is still in `tree.rs` because the code
   that stays or the tests use it too (`place_tooltip`, `slider_action`,
   `scrollbar_opacity`, `overflow`, `char_range`, `visual_lines_measured`),
   with `step_sheet`, `sheet_of`, `recede_scene`, `rubber_band` and the
   constants of a sheet's drag.
2. **Split `place_inner` by kind**: each arm of its `match` a function in the
   module of its kind. Done, with no change of behaviour, one family per
   commit: `place_inner` from about 2,900 lines to about 550, and `tree.rs`
   to about 11,100. Text (`text_tree.rs`), controls and mark rails
   (`control_tree.rs`), containers (`container_tree.rs`, a tooltip's anchor
   in `tooltip_tree.rs`), a scroll area and a uniform list
   (`scroll_tree.rs`), pictures and media (`media_tree.rs`, an effort slider
   in `slider_tree.rs`), the code editor and diff (`code_tree.rs`), diagrams,
   unit grids and level bars (`plot_tree.rs`). Each method binds its kind's
   fields with the arm's own pattern and takes the locals of `place_inner`
   it read. Left: the arms that are one call or a few lines (a panel, a
   stack, a shortcut hint, text laid out by limn, a measured list, a
   horizontal scroll, the plots, a zoom view); the work every element shares
   before and after the `match`, in `place_inner`; and `measure_uncached`,
   its measuring twin, one `match` in `tree.rs` still.
3. **Adopt rustfmt**, in one commit named in `.git-blame-ignore-revs`, and
   check it in CI. It touches more than 238 files.

### E. Scale

The cost of a frame grows with the whole tree. These make it grow with what
changed.

1. **Place apart from paint**: positions, regions and accessibility in one
   step, commands in another. It is what lets a subtree be reused.
2. **Subtrees that are reused**: `Element::cached(key, version, child)`. The
   measure first, which is safe now that measuring changes nothing; then its
   commands and regions.
3. **Why the layout grows faster than the rows.** Find the cause with a
   profile before changing anything; see [Cost](architecture.md#cost).
4. **`layout` returns a clone of the frame.** Measure it on a large screen
   before deciding whether to lend the frame instead.

### F. The project

1. **CI on macOS and Windows**: build, test, run the gallery.
2. **No test that fails sometimes.** Two of `limn-dnd` do, and passed when
   run again; the cause of neither is known:
   - `owner_pool_retirement_drops_window_off_caller_and_reuses_only_the_surface_permit`
     failed once in CI on 2026-11-09 ("the admitted window's final Drop runs
     outside the pool mutex"), and in none of 30 local runs.
   - `invalidate_cuts_the_resolved_seal_and_open_reader_until_a_fresh_clipboard_barrier`
     failed once on 2026-11-10 while the whole workspace was tested, and in
     none of six runs of `limn-dnd` alone.
3. **A page for cutting a release**, and tags.
4. **`CONTRIBUTING.md`, templates for issues and pull requests.**
5. **The rename, into a new repository**, last and alone.
   - The name is not chosen. `refine` was checked on 2026-11-10 and is taken:
     a crate of that name is on crates.io, and `refinedev/refine` is a widely
     used React framework. `guil` was checked the same day and is free on
     crates.io for every crate. Check a name there before settling on it.
   - Crate names, `use` paths, pages and the site change in one commit, with
     nothing else under way.
   - The new repository takes the whole history, so `blame` and `bisect`
     keep working.
   - The old repository is archived with a pointer to the new one, not
     deleted. Five crates are on crates.io as `limn*` at 0.1.43: each gets a
     last release under its old name that points to the new one.

## Not planned

- Scoping IDs by their place in the tree. See
  [Lineage](architecture.md#lineage) for why.
- A window, an event loop or a renderer inside `limn-core`. The host keeps
  them.
