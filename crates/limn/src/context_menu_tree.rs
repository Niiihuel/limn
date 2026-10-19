use super::*;
use crate::context_menu::{self, ContextMenuDefinition};
use crate::{ComponentPalette, ContextMenuEntry, ContextMenuItem};

#[derive(Clone, Debug)]
pub(super) struct ContextMenuRegion {
    pub definition: ContextMenuDefinition,
    pub rect: Rect,
    pub region_layer: usize,
}

impl PartialEq for ContextMenuRegion {
    fn eq(&self, other: &Self) -> bool {
        self.definition.id == other.definition.id
            && self.definition.entries == other.definition.entries
            && self.definition.open_action == other.definition.open_action
            && self.rect == other.rect
            && self.region_layer == other.region_layer
    }
}

impl Frame {
    /// The custom context menu under a point in this frame's coordinates.
    /// Floating surfaces and modal backdrops block targets beneath them.
    pub fn context_menu_at(&self, position: Point) -> Option<&str> {
        let modal = self.popover_focus.iter().rev().find(|scope| scope.modal);
        let floor = self.regions.iter().enumerate().rev().find_map(|(index, region)| {
            (region.kind == HitKind::Surface && (region.rect.contains(position)
                || modal.is_some_and(|scope| region.id.strip_prefix("popover_surface:") == Some(scope.dismiss_id.as_str()))))
                .then_some(index)
        }).unwrap_or(0);
        self.context_menu_regions.iter().rev().find(|region| {
            region.region_layer >= floor && region.rect.contains(position)
                && (!region.definition.entries.is_empty() || region.definition.open_action.is_some())
        }).map(|region| region.definition.id.as_str())
    }
}

#[derive(Clone)]
enum Target {
    Input { id: String },
    Editor { id: String, document_id: u64 },
    Text { block: usize, source: String },
    Custom { id: String },
}

pub(super) struct ContextMenuState {
    target: Target,
    position: Point,
    entries: Vec<ContextMenuEntry>,
    palette: ComponentPalette,
    selected: Option<usize>,
    pressed: Option<usize>,
    /// A menu opened inside a modal must not outlive that modal (or leak
    /// through a subsequently opened one).
    modal: Option<String>,
}

impl ContextMenuState {
    fn enabled(&self, index: usize) -> bool {
        matches!(self.entries.get(index), Some(ContextMenuEntry::Item(item)) if item.enabled)
    }
    pub(super) fn valid_in(&self, frame: &Frame) -> bool {
        if self.modal.as_deref()
            != frame
                .popover_focus
                .iter()
                .rev()
                .find(|scope| scope.modal)
                .map(|scope| scope.dismiss_id.as_str())
        {
            return false;
        }
        match &self.target {
            Target::Input { id } => frame.regions.iter().any(|region| {
                region.id == *id && matches!(region.kind, HitKind::TextInput | HitKind::TextArea)
            }),
            Target::Editor { id, .. } => frame
                .regions
                .iter()
                .any(|region| region.id == *id && region.kind == HitKind::CodeEditor),
            Target::Text { block, source } => frame
                .text_blocks
                .get(*block)
                .is_some_and(|text| text.source == *source),
            Target::Custom { id } => frame.context_menu_regions.iter().any(|region| {
                region.definition.id == *id && region.definition.open_action.is_none()
                    && region.definition.entries == self.entries
            }),
        }
    }
}

impl UiTree {
    /// Whether a context menu is open.
    pub fn context_menu_open(&self) -> bool {
        self.context_menu.is_some()
    }

    /// Close the open context menu, clearing the hovered and pressed controls with it.
    /// Returns whether one was open.
    pub fn dismiss_context_menu(&mut self) -> bool {
        let was_open = self.context_menu.is_some();
        self.close_context_menu(Instant::now());
        was_open
    }

    /// The host can match automatic text menus to its floating menu palette.
    pub fn set_context_menu_palette(&mut self, palette: ComponentPalette) {
        self.context_menu_palette = palette;
    }

