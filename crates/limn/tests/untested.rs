//! The components that had no tests of their own, or almost none:
//! `Attachment`, `Avatar`, `ChartLegend`, `Link` and `MenuSeparator`.
//! What a click or a key on a `Link` reports is in `controls.rs`.

use std::sync::Arc;

use limn::{
    Access, Attachment, AttachmentOrientation, Avatar, ChartLegend, Color, Component, ComponentPalette,
    DrawCommand, Element, Frame, HitKind, Icon, Link, MenuSeparator, Point, RasterImage, RasterImageFormat, Rect,
    Role, Size, UiAction, UiEvent, UiTree,
};

/// Every character is this wide, at any size, so a test can say where text ends.
const CHAR: f32 = 6.0;
const NARROW: f32 = 90.0;
const WIDE: f32 = 900.0;
const LONG: &str = "a-name-that-goes-on-for-much-longer-than-any-card-is-wide.tar.gz";

fn palette() -> ComponentPalette {
    ComponentPalette::neutral_dark()
}

fn tree(element: Element) -> UiTree {
    let mut tree = UiTree::new(Element::column(vec![
        Element::row(vec![element, Element::spacer().flex(1.0)]),
        Element::spacer().flex(1.0),
    ]));
    tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * CHAR);
    tree
}

fn frame_at(element: Element, width: f32) -> Frame {
    tree(element).layout(Size::new(width, 240.0))
}

fn frame(element: Element) -> Frame {
    frame_at(element, 480.0)
}

/// The room the component takes: where a mark after it and a mark under it land.
fn extent(element: Element) -> (f32, f32) {
    let mark = |id: &str| {
        Element::spacer().size(Some(1.0), Some(1.0)).accessible(Access::new(Role::Label).id(id).name(id))
    };
    let frame = UiTree::new(Element::column(vec![Element::row(vec![element, mark("after")]), mark("under")]))
        .layout(Size::new(WIDE, 600.0));
    let rect = |id: &str| frame.access.iter().find(|node| node.id == id).unwrap_or_else(|| panic!("no mark {id}")).rect;
    (rect("after").x, rect("under").y)
}

/// Each text drawn, with where it starts.
fn texts(frame: &Frame) -> Vec<(String, Point)> {
    frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, position, .. } => Some((text.clone(), *position)),
            _ => None,
        })
        .collect()
}

fn fills(frame: &Frame, color: Color) -> Vec<Rect> {
    frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillRect { rect, color: fill, .. } if *fill == color => Some(*rect),
            _ => None,
        })
        .collect()
}

fn icons(frame: &Frame) -> Vec<Icon> {
    frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Icon { icon, .. } => Some(*icon),
            _ => None,
        })
        .collect()
}

fn images(frame: &Frame) -> Vec<(Rect, f32)> {
    frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::RasterImage { rect, radius, .. } => Some((*rect, *radius)),
            _ => None,
        })
        .collect()
}

fn buttons(frame: &Frame) -> Vec<&str> {
    frame.regions.iter().filter(|region| region.kind == HitKind::Button).map(|region| region.id.as_str()).collect()
}

/// The one thing of its kind that was drawn.
fn only<T: Clone + std::fmt::Debug>(drawn: Vec<T>) -> T {
    match drawn.as_slice() {
        [one] => one.clone(),
        _ => panic!("one, got {drawn:?}"),
    }
}

fn picture() -> RasterImage {
    RasterImage::new("a".repeat(64), RasterImageFormat::Png, 4, Arc::from("AAAA"), "A picture").unwrap()
}

/// Everything the frame draws, listens on or tells the platform is at real numbers.
fn is_made_of_real_numbers(name: &str, frame: &Frame) {
    let real = |rect: &Rect| [rect.x, rect.y, rect.width, rect.height].iter().all(|value| value.is_finite());
    assert!(!frame.commands.is_empty(), "{name} draws nothing");
    for command in &frame.commands {
        let fine = match command {
            DrawCommand::FillRect { rect, .. }
            | DrawCommand::StrokeRect { rect, .. }
            | DrawCommand::Icon { rect, .. }
            | DrawCommand::RasterImage { rect, .. }
            | DrawCommand::PushClip(rect) => real(rect),
            DrawCommand::Text { position, font_size, .. } => {
                position.x.is_finite() && position.y.is_finite() && font_size.is_finite()
            }
            _ => true,
        };
        assert!(fine, "{name}: {command:?}");
    }
    for region in &frame.regions {
        assert!(real(&region.rect), "{name}: region {} is at {:?}", region.id, region.rect);
    }
    for node in &frame.access {
        assert!(real(&node.rect), "{name}: node {} is at {:?}", node.id, node.rect);
    }
}

