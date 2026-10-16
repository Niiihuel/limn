# Select

A button that shows the current choice and opens the list of the others. Use
it to choose one of a few options where a
[SegmentedControl](segmented-control.md) would not fit.

## Import

```rust
use limn::{Component, ControlSize, DropdownOption, Icon, PopoverPlacement, SearchField, Select, SelectVariant, SelectorSizing};
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

## As a field

```rust
Select::new("shell", label, options, open, "shell:dismiss", palette)
    .variant(SelectVariant::Field)
    .width(360.0)
    .render()
```

`SelectVariant::Button`, the default, draws the trigger as an outlined
button, its label in the middle unless it has an icon. `SelectVariant::Field`
draws it as a [TextField](text-field.md): the field's fill, border, radius
and height, the choice where a field has its text, after the icon if there
is one, and the chevron at the end. Open, or focused from the keyboard, it
has the border of a focused field in place of a button's focus ring. Use it
in a form, among text fields, where a button would stand out.
A field is never narrower than 80, as a text field is.

## Size

```rust
Select::new("shell", label, options, open, "shell:dismiss", palette)
    .variant(SelectVariant::Field)
    .size(ControlSize::Compact)
    .height(40.0)
    .render()
```

`size` takes a `ControlSize`. Unless it is set, the button is `Compact`, 28
high, and the field `Regular`, 32 high, as a text field is. `height` is for
a form whose inputs are taller than a control size, as `TextField::height`
is: the trigger is never shorter than its control size, and a field never
shorter than 28. A height that is not finite is ignored.

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

## A searchable list

`header` adds an optional element above the options viewport, using the
[DropdownMenu](dropdown-menu.md) header slot. The header is only visible while
the list is open and shares its focus scope; it does not scroll with the options.
It leaves the trigger and select dimensions unchanged. Size header controls to
fit inside the menu's 4-pixel inset on each side.

```rust
let options = shells.iter()
    .filter(|shell| shell.label.to_lowercase().contains(&query.to_lowercase()))
    .map(|shell| DropdownOption::new(&shell.action_id, &shell.label))
    .collect();

Select::new("shell", current_label, options, open, "shell:dismiss", palette)
    .variant(SelectVariant::Field)
    .width(240.0)
    .menu_width(280.0)
    .header(
        SearchField::new("shell:query", query, "Search shells", palette)
            .size(ControlSize::Compact)
            .width(264.0)
            .render(),
    )
    .render()
    .popover_initial_focus("shell:query")
    .popover_return_focus("shell")
```

The host handles `UiAction::InputChanged` from the search field and rebuilds
the options with its filtered data. `Select` never stores or filters that data.
Escape from the header reports the same dismiss action as from an option;
when the host closes the list, `popover_return_focus` restores focus to the trigger.
Omitting `header` retains the existing select markup and behavior.

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

## Accessibility

The trigger is told to the platform as a combo box that says whether its
list is open, with the current label as its value; each option is a menu item
that says whether it is the selected one. The trigger shows the choice and
not what is being chosen, so the host names it, or puts it in a
[Form field](form-field.md), whose label names it. A disabled select is
still there and says it cannot be used.

```rust
Select::new("shell", "Bash", options, open, "shell:dismiss", palette).accessibility_label("Shell")
```
