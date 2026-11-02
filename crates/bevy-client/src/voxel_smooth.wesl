// The smooth world's material (the "smooth open world" look): no
// textures at all. Each vertex carries its material's colour (rgb), its AO
// light (alpha), and in UV_0 how much it glows and how "grassy" it is.
// The fragment adds soft triplanar value noise (large patches and fine
// grain), turns grassy colours to rock on slopes, adds a cool rim light,
// glowing teal rune veins on glowing materials, then lights the result as
// StandardMaterial does (sun, ambient, shadows, fog).
//
// `params.mode.x`: 0 opaque ground, 1 the clear pass (ice, glass, crystal,
// steam; UV_0.y is the alpha there), 2 water (animated ripples, fresnel).
import bevy_pbr::render::{
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    pbr_fragment::pbr_input_from_standard_material,
    forward_io::{VertexOutput, FragmentOutput},
    mesh_view_bindings::{view, globals},
};

struct SmoothParams {
    mode: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(100)
var<uniform> params: SmoothParams;

fn hash3(p: vec3<f32>) -> f32 {
    var q = fract(p * 0.3183099 + vec3<f32>(0.1, 0.2, 0.3));
    q = q * 17.0;
    return fract(q.x * q.y * q.z * (q.x + q.y + q.z));
}

fn vnoise(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = mix(mix(hash3(i + vec3<f32>(0.0, 0.0, 0.0)), hash3(i + vec3<f32>(1.0, 0.0, 0.0)), u.x),
                mix(hash3(i + vec3<f32>(0.0, 1.0, 0.0)), hash3(i + vec3<f32>(1.0, 1.0, 0.0)), u.x), u.y);
    let b = mix(mix(hash3(i + vec3<f32>(0.0, 0.0, 1.0)), hash3(i + vec3<f32>(1.0, 0.0, 1.0)), u.x),
                mix(hash3(i + vec3<f32>(0.0, 1.0, 1.0)), hash3(i + vec3<f32>(1.0, 1.0, 1.0)), u.x), u.y);
    return mix(a, b, u.z);
}

fn fbm(p: vec3<f32>) -> f32 {
    return vnoise(p) * 0.55 + vnoise(p * 2.03 + 7.1) * 0.3 + vnoise(p * 4.1 + 3.7) * 0.15;
}

@fragment
fn fragment(vertex_output: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var in = vertex_output;
    let mode = params.mode.x;
    let col = in.color.rgb;
    let ao = in.color.a;
    let glow = in.uv.x;
    let grassy = in.uv.y;
    let wp = in.world_position.xyz;
    var n = normalize(in.world_normal);
    if !is_front {
        n = -n;
    }
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let V = normalize(view.world_position - wp);
    var out: FragmentOutput;

    if mode > 1.5 {
        // Water: two drifting ripple layers bend the normal; deep blue-teal
        // looking down, sky-pale at grazing angles.
        let t = globals.time;
        let r1 = vnoise(vec3<f32>(wp.x * 0.6 + t * 0.35, t * 0.2, wp.z * 0.6));
        let r2 = vnoise(vec3<f32>(wp.x * 1.7 - t * 0.5, t * 0.3 + 4.0, wp.z * 1.7 + t * 0.2));
        let r3 = vnoise(vec3<f32>(wp.z * 0.6 - t * 0.3, 9.0, wp.x * 0.6));
        n = normalize(n + vec3<f32>((r1 - 0.5) * 0.35 + (r2 - 0.5) * 0.15, 0.0, (r3 - 0.5) * 0.35));
        pbr_input.N = n;
        pbr_input.world_normal = n;
        let fres = pow(1.0 - max(dot(n, V), 0.0), 4.0);
        let deep = vec3<f32>(0.03, 0.17, 0.30);
        let shallow = vec3<f32>(0.08, 0.42, 0.50);
        let base = mix(deep, shallow, 0.4 + 0.3 * r1);
        pbr_input.material.base_color = vec4<f32>(base, 0.62 + 0.3 * fres);
        pbr_input.material.perceptual_roughness = 0.08;
        pbr_input.material.reflectance = vec3<f32>(0.6);
        // A faint teal shimmer on the crests.
        let crest = smoothstep(0.72, 0.9, r2 * 0.6 + r1 * 0.4);
        pbr_input.material.emissive = vec4<f32>(vec3<f32>(0.1, 0.5, 0.55) * crest * 0.6 + vec3<f32>(0.4, 0.55, 0.7) * fres * 0.25, 1.0);
        out.color = apply_pbr_lighting(pbr_input);
        out.color = main_pass_post_lighting_processing(pbr_input, out.color);
        return out;
    }

    // Triplanar soft noise: big patches and a fine grain.
    var w = pow(abs(n), vec3<f32>(4.0));
    w = w / (w.x + w.y + w.z);
    let big = fbm(wp * 0.18) * w.y + fbm(wp.zyx * 0.18 + 11.0) * w.x + fbm(wp.xzy * 0.18 + 23.0) * w.z;
    let fine = fbm(wp * 1.6) * w.y + fbm(wp.zyx * 1.6 + 5.0) * w.x + fbm(wp.xzy * 1.6 + 13.0) * w.z;
    // Horizontal strata on steep rock.
    let strata = vnoise(vec3<f32>(wp.x * 0.15, wp.y * 2.2, wp.z * 0.15));

    // Grassy colours keep to the flat tops; slopes show warm rock.
    var base = col;
    let rock = mix(col * 0.45, vec3<f32>(0.32, 0.28, 0.26), 0.65) * (0.85 + 0.3 * strata);
    let flat_t = smoothstep(0.5, 0.78, n.y + (big - 0.5) * 0.35);
    base = mix(base, mix(rock, base, flat_t), grassy);
    // Patches: a sunlit yellow-green or a cool shade on grass, gentle
    // value changes elsewhere.
    let hue = big - 0.5;
    base = base * (0.84 + 0.32 * big) * (0.9 + 0.2 * fine);
    base = base + vec3<f32>(0.06, 0.04, -0.03) * hue * grassy * flat_t;
    // Non-grassy steep faces get a subtle banding.
    base = base * mix(1.0, 0.88 + 0.24 * strata, (1.0 - grassy) * (1.0 - abs(n.y)) * 0.6);

    var alpha = 1.0;
    if mode > 0.5 {
        alpha = grassy;
        base = col * (0.9 + 0.2 * fine);
    }
    pbr_input.material.base_color = vec4<f32>(base * mix(1.0, ao, 0.85), alpha);
    pbr_input.diffuse_occlusion = vec3<f32>(mix(1.0, ao, 0.6));
    pbr_input.material.perceptual_roughness = select(0.88 - 0.2 * fine, 0.12, mode > 0.5);
    pbr_input.material.reflectance = vec3<f32>(select(0.25, 0.5, mode > 0.5));

    // Glow: veins of teal light (runes) and a steady core (crystal, torch,
    // the hearth), breathing slowly.
    let breathe = 0.8 + 0.2 * sin(globals.time * 1.3 + wp.x * 0.2 + wp.z * 0.15);
    let vein = smoothstep(0.035, 0.0, abs(fine - 0.5)) * glow;
    let teal = vec3<f32>(0.25, 1.0, 0.85);
    let core = col * smoothstep(0.6, 1.0, glow);
    var emit = (teal * vein * 2.2 + core * 1.6) * breathe;
    // Rim light: a cool sky edge on every silhouette.
    let rim = pow(1.0 - max(dot(n, V), 0.0), 3.0);
    emit = emit + vec3<f32>(0.35, 0.5, 0.75) * rim * 0.12 * ao;
    pbr_input.material.emissive = vec4<f32>(emit, 1.0);

    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
