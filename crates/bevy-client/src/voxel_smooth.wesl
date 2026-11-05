// The smooth world's material (the "smooth open world" look): no
// textures at all. Each vertex carries its material's colour (rgb), its AO
// light (alpha), and in UV_0 how much it glows and how "grassy" it is.
// The fragment adds soft triplanar value noise (large patches and fine
// grain), turns grassy colours to rock on slopes, adds a cool rim light,
// glowing teal rune veins on glowing materials, then lights the result as
// StandardMaterial does (sun, ambient, shadows, fog).
//
// `params.mode.y`: 1 for `?low` (one noise octave, one water ripple).
// `params.mode.z`: the Marsh's yellow-green ground fog (0-1, from `mood`):
// it lies in the lowlands under y 33.4 and thickens with distance.
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
// The rim light: colour (sRGB) and strength (`mood`).
@group(constants::MATERIAL_BIND_GROUP) @binding(101)
var<uniform> rim_light: vec4<f32>;
// The ground fog: colour (sRGB) and the height it lies under (m).
@group(constants::MATERIAL_BIND_GROUP) @binding(102)
var<uniform> low_fog: vec4<f32>;
// The realistic look's scanned ground (`look`): x 1 once the arrays are
// set. Layers: 0 grass, 1 soil, 2 rock, 3 snow.
@group(constants::MATERIAL_BIND_GROUP) @binding(103)
var<uniform> ground: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(104)
var ground_albedo: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(105)
var ground_albedo_s: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(106)
var ground_normal: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(107)
var ground_normal_s: sampler;

// Metres one texture tile covers, per layer (grass, soil, rock, snow).
const TILE_M = vec4<f32>(2.2, 2.6, 7.0, 4.0);
// The soil layer's reference colour (dirt in the material table, linear):
// the vertex colour over it tints the scan a little. The other layers are
// pulled from their scan's mean colour (measured from the JPEGs) towards a
// real albedo derived from the vertex colour (`toward`), so marsh grass
// reads olive, valley grass green and snow white, with the photo's detail.
const REF_SOIL = vec3<f32>(0.30, 0.19, 0.11);

struct Ground {
    albedo: vec3<f32>,
    n: vec3<f32>,
}

fn unpack_n(c: vec4<f32>) -> vec3<f32> {
    let xy = c.xy * 2.0 - 1.0;
    return vec3<f32>(xy, sqrt(max(1.0 - dot(xy, xy), 0.0)));
}

// One layer, projected from above only (grass, soil, snow lie on flats).
fn top_layer(layer: i32, wp: vec3<f32>, n: vec3<f32>, tile: f32, dx: vec3<f32>, dy: vec3<f32>) -> Ground {
    let uv = wp.xz / tile;
    let gx = dx.xz / tile;
    let gy = dy.xz / tile;
    var g: Ground;
    g.albedo = textureSampleGrad(ground_albedo, ground_albedo_s, uv, layer, gx, gy).rgb;
    let t = unpack_n(textureSampleGrad(ground_normal, ground_normal_s, uv, layer, gx, gy));
    // Whiteout blend onto the surface normal (Y-up projection).
    g.n = normalize(vec3<f32>(t.x + n.x, abs(t.z) * n.y, -t.y + n.z));
    return g;
}

// One layer, projected along all three axes (rock on cliffs).
fn tri_layer(layer: i32, wp: vec3<f32>, n: vec3<f32>, tile: f32, dx: vec3<f32>, dy: vec3<f32>) -> Ground {
    var w = pow(abs(n), vec3<f32>(4.0));
    w = w / (w.x + w.y + w.z);
    let px = wp.zy / tile;
    let py = wp.xz / tile;
    let pz = wp.xy / tile;
    var g: Ground;
    let xx = dx.zy / tile;
    let xy = dy.zy / tile;
    let yx = dx.xz / tile;
    let yy = dy.xz / tile;
    let zx = dx.xy / tile;
    let zy = dy.xy / tile;
    g.albedo = textureSampleGrad(ground_albedo, ground_albedo_s, px, layer, xx, xy).rgb * w.x
        + textureSampleGrad(ground_albedo, ground_albedo_s, py, layer, yx, yy).rgb * w.y
        + textureSampleGrad(ground_albedo, ground_albedo_s, pz, layer, zx, zy).rgb * w.z;
    var tx = unpack_n(textureSampleGrad(ground_normal, ground_normal_s, px, layer, xx, xy));
    var ty = unpack_n(textureSampleGrad(ground_normal, ground_normal_s, py, layer, yx, yy));
    var tz = unpack_n(textureSampleGrad(ground_normal, ground_normal_s, pz, layer, zx, zy));
    tx = vec3<f32>(tx.xy + n.zy, abs(tx.z) * n.x);
    ty = vec3<f32>(ty.xy + n.xz, abs(ty.z) * n.y);
    tz = vec3<f32>(tz.xy + n.xy, abs(tz.z) * n.z);
    g.n = normalize(tx.zyx * w.x + ty.xzy * w.y + tz.xyz * w.z);
    return g;
}

