//! Native optical material over a host-provided image, never a desktop capture.

use eframe::{egui, egui_wgpu, wgpu};
use limn::{Frame, GlassMaterial};
use std::cell::RefCell;
use wgpu::util::DeviceExt;

/// Paints glass commands over an explicitly supplied image or GPU texture.
///
/// Create once from `CreationContext::wgpu_render_state`. Uses the default
/// eframe render target (no MSAA or depth attachment). Upload only when the
/// backdrop changes; painting retains the GPU texture and pipeline. Blur is
/// refreshed on each paint call because imported textures can change in place,
/// and shared between panes with the same radius and backdrop dimensions.
pub struct GlassRenderer {
    state: egui_wgpu::RenderState,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    backdrop: Option<Backdrop>,
    blur: RefCell<BackdropBlur>,
}

struct Backdrop {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    id: egui::TextureId,
    size: [u32; 2],
}

impl GlassRenderer {
    pub fn new(state: &egui_wgpu::RenderState) -> Self {
        let device = &state.device;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("limn glass bindings"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(96),
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
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("limn liquid glass"),
            source: wgpu::ShaderSource::Wgsl(include_str!("glass.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("limn glass pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("limn glass pipeline"),
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
                    format: state.target_format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("limn glass backdrop sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Self {
            state: state.clone(),
            pipeline,
            layout,
            sampler,
            backdrop: None,
            blur: RefCell::new(BackdropBlur::new(device)),
        }
    }

    /// Use a GPU-produced backdrop without readback or another texture upload.
    ///
    /// The texture must belong to this renderer's device and contain
    /// premultiplied, gamma-encoded RGBA in `Rgba8Unorm` format. It must be D2,
    /// single-layer, single-mip, single-sampled, with `TEXTURE_BINDING` usage.
    /// Submit writes before egui paints; retain a clone to animate it in place.
    /// Invalid metadata returns false and keeps the previous backdrop. Device
    /// ownership and submission order remain the caller's responsibility.
    pub fn set_backdrop_texture(&mut self, texture: wgpu::Texture) -> bool {
        let descriptor = wgpu::TextureDescriptor {
            label: None,
            size: texture.size(),
            mip_level_count: texture.mip_level_count(),
            sample_count: texture.sample_count(),
            dimension: texture.dimension(),
            format: texture.format(),
            usage: texture.usage(),
            view_formats: &[],
        };
        if !valid_backdrop(
            &descriptor,
            self.state.device.limits().max_texture_dimension_2d,
        ) {
            return false;
        }
        self.replace_backdrop(texture);
        true
    }

    fn replace_backdrop(&mut self, texture: wgpu::Texture) {
        self.blur.get_mut().entries.clear();
        let view = texture.create_view(&Default::default());
        let mut renderer = self.state.renderer.write();
        let id = if let Some(previous) = &self.backdrop {
            renderer.update_egui_texture_from_wgpu_texture(
                &self.state.device,
                &view,
                wgpu::FilterMode::Linear,
                previous.id,
            );
            previous.id
        } else {
            renderer.register_native_texture(&self.state.device, &view, wgpu::FilterMode::Linear)
        };
        self.backdrop = Some(Backdrop {
            size: [texture.width(), texture.height()],
            texture,
            view,
            id,
        });
    }

    /// Upload an RGBA8 image (straight alpha). Invalid dimensions or byte lengths
    /// return false and leave the previous image intact. The image is stretched
    /// to the `backdrop_rect` supplied when painting.
    pub fn set_backdrop_rgba(&mut self, size: [u32; 2], rgba: &[u8]) -> bool {
        let device = &self.state.device;
        if !valid_upload(size, rgba.len(), device.limits().max_texture_dimension_2d) {
            return false;
        }
        if self.backdrop.as_ref().is_none_or(|image| {
            image.size != size
                || !image
                    .texture
                    .usage()
                    .contains(wgpu::TextureUsages::COPY_DST)
        }) {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("limn glass backdrop"),
                size: wgpu::Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            self.replace_backdrop(texture);
        }
        let premultiplied: Vec<u8> = rgba
            .chunks_exact(4)
            .flat_map(|p| {
                let alpha = u16::from(p[3]);
                [
                    ((u16::from(p[0]) * alpha + 127) / 255) as u8,
                    ((u16::from(p[1]) * alpha + 127) / 255) as u8,
                    ((u16::from(p[2]) * alpha + 127) / 255) as u8,
                    p[3],
                ]
            })
            .collect();
        self.state.queue.write_texture(
            self.backdrop.as_ref().unwrap().texture.as_image_copy(),
            &premultiplied,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size[0] * 4),
                rows_per_image: Some(size[1]),
            },
            wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
        );
        true
    }

    /// Paint the uploaded image; hosts can instead show identical pixels using
    /// a limn surface, placing the image in the tree's normal drawing order.
    pub fn paint_backdrop(&self, ui: &egui::Ui, area: egui::Rect) {
        if let Some(image) = &self.backdrop {
            ui.painter().image(
                image.id,
                area,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
    }

    /// Paint a complete frame. `origin` and `backdrop_rect` are window points,
    /// matching egui coordinates regardless of monitor scale or nested clips.
    pub fn paint_at(
        &self,
        ui: &egui::Ui,
        frame: &Frame,
        origin: egui::Pos2,
        backdrop_rect: egui::Rect,
    ) {
        self.blur.borrow_mut().begin_frame();
        super::paint_at_impl(ui, frame, origin, Some((self, backdrop_rect)), None);
    }

    /// Paint the backdrop at each matching `Surface` command's original depth,
    /// then render glass normally. Other surface IDs keep their own pixels.
    ///
    /// The image is stretched to `backdrop_rect` in window points, then clipped
    /// to the matching surface and active clips. Surface opacity is preserved.
    /// No CPU `SurfaceFrame` is needed; an absent image leaves normal surface
    /// painting in place. As with `paint_at`, `origin` is in window points.
    pub fn paint_at_surface(
        &self,
        ui: &egui::Ui,
        frame: &Frame,
        origin: egui::Pos2,
        backdrop_rect: egui::Rect,
        surface_id: &str,
    ) {
        self.blur.borrow_mut().begin_frame();
        let surface = self
            .backdrop
            .as_ref()
            .filter(|_| valid_rect(backdrop_rect))
            .map(|image| super::BackdropSurface {
                id: surface_id,
                texture: image.id,
                rect: backdrop_rect,
            });
        super::paint_at_impl(ui, frame, origin, Some((self, backdrop_rect)), surface);
    }

    #[allow(clippy::too_many_arguments)] // Mirrors the draw command and its host coordinate spaces.
    pub(crate) fn paint_glass(
        &self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        area: egui::Rect,
        radius: f32,
        material: GlassMaterial,
        opacity: f32,
        backdrop_rect: egui::Rect,
    ) -> bool {
        let Some(image) = &self.backdrop else {
            return false;
        };
        let Some(mapping) = backdrop_mapping(area, backdrop_rect) else {
            return false;
        };
        let viewport = ui.ctx().viewport_rect();
        if !valid_rect(viewport) {
            return false;
        }
        let material = material.normalized();
        let backdrop = if material.blur > 0.0 {
            self.blur
                .borrow_mut()
                .view(&self.state, image, backdrop_rect, material.blur)
        } else {
            image.view.clone()
        };
        let tint = material.tint;
        let parameters = [
            area.min.x,
            area.min.y,
            area.width(),
            area.height(),
            viewport.min.x,
            viewport.min.y,
            viewport.width(),
            viewport.height(),
            mapping[0],
            mapping[1],
            mapping[2],
            mapping[3],
            f32::from(tint.r) / 255.0,
            f32::from(tint.g) / 255.0,
            f32::from(tint.b) / 255.0,
            f32::from(tint.a) / 255.0,
            radius.max(0.0).min(area.width().min(area.height()) / 2.0),
            material.blur,
            material.refraction,
            material.dispersion,
            opacity.clamp(0.0, 1.0),
            ui.ctx().pixels_per_point(),
            u8::from(self.state.target_format.is_srgb()) as f32,
            0.0,
        ];
        let bytes: Vec<u8> = parameters
            .iter()
            .flat_map(|value| value.to_ne_bytes())
            .collect();
        // Each callback owns its uniforms: multiple panes in one frame cannot
        // overwrite one another before the deferred egui render pass.
        let uniform = self
            .state
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("limn glass parameters"),
                contents: &bytes,
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let bindings = self
            .state
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("limn glass draw"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&backdrop),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });
        painter.add(egui_wgpu::Callback::new_paint_callback(
            viewport,
            GlassCallback {
                pipeline: self.pipeline.clone(),
                bindings,
            },
        ));
        true
    }
}

impl Drop for GlassRenderer {
    fn drop(&mut self) {
        if let Some(image) = &self.backdrop {
            self.state.renderer.write().free_texture(&image.id);
        }
    }
}

struct GlassCallback {
    pipeline: wgpu::RenderPipeline,
    bindings: wgpu::BindGroup,
}

// Filter the premultiplied backdrop before refraction, so text becomes a
// continuous blur instead of a small number of visibly displaced copies.
const BLUR_SHADER: &str = r#"
@group(0) @binding(0) var image: texture_2d<f32>;
@group(0) @binding(1) var image_sampler: sampler;
@group(0) @binding(2) var<uniform> parameters: vec4<f32>;
struct Vertex { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vertex_main(@builtin(vertex_index) index: u32) -> Vertex {
    let corners = array<vec2<f32>, 6>(vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0),
        vec2(0.0, 1.0), vec2(1.0, 0.0), vec2(1.0, 1.0));
    let uv = corners[index];
    return Vertex(vec4(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0), uv);
}
@fragment fn fragment_main(input: Vertex) -> @location(0) vec4<f32> {
    // A 2x reduction with bilinear filtering averages neighboring texels.
    if parameters.z <= 0.0 { return textureSampleLevel(image, image_sampler, input.uv, 0.0); }
    let sigma = max(parameters.z, 0.1);
    let radius = i32(ceil(sigma * 3.0));
    var sum = vec4(0.0);
    var total = 0.0;
    for (var i = -radius; i <= radius; i += 1) {
        let position = f32(i);
        let weight = exp(-position * position / (2.0 * sigma * sigma));
        sum += textureSampleLevel(image, image_sampler, input.uv + parameters.xy * position, 0.0) * weight;
        total += weight;
    }
    return sum / total;
}
"#;

struct BlurEntry {
    key: [u32; 3],
    levels: Vec<wgpu::Texture>,
    horizontal: wgpu::Texture,
    output: wgpu::Texture,
    used: bool,
}

struct BackdropBlur {
    pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
    entries: Vec<BlurEntry>,
}

impl BackdropBlur {
    fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("limn separable backdrop blur"),
            source: wgpu::ShaderSource::Wgsl(BLUR_SHADER.into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("limn backdrop blur pipeline"),
            layout: None,
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
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("limn blur sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Self {
            pipeline,
            sampler,
            entries: Vec::new(),
        }
    }

