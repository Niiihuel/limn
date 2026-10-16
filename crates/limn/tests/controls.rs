//! One standard for every control that reports an action when it is used:
//! the pointer and the keyboard both reach it, and disabled it is passive
//! and stays where it was. A control that joins the library joins the table.

use limn::{
    Access, ActionRow, Alert, AlertDialog, Attachment, Breadcrumb, BreadcrumbItem, Button, ButtonVariant, Checkbox, Clipboard,
    Accordion, AccordionItem, Chip, NumberInput, Pagination, RadioGroup, RadioOption, Toggle, ToggleGroup, ToggleItem,
    Toolbar, ToolbarItem, ControlSize, SwitchVariant, Color, Swatch, SwatchPicker,
    Collapsible, Component, ComponentPalette, DatePicker, DictationButton, DictationState, DismissButton,
    DropdownList, DropdownMenu, DropdownMenuEntry, DropdownMenuItem, DropdownOption, Element, Empty, Frame,
    HitKind, Icon, IconButton, Key, Link, ListRow, Marker, MediaCard, MediaTile, MenuBar, MenuBarMenu, MenuHeader, MenuItem,
    MessageAction, MessageActions, NavigationAction, NavigationBar, NavigationWorkspace, Pager, PanelHeader,
    Point, RasterImage, RasterImageFormat, Rect, Role, SearchField, SectionHeader, SegmentedControl, Select, SelectVariant, Size, StatusBar, StatusBarItem, Step, Stepper, SuggestionItem, SuggestionList, Switch, Tab,
    TabItem, TabStrip, Table, TableColumn, TableRow, Tabs, TabsVariant, TimePicker, Toast, TooltipSpec, TreeGuides, TreeRow, UiAction, UiEvent,
    UiTree,
};

const WINDOW: Size = Size::new(480.0, 320.0);

fn palette() -> ComponentPalette {
    ComponentPalette::neutral_dark()
}

/// One fact a control is held to. A row names the ones it is known to break.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Check {
    Click,
    Enter,
    Space,
    TabReaches,
    DisabledHasNoRegion,
    DisabledIsSkippedByTab,
    DisabledIgnoresAClick,
    DisabledIgnoresEnter,
    DisabledKeepsItsPlace,
    Named,
    DisabledSaysSo,
}

struct Control {
    name: &'static str,
    /// Its hit region, its focus target and its node.
    id: &'static str,
    /// The ID of the action it reports: its own unless the row says otherwise.
    reports: &'static str,
    enabled: fn() -> Element,
    /// None where the type has no disabled state.
    disabled: Option<fn() -> Element>,
    known: &'static [Check],
}

fn control(name: &'static str, id: &'static str, enabled: fn() -> Element, disabled: fn() -> Element) -> Control {
    Control { name, id, reports: id, enabled, disabled: Some(disabled), known: &[] }
}

/// A control whose type has no disabled state.
fn always(name: &'static str, id: &'static str, enabled: fn() -> Element) -> Control {
    Control { name, id, reports: id, enabled, disabled: None, known: &[] }
}

impl Control {
    fn reports(mut self, id: &'static str) -> Self {
        self.reports = id;
        self
    }

    // No row needs it today; the next finding will.
    #[allow(dead_code)]
    fn known(mut self, known: &'static [Check]) -> Self {
        self.known = known;
        self
    }
}

fn anchor() -> Element {
    Button::new("anchor", "Open", palette()).render()
}

fn swatches() -> SwatchPicker {
    let swatches = vec![
        Swatch::new("it", Color::rgb(231, 76, 60), "Red"),
        Swatch::new("other", Color::rgb(52, 152, 219), "Blue"),
    ];
    SwatchPicker::new("accent", swatches, Some("other"), palette())
}

fn picture() -> RasterImage {
    RasterImage::new("a".repeat(64), RasterImageFormat::Png, 4, std::sync::Arc::from("AAAA"), "A picture").unwrap()
}

fn hint(title: &str) -> TooltipSpec {
    TooltipSpec::new(title)
}

