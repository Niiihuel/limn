//! What fills a model's context window, share by share.

use crate::components::control_radius;
use crate::page::px;
use crate::{
    Color, Component, ComponentPalette, Element, FontFamily, FramedCard, Icon, Spinner, Ticker,
    UnitGrid, UnitGridPart,
};

const COLUMNS: u16 = 36;
const ROWS: u16 = 6;
const GAP: f32 = 2.0;
const INSET: f32 = FramedCard::INSET;
const ROW_HEIGHT: f32 = FramedCard::ROW_HEIGHT;
const ROW_PAD: f32 = FramedCard::REACH;
const ITEM_HEIGHT: f32 = 24.0;
/// What stands where the count of an unmeasured share would be.
const UNMEASURED: &str = "–";

/// One share of the window, such as the conversation or the tools.
#[derive(Clone, Debug)]
pub struct ContextShare {
    pub id: String,
    pub label: String,
    pub tokens: u64,
    pub color: Color,
    /// What the share is made of. A share with items opens to list them.
    pub items: Vec<ContextItem>,
    pub action: Option<ContextShareAction>,
    /// Whether `tokens` was measured. A share that was not reads "–" where
    /// its count would be, and takes no cells.
    pub measured: bool,
}

impl ContextShare {
    pub fn new(id: impl Into<String>, label: impl Into<String>, tokens: u64, color: Color) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            tokens,
            color,
            items: Vec::new(),
            action: None,
            measured: true,
        }
    }

    /// A share the host could not measure: it is listed, without a count.
    pub fn unmeasured(mut self) -> Self {
        self.measured = false;
        self.tokens = 0;
        self
    }

    pub fn items(mut self, items: Vec<ContextItem>) -> Self {
        self.items = items;
        self
    }

    pub fn action(mut self, action: ContextShareAction) -> Self {
        self.action = Some(action);
        self
    }
}

/// One thing inside a share, such as an attached file. It can be removed.
#[derive(Clone, Debug)]
pub struct ContextItem {
    pub id: String,
    pub label: String,
    pub tokens: u64,
}

/// What a share offers to make it smaller, and how that is going.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContextShareAction {
    /// The label of an action that can be chosen.
    Offer(String),
    /// The label while the host carries it out.
    Busy(String),
    /// The label once it is done.
    Done(String),
}

/// The colours that carry meaning: a window that is filling up, one that is
/// full, and a share that was made smaller.
#[derive(Clone, Copy, Debug)]
pub struct ContextTones {
    pub warning: Color,
    pub danger: Color,
    pub success: Color,
}

/// The host owns every figure and passes them on each rebuild; the figures
/// count and the cells move to a new state by themselves. It takes the width
/// it is given and draws its own card on a frame a shade apart, with the foot
/// on the frame, so a host gives it a surface with rounded corners and no
/// padding: a popover, or a panel.
///
/// Actions: `"{id}:share:{share}"` for a share's row, which the host answers
/// by opening or closing it, `"{id}:action:{share}"` for its action and
/// `"{id}:remove:{share}:{item}"` for an item's removal.
pub struct ContextWindow {
    id: String,
    title: String,
    badge: Option<String>,
    shares: Vec<ContextShare>,
    capacity: u64,
    palette: ComponentPalette,
    tones: ContextTones,
    focused: Option<String>,
    expanded: Vec<String>,
    caption: String,
    status: String,
    status_done: bool,
    footer_action: Option<(String, String)>,
    header_icons: Vec<(String, Icon, String)>,
    status_lead: Option<String>,
    flat: bool,
    width: f32,
    format: fn(f32) -> String,
}

