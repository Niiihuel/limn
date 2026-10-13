//! Controlled disclosure rows for progressively received content.
//!
//! The host owns expansion state and supplies the toggle action ID. Loading is
//! a visual state only; this component never infers completion or runs actions
//! represented by its content.

use crate::{
    ButtonVariant, Collapse, Color, Component, ComponentPalette, ControlSize, Element, FontFamily,
    Icon, IconButton, Spinner, Tooltip, TooltipPlacement,
};

const HEADER_HEIGHT: f32 = 28.0;
const HEADER_PADDING: f32 = 6.0;
/// Where the body starts: under the title, past the leading icon.
const BODY_INDENT: f32 = 33.0;
const ICON_SIDE: f32 = 14.0;
const FOOTER_INSET: f32 = 2.0;
const FOOTER_HEIGHT: f32 = 28.0;

/// A compact disclosure header with a body present only while expanded.
/// Keep its action ID stable across content updates so focus and the loading
/// indicator retain their identity. Pointer and keyboard activation produce
/// the ordinary `UiAction::Clicked`; the host updates `expanded` in response.
pub struct Collapsible {
    id: String,
    title: String,
    content: Element,
    palette: ComponentPalette,
    expanded: bool,
    icon: Option<Icon>,
    loading: bool,
    thinking: bool,
    detail: Option<String>,
    collapse_footer: bool,
    enabled: bool,
    ink: Option<Color>,
}

impl Collapsible {
    pub fn new(
        toggle_action_id: impl Into<String>,
        title: impl Into<String>,
        content: Element,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            id: toggle_action_id.into(),
            title: title.into(),
            content,
            palette,
            expanded: false,
            icon: None,
            loading: false,
            thinking: false,
            detail: None,
            collapse_footer: false,
            enabled: true,
            ink: None,
        }
    }

    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    /// Repeat the disclosure at the end of long content, without another
    /// state or action. The distinct visual ID keeps focus and hit testing unique.
    pub fn collapse_footer(mut self, visible: bool) -> Self {
        self.collapse_footer = visible;
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Replace the leading icon with the framework's passive spinner. The
    /// leading slot keeps its width when this state changes.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Show model thinking with a static brain and a single-line text sweep.
    /// This takes precedence over `loading`; ordinary loading retains its
    /// spinner. Reduced motion keeps the title and icon completely static.
    pub fn thinking(mut self, thinking: bool) -> Self {
        self.thinking = thinking;
        self
    }

    /// Short trailing metadata, such as a received/partial state. Both title
    /// and detail truncate in their allotted width on narrow surfaces.
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// A status color from the host, such as its failure ink, for the icon,
    /// the detail, the chevron and the footer in place of the palette's
    /// muted. The title keeps the palette's text color.
    pub fn ink(mut self, ink: Color) -> Self {
        self.ink = Some(ink);
        self
    }

    /// A disabled one cannot be opened or closed: its header is passive, with
    /// a muted title and no hover, hit region or focus, and so is its footer.
    /// It stays as `expanded` says, and its content is the host's as ever.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable it; the same as `enabled(false)`.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
}

fn centered_icon(icon: Element) -> Element {
    Element::column(vec![
        Element::spacer().flex(1.0),
        icon.size(Some(ICON_SIDE), Some(ICON_SIDE)),
        Element::spacer().flex(1.0),
    ])
    .size(Some(ICON_SIDE), None)
}

