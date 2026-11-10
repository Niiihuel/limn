# Plan: stable, then renamed

limn takes a new name and opens to contributors once it is stable. This page
says what stable means here, what is done, and the order of what is left. It
is the one page here that changes every week: the pull request that changes
its facts changes the page.

Last brought up to date: 2026-11-09.

## What stable means

| # | Condition | Today |
| --- | --- | --- |
| 1 | `main` is the only line of work. No feature waits behind it in a branch, and applications build on `main` or on a tag. | No. Eleven feature pull requests are open and ten conflict with `main`. |
| 2 | Every [invariant](invariants.md) is **Held**. | Six are not: 1, 2, 3, 5, 9 and 12. |
| 3 | What a host or a backend compiles against is settled: every breaking change known today has landed. | No. See stage B. |
| 4 | A new control edits no file that another change is likely to edit. | No. `CHANGELOG.md` is in every open pull request, and `tree.rs` in most. |
| 5 | CI holds every rule a machine can hold, on every platform limn claims. | Partly. No check of formatting; Linux only. |
| 6 | A release is cut by following a page. | No such page. |

## Done

| Change | Pull request |
| --- | --- |
| A layout keeps every measurement it makes: no cap on the `MeasurementCache`. | #194 |
| `Frame::duplicate_ids`; state out of sight kept for the 64 most recent IDs; `with_tokens`. | #194 |
| One layout a frame: a focus asked for early waits for the layout that adds its control. | #194 |
| `StateStore`: fourteen maps of `UiTree` became one store with one sweep. | #195 |
| An `Element` is 688 bytes, down from 1,248, under a test that keeps it there. | #196 |
| The layout's tables hash with foldhash; text widths are kept by generation. A third less time. | #197 |
| Measuring changes no state, asserted in debug. | #198, open and passing |
| These pages, and `layout_bench`. | this one |

## Order of work

The order is by what blocks what. A stage does not wait for every item of the
one before it, only for those it names.

### A. Land what is written

Until the queue is empty, every change to the core makes it longer to land,
and every change that moves code makes it harder.

| Item | State on 2026-11-09 |
| --- | --- |
| #198 `feat/pure-measure` | Merges clean, CI passes. |
| #180 `feat/code-look` | Merges clean. |
| #182, #186, #187, #192 | Conflict in `CHANGELOG.md` and nowhere else. |
| #179 `feat/discord-surfaces`, and #188, #189, #190, #193 stacked on it | Conflict in `tree.rs`, `docs/backend.md` and `CHANGELOG.md`. #190 fails two tests of `text_effect_tests` in CI. |
| #191 `feat/aurora-liquid-glass` (6,300 lines) | Conflicts in `tree.rs`, `element.rs`, the gallery's `main.rs` and `CHANGELOG.md`. |

1. **Merge #198.**
2. **Stop the changelog from conflicting.** All ten conflicting pull requests
   conflict in `CHANGELOG.md`, and four of them only there: each adds a bullet
   on the same line. Proposed: one small file per change in a `changes/`
   folder, gathered into `CHANGELOG.md` when a release is cut. *Needs the
   maintainer's decision*; R17 and [the workflow](workflow.md#changelog)
   change with it.
3. **Bring each pull request up to `main` and merge it**, smallest first:
   #180, then #192, #186, #187, #182, then the stack on #179, then #191.
4. **Move the applications from their integration branches to `main`.**

### B. Settle the contracts

Each of these breaks a host or a backend, so each lands before limn calls
itself stable. They change code where it is, so they can go beside stage A;
those that touch `tree.rs` are easier after it.

| Item | Closes | What it takes |
| --- | --- | --- |
| Widget commands leave `DrawCommand`: `Spinner` and `ProgressRing` become polylines; `Record` and `EffortSlider` are composed in the core from the commands there are. | Invariant 9 | Finding which shape is missing first (a turned image, for `Record`). Looked at in the gallery: a test does not see it. |
| One shape for a press: position, button, modifiers, click count. `PointerDown`, `PointerDownModified` and `PointerDownEditor` stay as deprecated spellings of it (R14). | A debt | Every test writes `UiEvent::PointerDown(point)`, so the old form must keep compiling. |
| A key the tree derives is a type a host cannot write. | Invariant 3 | `extend/`, `slide/` and `popover_surface:` first. |
| An ID taken twice is loud in a debug build. | Invariants 2, 12 | *Needs a decision*: a panic in debug stops an application that works today. |
| What is public by accident: the fields of `Element` and `Style`. | Condition 3 | An audit; then a check of the public API in CI. |

Small gaps found while these pages were checked against the code, each a
pull request of its own:

- `scroll_to_end` and `scroll_to_index` keep a request for an ID that is
  never laid out (invariant 5).
- `Frame::duplicate_ids` does not see the rows of a virtual list (invariant 2).
- The code editor's `EditorView::update` reads the wall clock (invariant 11).
- Durations written as constants in `tree.rs`: a popover's change of view,
  the effort slider, a tooltip's delay (R11).
- `crates/limn-gallery/examples/captured_paste.rs` lays out after each event
  (R6).

### C. Open the core

These move code, so they wait for stage A.

1. **Split `tree.rs` along its seams**, with no change of behaviour, one move
   per pull request: `DrawCommand`; `UiEvent`, `UiAction` and `Key`; `Frame`
   and hit testing; the state of text inputs; formatted text; tooltips. Each
   becomes a child module of `tree`, as `zoom_view_tree.rs` is.
2. **Split `place_inner` by kind**: each arm of its `match` a function in the
   module of its kind.
3. **Take the widgets out of `ElementKind`.** A drawing that can say its own
   size, then `Flowchart`, `SequenceDiagram`, the plots and the bars as
   components over it. Each removes a variant and an arm.
4. **Adopt rustfmt**, in one commit named in `.git-blame-ignore-revs`, and
   check it in CI. Only with the queue empty: it touches 238 files.

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
2. **No test that fails sometimes.** `limn-dnd`'s
   `owner_pool_retirement_drops_window_off_caller_and_reuses_only_the_surface_permit`
   failed once in CI on 2026-11-09 ("the admitted window's final Drop runs
   outside the pool mutex") and passed when run again; it did not fail in 30
   local runs. The cause is not known.
3. **A page for cutting a release**, and tags.
4. **`CONTRIBUTING.md`, templates for issues and pull requests.**
5. **The rename**, last and alone: crate names, `use` paths, pages, the site
   and the repository in one pull request, with nothing else open, since it
   conflicts with everything. Five crates are on crates.io as `limn*` at
   0.1.43: each gets a last release under its old name that points to the
   new one.

## Not planned

- Scoping IDs by their place in the tree. See
  [Lineage](architecture.md#lineage) for why.
- A window, an event loop or a renderer inside `limn`. The host keeps them.
