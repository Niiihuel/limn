//! Normalize egui-winit's physical-key fallback before application shortcuts.
//!
//! egui-winit emits Key followed immediately by Text from one native keypress.
//! When a layout's printable symbol has no egui::Key, its physical position is
//! used instead. Virtual and remapped keyboards can put printable symbols at
//! positions normally occupied by Enter, Backspace, or navigation keys.

use eframe::egui::{self, Event, Key};
use std::collections::HashSet;

#[derive(Clone, Default)]
struct KeyboardNormalization {
    // A release can arrive in a later frame. Keep it paired with the removed
    // physical press, rather than exposing a release without a matching press.
    suppressed: HashSet<Key>,
}

/// Call before reading global key shortcuts or forwarding events to any view.
pub fn normalize_keyboard_input(ctx: &egui::Context) {
    if !ctx.input(|input| {
        input
            .events
            .iter()
            .any(|event| matches!(event, Event::Key { .. } | Event::WindowFocused(false)))
    }) {
        return;
    }
    let id = egui::Id::new(("lienzo_keyboard_normalization", ctx.viewport_id()));
    let mut state = ctx.data_mut(|data| {
        data.get_temp::<KeyboardNormalization>(id)
            .unwrap_or_default()
    });
    ctx.input_mut(|input| normalize(input, &mut state));
    ctx.data_mut(|data| data.insert_temp(id, state));
}

fn command_position(key: Key) -> bool {
    matches!(
        key,
        Key::ArrowDown
            | Key::ArrowLeft
            | Key::ArrowRight
            | Key::ArrowUp
            | Key::Escape
            | Key::Tab
            | Key::Backspace
            | Key::Enter
            | Key::Insert
            | Key::Delete
            | Key::Home
            | Key::End
            | Key::PageUp
            | Key::PageDown
            | Key::Copy
            | Key::Cut
            | Key::Paste
            | Key::BrowserBack
    ) || (Key::F1..=Key::F35).contains(&key)
}

fn printable(event: Option<&Event>) -> bool {
    matches!(event, Some(Event::Text(text))
        if !text.is_empty() && text.chars().all(|character| !character.is_control()))
}