impl Component for Collapsible {
    fn render(mut self) -> Element {
        if let Some(ink) = self.ink {
            self.palette.muted = ink;
        }
        let leading = if self.thinking {
            Element::icon(Icon::Brain).foreground(self.palette.muted)
        } else if self.loading {
            Spinner::new(format!("{}:spinner", self.id), self.palette.muted).render()
        } else if let Some(icon) = self.icon {
            Element::icon(icon).foreground(self.palette.muted)
        } else {
            Element::spacer()
        };
        let title = if self.thinking {
            Element::shimmer_truncated_text(
                format!("{}:text", self.id),
                self.title,
                crate::marker::thinking_highlight(self.palette),
            )
            .foreground(self.palette.muted)
        } else {
            let ink = if self.enabled { self.palette.text } else { self.palette.muted };
            Element::truncated_text(self.title).foreground(ink)
        };
        let mut header = vec![
            centered_icon(leading),
            title
                .font_family(FontFamily::Sans)
                .font_size(13.0)
                .size(None, Some(HEADER_HEIGHT - HEADER_PADDING * 2.0))
                // The title keeps its own width and is the first to give it
                // up, with an ellipsis, when the header is too narrow.
                .min_size(48.0, 0.0)
                .shrink_to_fit(),
        ];
        if let Some(detail) = self.detail.filter(|detail| !detail.is_empty()) {
            header.push(
                Element::truncated_text(detail)
                    .font_family(FontFamily::Sans)
                    .font_size(11.0)
                    .foreground(self.palette.muted)
                    .size(None, Some(HEADER_HEIGHT - HEADER_PADDING * 2.0))
                    // Short metadata right after the title; it yields only
                    // once the title is down to its minimum.
                    .max_width(240.0)
                    .shrink_to_fit(),
            );
        }
        header.push(Element::spacer().flex(1.0));
        header.push(centered_icon(
            Element::icon(if self.expanded {
                Icon::ChevronDown
            } else {
                Icon::ChevronRight
            })
            .foreground(self.palette.muted),
        ));
        let header = Element::row(header).gap(6.0).clip_children();
        let header = if self.enabled {
            self.palette.interactive_surface(Element::clickable(&self.id, header))
        } else {
            Element::panel(header)
        }
        .padding(HEADER_PADDING)
        .size(None, Some(HEADER_HEIGHT));
        let body_id = format!("{}:body", self.id);
        let footer = self.collapse_footer;
        let enabled = self.enabled;
        let (id, palette) = (self.id, self.palette);
        let content = self.content;
        let body = self.expanded.then(|| {
            let mut content = vec![Element::spacer().size(None, Some(4.0)), content];
            if !footer {
                content.push(Element::spacer().size(None, Some(4.0)));
            }
            // The footer owns its vertical inset; do not add another bottom
            // spacer and inter-item gap between the last line and its control.
            let content = Element::column(content).gap(4.0).flex(1.0);
            let mut rows = vec![
                Element::spacer().size(None, Some(2.0)),
                Element::row(vec![
                    // Under the title, past the leading icon.
                    Element::spacer().size(Some(BODY_INDENT), Some(1.0)),
                    content,
                    Element::spacer().size(Some(HEADER_PADDING), Some(1.0)),
                ]),
            ];
            if footer {
                rows.push(Element::spacer().size(None, Some(2.0)));
                rows.push(footer_row(&id, palette, enabled));
            }
            Element::column(rows)
        });
        // The header and what it discloses read as one block while open, and
        // the body opens and closes in place instead of appearing at once.
        let expanded = body.is_some();
        let block = Element::column(vec![header, Collapse::new(body_id, body).expand().render()])
            .radius(crate::components::surface_radius());
        if expanded {
            block.background(palette.hover)
        } else {
            block
        }
    }
}

/// Common quiet footer for transcript disclosures. It dispatches the header's
/// action so the host retains its session/request validation and scroll policy.
/// Public so a host's own disclosures can end with the same control. `id` is
/// the header's action ID.
pub fn collapse_footer(id: &str, palette: ComponentPalette) -> Element {
    footer_row(id, palette, true)
}

