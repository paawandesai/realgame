// The block world's atlas material (I-052 preview, from I-054): the
// mesher's UVs are in blocks (so a greedy-merged quad covers several), and
// UV_1 carries the tile's origin in the 16 x 16 atlas. Each fragment wraps
// its block UV into that tile, then shades exactly as StandardMaterial does
// (the base colour and emissive textures are both sampled at `in.uv`).
import bevy_pbr::render::{
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
    pbr_fragment::pbr_input_from_standard_material,
    forward_io::{VertexOutput, FragmentOutput},
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
};

@fragment
fn fragment(vertex_output: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var in = vertex_output;
    // A hair inside the tile, so nearest sampling never reads a neighbour.
    in.uv = in.uv_b + (fract(in.uv) * 0.996 + 0.002) * (1.0 / 16.0);
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = alpha_discard(
        pbr_input.material.flags,
        pbr_input.material.alpha_cutoff,
        pbr_input.material.base_color,
    );
    var out: FragmentOutput;
    if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        out.color = apply_pbr_lighting(pbr_input);
    } else {
        out.color = pbr_input.material.base_color;
    }
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