    fn begin_frame(&mut self) {
        // Keep allocations for active radii, but never accumulate slider history.
        self.entries.retain(|entry| entry.used);
        for entry in &mut self.entries {
            entry.used = false;
        }
    }

    fn view(
        &mut self,
        state: &egui_wgpu::RenderState,
        source: &Backdrop,
        area: egui::Rect,
        blur: f32,
    ) -> wgpu::TextureView {
        let key = [
            blur.to_bits(),
            area.width().to_bits(),
            area.height().to_bits(),
        ];
        let index = self
            .entries
            .iter()
            .position(|entry| entry.key == key)
            .unwrap_or_else(|| {
                let sizes = blur_levels(source.size, [area.width(), area.height()], blur);
                let size = sizes.last().copied().unwrap_or(source.size);
                let texture = |size: [u32; 2]| {
                    state.device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("limn blur texture"),
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
                    })
                };
                self.entries.push(BlurEntry {
                    key,
                    levels: sizes.into_iter().map(texture).collect(),
                    horizontal: texture(size),
                    output: texture(size),
                    used: false,
                });
                self.entries.len() - 1
            });
        let entry = &self.entries[index];
        if !entry.used {
            let mut encoder =
                state
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("limn backdrop blur"),
                    });
            let mut input = source.view.clone();
            for level in &entry.levels {
                let output = level.create_view(&Default::default());
                self.pass(&state.device, &mut encoder, &input, &output, [0.0; 4]);
                input = output;
            }
            let width = entry.output.width() as f32;
            let height = entry.output.height() as f32;
            let horizontal = entry.horizontal.create_view(&Default::default());
            let output = entry.output.create_view(&Default::default());
            self.pass(
                &state.device,
                &mut encoder,
                &input,
                &horizontal,
                [
                    1.0 / width,
                    0.0,
                    (blur * width / area.width() * 0.5).min(4.0),
                    0.0,
                ],
            );
            self.pass(
                &state.device,
                &mut encoder,
                &horizontal,
                &output,
                [
                    0.0,
                    1.0 / height,
                    (blur * height / area.height() * 0.5).min(4.0),
                    0.0,
                ],
            );
            state.queue.submit([encoder.finish()]);
        }
        self.entries[index].used = true;
        self.entries[index].output.create_view(&Default::default())
    }

    fn pass(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        input: &wgpu::TextureView,
        output: &wgpu::TextureView,
        parameters: [f32; 4],
    ) {
        let bytes: Vec<u8> = parameters
            .iter()
            .flat_map(|value| value.to_ne_bytes())
            .collect();
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("limn blur parameters"),
            contents: &bytes,
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("limn blur bindings"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(input),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform.as_entire_binding(),
                },
            ],
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("limn blur pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bindings, &[]);
        pass.draw(0..6, 0..1);
    }
}

