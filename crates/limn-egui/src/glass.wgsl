struct Parameters {
    area: vec4<f32>,
    viewport: vec4<f32>,
    mapping: vec4<f32>,
    tint: vec4<f32>,
    effects: vec4<f32>,
    options: vec4<f32>,
};
@group(0) @binding(0) var<uniform> params: Parameters;
@group(0) @binding(1) var backdrop: texture_2d<f32>;
@group(0) @binding(2) var backdrop_sampler: sampler;

struct Vertex {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex fn vertex_main(@builtin(vertex_index) index: u32) -> Vertex {
    let corners = array<vec2<f32>, 6>(
        vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0),
        vec2(0.0, 1.0), vec2(1.0, 0.0), vec2(1.0, 1.0),
    );
    let uv = corners[index];
    let point = params.area.xy + uv * params.area.zw;
    let clip = (point - params.viewport.xy) / params.viewport.zw * 2.0 - 1.0;
    return Vertex(vec4(clip.x, -clip.y, 0.0, 1.0), uv);
}

fn rounded_distance(point: vec2<f32>, half_size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(point) - half_size + vec2(radius);
    return length(max(q, vec2(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

fn sample_backdrop(uv: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(backdrop, backdrop_sampler, uv, 0.0);
}

fn linear_from_gamma(rgb: vec3<f32>) -> vec3<f32> {
    return select(pow((rgb + vec3(0.055)) / 1.055, vec3(2.4)), rgb / 12.92, rgb <= vec3(0.04045));
}

@fragment fn fragment_main(input: Vertex) -> @location(0) vec4<f32> {
    let half_size = params.area.zw * 0.5;
    let point = (input.uv - vec2(0.5)) * params.area.zw;
    let radius = params.effects.x;
    let distance = rounded_distance(point, half_size, radius);
    let aa = 0.8 / max(params.options.y, 0.1);
    let coverage = 1.0 - smoothstep(-aa, aa, distance);
    let q = abs(point) - half_size + vec2(radius);
    let outside = max(q, vec2(0.0));
    var normal = vec2(0.0, sign(point.y));
    if q.x > q.y { normal = vec2(sign(point.x), 0.0); }
    if dot(outside, outside) > 0.0001 { normal = normalize(outside) * sign(point); }
    // The rim is a convex bezel as wide as the corner radius, with the
    // squircle profile h = (1 - u^4)^(1/4): upright at the edge, flat inside.
    // In Apple's WWDC25 footage the rim is this thin and shows the middle of
    // the shape mirrored, which a displacement of one bezel width reproduces.
    let bezel = max(radius, 4.0);
    let depth = clamp(-distance / bezel, 0.0, 1.0);
    let u = 1.0 - depth;
    let slope = min(u * u * u / pow(max(1.0 - u * u * u * u, 0.0001), 0.75), 40.0);
    let surface_normal = normalize(vec3(normal * slope, 1.0));
    let ray = refract(vec3(0.0, 0.0, -1.0), surface_normal, 1.0 / 1.5);
    // The bend starts from nothing at the edge, so what is outside runs on
    // into the glass, squeezed, before the rim mirrors it: in Apple's footage
    // text and color enter a shape this way, like a drop. Dividing by 0.4,
    // the peak of this curve, makes `refraction` the largest displacement.
    let bend = ray.xy / max(-ray.z, 0.25) * smoothstep(0.0, 0.12, depth) / 0.4;
    let uv_per_point = params.mapping.zw / params.area.zw;
    // Further than this the lookup would cross the middle to the far side.
    let offset = bend * min(params.effects.z, bezel * 0.88) * uv_per_point;
    let uv = params.mapping.xy + input.uv * params.mapping.zw;
    let dispersion = bend * params.effects.w * uv_per_point;
    // The shared backdrop has already passed through the separable blur. The
    // rim squeezes it several times over, so one lookup there would alias and
    // leave hard bands; five along the bend keep it as soft as Apple's.
    var scene = vec4(0.0);
    for (var i = 0; i < 5; i += 1) {
        let at = uv + offset * (1.0 + (f32(i) - 2.0) * 0.08);
        var tap = sample_backdrop(at);
        if params.effects.w > 0.001 {
            tap.r = sample_backdrop(at + dispersion).r;
            tap.b = sample_backdrop(at - dispersion).b;
        }
        scene += tap * 0.2;
    }
    // Apple's web glass is `saturate(220%) blur(16px)` under the tint; the
    // weights are the CSS saturate() matrix, applied to gamma RGB as CSS does.
    let luma = dot(scene.rgb, vec3(0.213, 0.715, 0.072));
    scene = vec4(clamp(mix(vec3(luma), scene.rgb, 2.2), vec3(0.0), vec3(scene.a)), scene.a);
    let base_alpha = scene.a + params.tint.a * (1.0 - scene.a);
    // Texture samples are premultiplied; light acts on the straight color.
    // Keeping coverage separate prevents glowing RGB in fully transparent pixels.
    var rgb = mix(scene.rgb, params.tint.rgb, params.tint.a) / max(base_alpha, 0.00001);
    // The edge of iOS 27 in Apple's WWDC26 footage: a bright line a pixel
    // wide, strongest where the rim faces the light and on the side opposite,
    // and just inside it a faint darkened line that parts glass from content.
    let pixel = 1.0 / max(params.options.y, 0.1);
    let line = max(pixel, 0.75);
    let inset = -distance;
    let shade = smoothstep(0.0, line, inset) * (1.0 - smoothstep(line, line + 2.5, inset));
    rgb *= 1.0 - 0.14 * shade;
    let lighting = dot(normal, normalize(vec2(-0.55, -0.85)));
    let glint = (1.0 - smoothstep(0.0, line, inset)) * (0.25 + 0.65 * lighting * lighting);
    rgb = mix(rgb, vec3(1.0), glint);
    let alpha = base_alpha * coverage * params.options.x;
    // Decode straight gamma RGB before premultiplication for linear targets.
    if params.options.z > 0.5 { rgb = linear_from_gamma(rgb); }
    return vec4(rgb * alpha, alpha);
}
