//! The gallery's owned page texture, rendered before the window pass.
use eframe::egui;
use limn::{ComponentPalette, DrawCommand, Element, Frame, Rect, Size, UiTree};

pub fn background(p: ComponentPalette, size: Size, phase: f32) -> Frame {
    let mut frame = UiTree::new(Element::column((0..8).map(|i| {
        Element::text(format!("{}   AURORA  /  Moving type and color", i + 1)).font_size(24.0)
            .foreground(p.text).size(None, Some(48.0))
    }).collect()).padding(24.0)).layout(size);
    let mut commands = vec![DrawCommand::FillVerticalGradient { rect: Rect::new(0.0, 0.0, size.width, size.height), top: p.raised, bottom: p.surface }];
    for i in 0..12 {
        let x = (i as f32 * 76.0 + phase) % (size.width + 76.0) - 76.0;
        commands.push(DrawCommand::FillRect { rect: Rect::new(x, 0.0, 32.0, size.height), color: if i % 2 == 0 { p.link() } else { p.selected }, radius: 0.0 });
    }
    commands.append(&mut frame.commands);
    frame.commands = commands;
    frame
}

pub fn paint(renderer: &mut limn_egui::GlassRenderer, ui: &egui::Ui, frame: &Frame, origin: egui::Pos2, p: ComponentPalette, phase: f32) {
    let area = frame.commands.iter().find_map(|command| match command {
        DrawCommand::Surface { rect, id, .. } if id == "glass:backdrop" => Some(*rect), _ => None,
    });
    let Some(area) = area else { limn_egui::paint_at(ui, frame, origin); return; };
    let scale = ui.ctx().pixels_per_point();
    let size = [(area.width * scale).ceil() as u32, (area.height * scale).ceil() as u32];
    let background = background(p, Size::new(area.width, area.height), phase);
    if renderer.set_backdrop_frame(ui.ctx(), &background, size, scale) {
        let rect = egui::Rect::from_min_size(origin + egui::vec2(area.x, area.y), egui::vec2(area.width, area.height));
        renderer.paint_at_surface(ui, frame, origin, rect, "glass:backdrop");
    } else { limn_egui::paint_at(ui, frame, origin); }
}
