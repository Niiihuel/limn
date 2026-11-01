//! Shortcuts drawn as text app-wide. In a process of its own, as it changes
//! the tokens every component reads.

use limn::{
    set_tokens, Button, Color, Component, ComponentPalette, DrawCommand, DropdownMenu, DropdownMenuItem, Element, Frame,
    SelectorSizing, ShortcutStyle, Size, Tokens, Tooltip, TooltipStyle, UiTree,
};

const WINDOW: Size = Size::new(800.0, 600.0);

fn menu(palette: ComponentPalette) -> Frame {
    let entries = vec![DropdownMenuItem::new("upload", "Upload").shortcut("Ctrl+U").into()];
    let menu = DropdownMenu::new(Element::button("more", "More"), entries, true, "menu", palette)
        .sizing(SelectorSizing::COMPACT)
        .render();
    let mut tree = UiTree::new(menu);
    tree.set_reduced_motion(true);
    tree.layout(WINDOW)
}

fn hint(palette: ComponentPalette) -> Frame {
    let button = Button::new("model", "M", palette).render();
    let mut tree = UiTree::new(Tooltip::new(button, "model", "Model", palette).shortcut("Ctrl+Shift+I").render());
    tree.set_reduced_motion(true);
    tree.layout(WINDOW);
    assert!(tree.focus("model"));
    tree.layout(WINDOW)
}

/// The colors of the texts drawn exactly as `wanted`.
fn texts(frame: &Frame, wanted: &str) -> Vec<Color> {
    frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, color, .. } if text == wanted => Some(*color),
            _ => None,
        })
        .collect()
}

#[test]
fn text_shortcuts_replace_keycaps_in_menus_and_tooltips() {
    let palette = ComponentPalette::neutral_dark();
    // Keycaps by default: one text per key.
    assert!(texts(&menu(palette), "Ctrl+U").is_empty());
    assert!(texts(&hint(palette), "Ctrl+Shift+I").is_empty());

    let muted = Color::rgb(0xA3, 0xA3, 0xA3);
    let style = TooltipStyle {
        background: Color::rgb(25, 25, 25),
        border: Color::TRANSPARENT,
        keycap: Color::rgb(60, 60, 60),
        text: Color::WHITE,
        muted,
        shadow: Color::rgba(0, 0, 0, 80),
    };
    set_tokens(Tokens { shortcut_style: ShortcutStyle::Text, tooltip: Some(style), ..Tokens::DEFAULT });
    assert_eq!(texts(&menu(palette), "Ctrl+U"), vec![palette.muted]);
    assert!(texts(&menu(palette), "U").is_empty(), "no keycaps");
    assert_eq!(texts(&hint(palette), "Ctrl+Shift+I"), vec![muted]);
    assert_eq!(texts(&hint(palette), "Model"), vec![Color::WHITE]);
    set_tokens(Tokens::DEFAULT);
}
