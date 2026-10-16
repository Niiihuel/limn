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

## Native file-drop demo

The receive-only Wayland example runs in its own binary:

```sh
cargo run -p limn-gallery --bin file_drop
```

Use Linux/Wayland and drop fixture files into the outlined target. After the
complete list reaches EOF, choose **Accept Copy** or **Reject** within 30 s.
The demo limits a batch to eight files and 32 KiB of decoded paths; the
library's default decision timeout remains 2 s. It shows basename metadata
without opening files. Accept authorizes one local Copy finish attempt; it
does not prepare or send file content.

This binary attaches only `FileDropReceiver`, independently of the normal
gallery and its hot-reload/screenshot mode. Its close handshake keeps the
host event loop running until the worker reports Closed. A non-Wayland
window reports unavailable without attaching the legacy path. See
[File drops](file-drop.md) before using the unsafe API. A real fixture test on
Hyprland 0.55.0 with Dolphin 26.04.3 reached Ready but received no drop metadata:
drag events went to the eframe host clipboard's first data device instead of
the receiver's separate device. Ready proves initialization only. This setup
needs shared data-device ownership and a host/SDK integration hook that keeps
clipboard text working; changing bind order or disabling clipboard is not a
fix.

A later native run on private nested Weston 15.0.1 received two fixture
basenames, accepted the first offer with an explicit **Accept Copy** click
(Ended Ok), and rejected the second with **Reject** (Ended Cancelled). The
**Close safely** control then led to normal demo exit 0 before cleanup of the
source and compositor. This proves only metadata reception, explicit local
decisions and normal closure in that setup. It does not fix the Hyprland
conflict, verify clipboard functionality, open file contents, or exercise an
application upload/send. Other combinations remain unverified.

The target and Close control stay outside the scroll area. Metadata and event
logs scroll independently so that their growth cannot shift the target and
invalidate its frozen geometry. An earlier nested run was cancelled by that
layout change and then timed out; it is retained as a partial attempt, not a
normal-close success. Three real UiTree demo tests cover hit testing and stable
target geometry at 700×760 and 980×800. Private-server tests alone do not
establish native interoperability.

## Hot reload

```sh
dx serve --hot-patch -p limn-gallery --features hot
```

Save a file in `limn` or in the gallery and the running window updates in
under a second. The page you were on, the scroll position, focus and every
value you typed are kept.

The gallery gets this from the library, like any application can: see
[Hot reload](hot-reload.md) for how it works, what needs a restart and what
it costs. A restart is not a fresh start: the gallery keeps its state with
`limn::hot::remember`, so a change that cannot be patched still lands on the
same page with the same values. Passing `--page` starts clean.

The gallery keeps demo state in a map keyed by control ID rather than in one
field per demo, precisely so that adding a demo never changes a struct.

## Adding a page

The gallery has a page for each component. A module draws sections, and
`PAGES` gives each component the sections that are its own, so components
that are shown together can share a module and still have their own pages.

1. Write the sections in `crates/limn-gallery/src/pages/<name>.rs`:

   ```rust
   use super::{row, section, Cx, Section};
   use limn::{Component, Switch};

   pub fn view(cx: &Cx) -> Vec<Section> {
       vec![section(cx, "Switch", vec![row(vec![
           Switch::new("demo-switch", cx.state.flag("demo-switch"), cx.palette).render(),
       ])])]
   }
   ```

2. Declare the module and list the component in `PAGES` in `pages/mod.rs`:

   ```rust
   Page { id: "switch", group: "Entradas", title: "Switch", source: "inputs", view: inputs::view, sections: &["Switch"] },
   ```

   `title` is the name of the type and `id` that name in kebab-case: the
   address of the component's page on the site and the name of its guide,
   `docs/components/<id>.md`. `sections` lists the titles of its sections;
   leave it empty when the whole module is one component.

A test fails when a section is on no page or a page names a section that does
not exist. `Cx` gives a module the palette, the state and the width available
to it. `section`, `row` and `caption` are layout helpers shared by the
modules; an element without a title becomes a section with `.into()`.

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

A demo whose state is neither a flag nor a choice, such as the sort and the
column widths of the table page, answers its actions in `pages::update`.

## Capturing a page

```sh
cargo run -p limn-gallery -- --page dropdown-menu --set overlay-menu=open --screenshot menu.png
```

| Option | Effect |
| --- | --- |
| `--page <id>` | The page to open. IDs are in `PAGES`. |
| `--pages` | Print the components as JSON, one page each, and exit. |
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
