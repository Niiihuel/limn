use super::*;

/// The keys a host can forward in [`UiEvent::Key`] and
/// [`UiEvent::KeyReleased`]. The modifiers are not keys of their own: they
/// arrive as the fields of a key event and as [`UiEvent::ModifiersChanged`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Key {
    /// Page Up: without `ctrl`, scrolls up a page behind the focus or under the
    /// pointer.
    PageUp,
    /// Page Down: without `ctrl`, scrolls down a page behind the focus or under
    /// the pointer.
    PageDown,
    /// Escape: dismisses the open popover or cancels a drag in progress.
    Escape,
    /// Tab: outside an editable code editor, moves focus to the next control,
    /// or to the previous one with `shift`.
    Tab,
    /// Left arrow: moves the caret, or steps a focused slider, tab strip, menu
    /// or coach mark.
    ArrowLeft,
    /// Right arrow: moves the caret, or steps a focused slider, tab strip, menu
    /// or coach mark.
    ArrowRight,
    /// Up arrow: moves the caret up a line in a text area, or steps a tab strip
    /// or menu.
    ArrowUp,
    /// Down arrow: moves the caret down a line in a text area, or steps a tab
    /// strip or menu.
    ArrowDown,
    /// Home: start of the line, or of the whole text with `ctrl` or in a
    /// single-line field. Sliders, tab strips and menus read it too.
    Home,
    /// End: end of the line, or of the whole text with `ctrl` or in a
    /// single-line field. Sliders, tab strips and menus read it too.
    End,
    /// Backspace: deletes backward in the focused text field.
    Backspace,
    /// Delete: deletes forward in the focused text field.
    Delete,
    /// Enter: activates the focused button, submits a text input, or adds a
    /// line to a text area (which Ctrl+Enter submits, and Enter alone when it
    /// has `enter_submits`).
    Enter,
    /// Space: activates the focused button. Text fields take the character from
    /// `UiEvent::Text`, not from this key.
    Space,
    /// The A key: with `ctrl`, selects all of the focused text field.
    A,
    /// With `ctrl`, a Vim motion in a code editor in Vim mode.
    B,
    /// The C key: with `ctrl`, reports `UiAction::CopyRequested` for the
    /// selection.
    C,
    /// The D key: with `ctrl` in a code editor, selects the next occurrence, or
    /// every one with `shift`.
    D,
    /// Read only as the shortcut of an item in an open context menu.
    E,
    /// With `ctrl`, a Vim motion in a code editor in Vim mode.
    F,
    /// Read only as the shortcut of an item in an open context menu.
    G,
    /// Read only as the shortcut of an item in an open context menu.
    H,
    /// Read only as the shortcut of an item in an open context menu.
    I,
    /// With `ctrl`, a Vim motion in a code editor in Vim mode.
    J,
    /// Read only as the shortcut of an item in an open context menu.
    K,
    /// Read only as the shortcut of an item in an open context menu.
    L,
    /// Read only as the shortcut of an item in an open context menu.
    M,
    /// With `ctrl`, a Vim motion in a code editor in Vim mode.
    N,
    /// Read only as the shortcut of an item in an open context menu.
    O,
    /// With `ctrl`, a Vim motion in a code editor in Vim mode.
    P,
    /// Read only as the shortcut of an item in an open context menu.
    Q,
    /// With `ctrl`, a Vim motion in a code editor in Vim mode.
    R,
    /// The S key: with `ctrl` in a code editor, reports
    /// `UiAction::EditorSaveRequested`.
    S,
    /// Read only as the shortcut of an item in an open context menu.
    T,
    /// With `ctrl`, a Vim motion in a code editor in Vim mode.
    U,
    /// The V key. Pasted text arrives as `UiEvent::Paste`, not as this key.
    V,
    /// Read only as the shortcut of an item in an open context menu.
    W,
    /// The X key: with `ctrl`, reports `UiAction::CopyRequested` for the
    /// selection and deletes it.
    X,
    /// The Y key: with `ctrl` in an editable code editor, redoes.
    Y,
    /// The Z key: with `ctrl` in an editable code editor, undoes, or redoes
    /// with `shift`.
    Z,
    /// The slash key (`/`), read as a key for shortcuts such as Ctrl+/. The
    /// character still arrives as `UiEvent::Text`.
    Slash,
    /// Read only as the shortcut of an item in an open context menu.
    F1,
    /// Read only as the shortcut of an item in an open context menu.
    F2,
    /// Read only as the shortcut of an item in an open context menu.
    F3,
    /// Read only as the shortcut of an item in an open context menu.
    F4,
    /// Read only as the shortcut of an item in an open context menu.
    F5,
    /// Read only as the shortcut of an item in an open context menu.
    F6,
    /// Read only as the shortcut of an item in an open context menu.
    F7,
    /// Read only as the shortcut of an item in an open context menu.
    F8,
    /// Read only as the shortcut of an item in an open context menu.
    F9,
    /// Read only as the shortcut of an item in an open context menu.
    F10,
    /// Read only as the shortcut of an item in an open context menu.
    F11,
    /// Read only as the shortcut of an item in an open context menu.
    F12,
    /// Insert, read only as a host's shortcut.
    Insert,
    /// The `:` key. The character still arrives as `UiEvent::Text`.
    Colon,
    /// The `,` key. The character still arrives as `UiEvent::Text`.
    Comma,
    /// The `-` key. The character still arrives as `UiEvent::Text`.
    Minus,
    /// The `.` key. The character still arrives as `UiEvent::Text`.
    Period,
    /// The `+` key. The character still arrives as `UiEvent::Text`.
    Plus,
    /// The `=` key. The character still arrives as `UiEvent::Text`.
    Equals,
    /// The `;` key. The character still arrives as `UiEvent::Text`.
    Semicolon,
    /// The `[` key. The character still arrives as `UiEvent::Text`.
    OpenBracket,
    /// The `]` key. The character still arrives as `UiEvent::Text`.
    CloseBracket,
    /// The `{` key. The character still arrives as `UiEvent::Text`.
    OpenCurlyBracket,
    /// The `}` key. The character still arrives as `UiEvent::Text`.
    CloseCurlyBracket,
    /// The ``` key. The character still arrives as `UiEvent::Text`.
    Backtick,
    /// The `\` key. The character still arrives as `UiEvent::Text`.
    Backslash,
    /// The `|` key. The character still arrives as `UiEvent::Text`.
    Pipe,
    /// The `?` key. The character still arrives as `UiEvent::Text`.
    Questionmark,
    /// The `!` key. The character still arrives as `UiEvent::Text`.
    Exclamationmark,
    /// The `'` key. The character still arrives as `UiEvent::Text`.
    Quote,
    /// The 0 key, on the top row or the keypad. The character still
    /// arrives as `UiEvent::Text`.
    Num0,
    /// The 1 key, on the top row or the keypad. The character still
    /// arrives as `UiEvent::Text`.
    Num1,
    /// The 2 key, on the top row or the keypad. The character still
    /// arrives as `UiEvent::Text`.
    Num2,
    /// The 3 key, on the top row or the keypad. The character still
    /// arrives as `UiEvent::Text`.
    Num3,
    /// The 4 key, on the top row or the keypad. The character still
    /// arrives as `UiEvent::Text`.
    Num4,
    /// The 5 key, on the top row or the keypad. The character still
    /// arrives as `UiEvent::Text`.
    Num5,
    /// The 6 key, on the top row or the keypad. The character still
    /// arrives as `UiEvent::Text`.
    Num6,
    /// The 7 key, on the top row or the keypad. The character still
    /// arrives as `UiEvent::Text`.
    Num7,
    /// The 8 key, on the top row or the keypad. The character still
    /// arrives as `UiEvent::Text`.
    Num8,
    /// The 9 key, on the top row or the keypad. The character still
    /// arrives as `UiEvent::Text`.
    Num9,
    /// The F13 key, read only as a host's shortcut.
    F13,
    /// The F14 key, read only as a host's shortcut.
    F14,
    /// The F15 key, read only as a host's shortcut.
    F15,
    /// The F16 key, read only as a host's shortcut.
    F16,
    /// The F17 key, read only as a host's shortcut.
    F17,
    /// The F18 key, read only as a host's shortcut.
    F18,
    /// The F19 key, read only as a host's shortcut.
    F19,
    /// The F20 key, read only as a host's shortcut.
    F20,
    /// The F21 key, read only as a host's shortcut.
    F21,
    /// The F22 key, read only as a host's shortcut.
    F22,
    /// The F23 key, read only as a host's shortcut.
    F23,
    /// The F24 key, read only as a host's shortcut.
    F24,
    /// The F25 key, read only as a host's shortcut.
    F25,
    /// The F26 key, read only as a host's shortcut.
    F26,
    /// The F27 key, read only as a host's shortcut.
    F27,
    /// The F28 key, read only as a host's shortcut.
    F28,
    /// The F29 key, read only as a host's shortcut.
    F29,
    /// The F30 key, read only as a host's shortcut.
    F30,
    /// The F31 key, read only as a host's shortcut.
    F31,
    /// The F32 key, read only as a host's shortcut.
    F32,
    /// The F33 key, read only as a host's shortcut.
    F33,
    /// The F34 key, read only as a host's shortcut.
    F34,
    /// The F35 key, read only as a host's shortcut.
    F35,
    /// The key between the left Shift and Z on ISO keyboards, `<` on many
    /// of their layouts, named by its place. The character still arrives as
    /// `UiEvent::Text`.
    IntlBackslash,
}