fn controls() -> Vec<Control> {
    vec![
        control(
            "Button",
            "it",
            || Button::new("it", "Save", palette()).render(),
            || Button::new("it", "Save", palette()).disabled().render(),
        ),
        control(
            "Button with a picture",
            "it",
            || Button::new("it", "Join", palette()).image(picture()).render(),
            || Button::new("it", "Join", palette()).image(picture()).disabled().render(),
        ),
        control(
            "IconButton",
            "it",
            || IconButton::new("it", Icon::Search, palette()).accessibility_label("Search").render(),
            || IconButton::new("it", Icon::Search, palette()).accessibility_label("Search").disabled().render(),
        ),
        control(
            "Link",
            "it",
            || Link::new("it", "Read the docs", palette()).render(),
            || Link::new("it", "Read the docs", palette()).disabled().render(),
        ),
        control(
            "Switch",
            "it",
            || Switch::new("it", true, palette()).accessibility_label("Wi-Fi").render(),
            || Switch::new("it", true, palette()).accessibility_label("Wi-Fi").disabled().render(),
        ),
        control(
            "Switch, spacious on the accent",
            "it",
            || Switch::new("it", true, palette()).size(ControlSize::Spacious).variant(SwitchVariant::Accent).accessibility_label("Wi-Fi").render(),
            || {
                Switch::new("it", true, palette()).size(ControlSize::Spacious).variant(SwitchVariant::Accent).accessibility_label("Wi-Fi").disabled().render()
            },
        ),
        control(
            "Accordion header",
            "it",
            || Accordion::new("faq", vec![AccordionItem::new("it", "Billing", Element::spacer())], palette()).render(),
            || Accordion::new("faq", vec![AccordionItem::new("it", "Billing", Element::spacer()).disabled()], palette()).render(),
        ),
        control(
            "Pagination page",
            "pages:3",
            || Pagination::new("pages", 1, 5, palette()).render(),
            || Pagination::new("pages", 1, 5, palette()).disabled().render(),
        )
        .reports("pages=3"),
        control(
            "Chip remove button",
            "it",
            || Chip::new("Rust", palette()).remove_action("it").render(),
            || Chip::new("Rust", palette()).remove_action("it").disabled().render(),
        ),
        control(
            "Toolbar button",
            "it",
            || Toolbar::new("bar", palette()).group(vec![ToolbarItem::button("it", "Save")]).render(),
            || Toolbar::new("bar", palette()).group(vec![ToolbarItem::button("it", "Save").disabled()]).render(),
        ),
        control(
            "Checkbox",
            "it",
            || Checkbox::new("it", true, palette()).label("Remember me").render(),
            || Checkbox::new("it", true, palette()).label("Remember me").disabled().render(),
        ),
        control(
            "RadioGroup option",
            "it",
            || RadioGroup::new("plan", vec![RadioOption::new("it", "Team"), RadioOption::new("other", "Free")], Some("other"), palette()).render(),
            || {
                RadioGroup::new("plan", vec![RadioOption::new("it", "Team").disabled(), RadioOption::new("other", "Free")], Some("other"), palette())
                    .render()
            },
        ),
        control(
            "Toggle",
            "it",
            || Toggle::new("it", "Bold", true, palette()).render(),
            || Toggle::new("it", "Bold", true, palette()).disabled().render(),
        ),
        control(
            "ToggleGroup item",
            "it",
            || ToggleGroup::new("style", vec![ToggleItem::new("it", "Bold", false), ToggleItem::new("other", "Italic", true)], palette()).render(),
            || {
                ToggleGroup::new("style", vec![ToggleItem::new("it", "Bold", false).disabled(), ToggleItem::new("other", "Italic", true)], palette())
                    .render()
            },
        ),
        control(
            "SwatchPicker swatch",
            "it",
            || swatches().render(),
            || swatches().disabled().render(),
        ),
        control(
            "SwatchPicker custom swatch",
            "accent:custom",
            || {
                swatches()
                    .custom(Color::rgb(250, 240, 200), "Custom color")
                    .render()
            },
            || {
                swatches()
                    .custom(Color::rgb(250, 240, 200), "Custom color")
                    .disabled()
                    .render()
            },
        ),
        control(
            "NumberInput increment",
            "count:increment",
            || NumberInput::new("count", "3", palette()).render(),
            || NumberInput::new("count", "3", palette()).disabled().render(),
        ),
        control(
            "ListRow",
            "it",
            || ListRow::new("it", "Documents", palette()).width(240.0).render(),
            || ListRow::new("it", "Documents", palette()).width(240.0).disabled().render(),
        ),
        control(
            "MenuItem",
            "it",
            || MenuItem::new("it", "Rename", palette()).render(),
            || MenuItem::new("it", "Rename", palette()).disabled().render(),
        ),
        control(
            "MediaCard",
            "it",
            || MediaCard::new("it", "Kind of Blue", palette()).render(),
            || MediaCard::new("it", "Kind of Blue", palette()).disabled().render(),
        ),
        control(
            "MediaTile",
            "it",
            || MediaTile::new("it", "Kind of Blue", palette()).render(),
            || MediaTile::new("it", "Kind of Blue", palette()).disabled().render(),
        ),
        control(
            "DictationButton",
            "it",
            || DictationButton::new("it", DictationState::Idle, palette()).accessibility_label("Dictate").render(),
            || DictationButton::new("it", DictationState::Idle, palette()).accessibility_label("Dictate").disabled().render(),
        ),
        control(
            "Clipboard",
            "it",
            || Clipboard::new("it", "Copy link", palette()).render(),
            || Clipboard::new("it", "Copy link", palette()).disabled().render(),
        ),
        control(
            "DismissButton",
            "it",
            || DismissButton::new("it", palette()).render(),
            || DismissButton::new("it", palette()).disabled().render(),
        ),
        control(
            "Marker with an action",
            "it",
            || Marker::new("marker", "Show 4 more", palette()).action("it").render(),
            || Marker::new("marker", "Show 4 more", palette()).action("it").disabled().render(),
        ),
        control(
            "Collapsible header",
            "it",
            || Collapsible::new("it", "Details", Element::text("Body"), palette()).render(),
            || Collapsible::new("it", "Details", Element::text("Body"), palette()).disabled().render(),
        ),
        always("TreeRow", "it", || {
            TreeRow::new(
                |content| Element::clickable("it", content),
                Element::icon(Icon::Folder),
                Element::text("src"),
                TreeGuides::root(),
                palette(),
            )
            .width(240.0)
            .render()
        }),
        control(
            "Tabs tab",
            "it",
            || Tabs::new(vec![Tab::new("it", "Files", 80.0), Tab::new("other", "Search", 80.0)], palette()).render(),
            || {
                Tabs::new(vec![Tab::new("it", "Files", 80.0).disabled(), Tab::new("other", "Search", 80.0)], palette())
                    .render()
            },
        ),
        control(
            "Tabs tab, underlined",
            "it",
            || {
                Tabs::new(
                    vec![
                        Tab::new("it", "Files", 80.0),
                        Tab::new("other", "Search", 80.0),
                    ],
                    palette(),
                )
                .variant(TabsVariant::Underlined)
                .render()
            },
            || {
                Tabs::new(
                    vec![
                        Tab::new("it", "Files", 80.0).disabled(),
                        Tab::new("other", "Search", 80.0),
                    ],
                    palette(),
                )
                .variant(TabsVariant::Underlined)
                .render()
            },
        ),
        control(
            "SegmentedControl segment",
            "it",
            || {
                SegmentedControl::new(vec![Tab::new("it", "Day", 80.0), Tab::new("other", "Week", 80.0)], palette())
                    .render()
            },
            || {
                SegmentedControl::new(
                    vec![Tab::new("it", "Day", 80.0).disabled(), Tab::new("other", "Week", 80.0)],
                    palette(),
                )
                .render()
            },
        ),
        control(
            "TabStrip tab",
            "it",
            || {
                TabStrip::new("strip", vec![TabItem::new("it", "it:extent", Icon::File, "main.rs", 120.0)], palette())
                    .render()
            },
            || {
                TabStrip::new(
                    "strip",
                    vec![TabItem::new("it", "it:extent", Icon::File, "main.rs", 120.0).disabled()],
                    palette(),
                )
                .render()
            },
        ),
        control(
            "Stepper step",
            "it",
            || Stepper::new(vec![Step::new("it", "Account"), Step::new("other", "Billing")], 0, palette()).render(),
            || {
                Stepper::new(vec![Step::new("it", "Account").disabled(), Step::new("other", "Billing")], 0, palette())
                    .render()
            },
        ),
        control(
            "ActionRow with an action",
            "it",
            || ActionRow::new("Notifications", palette()).action("it").render(),
            || ActionRow::new("Notifications", palette()).action("it").disabled().render(),
        ),
        control(
            "Pager previous",
            "it",
            || Pager::new("3 / 12", palette()).previous("it", true).next("other", true).tooltips(hint("Previous"), hint("Next")).render(),
            || Pager::new("3 / 12", palette()).previous("it", false).next("other", true).tooltips(hint("Previous"), hint("Next")).render(),
        ),
        control(
            "Pager next",
            "it",
            || Pager::new("3 / 12", palette()).previous("other", true).next("it", true).tooltips(hint("Previous"), hint("Next")).render(),
            || Pager::new("3 / 12", palette()).previous("other", true).next("it", false).tooltips(hint("Previous"), hint("Next")).render(),
        ),
        control(
            "Breadcrumb segment",
            "it",
            || {
                Breadcrumb::new("crumbs", vec![BreadcrumbItem::new("Home").action("it"), BreadcrumbItem::new("limn")], palette())
                    .render()
            },
            || {
                Breadcrumb::new(
                    "crumbs",
                    vec![BreadcrumbItem::new("Home").action("it").disabled(), BreadcrumbItem::new("limn")],
                    palette(),
                )
                .render()
            },
        ),
        always("StatusBar item", "it", || {
            StatusBar::new(vec![StatusBarItem::new("it", 80.0, palette()).label("main")], Vec::new(), palette()).render()
        }),
        control(
            "MenuBar menu",
            "it",
            || MenuBar::new(vec![MenuBarMenu::new("it", "File", 60.0, "it:dismiss", Vec::new())], None, palette()).render(),
            || {
                MenuBar::new(vec![MenuBarMenu::new("it", "File", 60.0, "it:dismiss", Vec::new()).disabled()], None, palette())
                    .render()
            },
        ),
        control(
            "Select trigger",
            "it",
            || Select::new("it", "Bash", vec![DropdownOption::new("bash", "Bash")], false, "dismiss", palette()).render(),
            || {
                Select::new("it", "Bash", vec![DropdownOption::new("bash", "Bash")], false, "dismiss", palette())
                    .disabled()
                    .render()
            },
        ),
        control(
            "Select trigger as a field",
            "it",
            || {
                Select::new(
                    "it",
                    "Bash",
                    vec![DropdownOption::new("bash", "Bash")],
                    false,
                    "dismiss",
                    palette(),
                )
                .variant(SelectVariant::Field)
                .render()
            },
            || {
                Select::new(
                    "it",
                    "Bash",
                    vec![DropdownOption::new("bash", "Bash")],
                    false,
                    "dismiss",
                    palette(),
                )
                .variant(SelectVariant::Field)
                .disabled()
                .render()
            },
        ),
        // Of a fixed width: without one an enabled field fills its row and a
        // disabled one hugs its text, which moves the button with it.
        control(
            "SearchField clear button",
            "it:clear",
            || {
                SearchField::new("it", "rust", "Search", palette())
                    .clearable(true)
                    .width(240.0)
                    .render()
            },
            || {
                SearchField::new("it", "rust", "Search", palette())
                    .clearable(true)
                    .width(240.0)
                    .disabled()
                    .render()
            },
        ),
        control(
            "DropdownList option",
            "it",
            || DropdownList::new(anchor(), vec![DropdownOption::new("it", "Bash")], true, "dismiss", palette()).render(),
            || {
                DropdownList::new(anchor(), vec![DropdownOption::new("it", "Bash").disabled()], true, "dismiss", palette())
                    .render()
            },
        ),
        control(
            "DropdownMenu item",
            "it",
            || {
                let entries = vec![DropdownMenuEntry::Item(DropdownMenuItem::new("it", "Rename"))];
                DropdownMenu::new(anchor(), entries, true, "dismiss", palette()).render()
            },
            || {
                let entries = vec![DropdownMenuEntry::Item(DropdownMenuItem::new("it", "Rename").disabled())];
                DropdownMenu::new(anchor(), entries, true, "dismiss", palette()).render()
            },
        ),
        control(
            "DropdownMenu check box item",
            "it",
            || {
                let entries = vec![DropdownMenuItem::new("it", "Mute").checkbox(true).into()];
                DropdownMenu::new(anchor(), entries, true, "dismiss", palette()).render()
            },
            || {
                let entries = vec![
                    DropdownMenuItem::new("it", "Mute")
                        .checkbox(true)
                        .disabled()
                        .into(),
                ];
                DropdownMenu::new(anchor(), entries, true, "dismiss", palette()).render()
            },
        ),
        control(
            "DropdownMenu radio item",
            "it",
            || {
                let entries = vec![
                    DropdownMenuItem::new("it", "Only mentions")
                        .radio(false)
                        .into(),
                ];
                DropdownMenu::new(anchor(), entries, true, "dismiss", palette())
                    .context_menu_style()
                    .render()
            },
            || {
                let entries = vec![
                    DropdownMenuItem::new("it", "Only mentions")
                        .radio(false)
                        .disabled()
                        .into(),
                ];
                DropdownMenu::new(anchor(), entries, true, "dismiss", palette())
                    .context_menu_style()
                    .render()
            },
        ),
        control(
            "DropdownMenu two-line item",
            "it",
            || {
                let entries = vec![
                    DropdownMenuItem::new("it", "Notifications")
                        .description("Only mentions")
                        .into(),
                ];
                DropdownMenu::new(anchor(), entries, true, "dismiss", palette()).render()
            },
            || {
                let entries = vec![
                    DropdownMenuItem::new("it", "Notifications")
                        .description("Needs a permission")
                        .disabled()
                        .into(),
                ];
                DropdownMenu::new(anchor(), entries, true, "dismiss", palette()).render()
            },
        ),
        control(
            "DropdownMenu item at a point",
            "it",
            || {
                let entries = vec![DropdownMenuItem::new("it", "Mark as read").into()];
                DropdownMenu::new(anchor(), entries, true, "dismiss", palette())
                    .context_menu_style()
                    .at(Point::new(60.0, 40.0))
                    .render()
            },
            || {
                let entries = vec![
                    DropdownMenuItem::new("it", "Mark as read")
                        .disabled()
                        .into(),
                ];
                DropdownMenu::new(anchor(), entries, true, "dismiss", palette())
                    .context_menu_style()
                    .at(Point::new(60.0, 40.0))
                    .render()
            },
        ),
        control(
            "DatePicker day",
            "date:day:2026-10-13",
            || DatePicker::new("date", "2026-10-13", palette()).render(),
            || DatePicker::new("date", "2026-10-13", palette()).disabled().render(),
        ),
        control(
            "DatePicker month arrow",
            "date:month_next",
            || DatePicker::new("date", "2026-10-13", palette()).render(),
            || DatePicker::new("date", "2026-10-13", palette()).disabled().render(),
        ),
        control(
            "TimePicker arrow",
            "time:hour_up",
            || TimePicker::new("time", "11:06", palette()).render(),
            || TimePicker::new("time", "11:06", palette()).disabled().render(),
        ),
        control(
            "MessageActions action",
            "it",
            || MessageActions::new("message", palette()).action(MessageAction::new("it", Icon::Copy, "Copy")).render(),
            || {
                MessageActions::new("message", palette())
                    .action(MessageAction::new("it", Icon::Copy, "Copy").disabled("Nothing to copy"))
                    .render()
            },
        ),
        // The bar derives the target's ID from its own and reports the host's:
        // `dispatch_as` in navigation_bar.rs.
        control(
            "NavigationBar workspace",
            "nav:workspace:it",
            || NavigationBar::new("nav", palette()).workspaces(vec![NavigationWorkspace::new("it", "1")]).render(),
            || {
                NavigationBar::new("nav", palette())
                    .workspaces(vec![NavigationWorkspace::new("it", "1").enabled(false)])
                    .render()
            },
        )
        .reports("it"),
        control(
            "NavigationBar action",
            "nav:action:it",
            || {
                NavigationBar::new("nav", palette())
                    .actions(vec![NavigationAction::new("it", Icon::Search, "Search")])
                    .render()
            },
            || {
                NavigationBar::new("nav", palette())
                    .actions(vec![NavigationAction::new("it", Icon::Search, "Search").enabled(false)])
                    .render()
            },
        )
        .reports("it"),
        always("Attachment that opens", "it", || {
            Attachment::new("file", "notes.md", palette()).open_action("it").render()
        }),
        always("Attachment remove button", "it", || {
            Attachment::new("file", "notes.md", palette()).remove_action("it").render()
        }),
        always("Toast with an action", "it", || Toast::new("Saved", palette()).action("it").render()),
        always("Toast close button", "it", || Toast::new("Saved", palette()).dismissible("it").render()),
        always("Alert close button", "it", || Alert::new("Offline", "Changes are kept here.", palette()).dismiss("it").render()),
        always("Empty action", "it", || {
            Empty::new("No files", "Nothing has been added.", palette()).action("it", "Add a file", ButtonVariant::Default).render()
        }),
        always("PanelHeader back arrow", "it", || PanelHeader::new("Files", palette()).back("it", "Back").render()),
        always("PanelHeader close button", "it", || PanelHeader::new("Files", palette()).dismiss("it", "Close").render()),
        always("Table header that sorts", "table:sort:name", || {
            Table::new("table", vec![TableColumn::fixed("name", "Name", 160.0).sortable()], 320.0, palette()).render()
        }),
        always("Table row", "it", || {
            Table::new("table", vec![TableColumn::fixed("name", "Name", 160.0)], 320.0, palette())
                .rows(vec![TableRow::new("it", vec![Element::text("main.rs")])])
                .render()
        }),
        always("SectionHeader with an action", "it", || SectionHeader::new("Recent", palette()).action("it").render()),
        always("MenuHeader back arrow", "it", || MenuHeader::new(palette()).title("Projects").back("it", "Back").render()),
        control(
            "AlertDialog confirm",
            "it",
            || AlertDialog::new(Element::spacer(), true, "Delete it?", "It cannot be undone.", "it", "cancel", palette()).render(),
            || {
                AlertDialog::new(Element::spacer(), true, "Delete it?", "It cannot be undone.", "it", "cancel", palette())
                    .confirm_enabled(false)
                    .render()
            },
        ),
        always("AlertDialog cancel", "it", || {
            AlertDialog::new(Element::spacer(), true, "Delete it?", "It cannot be undone.", "confirm", "it", palette()).render()
        }),
        control(
            "SuggestionList item",
            "it",
            || {
                SuggestionList::new(anchor(), "list", vec![SuggestionItem::new("it", "main.rs")], 0, true, "dismiss", palette())
                    .render()
            },
            || {
                let items = vec![SuggestionItem::new("it", "main.rs").disabled()];
                SuggestionList::new(anchor(), "list", items, 0, true, "dismiss", palette()).render()
            },
        ),
    ]
}

