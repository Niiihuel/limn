use super::{caption, section, Cx, Section};
use crate::app::State;
use limn::{Component, Element, Pane, RadiusRole, SplitView, UiAction};

const ROW: &str = "split-demo";
const COLUMN: &str = "split-demo-side";
/// The demo's size, fixed so the update knows the room its panes share.
const WIDTH: f32 = 480.0;
const HEIGHT: f32 = 240.0;
const HANDLE: f32 = 8.0;
const MIN: f32 = 80.0;

/// The fractions of a split the host keeps, `default` until a handle moves.
fn fractions(state: &State, id: &str, default: [f32; 2]) -> Vec<f32> {
    let parsed: Vec<f32> = state.text(id).split(' ').filter_map(|part| part.parse().ok()).collect();
    if parsed.len() == 2 { parsed } else { default.to_vec() }
}

fn total(id: &str) -> f32 {
    if id == ROW { WIDTH - HANDLE } else { HEIGHT - HANDLE }
}

fn default(id: &str) -> [f32; 2] {
    if id == ROW { [0.4, 0.6] } else { [0.5, 0.5] }
}

pub fn update(state: &mut State, action: &UiAction) {
    let split = |id: &str| [ROW, COLUMN].into_iter().find(|split| id == format!("{split}:handle:0"));
    match action {
        UiAction::DragStarted { id, position } => {
            if let Some(split) = split(id) {
                let at = if split == ROW { position.x } else { position.y };
                let start = format!("{at} {}", fractions(state, split, default(split))[0]);
                state.set("split-demo-drag", &start);
            }
        }
        UiAction::DragMoved { id, position, .. } => {
            let start = state.text("split-demo-drag").split_once(' ').and_then(|(at, first)| {
                Some((at.parse::<f32>().ok()?, first.parse::<f32>().ok()?))
            });
            if let (Some(split), Some((at, first))) = (split(id), start) {
                let moved = if split == ROW { position.x } else { position.y } - at;
                let resized = SplitView::resized(&[first, 1.0 - first], 0, moved, total(split), &[MIN, MIN]);
                state.set(split, &format!("{} {}", resized[0], resized[1]));
            }
        }
        _ => {}
    }
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let pane = |label: &str| {
        Element::panel(caption(cx, label)).padding(12.0).background(palette.surface).radius(RadiusRole::Surface.px())
    };
    let side = fractions(cx.state, COLUMN, default(COLUMN));
    let diff_and_terminal = SplitView::column(
        COLUMN,
        vec![Pane::new(pane("Diff")).fraction(side[0]), Pane::new(pane("Terminal")).flex()],
        palette,
    )
    .render();
    let chat = fractions(cx.state, ROW, default(ROW));
    let split = SplitView::row(
        ROW,
        vec![Pane::new(pane("Chat")).fraction(chat[0]), Pane::new(diff_and_terminal).flex()],
        palette,
    )
    .render()
    .size(Some(WIDTH), Some(HEIGHT));
    vec![section(cx, "Panes divided by resize handles", vec![
        split,
        caption(cx, "Chat beside a diff over a terminal, 2:3 and half and half to start. Drag a boundary: no pane goes under 80 px."),
    ])]
}
