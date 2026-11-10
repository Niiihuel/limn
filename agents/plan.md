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
| 1 | One line of work. No feature waits behind it in a branch, and applications build on it or on a tag. | The line exists: `next` holds every branch. The applications still build on their own branches. |
| 2 | Every [invariant](invariants.md) is **Held**. | Six are not: 1, 2, 3, 5, 9 and 12. |
| 3 | What a host or a backend compiles against is settled: every breaking change known today has landed. | No. See stage B. |
| 4 | A new control edits no file that another change is likely to edit. | No. The eleven features that landed added 2,000 lines to `tree.rs`. |
| 5 | CI holds every rule a machine can hold, on every platform limn claims. | Partly. No check of formatting; Linux only; two tests of `limn-dnd` fail now and then. |
| 6 | A release is cut by following a page. | No such page. |

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

What the features needed to stand on the new core, so the next one knows:

- State they kept in maps of the tree went to the `StateStore`: an icon's
  motion, a sheet's spring, the tab indicator's lift and material.
- Two tests of a host's faces failed without the default features: they asked
  for text laid out by limn, which paints nothing without the bundled faces.
- The gallery's glass page read its bar before the input, for a loop that
  laid out twice. It reads it before the view is described.

Checked after the last merge, on 2026-11-10: the five commands pass (2,351
tests; 1,845 without the default features). No public name of `limn` or
`limn-egui` that the applications' branches had is missing from `next`, and
no line of their changelogs. The layout of `layout_bench` is 3% slower than
before the features: 1.15 ms for 1,000 rows, 7.8 ms for 4,000.

## Order of work

The order is by what blocks what. A stage does not wait for every item of the
one before it, only for those it names.

### A. One line of work

1. ~~Bring every branch onto the core as it is.~~ Done: `next`.
2. **Point the applications at `next`**, one at a time, and fix what each
   meets. One of them builds on a limn 315 commits behind `main`.
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

Small gaps, each a commit of its own:

- `scroll_to_end` and `scroll_to_index` keep a request for an ID that is
  never laid out (invariant 5).
- `sheet_detents`, `sheet_frames` and `popover_order` are maps of the tree
  that came with liquid glass, from before R3. They belong in the store.
- `Frame::duplicate_ids` does not see the rows of a virtual list (invariant 2).
- The code editor's `EditorView::update` reads the wall clock (invariant 11).
- Durations written as constants in `tree.rs`: a popover's change of view,
  the effort slider, a tooltip's delay (R11).
- `crates/limn-gallery/examples/captured_paste.rs` lays out after each event
  (R6).

### C. Open the core

These move code. Nothing waits behind them now, so they can start.

1. **Split `tree.rs` along its seams**, with no change of behaviour, one move
   per commit: `DrawCommand`; `UiEvent`, `UiAction` and `Key`; `Frame` and hit
   testing; the state of text inputs; formatted text; tooltips; sheets and
   glass. Each becomes a child module of `tree`, as `zoom_view_tree.rs` is.
2. **Split `place_inner` by kind**: each arm of its `match` a function in the
   module of its kind.
3. **Take the widgets out of `ElementKind`.** A drawing that can say its own
   size, then `Flowchart`, `SequenceDiagram`, the plots and the bars as
   components over it. Each removes a variant and an arm.
4. **Adopt rustfmt**, in one commit named in `.git-blame-ignore-revs`, and
   check it in CI. It touches more than 238 files.

### D. Scale

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

### E. The project

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
     used React framework. Check a name on crates.io for all the crates before
     settling on it.
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
- A window, an event loop or a renderer inside `limn`. The host keeps them.