/// The control at the top left of a window, with room around it.
fn stage(element: Element) -> UiTree {
    let mut tree = UiTree::new(Element::column(vec![
        Element::row(vec![element, Element::spacer().flex(1.0)]),
        Element::spacer().flex(1.0),
    ]));
    tree.layout(WINDOW);
    tree
}

fn key(key: Key) -> UiEvent {
    UiEvent::Key { key, shift: false, ctrl: false }
}

fn region(frame: &Frame, id: &str) -> Option<Rect> {
    frame.regions.iter().find(|region| region.id == id && region.kind == HitKind::Button).map(|region| region.rect)
}

/// The middle of the control's hit region.
fn middle(control: &Control) -> Result<Point, String> {
    let tree = stage((control.enabled)());
    let frame = tree.frame();
    let rect = region(frame, control.id).ok_or_else(|| {
        let ids: Vec<_> = frame.regions.iter().map(|region| region.id.as_str()).collect();
        format!("no button region {}, among {ids:?}", control.id)
    })?;
    let point = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
    match frame.hit_test(point) {
        Some(id) if id == control.id => Ok(point),
        other => Err(format!("the middle of its region is {other:?}")),
    }
}

fn click(tree: &mut UiTree, point: Point) -> Vec<UiAction> {
    let mut actions = tree.handle_event(UiEvent::PointerDown(point));
    actions.extend(tree.handle_event(UiEvent::PointerUp(point)));
    actions
}

