use std::sync::Arc;

use super::{caption, section, Cx, Section};
use limn::{AlertColors, AlertTone, DrawCommand, Element, Point, Rect};

const BAR: f32 = 3.0;
const GAP: f32 = 2.0;
/// The share of the track already played.
const PLAYED: f32 = 0.4;

/// Made-up loudness from 0 to 1, one value for each bar that fits.
fn loudness(count: usize) -> impl Iterator<Item = f32> {
    (0..count).map(|index| {
        let t = index as f32;
        (0.4 + 0.3 * (t * 0.37).sin() * (t * 0.11).cos() + 0.25 * (t * 1.7).sin().abs()).clamp(0.08, 1.0)
    })
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let width = cx.width.min(480.0);
    let accent = AlertColors::for_tone(AlertTone::Info, palette).accent;
    let rest = palette.border;
    let waveform = Element::drawing(move |size| {
        let count = ((size.width + GAP) / (BAR + GAP)).floor().max(0.0) as usize;
        loudness(count)
            .enumerate()
            .map(|(index, level)| {
                let x = index as f32 * (BAR + GAP);
                let height = (size.height * level).max(BAR);
                DrawCommand::FillRect {
                    rect: Rect::new(x, (size.height - height) / 2.0, BAR, height),
                    color: if x < size.width * PLAYED { accent } else { rest },
                    radius: BAR / 2.0,
                }
            })
            .collect()
    })
    .size(Some(width), Some(64.0));
    let ink = palette.text;
    let wave = Element::drawing(move |size| {
        let middle = size.height / 2.0;
        let points: Arc<[Point]> = (0..=64)
            .map(|step| {
                let x = size.width * step as f32 / 64.0;
                Point::new(x, middle - middle * 0.8 * (x / size.width * std::f32::consts::TAU * 2.0).sin())
            })
            .collect();
        vec![
            DrawCommand::Polyline { points: Arc::new([Point::new(0.0, middle), Point::new(size.width, middle)]), color: rest, width: 1.0, dashed: true },
            DrawCommand::Polyline { points, color: ink, width: 1.5, dashed: false },
        ]
    })
    .size(None, Some(48.0));
    vec![section(cx, "Drawing", vec![
        waveform,
        caption(cx, "Element::drawing(|size| commands): a track's loudness as bars, drawn by the app in the frame's own commands, the played part in the accent."),
        wave,
        caption(cx, "With no width it takes the width it is stretched to, and draws again at it: two waves over a dashed axis, whatever the window's size."),
    ])]
}