impl ContextWindow {
    /// `capacity` is the size of the window in tokens; zero is a window of
    /// unknown size, which the shares then fill between them.
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        shares: Vec<ContextShare>,
        capacity: u64,
        palette: ComponentPalette,
        tones: ContextTones,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            badge: None,
            shares,
            capacity,
            palette,
            tones,
            focused: None,
            expanded: Vec::new(),
            caption: String::new(),
            status: String::new(),
            status_done: false,
            footer_action: None,
            header_icons: Vec::new(),
            status_lead: None,
            flat: false,
            width: 320.0,
            format: Self::tokens_text,
        }
    }

    /// A short name beside the title, such as the model's.
    pub fn badge(mut self, badge: impl Into<String>) -> Self {
        self.badge = Some(badge.into());
        self
    }

    /// The share that stands out: the figure becomes its own and the other
    /// shares recede. Pass [`ContextWindow::share_under`] of the hovered ID.
    pub fn focused(mut self, share: Option<&str>) -> Self {
        self.focused = share.map(str::to_owned);
        self
    }

    /// The shares whose items are listed.
    pub fn expanded(mut self, shares: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.expanded = shares.into_iter().map(Into::into).collect();
        self
    }

    /// The words after the figure while no share is in focus, such as
    /// "of 200K tokens". A share in focus puts its label there.
    pub fn caption(mut self, caption: impl Into<String>) -> Self {
        self.caption = caption.into();
        self
    }

    /// The line at the foot, such as what is left of the window.
    pub fn status(mut self, status: impl Into<String>) -> Self {
        self.status = status.into();
        self.status_done = false;
        self
    }

    /// As [`ContextWindow::status`], in the colour of something achieved.
    pub fn status_done(mut self, status: impl Into<String>) -> Self {
        self.status = status.into();
        self.status_done = true;
        self
    }

    /// An action at the foot, opposite the status.
    pub fn footer_action(mut self, id: impl Into<String>, label: impl Into<String>) -> Self {
        self.footer_action = Some((id.into(), label.into()));
        self
    }

    /// A small action at the end of the header; `label` is its hint.
    pub fn header_icon(mut self, id: impl Into<String>, icon: Icon, label: impl Into<String>) -> Self {
        self.header_icons.push((id.into(), icon, label.into()));
        self
    }

    /// The words the status opens with, which read stronger than the rest:
    /// the share in focus, in "Files · 23% of the window".
    pub fn status_lead(mut self, lead: impl Into<String>) -> Self {
        self.status_lead = Some(lead.into());
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = width.max(200.0);
        }
        self
    }

    /// For a host whose surface already is the card, such as a popover: the
    /// content stands on that surface, with no frame and no card around it.
    pub fn flat(mut self) -> Self {
        self.flat = true;
        self
    }

    /// The content and the foot apart, for a host that scrolls one and pins
    /// the other under it. Use it with [`ContextWindow::flat`].
    pub fn parts(self) -> (Element, Element) {
        let (body, foot) = self.card().into_parts();
        (body, foot.unwrap_or_else(Element::spacer))
    }

    /// How a count of tokens is written, also while it counts.
    pub fn format(mut self, format: fn(f32) -> String) -> Self {
        self.format = format;
        self
    }

    /// The share whose row or action `control` is, for a card with this `id`.
    pub fn share_under<'a>(id: &str, control: &'a str) -> Option<&'a str> {
        let rest = control.strip_prefix(id)?;
        rest.strip_prefix(":share:")
            .or_else(|| rest.strip_prefix(":action:"))
    }

    /// "950", "4.2K", "175.6K", "1.25M".
    pub fn tokens_text(tokens: f32) -> String {
        let tokens = tokens.max(0.0);
        if tokens >= 999_950.0 {
            format!("{:.2}M", tokens / 1_000_000.0)
        } else if tokens >= 1_000.0 {
            format!("{:.1}K", tokens / 1_000.0)
        } else {
            format!("{}", tokens.round() as u32)
        }
    }

    fn percent_text(percent: f32) -> String {
        // A window that holds anything is never written as empty.
        if percent > 0.0 && percent < 0.5 {
            return "<1%".into();
        }
        format!("{}%", percent.round() as i32)
    }

    /// Cells each share takes of `total`: its part of the window, and never
    /// none for a share that holds anything.
    fn cells(&self, total: f32) -> Vec<f32> {
        let used: u64 = self.shares.iter().map(|share| share.tokens).sum();
        let window = self.capacity.max(used).max(1) as f32;
        let mut left = total;
        self.shares
            .iter()
            .map(|share| {
                let cells = if share.tokens == 0 {
                    0.0
                } else {
                    (share.tokens as f32 / window * total).round().max(1.0).min(left)
                };
                left -= cells;
                cells
            })
            .collect()
    }
}