fn tint(col: vec3<f32>, ref_col: vec3<f32>, k: f32) -> vec3<f32> {
    return clamp(pow(max(col, vec3<f32>(0.001)) / ref_col, vec3<f32>(k)), vec3<f32>(0.5), vec3<f32>(1.8));
}

// Shifts a scan's average colour towards a target (linear), keeping its
// detail: the grass scan is olive-brown, a living meadow is greener.
fn toward(t: vec3<f32>, mean: vec3<f32>, goal: vec3<f32>, k: f32) -> vec3<f32> {
    return t * mix(vec3<f32>(1.0), clamp(goal / mean, vec3<f32>(0.4), vec3<f32>(5.0)), k);
}

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

// The low ground fog (art bible, Marsh): yellow-green, lying under y
// 33.4, thicker with depth and distance.
fn ground_fog(c: vec4<f32>, wp: vec3<f32>) -> vec4<f32> {
    let amt = params.mode.z;
    if amt < 0.01 {
        return c;
    }
    let h = clamp((low_fog.w - wp.y) / 3.2, 0.0, 1.0);
    let d = distance(wp, view.world_position);
    let along = 1.0 - exp(-d * 0.06);
    let f = amt * h * (0.4 + 0.6 * h) * along * 0.55;
    // Pale yellow-green, so dark silhouettes (vents, reeds, posts) read
    // against it (B4 round 3: the old mustard haze flattened the hollow).
    let fog = pow(low_fog.rgb, vec3<f32>(2.2));
    return vec4<f32>(mix(c.rgb, fog, f), c.a);
}