impl Key {
    /// Every key, in the order the enum declares them.
    pub const ALL: &'static [Self] = &[
        Self::PageUp,
        Self::PageDown,
        Self::Escape,
        Self::Tab,
        Self::ArrowLeft,
        Self::ArrowRight,
        Self::ArrowUp,
        Self::ArrowDown,
        Self::Home,
        Self::End,
        Self::Backspace,
        Self::Delete,
        Self::Enter,
        Self::Space,
        Self::A,
        Self::B,
        Self::C,
        Self::D,
        Self::E,
        Self::F,
        Self::G,
        Self::H,
        Self::I,
        Self::J,
        Self::K,
        Self::L,
        Self::M,
        Self::N,
        Self::O,
        Self::P,
        Self::Q,
        Self::R,
        Self::S,
        Self::T,
        Self::U,
        Self::V,
        Self::W,
        Self::X,
        Self::Y,
        Self::Z,
        Self::Slash,
        Self::F1,
        Self::F2,
        Self::F3,
        Self::F4,
        Self::F5,
        Self::F6,
        Self::F7,
        Self::F8,
        Self::F9,
        Self::F10,
        Self::F11,
        Self::F12,
        Self::Insert,
        Self::Colon,
        Self::Comma,
        Self::Minus,
        Self::Period,
        Self::Plus,
        Self::Equals,
        Self::Semicolon,
        Self::OpenBracket,
        Self::CloseBracket,
        Self::OpenCurlyBracket,
        Self::CloseCurlyBracket,
        Self::Backtick,
        Self::Backslash,
        Self::Pipe,
        Self::Questionmark,
        Self::Exclamationmark,
        Self::Quote,
        Self::Num0,
        Self::Num1,
        Self::Num2,
        Self::Num3,
        Self::Num4,
        Self::Num5,
        Self::Num6,
        Self::Num7,
        Self::Num8,
        Self::Num9,
        Self::F13,
        Self::F14,
        Self::F15,
        Self::F16,
        Self::F17,
        Self::F18,
        Self::F19,
        Self::F20,
        Self::F21,
        Self::F22,
        Self::F23,
        Self::F24,
        Self::F25,
        Self::F26,
        Self::F27,
        Self::F28,
        Self::F29,
        Self::F30,
        Self::F31,
        Self::F32,
        Self::F33,
        Self::F34,
        Self::F35,
        Self::IntlBackslash,
    ];

    /// The key's English name, as hosts write it in a shortcut and egui
    /// names it: `"A"`, `"1"`, `"F13"`, `"Up"`, `"Space"`, `"Backtick"`.
    /// [`Self::from_name`] reads it back.
    pub fn name(self) -> &'static str {
        match self {
            Self::ArrowLeft => "Left",
            Self::ArrowRight => "Right",
            Self::ArrowUp => "Up",
            Self::ArrowDown => "Down",
            Self::PageUp => "PageUp",
            Self::PageDown => "PageDown",
            Self::Escape => "Escape",
            Self::Tab => "Tab",
            Self::Home => "Home",
            Self::End => "End",
            Self::Backspace => "Backspace",
            Self::Delete => "Delete",
            Self::Enter => "Enter",
            Self::Space => "Space",
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
            Self::E => "E",
            Self::F => "F",
            Self::G => "G",
            Self::H => "H",
            Self::I => "I",
            Self::J => "J",
            Self::K => "K",
            Self::L => "L",
            Self::M => "M",
            Self::N => "N",
            Self::O => "O",
            Self::P => "P",
            Self::Q => "Q",
            Self::R => "R",
            Self::S => "S",
            Self::T => "T",
            Self::U => "U",
            Self::V => "V",
            Self::W => "W",
            Self::X => "X",
            Self::Y => "Y",
            Self::Z => "Z",
            Self::Slash => "Slash",
            Self::F1 => "F1",
            Self::F2 => "F2",
            Self::F3 => "F3",
            Self::F4 => "F4",
            Self::F5 => "F5",
            Self::F6 => "F6",
            Self::F7 => "F7",
            Self::F8 => "F8",
            Self::F9 => "F9",
            Self::F10 => "F10",
            Self::F11 => "F11",
            Self::F12 => "F12",
            Self::Insert => "Insert",
            Self::Colon => "Colon",
            Self::Comma => "Comma",
            Self::Minus => "Minus",
            Self::Period => "Period",
            Self::Plus => "Plus",
            Self::Equals => "Equals",
            Self::Semicolon => "Semicolon",
            Self::OpenBracket => "OpenBracket",
            Self::CloseBracket => "CloseBracket",
            Self::OpenCurlyBracket => "OpenCurlyBracket",
            Self::CloseCurlyBracket => "CloseCurlyBracket",
            Self::Backtick => "Backtick",
            Self::Backslash => "Backslash",
            Self::Pipe => "Pipe",
            Self::Questionmark => "Questionmark",
            Self::Exclamationmark => "Exclamationmark",
            Self::Quote => "Quote",
            Self::Num0 => "0",
            Self::Num1 => "1",
            Self::Num2 => "2",
            Self::Num3 => "3",
            Self::Num4 => "4",
            Self::Num5 => "5",
            Self::Num6 => "6",
            Self::Num7 => "7",
            Self::Num8 => "8",
            Self::Num9 => "9",
            Self::F13 => "F13",
            Self::F14 => "F14",
            Self::F15 => "F15",
            Self::F16 => "F16",
            Self::F17 => "F17",
            Self::F18 => "F18",
            Self::F19 => "F19",
            Self::F20 => "F20",
            Self::F21 => "F21",
            Self::F22 => "F22",
            Self::F23 => "F23",
            Self::F24 => "F24",
            Self::F25 => "F25",
            Self::F26 => "F26",
            Self::F27 => "F27",
            Self::F28 => "F28",
            Self::F29 => "F29",
            Self::F30 => "F30",
            Self::F31 => "F31",
            Self::F32 => "F32",
            Self::F33 => "F33",
            Self::F34 => "F34",
            Self::F35 => "F35",
            Self::IntlBackslash => "IntlBackslash",
        }
    }

    /// The key a name stands for: [`Self::name`], the variant's own name
    /// (`"ArrowUp"`, `"Num1"`) or the character the key types (`"/"`,
    /// `"-"`), names in either case. `None` for anything else.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|key| {
            key.name().eq_ignore_ascii_case(name)
                || format!("{key:?}").eq_ignore_ascii_case(name)
                || key.symbol().is_some_and(|symbol| name.chars().eq([symbol]))
        })
    }

    /// The character a punctuation key types, for [`Self::from_name`].
    fn symbol(self) -> Option<char> {
        Some(match self {
            Self::Space => ' ',
            Self::Slash => '/',
            Self::Colon => ':',
            Self::Comma => ',',
            Self::Minus => '-',
            Self::Period => '.',
            Self::Plus => '+',
            Self::Equals => '=',
            Self::Semicolon => ';',
            Self::OpenBracket => '[',
            Self::CloseBracket => ']',
            Self::OpenCurlyBracket => '{',
            Self::CloseCurlyBracket => '}',
            Self::Backtick => '`',
            Self::Backslash => '\\',
            Self::Pipe => '|',
            Self::Questionmark => '?',
            Self::Exclamationmark => '!',
            Self::Quote => '\'',
            _ => return None,
        })
    }
}

