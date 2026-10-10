//! Boxed preference lists and action rows. The host supplies actions and data;
//! these components follow the same palette as the rest of the native UI.

use crate::{Component, ComponentPalette, CornerRadii, Element, FontFamily, Icon};

fn group_radius() -> f32 {
    crate::tokens().radius.card
}

pub struct PreferencesGroup {
    title: String,
    description: Option<String>,
    rows: Vec<Element>,
    palette: ComponentPalette,
}
impl PreferencesGroup {
    pub fn new(title: impl Into<String>, rows: Vec<Element>, palette: ComponentPalette) -> Self {
        Self {
            title: title.into(),
            description: None,
            rows,
            palette,
        }
    }
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}
impl Component for PreferencesGroup {
    fn render(self) -> Element {
        let mut heading = vec![
            Element::text(self.title)
                .font_family(FontFamily::Sans)
                .font_size(18.0)
                .foreground(self.palette.text),
        ];
        if let Some(description) = self.description {
            heading.push(
                Element::text(description)
                    .font_family(FontFamily::Sans)
                    .font_size(11.0)
                    .foreground(self.palette.muted),
            );
        }
        let last_row = self.rows.len().saturating_sub(1);
        let mut rows = Vec::new();
        for (index, row) in self.rows.into_iter().enumerate() {
            if !rows.is_empty() {
                rows.push(
                    Element::spacer()
                        .size(None, Some(1.0))
                        .background(self.palette.divider()),
                );
            }
            // The group's child clip is rectangular. Its boundary rows must
            // paint the same outer corners during hover, press and focus.
            rows.push(row.corner_radii(CornerRadii {
                top_left: if index == 0 { group_radius() } else { 0.0 },
                top_right: if index == 0 { group_radius() } else { 0.0 },
                bottom_right: if index == last_row { group_radius() } else { 0.0 },
                bottom_left: if index == last_row { group_radius() } else { 0.0 },
            }));
        }
        Element::column(vec![
            Element::column(heading).gap(4.0),
            Element::panel(Element::column(rows))
                .background(self.palette.surface)
                .border(self.palette.border, 1.0)
                .radius(group_radius())
                .clip_children(),
        ])
        .gap(6.0)
    }
}

