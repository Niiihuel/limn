# DateTimePicker

A [DatePicker](date-picker.md) over a [TimePicker](time-picker.md), each
under its label, for a local date and time. The two keep separate values and
IDs, so one half can be half typed while the other changes.

## Import

```rust
use limn::{Component, DatePicker, DateTimePicker, TimePicker};
```

## Usage

Pass an ID and a value for each half, the date's first. Each half reports as
it does on its own, so the host handles the date's ID as a date picker's and
the time's as a time picker's.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    let (year, month) = state.shown_month;
    DateTimePicker::new("task_date", "task_time", &state.date, &state.time, palette)
        .display_month(year, month)
        .render()
}

fn update(state: &mut State, action: &UiAction) {
    match action {
        UiAction::InputChanged { id, value } if id == "task_date" => state.date = value.clone(),
        UiAction::InputChanged { id, value } if id == "task_time" => state.time = value.clone(),
        UiAction::Clicked { id } => {
            let (year, month) = state.shown_month;
            if let Some(date) = DatePicker::selected_date(id, "task_date") {
                state.date = date.to_owned();
            } else if let Some(time) = TimePicker::adjusted_value(id, "task_time", &state.time) {
                state.time = time;
            } else if id == "task_date:month_prev" {
                state.shown_month = DatePicker::shift_month(year, month, -1);
            } else if id == "task_date:month_next" {
                state.shown_month = DatePicker::shift_month(year, month, 1);
            }
        }
        _ => {}
    }
}
```

The picker does not join the two values. The host does, once
`DatePicker::is_valid_date` and `TimePicker::is_valid_time` accept them.

## The month shown

`display_month(year, month)` is the date picker's: the month the calendar
shows, apart from the date chosen. Without it the calendar shows the month of
the date.

## Disabled

```rust
DateTimePicker::new("task_date", "task_time", &state.date, &state.time, palette)
    .disabled()
    .render()
```

`disabled()`, or `enabled(false)`, disables both halves: they show their
values in muted ink and nothing in them takes the pointer or focus.

## Actions

| Action | When |
| --- | --- |
| `UiAction::InputChanged { id, value }` | The text of either field changes; `id` is that half's. |
| `UiAction::Clicked { id }` with `{date_id}:day:YYYY-MM-DD`, `{date_id}:month_prev` or `{date_id}:month_next` | A day is chosen or an arrow of the calendar is pressed. |
| `UiAction::Clicked { id }` with `{time_id}:hour_up`, `{time_id}:hour_down`, `{time_id}:minute_up` or `{time_id}:minute_down` | An arrow of the time is pressed. |

## Accessibility

The picker declares nothing of its own: the arrows and the days are named as
in a [DatePicker](date-picker.md#accessibility) and a
[TimePicker](time-picker.md#accessibility). The two labels over the halves
come from `limn::labels()`.