/// A pointer button that has no event of its own: the middle button and the
/// two side buttons. Primary and secondary have their own events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OtherButton {
    /// The wheel button (mouse button 3).
    Middle,
    /// The first side button (mouse button 4).
    Back,
    /// The second side button (mouse button 5).
    Forward,
}

/// Input a host forwards to `UiTree::handle_event`. Positions are in
/// logical pixels.
#[derive(Clone, Debug, PartialEq)]
pub enum UiEvent {
    /// Open text or host-defined actions without changing the selection.
    ContextMenu {
        /// Where the secondary button was pressed; the menu opens there.
        position: Point,
    },
    /// Hosts map Shift+F10 or their platform's context-menu key here.
    ContextMenuKeyboard,
    /// Clipboard replies remain bound to the editor which requested them.
    PasteTo {
        /// The control that reported `UiAction::PasteRequested`. The text is
        /// dropped unless that control is still focused and laid out.
        id: String,
        /// The clipboard text to insert there.
        text: String,
    },
    /// Deliver an asynchronous reply only to its captured editing state.
    /// A rejected target never falls back to whichever control is focused now.
    PasteCaptured {
        /// The destination captured before the host authorized its request.
        target: PasteTarget,
        /// Clipboard text, subject to the control's ordinary paste policy.
        text: String,
    },
    /// The text an input method is composing, replacing what it sent before.
    /// A code editor shows it, and so does a text field or text area in a
    /// tree that lays its text out, in place of its selection, underlined.
    ImePreedit(String),
    /// An input method committed this text. A code editor ends its composition
    /// with it; other text fields take it as `Text`.
    ImeCommit(String),
    /// The composition ended without a commit: what was being composed is
    /// dropped.
    ImeCancelled,
    /// An input method asks the focused code editor or text field to delete
    /// text around the caret.
    ImeDeleteSurrounding {
        /// Characters to delete before the caret.
        before_chars: usize,
        /// Characters to delete after the caret.
        after_chars: usize,
    },
    /// Additive editor selection gesture; ordinary controls treat it as a click.
    PointerDownModified {
        /// Where the primary button was pressed.
        position: Point,
        /// True to extend a code editor's selection to `position`.
        shift: bool,
    },
    /// Native editor pointer gestures; other controls receive an ordinary click.
    PointerDownEditor {
        /// Where the primary button was pressed.
        position: Point,
        /// Extends the selection to `position`; with `alt`, selects a
        /// rectangle.
        shift: bool,
        /// Adds a caret at `position`; with `shift`, selects a rectangle.
        alt: bool,
        /// Whether Ctrl or Command was held. The tree does not read it.
        ctrl: bool,
        /// 1 for a single click, 2 for a double click, 3 for a triple click.
        click_count: u8,
    },
    /// Pixel wheel deltas; hosts retain the legacy wheel event outside editors.
    Scroll2D {
        /// Where the pointer is; what scrolls is under it.
        position: Point,
        /// Horizontal travel in logical pixels; a positive value moves toward
        /// the left edge of the content.
        delta_x: f32,
        /// Vertical travel in logical pixels; a positive value moves toward the
        /// top of the content.
        delta_y: f32,
        /// True for pixel-precise input, handled as `ScrollDirect`; false is
        /// eased as `Scroll` is.
        direct: bool,
        /// True while Shift is held: with a `delta_x` of 0, a code editor or
        /// diff scrolls sideways by `delta_y`.
        shift: bool,
    },
    /// A direct pan the host translated into `Scroll2D` events has ended: the
    /// finger is up. An iOS sheet the pan took comes to rest, or leaves.
    ScrollEnded,
    /// Clipboard text for the focused control. A text field inserts it as it
    /// does `Text`.
    Paste(String),
    /// Files dragged from another application are over the window.
    FilesHovered {
        /// The pointer's position, as far as the host knows it.
        position: Point,
    },
    /// Files dragged from another application were dropped on the window.
    FilesDropped {
        /// Where they were dropped.
        position: Point,
        /// Paths of the dropped files.
        paths: Vec<std::path::PathBuf>,
    },
    /// The clipboard held a picture and Ctrl+V was pressed. Pixels are RGBA,
    /// unpremultiplied, row by row from the top.
    PasteImage {
        /// Width in pixels.
        width: usize,
        /// Height in pixels.
        height: usize,
        /// `width * height * 4` bytes.
        rgba: Arc<[u8]>,
    },
    /// The pointer moved to this position: updates what is hovered and carries
    /// on a drag, a text selection or a scrollbar drag.
    PointerMove(Point),
    /// The pointer left the window: clears the hover and the press and ends any
    /// drag.
    PointerLeave,
    /// The primary button was pressed at this position, with no modifier held.
    PointerDown(Point),
    /// The primary button was released at this position. A release on the
    /// control that was pressed reports `UiAction::Clicked`.
    PointerUp(Point),
    /// A wheel step, converted to logical pixels by the host (`limn-egui`
    /// moves a notch 120 px, as a browser does). The scrolling area under the
    /// pointer eases to its new offset.
    Scroll {
        /// Where the pointer is; what scrolls is under it.
        position: Point,
        /// Travel in logical pixels; a positive value moves toward the top of
        /// the content.
        delta_y: f32,
    },
    /// Pixel-precise input, such as a touchpad, already supplies a continuous
    /// stream. Keep it attached to the gesture instead of adding wheel easing.
    ScrollDirect {
        /// Where the pointer is; what scrolls is under it.
        position: Point,
        /// Travel in logical pixels; a positive value moves toward the top of
        /// the content.
        delta_y: f32,
    },
    /// Typed characters, inserted into the focused text field or code editor.
    Text(String),
    /// A key was pressed.
    Key {
        /// The key that was pressed.
        key: Key,
        /// True while Shift is held.
        shift: bool,
        /// True while Ctrl is held. The egui backend sets it for Command too.
        ctrl: bool,
        /// True while Alt (Option) is held. The tree reads it only to match a
        /// context menu item's shortcut; a host reads it for its own, such as
        /// Alt+Left.
        alt: bool,
    },
    /// The pointer's back button (the first side button, mouse button 4) was
    /// pressed at this position. The tree does nothing with it; a host reads
    /// it to go back in its own history.
    PointerBack(Point),
    /// The pointer's forward button (the second side button, mouse button 5)
    /// was pressed at this position. The tree does nothing with it; a host
    /// reads it to go forward in its own history.
    PointerForward(Point),
    /// The middle or a side button went down or up at this position. Unlike
    /// `PointerBack` and `PointerForward`, releases come too, so a host can
    /// bind the button as a hold. The tree answers with `UiAction::OtherButton`.
    OtherButton {
        /// Which button changed.
        button: OtherButton,
        /// `true` on press, `false` on release.
        pressed: bool,
        /// Where the pointer was.
        position: Point,
    },
    /// A key was released. The tree only stops counting it as held; see
    /// [`UiTree::key_held`].
    KeyReleased {
        /// The key that was released.
        key: Key,
    },
    /// The modifiers held changed, such as Shift pressed or released alone.
    /// The tree keeps them for [`UiTree::modifiers`]; nothing else reads
    /// them.
    ModifiersChanged(Modifiers),
    /// A pinch on a touchpad or a touch screen: what lies under `position`
    /// is to be shown `factor` times as large. A zoom view under it zooms;
    /// nothing else reads it.
    Zoom {
        /// Where the fingers are.
        position: Point,
        /// Above 1 to zoom in, below 1 to zoom out; one that is not finite or
        /// not above zero is ignored.
        factor: f32,
    },
}