    fn close_context_menu(&mut self, now: Instant) {
        if self.context_menu.is_some() {
            self.invalidate_paste_targets();
        }
        self.context_menu = None;
        self.pressed = None;
        self.scroll_drag = None;
        self.set_hovered(None, now);
    }

    pub(super) fn context_menu_paint(
        &self,
        viewport: Rect,
    ) -> Option<(Element, Rect, Option<String>, Option<String>)> {
        let menu = self.context_menu.as_ref()?;
        let width = context_menu::WIDTH.min((viewport.width - 16.0).max(0.0));
        // As a native menu, it takes the window's height before it scrolls.
        let max_height = (viewport.height - 16.0).max(0.0);
        let measure = |text: &str, size: f32| {
            measured_width(
                self.text_measurer.as_deref(),
                text,
                size,
                FontFamily::Sans,
                self.sans_metrics.at_size(size).char_width,
            )
        };
        let (surface, height) = context_menu::surface(
            &menu.entries,
            menu.palette,
            width,
            max_height,
            &measure,
            self.sans_metrics
                .at_size(context_menu::LABEL_FONT_SIZE)
                .line_height,
        );
        let margin_x = 8.0_f32.min(viewport.width / 2.0);
        let margin_y = 8.0_f32.min(viewport.height / 2.0);
        let rect = Rect::new(
            menu.position
                .x
                .clamp(margin_x, (viewport.width - width - margin_x).max(margin_x)),
            menu.position.y.clamp(
                margin_y,
                (viewport.height - height - margin_y).max(margin_y),
            ),
            width,
            height,
        );
        Some((
            surface,
            rect,
            menu.selected.map(context_menu::row_id),
            menu.pressed.map(context_menu::row_id),
        ))
    }

