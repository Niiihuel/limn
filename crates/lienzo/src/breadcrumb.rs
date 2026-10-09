//! Compact native resource hierarchy, inspired by Rust/UI's composition.
//! Reference: https://rust-ui.com/docs/components/breadcrumb
//! Labels and actions are supplied by the host; no navigation is inferred.
use crate::{
    Component, ComponentPalette, DropdownMenu, DropdownMenuEntry, DropdownMenuItem, Element,
    FontFamily, Icon, Tooltip,
};
use unicode_width::UnicodeWidthStr;

const FONT_SIZE: f32 = 11.5;
const GAP: f32 = 4.0;
// Rust/UI's separator is a 14 px chevron. At 10 px the native 24 px Solar
// viewbox reduced its visible stroke to less than one physical pixel at 1x.
// Reserve the entire icon width so the flex row cannot squeeze it into text.
const SEPARATOR_WIDTH: f32 = 14.0;
const SEPARATOR_SPACE: f32 = SEPARATOR_WIDTH + 2.0 * GAP;

#[derive(Clone, Debug)]
pub struct BreadcrumbItem {
    label: String,
    action: Option<String>,
    icon: Option<Icon>,
    tooltip: Option<String>,
}

impl BreadcrumbItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            action: None,
            icon: None,
            tooltip: None,
        }
    }
    pub fn action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
}

pub struct Breadcrumb {
    id: String,
    items: Vec<BreadcrumbItem>,
    palette: ComponentPalette,
    width: f32,
    height: f32,
    overflow_open: bool,
}

impl Breadcrumb {
    pub fn new(
        id: impl Into<String>,
        items: Vec<BreadcrumbItem>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            id: id.into(),
            items,
            palette,
            width: 320.0,
            height: 28.0,
            overflow_open: false,
        }
    }
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = width.max(0.0);
        }
        self
    }
    pub fn height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.height = height.max(20.0);
        }
        self
    }
    /// Controlled menu for collapsed ancestors. Emits `{id}:overflow_toggle`
    /// and `{id}:overflow_dismiss`; choices dispatch their original actions.
    pub fn overflow_open(mut self, open: bool) -> Self {
        self.overflow_open = open;
        self
    }
}