@fragment
fn fragment(vertex_output: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var in = vertex_output;
    // Screen-space derivatives of the position, taken here in uniform
    // control flow, for the ground textures' mip selection.
    let wdx = dpdx(in.world_position.xyz);
    let wdy = dpdy(in.world_position.xyz);
    let mode = params.mode.x;
    let col = in.color.rgb;
    let ao = in.color.a;
    // A negative glow marks a wet, glossy surface (the Marsh's standing
    // pools, `built::puddle`): no glow, low roughness, a sky sheen.
    let wet = in.uv.x < -0.5;
    let glow = max(in.uv.x, 0.0);
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
        var r2 = r1;
        var r3 = 1.0 - r1;
        if params.mode.y < 0.5 {
            r2 = vnoise(vec3<f32>(wp.x * 1.7 - t * 0.5, t * 0.3 + 4.0, wp.z * 1.7 + t * 0.2));
            r3 = vnoise(vec3<f32>(wp.z * 0.6 - t * 0.3, 9.0, wp.x * 0.6));
        }
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
        // (No shimmer in the realistic look: only real light on water.)
        let crest = smoothstep(0.72, 0.9, r2 * 0.6 + r1 * 0.4) * (1.0 - ground.x);
        pbr_input.material.emissive = vec4<f32>(vec3<f32>(0.1, 0.5, 0.55) * crest * 0.6 + vec3<f32>(0.4, 0.55, 0.7) * fres * 0.25, 1.0);
        out.color = apply_pbr_lighting(pbr_input);
        out.color = main_pass_post_lighting_processing(pbr_input, out.color);
        out.color = ground_fog(out.color, wp);
        return out;
    }

    // Soft solid noise in world space (3D, so no projection seams): big
    // patches and a fine grain. `?low` (mode.y) takes one octave each.
    var big: f32;
    var fine: f32;
    var strata: f32;
    if params.mode.y > 0.5 {
        // `?low` (software GPUs, weak laptops): one octave for the patches,
        // and the grain only where it draws a rune vein.
        big = vnoise(wp * 0.25);
        fine = 0.5;
        if glow > 0.01 {
            fine = vnoise(wp * 1.6);
        }
        strata = 0.5;
    } else {
        big = fbm(wp * 0.18);
        fine = fbm(wp * 1.6 + 5.0);
        // Horizontal strata on steep rock.
        strata = vnoise(vec3<f32>(wp.x * 0.15, wp.y * 2.2, wp.z * 0.15));
    }

    // Grassy colours keep to the flat tops; slopes show warm rock.
    var base = col;
    let rock = mix(col * 0.45, vec3<f32>(0.32, 0.28, 0.26), 0.65) * (0.85 + 0.3 * strata);
    let flat_t = smoothstep(0.3, 0.6, n.y + (big - 0.5) * 0.3);
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
    // The realistic look: scanned ground on the terrain (not on buildings,
    // trees or anything that glows), chosen from what the vertex is:
    // snow (bright and grassy), grass (grassy and flat), soil (warm,
    // saturated or dark ground), rock (grey, or grass on steep slopes).
    let modelled = in.uv.x < -0.1 && in.uv.x > -0.5;
    if ground.x > 0.5 && mode < 0.5 && glow < 0.01 && !wet && !modelled {
        let lum = dot(col, vec3<f32>(0.2126, 0.7152, 0.0722));
        let sat = max(col.r, max(col.g, col.b)) - min(col.r, min(col.g, col.b));
        let steep = 1.0 - smoothstep(0.55, 0.8, n.y + (big - 0.5) * 0.2);
        let snow_w = grassy * smoothstep(0.45, 0.65, lum);
        let grass_w = grassy * (1.0 - snow_w) * (1.0 - steep);
        let soil_w = (1.0 - grassy) * max(smoothstep(0.1, 0.2, sat), 1.0 - smoothstep(0.08, 0.16, lum)) * (1.0 - steep * 0.6);
        let rock_w = max(1.0 - grass_w - snow_w - soil_w, 0.0);
        var acc = vec3<f32>(0.0);
        var nacc = vec3<f32>(0.0);
        if grass_w > 0.02 {
            let g = top_layer(0, wp, n, TILE_M.x, wdx, wdy);
            acc = acc + toward(g.albedo, vec3<f32>(0.076, 0.043, 0.006), col * 0.32, 0.7) * grass_w;
            nacc = nacc + g.n * grass_w;
        }
        if soil_w > 0.02 {
            let g = top_layer(1, wp, n, TILE_M.y, wdx, wdy);
            acc = acc + g.albedo * tint(col, REF_SOIL, 0.35) * soil_w;
            nacc = nacc + g.n * soil_w;
        }
        if rock_w > 0.02 {
            let g = tri_layer(2, wp, n, TILE_M.z, wdx, wdy);
            acc = acc + toward(g.albedo, vec3<f32>(0.157, 0.106, 0.03), col * 0.62, 0.45) * rock_w;
            nacc = nacc + g.n * rock_w;
        }
        if snow_w > 0.02 {
            let g = top_layer(3, wp, n, TILE_M.w, wdx, wdy);
            acc = acc + toward(g.albedo, vec3<f32>(0.247, 0.243, 0.302), col * 0.9, 0.8) * snow_w;
            nacc = nacc + g.n * snow_w;
        }
        let wsum = max(grass_w + soil_w + rock_w + snow_w, 0.001);
        // Large-scale variation, so the tiling doesn't show at a distance.
        base = acc / wsum * (0.85 + 0.3 * big);
        n = normalize(nacc);
        pbr_input.N = n;
        pbr_input.world_normal = n;
        pbr_input.clearcoat_N = n;
    }
    pbr_input.material.base_color = vec4<f32>(base * mix(1.0, ao, 0.85), alpha);
    pbr_input.diffuse_occlusion = vec3<f32>(mix(1.0, ao, 0.6));
    pbr_input.material.perceptual_roughness = select(0.88 - 0.2 * fine, 0.12, mode > 0.5);
    pbr_input.material.reflectance = vec3<f32>(select(0.25, 0.5, mode > 0.5));
    if wet {
        pbr_input.material.base_color = vec4<f32>(col * mix(1.0, ao, 0.5), 1.0);
        pbr_input.material.perceptual_roughness = 0.06;
        pbr_input.material.reflectance = vec3<f32>(0.7);
    }

    // Glow: veins of teal light (runes) and a steady core (crystal, torch,
    // the hearth), breathing slowly.
    let breathe = 0.8 + 0.2 * sin(globals.time * 1.3 + wp.x * 0.2 + wp.z * 0.15);
    let vein = smoothstep(0.022, 0.0, abs(fine - 0.5)) * glow;
    let teal = vec3<f32>(0.25, 1.0, 0.85);
    // Facets facing the eye glow brightest, so a crystal reads as a cut
    // gem and not a flat mint shape (B4 round 3 judges).
    let facing = max(dot(n, V), 0.0);
    let core = col * smoothstep(0.6, 1.0, glow) * (0.25 + 0.75 * facing * facing);
    var emit = (teal * vein * 1.1 + core * 0.75) * breathe;
    // Rim light: a cool sky edge on every silhouette.
    let rim = pow(1.0 - max(dot(n, V), 0.0), 3.0);
    emit = emit + pow(rim_light.rgb, vec3<f32>(2.2)) * rim * rim_light.w * ao;
    if wet {
        // The sky's sheen at grazing angles, and a faint ripple.
        let ripple = vnoise(vec3<f32>(wp.x * 2.0 + globals.time * 0.3, globals.time * 0.2, wp.z * 2.0));
        emit = emit + vec3<f32>(0.30, 0.38, 0.40) * pow(rim, 1.5) * (0.5 + 0.3 * ripple);
    }
    // Alpha 0: the glow is already in display units, so Bevy must not scale
    // it by the camera's exposure (about 1/1000 with the default EV100,
    // which left crystals, torches and runes looking unlit).
    pbr_input.material.emissive = vec4<f32>(emit, 0.0);

    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    out.color = ground_fog(out.color, wp);
    return out;
}
