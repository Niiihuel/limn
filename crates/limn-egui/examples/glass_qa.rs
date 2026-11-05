//! Headless glass captures and synchronized frame-cost measurements.
//!
//! Run with `--backdrop <png> --out <directory> [--frames 60] [--motion]`.
//! `WGPU_BACKEND=gl LIBGL_ALWAYS_SOFTWARE=1` exercises Mesa's software GL.

use eframe::{egui, egui_wgpu, wgpu};
use limn::{
    Component, ComponentPalette, DrawCommand, Element, FloatingBarMotion, FloatingGlassBar,
    FloatingGlassTab, Frame, GlassMaterial, GlassMenu, Icon, Point, Rect, Size, Switch, Tab, Tabs,
    UiTree,
};
use limn_egui::GlassRenderer;
use std::{
    path::{Path, PathBuf},
    time::Instant,
};

struct Qa {
    state: egui_wgpu::RenderState,
    context: egui::Context,
    glass: GlassRenderer,
    target: wgpu::Texture,
    size: [u32; 2],
}

impl Qa {
    async fn new(backdrop: &Path) -> Self {
        let mut config = egui_wgpu::WgpuConfiguration::default();
        if let egui_wgpu::WgpuSetup::CreateNew(setup) = &mut config.wgpu_setup {
            setup.native_adapter_selector = Some(std::sync::Arc::new(|adapters, _| {
                for adapter in adapters {
                    eprintln!("available_adapter={:?}", adapter.get_info());
                }
                let software =
                    std::env::var("LIBGL_ALWAYS_SOFTWARE").is_ok_and(|value| value == "1");
                adapters
                    .iter()
                    .find(|adapter| {
                        !software || adapter.get_info().device_type == wgpu::DeviceType::Cpu
                    })
                    .or_else(|| adapters.first())
                    .cloned()
                    .ok_or_else(|| "no adapter".into())
            }));
            setup.device_descriptor = std::sync::Arc::new(|adapter| wgpu::DeviceDescriptor {
                label: Some("limn glass QA downlevel"),
                required_limits: wgpu::Limits {
                    max_texture_dimension_2d: adapter.limits().max_texture_dimension_2d.min(8192),
                    ..wgpu::Limits::downlevel_defaults()
                },
                ..Default::default()
            });
        }
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let state = egui_wgpu::RenderState::create(&config, &instance, None, Default::default())
            .await
            .expect("headless adapter");
        eprintln!(
            "adapter={:?} limits={:?}",
            state.adapter.get_info(),
            state.device.limits()
        );
        let pixels = image::open(backdrop).expect("backdrop PNG").to_rgba8();
        let size = [pixels.width(), pixels.height()];
        let mut glass = GlassRenderer::new(&state);
        assert!(glass.set_backdrop_rgba(size, pixels.as_raw()));
        let target = state.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("glass QA output"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: state.target_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let context = egui::Context::default();
        limn_egui::install_fonts(&context);
        Self {
            state,
            context,
            glass,
            target,
            size,
        }
    }

    fn area(&self) -> egui::Rect {
        egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(self.size[0] as f32, self.size[1] as f32),
        )
    }

