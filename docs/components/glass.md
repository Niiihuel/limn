# NavigationBar

A pill-shaped desktop bar of workspace dots, a two-line clock and status
actions. Its surface is opaque, or a `GlassMaterial` that samples the scene
behind it.

## Import

```rust
use limn::{Component, GlassMaterial, Icon, NavigationAction, NavigationBar, NavigationWorkspace};
```

## Usage

The host owns every value: which workspace is selected, the clock's text and
which actions are active. The bar reports `UiAction::Clicked` with the action
ID a workspace, the clock or an action was given.

```rust
fn view(state: &State, width: f32, palette: ComponentPalette) -> Element {
    NavigationBar::new("desktop", palette)
        .workspaces(
            (1..=4)
                .map(|number| {
                    NavigationWorkspace::new(format!("workspace:{number}"), format!("Workspace {number}"))
                        .selected(state.workspace == number)
                })
                .collect(),
        )
        .clock("11:06:46", "TUE 13 OCT")
        .actions(vec![NavigationAction::new("settings", Icon::Settings, "Settings")])
        .width(width)
        .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if let Some(number) = id.strip_prefix("workspace:") {
            state.workspace = number.parse().unwrap_or(state.workspace);
        } else if id == "settings" {
            state.open_settings();
        }
    }
}
```

The bar's own control and animation IDs derive from the ID given to `new`, so
several bars can report the same host actions. The bar is
`NavigationBar::HEIGHT` high.

## Workspaces

```rust
NavigationWorkspace::new("workspace:3", "Workspace 3")
    .selected(true)
    .occupied(true)
```

A workspace is a dot whose label is its tooltip. `selected` shows the capsule
on it; the first selected one wins. `occupied` gives a workspace that holds
windows stronger ink. `disabled(true)` leaves its dot and tooltip without a
pointer or keyboard action.

## Clock

```rust
NavigationBar::new("desktop", palette)
    .clock("11:06:46", "TUE 13 OCT")
    .clock_action("calendar")
```

`clock(time, date)` shows the text the host formats; the bar runs no timer.
`clock_action` makes the clock a control that reports that ID.

## Actions

```rust
NavigationAction::new("battery", Icon::Laptop, "Battery")
    .value("100%")
    .active(state.panel == "battery")
```

An action is an icon whose label is its tooltip. `value` adds short status
text after the icon. `active` gives it the accent color and the palette's
`selected` fill, for an open panel or an enabled mode. `disabled(true)`
renders a passive icon that keeps its tooltip.

## Accent

```rust
NavigationBar::new("desktop", palette).accent(accent)
```

`accent` is the ink of the selected workspace and of active actions; the
palette's text color unless set.

## Width

Pass the available width with `width` on resize. A bar narrower than its
content becomes compact: it omits the date and the actions' values, which stay
in their tooltips, and scrolls horizontally, so every control is kept. Without
`width` the bar takes its natural width.

## Glass

```rust
NavigationBar::new("desktop", palette)
    .glass(GlassMaterial::dark().blur(8.0).refraction(32.0).dispersion(0.5))
```

`glass` replaces the opaque palette surface and its border with a backdrop
material. `GlassMaterial::new(tint)` takes the veil blended over the sampled
scene; `dark()` and `light()` are two presets. `blur` (0 to 64), `refraction`
(0 to 32) and `dispersion` (0 to 4) are in logical pixels, clamped to those
ranges. A host must supply the scene to refract: see
[Liquid glass](../liquid-glass.md) for the material and the backdrop. Omit
`glass` for the opaque bar.

## Keyboard

| Key | Effect |
| --- | --- |
| Left, Right | On a focused workspace, selects the neighbouring enabled one and reports its click. |
| Enter | Activates the action in focus. |

Focus on a control outside a narrow bar's viewport scrolls it into view.

## Motion

The capsule moves between workspaces over `motion.move_ms`, and the fill of a
control answers the pointer over `motion.hover_ms`. With
`UiTree::set_reduced_motion(true)` the capsule snaps to its place.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` with a workspace's `action_id` | An enabled workspace is activated. |
| `UiAction::Clicked { id }` with the ID given to `clock_action` | The clock is activated. |
| `UiAction::Clicked { id }` with an action's `action_id` | An enabled action is activated. |
