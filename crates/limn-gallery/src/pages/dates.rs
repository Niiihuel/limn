use super::{caption, row, section, Cx, Section};
use crate::app::State;
use limn::{Component, DatePicker, DateTimePicker, TimePicker, UiAction};

/// The live date fields and the date each shows before any choice.
const DATES: [(&str, &str); 2] = [("dates-date", "2025-05-14"), ("dates-both-date", "2025-05-14")];
/// The live time fields and the time each shows before any change.
const TIMES: [(&str, &str); 3] = [("dates-time", "09:30"), ("dates-time-partial", "9:5"), ("dates-both-time", "09:30")];

/// The year and month of a valid ISO date.
fn month_of(date: &str) -> Option<(i32, u32)> {
    if !DatePicker::is_valid_date(date) {
        return None;
    }
    Some((date[..4].parse().ok()?, date[5..7].parse().ok()?))
}

/// The month a calendar shows: the one navigated to, kept as the first day
/// of it, or else the month of its date.
fn shown(state: &State, id: &str, initial: &str) -> (i32, u32) {
    month_of(state.text(&format!("{id}-month")))
        .or_else(|| month_of(state.choice(id, initial)))
        .or_else(|| month_of(initial))
        .unwrap_or((2025, 1))
}

/// The pickers report IDs of their own, which no flag or choice can hold:
/// a month to show, a day chosen, a time stepped.
pub fn update(state: &mut State, action: &UiAction) {
    match action {
        UiAction::Clicked { id } => {
            for (date, initial) in DATES {
                let month = format!("{date}-month");
                let delta = match id.strip_prefix(date) {
                    Some(":month_prev") => -1,
                    Some(":month_next") => 1,
                    _ => 0,
                };
                if delta != 0 {
                    let (year, shown_month) = shown(state, date, initial);
                    let (year, shown_month) = DatePicker::shift_month(year, shown_month, delta);
                    state.set(&month, &format!("{year:04}-{shown_month:02}-01"));
                } else if let Some(chosen) = DatePicker::selected_date(id, date) {
                    state.set(date, chosen);
                    state.set(&month, chosen);
                }
            }
            for (time, initial) in TIMES {
                let current = state.choice(time, initial).to_owned();
                if let Some(stepped) = TimePicker::adjusted_value(id, time, &current) {
                    state.set(time, &stepped);
                }
            }
        }
        // A date typed in full brings its month into view.
        UiAction::InputChanged { id, value }
            if DATES.iter().any(|(date, _)| date == id) && month_of(value).is_some() =>
        {
            state.set(&format!("{id}-month"), value);
        }
        _ => {}
    }
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let state = cx.state;
    let valid = |ok: bool| if ok { "valid" } else { "not valid yet" };

    let (date_id, date_initial) = DATES[0];
    let date = state.choice(date_id, date_initial);
    let (year, month) = shown(state, date_id, date_initial);

    let (time_id, time_initial) = TIMES[0];
    let time = state.choice(time_id, time_initial);
    let (partial_id, partial_initial) = TIMES[1];
    let partial = state.choice(partial_id, partial_initial);

    let (both_date_id, both_date_initial) = DATES[1];
    let (both_time_id, both_time_initial) = TIMES[2];
    let both_date = state.choice(both_date_id, both_date_initial);
    let both_time = state.choice(both_time_id, both_time_initial);
    let (both_year, both_month) = shown(state, both_date_id, both_date_initial);
    let both_ok = DatePicker::is_valid_date(both_date) && TimePicker::is_valid_time(both_time);

    vec![
        section(cx, "Date picker", vec![
            row(vec![DatePicker::new(date_id, date, palette).display_month(year, month).render()]),
            caption(cx, &format!("Date: {date:?}, {}. The arrows change the month shown, not the date.", valid(DatePicker::is_valid_date(date)))),
            row(vec![DatePicker::new("dates-date-off", "2025-05-14", palette).disabled().render()]),
            caption(cx, "Disabled: the date and its month in muted ink, without pointer input or focus."),
        ]),
        section(cx, "Time picker", vec![
            row(vec![TimePicker::new(time_id, time, palette).render()]),
            caption(cx, &format!("Time: {time:?}, {}. The minutes carry into the hour, and the hours wrap at midnight.", valid(TimePicker::is_valid_time(time)))),
            row(vec![TimePicker::new(partial_id, partial, palette).render()]),
            caption(cx, &format!("Time: {partial:?}, {}. The arrows leave text that is not HH:MM as it is.", valid(TimePicker::is_valid_time(partial)))),
            row(vec![TimePicker::new("dates-time-off", "09:30", palette).disabled().render()]),
            caption(cx, "Disabled: the time in muted ink, without pointer input or focus."),
        ]),
        section(cx, "Date and time picker", vec![
            row(vec![DateTimePicker::new(both_date_id, both_time_id, both_date, both_time, palette)
                .display_month(both_year, both_month)
                .render()]),
            caption(cx, &if both_ok {
                format!("The host joins the two values: {both_date}T{both_time}")
            } else {
                "One half is not valid yet; the other keeps its value.".to_owned()
            }),
            row(vec![DateTimePicker::new("dates-both-date-off", "dates-both-time-off", "2025-05-14", "09:30", palette)
                .disabled()
                .render()]),
            caption(cx, "Disabled: both halves are passive."),
        ]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pickers_keep_their_month_their_day_and_their_time() {
        let mut state = State::default();
        let click = |id: &str| UiAction::Clicked { id: id.into() };
        update(&mut state, &click("dates-date:month_prev"));
        assert_eq!(shown(&state, "dates-date", "2025-05-14"), (2025, 4));
        // Navigating chooses nothing.
        assert_eq!(state.choice("dates-date", "2025-05-14"), "2025-05-14");
        update(&mut state, &click("dates-date:day:2025-03-31"));
        assert_eq!(state.text("dates-date"), "2025-03-31");
        assert_eq!(shown(&state, "dates-date", "2025-05-14"), (2025, 3));
        assert_eq!(shown(&state, "dates-both-date", "2025-05-14"), (2025, 5));
        update(&mut state, &UiAction::InputChanged { id: "dates-date".into(), value: "2024-02-29".into() });
        assert_eq!(shown(&state, "dates-date", "2025-05-14"), (2024, 2));

        update(&mut state, &click("dates-time:minute_down"));
        assert_eq!(state.text("dates-time"), "09:29");
        update(&mut state, &click("dates-time-partial:hour_up"));
        assert_eq!(state.choice("dates-time-partial", "9:5"), "9:5");
    }
}