/// Which modifier keys are held; see [`UiTree::modifiers`].
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Modifiers {
    /// True while Shift is held.
    pub shift: bool,
    /// True while Ctrl is held. The egui backend sets it for Command too.
    pub ctrl: bool,
    /// True while Alt (Option) is held.
    pub alt: bool,
}

/// What `UiTree::handle_event` reports for the host to act on.
#[derive(Clone, Debug, PartialEq)]
pub enum UiAction {
    /// An edit changed the document of a code editor.
    EditorChanged {
        /// ID of the code editor.
        id: String,
        /// The buffer's document ID.
        document_id: u64,
        /// The buffer's revision after the edit.
        revision: u64,
    },
    /// Ctrl+S was pressed in a code editor. Saving is the host's work.
    EditorSaveRequested {
        /// ID of the code editor.
        id: String,
        /// The buffer's document ID.
        document_id: u64,
        /// The buffer's revision when the key was pressed.
        revision: u64,
    },
    /// A code editor could not carry out a command, such as an edit, or a copy
    /// over the clipboard limit.
    EditorFailed {
        /// ID of the code editor.
        id: String,
        /// The reason, as text.
        error: String,
    },
    /// A control was activated: pressed and released with the pointer, or Enter
    /// or Space while it had focus. Escape reports it for an open popover's
    /// dismiss ID, and a host's context menu for the chosen item's action ID.
    Clicked {
        /// The control's ID, or the action ID it was given to report instead.
        id: String,
    },
    /// Continuous preview on pointer down/move; snapped selection on release.
    SliderChanged {
        /// ID of the slider.
        id: String,
        /// Position along the track, from 0 to 1. A committed one is snapped to
        /// a step.
        fraction: f32,
        /// False while the pointer drags. True on release, when the pointer
        /// leaves the window, and for a step from the keyboard.
        committed: bool,
    },
    /// An area slider moved: previewed while the pointer is down, committed
    /// on release and from the keyboard. See [`Element::area_slider`].
    AreaSliderChanged {
        /// ID of the slider.
        id: String,
        /// Share of the width from the left, from 0 to 1.
        x: f32,
        /// Share of the height from the top, from 0 to 1.
        y: f32,
        /// False while the pointer drags. True on release, when the pointer
        /// leaves the window, and for a step from the keyboard.
        committed: bool,
    },
    /// A press and drag over a [`Sketch`](crate::Sketch) ended: the mark it
    /// made, in the sketch's own shares. Released where the press began it is
    /// a dot or a zero-length line. Nothing is reported while the pointer is
    /// down; the mark in progress is drawn by the tree.
    Sketched {
        /// ID of the sketch.
        id: String,
        /// The finished mark.
        mark: crate::SketchMark,
    },
    /// The text of a text input or text area changed.
    InputChanged {
        /// ID of the field.
        id: String,
        /// The field's whole text after the change.
        value: String,
    },
    /// Enter was pressed in a text input, or Ctrl+Enter in a text area (Enter
    /// alone in one with `enter_submits`).
    Submitted {
        /// ID of the field.
        id: String,
        /// The field's whole text.
        value: String,
    },
    /// The host should put `text` on the clipboard, for example after Ctrl+C or
    /// Ctrl+X on a selection.
    CopyRequested {
        /// The selected text. Ctrl+C and Ctrl+X never report a password field's
        /// selection.
        text: String,
    },
    /// The paste item of a text context menu was chosen. The host reads the
    /// clipboard and answers with `UiEvent::PasteTo`.
    PasteRequested {
        /// The text field or code editor to paste into; `PasteTo` carries it
        /// back.
        id: String,
    },
    /// The pointer left the click tolerance of a pressed `draggable` element.
    /// No `Clicked` follows this press.
    DragStarted {
        /// ID of the dragged element.
        id: String,
        /// Where the press began, not where the pointer is now.
        position: Point,
    },
    /// `over` is the topmost `drop_target` under the pointer, never the source.
    DragMoved {
        /// ID of the dragged element.
        id: String,
        /// Where the pointer is.
        position: Point,
        /// The drop target under the pointer; `None` over none.
        over: Option<String>,
    },
    /// The pointer was released during a drag.
    Dropped {
        /// ID of the dragged element.
        id: String,
        /// Where the pointer was released.
        position: Point,
        /// The topmost `drop_target` there other than the source; `None` when
        /// it was dropped on none.
        target: Option<String>,
    },
    /// Escape or the pointer leaving the window ended the drag.
    DragCancelled {
        /// ID of the element that was being dragged.
        id: String,
    },
    /// `over` and `target` are the topmost `drop_target` under the pointer.
    FilesHovered {
        /// The position the host sent.
        position: Point,
        /// The drop target at `position`; `None` over none.
        over: Option<String>,
    },
    /// Files from another application were dropped: the answer to
    /// `UiEvent::FilesDropped`.
    FilesDropped {
        /// The position the host sent.
        position: Point,
        /// The drop target at `position`; `None` when they were dropped on
        /// none.
        target: Option<String>,
        /// The paths the host sent, unchanged.
        paths: Vec<std::path::PathBuf>,
    },
    /// The middle or a side button went down or up: the answer to
    /// `UiEvent::OtherButton`. The tree does nothing else with it.
    OtherButton {
        /// Which button changed.
        button: OtherButton,
        /// `true` on press, `false` on release.
        pressed: bool,
        /// Where the pointer was.
        position: Point,
    },
    /// A picture was pasted: the answer to `UiEvent::PasteImage`. The host
    /// decides what to do with it; `target` is the focused control, if any.
    PasteImage {
        /// The control that had focus when the paste arrived.
        target: Option<String>,
        /// Width in pixels.
        width: usize,
        /// Height in pixels.
        height: usize,
        /// `width * height * 4` bytes, unpremultiplied RGBA.
        rgba: Arc<[u8]>,
    },
    /// A zoom view's scale changed (see [`Element::zoom_view`]): the host may
    /// show it, or fetch a sharper picture to zoom into.
    Zoomed {
        /// The zoom view's ID.
        id: String,
        /// Logical pixels on screen for each pixel of the picture, where it
        /// is going to rest.
        scale: f32,
        /// It is at its fit, following the box.
        fitted: bool,
    },
}