    // CPU includes callback preparation, the shared blur and queue submission.
    // Wall time additionally waits for all GPU work; it is not a timestamp query.
    fn draw(&mut self, frame: &Frame, optical: bool) -> (f64, f64) {
        let start = Instant::now();
        let area = self.area();
        self.context.begin_pass(egui::RawInput {
            screen_rect: Some(area),
            ..Default::default()
        });
        let ui = egui::Ui::new(
            self.context.clone(),
            egui::Id::unique("glass QA"),
            egui::UiBuilder::new().max_rect(area),
        );
        self.glass.paint_backdrop(&ui, area);
        if optical {
            self.glass.paint_at(&ui, frame, egui::Pos2::ZERO, area);
        } else {
            limn_egui::paint_at(&ui, frame, egui::Pos2::ZERO);
        }
        let mut output = self.context.end_pass();
        let jobs = self
            .context
            .tessellate(output.shapes, output.pixels_per_point);
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: self.size,
            pixels_per_point: 1.0,
        };
        let mut renderer = self.state.renderer.write();
        for (id, deltas) in &output.textures_delta.set {
            for delta in deltas {
                renderer.update_texture(&self.state.device, &self.state.queue, *id, delta);
            }
        }
        let mut encoder = self
            .state
            .device
            .create_command_encoder(&Default::default());
        let preparations = renderer.update_buffers(
            &self.state.device,
            &self.state.queue,
            &mut encoder,
            &jobs,
            &screen,
        );
        let view = self.target.create_view(&Default::default());
        {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("glass QA compose"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    ..Default::default()
                })
                .forget_lifetime();
            renderer.render(&mut pass, &jobs, &screen);
        }
        self.state
            .queue
            .submit(preparations.into_iter().chain([encoder.finish()]));
        let cpu = start.elapsed().as_secs_f64() * 1000.0;
        self.state
            .device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("wait for glass frame");
        let wall = start.elapsed().as_secs_f64() * 1000.0;
        for id in &output.textures_delta.free {
            renderer.free_texture(id);
        }
        output.textures_delta.clear();
        (cpu, wall)
    }

    fn capture(&self, path: &Path) {
        let row = (self.size[0] * 4).div_ceil(256) * 256;
        let buffer = self.state.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("glass QA readback"),
            size: u64::from(row) * u64::from(self.size[1]),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .state
            .device
            .create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            self.target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: None,
                },
            },
            self.target.size(),
        );
        self.state.queue.submit([encoder.finish()]);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer.map_async(wgpu::MapMode::Read, .., move |result| {
            sender.send(result).unwrap()
        });
        self.state
            .device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("wait for capture");
        receiver.recv().unwrap().unwrap();
        let mapped = buffer.get_mapped_range(..).expect("mapped capture buffer");
        let mut rgba = Vec::with_capacity((self.size[0] * self.size[1] * 4) as usize);
        for line in mapped.chunks(row as usize) {
            rgba.extend_from_slice(&line[..self.size[0] as usize * 4]);
        }
        image::save_buffer(
            path,
            &rgba,
            self.size[0],
            self.size[1],
            image::ColorType::Rgba8,
        )
        .expect("write capture");
        drop(mapped);
        buffer.unmap();
    }
}

fn material(palette: ComponentPalette) -> GlassMaterial {
    GlassMaterial::new(limn::Color {
        a: 122,
        ..palette.surface
    })
    .saturation(2.2)
}

fn comparison_frame() -> Frame {
    // These values match the original Qt shader harness, not a component theme.
    let tint = limn::Color::rgba(38, 38, 40, 122);
    let material = GlassMaterial::new(tint).saturation(2.2);
    let mut frame = Frame::default();
    frame.commands = vec![
        DrawCommand::Glass {
            rect: Rect::new(40.0, 48.0, 360.0, 112.0),
            radius: 56.0,
            material,
            opacity: 1.0,
        },
        DrawCommand::Glass {
            rect: Rect::new(245.0, 205.0, 390.0, 180.0),
            radius: 32.0,
            material,
            opacity: 1.0,
        },
        DrawCommand::Glass {
            rect: Rect::new(500.0, 54.0, 130.0, 100.0),
            radius: 24.0,
            material,
            opacity: 1.0,
        },
    ];
    frame
}

fn measure(qa: &mut Qa, frame: &Frame, frames: usize, name: &str) {
    for _ in 0..10 {
        qa.draw(frame, true);
    }
    let mut cpu = Vec::new();
    let mut wall = Vec::new();
    for _ in 0..frames {
        let (a, b) = qa.draw(frame, true);
        cpu.push(a);
        wall.push(b);
    }
    cpu.sort_by(f64::total_cmp);
    wall.sort_by(f64::total_cmp);
    let p95 = (frames * 95 / 100).min(frames - 1);
    println!("{name}: n={frames} cpu_median_ms={:.4} cpu_p95_ms={:.4} synchronized_median_ms={:.4} synchronized_p95_ms={:.4}", cpu[frames/2], cpu[p95], wall[frames/2], wall[p95]);
}

