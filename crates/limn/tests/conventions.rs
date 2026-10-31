//! What every component is held to, checked across all of them at once. A
//! component that joins a family joins its row here.

use limn::{
    ActionRow, BreadcrumbItem, Button, ButtonVariant, Checkbox, Clipboard, Collapsible, Component, ComponentPalette,
    Accordion, AccordionItem, Chip, NumberInput, Pagination, RadioGroup, RadioOption, Toggle, ToggleGroup, ToggleItem,
    Toolbar, ToolbarItem, ColorPicker, Hsv, SwatchPicker,
    Composer, ContextMenuItem, DatePicker, DateTimePicker, DictationButton, DictationState, DismissButton,
    ContextMenuMetrics, DropdownMenu, DropdownMenuEntry, DropdownMenuItem, DropdownOption, Element, Icon, IconButton, Link, ListRow, Markdown, Marker, MediaCard, MediaTile,
    MenuBarMenu, MenuItem, MenuSeparator, NavigationAction, NavigationWorkspace, ResizeHandle, SearchField,
    SegmentedControl, Select, Size, Slider, Step, SuggestionItem, Switch, Tab, TabItem, Tabs, TabsVariant, TextArea,
    TextField, TimePicker, Toast, TreeGuides, TreeRow, UiTree, Point,
};

fn palette() -> ComponentPalette {
    ComponentPalette::neutral_dark()
}

/// Lay an element out and require every rectangle it draws or listens on to
/// be made of real numbers.
fn lays_out_with_real_numbers(name: &str, element: Element) {
    let frame = UiTree::new(element).layout(Size::new(480.0, 320.0));
    for region in &frame.regions {
        let rect = region.rect;
        assert!(
            [rect.x, rect.y, rect.width, rect.height].iter().all(|value| value.is_finite()),
            "{name}: region {} is at {rect:?}",
            region.id
        );
    }
}

