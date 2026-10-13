# Select

A button that shows the current choice and opens the list of the others. Use
it to choose one of a few options where a
[SegmentedControl](segmented-control.md) would not fit.

## Import

```rust
use limn::{Component, DropdownOption, Icon, PopoverPlacement, Select, SelectorSizing};
```

## Usage

A select is controlled: the host owns the choice and whether the list is
open. It takes the trigger's ID, the label of the current choice, the
options, the open state, a dismiss ID and the palette. Every action is a
`UiAction::Clicked`: the trigger reports its ID, an option reports its own,
and Escape or a click outside reports the dismiss ID.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    let options = vec![
        DropdownOption::new("shell:bash", "Bash").selected(state.shell == "bash"),
        DropdownOption::new("shell:zsh", "Zsh").selected(state.shell == "zsh"),
    ];
    Select::new("shell", state.shell_label(), options, state.shell_open, "shell:dismiss", palette)
        .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        match id.as_str() {
            "shell" => state.shell_open = !state.shell_open,
            "shell:dismiss" => state.shell_open = false,
            "shell:bash" | "shell:zsh" => {
                state.shell = id["shell:".len()..].to_owned();
                state.shell_open = false;
            }
            _ => {}
        }
    }
}
```

## Options

```rust
DropdownOption::new("shell:fish", "Fish").icon(Icon::Terminal).selected(true)
DropdownOption::new("shell:nu", "Nushell").disabled()
```

`selected(true)` puts a check on the option. A disabled option is passive: it
cannot be chosen or focused.

## Width

```rust
Select::new("shell", label, options, open, "shell:dismiss", palette)
    .width(360.0)
    .menu_width(320.0)
    .render()
```

The trigger is 200 wide by default and the list is as wide as the trigger.
`width` sets the trigger and `menu_width` the list. A value that is not
finite is ignored.

## Sizing

```rust
Select::new("shell", label, options, open, "shell:dismiss", palette)
    .sizing(SelectorSizing::COMPACT)
    .render()
```

With a `SelectorSizing` the trigger takes the profile's `min_width` and the
list its `preferred_width`, unless `width` or `menu_width` say otherwise.
`COMPACT` and `CATALOG` are the two profiles.

## Placement

```rust
Select::new("shell", label, options, open, "shell:dismiss", palette)
    .placement(PopoverPlacement::BottomStart)
    .render()
```

The list opens under the trigger's far edge, `BottomEnd`, by default.

## Icon and list palette

`icon` puts an icon before the label. `menu_palette` paints the list with
another palette than the trigger's.

## Disabled

```rust
Select::new("shell", label, options, open, "shell:dismiss", palette)
    .enabled(false)
    .render()
```

A disabled select shows its value, stays closed even when `open` is true, and
cannot be focused.

## Keyboard

| Key | Effect |
| --- | --- |
| Down | With the list open, moves the focus to the next enabled option |
| Enter | Chooses the focused option |
| Escape | Reports the dismiss ID |

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` with the trigger's ID | The trigger is activated. The host opens or closes the list. |
| `UiAction::Clicked { id }` with an option's ID | An enabled option is chosen. |
| `UiAction::Clicked { id }` with the dismiss ID | Escape is pressed or the pointer clicks outside the list. |