fn local_lens_frame(qa: &Qa) -> Frame {
    let palette = ComponentPalette::neutral_dark();
    let lens = GlassMaterial::new(limn::Color {
        a: 0,
        ..palette.selected
    })
    .blur(0.0)
    .magnify(1.1)
    .bezel(6.0);
    let view = |selected: bool| {
        Element::column(vec![
            Switch::new("qa:switch", selected, palette)
                .track_size(64.0, 28.0)
                .thumb_inset(2.0)
                .thumb_width(40.0)
                .ink(limn::AlertColors::for_tone(limn::AlertTone::Success, palette).accent)
                .glass(lens)
                .render(),
            Tabs::new(
                vec![
                    Tab::new("qa:home", "Home", 110.0).active(!selected),
                    Tab::new("qa:library", "Library", 110.0).active(selected),
                ],
                palette,
            )
            .height(56.0)
            .radius(28.0)
            .render()
            .glass_selection(lens),
        ])
        .gap(36.0)
        .padding(64.0)
    };
    let now = web_time::Instant::now();
    let size = Size::new(qa.size[0] as f32, qa.size[1] as f32);
    let mut tree = UiTree::new(view(false));
    tree.layout_at(size, now);
    tree.set_root(view(true));
    tree.layout_at(size, now);
    tree.layout_at(size, now + web_time::Duration::from_millis(100))
}

fn measure_captured_page(qa: &mut Qa, frames: usize) {
    let palette = ComponentPalette::neutral_dark();
    let mut page = UiTree::new(
        Element::column(
            (0..12)
                .map(|index| {
                    Element::text(format!("Album {index} · Live page content · 0123456789"))
                        .font_size(24.0)
                        .foreground(palette.text)
                })
                .collect(),
        )
        .gap(18.0)
        .padding(24.0)
        .background(palette.surface),
    );
    let scene = page.layout(Size::new(qa.size[0] as f32, qa.size[1] as f32));
    for (name, frame) in [
        ("captured-page-only", Frame::default()),
        ("captured-page-three-glass", phone_frame(qa.size)),
    ] {
        let mut cpu = Vec::new();
        let mut wall = Vec::new();
        for index in 0..frames + 10 {
            let start = Instant::now();
            assert!(qa.glass.set_backdrop_frame("qa:page", &scene, qa.size, 1.0));
            let capture = start.elapsed().as_secs_f64() * 1000.0;
            let (submission, _) = qa.draw(&frame, true);
            if index >= 10 {
                cpu.push(capture + submission);
                wall.push(start.elapsed().as_secs_f64() * 1000.0);
            }
        }
        cpu.sort_by(f64::total_cmp);
        wall.sort_by(f64::total_cmp);
        let p95 = (frames * 95 / 100).min(frames - 1);
        println!("{name}: n={frames} cpu_median_ms={:.4} cpu_p95_ms={:.4} synchronized_median_ms={:.4} synchronized_p95_ms={:.4} offscreen_page=included", cpu[frames/2], cpu[p95], wall[frames/2], wall[p95]);
    }
}

fn phone_frame(size: [u32; 2]) -> Frame {
    let w = size[0] as f32;
    let h = size[1] as f32;
    let material = material(ComponentPalette::neutral_dark());
    let mut frame = Frame::default();
    frame.commands = vec![
        DrawCommand::Glass {
            rect: Rect::new(16.0, h - 90.0, w - 32.0, 64.0),
            radius: 32.0,
            material,
            opacity: 1.0,
        },
        DrawCommand::Glass {
            rect: Rect::new(16.0, h - 170.0, w - 32.0, 64.0),
            radius: 32.0,
            material,
            opacity: 1.0,
        },
        DrawCommand::Glass {
            rect: Rect::new(w - 260.0, 90.0, 240.0, 360.0),
            radius: 32.0,
            material,
            opacity: 1.0,
        },
    ];
    frame
}