    fn open_text_context_menu(&mut self, position: Point, keyboard: bool, now: Instant) -> Option<UiAction> {
        self.close_context_menu(now);
        if !position.x.is_finite() || !position.y.is_finite() {
            return None;
        }
        // Ignore the old menu's paint until the next layout, but respect every
        // underlying floating surface and modal backdrop.
        let end = self
            .frame
            .regions
            .iter()
            .position(|r| r.id == "__context_menu:surface")
            .unwrap_or(self.frame.regions.len());
        let regions = &self.frame.regions[..end];
        let hit = regions
            .iter()
            .rev()
            .find(|region| {
                region.rect.contains(position)
                    && !matches!(region.kind, HitKind::Scroll | HitKind::Hover)
            })
            .cloned();
        let modal = self
            .frame
            .popover_focus
            .iter()
            .rev()
            .find(|scope| scope.modal)
            .map(|scope| scope.dismiss_id.clone());
        let floor = regions
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, region)| {
                (region.kind == HitKind::Surface
                    && (region.rect.contains(position)
                        || modal
                            .as_deref()
                            .is_some_and(|id| region.id == format!("popover_surface:{id}"))))
                .then_some(index)
            })
            .unwrap_or(0);
        let custom = self
            .frame
            .context_menu_regions
            .iter()
            .rev()
            .find(|region| {
                region.region_layer >= floor
                    && region.region_layer <= end
                    && region.rect.contains(position)
            })
            .cloned();
        let (target, entries, palette) = if let Some(region) = custom {
            if let Some(hit) = &hit {
                if matches!(
                    hit.kind,
                    HitKind::Button | HitKind::TextInput | HitKind::TextArea | HitKind::CodeEditor
                ) {
                    let next_focus = Some(hit.id.clone());
                    Self::replace_focused(&mut self.focused, &mut self.paste_epoch, next_focus);
                }
            }
            if let Some(id) = region.definition.open_action {
                return Some(UiAction::Clicked { id: self.dispatch_id(&id) });
            }
            (
                Target::Custom {
                    id: region.definition.id,
                },
                region.definition.entries,
                region.definition.palette,
            )
        } else if let Some(hit) = hit
            .as_ref()
            .filter(|hit| matches!(hit.kind, HitKind::TextInput | HitKind::TextArea))
        {
            let input = self.inputs.get(&hit.id)?;
            let selected = input.selected_range().is_some();
            let copy = selected && !input.password;
            let entries = text_entries(copy, copy, true, !input.value.is_empty());
            let next_focus = Some(hit.id.clone());
            Self::replace_focused(&mut self.focused, &mut self.paste_epoch, next_focus);
            (
                Target::Input { id: hit.id.clone() },
                entries,
                self.context_menu_palette,
            )
        } else if let Some(hit) = hit.as_ref().filter(|hit| hit.kind == HitKind::CodeEditor) {
            if !keyboard {
                let measurer = self.text_measurer.clone();
                let advance = self.metrics.char_width * 13.0 / DEFAULT_FONT_SIZE;
                let measure = |text: &str| {
                    measured_width(
                        measurer.as_deref(),
                        text,
                        13.0,
                        FontFamily::Monospace,
                        advance,
                    )
                };
                if let Some(editor) = self.code_editors.get_mut(&hit.id) {
                    editor.context_point(position, &measure);
                }
            }
            let editor = self.code_editors.get(&hit.id)?;
            let controller = editor.editor.controller.shared_buffer();
            let Ok(buffer) = controller.lock() else {
                return None;
            };
            let snapshot = buffer.snapshot();
            let copy = snapshot.selections().any(|selection| !selection.is_empty());
            let id = buffer.document_id();
            let all = snapshot.char_count() > 0;
            let mut entries = text_entries(
                copy,
                copy && !editor.editor.read_only,
                !editor.editor.read_only,
                all,
            );
            if !editor.editor.context_actions.is_empty() {
                entries.push(ContextMenuEntry::Separator);
                entries.extend(editor.editor.context_actions.clone());
            }
            let next_focus = Some(hit.id.clone());
            Self::replace_focused(&mut self.focused, &mut self.paste_epoch, next_focus);
            (
                Target::Editor {
                    id: hit.id.clone(),
                    document_id: id,
                },
                entries,
                self.context_menu_palette,
            )
        } else {
            // Button labels and backgrounds behind a modal are not text-copy
            // targets. Plain and formatted text share the existing selection.
            if hit.as_ref().is_some_and(|hit| {
                matches!(
                    hit.kind,
                    HitKind::Button | HitKind::Scrollbar | HitKind::CodeDiff
                )
            }) {
                return None;
            }
            let (index, block) =
                self.frame
                    .text_blocks
                    .iter()
                    .enumerate()
                    .rev()
                    .find(|(_, block)| {
                        block.region_layer >= floor
                            && block.region_layer <= end
                            && block.contains(position)
                    })?;
            let preserved = self.static_selection.as_ref().is_some_and(|selection| {
                selection.block == index && selection.source == block.source
            });
            if !preserved {
                let offset = block.offset_at(
                    position,
                    self.text_measurer.as_deref(),
                    self.metrics,
                    self.sans_metrics,
                );
                self.static_selection = Some(StaticSelection {
                    block: index,
                    source: block.source.clone(),
                    anchor: offset,
                    cursor: offset,
                    dragging: false,
                });
            } else if let Some(selection) = self.static_selection.as_mut() {
                selection.dragging = false;
            }
            let entries = text_entries(
                self.selected_text().is_some(),
                false,
                false,
                !block.source.is_empty(),
            );
            (
                Target::Text {
                    block: index,
                    source: block.source.clone(),
                },
                entries,
                self.context_menu_palette,
            )
        };
        if entries.is_empty() {
            return None;
        }
        let selected = keyboard
            .then(|| {
                entries
                    .iter()
                    .position(|entry| matches!(entry, ContextMenuEntry::Item(item) if item.enabled))
            })
            .flatten();
        self.scroll_offsets.remove(context_menu::SCROLL_ID);
        self.scroll_transitions.remove(context_menu::SCROLL_ID);
        self.invalidate_paste_targets();
        self.context_menu = Some(ContextMenuState {
            target,
            position,
            entries,
            palette,
            selected,
            pressed: None,
            modal,
        });
        self.pointer = Some(position);
        self.tooltip_hover_since = None;
        None
    }

    fn activate_context_menu(&mut self, index: usize, now: Instant) -> Vec<UiAction> {
        let Some(menu) = self.context_menu.take() else {
            return Vec::new();
        };
        self.pressed = None;
        self.set_hovered(None, now);
        if !menu.enabled(index) || !menu.valid_in(&self.frame) {
            return Vec::new();
        }
        let ContextMenuEntry::Item(item) = &menu.entries[index] else {
            return Vec::new();
        };
        if matches!(menu.target, Target::Custom { .. }) {
            return vec![UiAction::Clicked {
                id: item.action_id.clone(),
            }];
        }
        match &menu.target {
            Target::Input { id } if self.focused.as_deref() == Some(id) => {
                if item.action_id == "paste" {
                    return vec![UiAction::PasteRequested { id: id.clone() }];
                }
                let Some(input) = self.inputs.get_mut(id) else {
                    return Vec::new();
                };
                if item.action_id == "select_all" {
                    input.set_selection(input.value.chars().count(), Some(0));
                    input.keep_cursor_visible = true;
                    return Vec::new();
                }
                if input.password {
                    return Vec::new();
                }
                let Some((start, end)) = input.selected_range() else {
                    return Vec::new();
                };
                let mut actions = vec![UiAction::CopyRequested {
                    text: char_range(&input.value, start, end).to_owned(),
                }];
                if item.action_id == "cut" && input.delete_selection() {
                    actions.push(UiAction::InputChanged {
                        id: id.clone(),
                        value: input.value.clone(),
                    });
                }
                actions
            }
            Target::Editor { id, document_id } if self.focused.as_deref() == Some(id) => {
                if self
                    .code_editors
                    .get(id)
                    .and_then(|editor| editor.editor.controller.snapshot().ok())
                    .is_none_or(|snapshot| snapshot.document_id() != *document_id)
                {
                    return Vec::new();
                }
                if item.action_id == "paste" {
                    return if self
                        .code_editors
                        .get(id)
                        .is_some_and(|editor| !editor.editor.read_only)
                    {
                        vec![UiAction::PasteRequested { id: id.clone() }]
                    } else {
                        Vec::new()
                    };
                }
                let key = match item.action_id.as_str() {
                    "copy" => Key::C,
                    "cut" => Key::X,
                    "select_all" => Key::A,
                    _ => {
                        return vec![UiAction::Clicked {
                            id: item.action_id.clone(),
                        }]
                    }
                };
                self.handle_code_event(
                    &UiEvent::Key {
                        key,
                        shift: false,
                        ctrl: true,
                    },
                    now,
                )
                .unwrap_or_default()
            }
            Target::Text { block, source } => {
                let Some(selection) = self
                    .static_selection
                    .as_mut()
                    .filter(|selection| selection.block == *block && selection.source == *source)
                else {
                    return Vec::new();
                };
                if item.action_id == "select_all" {
                    selection.anchor = 0;
                    selection.cursor = source.chars().count();
                    let next_focus = None;
                    Self::replace_focused(&mut self.focused, &mut self.paste_epoch, next_focus);
                    return Vec::new();
                }
                self.selected_text()
                    .map(|text| vec![UiAction::CopyRequested { text: text.into() }])
                    .unwrap_or_default()
            }
            _ => Vec::new(),
        }
    }

    pub(super) fn handle_context_menu_event(
        &mut self,
        event: &UiEvent,
        now: Instant,
    ) -> Option<Vec<UiAction>> {
        match event {
            UiEvent::ContextMenu { position } => {
                // A secondary click outside an open popover closes it, as a
                // primary one does: it lands on the popover's backdrop, not on
                // a control of the popover that reports the same ID.
                let outside = |region: &HitRegion| {
                    region.kind == HitKind::Button
                        && self
                            .frame
                            .popover_focus
                            .iter()
                            .any(|scope| scope.dismiss_id == region.id)
                        && !self.frame.regions.iter().any(|surface| {
                            surface.rect.contains(*position)
                                && surface.id.strip_prefix("popover_surface:")
                                    == Some(region.id.as_str())
                        })
                };
                if let Some(backdrop) = self.frame.press_region(*position).filter(|r| outside(r)) {
                    let id = self.dispatch_id(&backdrop.id);
                    self.close_context_menu(now);
                    return Some(vec![UiAction::Clicked { id }]);
                }
                return Some(self.open_text_context_menu(*position, false, now).into_iter().collect());
            }
            UiEvent::ContextMenuKeyboard => {
                let point = self
                    .focused
                    .as_deref()
                    .and_then(|id| {
                        self.frame
                            .regions
                            .iter()
                            .rev()
                            .find(|region| region.id == id)
                    })
                    .map(|region| {
                        Point::new(region.rect.x + 8.0, region.rect.y + region.rect.height)
                    })
                    .or_else(|| {
                        self.static_selection
                            .as_ref()
                            .and_then(|selection| self.frame.text_blocks.get(selection.block))
                            .and_then(|block| block.runs.iter().find_map(|run| run.visible))
                            .map(|rect| Point::new(rect.x + 1.0, rect.y + rect.height))
                    });
                if let Some(mut point) = point {
                    point.y -= 1.0;
                    return Some(self.open_text_context_menu(point, true, now).into_iter().collect());
                }
                return Some(Vec::new());
            }
            UiEvent::PasteTo { id, text } => {
                if self.context_menu.is_some()
                    || self.focused.as_deref() != Some(id)
                    || !self.frame.regions.iter().any(|region| {
                        region.id == *id
                            && matches!(
                                region.kind,
                                HitKind::TextInput | HitKind::TextArea | HitKind::CodeEditor
                            )
                    })
                {
                    return Some(Vec::new());
                }
                return Some(self.handle_event_at(UiEvent::Paste(text.clone()), now));
            }
            UiEvent::PointerUp(_) if self.context_menu_swallow_release => {
                self.context_menu_swallow_release = false;
                return Some(Vec::new());
            }
            _ => {}
        }
        if self
            .context_menu
            .as_ref()
            .is_some_and(|menu| !menu.valid_in(&self.frame))
        {
            self.close_context_menu(now);
        }
        self.context_menu.as_ref()?;
        match event {
            UiEvent::Paste(text) => {
                let menu = self.context_menu.as_ref().unwrap();
                if matches!(menu.target, Target::Custom { .. }) {
                    let index = menu.entries.iter().position(|entry| matches!(entry,
                        ContextMenuEntry::Item(item) if item.enabled && item.shortcut.as_deref().is_some_and(|shortcut|
                            shortcut_matches(shortcut, Key::V, false, true)
                                || shortcut_matches(shortcut, Key::V, true, true))));
                    if let Some(index) = index {
                        // Custom owners receive their existing scoped action.
                        // They decide whether/how to request clipboard text;
                        // it must never leak into a previously focused input.
                        return Some(self.activate_context_menu(index, now));
                    }
                } else {
                    let target = match &menu.target {
                        Target::Input { id } => Some(id.clone()),
                        Target::Editor { id, document_id } => {
                            let same_document = self
                                .code_editors
                                .get(id)
                                .filter(|editor| !editor.editor.read_only)
                                .and_then(|editor| editor.editor.controller.snapshot().ok())
                                .is_some_and(|snapshot| snapshot.document_id() == *document_id);
                            if !same_document {
                                self.close_context_menu(now);
                                return Some(Vec::new());
                            }
                            Some(id.clone())
                        }
                        _ => None,
                    };
                    let enabled = menu.entries.iter().any(|entry| {
                        matches!(entry,
                        ContextMenuEntry::Item(item) if item.enabled && item.action_id == "paste")
                    });
                    if let Some(id) =
                        target.filter(|id| enabled && self.focused.as_deref() == Some(id))
                    {
                        self.close_context_menu(now);
                        // Native paste already contains the clipboard bytes.
                        // Applying PasteTo revalidates the visible owner and
                        // avoids requesting (and inserting) the payload twice.
                        return Some(self.handle_event_at(
                            UiEvent::PasteTo {
                                id,
                                text: text.clone(),
                            },
                            now,
                        ));
                    }
                }
            }
            UiEvent::PointerMove(point) => {
                self.pointer = Some(*point);
                let target = self
                    .frame
                    .hit_test(*point)
                    .filter(|id| id.starts_with("__context_menu:item:"))
                    .map(str::to_owned);
                // A menu owns one active row. Pointer movement replaces the
                // keyboard selection instead of painting a second highlight.
                self.context_menu.as_mut().unwrap().selected =
                    target.as_deref().and_then(row_index);
                self.set_hovered(target, now);
                if let Some(drag) = self.scroll_drag.as_ref() {
                    if drag.id == context_menu::SCROLL_ID {
                        if let Some(geometry) = self.frame.scrollbars.get(&drag.id) {
                            self.scroll_offsets
                                .insert(drag.id.clone(), geometry.offset_at(point.y, drag.grab_y));
                        }
                    }
                }
            }
            UiEvent::PointerLeave => {
                if self.hovered.is_some() {
                    self.context_menu.as_mut().unwrap().selected = None;
                }
                self.set_hovered(None, now);
                self.scroll_drag = None;
            }
            UiEvent::PointerDown(point)
            | UiEvent::PointerDownModified {
                position: point, ..
            }
            | UiEvent::PointerDownEditor {
                position: point, ..
            } => {
                let region = self.frame.hit_region(*point).cloned();
                if let Some(region) = region.as_ref().filter(|region| {
                    region.kind == HitKind::Scrollbar && region.id == context_menu::SCROLL_ID
                }) {
                    if let Some(geometry) = self.frame.scrollbars.get(&region.id) {
                        self.scroll_drag = Some(ScrollDrag {
                            id: region.id.clone(),
                            grab_y: (point.y - geometry.thumb.y).clamp(0.0, geometry.thumb.height),
                        });
                    }
                } else if let Some(index) = region.as_ref().and_then(|region| row_index(&region.id))
                {
                    if self
                        .context_menu
                        .as_ref()
                        .is_some_and(|menu| menu.enabled(index))
                    {
                        let menu = self.context_menu.as_mut().unwrap();
                        menu.pressed = Some(index);
                        menu.selected = None;
                    }
                } else if region
                    .as_ref()
                    .is_none_or(|region| region.id != "__context_menu:surface")
                {
                    self.close_context_menu(now);
                    self.context_menu_swallow_release = true;
                }
            }
            UiEvent::PointerUp(point) => {
                self.scroll_drag = None;
                let selected = self.frame.hit_test(*point).and_then(row_index);
                let pressed = self
                    .context_menu
                    .as_mut()
                    .and_then(|menu| menu.pressed.take());
                if let Some(selected) = selected.filter(|selected| Some(*selected) == pressed) {
                    return Some(self.activate_context_menu(selected, now));
                }
            }
            UiEvent::Scroll { position, delta_y }
            | UiEvent::ScrollDirect { position, delta_y }
            | UiEvent::Scroll2D {
                position, delta_y, ..
            } if self.frame.regions.iter().any(|region| {
                region.id == "__context_menu:surface" && region.rect.contains(*position)
            }) =>
            {
                self.scroll_regular_regions(*position, *delta_y, false, now);
            }
            // Outside the menu it closes, as a press there does, and the
            // wheel goes on to what is under it.
            UiEvent::Scroll { .. } | UiEvent::ScrollDirect { .. } | UiEvent::Scroll2D { .. }
                if scroll_at(event).is_some() =>
            {
                self.close_context_menu(now);
                return None;
            }
            UiEvent::Key {
                key: Key::Escape | Key::Tab,
                ..
            } => self.close_context_menu(now),
            UiEvent::Key {
                key: Key::ArrowDown | Key::ArrowUp | Key::Home | Key::End,
                ..
            } => {
                let UiEvent::Key { key, .. } = event else {
                    unreachable!()
                };
                self.set_hovered(None, now);
                let menu = self.context_menu.as_mut().unwrap();
                let enabled: Vec<_> = (0..menu.entries.len())
                    .filter(|index| menu.enabled(*index))
                    .collect();
                if !enabled.is_empty() {
                    let current = menu
                        .selected
                        .and_then(|selected| enabled.iter().position(|index| *index == selected));
                    let index = match key {
                        Key::Home => 0,
                        Key::End => enabled.len() - 1,
                        Key::ArrowUp => current
                            .map(|index| (index + enabled.len() - 1) % enabled.len())
                            .unwrap_or(enabled.len() - 1),
                        _ => current
                            .map(|index| (index + 1) % enabled.len())
                            .unwrap_or(0),
                    };
                    menu.selected = Some(enabled[index]);
                    let id = context_menu::row_id(enabled[index]);
                    let tall_row_top = self
                        .frame
                        .scroll_focus_targets
                        .get(&id)
                        .filter(|target| target.rect.height > target.viewport.height)
                        .map(|target| {
                            (
                                target.scroll_id.clone(),
                                target.layout_offset + target.rect.y - target.viewport.y,
                            )
                        });
                    self.reveal_scroll_focus(&id, 0.0, false);
                    if let Some((scroll_id, offset)) = tall_row_top {
                        // A wrapped action taller than this low menu cannot
                        // fit at once. Start with its name, rather than only
                        // its final lines; wheel/rail can reveal the rest.
                        let max_offset = self.scroll_max(&scroll_id);
                        self.scroll_offsets
                            .insert(scroll_id, offset.clamp(0.0, max_offset));
                    }
                }
            }
            UiEvent::Key {
                key: Key::Enter | Key::Space,
                ..
            } => {
                if let Some(index) = self.context_menu.as_ref().and_then(|menu| menu.selected) {
                    return Some(self.activate_context_menu(index, now));
                }
            }
            UiEvent::Key { key, ctrl, shift } => {
                let index = self.context_menu.as_ref().and_then(|menu| menu.entries.iter().position(|entry| matches!(entry,
                    ContextMenuEntry::Item(item) if item.enabled && item.shortcut.as_deref().is_some_and(|shortcut| shortcut_matches(shortcut, *key, *shift, *ctrl)))));
                if let Some(index) = index {
                    return Some(self.activate_context_menu(index, now));
                }
            }
            _ => {}
        }
        Some(Vec::new())
    }
}

