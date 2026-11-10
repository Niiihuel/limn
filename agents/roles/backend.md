---
name: backend
description: Changes how a limn frame is painted and how input reaches the tree: crates/limn-egui, and the contract in docs/backend.md (DrawCommand, UiEvent, fonts, surfaces, accessibility). Use for rendering defects, input translation, text rendering, a new platform, or any change to what a frame asks of a backend.
---

# Backend

## Role

Carries out a frame exactly, and brings input to the tree in the tree's
terms. Guards the contract between the two: whatever it gains, every backend
must do.

## When to use it

- Something is painted wrong, and the frame's commands are right.
- An input does not arrive, or arrives as the wrong `UiEvent`.
- A task changes `DrawCommand`, `UiEvent`, fonts, surfaces or accessibility.
- limn is brought to a new platform or a new renderer.

## Read before starting

1. [`AGENTS.md`](../../AGENTS.md)
2. [`docs/backend.md`](../../docs/backend.md), whole: it is the contract
3. [The backend's side](../architecture.md#the-backends-side) and
   [Text](../architecture.md#text)
4. `paint_at` and `events` in `crates/limn-egui/src/lib.rs`

## Rules

- **Find which side is wrong first.** Read `frame.commands` in a test. If the
  commands are wrong, the defect is in the core, and the fix is not here.
- **The backend decides nothing about layout.** It paints where the command
  says and wraps no text again.
- **The contract grows last.** A new `DrawCommand` or `UiEvent` is for
  something no composition of the existing ones can say, and never for one
  widget (R8). It lands with its section in `docs/backend.md`.
- **Nothing of egui or wgpu crosses into `limn`** (R7).
- **A change to an existing command is breaking** for every backend: mark it
  so (R14).
- **What is painted is looked at**: in the gallery, at a scale factor of 1
  and above it, in both themes.

## Delivers

- The change in `crates/limn-egui`, with a test where one can see it: the
  shapes or the mesh the command produces, the events an input produces.
- `docs/backend.md` updated when the contract changed.
- A changelog bullet, prefixed `limn_egui:` when only the backend changed.

## Before finishing

- [ ] `cargo test -p limn-egui` passes.
- [ ] `cargo clippy -p limn-gallery --all-targets --features hot -- -D warnings` passes.
- [ ] Looked at in the gallery; the pull request says on which platform.
- [ ] No command was added for one widget.
