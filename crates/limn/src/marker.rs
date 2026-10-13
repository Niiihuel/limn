//! Conversation event rows. The caller supplies the observed state and label.

use crate::{
    Access, Color, Component, ComponentPalette, Element, FontFamily, Icon, Role, Spinner, TextShimmer,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MarkerVariant {
    #[default]
    Default,
    Separator,
    Border,
}

/// The highlight that sweeps across "thinking" text. Public so a host's own
/// activity rows can shimmer exactly like [`Marker`] and [`crate::Collapsible`].
pub fn thinking_highlight(palette: ComponentPalette) -> Color {
    let blend = |base: u8, highlight: u8| ((u16::from(base) + u16::from(highlight)) / 2) as u8;
    let base = palette.muted;
    let highlight = palette.text;
    Color::rgba(
        blend(base.r, highlight.r),
        blend(base.g, highlight.g),
        blend(base.b, highlight.b),
        blend(base.a, highlight.a),
    )
}

/// A passive event label, or an explicit action when `action` is supplied.
/// Loading is a presentation choice; this component does not infer progress.
pub struct Marker {
    id: String,
    label: String,
    palette: ComponentPalette,
    variant: MarkerVariant,
    icon: Option<Icon>,
    loading: bool,
    shimmer: bool,
    thinking: bool,
    truncate: bool,
    action: Option<String>,
    enabled: bool,
    ink: Option<Color>,
}

impl Marker {
    pub fn new(id: impl Into<String>, label: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            palette,
            variant: MarkerVariant::Default,
            icon: None,
            loading: false,
            shimmer: false,
            thinking: false,
            truncate: false,
            action: None,
            enabled: true,
            ink: None,
        }
    }

    pub fn variant(mut self, variant: MarkerVariant) -> Self {
        self.variant = variant;
        self
    }
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }
    pub fn shimmer(mut self, shimmer: bool) -> Self {
        self.shimmer = shimmer;
        self
    }
    /// Present model thinking with a static brain and a subtle text highlight.
    /// This takes precedence over `loading` without changing other markers'
    /// spinner presentation. Reduced motion is handled by `TextShimmer`.
    pub fn thinking(mut self, thinking: bool) -> Self {
        self.thinking = thinking;
        self
    }
    /// Keep a compact row on one line. The host can expose the full label in
    /// a tooltip; the default still wraps and retains selectable source text.
    pub fn truncate(mut self, truncate: bool) -> Self {
        self.truncate = truncate;
        self
    }
    pub fn action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }
    /// A disabled marker keeps the place and the look of its `action` and is
    /// passive: no hover, hit region or focus. Without an action a marker is
    /// passive already and this changes nothing.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable it; the same as `enabled(false)`.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
    /// A status color from the host, such as its failure ink, for the label,
    /// the icon and the spinner in place of the palette's muted.
    pub fn ink(mut self, ink: Color) -> Self {
        self.ink = Some(ink);
        self
    }
}