/// No text starts before `left` or ends after `right`.
fn text_stays_between(name: &str, frame: &Frame, left: f32, right: f32) {
    for (text, position) in texts(frame) {
        let end = position.x + text.chars().count() as f32 * CHAR;
        assert!(position.x >= left - 0.01 && end <= right + 0.01, "{name}: {text:?} runs from {} to {end}, outside {left} to {right}", position.x);
    }
}

fn click(tree: &mut UiTree, point: Point) -> Vec<UiAction> {
    let mut actions = tree.handle_event(UiEvent::PointerDown(point));
    actions.extend(tree.handle_event(UiEvent::PointerUp(point)));
    actions
}

fn middle_of(frame: &Frame, id: &str) -> Point {
    let rect = frame.regions.iter().find(|region| region.id == id).unwrap_or_else(|| panic!("no region {id}")).rect;
    Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
}

// Attachment

fn attachment() -> Attachment {
    Attachment::new("file", "notes.md", palette()).description("2 KB")
}

#[test]
fn an_attachment_lays_out_with_real_numbers_narrow_and_wide() {
    for width in [NARROW, WIDE] {
        for (name, element) in [
            ("a file", attachment().remove_action("remove").render()),
            ("an image", attachment().orientation(AttachmentOrientation::Vertical).remove_action("remove").render()),
            ("a preview", attachment().preview(picture()).preview_only().open_action("open").render()),
        ] {
            is_made_of_real_numbers(name, &frame_at(element, width));
        }
    }
}

#[test]
fn an_attachment_is_220_wide_and_as_tall_as_a_file_until_told_otherwise() {
    assert_eq!(extent(attachment().render()), (220.0, Attachment::FILE_HEIGHT));
}

#[test]
fn an_attachment_takes_the_width_it_is_given_and_never_under_80() {
    assert_eq!(extent(attachment().width(300.0).render()).0, 300.0);
    assert_eq!(extent(attachment().width(10.0).render()).0, 80.0);
    assert_eq!(extent(attachment().width(-40.0).render()).0, 80.0);
}

#[test]
fn an_attachment_ignores_a_width_that_is_not_a_number() {
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(extent(attachment().width(bad).render()).0, 220.0);
    }
}

#[test]
fn a_vertical_attachment_is_as_tall_as_an_image() {
    let vertical = attachment().orientation(AttachmentOrientation::Vertical).render();
    assert_eq!(extent(vertical), (220.0, Attachment::IMAGE_HEIGHT));
}

#[test]
fn a_preview_alone_is_a_square_of_its_width() {
    assert_eq!(extent(attachment().preview_only().width(120.0).render()), (120.0, 120.0));
}

#[test]
fn a_preview_alone_draws_no_caption() {
    assert!(texts(&frame(attachment().preview_only().render())).is_empty());
}

#[test]
fn an_orientation_given_after_preview_only_brings_the_caption_back() {
    let element = attachment().preview_only().orientation(AttachmentOrientation::Vertical).render();
    let drawn: Vec<String> = texts(&frame(element)).into_iter().map(|(text, _)| text).collect();
    assert_eq!(drawn, ["notes.md", "2 KB"]);
}

#[test]
fn an_attachment_draws_its_description_under_its_title() {
    let drawn = texts(&frame(attachment().render()));
    let [(title, title_at), (description, description_at)] = drawn.as_slice() else {
        panic!("two lines, got {drawn:?}");
    };
    assert_eq!((title.as_str(), description.as_str()), ("notes.md", "2 KB"));
    assert_eq!(title_at.x, description_at.x);
    assert!(description_at.y > title_at.y);
}

#[test]
fn a_long_title_is_cut_inside_the_card() {
    for width in [80.0, 120.0, 220.0, 320.0] {
        for (name, element) in [
            ("a file", Attachment::new("file", LONG, palette()).description(LONG).width(width).render()),
            ("a file with a remove button", Attachment::new("file", LONG, palette()).width(width).remove_action("remove").render()),
            (
                "an image",
                Attachment::new("file", LONG, palette()).orientation(AttachmentOrientation::Vertical).width(width).render(),
            ),
        ] {
            let frame = frame(element);
            text_stays_between(name, &frame, 0.0, width);
            assert!(texts(&frame).iter().all(|(text, _)| text != LONG), "{name} at {width} draws the whole title");
        }
    }
}

