# DatePicker

A field for a local calendar date as `YYYY-MM-DD` over a month calendar to
choose it from. Use it inline, in a form or a dialog; it does not open or
close.

## Import

```rust
use limn::{Component, DatePicker};
```

## Usage

A date picker is controlled: pass the date as text each frame, and store what
the field and the calendar report. The field reports every edit under the
picker's own ID, and a day under `{id}:day:YYYY-MM-DD`, which
`DatePicker::selected_date` decodes.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    let (year, month) = state.shown_month;
    DatePicker::new("due", &state.due, palette).display_month(year, month).render()
}

fn update(state: &mut State, action: &UiAction) {
    match action {
        UiAction::InputChanged { id, value } if id == "due" => state.due = value.clone(),
        UiAction::Clicked { id } => {
            let (year, month) = state.shown_month;
            if let Some(date) = DatePicker::selected_date(id, "due") {
                state.due = date.to_owned();
            } else if id == "due:month_prev" {
                state.shown_month = DatePicker::shift_month(year, month, -1);
            } else if id == "due:month_next" {
                state.shown_month = DatePicker::shift_month(year, month, 1);
            }
        }
        _ => {}
    }
}
```

The text is the host's, valid or not: a date half typed stays as typed.
`DatePicker::is_valid_date(value)` says whether it is a real date, from year
1 to 9999, written with every digit.

## The month shown

```rust
DatePicker::new("due", &state.due, palette).display_month(2025, 5).render()
```

The month shown is separate from the date, so the arrows browse without
choosing anything. Without `display_month` the calendar shows the month of
the date, or the current month in UTC when the text is not a valid date. A
year outside 1 to 9999 or a month outside 1 to 12 is ignored.

`DatePicker::shift_month(year, month, delta)` gives the month `delta` months
away and stops at January of year 1 and December of 9999, where the arrow
towards the end is drawn disabled.

The weeks start on Monday. The days of the months before and after that fill
the first and last weeks are muted, and can be chosen like the others.

## Disabled

```rust
DatePicker::new("due", &state.due, palette).disabled().render()
DatePicker::new("due", &state.due, palette).enabled(state.can_edit).render()
```

A disabled picker shows its date and its month in muted ink. The field, the
arrows and the days take no pointer or focus and report nothing.

## Keyboard

The field, the two arrows and every day are tab stops. Enter or Space on an
arrow or a day reports its click.

## Actions

| Action | When |
| --- | --- |
| `UiAction::InputChanged { id, value }` | The text of the field changes; `id` is the picker's. |
| `UiAction::Clicked { id }` with `{id}:day:YYYY-MM-DD` | A day is chosen. |
| `UiAction::Clicked { id }` with `{id}:month_prev` or `{id}:month_next` | An arrow of the calendar is pressed. |

## Accessibility

The field is called "Date", the library's own word, unless the picker is
given a name of its own:

```rust
DatePicker::new("start", &state.start, palette).accessibility_label("Start date").render()
```

Each arrow is a button named by the month it goes to, such as "April 2025",
and each day a button named by its whole date, such as "14 May 2025", that
says whether it is the chosen one. In a disabled picker they are still there,
as disabled. The names of the months and the weekdays come from
`limn::labels()`.