impl Component for Breadcrumb {
    fn render(self) -> Element {
        if self.items.is_empty() {
            return Element::spacer().size(Some(self.width), Some(self.height));
        }
        let last = self.items.len() - 1;
        // This estimate chooses which ancestors to collapse, never their paint
        // widths. The native measurer lays out every retained ancestor at its
        // intrinsic width; only the final resource receives remaining space.
        let estimated = |index: usize| {
            let item = &self.items[index];
            UnicodeWidthStr::width(item.label.as_str()) as f32 * FONT_SIZE * 0.75
                + 4.0
                + if item.icon.is_some() { 18.0 } else { 0.0 }
        };
        let keep_root = self.width >= 84.0 && last > 0 && self.items[0].action.is_some();
        let mut visible: Vec<usize> = if self.width < 84.0 {
            vec![last]
        } else {
            (0..self.items.len()).collect()
        };
        let minimum = if keep_root { 2 } else { 1 };
        while visible.len() > minimum
            && visible.iter().copied().map(estimated).sum::<f32>()
                + SEPARATOR_SPACE * visible.len().saturating_sub(1) as f32
                > self.width
        {
            visible.remove(usize::from(keep_root));
        }
        let root_collapsed = keep_root && estimated(0) > (self.width * 0.4).max(24.0);
        if root_collapsed {
            visible.retain(|index| *index != 0);
        }
        let has_hidden = visible.len() < self.items.len();
        let current_reserve = estimated(last).min(96.0).min(self.width);
        let show_overflow = has_hidden && self.width >= 84.0;
        while show_overflow
            && visible.len() > 1
            && visible
                .iter()
                .copied()
                .filter(|index| *index != last)
                .map(estimated)
                .sum::<f32>()
                + SEPARATOR_SPACE * visible.len() as f32
                + current_reserve
                + 18.0
                > self.width
        {
            let position = usize::from(visible.len() > 2 && visible[0] == 0 && keep_root);
            visible.remove(position);
        }
        let hidden = (0..last)
            .filter(|index| !visible.contains(index))
            .collect::<Vec<_>>();
        let full_path = self
            .items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>()
            .join(" › ");
        let overflow_position = usize::from(visible.first() == Some(&0));
        let mut segments: Vec<Option<usize>> = visible.into_iter().map(Some).collect();
        if show_overflow {
            segments.insert(overflow_position, None);
        }
        let mut children = Vec::new();
        for (position, index) in segments.into_iter().enumerate() {
            if position > 0 {
                children.push(
                    Element::icon(Icon::ChevronRight)
                        .foreground(self.palette.muted)
                        .size(Some(SEPARATOR_WIDTH), Some(self.height)),
                );
            }
            let Some(index) = index else {
                let navigable = hidden
                    .iter()
                    .any(|index| self.items[*index].action.is_some());
                let visual = Element::text("…")
                    .font_family(FontFamily::Sans)
                    .font_size(FONT_SIZE)
                    .foreground(self.palette.muted);
                let anchor = if navigable {
                    self.palette.bar_item_surface(Element::clickable(
                        format!("{}:overflow_toggle", self.id),
                        visual,
                    ))
                } else {
                    Element::panel(visual)
                };
                let anchor = Tooltip::new(
                    anchor.padding(2.0).size(Some(18.0), Some(self.height)),
                    format!("{}:overflow_toggle", self.id),
                    "Ruta completa",
                    self.palette,
                )
                .description(&full_path)
                .max_width(320.0)
                .render();
                if !navigable {
                    children.push(anchor);
                    continue;
                }
                let entries = hidden
                    .iter()
                    .map(|index| {
                        let item = &self.items[*index];
                        let label = item.tooltip.as_deref().unwrap_or(&item.label);
                        let mut entry =
                            DropdownMenuItem::new(item.action.as_deref().unwrap_or(""), label);
                        if let Some(icon) = item.icon {
                            entry = entry.icon(icon);
                        }
                        if item.action.is_none() {
                            entry = entry.disabled();
                        }
                        DropdownMenuEntry::Item(entry)
                    })
                    .collect::<Vec<_>>();
                children.push(
                    DropdownMenu::new(
                        anchor,
                        entries,
                        self.overflow_open,
                        format!("{}:overflow_dismiss", self.id),
                        self.palette,
                    )
                    .width(self.width.clamp(180.0, 320.0))
                    .render(),
                );
                continue;
            };
            let item = &self.items[index];
            let current = index == last;
            let mut content = Vec::new();
            if let Some(icon) = item.icon.filter(|_| self.width >= 40.0) {
                content.push(
                    Element::icon(icon)
                        .foreground(if current {
                            self.palette.text
                        } else {
                            self.palette.muted
                        })
                        .size(Some(14.0), Some(self.height - 4.0)),
                );
            }
            content.push(
                Element::faded_text(&item.label, 16.0)
                    .font_family(FontFamily::Sans)
                    .font_size(FONT_SIZE)
                    .foreground(if current {
                        self.palette.text
                    } else {
                        self.palette.muted
                    })
                    .size(None, Some(self.height - 4.0))
                    .min_size(0.0, 0.0)
                    .flex(1.0),
            );
            let visual = Element::row(content).gap(4.0).clip_children();
            let action = item.action.as_ref();
            let anchor_id = action
                .cloned()
                .unwrap_or_else(|| format!("{}:item:{index}", self.id));
            let anchor = if let Some(action) = action {
                self.palette
                    .bar_item_surface(Element::clickable(action, visual))
            } else {
                Element::panel(visual)
            }
            .padding(2.0)
            .size(None, Some(self.height));
            let help = item.tooltip.as_deref().unwrap_or(&full_path);
            let tooltip =
                Tooltip::new(anchor, anchor_id, &item.label, self.palette).max_width(320.0);
            let tooltip = if help != item.label || item.label.chars().count() > 40 {
                tooltip.description(help)
            } else {
                tooltip
            };
            let segment = tooltip.render();
            if current {
                // The slot may stretch, but its action must hug the resource.
                // Keeping the spare space outside the trigger avoids a header-
                // wide hover and a misleading clickable blank area. The
                // slot still clips long paths and their hit bounds on resize.
                children.push(
                    Element::row(vec![segment, Element::spacer().flex(1.0)])
                        .clip_children()
                        .flex(1.0),
                );
            } else {
                children.push(segment);
            }
        }
        Element::row(children)
            .gap(GAP)
            .size(Some(self.width), Some(self.height))
            .clip_children()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, HitKind, Key, Point, Size, UiAction, UiEvent, UiTree};