#[test]
fn a_remove_button_takes_room_from_the_caption() {
    let title = |attachment: Attachment| texts(&frame(attachment.render()))[0].0.chars().count();
    let long = || Attachment::new("file", LONG, palette());
    assert!(title(long().remove_action("remove")) < title(long()));
}

#[test]
fn an_attachment_shows_a_file_icon_until_it_is_given_another() {
    assert_eq!(icons(&frame(attachment().render())), [Icon::File]);
    assert_eq!(icons(&frame(attachment().icon(Icon::Folder).render())), [Icon::Folder]);
}

#[test]
fn a_preview_takes_the_place_of_the_icon() {
    let frame = frame(attachment().preview(picture()).render());
    assert!(icons(&frame).is_empty());
    let (rect, _) = only(images(&frame));
    assert_eq!((rect.width, rect.height), (40.0, 40.0));
}

#[test]
fn a_preview_alone_fills_the_card_but_for_its_inset() {
    let frame = frame(attachment().preview(picture()).preview_only().width(120.0).render());
    let (rect, _) = only(images(&frame));
    assert_eq!((rect.x, rect.y, rect.width, rect.height), (2.0, 2.0, 116.0, 116.0));
}

#[test]
fn an_attachment_without_actions_has_nothing_to_press() {
    let mut tree = tree(attachment().render());
    let frame = tree.layout(Size::new(480.0, 240.0));
    assert!(buttons(&frame).is_empty(), "{:?}", frame.regions);
    assert!(click(&mut tree, Point::new(110.0, 32.0)).is_empty());
}

#[test]
fn a_click_on_an_attachment_that_opens_reports_its_open_action() {
    let mut tree = tree(attachment().open_action("open").render());
    tree.layout(Size::new(480.0, 240.0));
    assert_eq!(click(&mut tree, Point::new(110.0, 32.0)), [UiAction::Clicked { id: "open".into() }]);
}

#[test]
fn a_click_on_the_remove_button_reports_the_remove_action_alone() {
    for orientation in [AttachmentOrientation::Horizontal, AttachmentOrientation::Vertical] {
        let element = attachment().orientation(orientation).open_action("open").remove_action("remove").render();
        let mut tree = tree(element);
        let frame = tree.layout(Size::new(480.0, 240.0));
        let remove = middle_of(&frame, "remove");
        assert_eq!(click(&mut tree, remove), [UiAction::Clicked { id: "remove".into() }], "{orientation:?}");
    }
}

#[test]
fn the_remove_button_of_a_vertical_attachment_is_in_its_top_far_corner() {
    let element = attachment().orientation(AttachmentOrientation::Vertical).remove_action("remove").render();
    let frame = frame(element);
    let rect = frame.regions.iter().find(|region| region.id == "remove" && region.kind == HitKind::Button).unwrap().rect;
    assert_eq!((rect.x, rect.y, rect.width, rect.height), (220.0 - 6.0 - 24.0, 6.0, 24.0, 24.0));
}

#[test]
fn a_preview_alone_keeps_its_remove_button() {
    let frame = frame(attachment().preview_only().remove_action("remove").render());
    assert_eq!(buttons(&frame), ["remove"]);
}

#[test]
fn help_describes_an_attachment_that_opens_in_place_of_its_description() {
    let described = |attachment: Attachment| {
        let frame = frame(attachment.open_action("open").render());
        frame.access.iter().find(|node| node.id == "open").unwrap().description.clone()
    };
    assert_eq!(described(attachment()).as_deref(), Some("2 KB"));
    assert_eq!(described(attachment().help("Markdown, edited today")).as_deref(), Some("Markdown, edited today"));
    assert_eq!(described(Attachment::new("file", "notes.md", palette())), None);
}

// Avatar

fn initial(name: &str) -> String {
    let drawn = texts(&frame(Avatar::new(name, palette()).render()));
    let [(text, _)] = drawn.as_slice() else { panic!("one letter, got {drawn:?}") };
    text.clone()
}