fn normalize(input: &mut egui::InputState, state: &mut KeyboardNormalization) {
    let source = std::mem::take(&mut input.events);
    let mut normalized = Vec::with_capacity(source.len());
    let mut source = source.into_iter().peekable();
    while let Some(mut event) = source.next() {
        match &mut event {
            Event::Key {
                key,
                physical_key,
                pressed,
                modifiers,
                repeat,
            } => {
                if *pressed
                    && *physical_key == Some(*key)
                    && command_position(*key)
                    && !modifiers.ctrl
                    && !modifiers.command
                    && !modifiers.mac_cmd
                    && printable(source.peek())
                {
                    state.suppressed.insert(*key);
                    input.keys_down.remove(key);
                    continue;
                }
                if !*pressed && *physical_key == Some(*key) && state.suppressed.remove(key) {
                    input.keys_down.remove(key);
                    continue;
                }
                if *pressed {
                    if physical_key.is_some_and(|physical| state.suppressed.remove(&physical)) {
                        // A new genuine press supersedes a missing release or
                        // a changed keymap at the same physical position.
                        *repeat = false;
                    }
                    input.keys_down.insert(*key);
                } else {
                    input.keys_down.remove(key);
                }
            }
            Event::WindowFocused(false) => {
                state.suppressed.clear();
                input.keys_down.clear();
            }
            _ => {}
        }
        normalized.push(event);
    }
    input.events = normalized;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(key: Key, pressed: bool) -> Event {
        Event::Key {
            key,
            physical_key: Some(key),
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::default(),
        }
    }

    fn frame(ctx: &egui::Context, events: Vec<Event>) -> (Vec<Event>, HashSet<Key>) {
        let mut observed = None;
        let mut output = ctx.run_ui(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ui| {
                normalize_keyboard_input(ui.ctx());
                observed = Some(ui.ctx().input(|input| {
                    for key in [Key::Enter, Key::Backspace, Key::Escape, Key::Delete] {
                        assert_eq!(
                            input.key_pressed(key),
                            input.events.iter().any(|event| matches!(event,
                                Event::Key { key: candidate, pressed: true, .. } if *candidate == key
                            ))
                        );
                    }
                    (input.events.clone(), input.keys_down.clone())
                }));
            },
        );
        output.textures_delta.clear();
        observed.unwrap()
    }

    #[test]
    fn physical_control_fallbacks_preserve_punctuation_and_unicode_as_text_only() {
        let ctx = egui::Context::default();
        let mut source = Vec::new();
        let mut expected = Vec::new();
        for (position, text) in [
            (Key::Backspace, "_"),
            (Key::Backspace, "%"),
            (Key::Enter, ">"),
            (Key::Escape, "é"),
            (Key::Delete, "世界"),
            (Key::ArrowLeft, "🧠"),
            (Key::F2, "€"),
        ] {
            source.extend([
                key(position, true),
                Event::Text(text.into()),
                key(position, false),
            ]);
            expected.push(Event::Text(text.into()));
        }
        let (events, held) = frame(&ctx, source);
        assert_eq!(events, expected);
        assert!(held.is_empty());
        assert_eq!(
            super::super::events(&ctx, egui::Pos2::ZERO),
            expected
                .into_iter()
                .map(|event| match event {
                    Event::Text(text) => lienzo::UiEvent::Text(text),
                    _ => unreachable!(),
                })
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn real_keys_shortcuts_and_event_boundaries_are_preserved() {
        let mut source = vec![
            key(Key::Enter, true),
            key(Key::Enter, false),
            Event::Text("界".into()),
            key(Key::Enter, true),
            key(Key::A, true),
            Event::Text("a".into()),
            key(Key::A, false),
            key(Key::Enter, false),
            key(Key::Backspace, true),
            Event::PointerMoved(egui::Pos2::new(1.0, 2.0)),
            Event::Text("_".into()),
            key(Key::Backspace, false),
            key(Key::Enter, true),
            Event::Text("\n".into()),
            key(Key::Enter, false),
            key(Key::Space, true),
            Event::Text(" ".into()),
            key(Key::Space, false),
        ];
        for modifiers in [
            egui::Modifiers {
                ctrl: true,
                ..Default::default()
            },
            egui::Modifiers {
                command: true,
                mac_cmd: true,
                ..Default::default()
            },
        ] {
            let mut shortcut = key(Key::Enter, true);
            if let Event::Key {
                modifiers: value, ..
            } = &mut shortcut
            {
                *value = modifiers;
            }
            source.extend([shortcut, Event::Text(">".into()), key(Key::Enter, false)]);
        }
        // Without a matching physical position there is no evidence that the
        // semantic key came from egui-winit's fallback.
        for physical_key in [None, Some(Key::A)] {
            let mut semantic = key(Key::Enter, true);
            if let Event::Key {
                physical_key: value,
                ..
            } = &mut semantic
            {
                *value = physical_key;
            }
            source.extend([semantic, Event::Text(">".into()), key(Key::Enter, false)]);
        }
        let (events, held) = frame(&egui::Context::default(), source.clone());
        assert_eq!(events, source);
        assert!(held.is_empty());
    }

    #[test]
    fn suppressed_release_crosses_frames_without_leaving_a_key_held() {
        let ctx = egui::Context::default();
        let (events, held) = frame(&ctx, vec![key(Key::Enter, true), Event::Text(">".into())]);
        assert_eq!(events, vec![Event::Text(">".into())]);
        assert!(!held.contains(&Key::Enter));
        let (events, held) = frame(&ctx, vec![key(Key::Enter, false)]);
        assert!(events.is_empty());
        assert!(!held.contains(&Key::Enter));
        let (events, held) = frame(&ctx, vec![key(Key::Enter, true)]);
        assert_eq!(events, vec![key(Key::Enter, true)]);
        assert!(held.contains(&Key::Enter));
        let (events, held) = frame(&ctx, vec![key(Key::Enter, false)]);
        assert_eq!(events, vec![key(Key::Enter, false)]);
        assert!(held.is_empty());
    }

    #[test]
    fn genuine_press_and_focus_reset_supersede_suppressed_positions() {
        let ctx = egui::Context::default();
        frame(&ctx, vec![key(Key::Escape, true), Event::Text("é".into())]);
        let (events, held) = frame(&ctx, vec![key(Key::Escape, true)]);
        assert_eq!(events, vec![key(Key::Escape, true)]);
        assert!(held.contains(&Key::Escape));
        frame(&ctx, vec![key(Key::Escape, false)]);
        frame(&ctx, vec![key(Key::Enter, true), Event::Text(">".into())]);
        frame(&ctx, vec![Event::WindowFocused(false)]);
        let (events, held) = frame(&ctx, vec![key(Key::Enter, true), key(Key::Enter, false)]);
        assert_eq!(events, vec![key(Key::Enter, true), key(Key::Enter, false)]);
        assert!(held.is_empty());
    }
}
