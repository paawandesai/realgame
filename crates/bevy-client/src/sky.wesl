// The smooth world's sky: a stylised gradient (pale haze at the horizon
// matching the distance fog, deep blue overhead), a soft sun with a warm
// halo, and slow procedural clouds. No textures; drawn on a large sphere
// that follows the camera.
import bevy_pbr::render::{
    forward_io::VertexOutput,
    mesh_view_bindings::{view, globals},
};

fn h2(p: vec2<f32>) -> f32 {
    var q = fract(p * vec2<f32>(0.1031, 0.1030));
    q = q + dot(q, q.yx + 33.33);
    return fract((q.x + q.y) * q.x);
}

fn n2(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(h2(i), h2(i + vec2<f32>(1.0, 0.0)), u.x),
               mix(h2(i + vec2<f32>(0.0, 1.0)), h2(i + vec2<f32>(1.0, 1.0)), u.x), u.y);
}

fn fbm2(p: vec2<f32>) -> f32 {
    var s = 0.0;
    var a = 0.5;
    var q = p;
    for (var i = 0; i < 5; i = i + 1) {
        s = s + a * n2(q);
        q = q * 2.07 + vec2<f32>(1.7, 9.2);
        a = a * 0.5;
    }
    return s;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let d = normalize(in.world_position.xyz - view.world_position);
    let sun = normalize(vec3<f32>(-0.6, 1.0, 0.45));
    let up = clamp(d.y, -1.0, 1.0);
    let horizon = vec3<f32>(0.70, 0.85, 1.0);
    let mid = vec3<f32>(0.42, 0.66, 0.95);
    let zenith = vec3<f32>(0.16, 0.38, 0.80);
    var col = mix(horizon, mid, smoothstep(0.0, 0.25, up));
    col = mix(col, zenith, smoothstep(0.25, 0.9, up));
    // Below the horizon: the fog colour (the far ground fades into it).
    col = mix(col, horizon * 0.95, smoothstep(0.0, -0.1, up));
    // A faint teal aurora band low in the north (cultivation sky).
    let north = smoothstep(0.2, 1.0, -d.z) * smoothstep(0.05, 0.18, up) * smoothstep(0.45, 0.2, up);
    let wave = n2(vec2<f32>(d.x * 6.0 + globals.time * 0.03, up * 10.0));
    col = col + vec3<f32>(0.10, 0.45, 0.40) * north * wave * 0.35;
    // The sun: a disc, a halo, and warm light low around it.
    let s = max(dot(d, sun), 0.0);
    col = col + vec3<f32>(1.0, 0.9, 0.7) * (pow(s, 900.0) * 6.0 + pow(s, 40.0) * 0.35 + pow(s, 6.0) * 0.12);
    // Clouds on a plane overhead, drifting.
    if up > 0.0 {
        let p = d.xz / (up + 0.12) * 1.6 + vec2<f32>(globals.time * 0.012, globals.time * 0.004);
        let c = fbm2(p);
        let cover = smoothstep(0.5, 0.78, c) * smoothstep(0.0, 0.18, up);
        let lit = vec3<f32>(1.0, 0.98, 0.95) + vec3<f32>(0.25, 0.2, 0.1) * pow(s, 8.0);
        let shade = mix(lit, vec3<f32>(0.72, 0.78, 0.9), smoothstep(0.62, 0.95, c));
        col = mix(col, shade, cover * 0.85);
    }
    return vec4<f32>(col, 1.0);
}
