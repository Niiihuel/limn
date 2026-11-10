---
name: core
description: Changes limn's engine: the tree, layout, events and retained state in crates/limn-core/src/tree.rs, element.rs, state_store.rs and the *_tree.rs modules. Use when a task changes how elements are measured, placed, hit, focused, animated or remembered, or when a component needs something the layout does not offer yet.
---

# Core

## Role

Keeps the engine small and its invariants true. A change here is paid for by
every frame of every application, and read by every later change.

## When to use it

- A task changes `tree.rs`, `element.rs`, `state_store.rs`, a cache, or a
  `*_tree.rs` module.
- A component cannot be built from the elements there are.
- A frame costs too much, or state grows.

## Read before starting

1. [`AGENTS.md`](../../AGENTS.md)
2. [`architecture.md`](../architecture.md), whole
3. [`invariants.md`](../invariants.md)
4. [`plan.md`](../plan.md): the stage the task belongs to, and what it waits for
5. The functions the change touches, and every caller of each

## Rules

- **Ask first whether the core must change.** An option on an element, a
  component over existing elements, or `Element::drawing` often does it (R9).
- **State goes in the `StateStore`**, as a type of its own, changed while
  placing and only read while measuring (R3, R4).
- **Say how long the state lasts** in the doc comment of its type: while laid
  out, or dormant among the 64 (R5).
- **New code for a feature goes in a child module of `tree`**, not in
  `tree.rs` (R10).
- **A request about a control is queued**; it never needs the control to be
  in the last layout (R6).
- **Nothing a backend must learn**: no new `DrawCommand` for a widget (R8).
- **Measure before and after** when the change is on the path of every
  element: `measure`, `place`, a cache, `Element`'s size (R16).
- **A pass that repeats costs a whole pass.** A new reason for `layout_at` to
  run again comes with its measured cost.

## Delivers

- The change, with a test that fails without it. For state: a test that it is
  forgotten when its control goes.
- Numbers from `layout_bench`, before and after, when R16 applies.
- [`architecture.md`](../architecture.md) and [`invariants.md`](../invariants.md)
  updated when what they say changed.
- A changelog bullet when a host can see the change.

## Before finishing

- [ ] The suite passes in a debug build, where `states_mut!` asserts.
- [ ] `cargo test -p limn -p limn-core -p limn-components --no-default-features` passes.
- [ ] `tree.rs` is not longer than it was, or the pull request says why.
- [ ] `Element` is within its size budget.
- [ ] No new field on `UiTree` holds state for one element.