impl Component for ContextWindow {
    fn render(self) -> Element {
        self.card().render()
    }
}

impl ContextWindow {
    fn card(self) -> FramedCard {
        let palette = self.palette;
        let id = self.id.as_str();
        let inner = px(self.width - INSET * 2.0);
        let sans = |text: &str, size: f32, ink: Color| {
            Element::text(text)
                .font_family(FontFamily::Sans)
                .font_size(size)
                .foreground(ink)
        };
        let used: u64 = self.shares.iter().map(|share| share.tokens).sum();
        let focus = self
            .focused
            .as_deref()
            .and_then(|focused| self.shares.iter().position(|share| share.id == focused));
        let focused = focus.map(|index| &self.shares[index]);

        let mut header = vec![sans(&self.title, 13.0, palette.text).centered_y(), Element::spacer().flex(1.0)];
        if let Some(badge) = &self.badge {
            header.push(
                Element::row(vec![Element::text(badge)
                    .font_family(FontFamily::Monospace)
                    .font_size(11.0)
                    .foreground(palette.muted)
                    .centered_y()])
                .padding(6.0)
                .background(palette.hover)
                .radius(control_radius())
                .centered_y(),
            );
        }

        for (action, icon, label) in &self.header_icons {
            header.push(
                palette
                    .interactive_surface(Element::icon_button(action, *icon, label))
                    .foreground(palette.muted)
                    .size(Some(24.0), Some(24.0))
                    .centered_y(),
            );
        }

        // The figure is the window's, or the share's in focus.
        let fraction = if self.capacity == 0 { 0.0 } else { used as f32 / self.capacity as f32 };
        let percent_ink = if fraction >= 0.95 {
            self.tones.danger
        } else if fraction >= 0.8 {
            self.tones.warning
        } else {
            palette.muted
        };
        let caption = focused.map_or(self.caption.as_str(), |share| share.label.as_str());
        let figure_text = |element: Element| {
            element
                .font_family(FontFamily::SansMedium)
                .font_size(28.0)
                .foreground(palette.text)
        };
        let mut figure = vec![
            // A share that was not measured has no count to lend the figure.
            if focused.is_some_and(|share| !share.measured) {
                figure_text(Element::text(UNMEASURED))
            } else {
                figure_text(Element::rolling_number(
                    format!("{id}:figure"),
                    focused.map_or(used, |share| share.tokens) as f32,
                    self.format,
                ))
            },
            Ticker::new(
                format!("{id}:caption"),
                focused.map_or("", |share| share.id.as_str()),
                sans(caption, 13.0, palette.muted).size(None, Some(18.0)),
            )
            .height(18.0)
            .render()
            .flex(1.0)
            .centered_y(),
        ];
        if self.capacity > 0 {
            figure.push(
                Element::rolling_number(format!("{id}:percent"), fraction * 100.0, Self::percent_text)
                    .font_family(FontFamily::Sans)
                    .font_size(13.0)
                    .foreground(percent_ink)
                    .animate_foreground(format!("{id}:percent:ink"))
                    .centered_y(),
            );
        }

        let cell = ((inner - f32::from(COLUMNS - 1) * GAP) / f32::from(COLUMNS)).floor().max(2.0);
        let cells = self.cells(f32::from(COLUMNS * ROWS));
        let grid = Element::unit_grid(UnitGrid {
            id: format!("{id}:grid"),
            columns: COLUMNS,
            rows: ROWS,
            cell,
            gap: GAP,
            radius: (cell / 4.0).floor(),
            parts: self
                .shares
                .iter()
                .zip(cells)
                .map(|(share, cells)| UnitGridPart {
                    key: share.id.clone(),
                    cells,
                    color: share.color,
                })
                .collect(),
            empty: palette.hover,
            focus,
        });

        let mut card = FramedCard::new(palette);
        if self.flat {
            card = card.flat();
        }
        let mut card = card
            .width(self.width)
            .section(Element::row(header).gap(4.0).size(None, Some(28.0)))
            .section(Element::row(figure).gap(8.0).size(None, Some(40.0)))
            .section(grid)
            .space(8.0);
        for share in &self.shares {
            let row_id = format!("{id}:share:{}", share.id);
            let action_id = format!("{id}:action:{}", share.id);
            let open = !share.items.is_empty() && self.expanded.contains(&share.id);
            let receded = focus.is_some() && focused.is_some_and(|focused| focused.id != share.id);
            let mut label = vec![
                Element::spacer()
                    .size(Some(8.0), Some(8.0))
                    .background(share.color)
                    .radius(2.0)
                    .centered_y(),
                sans(&share.label, 13.0, if receded { palette.muted } else { palette.text }).centered_y(),
            ];
            if !share.items.is_empty() {
                label.push(
                    Element::icon(if open { Icon::ChevronDown } else { Icon::ChevronRight })
                        .foreground(palette.muted)
                        .size(Some(12.0), Some(12.0))
                        .centered_y(),
                );
            }
            label.push(Element::spacer().flex(1.0));
            let mut row = vec![Element::clickable(&row_id, Element::row(label).gap(8.0))
                .size(None, Some(ROW_HEIGHT))
                .flex(1.0)];
            match &share.action {
                Some(ContextShareAction::Offer(offer)) => row.push(
                    Element::clickable(
                        &action_id,
                        Element::row(vec![
                            Element::icon(Icon::Compact)
                                .foreground(palette.muted)
                                .size(Some(12.0), Some(12.0))
                                .centered_y(),
                            sans(offer, 12.0, palette.muted).centered_y(),
                        ])
                        .gap(5.0),
                    )
                    .hover_foreground(palette.text)
                    .size(None, Some(ROW_HEIGHT)),
                ),
                Some(ContextShareAction::Busy(busy)) => row.push(
                    Element::row(vec![
                        Spinner::new(format!("{action_id}:busy"), palette.muted)
                            .size(12.0)
                            .render()
                            .centered_y(),
                        sans(busy, 12.0, palette.muted).centered_y(),
                    ])
                    .gap(5.0),
                ),
                Some(ContextShareAction::Done(done)) => row.push(
                    Element::row(vec![
                        Element::icon(Icon::Check)
                            .foreground(self.tones.success)
                            .size(Some(12.0), Some(12.0))
                            .centered_y(),
                        sans(done, 12.0, self.tones.success).centered_y(),
                    ])
                    .gap(5.0)
                    .animate_enter(format!("{action_id}:done")),
                ),
                None => {}
            }
            row.push(
                if share.measured {
                    Element::rolling_number(format!("{id}:tokens:{}", share.id), share.tokens as f32, self.format)
                        .foreground(if receded { palette.muted } else { palette.text })
                } else {
                    Element::text(UNMEASURED).foreground(palette.muted)
                }
                .font_family(FontFamily::Sans)
                .font_size(13.0)
                .centered_y(),
            );
            let row = FramedCard::row(
                format!("{row_id}:surface"),
                Element::row(row).gap(10.0),
                [row_id.clone(), action_id.clone()],
                palette,
            );
            let items = open.then(|| {
                Element::column(
                    share
                        .items
                        .iter()
                        .map(|item| {
                            let row = Element::row(vec![
                                // Under the share's label, past its mark.
                                Element::spacer().size(Some(ROW_PAD + 6.0), None),
                                Element::truncated_text(&item.label)
                                    .font_family(FontFamily::Monospace)
                                    .font_size(12.0)
                                    .foreground(palette.text)
                                    .flex(1.0)
                                    .centered_y(),
                                Element::text((self.format)(item.tokens as f32))
                                    .font_family(FontFamily::Sans)
                                    .font_size(12.0)
                                    .foreground(palette.muted)
                                    .centered_y(),
                                palette
                                    .interactive_surface(Element::icon_button(
                                        format!("{id}:remove:{}:{}", share.id, item.id),
                                        Icon::Close,
                                        &item.label,
                                    ))
                                    .foreground(palette.muted)
                                    .size(Some(20.0), Some(20.0))
                                    .centered_y(),
                                Element::spacer().size(Some(ROW_PAD - 10.0), None),
                            ])
                            .gap(10.0)
                            .size(None, Some(ITEM_HEIGHT));
                            // A slot of its own, so a removed item closes where it was.
                            Element::column(vec![row])
                                .animate_height(format!("{id}:item:{}:{}", share.id, item.id), false)
                        })
                        .chain([Element::spacer().size(None, Some(4.0))])
                        .collect(),
                )
                .keep_leaving(format!("{id}:items:{}:list", share.id))
            });
            // An open share and what it lists read as one block.
            card = card.rows(FramedCard::block(format!("{id}:items:{}", share.id), row, items, palette));
        }

        let status_ink = if self.status_done { self.tones.success } else { palette.muted };
        // The words the status opens with read stronger, such as a share's name.
        let lead = self
            .status_lead
            .as_deref()
            .filter(|lead| !lead.is_empty() && self.status.starts_with(lead));
        let rest = &self.status[lead.map_or(0, str::len)..];
        let mut line = Vec::new();
        if let Some(lead) = lead {
            line.push(
                Element::truncated_text(lead)
                    .font_family(FontFamily::Sans)
                    .font_size(12.0)
                    .foreground(palette.text)
                    .size(None, Some(18.0)),
            );
        }
        line.push(
            Element::truncated_text(rest)
                .font_family(FontFamily::Sans)
                .font_size(12.0)
                .foreground(status_ink)
                .size(None, Some(18.0))
                .flex(1.0),
        );
        let mut footer = vec![
            Ticker::new(format!("{id}:status"), &self.status, Element::row(line))
                .height(18.0)
                .render()
                .flex(1.0)
                .centered_y(),
        ];
        if let Some((action, label)) = &self.footer_action {
            footer.push(
                palette
                    .interactive_surface(Element::clickable(
                        action,
                        Element::row(vec![
                            sans(label, 12.0, palette.text).centered_y(),
                            Element::icon(Icon::ArrowRight)
                                .foreground(palette.text)
                                .size(Some(12.0), Some(12.0))
                                .centered_y(),
                        ])
                        .gap(4.0),
                    ))
                    .padding(6.0)
                    .size(None, Some(26.0))
                    .centered_y(),
            );
        }
        card.foot(Element::row(footer).gap(6.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Frame, Rect, Size, UiTree};
    use std::time::{Duration, Instant};

    const SIZE: Size = Size::new(400.0, 600.0);
    const SYSTEM: Color = Color::rgba(40, 40, 40, 255);
    const CHAT: Color = Color::rgba(40, 90, 240, 255);
    const EMPTY: Color = Color::rgba(230, 230, 230, 255);

    fn palette() -> ComponentPalette {
        ComponentPalette {
            surface: Color::rgba(255, 255, 255, 255),
            raised: Color::rgba(250, 250, 250, 255),
            hover: EMPTY,
            selected: Color::rgba(220, 220, 220, 255),
            border: Color::rgba(200, 200, 200, 255),
            text: Color::rgba(20, 20, 20, 255),
            muted: Color::rgba(120, 120, 120, 255),
            shadow: Color::rgba(0, 0, 0, 40),
        }
    }

    fn card(chat: u64, focused: Option<&str>) -> Element {
        ContextWindow::new(
            "context",
            "Context window",
            vec![
                ContextShare::new("system", "System", 20_000, SYSTEM),
                ContextShare::new("chat", "Conversation", chat, CHAT),
            ],
            200_000,
            palette(),
            ContextTones { warning: SYSTEM, danger: SYSTEM, success: SYSTEM },
        )
        .caption("of 200K tokens")
        .focused(focused)
        .render()
    }

    fn has_text(frame: &Frame, wanted: &str) -> bool {
        frame
            .commands
            .iter()
            .any(|command| matches!(command, DrawCommand::Text { text, .. } if text == wanted))
    }

    /// The cells of the grid, in reading order.
    fn cells(frame: &Frame) -> Vec<(Rect, Color)> {
        let cells: Vec<_> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. } if rect.width == rect.height && rect.width < 8.0 => {
                    Some((*rect, *color))
                }
                _ => None,
            })
            .collect();
        assert_eq!(cells.len(), usize::from(COLUMNS * ROWS));
        cells
    }

    fn count(frame: &Frame, color: Color) -> usize {
        cells(frame).iter().filter(|(_, ink)| *ink == color).count()
    }

    #[test]
    fn each_share_takes_its_part_of_the_cells_and_the_rest_stay_empty() {
        let mut tree = UiTree::new(card(100_000, None));
        let frame = tree.layout(SIZE);
        // 216 cells for 200K: a tenth is 22 of them, a half 108.
        assert_eq!(count(&frame, SYSTEM), 22);
        assert_eq!(count(&frame, CHAT), 108);
        assert_eq!(count(&frame, EMPTY), 86);
        assert!(has_text(&frame, "120.0K") && has_text(&frame, "60%") && has_text(&frame, "of 200K tokens"));
        assert!(!tree.is_animating());
        // The cells stay inside the card's inset on both sides.
        let cells = cells(&frame);
        assert!(cells[0].0.x >= INSET && cells[35].0.x + cells[35].0.width <= 320.0 - INSET);
        assert_eq!(cells[0].0.y, cells[35].0.y);
        assert!(cells[36].0.y > cells[0].0.y);
    }

    #[test]
    fn a_share_that_shrinks_counts_down_and_gives_its_cells_up_from_the_end() {
        let start = Instant::now() - Duration::from_secs(10);
        let roll = Duration::from_millis(u64::from(crate::tokens().motion.move_ms) * 3);
        let mut tree = UiTree::new(card(100_000, None));
        tree.layout_at(SIZE, start);
        tree.set_root(card(40_000, None));
        tree.layout_at(SIZE, start);
        let frame = tree.layout_at(SIZE, start + roll / 3);
        // Midway the figures read values in between and the tail has emptied.
        assert!(!has_text(&frame, "120.0K") && !has_text(&frame, "60.0K"));
        let chat = count(&frame, CHAT);
        assert!(chat < 108 && chat > 43, "{chat}");
        let cells_now = cells(&frame);
        assert_eq!(cells_now[22].1, CHAT, "the share keeps its first cells");
        assert_eq!(cells_now[129].1, EMPTY, "and gives up its last ones");

        let frame = tree.layout_at(SIZE, start + roll * 2);
        assert_eq!(count(&frame, CHAT), 43);
        assert!(has_text(&frame, "60.0K") && has_text(&frame, "40.0K") && has_text(&frame, "30%"));
        assert!(!tree.is_animating());
    }

    #[test]
    fn a_share_in_focus_lends_the_figure_its_count_and_the_others_recede() {
        let start = Instant::now() - Duration::from_secs(10);
        let roll = Duration::from_millis(u64::from(crate::tokens().motion.move_ms) * 3);
        let mut tree = UiTree::new(card(100_000, None));
        tree.layout_at(SIZE, start);
        tree.set_root(card(100_000, Some("system")));
        tree.layout_at(SIZE, start);
        let frame = tree.layout_at(SIZE, start + roll * 2);
        assert!(has_text(&frame, "System") && !has_text(&frame, "of 200K tokens"));
        assert_eq!(count(&frame, SYSTEM), 22);
        assert_eq!(count(&frame, CHAT), 0, "the other share has receded");
        // The share's own row and the figure both read its count; the percentage stays the window's.
        let figures = frame
            .commands
            .iter()
            .filter(|command| matches!(command, DrawCommand::Text { text, .. } if text == "20.0K"))
            .count();
        assert_eq!(figures, 2);
        assert!(has_text(&frame, "60%"));
    }

    #[test]
    fn reduced_motion_shows_the_new_state_at_once() {
        let mut tree = UiTree::new(card(100_000, None));
        tree.set_reduced_motion(true);
        tree.layout(SIZE);
        tree.set_root(card(40_000, None));
        let frame = tree.layout(SIZE);
        assert_eq!(count(&frame, CHAT), 43);
        assert!(has_text(&frame, "60.0K"));
        assert!(!tree.is_animating());
    }

    #[test]
    fn rows_report_their_share_and_items_open_under_it() {
        assert_eq!(ContextWindow::share_under("context", "context:share:chat"), Some("chat"));
        assert_eq!(ContextWindow::share_under("context", "context:action:chat"), Some("chat"));
        assert_eq!(ContextWindow::share_under("context", "context:remove:files:a"), None);
        assert_eq!(ContextWindow::share_under("other", "context:share:chat"), None);
        let files = |expanded: bool| {
            ContextWindow::new(
                "context",
                "Context window",
                vec![ContextShare::new("files", "Files", 30_000, CHAT).items(vec![ContextItem {
                    id: "a".into(),
                    label: "schema.sql".into(),
                    tokens: 9_400,
                }])],
                200_000,
                palette(),
                ContextTones { warning: SYSTEM, danger: SYSTEM, success: SYSTEM },
            )
            .expanded(expanded.then_some("files"))
            .render()
        };
        let mut tree = UiTree::new(files(false));
        assert!(!has_text(&tree.layout(SIZE), "schema.sql"));
        tree.set_reduced_motion(true);
        tree.set_root(files(true));
        let frame = tree.layout(SIZE);
        assert!(has_text(&frame, "schema.sql") && has_text(&frame, "9.4K"));
        assert!(frame.regions.iter().any(|region| region.id == "context:remove:files:a"));
    }

    /// The colour of the first text that `wanted` accepts.
    fn ink(frame: &Frame, wanted: impl Fn(&str) -> bool) -> Option<Color> {
        frame.commands.iter().find_map(|command| match command {
            DrawCommand::Text { text, color, .. } if wanted(text) => Some(*color),
            _ => None,
        })
    }

    #[test]
    fn the_figure_is_set_in_a_heavier_face_and_the_percentage_fades_to_its_new_colour() {
        let start = Instant::now() - Duration::from_secs(10);
        let fade = Duration::from_millis(u64::from(crate::tokens().motion.move_ms));
        // 160K of 200K is the warning's threshold; the tones here are all SYSTEM.
        let mut tree = UiTree::new(card(140_000, None));
        let frame = tree.layout_at(SIZE, start);
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::Text { text, font_family: FontFamily::SansMedium, .. } if text == "160.0K"
        )));
        assert_eq!(ink(&frame, |text| text.ends_with('%')), Some(SYSTEM));
        tree.set_root(card(40_000, None));
        tree.layout_at(SIZE, start);
        let midway = ink(&tree.layout_at(SIZE, start + fade / 3), |text| text.ends_with('%')).unwrap();
        assert!(midway.r > SYSTEM.r && midway.r < palette().muted.r, "{midway:?}");
        let frame = tree.layout_at(SIZE, start + fade * 4);
        assert_eq!(ink(&frame, |text| text == "30%"), Some(palette().muted));
        assert!(!tree.is_animating());
    }

    #[test]
    fn an_item_that_is_removed_closes_where_it_was() {
        let files = |names: &[&str]| {
            ContextWindow::new(
                "context",
                "Context window",
                vec![ContextShare::new("files", "Files", 30_000, CHAT).items(
                    names
                        .iter()
                        .map(|name| ContextItem { id: (*name).into(), label: format!("{name}.sql"), tokens: 9_400 })
                        .collect(),
                )],
                200_000,
                palette(),
                ContextTones { warning: SYSTEM, danger: SYSTEM, success: SYSTEM },
            )
            .expanded(["files"])
            .status("left")
            .render()
        };
        let y = |frame: &Frame, wanted: &str| {
            frame.commands.iter().find_map(|command| match command {
                DrawCommand::Text { text, position, .. } if text == wanted => Some(position.y),
                _ => None,
            })
        };
        let start = Instant::now() - Duration::from_secs(10);
        let exit = Duration::from_millis(u64::from(crate::tokens().motion.exit_ms));
        let mut tree = UiTree::new(files(&["first", "second"]));
        let before = tree.layout_at(SIZE, start);
        let (second, foot) = (y(&before, "second.sql").unwrap(), y(&before, "left").unwrap());
        tree.set_root(files(&["second"]));
        tree.layout_at(SIZE, start);
        // Midway the removed row still shows, fading, and what follows has moved part of the way.
        let frame = tree.layout_at(SIZE, start + exit / 2);
        let leaving = ink(&frame, |text| text == "first.sql").expect("the removed row is still closing");
        assert!(leaving.a < palette().text.a, "{leaving:?}");
        let moving = y(&frame, "second.sql").unwrap();
        assert!(moving < second && moving > second - ITEM_HEIGHT, "{moving}");
        assert!(!frame.regions.iter().any(|region| region.id == "context:remove:files:first"));

        let frame = tree.layout_at(SIZE, start + exit * 3);
        assert!(!has_text(&frame, "first.sql"));
        assert_eq!(y(&frame, "second.sql"), Some(second - ITEM_HEIGHT));
        assert_eq!(y(&frame, "left"), Some(foot - ITEM_HEIGHT));
        assert!(!tree.is_animating());
        // With reduced motion the row is gone at once.
        let mut still = UiTree::new(files(&["first", "second"]));
        still.set_reduced_motion(true);
        still.layout(SIZE);
        still.set_root(files(&["second"]));
        assert!(!has_text(&still.layout(SIZE), "first.sql"));
    }

    #[test]
    fn an_unmeasured_share_is_listed_with_a_dash_and_takes_no_cells() {
        let card = |focused: Option<&str>| {
            ContextWindow::new(
                "context",
                "Context window",
                vec![
                    ContextShare::new("system", "System", 20_000, SYSTEM),
                    ContextShare::new("tools", "Tools", 5_000, CHAT).unmeasured(),
                ],
                200_000,
                palette(),
                ContextTones { warning: SYSTEM, danger: SYSTEM, success: SYSTEM },
            )
            .focused(focused)
            .render()
        };
        let mut tree = UiTree::new(card(None));
        tree.set_reduced_motion(true);
        let frame = tree.layout(SIZE);
        assert_eq!(count(&frame, CHAT), 0);
        assert!(has_text(&frame, "–") && has_text(&frame, "10%"));
        // In focus it lends the figure no count either.
        tree.set_root(card(Some("tools")));
        let frame = tree.layout(SIZE);
        let dashes = frame
            .commands
            .iter()
            .filter(|command| matches!(command, DrawCommand::Text { text, .. } if text == "–"))
            .count();
        assert_eq!(dashes, 2);
        assert!(has_text(&frame, "Tools") && !has_text(&frame, "0"));
    }

    #[test]
    fn counts_are_written_short() {
        for (tokens, text) in [(0.0, "0"), (950.0, "950"), (4_200.0, "4.2K"), (175_600.0, "175.6K"), (1_250_000.0, "1.25M")] {
            assert_eq!(ContextWindow::tokens_text(tokens), text);
        }
    }
}
