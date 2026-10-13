//! A real limn page rendered once, then sampled by the optical playground.

use eframe::{egui, egui_wgpu, wgpu};
use limn::{Color, Element, FontFamily, Icon, Size, UiTree};

const SIZE: [u32; 2] = [1536, 960];
const INK: Color = Color::rgb(28, 36, 55);

fn text(value: &str, size: f32, color: Color) -> Element {
    Element::text(value)
        .font_family(FontFamily::Sans)
        .font_size(size)
        .foreground(color)
}

fn page() -> Element {
    let apps = [
        ("Files", Icon::Folder, Color::rgb(45, 116, 238)),
        ("Photos", Icon::Image, Color::rgb(222, 73, 128)),
        ("Music", Icon::Audio, Color::rgb(146, 84, 225)),
        ("Messages", Icon::Chat, Color::rgb(30, 167, 115)),
        ("Ideas", Icon::Bolt, Color::rgb(230, 145, 34)),
        ("Settings", Icon::Settings, Color::rgb(92, 111, 139)),
    ];
    let tiles = apps
        .into_iter()
        .map(|(label, icon, fill)| {
            Element::column(vec![
                Element::panel(
                    Element::icon(icon)
                        .large_icon(29.0)
                        .foreground(Color::WHITE),
                )
                .background(fill)
                .radius(17.0)
                .center_content()
                .size(Some(68.0), Some(60.0)),
                text(label, 13.0, INK),
            ])
            .gap(9.0)
            .flex(1.0)
        })
        .collect();
    let article = Element::column(vec![
        text("DESIGN / 01", 11.0, Color::rgb(104, 93, 80)),
        text("A day to create", 25.0, INK),
        text("Ideas take shape through motion.\nLight, color and small details\nmake an experience clear.", 15.0, INK),
        Element::row(vec![Element::icon(Icon::Book).foreground(INK), text("Read the story", 14.0, INK)]).gap(8.0),
    ]).gap(12.0).padding(20.0).background(Color::rgb(249, 230, 205)).radius(18.0).flex(1.0);
    let projects = Element::column(vec![
        text("YOUR SPACE", 11.0, Color::rgb(81, 111, 132)),
        text("Everything connected", 25.0, INK),
        text("Library        24 items\nProjects       8 in progress\nFavorites      12 saved", 15.0, INK),
        Element::row(vec![Element::icon(Icon::Globe).foreground(INK), text("Explore collections", 14.0, INK)]).gap(8.0),
    ]).gap(12.0).padding(20.0).background(Color::rgb(207, 229, 239)).radius(18.0).flex(1.0);
    Element::column(vec![
        Element::row(vec![
            text("Your library", 30.0, INK),
            Element::spacer().flex(1.0),
            text("TUESDAY 13 · 11:06", 12.0, Color::rgb(104, 113, 132)),
        ]),
        Element::row(vec![
            text("Recent", 15.0, Color::rgb(41, 108, 210)),
            text("Collections", 15.0, INK),
            text("Shared", 15.0, INK),
        ])
        .gap(26.0)
        .padding(8.0),
        Element::row(tiles).gap(12.0).padding(8.0),
        Element::row(vec![article, projects]).gap(16.0).flex(1.0),
    ])
    .gap(16.0)
    .padding(24.0)
    .background(Color::rgb(239, 242, 248))
    .size(Some(768.0), Some(480.0))
}

fn content_frame() -> (egui::Context, egui::FullOutput) {
    let context = egui::Context::default();
    context.set_pixels_per_point(2.0);
    limn_egui::install_fonts(&context);
    let mut tree = UiTree::new(page());
    let output = context.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(768.0, 480.0),
            )),
            ..Default::default()
        },
        |ui| {
            limn_egui::attach_fonts(&mut tree, ui.ctx());
            let frame = tree.layout(Size::new(768.0, 480.0));
            limn_egui::paint_at(ui, &frame, egui::Pos2::ZERO);
        },
    );
    (context, output)
}

/// Opaque RGBA8 atlas, 768 × 480 logical points at 2× density.
pub fn texture(state: &egui_wgpu::RenderState) -> wgpu::Texture {
    let (context, mut output) = content_frame();
    let primitives = context.tessellate(output.shapes, output.pixels_per_point);
    let texture = state.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("limn glass content atlas"),
        size: wgpu::Extent3d {
            width: SIZE[0],
            height: SIZE[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let mut renderer = egui_wgpu::Renderer::new(
        &state.device,
        wgpu::TextureFormat::Rgba8Unorm,
        egui_wgpu::RendererOptions::default(),
    );
    for (id, deltas) in output.textures_delta.set.drain() {
        for delta in deltas {
            renderer.update_texture(&state.device, &state.queue, id, &delta);
        }
    }
    let screen = egui_wgpu::ScreenDescriptor {
        size_in_pixels: SIZE,
        pixels_per_point: output.pixels_per_point,
    };
    let mut encoder = state
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("limn glass content upload"),
        });
    let commands = renderer.update_buffers(
        &state.device,
        &state.queue,
        &mut encoder,
        &primitives,
        &screen,
    );
    let view = texture.create_view(&Default::default());
    {
        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("limn glass content render"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        renderer.render(&mut pass.forget_lifetime(), &primitives, &screen);
    }
    state
        .queue
        .submit(commands.into_iter().chain([encoder.finish()]));
    for id in output.textures_delta.free.drain() {
        renderer.free_texture(&id);
    }
    texture
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atlas_contains_real_text_and_all_six_icons() {
        let frame = UiTree::new(page()).layout(Size::new(768.0, 480.0));
        let icons = frame
            .commands
            .iter()
            .filter(|command| matches!(command, limn::DrawCommand::Icon { .. }))
            .count();
        assert_eq!(icons, 8);
        assert!(frame.commands.iter().any(|command| matches!(command, limn::DrawCommand::Text { text, .. } if text == "Your library")));
        assert!(frame.commands.iter().any(|command| matches!(command, limn::DrawCommand::Text { text, .. } if text.contains("Ideas take shape"))));
    }

    #[test]
    fn atlas_initializes_fonts_and_icons_in_a_fresh_context() {
        let (context, mut output) = content_frame();
        output.textures_delta.clear();
        assert_eq!(output.pixels_per_point, 2.0);
        let primitives = context.tessellate(output.shapes, output.pixels_per_point);
        assert!(!primitives.is_empty());
    }
}