#[test]
fn an_avatar_lays_out_with_real_numbers_narrow_and_wide() {
    for width in [NARROW, WIDE] {
        is_made_of_real_numbers("initial", &frame_at(Avatar::new("Ana", palette()).render(), width));
        is_made_of_real_numbers("photo", &frame_at(Avatar::new("Ana", palette()).image(picture()).render(), width));
    }
}

#[test]
fn an_avatar_shows_the_first_letter_of_the_name_in_capitals() {
    assert_eq!(initial("ana"), "A");
    assert_eq!(initial("  émile"), "É");
}

#[test]
fn an_avatar_without_a_name_shows_a_question_mark() {
    assert_eq!(initial(""), "?");
    assert_eq!(initial("   "), "?");
}

#[test]
fn a_long_name_is_still_one_letter_inside_the_circle() {
    let frame = frame(Avatar::new(LONG, palette()).render());
    text_stays_between("avatar", &frame, 0.0, 32.0);
    assert_eq!(texts(&frame).len(), 1);
}

#[test]
fn an_avatar_is_a_square_of_32_until_it_is_given_a_size() {
    assert_eq!(extent(Avatar::new("Ana", palette()).render()), (32.0, 32.0));
    assert_eq!(extent(Avatar::new("Ana", palette()).size(64.0).render()), (64.0, 64.0));
}

#[test]
fn the_size_of_an_avatar_stays_between_16_and_256() {
    assert_eq!(extent(Avatar::new("Ana", palette()).size(2.0).render()), (16.0, 16.0));
    assert_eq!(extent(Avatar::new("Ana", palette()).size(-8.0).render()), (16.0, 16.0));
    assert_eq!(extent(Avatar::new("Ana", palette()).size(4000.0).render()), (256.0, 256.0));
}

#[test]
fn an_avatar_ignores_a_size_that_is_not_a_number() {
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(extent(Avatar::new("Ana", palette()).size(bad).render()), (32.0, 32.0));
    }
}

#[test]
fn the_circle_is_filled_with_the_selected_color_at_half_its_size() {
    let frame = frame(Avatar::new("Ana", palette()).size(48.0).render());
    let circle = frame.commands.iter().find_map(|command| match command {
        DrawCommand::FillRect { rect, color, radius } if *color == palette().selected => Some((*rect, *radius)),
        _ => None,
    });
    assert_eq!(circle, Some((Rect::new(0.0, 0.0, 48.0, 48.0), 24.0)));
}

#[test]
fn the_letter_is_in_the_middle_of_the_circle_and_grows_with_it() {
    for size in [32.0, 96.0] {
        let frame = frame(Avatar::new("Ana", palette()).size(size).render());
        let letter = frame.commands.iter().find_map(|command| match command {
            DrawCommand::Text { position, font_size, .. } => Some((position.x, *font_size)),
            _ => None,
        });
        assert_eq!(letter, Some(((size - CHAR) / 2.0, size * 0.42)));
    }
}

#[test]
fn a_photo_takes_the_place_of_the_letter_as_a_circle() {
    let frame = frame(Avatar::new("Ana", palette()).image(picture()).size(48.0).render());
    assert!(texts(&frame).is_empty());
    assert_eq!(images(&frame), [(Rect::new(0.0, 0.0, 48.0, 48.0), 24.0)]);
}

#[test]
fn an_avatar_listens_to_nothing() {
    let mut tree = tree(Avatar::new("Ana", palette()).render());
    let frame = tree.layout(Size::new(480.0, 240.0));
    assert!(frame.regions.is_empty(), "{:?}", frame.regions);
    assert!(click(&mut tree, Point::new(16.0, 16.0)).is_empty());
}

#[test]
fn an_avatar_is_decoration_until_the_host_names_it() {
    // Beside a person's name it would only be said twice.
    assert!(frame(Avatar::new("Ana", palette()).render()).access.is_empty());
    assert!(frame(Avatar::new("Ana", palette()).image(picture()).render()).access.is_empty());
    for avatar in [Avatar::new("Ana", palette()), Avatar::new("Ana", palette()).image(picture())] {
        let named = frame(avatar.render().accessible(Access::new(Role::Image).id("ana").name("Ana")));
        let node = named.access.iter().find(|node| node.id == "ana").expect("a node for the avatar");
        assert_eq!((node.role, node.name.as_str()), (Role::Image, "Ana"));
        assert_eq!((node.rect.width, node.rect.height), (32.0, 32.0));
    }
}

// ChartLegend

const SCALE: [Color; 3] = [Color::rgb(10, 20, 30), Color::rgb(40, 50, 60), Color::rgb(70, 80, 90)];

