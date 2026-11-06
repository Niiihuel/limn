struct Parameters {
    area: vec4<f32>,
    viewport: vec4<f32>,
    mapping: vec4<f32>,
    tint: vec4<f32>,
    effects: vec4<f32>,
    options: vec4<f32>,
    optical: vec4<f32>,
    lens: vec4<f32>,
    // A press: where, as shares of the surface (xy), and how much light it brings (z).
    glow: vec4<f32>,
};
@group(0) @binding(0) var<uniform> params: Parameters;
@group(0) @binding(1) var backdrop: texture_2d<f32>;
@group(0) @binding(2) var backdrop_sampler: sampler;
@group(0) @binding(3) var sharp: texture_2d<f32>;

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
    let c = max(q, vec2(0.0));
    let capsule = radius >= min(half_size.x, half_size.y) - 0.5;
    let power = select(params.optical.z, 2.0, capsule);
    let corner = pow(pow(c.x, power) + pow(c.y, power), 1.0 / power);
    return corner + min(max(q.x, q.y), 0.0) - radius;
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
    if coverage <= 0.0 { return vec4(0.0); }
    let e = vec2(0.5, 0.0);
    let gradient = vec2(
        rounded_distance(point + e.xy, half_size, radius) - rounded_distance(point - e.xy, half_size, radius),
        rounded_distance(point + e.yx, half_size, radius) - rounded_distance(point - e.yx, half_size, radius));
    var normal = vec2(0.0, -1.0);
    if dot(gradient, gradient) > 0.00000001 { normal = normalize(gradient); }
    let bezel = max(select(radius, min(params.lens.x, radius), params.lens.x > 0.0), 4.0);
    let depth = clamp(-distance / bezel, 0.0, 1.0);
    let u = 1.0 - depth;
    let k = clamp(24.0 / bezel, 2.0, 4.0);
    let slope = min(pow(u, k - 1.0) / pow(max(1.0 - pow(u, k), 0.0001), (k - 1.0) / k), 40.0);
    let surface_normal = normalize(vec3(normal * slope, 1.0));
    let ray = refract(vec3(0.0, 0.0, -1.0), surface_normal, 1.0 / 1.5);
    // The bend starts from nothing at the edge, so what is outside runs on
    // into the glass, squeezed, before the rim mirrors it: in Apple's footage
    // text and color enter a shape this way, like a drop. Dividing by 0.4,
    // the peak of this curve, makes `refraction` the largest displacement.
    let bend = ray.xy / max(-ray.z, 0.25) * smoothstep(0.0, 0.12, depth) / 0.4;
    let uv_per_point = params.mapping.zw / params.area.zw;
    // Further than this the lookup would cross the middle to the far side.
    let offset = bend * min(params.effects.z, bezel * 0.88) * uv_per_point * params.optical.w;
    let uv = params.mapping.xy + ((input.uv - vec2(0.5)) / params.lens.y + vec2(0.5)) * params.mapping.zw;
    let dispersion = bend * params.effects.w * uv_per_point * params.optical.w;
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
    let rim_view = params.optical.w * pow(1.0 - depth, 1.6) * 0.9;
    if rim_view > 0.002 {
        var crisp = vec4(0.0);
        for (var i = 0; i < 3; i += 1) {
            let at = uv + offset * (0.88 + 0.08 * f32(i));
            var tap = textureSampleLevel(sharp, backdrop_sampler, at, 0.0);
            if params.effects.w > 0.001 {
                tap.r = textureSampleLevel(sharp, backdrop_sampler, at + dispersion, 0.0).r;
                tap.b = textureSampleLevel(sharp, backdrop_sampler, at - dispersion, 0.0).b;
            }
            crisp += tap / 3.0;
        }
        scene = mix(scene, crisp, rim_view);
    }
    let luma = dot(scene.rgb, vec3(0.213, 0.715, 0.072));
    scene = vec4(clamp(mix(vec3(luma), scene.rgb, params.optical.x), vec3(0.0), vec3(scene.a)) * (1.0 - params.optical.y), scene.a);
    let tint_alpha = params.tint.a * params.lens.z;
    let base_alpha = scene.a + tint_alpha * (1.0 - scene.a);
    // Texture samples are premultiplied; light acts on the straight color.
    // Keeping coverage separate prevents glowing RGB in fully transparent pixels.
    var rgb = mix(scene.rgb, params.tint.rgb, tint_alpha) / max(base_alpha, 0.00001);
    let pixel = 1.0 / max(params.options.y, 0.1);
    let line = max(pixel, 0.75);
    let inset = -distance;
    let shade = smoothstep(0.0, line, inset) * (1.0 - smoothstep(line, line + max(2.5, 0.25 * bezel), inset));
    rgb *= 1.0 - 0.14 * shade * params.optical.w;
    let facing = dot(normal, normalize(vec2(-0.55, -0.85)));
    let toward = pow(max(facing, 0.0), 1.5);
    let away = pow(max(-facing, 0.0), 2.0);
    let glint = (1.0 - smoothstep(0.0, line, inset)) * (0.10 + 0.45 * toward + 0.08 * away) * params.optical.w;
    rgb = mix(rgb, vec3(1.0), glint);
    let caught = exp(-inset / max(0.3 * bezel, 1.5)) * params.optical.w;
    let lit = 0.03 + 0.09 * toward + 0.02 * away;
    rgb = mix(rgb, vec3(1.0), caught * lit);
    var alpha = mix(base_alpha, 1.0, max(glint, caught * lit));
    // Pressed, the glass brightens around the press: a wide, soft lift, about 8% at full
    // strength, as aurora's interactive glass and Apple's.
    if params.glow.z > 0.001 {
        let size = params.area.zw;
        let off = (input.uv - params.glow.xy) * size;
        let reach = 0.75 * max(size.x, size.y);
        let lift = params.glow.z * (0.55 + 0.45 * exp(-dot(off, off) / (reach * reach)));
        rgb = mix(rgb, vec3(1.0), 0.14 * lift);
    }
    rgb = mix(rgb, vec3(1.0), params.lens.w * 0.12);
    alpha = mix(alpha, 1.0, params.lens.w * 0.084) * coverage * params.options.x;
    // Decode straight gamma RGB before premultiplication for linear targets.
    if params.options.z > 0.5 { rgb = linear_from_gamma(rgb); }
    return vec4(rgb * alpha, alpha);
}