fn one_click_of(control: &Control, actions: Vec<UiAction>) -> Result<(), String> {
    if actions == [UiAction::Clicked { id: control.reports.to_owned() }] {
        Ok(())
    } else {
        Err(format!("reported {actions:?}"))
    }
}

fn nothing(actions: Vec<UiAction>) -> Result<(), String> {
    if actions.is_empty() {
        Ok(())
    } else {
        Err(format!("reported {actions:?}"))
    }
}

/// What a key does with the keyboard focus on the control.
fn pressed(control: &Control, pressed: Key) -> Result<(), String> {
    let mut tree = stage((control.enabled)());
    if !tree.focus(control.id) {
        return Err("it does not take the focus".into());
    }
    one_click_of(control, tree.handle_event(key(pressed)))
}

/// Whether Tab, pressed from nothing, ever stops on the ID.
fn tab_reaches(element: Element, id: &str) -> bool {
    let mut tree = stage(element);
    (0..64).any(|_| {
        tree.handle_event(key(Key::Tab));
        tree.layout(WINDOW);
        tree.focused_id() == Some(id)
    })
}

/// The room the whole component takes, as where a mark after it and a mark
/// under it land, and where the platform is told the control is.
fn place(element: Element, id: &str) -> (f32, f32, Option<Rect>) {
    let mark = |id: &str| {
        Element::spacer().size(Some(1.0), Some(1.0)).accessible(Access::new(Role::Label).id(id).name(id))
    };
    let frame = UiTree::new(Element::column(vec![Element::row(vec![element, mark("after")]), mark("under")])).layout(WINDOW);
    let rect = |id: &str| frame.access.iter().find(|node| node.id == id).map(|node| node.rect);
    (rect("after").expect("the mark after it").x, rect("under").expect("the mark under it").y, rect(id))
}