pub struct ActionRow {
    title: String,
    subtitle: Option<String>,
    icon: Option<Icon>,
    suffix: Option<Element>,
    action: Option<String>,
    palette: ComponentPalette,
}
impl ActionRow {
    pub fn new(title: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            icon: None,
            suffix: None,
            action: None,
            palette,
        }
    }
    pub fn subtitle(mut self, text: impl Into<String>) -> Self {
        self.subtitle = Some(text.into());
        self
    }
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }
    pub fn suffix(mut self, element: Element) -> Self {
        self.suffix = Some(element);
        self
    }
    pub fn action(mut self, id: impl Into<String>) -> Self {
        self.action = Some(id.into());
        self
    }
}
impl Component for ActionRow {
    fn render(self) -> Element {
        let minimum_height = if self.subtitle.is_some() { 54.0 } else { 36.0 };
        let mut copy = vec![
            Element::truncated_text(self.title)
                .font_family(FontFamily::Sans)
                .font_size(12.0)
                .foreground(self.palette.text),
        ];
        if let Some(subtitle) = self.subtitle {
            copy.push(
                Element::text(subtitle)
                    .font_family(FontFamily::Sans)
                    .font_size(11.0)
                    .foreground(self.palette.muted),
            );
        }
        let center = |child: Element| {
            Element::column(vec![
                Element::spacer().flex(1.0),
                child,
                Element::spacer().flex(1.0),
            ])
        };
        let mut children = Vec::new();
        if let Some(icon) = self.icon {
            children.push(
                center(Element::icon(icon).foreground(self.palette.text)).size(Some(20.0), None),
            );
        }
        children.push(center(Element::column(copy).gap(4.0)).flex(1.0));
        if let Some(suffix) = self.suffix {
            children.push(center(suffix));
        }
        let content = Element::row(children)
            .gap(10.0)
            .padding(10.0)
            .min_size(0.0, minimum_height);
        if let Some(id) = self.action {
            self.palette
                .interactive_surface(Element::clickable(id, content))
        } else {
            content
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Frame, Point, Rect, Size, UiAction, UiEvent, UiTree};

    fn same_bounds(rect: Rect, bounds: Rect) -> bool {
        (rect.x - bounds.x).abs() < 0.001
            && (rect.y - bounds.y).abs() < 0.001
            && (rect.width - bounds.width).abs() < 0.001
            && (rect.height - bounds.height).abs() < 0.001
    }

    fn painted_corners(frame: &Frame, bounds: Rect, color: crate::Color) -> Option<CornerRadii> {
        frame.commands.iter().find_map(|command| match command {
            DrawCommand::FillCornerRect {
                rect,
                color: ink,
                radii,
            } if same_bounds(*rect, bounds) && *ink == color => Some(*radii),
            DrawCommand::FillRect {
                rect,
                color: ink,
                radius,
            } if same_bounds(*rect, bounds) && *ink == color => Some(CornerRadii::same(*radius)),
            _ => None,
        })
    }

    #[test]
    fn grouped_action_rows_keep_outer_corners_and_full_hit_targets_in_every_state() {
        let palette = ComponentPalette::neutral_dark();
        for width in [220.0, 360.0, 900.0] {
            for count in [1, 2, 3] {
                let mut tree = UiTree::new(
                    PreferencesGroup::new(
                        "Cuenta",
                        (0..count)
                            .map(|index| {
                                ActionRow::new(format!("Acción {index}"), palette)
                                    .action(format!("action:{index}"))
                                    .render()
                            })
                            .collect(),
                        palette,
                    )
                    .render(),
                );
                tree.set_reduced_motion(true);
                let viewport = Size::new(width, 500.0);
                let initial = tree.layout(viewport);
                for index in 0..count {
                    let id = format!("action:{index}");
                    let bounds = initial
                        .regions
                        .iter()
                        .find(|region| region.id == id)
                        .unwrap()
                        .rect;
                    let expected = CornerRadii {
                        top_left: if index == 0 { group_radius() } else { 0.0 },
                        top_right: if index == 0 { group_radius() } else { 0.0 },
                        bottom_right: if index == count - 1 {
                            group_radius()
                        } else {
                            0.0
                        },
                        bottom_left: if index == count - 1 {
                            group_radius()
                        } else {
                            0.0
                        },
                    };
                    assert_eq!(bounds.width, width);
                    let point = Point::new(bounds.x + 1.0, bounds.y + 1.0);
                    assert_eq!(initial.hit_test(point), Some(id.as_str()));
                    tree.handle_event(UiEvent::PointerMove(point));
                    let hovered = tree.layout(viewport);
                    assert_eq!(
                        painted_corners(&hovered, bounds, palette.hover),
                        Some(expected)
                    );
                    tree.handle_event(UiEvent::PointerDown(point));
                    let pressed = tree.layout(viewport);
                    assert_eq!(
                        painted_corners(&pressed, bounds, palette.selected),
                        Some(expected)
                    );
                    assert_eq!(
                        tree.handle_event(UiEvent::PointerUp(point)),
                        vec![UiAction::Clicked { id: id.clone() }]
                    );
                    assert!(tree.focus(&id));
                    let focused = tree.layout(viewport);
                    // Hit rectangles are intersected with the group clip;
                    // fractional heading metrics can round their height by
                    // a few ulps relative to the original painted rectangle.
                    assert!(focused.commands.iter().any(|command| match command {
                        DrawCommand::StrokeCornerRect {
                            rect, width, radii, ..
                        } => same_bounds(*rect, bounds) && *width == 2.0 && *radii == expected,
                        DrawCommand::StrokeRect {
                            rect,
                            width,
                            radius,
                            ..
                        } =>
                            same_bounds(*rect, bounds)
                                && *width == 2.0
                                && CornerRadii::same(*radius) == expected,
                        _ => false,
                    }));
                }
            }
        }
    }

    #[test]
    fn account_group_rounds_manage_hover_without_changing_disconnect_suffix_action() {
        let palette = ComponentPalette::neutral_dark();
        for width in [220.0, 360.0, 900.0] {
            let mut tree = UiTree::new(
                PreferencesGroup::new(
                    "Cuenta",
                    vec![
                        ActionRow::new("Administrar cuenta", palette)
                            .subtitle("Abre la página del proveedor en tu navegador.")
                            .action("provider_account:codex")
                            .suffix(Element::icon(Icon::ArrowRight))
                            .render(),
                        ActionRow::new("Desconectar", palette)
                            .subtitle("Quita esta conexión de OpenIDE.")
                            .suffix(
                                Element::button("provider_disconnect:codex", "Desconectar")
                                    .size(Some(100.0), Some(32.0)),
                            )
                            .render(),
                    ],
                    palette,
                )
                .render(),
            );
            tree.set_reduced_motion(true);
            let viewport = Size::new(width, 500.0);
            let initial = tree.layout(viewport);
            let account = initial
                .regions
                .iter()
                .find(|region| region.id == "provider_account:codex")
                .unwrap()
                .rect;
            tree.handle_event(UiEvent::PointerMove(Point::new(
                account.x + 1.0,
                account.y + 1.0,
            )));
            let hovered = tree.layout(viewport);
            assert_eq!(
                painted_corners(&hovered, account, palette.hover),
                Some(CornerRadii::top(group_radius()))
            );
            let disconnect = hovered
                .regions
                .iter()
                .find(|region| region.id == "provider_disconnect:codex")
                .unwrap()
                .rect;
            assert!(disconnect.x >= 0.0 && disconnect.x + disconnect.width <= width);
            let point = Point::new(
                disconnect.x + disconnect.width / 2.0,
                disconnect.y + disconnect.height / 2.0,
            );
            tree.handle_event(UiEvent::PointerDown(point));
            assert_eq!(
                tree.handle_event(UiEvent::PointerUp(point)),
                vec![UiAction::Clicked {
                    id: "provider_disconnect:codex".into()
                }]
            );
        }
    }

    #[test]
    fn rows_are_separated_by_a_rule_that_reads_on_the_group_surface() {
        // How far apart two colors are, as a viewer would judge a thin line.
        let distance = |a: crate::Color, b: crate::Color| {
            (i32::from(a.r) - i32::from(b.r)).abs()
                + (i32::from(a.g) - i32::from(b.g)).abs()
                + (i32::from(a.b) - i32::from(b.b)).abs()
        };
        for palette in [ComponentPalette::neutral_dark(), crate::Theme::DARK.palette()] {
            let divider = palette.divider();
            assert!(
                distance(divider, palette.surface) >= distance(palette.border, palette.surface) + 60,
                "the rule must stand out more than the border does"
            );
            let group = PreferencesGroup::new(
                "Plugins",
                vec![
                    ActionRow::new("Uno", palette).render(),
                    ActionRow::new("Dos", palette).render(),
                    ActionRow::new("Tres", palette).render(),
                ],
                palette,
            )
            .render();
            let frame = UiTree::new(group).layout(Size::new(400.0, 300.0));
            let rules: Vec<Rect> = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::FillRect { rect, color, .. } if *color == divider && rect.height == 1.0 => {
                        Some(*rect)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(rules.len(), 2, "one rule between each pair of rows");
            assert!(rules.iter().all(|rule| rule.width > 390.0), "rules span the group");
        }
    }
}
