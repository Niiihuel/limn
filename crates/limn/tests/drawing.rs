//! `Element::drawing`: what the application draws itself, placed and clipped
//! by the tree.

use std::sync::{Arc, Mutex};

use limn::{Access, Color, DrawCommand, Element, Rect, Role, Size, UiTree};

const INK: Color = Color::rgb(10, 20, 30);

fn fill(rect: Rect) -> DrawCommand {
    DrawCommand::FillRect { rect, color: INK, radius: 0.0 }
}

/// The commands the drawing left in the frame, from its own clip to the end.
fn drawn(drawing: Element) -> Vec<DrawCommand> {
    let window = Element::column(vec![Element::spacer().size(None, Some(20.0)), drawing]);
    let frame = UiTree::new(window).layout(Size::new(300.0, 200.0));
    let start = frame.commands.iter().position(|command| matches!(command, DrawCommand::PushClip(_))).expect("the drawing's clip");
    frame.commands[start..].to_vec()
}

#[test]
fn a_drawing_is_placed_and_clipped_inside_its_padding() {
    let given = Arc::new(Mutex::new(None));
    let seen = given.clone();
    let drawing = Element::drawing(move |size| {
        *seen.lock().unwrap() = Some(size);
        vec![fill(Rect::new(0.0, 0.0, size.width, size.height))]
    })
    .size(Some(100.0), Some(40.0))
    .padding(5.0);
    let content = Rect::new(5.0, 25.0, 90.0, 30.0);
    assert_eq!(drawn(drawing), vec![DrawCommand::PushClip(content), fill(content), DrawCommand::PopClip]);
    assert_eq!(*given.lock().unwrap(), Some(Size::new(90.0, 30.0)));
}

#[test]
fn a_drawing_cannot_leave_a_clip_open_or_close_one_it_did_not_open() {
    let drawing = Element::drawing(|_| {
        vec![
            DrawCommand::PopClip,
            DrawCommand::PushClip(Rect::new(0.0, 0.0, 10.0, 10.0)),
            fill(Rect::new(0.0, 0.0, 50.0, 50.0)),
        ]
    })
    .size(Some(100.0), Some(40.0));
    assert_eq!(
        drawn(drawing),
        vec![
            DrawCommand::PushClip(Rect::new(0.0, 20.0, 100.0, 40.0)),
            DrawCommand::PushClip(Rect::new(0.0, 20.0, 10.0, 10.0)),
            fill(Rect::new(0.0, 20.0, 50.0, 50.0)),
            DrawCommand::PopClip,
            DrawCommand::PopClip,
        ]
    );
}

#[test]
fn a_drawing_given_no_height_takes_none_and_the_width_it_is_stretched_to() {
    let frame = UiTree::new(Element::column(vec![Element::drawing(|_| Vec::new())])).layout(Size::new(300.0, 200.0));
    let clip = frame.commands.iter().find(|command| matches!(command, DrawCommand::PushClip(_)));
    assert_eq!(clip, Some(&DrawCommand::PushClip(Rect::new(0.0, 0.0, 300.0, 0.0))));
}

#[test]
fn a_named_drawing_tells_the_platform_what_it_shows() {
    let drawing = Element::drawing(|_| Vec::new())
        .size(Some(100.0), Some(40.0))
        .accessible(Access::new(Role::Image).id("waveform").name("Waveform"));
    let frame = UiTree::new(Element::column(vec![drawing])).layout(Size::new(300.0, 200.0));
    let node = frame.access.iter().find(|node| node.id == "waveform").expect("a node for the drawing");
    assert_eq!((node.role, node.name.as_str()), (Role::Image, "Waveform"));
}