impl Component for Marker {
    fn render(mut self) -> Element {
        if let Some(ink) = self.ink {
            self.palette.muted = ink;
        }
        let said = Access::new(Role::Button).name(self.label.clone());
        let label = if self.thinking || self.loading || self.shimmer {
            let highlight = if self.thinking {
                thinking_highlight(self.palette)
            } else {
                self.palette.text
            };
            if self.truncate {
                Element::shimmer_truncated_text(format!("{}:text", self.id), self.label, highlight)
            } else {
                TextShimmer::new(format!("{}:text", self.id), self.label, self.palette)
                    .colors(self.palette.muted, highlight)
                    .render()
            }
        } else {
            if self.truncate {
                Element::truncated_text(self.label)
            } else {
                Element::text(self.label)
            }
        }
        .foreground(self.palette.muted)
        .font_family(FontFamily::Sans)
        .font_size(13.0);
        let centered = |child| {
            Element::column(vec![
                Element::spacer().flex(1.0),
                child,
                Element::spacer().flex(1.0),
            ])
        };
        let separator = self.variant == MarkerVariant::Separator;
        let label = if self.thinking || separator {
            centered(label)
        } else {
            label
        };
        let mut content = Vec::new();
        if self.thinking {
            content.push(
                Element::column(vec![
                    Element::spacer().flex(1.0),
                    Element::icon(Icon::Brain)
                        .foreground(self.palette.muted)
                        .size(Some(16.0), Some(16.0)),
                    Element::spacer().flex(1.0),
                ])
                .size(Some(16.0), None),
            );
        } else if self.loading {
            let spinner = Spinner::new(format!("{}:spinner", self.id), self.palette.muted).render();
            content.push(if separator {
                centered(spinner)
            } else {
                spinner
            });
        } else if let Some(icon) = self.icon {
            let icon = Element::icon(icon).foreground(self.palette.muted);
            content.push(if separator { centered(icon) } else { icon });
        }
        content.push(label.flex(1.0));
        let row = Element::row(content)
            .gap(8.0)
            .min_size(0.0, if self.thinking { 22.0 } else { 20.0 });
        let line = || {
            Element::spacer()
                .size(None, Some(1.0))
                .background(self.palette.border)
        };
        let body = match self.variant {
            MarkerVariant::Default => row,
            MarkerVariant::Border => Element::column(vec![row, line()]).gap(8.0),
            MarkerVariant::Separator => Element::row(vec![
                // The rules take the width the label leaves, so they hug it
                // at any width; a label too long for the row wraps instead.
                centered(line()).min_size(16.0, 0.0).flex(1.0),
                Element::panel(row).shrink_to_fit(),
                centered(line()).min_size(16.0, 0.0).flex(1.0),
            ])
            .gap(10.0),
        };
        match self.action {
            Some(action) if self.enabled => self
                .palette
                .interactive_surface(Element::clickable(action, body).accessible(said))
                .padding(6.0)
                .min_size(0.0, 32.0),
            // The row is muted as it is; only the control goes.
            Some(action) => Element::panel(body)
                .padding(6.0)
                .min_size(0.0, 32.0)
                .accessible(said.id(action).disabled(true)),
            None => body,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, HitKind, Key, Point, Size, UiAction, UiEvent, UiTree};
    use web_time::{Duration, Instant};

    #[test]
    fn a_marker_with_an_action_is_a_button_named_by_its_label_enabled_or_not() {
        let palette = ComponentPalette::neutral_dark();
        let nodes = |marker: Marker| UiTree::new(marker.render()).layout(Size::new(240.0, 40.0)).access;
        for shimmer in [false, true] {
            let marker = || Marker::new("event", "Revisar cambios", palette).shimmer(shimmer).action("review");
            let enabled = nodes(marker());
            assert_eq!((enabled[0].id.as_str(), enabled[0].role, enabled[0].name.as_str()), ("review", Role::Button, "Revisar cambios"));
            assert!(enabled[0].actionable && !enabled[0].disabled);
            let disabled = nodes(marker().disabled());
            assert_eq!((disabled[0].id.as_str(), disabled[0].role, disabled[0].name.as_str()), ("review", Role::Button, "Revisar cambios"));
            assert!(disabled[0].disabled && !disabled[0].actionable);
        }
        // Without an action it is text, not a control.
        assert!(nodes(Marker::new("event", "Revisar cambios", palette)).is_empty());
    }

    #[test]
    fn thinking_marker_keeps_the_brain_static_and_only_shimmers_the_text() {
        let palette = ComponentPalette::neutral_dark();
        let make = || {
            Marker::new("thinking:response", "Pensando", palette)
                .icon(Icon::Code)
                .loading(true)
                .thinking(true)
                .render()
        };
        assert_eq!(make().style.min_height, 22.0);
        let start = Instant::now();
        let mut tree = UiTree::new(make());
        let first = tree.layout_at(Size::new(240.0, 24.0), start);
        let moving = tree.layout_at(Size::new(240.0, 24.0), start + Duration::from_millis(640));
        let brain = |frame: &crate::Frame| {
            frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Icon {
                        rect,
                        icon: Icon::Brain,
                        tint,
                    } => Some((*rect, *tint)),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(brain(&first), brain(&moving));
        assert_eq!(brain(&first).len(), 1);
        assert_eq!(brain(&first)[0].0.width, 16.0);
        assert_eq!(brain(&first)[0].0.height, 16.0);
        assert!(!moving
            .commands
            .iter()
            .any(|command| matches!(command, DrawCommand::Spinner { .. })));
        assert_ne!(first.commands, moving.commands);
        assert_eq!(first.text_layouts(), moving.text_layouts());
        assert_eq!(first.text_layouts()[0].source, "Pensando");
        assert!(tree.is_animating());
        assert!(moving.regions.is_empty());
        let brightest = ((u16::from(palette.muted.r) + u16::from(palette.text.r)) / 2) as u8;
        let tones = moving
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { color, .. } => Some(*color),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(tones.iter().any(|color| color.r > palette.muted.r));
        assert!(tones.iter().all(|color| color.r <= brightest));
        tree.set_root(make());
        let rebuilt = tree.layout_at(Size::new(240.0, 24.0), start + Duration::from_millis(640));
        assert_eq!(
            moving.commands, rebuilt.commands,
            "stable marker ID retains the text phase"
        );
    }

    #[test]
    fn thinking_marker_reduced_motion_is_static_and_offscreen_text_stops_repainting() {
        let palette = ComponentPalette::neutral_dark();
        let make = || {
            Marker::new("thinking", "Pensando", palette)
                .thinking(true)
                .render()
        };
        let start = Instant::now();
        let mut tree = UiTree::new(make());
        tree.set_reduced_motion(true);
        let first = tree.layout_at(Size::new(240.0, 22.0), start);
        let later = tree.layout_at(Size::new(240.0, 22.0), start + Duration::from_secs(2));
        assert_eq!(first.commands, later.commands);
        assert!(!tree.is_animating());
        assert_eq!(
            first
                .commands
                .iter()
                .filter(|command| matches!(command,
                    DrawCommand::Text { text, color, .. }
                        if text == "Pensando" && *color == palette.muted
                ))
                .count(),
            1
        );
        assert!(first.commands.iter().any(|command| matches!(
            command,
            DrawCommand::Icon {
                icon: Icon::Brain,
                ..
            }
        )));
        tree.set_reduced_motion(false);
        tree.set_root(
            crate::ScrollArea::new(
                "timeline",
                Element::column(vec![Element::spacer().size(None, Some(400.0)), make()]),
            )
            .render()
            .size(None, Some(60.0)),
        );
        tree.layout_at(Size::new(240.0, 60.0), start + Duration::from_secs(3));
        assert!(
            !tree.is_animating(),
            "offscreen thinking does not request frames"
        );
    }

    #[test]
    fn loading_and_completed_markers_preserve_their_existing_icons() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = UiTree::new(
            Marker::new("tool", "Leyendo archivo", palette)
                .thinking(false)
                .loading(true)
                .icon(Icon::File)
                .render(),
        );
        let frame = tree.layout(Size::new(240.0, 24.0));
        assert!(frame
            .commands
            .iter()
            .any(|command| matches!(command, DrawCommand::Spinner { .. })));
        assert!(!frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::Icon {
                icon: Icon::Brain,
                ..
            }
        )));
        tree.set_root(
            Marker::new("response", "Respuesta recibida", palette)
                .thinking(false)
                .loading(false)
                .icon(Icon::Check)
                .render(),
        );
        let frame = tree.layout(Size::new(240.0, 24.0));
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::Icon {
                icon: Icon::Check,
                ..
            }
        )));
        assert!(!tree.is_animating());
    }

    #[test]
    fn compact_markers_truncate_plain_and_shimmer_labels_within_the_action_row() {
        let measure = |text: &str| text.chars().count() as f32 * 8.0;
        let start = Instant::now();
        for width in [180.0, 360.0] {
            for (loading, thinking, reduced) in [
                (false, false, false),
                (true, false, false),
                (false, true, false),
                (true, false, true),
                (false, true, true),
            ] {
                let mut tree = UiTree::new(
                    Marker::new(
                        "compact",
                        "Exploración de archivos · src/main.rs · Completada · Unicode 終",
                        ComponentPalette::neutral_dark(),
                    )
                    .icon(Icon::File)
                    .loading(loading)
                    .thinking(thinking)
                    .truncate(true)
                    .action("open-tool")
                    .render(),
                );
                tree.set_text_measurer(move |text, _, _| measure(text));
                tree.set_reduced_motion(reduced);
                let frame =
                    tree.layout_at(Size::new(width, 32.0), start + Duration::from_millis(640));
                let action = frame.regions.iter().find(|r| r.id == "open-tool").unwrap();
                assert_eq!(action.rect.height, 32.0);
                let text: Vec<_> = frame
                    .commands
                    .iter()
                    .filter_map(|command| match command {
                        DrawCommand::Text { text, position, .. } => Some((text, position)),
                        _ => None,
                    })
                    .collect();
                assert!(!text.is_empty());
                for (text, position) in text {
                    assert!(text.ends_with('…'), "{text}");
                    assert!(position.x >= 0.0);
                    assert!(position.x + measure(text) <= width + 0.1);
                    assert!(position.y >= 0.0 && position.y + 13.0 <= 32.0);
                }
                assert_eq!(tree.is_animating(), (loading || thinking) && !reduced);
            }
        }
    }

    #[test]
    fn separator_keeps_its_icon_next_to_wrapping_text_and_centers_both_rules() {
        for width in [180.0, 620.0] {
            for label in [
                "Contexto compactado",
                "Contexto compactado · instrucciones y archivos preservados",
            ] {
                let mut tree = UiTree::new(Element::column(vec![Marker::new(
                    "checkpoint",
                    label,
                    ComponentPalette::neutral_dark(),
                )
                .variant(MarkerVariant::Separator)
                .icon(Icon::History)
                .render()]));
                let frame = tree.layout(Size::new(width, 240.0));
                let icon = frame
                    .commands
                    .iter()
                    .find_map(|command| match command {
                        DrawCommand::Icon {
                            rect,
                            icon: Icon::History,
                            ..
                        } => Some(*rect),
                        _ => None,
                    })
                    .unwrap();
                let layouts = frame.text_layouts();
                let text = &layouts[0];
                assert_eq!(text.source, label);
                let first = text.runs.first().unwrap().rect;
                let last = text.runs.last().unwrap().rect;
                assert!((first.x - icon.x - icon.width - 8.0).abs() < 0.1);
                let text_center = (first.y + last.y + last.height) / 2.0;
                assert!((icon.y + icon.height / 2.0 - text_center).abs() < 0.1);
                let rules = frame
                    .commands
                    .iter()
                    .filter_map(|command| match command {
                        DrawCommand::FillRect { rect, .. } if rect.height == 1.0 => Some(*rect),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                assert_eq!(rules.len(), 2);
                for rule in rules {
                    assert!((rule.y + rule.height / 2.0 - text_center).abs() < 0.1);
                }
                assert!(frame.regions.is_empty());
            }
        }
    }

    #[test]
    fn markers_wrap_without_losing_content_and_only_explicit_actions_receive_focus() {
        for width in [180.0, 360.0, 900.0] {
            for variant in [
                MarkerVariant::Default,
                MarkerVariant::Separator,
                MarkerVariant::Border,
            ] {
                let label = "Conversación compactada · instrucciones preservadas · 終わり";
                let mut tree = UiTree::new(
                    Marker::new("checkpoint", label, ComponentPalette::neutral_dark())
                        .variant(variant)
                        .icon(Icon::History)
                        .render(),
                );
                let frame = tree.layout(Size::new(width, 240.0));
                assert_eq!(frame.text_layouts()[0].source, label);
                assert!(frame.regions.is_empty());
                assert!(!tree.is_animating());
                for run in &frame.text_layouts()[0].runs {
                    assert!(
                        run.rect.x >= -0.1 && run.rect.x + run.rect.width <= width + 0.1,
                        "{run:?}"
                    );
                }
                let rules = frame.commands.iter().filter(|command| matches!(command, DrawCommand::FillRect { rect, .. } if rect.height == 1.0)).count();
                assert_eq!(
                    rules,
                    match variant {
                        MarkerVariant::Default => 0,
                        MarkerVariant::Separator => 2,
                        MarkerVariant::Border => 1,
                    }
                );
            }
        }
        let mut tree = UiTree::new(
            Marker::new("event", "Ver resultado", ComponentPalette::neutral_dark())
                .action("open-result")
                .render(),
        );
        let frame = tree.layout(Size::new(260.0, 50.0));
        let target = frame
            .regions
            .iter()
            .find(|region| region.kind == HitKind::Button)
            .unwrap()
            .rect;
        let point = Point::new(target.x + 5.0, target.y + 5.0);
        tree.handle_event(UiEvent::PointerDown(point));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(point)),
            vec![UiAction::Clicked {
                id: "open-result".into()
            }]
        );
        assert!(tree.focus("open-result"));
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: Key::Enter,
                shift: false,
                ctrl: false
            }),
            vec![UiAction::Clicked {
                id: "open-result".into()
            }]
        );
    }

    #[test]
    fn ink_colors_the_label_and_the_icon_and_leaves_the_palette_alone() {
        let palette = ComponentPalette::neutral_dark();
        let ink = Color::rgb(220, 90, 90);
        let frame_of = |marker: Marker| {
            let mut tree = UiTree::new(marker.icon(Icon::Code).render());
            tree.set_reduced_motion(true);
            tree.layout(Size::new(240.0, 24.0))
        };
        let plain = frame_of(Marker::new("event", "Solicitud fallida", palette));
        let inked = frame_of(Marker::new("event", "Solicitud fallida", palette).ink(ink));
        let colors = |frame: &crate::Frame| {
            frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { color, .. } => Some(*color),
                    DrawCommand::Icon { tint, .. } => Some(*tint),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(colors(&plain), vec![palette.muted; 2]);
        assert_eq!(colors(&inked), vec![ink; 2]);
        // The same as a palette whose muted is the ink, with nothing moved.
        let by_palette = frame_of(Marker::new(
            "event",
            "Solicitud fallida",
            ComponentPalette { muted: ink, ..palette },
        ));
        assert_eq!(inked.commands, by_palette.commands);
        assert_eq!(plain.text_layouts(), inked.text_layouts());
    }

    #[test]
    fn a_disabled_action_keeps_its_place_and_takes_no_pointer_or_focus() {
        let palette = ComponentPalette::neutral_dark();
        let size = Size::new(260.0, 50.0);
        for variant in [MarkerVariant::Default, MarkerVariant::Separator, MarkerVariant::Border] {
            let tree_of = |enabled| {
                let mut tree = UiTree::new(Element::column(vec![
                    Marker::new("event", "Ver resultado", palette)
                        .variant(variant)
                        .icon(Icon::File)
                        .action("open-result")
                        .enabled(enabled)
                        .render(),
                    Element::spacer().flex(1.0),
                ]));
                tree.set_reduced_motion(true);
                tree
            };
            let mut tree = tree_of(true);
            let enabled = tree.layout(size);
            let [target] = enabled.regions.as_slice() else {
                panic!("one region, got {:?}", enabled.regions);
            };
            let point = Point::new(target.rect.x + 5.0, target.rect.y + 5.0);

            let mut tree = tree_of(false);
            let disabled = tree.layout(size);
            assert!(disabled.regions.is_empty());
            tree.handle_event(UiEvent::PointerMove(point));
            assert!(tree.handle_event(UiEvent::PointerDown(point)).is_empty());
            assert!(tree.handle_event(UiEvent::PointerUp(point)).is_empty());
            for id in ["event", "open-result"] {
                assert!(!tree.focus(id));
            }
            let enter = UiEvent::Key { key: Key::Enter, shift: false, ctrl: false };
            assert!(tree.handle_event(enter).is_empty());
            // Muted as ever and in the same place, hovered or not: only the control is gone.
            assert_eq!(disabled.commands, enabled.commands);
            assert_eq!(tree.layout(size).commands, disabled.commands);
        }
        // Without an action there is nothing to disable.
        let passive = |enabled| Marker::new("event", "Ver resultado", palette).enabled(enabled).render();
        let frame = |element| UiTree::new(element).layout(size);
        assert_eq!(frame(passive(false)), frame(passive(true)));
    }
}