fn measure_video(qa: &mut Qa, path: &Path, frames: usize) {
    let video = std::process::Command::new("ffmpeg")
        .args(["-v", "error", "-ss", "1", "-i"])
        .arg(path)
        .args([
            "-frames:v",
            "12",
            "-vf",
            &format!("scale={}:{}", qa.size[0], qa.size[1]),
            "-pix_fmt",
            "rgba",
            "-f",
            "rawvideo",
            "-",
        ])
        .output()
        .expect("ffmpeg video input");
    assert!(video.status.success(), "video decoding failed");
    let bytes = (qa.size[0] * qa.size[1] * 4) as usize;
    let inputs: Vec<_> = video.stdout.chunks_exact(bytes).collect();
    assert!(!inputs.is_empty());
    let texture = qa.state.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("glass QA changing video"),
        size: wgpu::Extent3d {
            width: qa.size[0],
            height: qa.size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    assert!(qa.glass.set_backdrop_texture(texture.clone()));
    for (name, frame) in [
        ("changing-video-only", Frame::default()),
        ("changing-video-three-glass", phone_frame(qa.size)),
    ] {
        let mut cpu = Vec::new();
        let mut wall = Vec::new();
        for index in 0..frames + 10 {
            let start = Instant::now();
            qa.state.queue.write_texture(
                texture.as_image_copy(),
                inputs[index % inputs.len()],
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(qa.size[0] * 4),
                    rows_per_image: Some(qa.size[1]),
                },
                texture.size(),
            );
            let upload = start.elapsed().as_secs_f64() * 1000.0;
            let (a, _) = qa.draw(&frame, true);
            if index >= 10 {
                cpu.push(upload + a);
                wall.push(start.elapsed().as_secs_f64() * 1000.0);
            }
        }
        cpu.sort_by(f64::total_cmp);
        wall.sort_by(f64::total_cmp);
        let p95 = (frames * 95 / 100).min(frames - 1);
        println!("{name}: n={frames} cpu_median_ms={:.4} cpu_p95_ms={:.4} synchronized_median_ms={:.4} synchronized_p95_ms={:.4} decode=excluded upload=included", cpu[frames/2], cpu[p95], wall[frames/2], wall[p95]);
    }
}

