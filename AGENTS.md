# AGENTS.md

Rules for whoever changes this repository: a person, or an AI assistant such
as Claude Code, Codex, Cursor or Zed's agent. Read this page before any task.
The pages under [`agents/`](agents/README.md) hold the detail it points to.

## 1. What limn is

limn is a native UI library for Rust. Each frame an application describes its
window as a tree of elements. limn lays the tree out, keeps the state of every
control between frames, and returns a `Frame`: drawing commands, hit regions
and accessibility nodes. A backend paints the frame and translates input. The
core needs no window, no GPU and no async runtime.

| Crate | Holds | Depends on |
| --- | --- | --- |
| `limn` | The core: elements, layout, events, retained state, the frame, and every component. | `limn-editor`; `limn-language` with the `syntax` feature |
| `limn-egui` | The backend: paints a frame with egui and wgpu, translates input, publishes accessibility. | `limn` |
| `limn-editor` | The text buffer and viewport under the code editor. | none of the others |
| `limn-language` | Tree-sitter highlighting, Markdown analysis, a language server client. | `limn-editor` |
| `limn-dnd` | Files dragged in from other applications, and out to them, on Wayland. | none of the others |
| `limn-video` | Short muted H.264 clips on a `limn-egui` surface. | `limn`, `limn-egui` |
| `limn-gallery` | Every component running, with hot reload. The site is built from it. | all but `limn-language` |

## 2. Read first

