@group(0) @binding(0) var<uniform> params: vec4<f32>;
@group(0) @binding(1) var content: texture_2d<f32>;
@group(0) @binding(2) var content_sampler: sampler;

struct Vertex { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> };

@vertex fn vertex_main(@builtin(vertex_index) index: u32) -> Vertex {
    let corners = array<vec2<f32>, 6>(vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0), vec2(0.0, 1.0), vec2(1.0, 0.0), vec2(1.0, 1.0));
    let uv = corners[index];
    return Vertex(vec4(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0), uv);
}

@fragment fn fragment_main(input: Vertex) -> @location(0) vec4<f32> {
    let phase = params.z;
    let scene = u32(params.w);
    let p = input.uv * params.xy;
    let drift = vec2(sin(phase) * 90.0, (cos(phase) - 1.0) * 55.0);
    let moving = p + drift;
    if scene == 5u {
        // Real limn text and icons, rendered once and moved on the GPU.
        return textureSampleLevel(content, content_sampler, (p + vec2(phase / 6.2831853 * 768.0, 0.0)) / vec2(768.0, 480.0), 0.0);
    }
    if scene == 2u {
        let cell = abs(fract(moving / 32.0 + 0.5) - 0.5) * 32.0;
        let line = 1.0 - smoothstep(0.5, 1.5, min(cell.x, cell.y));
        return vec4(mix(vec3(0.10, 0.16, 0.19), vec3(0.86, 0.91, 0.88), line), 1.0);
    }
    if scene == 3u {
        let cell = floor(moving / 42.0);
        let check = (u32(abs(cell.x + 10000.0)) + u32(abs(cell.y + 10000.0))) % 2u;
        return vec4(select(vec3(0.10, 0.11, 0.15), vec3(0.92, 0.79, 0.62), check == 0u), 1.0);
    }
    if scene == 4u {
        let center = params.xy * 0.5 + drift;
        let distance = length(p - center);
        let ring = 0.5 + 0.5 * cos(distance * 0.13 - phase);
        let color = vec3(0.35, 0.55, 0.65) + vec3(0.28, 0.28, 0.25) * cos(distance * 0.011 + vec3(0.0, 2.0, 4.0) + phase);
        return vec4(color * (0.4 + ring * 0.6), 1.0);
    }
    let uv = moving / params.xy;
    let band = 0.5 + 0.5 * sin((uv.x * 2.4 + uv.y * 0.55 + sin(uv.y * 3.0 + uv.x * 2.0 + phase) * 0.15) * 6.2831853);
    let low = select(vec3(0.153, 0.075, 0.063), vec3(0.035, 0.129, 0.212), scene == 1u);
    let high = select(vec3(0.824, 0.514, 0.275), vec3(0.349, 0.745, 0.718), scene == 1u);
    return vec4(mix(low, high, band * band), 1.0);
}
