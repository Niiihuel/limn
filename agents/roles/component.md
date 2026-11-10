---
name: component
description: Adds or changes a limn component: a struct in a file of its own under crates/limn-components/src that renders to elements, with its tests, its gallery page and its guide. Use for a new control, a new option or variant of an existing one, or a fix to how a component looks or what it reports.
---

# Component

## Role

Packages a decision that should be made once: a layout that is easy to get
wrong, a set of states, a keyboard behaviour. Builds it from the elements
there are, without touching the layout.

## When to use it

- A new control, or an option on one that exists.
- A component looks wrong, or reports the wrong action.

## Read before starting

1. [`AGENTS.md`](../../AGENTS.md)
2. [`docs/contributing.md`](../../docs/contributing.md), whole: it is the
   procedure, and this page does not repeat it
3. [`docs/components.md`](../../docs/components.md): what exists already
4. `crates/limn-components/src/progress.rs`: a short, complete component, the one
   `docs/contributing.md` walks through
5. The component nearest to the new one, and its tests

## Rules

- **Look for it first.** Many requests are an option on a component that
  exists.
- **Controlled** (R1): it takes its value and reports actions.
- **Every ID it creates derives from the caller's** (R2):
  `format!("{id}:body")`.
- **No literal color, radius or duration** (R11): the palette and `tokens()`.
- **It does not touch `tree.rs` or `element.rs`.** If it seems to need to,
  stop: that is a task for the [core](core.md) role, in a pull request of its
  own, first.
- **Show every state in the gallery**, not one happy example: disabled,
  loading, empty, long text, narrow.

## Delivers

- `crates/limn-components/src/<name>.rs`, with `mod` and `pub use` in
  `crates/limn-components/src/lib.rs`.
- Tests in the same file: the geometry or paint that defines it, each action
  it reports, bad input to each numeric builder, a narrow width if it holds
  text.
- Its row in `crates/limn/tests/controls.rs` if it reports an action when
  activated, and in `crates/limn/tests/conventions.rs` if it joins a family
  there.
- Its page in `crates/limn-gallery/src/pages/`, listed in `PAGES` with
  `since`.
- Its row in `docs/components.md` and its guide `docs/components/<id>.md`.
- A changelog bullet.

## Before finishing

- [ ] `cargo test -p limn -p limn-components -p limn-egui -p limn-gallery` passes.
- [ ] Looked at in the gallery, in both themes and with square corners.
- [ ] `frame.duplicate_ids()` is empty with two of it on one screen.
- [ ] Every public item has a doc comment.
