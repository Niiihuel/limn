//! Shared VS Code-style shortcut notation, keycaps and geometry.

use crate::{Color, Icon};

pub(crate) const FONT_SIZE: f32 = 11.0;
pub(crate) const MENU_FONT_SIZE: f32 = 12.0;
pub(crate) const HEIGHT: f32 = 20.0;
pub(crate) const HORIZONTAL_PADDING: f32 = 5.0;
pub(crate) fn radius() -> f32 {
    crate::tokens().radius.control
}
pub(crate) const ICON_SIZE: f32 = 12.0;
pub(crate) const KEY_SPACING: f32 = 2.0;
pub(crate) const CHORD_GAP: f32 = 6.0;
pub(crate) const MIN_WIDTH: f32 = 22.0;

pub(crate) fn label(value: &str) -> &str {
    value.trim()
}

/// Menus use a single accelerator label; instructional hints use outlined
/// keycaps. Both share the same platform notation and chord boundaries.
pub(crate) fn menu_label(value: &str) -> String {
    menu_label_for_platform(value, cfg!(target_os = "macos"))
}

fn menu_label_for_platform(value: &str, macos: bool) -> String {
    if label(value) == "↑↓" {
        return "↑↓".to_owned();
    }
    chords(value)
        .into_iter()
        .map(|chord| {
            chord
                .into_iter()
                .map(|key| key_label(key, macos))
                .collect::<Vec<_>>()
                .join(if macos { "" } else { "+" })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn key_label(key: &str, macos: bool) -> &str {
    if macos {
        match key {
            "Ctrl" | "Control" => "⌃",
            "Cmd" | "Command" | "Meta" | "Super" => "⌘",
            "Shift" => "⇧",
            "Alt" | "Option" => "⌥",
            "Enter" | "Return" => "↵",
            "Esc" | "Escape" => "⎋",
            "Backspace" => "⌫",
            "Delete" => "⌦",
            "Left" => "←",
            "Right" => "→",
            "Up" => "↑",
            "Down" => "↓",
            key => key,
        }
    } else {
        match key {
            "Control" => "Ctrl",
            "Escape" => "Esc",
            "Return" => "Enter",
            "Meta" | "Super" if cfg!(target_os = "windows") => "Windows",
            "Meta" => "Super",
            key => key,
        }
    }
}

/// Spaces separate sequential chords, while whitespace around `+` belongs
/// to the current chord. A literal plus is written `Ctrl++`; a minus is `Ctrl+-`.
fn chords(value: &str) -> Vec<Vec<&str>> {
    let value = label(value);
    if value.is_empty() {
        return Vec::new();
    }
    let mut chords = Vec::new();
    let mut start = 0;
    for (index, character) in value.char_indices() {
        if character.is_whitespace() && index >= start {
            let before = value[start..index].trim_end();
            let after = value[index..].trim_start();
            let awaiting_key = before
                .strip_suffix('+')
                .is_some_and(|prefix| !prefix.is_empty() && !prefix.trim_end().ends_with('+'));
            if !before.is_empty() && !after.is_empty() && !awaiting_key && !after.starts_with('+') {
                chords.push(chord_keys(before));
                start = value.len() - after.len();
            }
        }
    }
    if start < value.len() {
        chords.push(chord_keys(&value[start..]));
    }
    chords
}

fn chord_keys(value: &str) -> Vec<&str> {
    let mut keys = Vec::new();
    let mut remaining = value;
    loop {
        if remaining == "+" {
            keys.push(remaining);
            break;
        }
        if let Some((key, tail)) = remaining.split_once('+') {
            let key = key.trim();
            let tail = tail.trim();
            if key.is_empty() || tail.is_empty() {
                return vec![value];
            }
            keys.push(key);
            remaining = tail;
        } else {
            keys.push(remaining);
            break;
        }
    }
    keys
}

pub(crate) enum Part<'a> {
    Key(&'a str),
    Separator,
    KeyGap,
    ChordGap,
}

pub(crate) fn parts(value: &str) -> Vec<Part<'_>> {
    parts_for_platform(value, cfg!(target_os = "macos"))
}

fn parts_for_platform(value: &str, macos: bool) -> Vec<Part<'_>> {
    if label(value) == "↑↓" {
        return vec![Part::Key("↑"), Part::ChordGap, Part::Key("↓")];
    }
    let mut parts = Vec::new();
    for (chord_index, chord) in chords(value).into_iter().enumerate() {
        if chord_index > 0 {
            parts.push(Part::ChordGap);
        }
        for (key_index, key) in chord.into_iter().enumerate() {
            if key_index > 0 {
                parts.push(if macos { Part::KeyGap } else { Part::Separator });
            }
            parts.push(Part::Key(key_label(key, macos)));
        }
    }
    parts
}

pub(crate) fn cap_width(text_width: f32) -> f32 {
    (text_width.max(0.0) + HORIZONTAL_PADDING * 2.0).max(MIN_WIDTH)
}

pub(crate) fn key_icon(key: &str) -> Option<Icon> {
    match key {
        "↑" => Some(Icon::ArrowUp),
        "↓" => Some(Icon::ArrowDown),
        "←" => Some(Icon::ArrowLeft),
        "→" => Some(Icon::ArrowRight),
        _ => None,
    }
}

pub(crate) fn width(value: &str, measure: impl Fn(&str) -> f32) -> f32 {
    parts(value)
        .into_iter()
        .map(|part| match part {
            Part::Key(key) => cap_width(if key_icon(key).is_some() {
                ICON_SIZE
            } else {
                measure(key)
            }),
            Part::Separator => measure("+") + KEY_SPACING * 2.0,
            Part::KeyGap => KEY_SPACING * 2.0,
            Part::ChordGap => CHORD_GAP,
        })
        .sum()
}

/// A subtle neutral surface and outline retain the parent theme's contrast.
/// The lower edge is painted separately, without stacking two translucent outlines.
pub(crate) fn cap_fill(ink: Color) -> Color {
    Color::rgba(128, 128, 128, ((u16::from(ink.a) * 10 + 50) / 100) as u8)
}

pub(crate) fn cap_border(ink: Color) -> Color {
    Color::rgba(
        ink.r,
        ink.g,
        ink.b,
        ((u16::from(ink.a) * 16 + 50) / 100) as u8,
    )
}

pub(crate) fn cap_bottom_border(ink: Color) -> Color {
    Color::rgba(
        ink.r,
        ink.g,
        ink.b,
        ((u16::from(ink.a) * 22 + 50) / 100) as u8,
    )
}

pub(crate) fn separator_ink(ink: Color) -> Color {
    Color::rgba(
        ink.r,
        ink.g,
        ink.b,
        ((u16::from(ink.a) * 60 + 50) / 100) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_menu_accelerators_preserve_combinations_and_chords() {
        assert_eq!(
            menu_label_for_platform("Ctrl+Shift+Z", false),
            "Ctrl+Shift+Z"
        );
        assert_eq!(menu_label_for_platform("Ctrl++", false), "Ctrl++");
        assert_eq!(menu_label_for_platform("Ctrl+-", false), "Ctrl+-");
        assert_eq!(
            menu_label_for_platform("Ctrl+K Ctrl+C", false),
            "Ctrl+K Ctrl+C"
        );
        assert_eq!(
            menu_label_for_platform("Ctrl + K   Ctrl + C", false),
            "Ctrl+K Ctrl+C"
        );
        assert_eq!(menu_label_for_platform("Shift+F12", false), "Shift+F12");
        assert_eq!(menu_label_for_platform("  ", false), "");
        assert_eq!(menu_label_for_platform("↑↓", false), "↑↓");
    }

    #[test]
    fn macos_control_and_command_remain_distinct() {
        assert_eq!(menu_label_for_platform("Ctrl+Shift+Z", true), "⌃⇧Z");
        assert_eq!(menu_label_for_platform("Cmd+Shift+Z", true), "⌘⇧Z");
        assert_eq!(menu_label_for_platform("Alt+Enter", true), "⌥↵");
        assert_eq!(menu_label_for_platform("Cmd+K Cmd+C", true), "⌘K ⌘C");
        assert_eq!(menu_label_for_platform("Ctrl++", true), "⌃+");
    }

    #[test]
    fn keycaps_distinguish_sequential_chords_and_literal_plus() {
        let parts = parts_for_platform("Ctrl++ Ctrl+-", false);
        assert!(matches!(
            parts.as_slice(),
            [
                Part::Key("Ctrl"),
                Part::Separator,
                Part::Key("+"),
                Part::ChordGap,
                Part::Key("Ctrl"),
                Part::Separator,
                Part::Key("-")
            ]
        ));
        assert!(matches!(
            parts_for_platform("Cmd+K Cmd+C", true).as_slice(),
            [
                Part::Key("⌘"),
                Part::KeyGap,
                Part::Key("K"),
                Part::ChordGap,
                Part::Key("⌘"),
                Part::KeyGap,
                Part::Key("C")
            ]
        ));
    }

    #[test]
    fn literal_plus_completes_a_chord_even_with_spaces_around_its_joiner() {
        for input in ["Ctrl++ Ctrl+-", "Ctrl + +   Ctrl + -", "Ctrl+ +\tCtrl+ -"] {
            assert_eq!(menu_label_for_platform(input, false), "Ctrl++ Ctrl+-");
            assert_eq!(menu_label_for_platform(input, true), "⌃+ ⌃-");
            assert!(matches!(
                parts_for_platform(input, false).as_slice(),
                [
                    Part::Key("Ctrl"),
                    Part::Separator,
                    Part::Key("+"),
                    Part::ChordGap,
                    Part::Key("Ctrl"),
                    Part::Separator,
                    Part::Key("-")
                ]
            ));
        }
        assert_eq!(menu_label_for_platform("+ Ctrl + K", false), "+ Ctrl+K");
        assert_eq!(
            menu_label_for_platform("Ctrl + K Ctrl + C", false),
            "Ctrl+K Ctrl+C"
        );
    }
}
