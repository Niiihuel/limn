# Slider

A value dragged along a track, from 0 to 1: a seek position or a volume.

## Import

```rust
use limn::{Component, Slider};
```

## Usage

A slider is controlled: pass its fraction each frame and store the one it
reports. Values outside 0 to 1 are clamped.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    Slider::new("volume", state.volume, palette).width(280.0).render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::SliderChanged { id, fraction, committed } = action {
        if id == "volume" {
            state.volume = *fraction;
            if *committed {
                state.save_volume();
            }
        }
    }
}
```

`committed` is false during a drag and true on release, so a host can follow
the thumb and write the value once.

## Steps

```rust
Slider::new("quality", state.quality, palette).steps(5).render()
```

With `steps(n)` the slider rests on one of `n` evenly spaced stops, the ends
included. Below two it is continuous, which is the default.

## Marks and labels

```rust
Slider::new("font-size", state.font_size, palette)
    .steps(7)
    .mark_labels(["12px", "14px", "15px", "16px", "18px", "20px", "24px"])
    .default_stop(3)
    .render()
```

`marks()` draws a hairline across the track at each stop, behind the thumb,
in the palette's border color. `mark_labels` names the stops from the first,
in 12 pixel muted text over the track, and draws the marks too. Each label is
one line. The labels at the ends line up with the ends of the slider; the
others center on their stop. Labels past the last stop are ignored, and an
empty one leaves its stop unnamed.

`default_stop(i)` shows the label of stop `i`, counted from 0, in the
slider's ink: the value the setting has until it is changed.

Without `steps` of 2 or more there are no stops, so neither marks nor labels.
Past 100 stops there are no marks: that close together they would only gray
the track. The labels still show.

## Thumb, bar and mark style

```rust
Slider::new("opacity", state.opacity, palette)
    .thumb(16.0, 20.0)
    .bar(4.0, 6.0)
    .steps(5)
    .marks()
    .mark_style(MarkStyle {
        dash: Size::new(2.0, 24.0),
        label_size: 12.0,
        label_weight_medium: true,
        default_mark_ink: Some(theme.success),
    })
    .default_stop(2)
    .render()
```

`thumb(idle, active)` sets the thumb's diameter at rest and while the pointer
is over the slider. `bar(idle, active)` sets the track's thickness the same
way. Both default to 12 and 4, so the two states are alike. A value that is
not finite is ignored and a negative one is zero. The slider is as tall as
its largest thumb.

`mark_style(MarkStyle { .. })` sets how the marks and the stops' names look:
`dash` is a mark's width and height, 1 by 12 unless set; `label_size` is the
text size, 12; `label_weight_medium` draws the labels in the sans family's
medium weight; `default_mark_ink` colors the mark at the stop that
`default_stop` names, in place of the border color. `MarkStyle::default()` is
the look a slider has without it. A mark taller than the thumb makes the
slider taller, so the marks are not cut short.

## Ink

```rust
Slider::new("seek", state.position, palette).ink(accent).render()
```

`ink` colors the filled part, the thumb and the default stop's label, in place
of the palette's text.

## A level on the track

```rust
let over = state.level >= state.threshold;
Slider::new("sensitivity", state.threshold, palette)
    .ink(accent)
    .level(state.level, if over { green } else { yellow })
    .accessibility_label("Input sensitivity")
    .render()
```

`level(level, color)` draws a value that moves on its own along the track,
from the start to `level`, in `color`, in place of the fill: a meter whose
threshold is the slider's value, such as a microphone's input under the
sensitivity the thumb sets. The host passes the new level on every rebuild,
and asks for a repaint while it moves. The platform is told only the value.
Disabled, the level is muted.

## Disabled

```rust
Slider::new("volume", state.volume, palette).enabled(false).render()
```

A disabled slider shows its value and takes no pointer or focus. It has no
thumb, and its fill and every label are in the muted color.

## Keyboard

| Key | Moves the value |
| --- | --- |
| Left, Right | By a twentieth, or by one stop with `steps` |
| Home, End | To either end |

## Actions

| Action | When |
| --- | --- |
| `UiAction::SliderChanged { id, fraction, committed }` | The value changes: during a drag with `committed` false, on release or from the keyboard with it true. |

## Accessibility

The platform is told a slider and where it stands from 0 to 1, disabled or
not. A slider shows no text, so the host names it, or puts it in a
[Form field](form-field.md), whose label names it.

```rust
Slider::new("volume", volume, palette).accessibility_label("Volume")
```

A slider with labels says the label of the stop its value is on, or the
nearest one during a drag, as its value in words: "16px".

## Limits

The thumb and the bar take their active size only while the pointer is over
the slider. A keyboard focus does not enlarge them, and a drag that leaves the
slider brings them back to rest: the element tree reveals a part of a control
on its pointer region alone.

Labels do not move apart or wrap when they are too wide for their stops, and
the track stays where it is. A label centers on its stop only while it is no
wider than twice the way from its stop to the nearer end stop; a wider one
starts that way left of its stop and runs on to the right, over its
neighbours, and past the slider's end if it must. Give the slider the width
its labels need.