fn blur_levels(mut size: [u32; 2], logical: [f32; 2], blur: f32) -> Vec<[u32; 2]> {
    let mut levels = Vec::new();
    // Sigma is half the material's logical blur radius. Reduce until the
    // separable kernel needs at most 25 samples per axis, including at HiDPI.
    loop {
        let next = std::array::from_fn(|axis| {
            if blur * size[axis] as f32 / logical[axis] > 8.0 {
                size[axis].div_ceil(2)
            } else {
                size[axis]
            }
        });
        if next == size {
            break;
        }
        size = next;
        levels.push(size);
    }
    levels
}

impl egui_wgpu::CallbackTrait for GlassCallback {
    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        _resources: &egui_wgpu::CallbackResources,
    ) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bindings, &[]);
        render_pass.draw(0..6, 0..1);
    }
}

fn valid_upload(size: [u32; 2], length: usize, max_side: u32) -> bool {
    size.iter().all(|side| *side > 0 && *side <= max_side)
        && size[0].checked_mul(4).is_some()
        && u64::from(size[0])
            .checked_mul(u64::from(size[1]))
            .and_then(|n| n.checked_mul(4))
            == Some(length as u64)
        && length <= 256 * 1024 * 1024
}

fn valid_backdrop(texture: &wgpu::TextureDescriptor<'_>, max_side: u32) -> bool {
    texture.dimension == wgpu::TextureDimension::D2
        && texture.size.depth_or_array_layers == 1
        && (1..=max_side).contains(&texture.size.width)
        && (1..=max_side).contains(&texture.size.height)
        && texture.mip_level_count == 1
        && texture.sample_count == 1
        && texture.format == wgpu::TextureFormat::Rgba8Unorm
        && texture.usage.contains(wgpu::TextureUsages::TEXTURE_BINDING)
}