/// Anything that can be disabled says so both ways: `disabled()` for the
/// fixed case and `enabled(bool)` for a state the host computes.
#[test]
fn what_can_be_disabled_takes_both_spellings() {
    let palette = palette();
    // Each line would not compile without both.
    let _ = Button::new("a", "A", palette).enabled(false).disabled();
    let _ = IconButton::new("a", Icon::Search, palette).enabled(false).disabled();
    let _ = TextField::new("a", "", "", palette).enabled(false).disabled();
    let _ = TextArea::new("a", "", "", palette).enabled(false).disabled();
    let _ = SearchField::new("a", "", "", palette).enabled(false).disabled();
    let _ = Switch::new("a", true, palette).enabled(false).disabled();
    let _ = Checkbox::new("a", true, palette).enabled(false).disabled();
    let _ = RadioGroup::new("a", Vec::new(), None, palette).enabled(false).disabled();
    let _ = RadioOption::new("a", "A").enabled(false).disabled();
    let _ = SwatchPicker::new("a", Vec::new(), None, palette).enabled(false).disabled();
    let _ = ColorPicker::new("a", Hsv::new(0.0, 1.0, 1.0), palette).enabled(false).disabled();
    let _ = Toggle::new("a", "A", true, palette).enabled(false).disabled();
    let _ = ToggleGroup::new("a", Vec::new(), palette).enabled(false).disabled();
    let _ = ToggleItem::new("a", "A", true).enabled(false).disabled();
    let _ = NumberInput::new("a", "1", palette).enabled(false).disabled();
    let _ = Accordion::new("a", Vec::new(), palette).enabled(false).disabled();
    let _ = AccordionItem::new("a", "A", Element::spacer()).enabled(false).disabled();
    let _ = Pagination::new("a", 1, 3, palette).enabled(false).disabled();
    let _ = Chip::new("A", palette).enabled(false).disabled();
    let _ = Toolbar::new("a", palette).enabled(false).disabled();
    let _ = ToolbarItem::button("a", "A").enabled(false).disabled();
    let _ = Slider::new("a", 0.5, palette).enabled(false).disabled();
    let _ = Select::new("a", "A", Vec::new(), false, "a:dismiss", palette).enabled(false).disabled();
    let _ = ListRow::new("a", "A", palette).enabled(false).disabled();
    let _ = MenuItem::new("a", "A", palette).enabled(false).disabled();
    let _ = MediaCard::new("a", "A", palette).enabled(false).disabled();
    let _ = MediaTile::new("a", "A", palette).enabled(false).disabled();
    let _ = DictationButton::new("a", DictationState::Idle, palette).enabled(false).disabled();
    let _ = Tab::new("a", "A", 80.0).enabled(false).disabled();
    let _ = DropdownOption::new("a", "A").enabled(false).disabled();
    let _ = DropdownMenuItem::new("a", "A").enabled(false).disabled();
    let _ = ContextMenuItem::new("a", "A").enabled(false).disabled();
    let _ = NavigationWorkspace::new("a", "A").enabled(false);
    let _ = NavigationAction::new("a", Icon::Search, "A").enabled(false);
    let _ = Link::new("a", "A", palette).enabled(false).disabled();
    let _ = Clipboard::new("a", "Copy", palette).enabled(false).disabled();
    let _ = DismissButton::new("a", palette).enabled(false).disabled();
    let _ = Marker::new("a", "A", palette).enabled(false).disabled();
    let _ = Collapsible::new("a", "A", Element::spacer(), palette).enabled(false).disabled();
    let _ = DatePicker::new("a", "2026-10-13", palette).enabled(false).disabled();
    let _ = TimePicker::new("a", "11:06", palette).enabled(false).disabled();
    let _ = DateTimePicker::new("a", "b", "2026-10-13", "11:06", palette).enabled(false).disabled();
    let _ = Step::new("a", "A").enabled(false).disabled();
    let _ = ActionRow::new("A", palette).enabled(false).disabled();
    let _ = ResizeHandle::vertical("a", palette).enabled(false).disabled();
    let _ = TabItem::new("a", "a:extent", Icon::Folder, "A", 120.0).enabled(false).disabled();
    let _ = MenuBarMenu::new("a", "A", 60.0, "a:dismiss", Vec::new()).enabled(false).disabled();
    let _ = SuggestionItem::new("a", "A").enabled(false).disabled();
    let _ = BreadcrumbItem::new("A").enabled(false).disabled();
    let _ = Composer::new("a", "", "", 320.0, palette).enabled(false).disabled();
}

/// The two spellings mean the same: a disabled control takes no pointer.
#[test]
fn a_disabled_control_reports_nothing_whichever_way_it_was_said() {
    let palette = palette();
    for (name, element) in [
        ("Slider::disabled", Slider::new("s", 0.5, palette).disabled().render()),
        ("Slider::enabled(false)", Slider::new("s", 0.5, palette).enabled(false).render()),
        ("ListRow::disabled", ListRow::new("s", "Row", palette).disabled().render()),
        ("ListRow::enabled(false)", ListRow::new("s", "Row", palette).enabled(false).render()),
        ("MenuItem::disabled", MenuItem::new("s", "Item", palette).disabled().render()),
        ("MenuItem::enabled(false)", MenuItem::new("s", "Item", palette).enabled(false).render()),
    ] {
        assert!(press(element).is_empty(), "{name} reported an action");
    }
    // The same press on the same controls, enabled, is heard: the test presses where they are.
    assert!(!press(ListRow::new("s", "Row", palette).render()).is_empty());
    assert!(!press(MenuItem::new("s", "Item", palette).render()).is_empty());
    assert!(!press(Slider::new("s", 0.5, palette).render()).is_empty());
}

