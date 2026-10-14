use super::{caption, row, section, Cx, Section};
use crate::app::State;
use limn::{
    AlertDialog, AlertTone, Button, ButtonVariant, Color, Component, ContextMenu, ContextMenuEntry,
    ContextMenuItem, DropdownList, DropdownMenu, DropdownMenuEntry, DropdownMenuItem, DropdownOption,
    Element, FontFamily, FormField, Frame, Icon, Modal, Point, Popover, TextField, Toast, TooltipSpec,
};

/// The area a secondary click opens the menu at a point on.
const POINT_AREA: &str = "overlay-point-area";

const LANGUAGES: [(&str, &str); 3] = [("rust", "Rust"), ("python", "Python"), ("typescript", "TypeScript")];

fn trigger(cx: &Cx, id: &str, label: &str) -> Element {
    Button::new(id, label, cx.palette).variant(ButtonVariant::Outline).render()
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let state = cx.state;
    let open = |group: &str| state.text(group) == "open";

    let popover = Popover::new(
        trigger(cx, "overlay-popover=open", "Open popover"),
        Element::column(vec![
            Element::text("Share link")
                .font_family(FontFamily::Sans)
                .font_size(13.0)
                .foreground(palette.text),
            caption(cx, "Anyone with the link can view the document."),
            Button::new("overlay-popover=copied", "Copy link", palette).render(),
        ])
        .gap(8.0)
        .padding(12.0),
        open("overlay-popover"),
        "overlay-popover=closed",
        palette,
    )
    .width(260.0)
    .render();

    let language = state.choice("overlay-language", LANGUAGES[0].0);
    let label = LANGUAGES.iter().find(|(id, _)| *id == language).map_or(LANGUAGES[0].1, |(_, label)| label);
    let list = DropdownList::new(
        Button::new("overlay-list=open", label, palette)
            .variant(ButtonVariant::Outline)
            .trailing_icon(Icon::ChevronDown)
            .render(),
        LANGUAGES
            .iter()
            .map(|(id, label)| {
                DropdownOption::new(format!("overlay-language={id}&overlay-list=closed"), *label)
                    .selected(*id == language)
            })
            .collect(),
        open("overlay-list"),
        "overlay-list=closed",
        palette,
    )
    .width(200.0)
    .render();

    let menu = DropdownMenu::new(
        trigger(cx, "overlay-menu=open", "Open menu"),
        vec![
            DropdownMenuEntry::Heading("File".into()),
            DropdownMenuItem::new("overlay-menu=rename", "Rename").icon(Icon::Edit).shortcut("F2").into(),
            DropdownMenuItem::new("overlay-menu=archive", "Archive").icon(Icon::Archive).into(),
            DropdownMenuItem::new("overlay-menu=pinned", "Pin").icon(Icon::Pin).disabled().into(),
            DropdownMenuEntry::Separator,
            DropdownMenuItem::new("overlay-menu=delete", "Delete")
                .icon(Icon::Trash)
                .shortcut("Del")
                .destructive(Color::rgb(229, 83, 75))
                .into(),
        ],
        open("overlay-menu"),
        "overlay-menu=closed",
        palette,
    )
    .width(220.0)
    .render();

    let context = ContextMenu::new(
        "overlay-context",
        Element::panel(caption(cx, "Right-click here"))
            .padding(24.0)
            .border(palette.border, 1.0)
            .radius(limn::tokens().radius.card),
        vec![
            ContextMenuEntry::Item(ContextMenuItem::new("overlay-context-action=copy", "Copy").icon(Icon::File).shortcut("Ctrl+C")),
            ContextMenuEntry::Item(ContextMenuItem::new("overlay-context-action=paste", "Paste").enabled(false)),
            ContextMenuEntry::Separator,
            ContextMenuEntry::Item(ContextMenuItem::new("overlay-context-action=delete", "Delete").icon(Icon::Trash)),
        ],
        palette,
    )
    .render();

    vec![
        section(cx, "Popover", vec![row(vec![popover])]),
        section(cx, "Dropdown list", vec![row(vec![list])]),
        section(cx, "Menu", vec![row(vec![menu]), caption(cx, &format!("Selected: {}", state.text("overlay-menu")))]),
        section(cx, "Menu items", vec![
            row(vec![
                DropdownMenu::new(
                    trigger(cx, "overlay-items=open", "Channel menu"),
                    item_entries(state, "overlay-items=closed"),
                    open("overlay-items"),
                    "overlay-items=closed",
                    palette,
                )
                .context_menu_style()
                .width(260.0)
                .render(),
            ]),
            caption(cx, &format!("Notifications: {}", state.choice("overlay-items-level", "mentions"))),
        ]),
        section(cx, "Context menu", vec![row(vec![context])]),
        section(cx, "Menu at a point", vec![point_menu(cx)]),
        section(cx, "Dialogs", vec![
            row(vec![
                trigger(cx, "overlay-modal=open", "Open modal"),
                trigger(cx, "overlay-alert=open", "Open confirmation"),
            ]),
            caption(cx, &format!("Confirmation: {}", state.text("overlay-alert"))),
        ]),
        section(cx, "Toast", vec![row(vec![
            trigger(cx, "overlay-toast=shown", "Show toasts"),
            // Its own ID: the toast's close button dispatches the same action.
            trigger(cx, "overlay-toast-hide", "Hide").dispatch_as("overlay-toast=hidden"),
        ])]),
    ]
}

