# TimePicker

A field for a local time of day as 24-hour `HH:MM`, with arrows that step the
hour and the minute.

## Import

```rust
use limn::{Component, TimePicker};
```

## Usage

A time picker is controlled: pass the time as text each frame, and store what
the field and the arrows report. The field reports every edit under the
picker's own ID. An arrow reports a click, and `TimePicker::adjusted_value`
turns that click into the new time.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    TimePicker::new("alarm", &state.alarm, palette).render()
}

fn update(state: &mut State, action: &UiAction) {
    match action {
        UiAction::InputChanged { id, value } if id == "alarm" => state.alarm = value.clone(),
        UiAction::Clicked { id } => {
            if let Some(time) = TimePicker::adjusted_value(id, "alarm", &state.alarm) {
                state.alarm = time;
            }
        }
        _ => {}
    }
}
```

`adjusted_value(action_id, id, current)` is `None` for an action that is not
one of this picker's arrows, so it can be tried on every click.

## Stepping

An hour arrow moves the time by an hour and wraps at midnight: an hour up
from `23:15` is `00:15`. A minute arrow moves it by a minute and carries into
the hour: a minute up from `23:59` is `00:00`.

## Text that is not a time

```rust
TimePicker::is_valid_time("09:30") // true
TimePicker::is_valid_time("9:30")  // false
```

The text is the host's, valid or not. While it is not two digits, a colon and
two digits, from `00:00` to `23:59`, the hour and the minute show `--` and
`adjusted_value` is `None`, so an arrow leaves a time half typed as it is.

## Disabled

```rust
TimePicker::new("alarm", &state.alarm, palette).disabled().render()
TimePicker::new("alarm", &state.alarm, palette).enabled(state.can_edit).render()
```

A disabled picker shows its time in muted ink. The field and the arrows take
no pointer or focus and report nothing.

## Keyboard

The field and the four arrows are tab stops. Enter or Space on an arrow
reports its click.

## Actions

| Action | When |
| --- | --- |
| `UiAction::InputChanged { id, value }` | The text of the field changes; `id` is the picker's. |
| `UiAction::Clicked { id }` with `{id}:hour_up`, `{id}:hour_down`, `{id}:minute_up` or `{id}:minute_down` | An arrow is pressed. |

## Accessibility

Each arrow is a button named by its unit and the value it goes to, such as
"Hour 10", or by its unit alone when the text is not a valid time. In a
disabled picker they are still there, as disabled. The words for the units
come from `limn::labels()`.