/// Press and release in the middle of the window, where a lone control is.
fn press(element: Element) -> Vec<limn::UiAction> {
    let window = Size::new(320.0, 80.0);
    let mut tree = UiTree::new(Element::panel(element.centered()).size(Some(window.width), Some(window.height)));
    tree.layout(window);
    let middle = limn::Point::new(window.width / 2.0, window.height / 2.0);
    let mut actions = tree.handle_event(limn::UiEvent::PointerDown(middle));
    actions.extend(tree.handle_event(limn::UiEvent::PointerUp(middle)));
    actions
}

/// A size that is not a number is ignored or brought to a floor; it never
/// reaches the layout.
#[test]
fn a_size_that_is_not_a_number_never_reaches_the_layout() {
    let palette = palette();
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -1.0] {
        lays_out_with_real_numbers("Tab::new", Tabs::new(vec![Tab::new("t", "Tab", bad)], palette).height(bad).render());
        lays_out_with_real_numbers(
            "Tabs::gap",
            Tabs::new(
                vec![Tab::new("t", "Tab", 80.0), Tab::new("u", "Tab", 80.0)],
                palette,
            )
            .variant(TabsVariant::Underlined)
            .gap(bad)
            .fill_width(bad)
            .render(),
        );
        lays_out_with_real_numbers(
            "SegmentedControl::height",
            SegmentedControl::new(vec![Tab::new("t", "Tab", 80.0)], palette).height(bad).render(),
        );
        lays_out_with_real_numbers("MenuSeparator::new", MenuSeparator::new(bad, palette.border).render());
        lays_out_with_real_numbers(
            "TreeRow::width",
            TreeRow::new(|content| Element::clickable("row", content), Element::spacer(), Element::text("Row"), TreeGuides::root(), palette)
                .width(bad)
                .height(bad)
                .render(),
        );
        lays_out_with_real_numbers(
            "Toast::layer",
            Toast::layer(Element::spacer(), vec![Toast::new("Saved", palette).width(bad).render()], bad),
        );
        lays_out_with_real_numbers("Button::width", Button::new("b", "B", palette).variant(ButtonVariant::Outline).width(bad).render());
        lays_out_with_real_numbers("Slider::width", Slider::new("s", 0.5, palette).width(bad).render());
        lays_out_with_real_numbers("SearchField::height", SearchField::new("s", "", "S", palette).height(bad).render());
        lays_out_with_real_numbers(
            "Markdown::font_size",
            Markdown::new("# a\n\n- [b](https://example.test/b)", palette)
                .font_size(bad)
                .render()
                .element,
        );
        lays_out_with_real_numbers(
            "Select::height",
            Select::new("s", "S", Vec::new(), false, "s:dismiss", palette)
                .height(bad)
                .render(),
        );
        lays_out_with_real_numbers(
            "Select::height as a field",
            Select::new("s", "S", Vec::new(), false, "s:dismiss", palette)
                .variant(limn::SelectVariant::Field)
                .width(bad)
                .height(bad)
                .render(),
        );
        lays_out_with_real_numbers("ColorPicker::width", ColorPicker::new("c", Hsv::new(bad, bad, bad), palette).width(bad).render());
        let menu = || DropdownMenu::new(Element::spacer(), vec![DropdownMenuItem::new("i", "I").into()], true, "d", palette);
        lays_out_with_real_numbers("DropdownMenu::at", menu().at(Point::new(bad, 40.0)).render());
        lays_out_with_real_numbers("DropdownMenu::at", menu().at(Point::new(40.0, bad)).render());
        let metrics = ContextMenuMetrics {
            row_height: bad,
            label_size: bad,
            detail_size: bad,
            inset: bad,
            horizontal_padding: bad,
            gap: bad,
            separator_height: bad,
            scrollbar_gutter: bad,
            header_height: bad,
            max_height: bad,
        };
        lays_out_with_real_numbers(
            "DropdownMenu::context_metrics",
            DropdownMenu::new(
                Element::spacer(),
                vec![
                    DropdownMenuItem::new("i", "I").into(),
                    DropdownMenuEntry::Separator,
                    DropdownMenuItem::new("j", "J").into(),
                ],
                true,
                "d",
                palette,
            )
            .context_menu_style()
            .context_metrics(metrics)
            .render(),
        );
    }
}

