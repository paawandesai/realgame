// Ray-traced atom impostor (gfx-lab, plan "Bench close-up rendering").
// The mesh is a camera-facing quad (atoms.rs `billboard`: +Z towards the
// camera, +X = cross(world up, +Z)). Each fragment rebuilds the sphere's
// centre from its UV, casts the view ray against the sphere, discards misses,
// then lights the hit point with Bevy's PBR code and writes its true depth,
// so atoms are perfectly round at any zoom and cut cleanly into bonds.
import bevy_pbr::render::{
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    pbr_fragment::pbr_input_from_standard_material,
    forward_io::VertexOutput,
    mesh_view_bindings::view,
};

struct AtomParams {
    // x: radius (m); y: quad half-size over radius.
    shape: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(100)
var<uniform> params: AtomParams;

struct AtomOut {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
}

@fragment
fn fragment(vertex_output: VertexOutput, @builtin(front_facing) is_front: bool) -> AtomOut {
    var in = vertex_output;
    let r = params.shape.x;
    let h = r * params.shape.y;
    let fwd = normalize(in.world_normal);
    let right = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), fwd));
    let up = cross(fwd, right);
    let local = vec2<f32>(in.uv.x * 2.0 - 1.0, 1.0 - in.uv.y * 2.0) * h;
    let c = in.world_position.xyz - right * local.x - up * local.y;

    let ro = view.world_position;
    let rd = normalize(in.world_position.xyz - ro);
    let oc = ro - c;
    let b = dot(oc, rd);
    let disc = b * b - (dot(oc, oc) - r * r);
    if disc < 0.0 {
        discard;
    }
    let t = -b - sqrt(disc);
    let hit = ro + rd * t;
    let n = normalize(hit - c);

    let clip = view.clip_from_world * vec4<f32>(hit, 1.0);
    let depth = clip.z / clip.w;

    in.world_position = vec4<f32>(hit, 1.0);
    in.world_normal = n;
    in.position = vec4<f32>(in.position.xy, depth, in.position.w);
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.world_position = vec4<f32>(hit, 1.0);
    pbr_input.frag_coord = vec4<f32>(in.position.xy, depth, in.position.w);
    pbr_input.world_normal = n;
    pbr_input.N = n;
    pbr_input.clearcoat_N = n;
    pbr_input.V = -rd;
    // A soft rim so atoms read against the dark room (plan: rim light).
    let rim = pow(1.0 - max(dot(n, -rd), 0.0), 3.0);
    pbr_input.material.emissive = vec4<f32>(pbr_input.material.base_color.rgb * rim * 0.6, 1.0);

    var out: AtomOut;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    out.depth = depth;
    return out;
}