fn legend() -> Element {
    ChartLegend::new(SCALE, "Less", "More", palette()).render()
}

#[test]
fn a_legend_lays_out_with_real_numbers_narrow_and_wide() {
    for width in [NARROW, WIDE] {
        is_made_of_real_numbers("legend", &frame_at(legend(), width));
    }
}

#[test]
fn a_legend_is_16_tall() {
    assert_eq!(extent(legend()).1, 16.0);
}

#[test]
fn a_legend_draws_its_low_label_first_and_its_high_label_last() {
    let drawn = texts(&frame(legend()));
    let [(low, low_at), (high, high_at)] = drawn.as_slice() else { panic!("two labels, got {drawn:?}") };
    assert_eq!((low.as_str(), high.as_str()), ("Less", "More"));
    assert!(low_at.x < high_at.x);
}

#[test]
fn a_legend_draws_a_square_of_10_for_each_color_in_order_between_the_labels() {
    let frame = frame(legend());
    let labels = texts(&frame);
    let squares: Vec<Rect> = SCALE.iter().map(|color| {
        only(fills(&frame, *color))
    }).collect();
    for square in &squares {
        assert_eq!((square.width, square.height, square.y), (10.0, 10.0, 3.0));
    }
    assert!(squares.windows(2).all(|pair| pair[0].x + 10.0 + 5.0 == pair[1].x), "{squares:?}");
    assert!(labels[0].1.x + 4.0 * CHAR <= squares[0].x && squares[2].x + 10.0 <= labels[1].1.x);
}

#[test]
fn a_legend_without_colors_is_its_two_labels() {
    let frame = frame(ChartLegend::new(Vec::new(), "Less", "More", palette()).render());
    assert_eq!(texts(&frame).len(), 2);
    assert!(!frame.commands.iter().any(|command| matches!(command, DrawCommand::FillRect { .. })));
}

#[test]
fn a_legend_cuts_its_labels_to_the_width_it_is_given() {
    let held = Element::panel(ChartLegend::new(SCALE, LONG, LONG, palette()).render()).size(Some(NARROW), None);
    let frame = frame(held);
    let drawn = texts(&frame);
    assert!(drawn.iter().all(|(text, _)| text != LONG), "{drawn:?}");
    assert!(drawn.iter().any(|(text, _)| text.ends_with('…')), "{drawn:?}");
    // The squares stay whole and inside.
    for color in SCALE {
        let square = fills(&frame, color)[0];
        assert!(square.width == 10.0 && square.x + square.width <= NARROW, "{square:?}");
    }
    // With room, nothing is cut.
    let roomy: Vec<String> = texts(&self::frame(legend())).into_iter().map(|(text, _)| text).collect();
    assert_eq!(roomy, ["Less", "More"]);
}

#[test]
fn a_legend_listens_to_nothing() {
    assert!(frame(legend()).regions.is_empty());
}

// Link

fn link() -> Link {
    Link::new("docs", "Read the docs", palette())
}

fn drawn_label(frame: &Frame) -> (Color, f32) {
    frame
        .commands
        .iter()
        .find_map(|command| match command {
            DrawCommand::Text { color, font_size, .. } => Some((*color, *font_size)),
            _ => None,
        })
        .expect("a label")
}

#[test]
fn a_link_lays_out_with_real_numbers_narrow_and_wide() {
    for width in [NARROW, WIDE] {
        is_made_of_real_numbers("enabled", &frame_at(link().render(), width));
        is_made_of_real_numbers("disabled", &frame_at(link().disabled().render(), width));
    }
}

#[test]
fn a_link_hugs_its_label() {
    let frame = frame(link().render());
    let [region] = frame.regions.as_slice() else { panic!("one region, got {:?}", frame.regions) };
    assert_eq!((region.rect.x, region.rect.width), (0.0, "Read the docs".len() as f32 * CHAR));
    assert_eq!(texts(&frame)[0].1.x, 0.0);
}

#[test]
fn a_link_is_drawn_in_the_link_ink_of_its_palette_at_13() {
    assert_eq!(drawn_label(&frame(link().render())), (palette().link(), 13.0));
}

#[test]
fn a_link_takes_the_ink_it_is_given() {
    let ink = Color::rgb(200, 30, 60);
    assert_eq!(drawn_label(&frame(link().ink(ink).render())).0, ink);
}

