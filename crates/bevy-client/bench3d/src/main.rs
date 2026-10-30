use std::time::Instant;
use voxel_bench3d::World;
fn main() {
    for chunks in [1usize, 8, 32] {
        let mut w = World::new(chunks);
        w.step();
        let steps = 200;
        let t = Instant::now();
        for _ in 0..steps {
            w.step();
        }
        let ms = t.elapsed().as_secs_f64() * 1000.0 / steps as f64;
        println!(
            "native: {chunks:>2} chunks: {ms:.3} ms/step, {:.1} chunk-steps per ms",
            chunks as f64 / ms
        );
    }
}