/// What can be scrolled with the wheel can be scrolled from the keyboard.
#[test]
fn page_down_and_page_up_scroll_what_the_wheel_scrolls() {
    use limn::{Key, ScrollArea, UiEvent};
    let palette = palette();
    let rows: Vec<Element> = (0..60).map(|row| Button::new(format!("row-{row}"), format!("Row {row}"), palette).render()).collect();
    let window = Size::new(320.0, 240.0);
    let mut tree = UiTree::new(ScrollArea::new("list", Element::column(rows)).render().size(Some(window.width), Some(window.height)));
    let key = |key| UiEvent::Key { key, shift: false, ctrl: false, alt: false };
    let settle = |tree: &mut UiTree| {
        // A smooth scroll arrives over a few frames.
        let start = web_time::Instant::now();
        for frame in 0..40 {
            tree.layout_at(window, start + web_time::Duration::from_millis(16 * frame));
        }
        tree.scroll_offset("list")
    };
    tree.layout(window);
    assert_eq!(tree.scroll_offset("list"), 0.0);
    tree.handle_event(key(Key::PageDown));
    let down = settle(&mut tree);
    assert!(down > window.height * 0.5 && down < window.height, "a page down went {down}");
    tree.handle_event(key(Key::PageDown));
    assert!(settle(&mut tree) > down);
    tree.handle_event(key(Key::PageUp));
    tree.handle_event(key(Key::PageUp));
    assert_eq!(settle(&mut tree), 0.0);
}

/// The builders every component is made of hold the same line: a number that
/// is not finite changes nothing.
#[test]
fn an_element_ignores_numbers_that_are_not_finite() {
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let set = Element::spacer().padding(4.0).gap(5.0).flex(2.0).radius(3.0).min_size(6.0, 7.0).size(Some(8.0), Some(9.0));
        let kept = set.style;
        let after = set.padding(bad).gap(bad).flex(bad).radius(bad).min_size(bad, bad).size(Some(bad), Some(bad)).style;
        assert_eq!(
            (after.padding, after.gap, after.flex, after.radius, after.min_width, after.min_height, after.width, after.height),
            (kept.padding, kept.gap, kept.flex, kept.radius, kept.min_width, kept.min_height, kept.width, kept.height),
            "{bad}"
        );
    }
    let cleared = Element::spacer().size(Some(8.0), Some(9.0)).size(None, Some(-1.0)).style;
    assert_eq!((cleared.width, cleared.height), (None, Some(0.0)));
}

#[test]
fn the_last_border_said_is_the_one_drawn() {
    let color = limn::Color::WHITE;
    assert!(Element::spacer().border(color, 1.0).dashed_border(color, 1.0).style.dashed_border);
    assert!(!Element::spacer().dashed_border(color, 1.0).border(color, 1.0).style.dashed_border);
    let none = Element::spacer().dashed_border(color, f32::NAN).style;
    assert!(none.border.is_none() && !none.dashed_border);
}

/// Disabling a button does not move what it says.
#[test]
fn a_disabled_button_keeps_its_label_where_the_enabled_one_has_it() {
    let palette = palette();
    let label_at = |button: Button| {
        let frame = UiTree::new(button.render()).layout(Size::new(320.0, 120.0));
        frame
            .commands
            .iter()
            .find_map(|command| match command {
                limn::DrawCommand::Text { position, .. } => Some(position.x),
                _ => None,
            })
            .expect("a label")
    };
    for width in [80.0, 160.0, 240.0] {
        let button = || Button::new("a", "Save", palette).width(width);
        assert!((label_at(button()) - label_at(button().disabled())).abs() < 1.0, "at {width}");
    }
}