/// Check boxes, radio items, two-line items, a reason, and submenus two deep.
fn item_entries(state: &State, close: &str) -> Vec<DropdownMenuEntry> {
    let level = state.choice("overlay-items-level", "mentions");
    let radio = |value: &str, label: &str| {
        DropdownMenuItem::new(format!("overlay-items-level={value}&{close}"), label)
            .radio(level == value)
            .into()
    };
    let snooze = |value: &str, label: &str| {
        DropdownMenuItem::new(format!("overlay-items-snooze={value}&{close}"), label).into()
    };
    vec![
        DropdownMenuItem::new(format!("overlay-items-unread&{close}"), "Show unread only")
            .checkbox(state.flag("overlay-items-unread"))
            .into(),
        DropdownMenuItem::new("overlay-items-muted", "Hide muted channels")
            .checkbox(true)
            .disabled()
            .into(),
        DropdownMenuItem::new("overlay-items-notify", "Notifications")
            .icon(Icon::Bell)
            .description(match level {
                "all" => "All messages",
                "nothing" => "Nothing",
                _ => "Only @mentions",
            })
            .submenu(vec![
                radio("all", "All messages"),
                radio("mentions", "Only @mentions"),
                radio("nothing", "Nothing"),
                DropdownMenuItem::new("overlay-items-level-off", "Server default")
                    .radio(false)
                    .disabled()
                    .into(),
            ])
            .into(),
        DropdownMenuItem::new("overlay-items-status", "Snooze")
            .submenu(vec![
                DropdownMenuItem::new("overlay-items-for", "For a while")
                    .submenu(vec![
                        snooze("15m", "For 15 minutes"),
                        snooze("1h", "For an hour"),
                        snooze("8h", "For 8 hours"),
                    ])
                    .into(),
                snooze("tomorrow", "Until tomorrow"),
            ])
            .into(),
        DropdownMenuEntry::Separator,
        DropdownMenuItem::new("overlay-items-edit", "Edit channel")
            .icon(Icon::Settings)
            .description("Needs the Manage Channels permission")
            .tooltip_with(TooltipSpec::new("Needs the Manage Channels permission"))
            .disabled()
            .into(),
    ]
}

/// A menu that opens where the secondary click was, with the same items.
fn point_menu(cx: &Cx) -> Element {
    let area = Element::clickable(POINT_AREA, caption(cx, "Right-click anywhere here"))
        .padding(24.0)
        .border(cx.palette.border, 1.0)
        .radius(limn::tokens().radius.card);
    let at = cx
        .state
        .text("overlay-point")
        .split_once(',')
        .and_then(|(x, y)| Some(Point::new(x.parse().ok()?, y.parse().ok()?)));
    let Some(at) = at else {
        return area;
    };
    DropdownMenu::new(
        area,
        item_entries(cx.state, "overlay-point=closed"),
        true,
        "overlay-point=closed",
        cx.palette,
    )
    .context_menu_style()
    .width(260.0)
    .at(at)
    .render()
}

/// A secondary click on the area of the menu at a point opens it there, as a
/// host does: it keeps the point and passes it to `DropdownMenu::at`.
pub fn secondary_click(state: &mut State, frame: &Frame, position: Point) -> bool {
    let inside = frame
        .regions
        .iter()
        .any(|region| region.id == POINT_AREA && region.rect.contains(position));
    if inside {
        state.set("overlay-point", &format!("{},{}", position.x, position.y));
    }
    inside
}
/// Wrap the window in the layers this page controls.
pub fn layers(cx: &Cx, base: Element) -> Element {
    let palette = cx.palette;
    let state = cx.state;
    let base = Modal::new(
        base,
        state.text("overlay-modal") == "open",
        "New project",
        "You can change the name later.",
        "overlay-modal=closed",
        palette,
    )
    .width(Modal::fit_width(cx.width, 420.0))
    .body(
        FormField::new(
            "Name",
            TextField::new("overlay-modal-name", state.text("overlay-modal-name"), "My project", palette)
                .render(),
            palette,
        )
        .hint("Letters, numbers, and hyphens only.")
        .render(),
    )
    .footer(Element::row(vec![
        Element::spacer().flex(1.0),
        Button::new("overlay-modal=cancelled", "Cancel", palette).variant(ButtonVariant::Outline).render(),
        Button::new("overlay-modal=created", "Create", palette).render(),
    ])
    .gap(8.0))
    .initial_focus("overlay-modal-name")
    .backdrop_dismiss(true)
    .render();
    let base = AlertDialog::new(
        base,
        state.text("overlay-alert") == "open",
        "Delete the project?",
        "This action cannot be undone.",
        "overlay-alert=confirmed",
        "overlay-alert=cancelled",
        palette,
    )
    .confirm_label("Delete")
    .cancel_label("Cancel")
    .destructive(Color::rgb(161, 67, 71))
    .render();
    let toasts = if state.text("overlay-toast") == "shown" {
        vec![
            Toast::new("Changes saved", palette)
                .description("limn/src/empty.rs")
                .tone(AlertTone::Success)
                .render(),
            Toast::new("Offline", palette)
                .description("Retrying in a few seconds.")
                .tone(AlertTone::Error)
                .dismissible("overlay-toast=hidden")
                .render(),
        ]
    } else if state.flag("button-clipboard-link") || state.flag("button-clipboard-label") {
        vec![Toast::new("Link copied", palette)
            .description("https://limn.dev/components/clipboard")
            .render()]
    } else {
        Vec::new()
    };
    Toast::layer(base, toasts, 16.0)
}