fn said(element: Element, id: &str, disabled: bool) -> Result<(), String> {
    let tree = stage(element);
    let nodes = &tree.frame().access;
    let Some(node) = nodes.iter().find(|node| node.id == id) else {
        let ids: Vec<_> = nodes.iter().map(|node| node.id.as_str()).collect();
        return Err(format!("no node {id}, among {ids:?}"));
    };
    if node.name.is_empty() {
        return Err("its node has no name".into());
    }
    if node.disabled != disabled {
        return Err(format!("its node says disabled is {}", node.disabled));
    }
    Ok(())
}

fn verdict(control: &Control, check: Check) -> Result<(), String> {
    let disabled = || control.disabled.map(|build| build());
    match check {
        Check::Click => {
            let point = middle(control)?;
            let mut tree = stage((control.enabled)());
            one_click_of(control, click(&mut tree, point))
        }
        Check::Enter => pressed(control, Key::Enter),
        Check::Space => pressed(control, Key::Space),
        Check::TabReaches => {
            if tab_reaches((control.enabled)(), control.id) {
                Ok(())
            } else {
                Err("Tab never stops on it".into())
            }
        }
        Check::DisabledHasNoRegion => {
            let Some(element) = disabled() else { return Ok(()) };
            match region(stage(element).frame(), control.id) {
                Some(_) => Err("it keeps its button region".into()),
                None => Ok(()),
            }
        }
        Check::DisabledIsSkippedByTab => {
            let Some(element) = disabled() else { return Ok(()) };
            if tab_reaches(element, control.id) {
                Err("Tab stops on it".into())
            } else {
                Ok(())
            }
        }
        Check::DisabledIgnoresAClick => {
            let Some(element) = disabled() else { return Ok(()) };
            let point = middle(control)?;
            nothing(click(&mut stage(element), point))
        }
        Check::DisabledIgnoresEnter => {
            let Some(element) = disabled() else { return Ok(()) };
            let mut tree = stage(element);
            if tree.focus(control.id) {
                return Err("it takes the focus".into());
            }
            // A dialog keeps the focus on another of its buttons, and Enter is that button's.
            let elsewhere = tree.focused_id().is_some_and(|focused| focused != control.id);
            let actions = tree.handle_event(key(Key::Enter));
            if elsewhere && !actions.contains(&UiAction::Clicked { id: control.reports.to_owned() }) {
                return Ok(());
            }
            nothing(actions)
        }
        Check::DisabledKeepsItsPlace => {
            let Some(element) = disabled() else { return Ok(()) };
            let (enabled, disabled) = (place((control.enabled)(), control.id), place(element, control.id));
            if enabled == disabled {
                Ok(())
            } else {
                Err(format!("enabled it is at {enabled:?}, disabled at {disabled:?}"))
            }
        }
        Check::Named => said((control.enabled)(), control.id, false),
        Check::DisabledSaysSo => match disabled() {
            Some(element) => said(element, control.id, true),
            None => Ok(()),
        },
    }
}

