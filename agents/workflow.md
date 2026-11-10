# Workflow

From a task to a merged pull request: where to work, what to run, how to
write the commit and the pull request, and how to measure. The rules are in
[`AGENTS.md`](../AGENTS.md#3-rules); this page is how to follow them.

Until the new repository opens there are no pull requests: [the plan](plan.md#how-work-lands-now)
says how work lands meanwhile. Everything else on this page applies now.

## Branch

One topic, one branch from `main`, named `<type>/<topic>`: `feat/drag-handle`,
`fix/centered-y-shrink`, `perf/layout-hashing`, `docs/agents`.

Work in a worktree beside the clone, so `main` and other topics stay as they
are:

```sh
git fetch origin
git worktree add ../limn-<topic> -b <type>/<topic> origin/main
```

- Each worktree builds in its own `target/`. A `CARGO_TARGET_DIR` shared by
  two worktrees has mixed the builds of two branches.
- A change that needs another open pull request branches from that one's
  branch, and says so in its description. It is retargeted to `main` when the
  first merges.
- Remove the worktree and the branch once the pull request is merged:
  `git worktree remove ../limn-<topic>`.

## Change

1. Read the files the change touches, and the page that describes the
   behaviour. Find every caller of what changes.
2. Write the test first when the change is a fix: it fails, then it passes.
3. Make the smallest change that is complete: the code, its test, its doc
   comment, its page, its changelog line. A change that closes an open
   invariant or a debt also updates [`invariants.md`](invariants.md),
   [`architecture.md`](architecture.md#known-debts) and [`plan.md`](plan.md).
4. Write as the code around it does. Do not reformat what the change does not
   touch (R18).

Where things go:

| The change | Goes in |
| --- | --- |
| A new control or a new look | A component: a file of its own in `crates/limn-components/src/`. See [`docs/contributing.md`](../docs/contributing.md). |
| Something the layout must remember for an element | A type in the `StateStore`. See [State](architecture.md#state). |
| Something a feature needs inside the layout | A child module of `tree`, `<feature>_tree.rs`, with its tests in `<feature>_tests.rs`. |
| How a frame is painted | `crates/limn-egui`, and [`docs/backend.md`](../docs/backend.md) when the contract changes. |
| A component's demonstration | A page in `crates/limn-gallery/src/pages/`. |

## Check

While editing, run the narrowest thing that answers, in the crate the test
or the change is in:

```sh
cargo test -p <crate> <name of the test>
cargo clippy -p <crate> --all-targets -- -D warnings
```

| The test or the change is of | Its crate | Its files |
| --- | --- | --- |
| The engine | `limn-core` | `crates/limn-core/src/` |
| A component | `limn-components` | `crates/limn-components/src/` |
| A screen, through the public API | `limn` | `crates/limn/tests/` |

Before the pull request, run what CI runs, on Rust 1.99.1, and read the
output:

```sh
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p limn-gallery --all-targets --features hot -- -D warnings
cargo test --workspace
cargo test -p limn -p limn-core -p limn-components --no-default-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

- CI pins the compiler so that a newer clippy cannot fail a pull request
  that changed nothing. Use the same version: `rustup toolchain install 1.99.1`.
- `--no-default-features` builds without Tree-sitter grammars, as the
  browser's build does, and without the bundled fonts. A test that needs
  either says so with `#[cfg(feature = "...")]`.
- A change to how something looks is also looked at: run the gallery
  (`cargo run -p limn-gallery`), in both themes and with square corners. A
  test does not see a label two pixels low.
- A test that fails only sometimes is a defect. Say so in the pull request
  with its name and its message; do not run it again until it passes and
  leave it at that.

## Commit

```
<type>(<scope>): <what is true once this is in>
```

| Part | Is |
| --- | --- |
| `type` | `feat`, `fix`, `perf`, `refactor`, `docs`, `test` or `chore`. |
| `scope` | The crate without its prefix, or the area: `limn`, `egui`, `layout`, `menu`, `code`. Omitted for the whole repository. |
| The sentence | In the present tense, lower case, no full stop. It says what limn does now, not what was done to it. |

```
feat(limn): a popover can grow out of its corner nearest its trigger
fix(limn): a lifted lens keeps its capsule
perf(limn): the focus walk's set hashes with foldhash
refactor(limn): measuring changes no state
```

The body, when there is one, says why. One commit holds one change that
builds and passes on its own.

## Changelog

Add one bullet at the top of `## Unreleased` in `CHANGELOG.md`.
`.gitattributes` merges that file by union: when two branches each add a
line, a merge keeps both.

- It says what changed for a host, in a sentence a host understands without
  the code: the name of what is new, what it does, what it replaces.
- A change a host must act on starts with `**Breaking**:` and says what to
  change.
- A change no host can see (a refactor, a test) has no bullet.

## Pull request

The title is the commit's sentence, or the topic when there are several
commits. The description has these parts, each only if it has something to
say:

```markdown
## Why
What was wrong or missing, and how it was found.

## What
The change, by behaviour. Not a list of files.

## Behaviour and cost
What a host sees differently. Numbers, before and after, when speed is claimed.

## Test plan
- [x] The commands that were run, with the count that passed.
- [ ] What was not verified, and why.
```

A maintainer merges a pull request once CI passes and it has been read
against the rules; [the reviewer's role](roles/reviewer.md) says how.

## Measuring

`crates/limn/examples/layout_bench.rs` is the measure a change to the layout
is judged by (R16). It reports the best of 30 frames for each row count.

```sh
cargo build --release -p limn --example layout_bench
taskset -c 6 target/release/examples/layout_bench
```

To compare two commits:

1. Build each in its own worktree, so each has its own `target/`.
2. Pin both to the same core with `taskset -c <core>`. Pick one that is idle.
3. Run them in turn, three times each: before, after, before, after.
4. Report the range of each, not one figure.

Read the result with care:

- Pinned, three runs of one build differ by about 2%. A difference smaller
  than that is not a result.
- Not pinned, on a machine with a browser open and the `powersave` governor,
  runs of one build have differed by 30%. Such numbers show nothing.
- The benchmark is one shape of tree. A change aimed at another shape (text
  that wraps, a deep tree, many popovers) adds its own row count or its own
  view to the example, in the same pull request.

To find where the time goes when perf events are not allowed
(`perf_event_paranoid` above 1): a scratch crate that runs the same loop under
the `pprof` crate, with its default DWARF unwinding and
`[profile.release] debug = true`, and sums the samples by symbol. This was
last done on 2026-11-09; the recipe is not kept in the repository.