fn motion_frames(qa: &mut Qa, out: &Path) {
    let palette = ComponentPalette::neutral_dark();
    let glass = material(palette);
    let clear = GlassMaterial::new(limn::Color {
        a: 0,
        ..palette.selected
    })
    .blur(0.0)
    .magnify(1.1)
    .bezel(6.0);
    let start = web_time::Instant::now();
    let viewport = Size::new(qa.size[0] as f32, qa.size[1] as f32);
    let wrap = |content| {
        Element::column(vec![
            Element::spacer().size(None, Some(110.0)),
            Element::row(vec![Element::spacer().size(Some(100.0), None), content]),
        ])
    };
    let tabs = |selected| {
        wrap(
            Tabs::new(
                vec![
                    Tab::new("home", "Home", 110.0).active(selected == 0),
                    Tab::new("browse", "Browse", 110.0).active(selected == 1),
                    Tab::new("library", "Library", 110.0).active(selected == 2),
                ],
                palette,
            )
            .height(56.0)
            .radius(28.0)
            .render()
            .glass_selection(clear),
        )
    };
    let switch = |value| {
        wrap(
            Switch::new("switch", value, palette)
                .track_size(64.0, 28.0)
                .thumb_inset(2.0)
                .thumb_width(40.0)
                .ink(limn::AlertColors::for_tone(limn::AlertTone::Success, palette).accent)
                .glass(clear)
                .render(),
        )
    };
    let menu = |open| {
        wrap(
            GlassMenu::new(
                "menu",
                Element::button("more", "…").size(Some(44.0), Some(44.0)),
                Rect::new(100.0, 110.0, 44.0, 44.0),
                vec![
                    limn::DropdownMenuItem::new("queue", "Add to queue").into(),
                    limn::DropdownMenuItem::new("like", "Like this song").into(),
                    limn::DropdownMenuItem::new("album", "Open album").into(),
                ],
                open,
                glass,
                palette,
            )
            .width(280.0)
            .render(),
        )
    };
    let sheet = |open| {
        limn::BottomSheet::new(
            Element::spacer(),
            open,
            "Audio output",
            "Choose a route",
            "sheet:dismiss",
            palette,
        )
        .body(
            Element::column(vec![
                Element::button("speaker", "This phone"),
                Element::button("headphones", "Headphones"),
            ])
            .gap(12.0),
        )
        .max_height(360.0)
        .glass(glass)
        .render()
    };
    for (name, before, after) in [
        ("tabs", tabs(0), tabs(2)),
        ("tabs-return", tabs(2), tabs(1)),
        ("switch", switch(false), switch(true)),
        ("switch-off", switch(true), switch(false)),
        ("menu", menu(false), menu(true)),
        ("menu-close", menu(true), menu(false)),
        ("sheet", sheet(false), sheet(true)),
        ("sheet-close", sheet(true), sheet(false)),
    ] {
        let dir = out.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        let mut tree = UiTree::new(before);
        tree.layout_at(viewport, start);
        let transition = start + web_time::Duration::from_secs(2);
        tree.layout_at(viewport, transition);
        tree.set_root(after);
        for index in 0..21 {
            let now = transition + web_time::Duration::from_secs_f64(index as f64 / 30.0);
            let frame = tree.layout_at(viewport, now);
            qa.draw(&frame, true);
            qa.capture(&dir.join(format!("{index:03}.png")));
        }
    }
    // Exercise a held drag, not only a host changing the checked value.
    let dir = out.join("switch-drag");
    std::fs::create_dir_all(&dir).unwrap();
    let mut tree = UiTree::new(switch(false));
    tree.layout_at(viewport, start);
    let transition = start + web_time::Duration::from_secs(2);
    tree.layout_at(viewport, transition);
    tree.handle_event_at(
        limn::UiEvent::PointerMove(Point::new(112.0, 124.0)),
        transition,
    );
    tree.handle_event_at(
        limn::UiEvent::PointerDown(Point::new(112.0, 124.0)),
        transition,
    );
    for index in 0..24 {
        let now = transition + web_time::Duration::from_secs_f64(index as f64 / 30.0);
        if (2..8).contains(&index) {
            tree.handle_event_at(
                limn::UiEvent::PointerMove(Point::new(112.0 + (index - 1) as f32 * 6.0, 124.0)),
                now,
            );
        }
        if index == 8 {
            tree.handle_event_at(limn::UiEvent::PointerUp(Point::new(152.0, 124.0)), now);
            tree.set_root(switch(true));
        }
        let frame = tree.layout_at(viewport, now);
        qa.draw(&frame, true);
        qa.capture(&dir.join(format!("{index:03}.png")));
    }
    let dir = out.join("collapse");
    std::fs::create_dir_all(&dir).unwrap();
    let mut motion = FloatingBarMotion::new(false, start);
    motion.set_collapsed(true, start, false);
    for index in 0..30 {
        let now = start + web_time::Duration::from_secs_f64(index as f64 / 30.0);
        let bar = FloatingGlassBar::new(
            "bar",
            vec![
                FloatingGlassTab::new("home", "Home", Icon::Home),
                FloatingGlassTab::new("browse", "Explore", Icon::Search),
                FloatingGlassTab::new("library", "Library", Icon::Library).selected(true),
            ],
            Element::clickable(
                "search",
                Element::icon(Icon::Search)
                    .large_icon(24.0)
                    .foreground(palette.text),
            ),
            glass,
            palette,
        )
        .player(
            Element::row(vec![
                Element::text("Helium · The Chainsmokers")
                    .foreground(palette.text)
                    .flex(1.0),
                Element::clickable(
                    "play",
                    Element::icon(Icon::Pause)
                        .large_icon(20.0)
                        .foreground(palette.text),
                ),
            ])
            .padding(12.0),
        )
        .collapse(motion.value(now))
        .render();
        let mut tree = UiTree::new(wrap(bar));
        let frame = tree.layout_at(viewport, now);
        qa.draw(&frame, true);
        qa.capture(&dir.join(format!("{index:03}.png")));
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut backdrop = None;
    let mut out = PathBuf::from("/tmp/limn-glass-qa");
    let mut frames = 60;
    let mut motion = false;
    let mut video = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--backdrop" => backdrop = args.next().map(PathBuf::from),
            "--out" => out = PathBuf::from(args.next().expect("output directory")),
            "--frames" => frames = args.next().unwrap().parse::<usize>().unwrap().max(1),
            "--motion" => motion = true,
            "--video" => video = args.next().map(PathBuf::from),
            _ => panic!("unknown argument {arg}"),
        }
    }
    std::fs::create_dir_all(&out).unwrap();
    let mut qa = pollster::block_on(Qa::new(&backdrop.expect("--backdrop <png>")));
    let frame = comparison_frame();
    qa.draw(&frame, true);
    qa.capture(&out.join("limn.png"));
    measure(&mut qa, &Frame::default(), frames, "backdrop-only");
    measure(&mut qa, &frame, frames, "three-glass-shared-blur");
    let local = local_lens_frame(&qa);
    measure(&mut qa, &local, frames, "two-local-selection-lenses");
    if motion {
        motion_frames(&mut qa, &out);
    }
    measure_captured_page(&mut qa, frames);
    if let Some(path) = video {
        measure_video(&mut qa, &path, frames);
    }
}