1. This page.
2. [`docs/concepts.md`](docs/concepts.md): the model, as a host sees it.
3. [`agents/architecture.md`](agents/architecture.md): how the core is built.
4. [`agents/invariants.md`](agents/invariants.md): what always holds, and what checks it.
5. [`agents/workflow.md`](agents/workflow.md): branch, checks, commit, pull request.
6. The role that fits the task, in [`agents/roles/`](agents/README.md#roles).

[`agents/plan.md`](agents/plan.md) says what is being worked on, and in what order.

## 3. Rules

Each rule gives its reason, and its check where one exists. A rule with no
automatic check is still a rule: a reviewer holds it. Cite a rule by its
number in a review.

### The model

**R1. A component stores nothing between frames.** It takes its value and
reports actions; the host owns every value a user can change.
*Why:* the window stays a function of the host's state, so a screen is tested
by laying it out. *Check:* `crates/limn/tests/controls.rs`, review.

**R2. Every mounted control has an ID of its own, and an ID is never parsed.**
A component derives the IDs it creates from its caller's, as
`format!("{id}:body")`. No code learns anything by splitting an ID.
*Why:* the ID is the control's memory and its name in actions; two controls
under one ID share hover, focus and transitions. *Check:*
`Frame::duplicate_ids()` is empty in a test of the screen.

**R3. What the tree remembers for an element goes in the `StateStore`.** No
new field on `UiTree`, no new map keyed by ID.
*Why:* one store sweeps every kind by one rule; a map of its own is a sweep
someone forgets. *Check:* review. See [State](agents/architecture.md#state).

**R4. Measuring changes no state.** Inside a layout, state changes while
placing, through `states_mut!`.
*Why:* a parent may measure a child twice, or measure and never place it; the
answer must be the same each time. *Check:* a debug assertion that the whole
test suite runs under.

**R5. Nothing kept grows with the IDs a host has ever used.** State lasts
while its control is laid out, or is kept dormant for the 64 IDs of its kind
used most recently.
*Why:* a chat makes one ID per message, for ever. *Check:*
`state_out_of_sight_is_kept_only_for_the_ids_laid_out_most_recently`.

**R6. A host lays out once a frame, after its input.** What is asked about a
control (focus it, reveal it, scroll to it) is queued for the next layout. No
API may need a layout of its own from the host.
*Why:* a second layout doubles the cost of every frame. *Check:* review; the
loop in [`docs/backend.md`](docs/backend.md#the-frame-loop).

### The boundaries

**R7. `limn` names no window, GPU or backend.** No type of a third-party
crate appears in its public API, but `web_time::Instant`, the clock that also
runs in a browser. A new dependency of `limn` needs its reason in the pull
request.
*Why:* a frame is plain data that any backend can paint and any test can
read. *Check:* review of `crates/limn/Cargo.toml`.

**R8. `DrawCommand` gets no command for one widget.** A new look is built from
the commands there are: rectangles, gradients, meshes, polylines, glyphs,
images, clips.
*Why:* every backend must paint every command; a command per widget makes
each widget every backend's work. *Check:* review. `Spinner`, `Record`,
`ProgressRing` and `EffortSlider` are the debt this rule stops from growing;
see the [plan](agents/plan.md).

**R9. `ElementKind` gets a variant only for a new way to lay out.** A new
control is a component: a struct in a file of its own that renders to
elements. What it draws itself goes through `Element::drawing`.
*Why:* every variant is matched across `tree.rs`; a component touches none of
it. *Check:* review.

**R10. New code does not go in `tree.rs`.** What a feature needs inside the
layout goes in a child module of `tree`, as `zoom_view_tree.rs` and
`horizontal_scroll_tree.rs` are, and its tests in a `*_tests.rs` beside it.
The call into that module is all `tree.rs` gains.
*Why:* a file every change meets is a conflict in every pull request.
*Check:* review.

### Components

**R11. A component follows the conventions in
[`docs/contributing.md`](docs/contributing.md#conventions).** Colors come from
the palette; radii and durations from `tokens()`; numbers from a caller are
validated; a disabled control renders passive elements; whatever can be
disabled has both `disabled()` and `enabled(bool)`.
*Check:* `crates/limn/tests/conventions.rs` and `tests/controls.rs`, for
numbers and disabling, for the components listed there. The palette and the
tokens: review.

**R12. Whatever moves can stand still.** An animation takes its duration from
`tokens().motion`, settles at once under reduced motion, and asks for its
frames through `UiTree::animation_repaint_after`, never a timer of the host.
*Check:* a test with `layout_at` at chosen instants; see
[`docs/testing.md`](docs/testing.md#animation).

### The public API

**R13. Everything public has a doc comment.** It says what the name does not:
the unit, the clamp, what is reported.
*Check:* `#![warn(missing_docs)]`, which CI denies.

**R14. A name that changes keeps its old form, `#[deprecated]`, with the new
name in its note.** A change that cannot do so is marked `**Breaking**` in the
changelog, with what a host must change.
*Why:* applications build on `main`.

### The work

**R15. A change of behaviour comes with a test that fails without it.** Tests
open no window: they lay out a tree and read the frame.

**R16. A claim about speed comes with numbers.** They are measured with
`layout_bench` as [the workflow](agents/workflow.md#measuring) says, before and
after, and written in the pull request.

**R17. One topic, one branch from `main`, one pull request.** Nothing is
pushed to `main`. A change a host can see adds its line to `CHANGELOG.md`.
Until the repository opens to contributors, [the plan](agents/plan.md#how-work-lands-now)
says how work lands instead.

**R18. `cargo fmt` is never run over the workspace.** The tree is not in
rustfmt's style, and CI does not check it: a reformat buries the change and
conflicts with every open branch. Write as the code around the change does.

**R19. The page changes with the behaviour, in the same pull request.**
`docs/` is for hosts; `agents/` is for whoever changes limn. Pages name
symbols and files, never line numbers.

**R20. Nothing is copied from another project.** Zed's GPUI and Warp's WarpUI
are read for their ideas; [the architecture](agents/architecture.md#lineage)
says which. Most of both repositories is under the GPL or the AGPL, which
limn's `MIT OR Apache-2.0` cannot take in.

**R21. Say only what was checked.** A statement about what limn does is read
from the code or shown by a test; a number is measured. What was not checked
is said to be unchecked.

## 4. Commands

CI runs these on Rust 1.99.1, and a pull request passes them all:

```sh
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p limn-gallery --all-targets --features hot -- -D warnings
cargo test --workspace
cargo test -p limn --no-default-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

On Linux, `limn-dnd` links the system's Wayland client and winit needs
xkbcommon: `libwayland-dev`, `libxkbcommon-dev` and `pkg-config` on Debian.

While editing, run the narrowest thing that answers:
`cargo test -p limn <name of the test>`.

## 5. Roles

| Role | File | For |
| --- | --- | --- |
| Core | [`agents/roles/core.md`](agents/roles/core.md) | The tree, layout, events, retained state. |
| Component | [`agents/roles/component.md`](agents/roles/component.md) | Adding or changing a component. |
| Backend | [`agents/roles/backend.md`](agents/roles/backend.md) | `limn-egui`, and what a frame asks of a backend. |
| Reviewer | [`agents/roles/reviewer.md`](agents/roles/reviewer.md) | Reading a change against these rules before it merges. |

## 6. Before saying it is done

- [ ] The commands of section 4 pass, and the output was read.
- [ ] A test fails without the change (R15).
- [ ] `CHANGELOG.md` and the pages that describe the behaviour are updated (R17, R19).
- [ ] Anything not verified is named in the pull request (R21).