    #[test]
    fn current_resource_hover_hugs_label_and_leaves_blank_header_passive() {
        for width in [180.0, 520.0, 1200.0] {
            let mut tree = UiTree::new(
                Breadcrumb::new(
                    "path",
                    vec![
                        BreadcrumbItem::new("docs").action("folder"),
                        BreadcrumbItem::new("alerts.md")
                            .icon(Icon::File)
                            .action("file"),
                    ],
                    ComponentPalette::neutral_dark(),
                )
                .width(width)
                .render(),
            );
            tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 6.0);
            tree.set_reduced_motion(true);
            let size = Size::new(width, 28.0);
            let initial = tree.layout(size);
            let rect = initial
                .regions
                .iter()
                .find(|r| r.id == "file" && r.kind == HitKind::Button)
                .unwrap()
                .rect;
            assert!(
                rect.width <= 80.0,
                "file action must fit its icon, text and padding: {rect:?}"
            );
            assert_ne!(
                initial.hit_test(Point::new(width - 2.0, 14.0)),
                Some("file")
            );
            tree.handle_event(UiEvent::PointerMove(Point::new(rect.x + 4.0, 14.0)));
            let hover = tree.layout(size);
            let after = hover
                .regions
                .iter()
                .find(|r| r.id == "file" && r.kind == HitKind::Button)
                .unwrap()
                .rect;
            assert_eq!(rect, after, "hover must not change geometry");
            assert_eq!(rect.y, 0.0);
            assert_eq!(rect.height, size.height, "hover fills the breadcrumb cell");
            assert!(hover.commands.iter().any(
                |c| matches!(c, DrawCommand::FillRect { rect: hover_rect, color, radius }
                if *hover_rect == rect && *color == ComponentPalette::neutral_dark().hover && *radius == 0.0)
            ));
        }
    }

    #[test]
    fn short_segments_stay_intrinsic_when_the_header_grows() {
        let mut positions = Vec::new();
        for width in [240.0, 900.0] {
            let mut tree = UiTree::new(
                Breadcrumb::new(
                    "editor",
                    vec![
                        BreadcrumbItem::new("src"),
                        BreadcrumbItem::new("commands.rs").icon(Icon::File),
                    ],
                    ComponentPalette::neutral_dark(),
                )
                .width(width)
                .render(),
            );
            tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 8.0);
            let frame = tree.layout(Size::new(width, 28.0));
            let text_x = |label: &str| {
                frame
                    .commands
                    .iter()
                    .find_map(|command| match command {
                        DrawCommand::Text { text, position, .. } if text == label => {
                            Some(position.x)
                        }
                        _ => None,
                    })
                    .unwrap()
            };
            let separator = frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::Icon {
                        icon: Icon::ChevronRight,
                        rect,
                        ..
                    } => Some(*rect),
                    _ => None,
                })
                .unwrap();
            assert!(separator.x - (text_x("src") + 24.0) <= 8.0);
            assert!(
                text_x("commands.rs") < 100.0,
                "filename must follow its parent immediately"
            );
            positions.push((separator.x, text_x("commands.rs")));
        }
        assert_eq!(
            positions[0], positions[1],
            "extra header space belongs after the resource"
        );
    }

    #[test]
    fn collapsed_ancestors_preserve_file_priority_and_complete_path_help() {
        let mut tree = UiTree::new(
            Breadcrumb::new(
                "deep",
                vec![
                    BreadcrumbItem::new("project"),
                    BreadcrumbItem::new("an_extremely_long_ancestor_directory"),
                    BreadcrumbItem::new("src"),
                    BreadcrumbItem::new("main.rs").icon(Icon::File),
                ],
                ComponentPalette::neutral_dark(),
            )
            .width(240.0)
            .render(),
        );
        tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 8.0);
        let now = std::time::Instant::now();
        let size = Size::new(240.0, 160.0);
        let frame = tree.layout_at(size, now);
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, .. } if text == "main.rs"
        )));
        assert!(!frame.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, .. } if text == "an_extremely_long_ancestor_directory"
        )));
        let overflow = frame
            .regions
            .iter()
            .find(|region| region.id == "deep:overflow_toggle")
            .unwrap()
            .rect;
        assert!(
            !frame.regions.iter().any(|region| {
                region.id == "deep:overflow_toggle" && region.kind == HitKind::Button
            }),
            "passive ancestors expose path help without an inert menu trigger"
        );
        tree.handle_event_at(
            UiEvent::PointerMove(Point::new(overflow.x + 4.0, overflow.y + 10.0)),
            now,
        );
        let help = tree.layout_at(size, now + std::time::Duration::from_millis(700));
        let text = help
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");
        assert!(
            text.contains("an_extremely_long_ancestor_directory"),
            "hidden ancestors remain available in the tooltip: {text}"
        );
        assert!(help
            .regions
            .iter()
            .all(|region| region.rect.x >= 0.0 && region.rect.x + region.rect.width <= 240.01));
    }

    #[test]
    fn explicit_current_actions_work_and_chevrons_have_fixed_geometry() {
        for width in [120.0, 240.0, 480.0] {
            let mut tree = UiTree::new(
                Breadcrumb::new(
                    "file",
                    vec![
                        BreadcrumbItem::new("Cambios").action("back"),
                        BreadcrumbItem::new("apps"),
                        BreadcrumbItem::new("agents-window"),
                        BreadcrumbItem::new("src"),
                        BreadcrumbItem::new("archivo_muy_largo.rs").action("copy_current"),
                    ],
                    ComponentPalette::neutral_dark(),
                )
                .width(width)
                .render(),
            );
            let frame = tree.layout(Size::new(width, 28.0));
            let root = frame
                .regions
                .iter()
                .find(|region| region.id == "back" && region.kind == HitKind::Button)
                .or_else(|| {
                    frame.regions.iter().find(|region| {
                        region.id == "file:overflow_toggle" && region.kind == HitKind::Button
                    })
                })
                .unwrap();
            assert_eq!(
                frame.hit_test(Point::new(root.rect.x + 3.0, 14.0)),
                Some(root.id.as_str())
            );
            assert!(frame
                .regions
                .iter()
                .any(|region| region.id == "copy_current"));
            for region in &frame.regions {
                assert!(
                    region.rect.x >= 0.0 && region.rect.x + region.rect.width <= width + 0.01,
                    "{region:?}"
                );
            }
            assert!(frame.commands.iter().any(|command| matches!(command, DrawCommand::Icon { icon: Icon::ChevronRight, rect, .. } if rect.width == 14.0)));
            assert!(!frame
                .commands
                .iter()
                .any(|command| matches!(command, DrawCommand::Text { text, .. } if text == "/")));
            assert!(tree.focus("copy_current"));
            assert_eq!(
                tree.handle_event(UiEvent::Key {
                    key: Key::Enter,
                    shift: false,
                    ctrl: false
                }),
                vec![UiAction::Clicked {
                    id: "copy_current".into()
                }]
            );
        }
    }

    #[test]
    fn collapsed_ancestor_menu_preserves_actions_keyboard_and_dismiss_scope() {
        let render = |open| {
            Breadcrumb::new(
                "path",
                vec![
                    BreadcrumbItem::new("a_very_long_workspace_name").action("open_root"),
                    BreadcrumbItem::new("nested")
                        .action("open_nested")
                        .tooltip("a_very_long_workspace_name/nested"),
                    BreadcrumbItem::new("main.rs").action("copy_file"),
                ],
                ComponentPalette::neutral_dark(),
            )
            .width(180.0)
            .overflow_open(open)
            .render()
        };
        let mut tree = UiTree::new(render(false));
        tree.set_reduced_motion(true);
        let size = Size::new(240.0, 320.0);
        let frame = tree.layout(size);
        let trigger = frame
            .regions
            .iter()
            .find(|region| region.id == "path:overflow_toggle" && region.kind == HitKind::Button)
            .unwrap()
            .rect;
        let point = Point::new(
            trigger.x + trigger.width / 2.0,
            trigger.y + trigger.height / 2.0,
        );
        tree.handle_event(UiEvent::PointerDown(point));
        assert!(tree
            .handle_event(UiEvent::PointerUp(point))
            .iter()
            .any(|a| matches!(a, UiAction::Clicked { id } if id == "path:overflow_toggle")));
        tree.set_root(render(true));
        let open = tree.layout(size);
        for action in ["open_root", "open_nested"] {
            let region = open
                .regions
                .iter()
                .find(|region| region.id == action && region.kind == HitKind::Button)
                .unwrap();
            assert!(region.rect.x >= 0.0 && region.rect.x + region.rect.width <= size.width + 0.5);
        }
        assert!(tree.focus("open_nested"));
        assert!(tree
            .handle_event(UiEvent::Key {
                key: Key::Enter,
                ctrl: false,
                shift: false
            })
            .iter()
            .any(|a| matches!(a, UiAction::Clicked { id } if id == "open_nested")));
        assert!(tree
            .handle_event(UiEvent::Key {
                key: Key::Escape,
                ctrl: false,
                shift: false
            })
            .iter()
            .any(|a| matches!(a, UiAction::Clicked { id } if id == "path:overflow_dismiss")));
    }

    #[test]
    fn extremely_narrow_and_empty_paths_keep_current_inside_available_width() {
        for width in [0.0, 32.0, 80.0] {
            let frame = UiTree::new(
                Breadcrumb::new(
                    "path",
                    vec![
                        BreadcrumbItem::new("root"),
                        BreadcrumbItem::new("当前 👩‍💻 文件.rs"),
                    ],
                    ComponentPalette::neutral_dark(),
                )
                .width(width)
                .render(),
            )
            .layout(Size::new(width, 28.0));
            assert!(!frame
                .regions
                .iter()
                .any(|region| region.kind == HitKind::Button));
            assert!(!frame.commands.iter().any(
                |command| matches!(command, DrawCommand::Text { text, .. } if text == "root")
            ));
        }
        let frame = UiTree::new(
            Breadcrumb::new("empty", Vec::new(), ComponentPalette::neutral_dark())
                .width(120.0)
                .render(),
        )
        .layout(Size::new(120.0, 28.0));
        assert!(frame.regions.is_empty());
    }

    #[test]
    fn rust_ui_example_has_visible_centered_passive_chevrons_between_links() {
        for height in [24.0, 28.0, 32.0, 40.0] {
            let palette = ComponentPalette::neutral_dark();
            let mut tree = UiTree::new(
                Breadcrumb::new(
                    "docs",
                    vec![
                        BreadcrumbItem::new("Home").action("home"),
                        BreadcrumbItem::new("Components").action("components"),
                        BreadcrumbItem::new("Breadcrumb"),
                    ],
                    palette,
                )
                .width(360.0)
                .height(height)
                .render(),
            );
            tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 6.0);
            let frame = tree.layout(Size::new(360.0, height));
            let mut clips: Vec<crate::Rect> = Vec::new();
            let mut separators = Vec::new();
            for command in &frame.commands {
                match command {
                    DrawCommand::PushClip(rect) => clips.push(*rect),
                    DrawCommand::PopClip => {
                        clips.pop();
                    }
                    DrawCommand::Icon {
                        icon: Icon::ChevronRight,
                        rect,
                        tint,
                    } => {
                        assert_eq!((rect.width, rect.height), (14.0, 14.0));
                        assert_eq!(*tint, palette.muted);
                        assert_eq!(tint.a, u8::MAX);
                        assert!((rect.y + rect.height / 2.0 - height / 2.0).abs() < 0.01);
                        assert!(clips.iter().all(|clip| rect.x >= clip.x
                            && rect.y >= clip.y
                            && rect.x + rect.width <= clip.x + clip.width
                            && rect.y + rect.height <= clip.y + clip.height));
                        assert_eq!(frame.hit_test(Point::new(rect.x + 7.0, rect.y + 7.0)), None);
                        separators.push(*rect);
                    }
                    _ => {}
                }
            }
            assert_eq!(separators.len(), 2);
            let action = |id: &str| {
                frame
                    .regions
                    .iter()
                    .find(|region| region.id == id && region.kind == HitKind::Button)
                    .unwrap()
                    .rect
            };
            let home = action("home");
            let components = action("components");
            assert!(home.x + home.width < separators[0].x);
            assert!(separators[0].x + separators[0].width < components.x);
            assert!(components.x + components.width < separators[1].x);
            let point = Point::new(components.x + components.width / 2.0, height / 2.0);
            tree.handle_event(UiEvent::PointerDown(point));
            assert_eq!(
                tree.handle_event(UiEvent::PointerUp(point)),
                vec![UiAction::Clicked {
                    id: "components".into()
                }]
            );
            assert!(tree.focus("home"));
            assert_eq!(
                tree.handle_event(UiEvent::Key {
                    key: Key::Enter,
                    shift: false,
                    ctrl: false
                }),
                vec![UiAction::Clicked { id: "home".into() }]
            );
        }
    }

    #[test]
    fn collapsed_path_keeps_a_whole_separator_before_the_current_resource() {
        for width in [84.0, 120.0, 180.0] {
            let frame = UiTree::new(
                Breadcrumb::new(
                    "path",
                    vec![
                        BreadcrumbItem::new("a_long_project_name").action("project"),
                        BreadcrumbItem::new("components").action("folder"),
                        BreadcrumbItem::new("breadcrumb.rs").action("current"),
                    ],
                    ComponentPalette::neutral_dark(),
                )
                .width(width)
                .render(),
            )
            .layout(Size::new(width, 28.0));
            let chevrons = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Icon {
                        icon: Icon::ChevronRight,
                        rect,
                        ..
                    } => Some(*rect),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(chevrons.len(), 1);
            let chevron = chevrons[0];
            assert_eq!((chevron.width, chevron.height), (14.0, 14.0));
            assert!(chevron.x >= 0.0 && chevron.x + chevron.width < width);
            let current = frame
                .regions
                .iter()
                .find(|r| r.id == "current" && r.kind == HitKind::Button)
                .unwrap()
                .rect;
            assert!(current.x > chevron.x + chevron.width);
            assert!(current.width > 0.0 && current.x + current.width <= width + 0.01);
        }
    }
}