/// Hold every row to one fact. A row marked as known to break it must still
/// break it: a mark that outlives its reason is as wrong as a failure.
fn every_control(check: Check) {
    let mut wrong = Vec::new();
    for control in controls() {
        match (verdict(&control, check), control.known.contains(&check)) {
            (Err(why), false) => wrong.push(format!("{}: {why}", control.name)),
            (Ok(()), true) => wrong.push(format!("{}: marked as known and it holds now; remove the mark", control.name)),
            _ => {}
        }
    }
    assert!(wrong.is_empty(), "{check:?}\n{}", wrong.join("\n"));
}

#[test]
fn a_click_reports_one_action_with_its_id() {
    every_control(Check::Click);
}

#[test]
fn enter_on_the_focused_control_reports_one_action() {
    every_control(Check::Enter);
}

#[test]
fn space_on_the_focused_control_reports_one_action() {
    every_control(Check::Space);
}

#[test]
fn tab_reaches_every_control() {
    every_control(Check::TabReaches);
}

#[test]
fn a_disabled_control_has_no_button_region() {
    every_control(Check::DisabledHasNoRegion);
}

#[test]
fn tab_does_not_stop_on_a_disabled_control() {
    every_control(Check::DisabledIsSkippedByTab);
}

#[test]
fn a_click_where_a_disabled_control_is_reports_nothing() {
    every_control(Check::DisabledIgnoresAClick);
}