fn row_index(id: &str) -> Option<usize> {
    id.strip_prefix("__context_menu:item:")?.parse().ok()
}

fn shortcut_matches(shortcut: &str, key: Key, shift: bool, ctrl: bool) -> bool {
    let mut shortcut_ctrl = false;
    let mut shortcut_shift = false;
    let mut shortcut_key = None;
    for part in shortcut.split('+').map(str::trim) {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" | "cmd" | "command" | "⌘" => shortcut_ctrl = true,
            "shift" => shortcut_shift = true,
            "alt" | "option" | "" => return false,
            _ if shortcut_key.is_some() => return false,
            _ => shortcut_key = Some(part),
        }
    }
    shortcut_ctrl == ctrl
        && shortcut_shift == shift
        && shortcut_key.is_some_and(|part| part.eq_ignore_ascii_case(&format!("{key:?}")))
}

fn text_entries(copy: bool, cut: bool, paste: bool, select_all: bool) -> Vec<ContextMenuEntry> {
    vec![
        ContextMenuEntry::Item(
            ContextMenuItem::new("cut", crate::labels().cut)
                .shortcut("Ctrl+X")
                .enabled(cut),
        ),
        ContextMenuEntry::Item(
            ContextMenuItem::new("copy", crate::labels().copy)
                .shortcut("Ctrl+C")
                .enabled(copy),
        ),
        ContextMenuEntry::Item(
            ContextMenuItem::new("paste", crate::labels().paste)
                .shortcut("Ctrl+V")
                .enabled(paste),
        ),
        ContextMenuEntry::Separator,
        ContextMenuEntry::Item(
            ContextMenuItem::new("select_all", crate::labels().select_all)
                .shortcut("Ctrl+A")
                .enabled(select_all),
        ),
    ]
}

#[cfg(test)]
#[path = "context_menu_tests.rs"]
mod tests;
