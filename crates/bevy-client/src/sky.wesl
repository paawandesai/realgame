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

struct SkyParams {
    low: vec4<f32>,
    sun: vec4<f32>,
    haze: vec4<f32>,
    top: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0)
var<uniform> params: SkyParams;

// The realistic look's sky: single scattering in a plane-parallel air
// column, Rayleigh (blue sky, red sunsets) plus Mie (the white glow round
// the sun and the pale horizon). Optical depths are air masses over a
// scale height (8 km air, 1.2 km haze); coefficients per km at sea level.
fn physical_sky(d: vec3<f32>, sun: vec3<f32>) -> vec3<f32> {
    let rayleigh = vec3<f32>(5.8e-3, 13.5e-3, 33.1e-3);
    let mie = 12e-3;
    let mu = dot(d, sun);
    let zen = max(d.y, 0.0);
    let od_r = 8.0 / (zen + 0.15);
    let od_m = 1.2 / (zen + 0.15);
    let sy = max(sun.y, 0.0);
    let sun_t = exp(-(rayleigh * (8.0 / (sy + 0.15)) + mie * 1.1 * (1.2 / (sy + 0.15))));
    let ext = rayleigh * od_r + vec3<f32>(mie * 1.1 * od_m);
    let t = exp(-ext);
    let pr = 3.0 / (16.0 * 3.14159265) * (1.0 + mu * mu);
    let g = 0.76;
    let pm = 3.0 / (8.0 * 3.14159265) * ((1.0 - g * g) * (1.0 + mu * mu))
        / ((2.0 + g * g) * pow(1.0 + g * g - 2.0 * g * mu, 1.5));
    let ins = (rayleigh * pr * od_r + vec3<f32>(mie * pm * od_m)) * sun_t * (1.0 - t) / ext;
    return ins * 30.0;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let d = normalize(in.world_position.xyz - view.world_position);
    let sun = normalize(params.sun.xyz);
    let warm = params.sun.w;
    let up = clamp(d.y, -1.0, 1.0);
    // The horizon is the area's haze (the distance fog's colour, sRGB
    // components from `mood`, here in linear light); overhead stays blue,
    // warmed a little when the Kiln is relit.
    let horizon = pow(params.haze.rgb, vec3<f32>(2.2)) * 1.1;
    // Relit Kiln: dusk. The band above the horizon turns orange and the
    // zenith a deep dusky blue-violet.
    var mid = mix(mix(vec3<f32>(0.42, 0.66, 0.95), horizon, 0.25), vec3<f32>(0.95, 0.42, 0.20), warm * 0.8);
    var zenith = mix(vec3<f32>(0.16, 0.38, 0.80), vec3<f32>(0.14, 0.12, 0.34), warm * 0.85);
    let new_look = params.top.w > 0.5;
    if new_look {
        // The new look: each region's own sky overhead, fading to its haze
        // through a warm band near the horizon (painted depth).
        zenith = pow(params.top.rgb, vec3<f32>(2.2));
        mid = mix(horizon, zenith, 0.45);
    }
    var col = mix(horizon, mid, smoothstep(0.0, 0.25, up));
    col = mix(col, zenith, smoothstep(0.25, 0.9, up));
    if new_look {
        // Real sky for this sun, settling into the fog's colour right at
        // the horizon so far hills fade into the same air.
        col = physical_sky(vec3<f32>(d.x, max(d.y, 0.0), d.z), sun);
        col = mix(col, horizon, smoothstep(0.08, 0.0, up) * 0.6);
    }
    // Below the horizon: the fog colour (the far ground fades into it).
    col = mix(col, horizon * 0.95, smoothstep(0.0, -0.1, up));
    // A faint teal aurora band low in the north (cultivation sky).
    let north = smoothstep(0.2, 1.0, -d.z) * smoothstep(0.05, 0.18, up) * smoothstep(0.45, 0.2, up);
    if new_look {
        // Only science glows in the new look: no aurora.
    } else if params.low.x < 0.5 {
        let wave = n2(vec2<f32>(d.x * 6.0 + globals.time * 0.03, up * 10.0));
        col = col + vec3<f32>(0.10, 0.45, 0.40) * north * wave * 0.35;
    } else {
        col = col + vec3<f32>(0.10, 0.45, 0.40) * north * 0.17;
    }
    // The sun: a disc, a halo, and warm light low around it.
    let s = max(dot(d, sun), 0.0);
    let sun_col = mix(vec3<f32>(1.0, 0.9, 0.7), vec3<f32>(1.0, 0.55, 0.25), warm);
    col = col + sun_col * (pow(s, 900.0) * 6.0 + pow(s, 40.0) * (0.35 + 0.5 * warm) + pow(s, 6.0) * (0.12 + 0.35 * warm));
    // Clouds on a plane overhead, drifting.
    if up > 0.0 {
        let p = d.xz / (up + 0.12) * 1.6 + vec2<f32>(globals.time * 0.012, globals.time * 0.004);
        var c: f32;
        if params.low.x > 0.5 {
            c = n2(p) * 0.75 + 0.12;
        } else {
            c = fbm2(p);
        }
        let cover = smoothstep(0.5, 0.78, c) * smoothstep(0.0, 0.18, up);
        let lit = mix(vec3<f32>(1.0, 0.98, 0.95), vec3<f32>(1.0, 0.62, 0.40), warm) + vec3<f32>(0.25, 0.2, 0.1) * pow(s, 8.0);
        let shade = mix(lit, mix(vec3<f32>(0.72, 0.78, 0.9), vec3<f32>(0.45, 0.32, 0.45), warm), smoothstep(0.62, 0.95, c));
        col = mix(col, shade, cover * 0.85);
    }
    // Underground the sky is never seen lit.
    col = col * (1.0 - 0.9 * params.haze.a);
    return vec4<f32>(col, 1.0);
}