#[test]
fn enter_reports_nothing_after_focus_is_asked_for_a_disabled_control() {
    every_control(Check::DisabledIgnoresEnter);
}

#[test]
fn a_disabled_control_takes_the_room_it_takes_enabled() {
    every_control(Check::DisabledKeepsItsPlace);
}

#[test]
fn every_control_has_a_named_node() {
    every_control(Check::Named);
}

#[test]
fn a_disabled_control_has_a_named_node_that_says_so() {
    every_control(Check::DisabledSaysSo);
}

/// The harness can fail: a control that swallows the keyboard, has no name
/// and moves when disabled is caught by each check it breaks.
#[test]
fn the_checks_catch_a_control_that_breaks_them() {
    let broken = Control {
        name: "broken",
        id: "it",
        reports: "it",
        enabled: || Element::clickable("it", Element::spacer().size(Some(40.0), Some(20.0))),
        disabled: Some(|| Element::clickable("it", Element::spacer().size(Some(60.0), Some(20.0)))),
        known: &[],
    };
    for check in [Check::Click, Check::Enter, Check::Space, Check::TabReaches] {
        assert_eq!(verdict(&broken, check), Ok(()), "{check:?}");
    }
    for check in [
        Check::DisabledHasNoRegion,
        Check::DisabledIsSkippedByTab,
        Check::DisabledIgnoresAClick,
        Check::DisabledIgnoresEnter,
        Check::DisabledKeepsItsPlace,
        Check::Named,
        Check::DisabledSaysSo,
    ] {
        assert!(verdict(&broken, check).is_err(), "{check:?} let it through");
    }
}