fn valid_rect(area: egui::Rect) -> bool {
    area.is_finite()
        && area.width().is_finite()
        && area.height().is_finite()
        && area.width() > 0.0
        && area.height() > 0.0
}

fn backdrop_mapping(area: egui::Rect, backdrop: egui::Rect) -> Option<[f32; 4]> {
    (valid_rect(area) && valid_rect(backdrop))
        .then(|| {
            [
                (area.min.x - backdrop.min.x) / backdrop.width(),
                (area.min.y - backdrop.min.y) / backdrop.height(),
                area.width() / backdrop.width(),
                area.height() / backdrop.height(),
            ]
        })
        .filter(|mapping| mapping.iter().all(|value| value.is_finite()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_backdrop_requires_a_sampleable_single_layer_rgba_texture() {
        let base = wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 320,
                height: 180,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        };
        assert!(valid_backdrop(&base, 4096));
        assert!(!valid_backdrop(&base, 256));
        for descriptor in [
            wgpu::TextureDescriptor {
                size: wgpu::Extent3d {
                    width: 0,
                    ..base.size
                },
                ..base.clone()
            },
            wgpu::TextureDescriptor {
                size: wgpu::Extent3d {
                    depth_or_array_layers: 2,
                    ..base.size
                },
                ..base.clone()
            },
            wgpu::TextureDescriptor {
                dimension: wgpu::TextureDimension::D3,
                ..base.clone()
            },
            wgpu::TextureDescriptor {
                sample_count: 4,
                ..base.clone()
            },
            wgpu::TextureDescriptor {
                mip_level_count: 2,
                ..base.clone()
            },
            wgpu::TextureDescriptor {
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                ..base.clone()
            },
            wgpu::TextureDescriptor {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                ..base.clone()
            },
        ] {
            assert!(!valid_backdrop(&descriptor, 4096), "{descriptor:?}");
        }
    }

    #[test]
    fn gpu_backdrop_surface_keeps_depth_clip_opacity_and_other_surface_sources() {
        let context = egui::Context::default();
        let origin = egui::pos2(20.0, 30.0);
        let target = egui::TextureId::User(42);
        let mut frame = Frame::default();
        frame.commands = vec![
            limn::DrawCommand::FillRect {
                rect: limn::Rect::new(0.0, 0.0, 100.0, 100.0),
                color: limn::Color::rgb(0, 0, 0),
                radius: 0.0,
            },
            limn::DrawCommand::PushClip(limn::Rect::new(0.0, 0.0, 70.0, 70.0)),
            limn::DrawCommand::Surface {
                rect: limn::Rect::new(10.0, 10.0, 100.0, 40.0),
                id: "gpu-backdrop".into(),
                label: "Backdrop".into(),
                opacity: 0.5,
            },
            limn::DrawCommand::Surface {
                rect: limn::Rect::new(10.0, 55.0, 20.0, 10.0),
                id: "ordinary".into(),
                label: "Other".into(),
                opacity: 1.0,
            },
            limn::DrawCommand::PopClip,
            limn::DrawCommand::FillRect {
                rect: limn::Rect::new(0.0, 0.0, 10.0, 10.0),
                color: limn::Color::WHITE,
                radius: 0.0,
            },
        ];
        let backdrop_rect = egui::Rect::from_min_size(origin, egui::vec2(200.0, 100.0));
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(400.0, 300.0),
                )),
                ..Default::default()
            },
            |ui| {
                assert!(crate::present_surface(
                    ui.ctx(),
                    "ordinary",
                    crate::SurfaceFrame::from_rgba([1, 1], &[255; 4], 1.0).unwrap()
                ));
                super::super::paint_at_impl(
                    ui,
                    &frame,
                    origin,
                    None,
                    Some(super::super::BackdropSurface {
                        id: "gpu-backdrop",
                        texture: target,
                        rect: backdrop_rect,
                    }),
                );
            },
        );
        output.textures_delta.clear();
        let drawn: Vec<_> = output
            .shapes
            .iter()
            .filter(|shape| !matches!(shape.shape, egui::Shape::Noop))
            .collect();
        assert_eq!(drawn.len(), 4);
        assert!(matches!(drawn[0].shape, egui::Shape::Rect(_)));
        let egui::Shape::Mesh(backdrop) = &drawn[1].shape else {
            panic!("backdrop must retain the surface's paint depth")
        };
        assert_eq!(backdrop.texture_id, target);
        assert_eq!(backdrop.calc_bounds(), backdrop_rect);
        assert!(backdrop
            .vertices
            .iter()
            .all(|vertex| vertex.color == egui::Color32::from_white_alpha(127)));
        assert_eq!(
            drawn[1].clip_rect,
            egui::Rect::from_min_max(egui::pos2(30.0, 40.0), egui::pos2(90.0, 80.0))
        );
        let egui::Shape::Mesh(other) = &drawn[2].shape else {
            panic!("other surfaces must still paint their own image")
        };
        assert_ne!(other.texture_id, target);
        assert!(matches!(drawn[3].shape, egui::Shape::Rect(_)));
    }

    #[test]
    fn fallback_preserves_origin_clip_and_animation_opacity_without_translucent_tint() {
        let context = egui::Context::default();
        let mut frame = Frame::default();
        frame.commands = vec![
            limn::DrawCommand::PushClip(limn::Rect::new(0.0, 0.0, 70.0, 70.0)),
            limn::DrawCommand::Glass {
                rect: limn::Rect::new(10.0, 10.0, 100.0, 40.0),
                radius: 20.0,
                material: GlassMaterial {
                    tint: limn::Color::rgba(20, 40, 60, 0),
                    blur: 8.0,
                    refraction: 12.0,
                    dispersion: 0.0,
                },
                opacity: 0.5,
            },
            limn::DrawCommand::PopClip,
        ];
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(400.0, 300.0),
                )),
                ..Default::default()
            },
            |ui| {
                crate::paint_at(ui, &frame, egui::pos2(20.0, 30.0));
            },
        );
        output.textures_delta.clear();
        let pane = output
            .shapes
            .iter()
            .find(|shape| {
                matches!(&shape.shape,
                    egui::Shape::Rect(rect) if rect.rect.min == egui::pos2(30.0, 40.0)
                )
            })
            .expect("glass should have a filled fallback");
        let egui::Shape::Rect(rect) = &pane.shape else {
            unreachable!()
        };
        assert_eq!(
            rect.fill,
            egui::Color32::from_rgba_unmultiplied(20, 40, 60, 128)
        );
        assert_eq!(
            pane.clip_rect,
            egui::Rect::from_min_max(egui::pos2(20.0, 30.0), egui::pos2(90.0, 100.0))
        );
        assert!(!output
            .shapes
            .iter()
            .any(|shape| matches!(shape.shape, egui::Shape::Callback(_))));
    }

    #[test]
    fn shader_validates_without_optional_gpu_capabilities() {
        let shader = wgpu::naga::front::wgsl::parse_str(include_str!("glass.wgsl"))
            .expect("glass shader should parse");
        wgpu::naga::valid::Validator::new(
            wgpu::naga::valid::ValidationFlags::all(),
            wgpu::naga::valid::Capabilities::empty(),
        )
        .validate(&shader)
        .expect("glass shader should validate");
        let uniforms = shader
            .types
            .iter()
            .find_map(|(_, ty)| (ty.name.as_deref() == Some("Parameters")).then_some(&ty.inner))
            .expect("glass parameter block should exist");
        assert!(
            matches!(uniforms, wgpu::naga::TypeInner::Struct { span: 96, .. }),
            "Rust upload must match the WGSL uniform layout"
        );
        let blur = wgpu::naga::front::wgsl::parse_str(BLUR_SHADER)
            .expect("backdrop blur shader should parse");
        wgpu::naga::valid::Validator::new(
            wgpu::naga::valid::ValidationFlags::all(),
            wgpu::naga::valid::Capabilities::empty(),
        )
        .validate(&blur)
        .expect("backdrop blur shader should validate");
    }

    #[test]
    fn blur_pyramid_preserves_logical_radius_across_monitor_scales() {
        let logical = [800.0, 400.0];
        assert!(blur_levels([800, 400], logical, 0.0).is_empty());
        assert!(blur_levels([800, 400], logical, 8.0).is_empty());
        assert_eq!(blur_levels([1600, 800], logical, 8.0), [[800, 400]]);
        assert_eq!(
            blur_levels([3200, 1600], logical, 8.0),
            [[1600, 800], [800, 400]]
        );
        assert_eq!(
            blur_levels([1600, 800], logical, 64.0),
            [[800, 400], [400, 200], [200, 100], [100, 50]]
        );
    }

    #[test]
    fn blur_pyramid_bounds_kernel_for_odd_tiny_and_stretched_backdrops() {
        for (size, logical, blur) in [
            ([801, 399], [400.5, 199.5], 64.0),
            ([4096, 1], [200.0, 1.0], 64.0),
            ([1, 1], [0.01, 0.01], 64.0),
            ([15, 7], [150.0, 70.0], 4.0),
        ] {
            let levels = blur_levels(size, logical, blur);
            let output = levels.last().copied().unwrap_or(size);
            assert!(levels.iter().all(|level| level[0] > 0 && level[1] > 0));
            assert!(levels.len() <= 12);
            for axis in 0..2 {
                let radius = blur * output[axis] as f32 / logical[axis];
                assert!(output[axis] == 1 || radius <= 8.0);
            }
        }
        assert_eq!(
            blur_levels([4096, 1], [200.0, 1.0], 64.0).last(),
            Some(&[16, 1]),
            "a one-pixel axis must not force the other axis to a single texel"
        );
        assert_eq!(
            blur_levels([800, 400], [800.0, 4000.0], 64.0).last(),
            Some(&[100, 400]),
            "a stretched axis should retain detail when no reduction is needed"
        );
    }

    #[test]
    fn backdrop_mapping_preserves_nonzero_origin_and_partial_overlap() {
        let backdrop = egui::Rect::from_min_size(egui::pos2(120.0, 80.0), egui::vec2(400.0, 200.0));
        let area = egui::Rect::from_min_size(egui::pos2(100.0, 100.0), egui::vec2(200.0, 60.0));
        assert_eq!(
            backdrop_mapping(area, backdrop),
            Some([-0.05, 0.1, 0.5, 0.3])
        );
    }

    #[test]
    fn backdrop_mapping_rejects_degenerate_and_nonfinite_geometry() {
        let good = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(300.0, 200.0));
        assert!(backdrop_mapping(good, egui::Rect::NOTHING).is_none());
        assert!(backdrop_mapping(
            good,
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(0.0, 10.0))
        )
        .is_none());
        assert!(backdrop_mapping(egui::Rect::EVERYTHING, good).is_none());
        let overflowing =
            egui::Rect::from_min_max(egui::pos2(-f32::MAX, 0.0), egui::pos2(f32::MAX, 10.0));
        assert!(backdrop_mapping(overflowing, good).is_none());
    }

    #[test]
    fn rgba_upload_requires_exact_nonzero_bounded_image() {
        assert!(valid_upload([2, 3], 24, 4096));
        assert!(!valid_upload([0, 3], 0, 4096));
        assert!(!valid_upload([2, 3], 23, 4096));
        assert!(!valid_upload([2, 3], 25, 4096));
        assert!(!valid_upload([4097, 1], 16388, 4096));
        assert!(!valid_upload([u32::MAX, u32::MAX], usize::MAX, u32::MAX));
    }
}