#[test]
fn a_disabled_link_is_muted_whatever_ink_it_was_given() {
    let frame = frame(link().ink(Color::rgb(200, 30, 60)).disabled().render());
    assert_eq!(drawn_label(&frame).0, palette().muted);
}

#[test]
fn a_link_takes_the_font_size_it_is_given_enabled_or_not() {
    assert_eq!(drawn_label(&frame(link().font_size(18.0).render())).1, 18.0);
    assert_eq!(drawn_label(&frame(link().font_size(18.0).disabled().render())).1, 18.0);
}

#[test]
fn a_link_ignores_a_font_size_that_is_not_above_zero() {
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.0, -4.0] {
        assert_eq!(drawn_label(&frame(link().font_size(bad).render())).1, 13.0, "{bad}");
    }
}

#[test]
fn a_link_is_underlined_under_the_pointer_and_not_at_rest() {
    let mut tree = tree(link().render());
    let size = Size::new(480.0, 240.0);
    let rest = tree.layout(size);
    let point = middle_of(&rest, "docs");
    tree.handle_event(UiEvent::PointerMove(point));
    let hovered = tree.layout(size);
    assert_eq!(rest.commands.len() + 1, hovered.commands.len(), "{:?}", hovered.commands);
    let under = hovered.commands.iter().find(|command| !rest.commands.contains(command)).expect("an underline");
    let DrawCommand::FillRect { rect, color, .. } = under else { panic!("not a rule: {under:?}") };
    assert_eq!((rect.x, rect.width, *color), (0.0, "Read the docs".len() as f32 * CHAR, palette().link()));
}

#[test]
fn a_long_link_is_cut_to_the_width_it_is_given_enabled_or_not() {
    for (name, element) in [
        ("enabled", Link::new("docs", LONG, palette()).render()),
        ("disabled", Link::new("docs", LONG, palette()).disabled().render()),
    ] {
        let frame = frame(Element::panel(element).size(Some(NARROW), None));
        text_stays_between(name, &frame, 0.0, NARROW);
        let (label, _) = only(texts(&frame));
        assert!(label.ends_with('…') && LONG.starts_with(label.trim_end_matches('…')), "{name} draws {label:?}");
    }
}

#[test]
fn the_target_of_a_long_link_is_as_wide_as_what_it_is_given() {
    let frame = frame(Element::panel(Link::new("docs", LONG, palette()).render()).size(Some(NARROW), None));
    assert_eq!(frame.regions[0].rect.width, NARROW);
}

// MenuSeparator

const RULE: Color = Color::rgb(90, 120, 150);

#[test]
fn a_separator_lays_out_with_real_numbers_narrow_and_wide() {
    for width in [NARROW, WIDE] {
        is_made_of_real_numbers("separator", &frame_at(MenuSeparator::new(220.0, RULE).render(), width));
    }
}

#[test]
fn a_separator_is_as_wide_as_it_was_told() {
    let (_, height) = extent(MenuSeparator::new(220.0, RULE).render());
    assert_eq!(extent(MenuSeparator::new(220.0, RULE).render()), (220.0, height));
    assert_eq!(extent(MenuSeparator::new(64.0, RULE).render()), (64.0, height));
}

#[test]
fn a_separator_of_a_negative_width_takes_no_width() {
    assert_eq!(extent(MenuSeparator::new(-20.0, RULE).render()).0, 0.0);
}

#[test]
fn the_rule_is_one_pixel_of_the_given_color_in_the_middle_of_its_height() {
    let (_, height) = extent(MenuSeparator::new(220.0, RULE).render());
    let frame = frame(MenuSeparator::new(220.0, RULE).render());
    let rule = only(fills(&frame, RULE));
    assert_eq!((rule.height, rule.y), (1.0, (height - 1.0) / 2.0));
}

#[test]
fn the_rule_is_set_in_from_both_ends_by_the_space_above_it() {
    let frame = frame(MenuSeparator::new(220.0, RULE).render());
    let rule = only(fills(&frame, RULE));
    assert!(rule.y > 0.0);
    assert_eq!((rule.x, rule.width), (rule.y, 220.0 - 2.0 * rule.y));
}

#[test]
fn a_separator_listens_to_nothing_and_tells_the_platform_nothing() {
    let frame = frame(MenuSeparator::new(220.0, RULE).render());
    assert!(frame.regions.is_empty() && frame.access.is_empty());
}
