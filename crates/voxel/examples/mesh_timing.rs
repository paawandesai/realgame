//! Times smooth meshing per chunk and per part (surface, props, built
//! models) over the whole generated world: where the world's frame time
//! goes (graphics plan, world speed fix).
//!   cargo run --release -p realgame-voxel --example mesh_timing
use realgame_voxel::{smooth, World, CHUNKS};
use std::time::Instant;

fn main() {
    let t = Instant::now();
    let w = realgame_voxel::gen::generate(1);
    println!("generate: {:.1} ms", t.elapsed().as_secs_f64() * 1e3);
    let n = (CHUNKS[0] * CHUNKS[1] * CHUNKS[2]) as usize;
    let mut rows = vec![];
    let (mut verts, mut bytes) = (0usize, 0usize);
    let total = Instant::now();
    for c in 0..n {
        let t = Instant::now();
        let m = smooth::mesh_chunk_smooth(&w, c);
        let ms = t.elapsed().as_secs_f64() * 1e3;
        let v: usize = [&m.opaque, &m.clear, &m.water, &m.props, &m.props_far]
            .iter()
            .map(|d| d.positions.len())
            .sum();
        let i: usize = [&m.opaque, &m.clear, &m.water, &m.props, &m.props_far]
            .iter()
            .map(|d| d.indices.len())
            .sum();
        verts += v;
        bytes += v * (12 + 12 + 16) + i * 4;
        rows.push((ms, c, v, World::chunk_origin(c)));
    }
    let all = total.elapsed().as_secs_f64() * 1e3;
    rows.sort_by(|a, b| b.0.total_cmp(&a.0));
    println!("all {n} chunks: {all:.0} ms, mean {:.2} ms, {verts} verts, ~{:.1} MB as f32 pos+nrm+rgba+u32 idx", all / n as f64, bytes as f64 / 1e6);
    for r in rows.iter().take(12) {
        println!(
            "  chunk {:3} at {:?}: {:.2} ms, {} verts",
            r.1, r.3, r.0, r.2
        );
    }
    // Parts, on the slowest chunk.
    let c = rows[0].1;
    let o = World::chunk_origin(c);
    let hi = o.map(|v| v + realgame_voxel::CHUNK);
    let t = Instant::now();
    let _ = realgame_voxel::props::models(&w, o, hi);
    println!("props near: {:.2} ms", t.elapsed().as_secs_f64() * 1e3);
    let t = Instant::now();
    let _ = realgame_voxel::props::models_far(&w, o, hi);
    println!("props far: {:.2} ms", t.elapsed().as_secs_f64() * 1e3);
    let t = Instant::now();
    let _ = realgame_voxel::built::models(&w, o, hi);
    println!("built: {:.2} ms", t.elapsed().as_secs_f64() * 1e3);
}
