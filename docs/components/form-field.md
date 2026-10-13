# FormField

A label over a control, with the hints and the error that belong to it. The
host builds the control and owns its value; the field owns the order and the
spacing of the text around it.

## Import

```rust
use limn::{Component, FormField, FormFieldSizing, TextField, TextRole};
```

## Usage

A form field takes a label, the rendered control and the palette. The control
fills the field's width. The field reports nothing of its own: handle the
control's actions as you would without it.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    FormField::new(
        "Name",
        TextField::new("name", &state.name, "My project", palette).render(),
        palette,
    )
    .render()
}
```

## Hints

```rust
FormField::new("Program", control, palette)
    .hint("Absolute path to an existing file.")
    .hint("It runs without a shell.")
    .hint_above(true)
    .render()
```

Each `hint` adds a muted line under the control. `hint_above(true)` puts the
hints between the label and the control instead.

## Error

```rust
FormField::new("Path", control, palette)
    .hint("Missing folders are created.")
    .error("A file with that path already exists.", danger)
    .render()
```

`error` adds a last line, under everything else, in the color you pass. The
field does not decide what the color means.

## Sizing

```rust
FormField::new("Instructions", control, palette)
    .sizing(FormFieldSizing {
        label_muted: true,
        label_height: 18.0,
        hint_role: TextRole::Caption,
        gap: 8.0,
        ..FormFieldSizing::DEFAULT
    })
    .render()
```

`FormFieldSizing` sets the label's type role (`label_role`), whether the
label is muted, the least height of its line, the role of the hints and the
error, and the gap between the lines. `FormFieldSizing::DEFAULT` is a caption
label, small hints and a gap of 7.

## Rows

```rust
let mut rows = FormField::new("Program", program, palette).rows();
rows.extend(FormField::new("Working directory", cwd, palette).rows());
Element::column(rows).gap(7.0)
```

`rows()` returns the field's lines in order instead of the rendered column,
for a form that sets every line of every field in one column of its own. The
form then owns the gap.
