# Gallery and hot reload

The gallery is an application that shows every component in both themes. Use
it to browse what exists, to develop a component with live reload, and to
capture screenshots.

It lives in `crates/limn-gallery`. Run everything from `nix-shell`.

## Opening the gallery

```sh
cargo run -p limn-gallery
```

The sidebar lists the pages. "Tema claro" at the bottom switches theme, and
the line beneath it shows the last action received, which is a quick way to
check that a control reports what you expect.

The page **Forma y movimiento** changes the [tokens](tokens.md) live: square,
default or round corners, three paces, and a switch for reduced motion.

## Hot reload

```sh
dx serve --hot-patch -p limn-gallery --features hot
```

Save a file in `limn` or in the gallery and the running window updates in
under a second. The page you were on, the scroll position, focus and every
value you typed are kept.

### How it works

`dx` recompiles only what changed and patches the new code into the running
process. The gallery calls its `view` and `update` functions through
`subsecond::call`, so the next frame after a patch runs the new code. Its
state lives in the host, outside the patched functions, so it survives.

### What needs a restart

| Change | Why |
| --- | --- |
| The fields of a live struct, such as the gallery's `State` or `UiTree` | Existing values in memory keep the old layout. |
| Code that runs once at startup, such as `install_fonts` | It has already run. |

The gallery keeps demo state in a map keyed by control ID rather than in one
field per demo, precisely so that adding a demo never changes a struct.

A restart is not a fresh start. Under the `hot` feature the gallery saves its
state to a file in the system's temporary directory after every action and
reads it back when it launches, so a change that cannot be patched still
lands on the same page with the same values. Passing `--page` starts clean.

### Costs

- Each patch loads new code and leaves the old code in memory, a few megabytes
  each time. Restart after a long session.
- The first build under `dx` is a full build. Dependencies are compiled with
  optimizations even in development builds, so that first build is slower
  and every frame afterwards is faster. Workspace crates stay unoptimized to
  keep a patch under a second.
- `subsecond` and `dioxus-devtools` must be the same version as the `dx`
  binary. Both are pinned in the gallery's `Cargo.toml`.
- Dioxus is only this development tool. The `hot` feature is off by default
  and nothing else in the workspace depends on it.

## Adding a page

1. Create `crates/limn-gallery/src/pages/<name>.rs` with one function:

   ```rust
   use super::{row, section, Cx};
   use limn::{Button, Component, Element};

   pub fn view(cx: &Cx) -> Element {
       section(cx, "Buttons", vec![row(vec![
           Button::new("demo-save", "Save", cx.palette).render(),
       ])])
   }
   ```

2. Declare the module and add one line to `PAGES` in `pages/mod.rs`.

`Cx` gives a page the palette, the state and the width available to it.
`section`, `row` and `caption` are layout helpers shared by the pages.

### Demo state without update code

Pages do not write code to handle actions. The gallery interprets the ID of
whatever was clicked:

| ID | Effect |
| --- | --- |
| `name` | Flips a flag. Read it with `cx.state.flag("name")`. |
| `group=value` | Chooses within a group. Read it with `cx.state.choice("group", default)`. |
| `a=1&b=2` | Makes several choices at once. |

Text typed into an input is stored under the input's ID and read with
`cx.state.text("id")`.

So a switch is one line: `Switch::new("demo-switch",
cx.state.flag("demo-switch"), cx.palette)`.

## Capturing a page

```sh
cargo run -p limn-gallery -- --page overlays --set overlay-menu=open --screenshot menu.png
```

| Option | Effect |
| --- | --- |
| `--page <id>` | The page to open. IDs are in `PAGES`. |
| `--light` | Light theme. |
| `--set <id=value>` | Start as if that control had been clicked. Repeatable. Use it to capture an open menu or dialog. |
| `--hover <id>` | Hold the pointer over a control, to capture its hover state or tooltip. |
| `--end` | Scroll the page to its end. |
| `--screenshot <file.png>` | Save the window and exit. Runs with reduced motion so nothing is caught mid-transition. |

The screenshot is the size the window manager gives the window, so a tiling
window manager will produce different sizes at different times.

## See also

- [Testing](testing.md) for checks that do not need a window.
- [Contributing a component](contributing.md) for the full workflow.