fn footer_row(id: &str, palette: ComponentPalette, enabled: bool) -> Element {
    let button_id = format!("{id}:collapse");
    let button = IconButton::new(
        &button_id,
        Icon::ArrowUp,
        ComponentPalette {
            text: palette.muted,
            ..palette
        },
    )
    .variant(ButtonVariant::Ghost)
    .size(ControlSize::Dense)
    .enabled(enabled)
    .render();
    // A disabled footer is passive content and has nothing to dispatch.
    let button = if enabled { button.dispatch_as(id) } else { button };
    Element::row(vec![
        Element::spacer().flex(1.0),
        Tooltip::new(button, button_id, crate::labels().collapse, palette)
            .placement(TooltipPlacement::Above)
            .render(),
        Element::spacer().flex(1.0),
    ])
    .bottom_background_fade(
        Color {
            a: 24,
            ..palette.shadow
        },
        FOOTER_HEIGHT,
    )
    .padding(FOOTER_INSET)
    .size(None, Some(FOOTER_HEIGHT))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, HitKind, Key, Point, Size, UiAction, UiEvent, UiTree};
    use web_time::{Duration, Instant};

    #[test]
    fn footer_collapses_the_same_disclosure_and_does_not_duplicate_its_identity() {
        for width in [180.0, 360.0, 900.0] {
            let make = |expanded| {
                Collapsible::new(
                    "details",
                    "Actividad",
                    Element::text("Contenido extenso\nResultado final"),
                    ComponentPalette::neutral_dark(),
                )
                .expanded(expanded)
                .collapse_footer(true)
                .render()
            };
            let mut tree = UiTree::new(make(true));
            let frame = tree.layout(Size::new(width, 400.0));
            let header = frame
                .regions
                .iter()
                .find(|r| r.id == "details")
                .unwrap()
                .rect;
            let footer = frame
                .regions
                .iter()
                .find(|r| r.id == "details:collapse")
                .unwrap()
                .rect;
            assert!(footer.y >= header.y + header.height);
            assert!(footer.x >= 0.0 && footer.x + footer.width <= width);
            assert_eq!(footer.width, 24.0);
            assert_eq!(footer.height, 24.0);
            assert!((footer.x + footer.width / 2.0 - width / 2.0).abs() < 0.1);
            assert!(
                !frame.commands.iter().any(|command| matches!(command,
                    DrawCommand::Text { text, .. } if text == "Contraer"
                )),
                "the footer label belongs only to its tooltip"
            );
            assert_eq!(
                frame.regions.iter().filter(|r| r.id == "details").count(),
                1
            );
            assert!(tree.focus("details:collapse"));
            assert_eq!(
                tree.handle_event(UiEvent::Key {
                    key: Key::Enter,
                    shift: false,
                    ctrl: false,
                }),
                vec![UiAction::Clicked {
                    id: "details".into()
                }]
            );
            tree.set_root(make(false));
            let frame = tree.layout(Size::new(width, 400.0));
            assert!(!frame.regions.iter().any(|r| r.id == "details:collapse"));
            assert!(
                !frame
                    .text_layouts()
                    .iter()
                    .any(|l| l.source.contains("Resultado final"))
            );
        }
    }

    #[test]
    fn footer_fade_is_passive_and_only_the_icon_gets_hover_and_pointer_activation() {
        let palette = ComponentPalette::neutral_dark();
        for width in [180.0, 360.0, 900.0] {
            let mut tree = UiTree::new(
                Collapsible::new(
                    "details",
                    "Actividad",
                    Element::text("Última línea"),
                    palette,
                )
                .expanded(true)
                .collapse_footer(true)
                .render(),
            );
            tree.set_reduced_motion(true);
            let size = Size::new(width, 300.0);
            let frame = tree.layout(size);
            let button = frame
                .regions
                .iter()
                .find(|region| region.id == "details:collapse")
                .unwrap()
                .rect;
            let (fade_index, fade) = frame
                .commands
                .iter()
                .enumerate()
                .find_map(|(index, command)| {
                    if let DrawCommand::FillVerticalGradient { rect, top, bottom } = command {
                        assert_eq!(top.a, 0);
                        assert!(
                            bottom.a > 0 && bottom.a <= 32,
                            "the footer fade stays subtle"
                        );
                        Some((index, *rect))
                    } else {
                        None
                    }
                })
                .expect("a native footer gradient");
            assert_eq!(fade.height, FOOTER_HEIGHT);
            assert_eq!(button.y - fade.y, FOOTER_INSET);
            assert_eq!(
                fade.y + fade.height - button.y - button.height,
                FOOTER_INSET
            );
            assert_eq!(fade.width, width);
            for block in frame
                .text_layouts()
                .iter()
                .filter(|block| block.source == "Última línea")
            {
                assert!(
                    block
                        .runs
                        .iter()
                        .all(|run| run.rect.y + run.rect.height <= fade.y),
                    "the dedicated footer never masks the last content line"
                );
            }
            assert!(
                frame.commands[fade_index + 1..]
                    .iter()
                    .any(|command| matches!(
                        command,
                        DrawCommand::Icon {
                            icon: Icon::ArrowUp,
                            ..
                        }
                    )),
                "footer icon is painted over its passive gradient"
            );
            assert!(
                !frame.commands.iter().any(|command| matches!(command,
                    DrawCommand::FillRect { rect, .. }
                        if rect.y >= fade.y && rect.height <= 1.0 && rect.width > button.width
                )),
                "no divider spans the footer"
            );

            let side = Point::new(8.0, button.y + button.height / 2.0);
            assert_ne!(frame.hit_test(side), Some("details:collapse"));
            // The open block is filled; the footer's empty side adds no fill of its own.
            let fills = |frame: &crate::Frame| {
                frame
                    .commands
                    .iter()
                    .filter(|command| matches!(command,
                        DrawCommand::FillRect { color, .. } if *color == palette.hover
                    ))
                    .count()
            };
            let resting = fills(&frame);
            tree.handle_event(UiEvent::PointerMove(side));
            let side_hover = tree.layout(size);
            assert_eq!(fills(&side_hover), resting, "the empty footer is not a hover surface");
            tree.handle_event(UiEvent::PointerDown(side));
            assert!(tree.handle_event(UiEvent::PointerUp(side)).is_empty());

            let below = Point::new(button.x + button.width / 2.0, fade.y + fade.height - 0.5);
            assert_ne!(frame.hit_test(below), Some("details:collapse"));
            tree.handle_event(UiEvent::PointerMove(below));
            let bottom_hover = tree.layout(size);
            assert_eq!(
                fills(&bottom_hover),
                resting,
                "the bottom inset stays passive instead of extending hover to the border"
            );

            let center = Point::new(
                button.x + button.width / 2.0,
                button.y + button.height / 2.0,
            );
            tree.handle_event(UiEvent::PointerMove(center));
            let hovered = tree.layout(size);
            // Besides the open block's own fill, only the button is lit.
            let highlights: Vec<_> = hovered
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::FillRect { rect, color, .. }
                        if *color == palette.hover && rect.width <= button.width =>
                    {
                        Some(*rect)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(highlights, vec![button]);
            assert_eq!(fills(&hovered), resting + 1);
            tree.handle_event(UiEvent::PointerDown(center));
            assert_eq!(
                tree.handle_event(UiEvent::PointerUp(center)),
                vec![UiAction::Clicked {
                    id: "details".into()
                }]
            );
        }
    }

    #[test]
    fn footer_owns_its_inset_without_a_duplicate_body_bottom_spacer() {
        let mut tree = UiTree::new(
            Collapsible::new(
                "details",
                "Actividad",
                Element::button("last", "Último resultado").size(None, Some(36.0)),
                ComponentPalette::neutral_dark(),
            )
            .expanded(true)
            .collapse_footer(true)
            .render(),
        );
        let frame = tree.layout(Size::new(360.0, 300.0));
        let last = frame
            .regions
            .iter()
            .find(|region| region.id == "last")
            .unwrap()
            .rect;
        let footer = frame
            .regions
            .iter()
            .find(|region| region.id == "details:collapse")
            .unwrap()
            .rect;
        assert_eq!(footer.y - last.y - last.height, 2.0 + FOOTER_INSET);
        assert_eq!((footer.width, footer.height), (24.0, 24.0));
    }

    #[test]
    fn thinking_header_stays_one_line_with_measured_ellipsis_and_static_brain() {
        let palette = ComponentPalette::neutral_dark();
        let start = Instant::now();
        let measure = |text: &str| {
            text.chars()
                .map(|ch| if ch == 'i' { 3.0 } else { 9.0 })
                .sum::<f32>()
        };
        for width in [100.0, 180.0, 240.0, 360.0, 900.0] {
            let make = || {
                Collapsible::new(
                    "reasoning:toggle",
                    "Pensando · revisando instrucciones y referencias del proyecto antes de responder",
                    Element::text("Contenido privado hasta expandir"),
                    palette,
                )
                .icon(Icon::Code)
                .loading(true)
                .thinking(true)
                .detail("En curso")
                .render()
            };
            let mut tree = UiTree::new(make());
            tree.set_text_measurer(move |text, _, _| measure(text));
            let size = Size::new(width, HEADER_HEIGHT);
            let first = tree.layout_at(size, start);
            let moving = tree.layout_at(size, start + Duration::from_millis(640));
            let header = first
                .regions
                .iter()
                .find(|region| region.id == "reasoning:toggle")
                .unwrap();
            assert_eq!(header.rect.width, width);
            assert_eq!(header.rect.height, 28.0);
            let icons = first
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Icon { rect, .. } => Some(*rect),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(icons.len(), 2);
            for rect in &icons {
                assert_eq!(rect.width, 14.0);
                assert_eq!(rect.height, 14.0);
                assert_eq!(rect.y + rect.height / 2.0, header.rect.height / 2.0);
                assert!(rect.x >= 6.0 && rect.x + rect.width <= width - 6.0);
            }
            assert!(
                first.text_layouts().is_empty(),
                "collapsed title stays passive single-line text"
            );
            let title_runs = moving
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text {
                        text,
                        position,
                        font_size,
                        ..
                    } if *font_size == 13.0 => Some((text, position)),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert!(!title_runs.is_empty());
            let detail = moving
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::Text {
                        text,
                        position,
                        font_size,
                        ..
                    } if *font_size == 11.0 => Some((text, position)),
                    _ => None,
                });
            // The narrowest header has no room left for the detail.
            assert_eq!(detail.is_some(), width > 100.0);
            let title_limit = detail.map_or(icons[1].x, |detail| detail.1.x);
            assert!(title_runs.iter().all(|(text, position)| {
                // Only the widest header fits the whole title.
                (text.ends_with('…') || width == 900.0)
                    && !text.contains('\n')
                    && position.x >= icons[0].x + icons[0].width + 6.0
                    && position.x + measure(text) <= title_limit - 6.0 + 0.1
                    && position.y >= 0.0
                    && position.y + 13.0 <= HEADER_HEIGHT
            }));
            if let Some(detail) = detail {
                assert!(detail.1.x + measure(detail.0) <= icons[1].x - 6.0 + 0.1);
                assert!(detail.1.y >= 0.0 && detail.1.y + 11.0 <= HEADER_HEIGHT);
            }
            assert!(
                !moving
                    .commands
                    .iter()
                    .any(|command| matches!(command, DrawCommand::Spinner { .. }))
            );
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
            assert_ne!(first.commands, moving.commands);
            assert!(tree.is_animating());
            tree.set_root(make());
            let rebuilt = tree.layout_at(size, start + Duration::from_millis(640));
            assert_eq!(
                moving.commands, rebuilt.commands,
                "header ID preserves shimmer phase"
            );
            assert!(tree.focus("reasoning:toggle"));
            assert_eq!(
                tree.handle_event(UiEvent::Key {
                    key: Key::Enter,
                    shift: false,
                    ctrl: false,
                }),
                vec![UiAction::Clicked {
                    id: "reasoning:toggle".into()
                }]
            );
        }
    }

    #[test]
    fn thinking_header_reduced_motion_and_completion_stop_the_text_sweep() {
        let palette = ComponentPalette::neutral_dark();
        let start = Instant::now();
        let mut tree = UiTree::new(
            Collapsible::new("reasoning", "Pensando", Element::text("Detalle"), palette)
                .thinking(true)
                .render(),
        );
        tree.set_reduced_motion(true);
        let first = tree.layout_at(Size::new(240.0, HEADER_HEIGHT), start);
        let later = tree.layout_at(
            Size::new(240.0, HEADER_HEIGHT),
            start + Duration::from_secs(2),
        );
        assert_eq!(first.commands, later.commands);
        assert!(!tree.is_animating());
        assert_eq!(first.commands.iter().filter(|command| matches!(command,
            DrawCommand::Text { text, color, .. } if text == "Pensando" && *color == palette.muted
        )).count(), 1);
        tree.set_reduced_motion(false);
        tree.set_root(
            Collapsible::new(
                "reasoning",
                "Razonamiento",
                Element::text("Detalle"),
                palette,
            )
            .thinking(false)
            .icon(Icon::Brain)
            .render(),
        );
        let frame = tree.layout_at(
            Size::new(240.0, HEADER_HEIGHT),
            start + Duration::from_secs(3),
        );
        assert!(!tree.is_animating());
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, color, .. } if text == "Razonamiento" && *color == palette.text
        )));
    }

    #[test]
    fn ordinary_loading_headers_keep_the_spinner_and_offscreen_thinking_is_idle() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = UiTree::new(
            Collapsible::new("tool", "Lectura", Element::spacer(), palette)
                .thinking(false)
                .loading(true)
                .icon(Icon::File)
                .render(),
        );
        for width in [100.0, 180.0, 240.0, 360.0, 900.0] {
            let frame = tree.layout(Size::new(width, HEADER_HEIGHT));
            let spinner = frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::Spinner { rect, .. } => Some(*rect),
                    _ => None,
                })
                .unwrap();
            assert_eq!(spinner.width, 14.0);
            assert_eq!(spinner.height, 14.0);
            assert_eq!(spinner.y + spinner.height / 2.0, 14.0);
            assert!(spinner.x >= 6.0 && spinner.x + spinner.width <= width - 6.0);
        }
        tree.set_root(
            crate::ScrollArea::new(
                "timeline",
                Element::column(vec![
                    Element::spacer().size(None, Some(400.0)),
                    Collapsible::new("offscreen", "Pensando", Element::spacer(), palette)
                        .thinking(true)
                        .render(),
                ]),
            )
            .render()
            .size(None, Some(60.0)),
        );
        tree.layout(Size::new(240.0, 60.0));
        assert!(!tree.is_animating());
    }

    #[test]
    fn ink_colors_the_secondary_parts_and_the_title_keeps_its_text_color() {
        let palette = ComponentPalette::neutral_dark();
        let ink = Color::rgb(220, 90, 90);
        let make = |palette: ComponentPalette, ink: Option<Color>| {
            let row = Collapsible::new("tool", "Exploración", Element::text("cuerpo"), palette)
                .icon(Icon::Search)
                .detail("2 fallidas")
                .expanded(true)
                .collapse_footer(true);
            let mut tree = UiTree::new(match ink {
                Some(ink) => row.ink(ink).render(),
                None => row.render(),
            });
            tree.set_reduced_motion(true);
            tree.layout(Size::new(360.0, 200.0))
        };
        let inked = make(palette, Some(ink));
        let color_of = |label: &str| {
            inked
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::Text { text, color, .. } if text == label => Some(*color),
                    _ => None,
                })
                .unwrap()
        };
        assert_eq!(color_of("Exploración"), palette.text);
        assert_eq!(color_of("2 fallidas"), ink);
        let tints = inked
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Icon { tint, .. } => Some(*tint),
                _ => None,
            })
            .collect::<Vec<_>>();
        // The leading icon, the chevron and the footer's arrow.
        assert_eq!(tints, vec![ink; 3]);
        assert_eq!(
            inked.commands,
            make(ComponentPalette { muted: ink, ..palette }, None).commands
        );
        assert_ne!(inked.commands, make(palette, None).commands);
    }

    #[test]
    fn a_disabled_one_stays_as_it_is_with_a_passive_header_and_footer_in_place() {
        let palette = ComponentPalette::neutral_dark();
        let size = Size::new(360.0, 200.0);
        for expanded in [false, true] {
            let tree_of = |enabled| {
                let mut tree = UiTree::new(
                    Collapsible::new(
                        "details",
                        "Actividad",
                        Element::button("inside", "Resultado"),
                        palette,
                    )
                    .icon(Icon::Search)
                    .detail("2 pasos")
                    .expanded(expanded)
                    .collapse_footer(true)
                    .enabled(enabled)
                    .render(),
                );
                tree.set_reduced_motion(true);
                tree
            };
            // Every icon and line of text, with its place.
            let places = |frame: &crate::Frame| {
                frame
                    .commands
                    .iter()
                    .filter_map(|command| match command {
                        DrawCommand::Icon { rect, .. } => Some((rect.x, rect.y)),
                        DrawCommand::Text { position, .. } => Some((position.x, position.y)),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            };
            let enabled = tree_of(true).layout(size);
            let rect_of = |id: &str| enabled.regions.iter().find(|r| r.id == id).map(|r| r.rect);
            let mut tree = tree_of(false);
            let disabled = tree.layout(size);
            // Only the host's own content can still be pressed.
            let buttons = disabled
                .regions
                .iter()
                .filter(|region| region.kind == HitKind::Button)
                .map(|region| region.id.as_str())
                .collect::<Vec<_>>();
            assert_eq!(buttons, if expanded { vec!["inside"] } else { Vec::new() });
            for rect in [rect_of("details"), rect_of("details:collapse")].into_iter().flatten() {
                let point = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
                tree.handle_event(UiEvent::PointerMove(point));
                assert!(tree.handle_event(UiEvent::PointerDown(point)).is_empty());
                assert!(tree.handle_event(UiEvent::PointerUp(point)).is_empty());
            }
            for id in ["details", "details:collapse"] {
                assert!(!tree.focus(id));
            }
            let enter = UiEvent::Key { key: Key::Enter, shift: false, ctrl: false };
            assert!(tree.handle_event(enter).is_empty());
            assert_eq!(places(&disabled), places(&enabled));
            assert!(disabled.commands.iter().any(|command| matches!(command,
                DrawCommand::Text { text, color, .. } if text == "Actividad" && *color == palette.muted
            )));
            assert_eq!(tree.focus("inside"), expanded);
        }
    }
}
