# Hot reload

Save a file and the running window takes the change in under a second, with
its state intact: the page you were on, what you typed, the scroll position.
This page is for an application built on limn; the gallery uses the same
calls.

limn does not reload by itself. It runs under
[`dx`](https://dioxuslabs.com/learn/0.7/getting_started/), the Dioxus tool,
which recompiles only what changed and patches the new code into the running
process. The library's part is three calls in `limn::hot`.

## Set it up

Turn on the `hot` feature of `limn`, behind a feature of your own so that a
release build never has it:

```toml
[features]
hot = ["limn/hot", "limn-egui/hot"]
```

Then three lines. Each is a plain call that does nothing without the feature,
so there is nothing to write twice and no `cfg` in your code:

```rust
fn main() -> eframe::Result {
    // 1. First thing in `main`: find `dx`, if it is serving.
    limn::hot::connect();
    // ... open the window
}

// 2. Once, when the window exists: paint again when a patch lands.
limn_egui::repaint_on_patch(&ctx);

// 3. Every frame: run what you edit through the patch table.
tree.set_root(limn::hot::call(|| view(&state, window)));
for action in tree.handle_event(event) {
    limn::hot::call(|| update(&mut state, &action));
}
```

`crates/limn-egui/examples/counter.rs` is a whole application with those
three lines. In your application's directory, run it under `dx` and edit it
while it is open:

```sh
dx serve --hot-patch --features hot
```

In limn's own repository, the application to run this way is the gallery:
`dx serve --hot-patch -p limn-gallery --features hot`. `dx` cannot patch an
example of `limn-egui` there, because it tries to rebuild the crates of the
workspace that depend on it and were not part of the build.

With a backend other than `limn-egui`, give `limn::hot::on_patch` whatever
asks your window to paint again.

## Why the state survives

A patch replaces functions, not values. Your state lives in your application,
outside the functions that were patched, so the next frame calls the new
`view` with the same state and the `UiTree` still remembers focus, scroll
and every control's own state by its ID.

Only what runs inside `limn::hot::call` is looked up again after a patch. A
function called outside it keeps running as it was compiled at launch.

## What needs a restart

| Change | Why |
| --- | --- |
| The fields of a live struct, such as your state or a component in use | Values already in memory keep the old layout. |
| Code that runs once at startup, such as `install_fonts` | It has already run. |
| A dependency, or a `Cargo.toml` | It is not part of what is patched. |

`dx` restarts the application by itself when it cannot patch. To make that
restart land where you were, keep your state across it:

```rust
// After each action:
limn::hot::remember("my-app", &state.serialize());

// At launch:
let state = limn::hot::restore("my-app")
    .map(|text| State::deserialize(&text))
    .unwrap_or_default();
```

The state is yours to turn into text and back. It is kept in a file in the
system's temporary directory, and only in a build with the `hot` feature: a
release build writes nothing and restores nothing.

A state held in a map keyed by ID, as the gallery's is, takes a new entry
without changing a struct, so adding a control never needs a restart.

## Costs and limits

- `limn`'s `hot` feature pins `subsecond` and `dioxus-devtools` to one exact
  version, because they must be the version of the `dx` binary: 0.7.9 today.
  With another `dx`, patches are refused.
- Each patch loads new code and leaves the old in memory, a few megabytes
  each time. Restart after a long session.
- The first build under `dx` is a full build, in a target directory of its
  own.
- It works on desktop targets. In a browser build the feature compiles to
  nothing.
- Dioxus is only this development tool. Nothing depends on it without the
  feature.
