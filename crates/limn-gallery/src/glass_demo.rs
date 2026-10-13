//! One GPU backdrop shared by the visible scene and the optical material.

use eframe::{egui, egui_wgpu, wgpu};
use limn::{DrawCommand, Frame};
use limn_egui::GlassRenderer;
use std::time::{Duration, Instant};

pub struct GlassDemo {
    renderer: GlassRenderer,
    state: egui_wgpu::RenderState,
    scene: Option<SceneRenderer>,
    painted: Option<([u32; 2], [f32; 2], String, f32)>,
    phase: f32,
    updated: Instant,
}

impl GlassDemo {
    pub fn new(state: &egui_wgpu::RenderState) -> Self {
        Self {
            renderer: GlassRenderer::new(state),
            state: state.clone(),
            scene: None,
            painted: None,
            phase: 0.0,
            updated: Instant::now(),
        }
    }

    pub fn paint(
        &mut self,
        ui: &egui::Ui,
        frame: &Frame,
        origin: egui::Pos2,
        scene: &str,
        animate: bool,
        speed: f32,
    ) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.updated);
        self.updated = now;
        let backdrop = frame.commands.iter().find_map(|command| match command {
            DrawCommand::Surface { rect, id, .. } if id == crate::pages::glass::BACKDROP => {
                Some(*rect)
            }
            _ => None,
        });
        let Some(backdrop) = backdrop else {
            limn_egui::paint_at(ui, frame, origin);
            return;
        };
        let rect = egui::Rect::from_min_size(
            origin + egui::vec2(backdrop.x, backdrop.y),
            egui::vec2(backdrop.width, backdrop.height),
        );
        if !rect.intersect(ui.clip_rect()).is_positive() {
            limn_egui::paint_at(ui, frame, origin);
            return;
        }
        self.phase = advance_phase(self.phase, elapsed, animate, speed);
        let size = backdrop_size(rect.size(), ui.ctx().pixels_per_point());
        let logical = [backdrop.width, backdrop.height];
        let key = (size, logical, scene.to_owned(), self.phase);
        if self.painted.as_ref() != Some(&key) {
            let gpu = self
                .scene
                .get_or_insert_with(|| SceneRenderer::new(&self.state));
            let resized = gpu
                .target
                .as_ref()
                .is_none_or(|t| [t.width(), t.height()] != size);
            gpu.paint(&self.state, size, logical, scene_index(scene), self.phase);
            let needs_import = resized || self.painted.as_ref().is_none_or(|key| key.0 != size);
            if needs_import
                && !self
                    .renderer
                    .set_backdrop_texture(gpu.target.as_ref().unwrap().clone())
            {
                limn_egui::paint_at(ui, frame, origin);
                return;
            }
            self.painted = Some(key);
        }
        self.renderer
            .paint_at_surface(ui, frame, origin, rect, crate::pages::glass::BACKDROP);
        if animate {
            ui.ctx().request_repaint();
        }
    }
}

fn advance_phase(phase: f32, elapsed: Duration, animate: bool, speed: f32) -> f32 {
    if !animate {
        return phase;
    }
    let speed = if speed.is_finite() {
        speed.clamp(0.0, 1.0)
    } else {
        0.5
    };
    (phase + elapsed.as_secs_f32().min(0.1) * (0.15 + speed * 1.85))
        .rem_euclid(std::f32::consts::TAU)
}

fn backdrop_size(logical: egui::Vec2, scale: f32) -> [u32; 2] {
    let density = scale
        .min(2048.0 / logical.x.max(1.0))
        .min(2048.0 / logical.y.max(1.0));
    [
        (logical.x * density).ceil().clamp(1.0, 2048.0) as u32,
        (logical.y * density).ceil().clamp(1.0, 2048.0) as u32,
    ]
}

fn scene_index(scene: &str) -> f32 {
    match scene {
        "ocean" => 1.0,
        "grid" => 2.0,
        "checker" => 3.0,
        "rings" => 4.0,
        "content" => 5.0,
        _ => 0.0,
    }
}

struct SceneRenderer {
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    bindings: wgpu::BindGroup,
    target: Option<wgpu::Texture>,
}

impl SceneRenderer {
    fn new(state: &egui_wgpu::RenderState) -> Self {
        let device = &state.device;
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("glass scene parameters"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("glass scene bindings"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(16),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let content = crate::glass_content::texture(state);
        let view = content.create_view(&Default::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("glass scene sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("glass scene"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniforms.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("glass animated backgrounds"),
            source: wgpu::ShaderSource::Wgsl(include_str!("glass_scene.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("glass scene pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("glass scene pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        Self {
            pipeline,
            uniforms,
            bindings,
            target: None,
        }
    }

    fn paint(
        &mut self,
        state: &egui_wgpu::RenderState,
        size: [u32; 2],
        logical: [f32; 2],
        scene: f32,
        phase: f32,
    ) {
        if self
            .target
            .as_ref()
            .is_none_or(|t| [t.width(), t.height()] != size)
        {
            self.target = Some(state.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("glass shared animated backdrop"),
                size: wgpu::Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            }));
        }
        let parameters = [logical[0], logical[1], phase, scene];
        let bytes: Vec<u8> = parameters.iter().flat_map(|v| v.to_ne_bytes()).collect();
        state.queue.write_buffer(&self.uniforms, 0, &bytes);
        let view = self
            .target
            .as_ref()
            .unwrap()
            .create_view(&Default::default());
        let mut encoder = state
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("glass backdrop frame"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("glass backdrop pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bindings, &[]);
            pass.draw(0..6, 0..1);
        }
        state.queue.submit([encoder.finish()]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn animation_pause_resume_and_speed_are_continuous_and_bounded() {
        let phase = advance_phase(0.8, Duration::from_millis(16), true, 0.5);
        assert!(phase > 0.8);
        assert_eq!(
            advance_phase(phase, Duration::from_secs(60), false, 0.5),
            phase
        );
        assert!(advance_phase(phase, Duration::from_secs(60), true, 1.0) - phase <= 0.201);
        assert!(advance_phase(phase, Duration::from_millis(16), true, f32::NAN).is_finite());
        assert!(advance_phase(6.28, Duration::from_secs(1), true, 1.0) < std::f32::consts::TAU);
    }

    #[test]
    fn hidpi_texture_is_bounded_without_changing_logical_backdrop() {
        assert_eq!(backdrop_size(egui::vec2(2200.0, 380.0), 2.0), [2048, 354]);
        assert_eq!(backdrop_size(egui::vec2(900.0, 380.0), 1.0), [900, 380]);
    }

    #[test]
    fn scene_shader_is_valid_wgsl() {
        let shader = wgpu::naga::front::wgsl::parse_str(include_str!("glass_scene.wgsl")).unwrap();
        wgpu::naga::valid::Validator::new(
            wgpu::naga::valid::ValidationFlags::all(),
            wgpu::naga::valid::Capabilities::empty(),
        )
        .validate(&shader)
        .unwrap();
    }
}
