//! Publishes the run's result for the test script: `window.__gfxlab`, a JSON
//! string with the features, the adapter Bevy picked and frame times.
//! Frame interval is wall time between frames (`Time<Real>`), so on a
//! software GPU in the cloud it measures CPU plus software rasterising: good
//! for "does it run" and relative cost, not for real-hardware fps.

use bevy::prelude::*;
use bevy::render::renderer::RenderAdapterInfo;

use crate::Features;

const WARMUP: usize = 60;
const SAMPLES: usize = 180;

#[derive(Resource, Default)]
struct Sampler {
    frame: usize,
    ms: Vec<f32>,
    done: bool,
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Sampler>().add_systems(Last, sample);
}

fn sample(
    time: Res<Time<Real>>,
    mut s: ResMut<Sampler>,
    features: Res<Features>,
    adapter: Option<Res<RenderAdapterInfo>>,
) {
    if s.done {
        return;
    }
    s.frame += 1;
    if s.frame > WARMUP {
        let ms = time.delta_secs() * 1000.0;
        s.ms.push(ms);
    }
    if s.ms.len() < SAMPLES {
        return;
    }
    s.done = true;
    let mut v = s.ms.clone();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let pct = |p: f32| v[((v.len() - 1) as f32 * p) as usize];
    let (name, backend) = adapter
        .map(|a| (a.name.clone(), format!("{:?}", a.backend)))
        .unwrap_or_default();
    let json = format!(
        "{{\"features\":\"{}\",\"adapter\":\"{}\",\"backend\":\"{}\",\"frames\":{},\"p50_ms\":{:.2},\"p95_ms\":{:.2},\"max_ms\":{:.2}}}",
        features.0.join(","),
        name.replace('"', "'"),
        backend,
        v.len(),
        pct(0.5),
        pct(0.95),
        v.last().copied().unwrap_or(0.0)
    );
    info!("GFXLAB {json}");
    #[cfg(target_arch = "wasm32")]
    if let Some(w) = web_sys::window() {
        let _ = js_sys::Reflect::set(
            &w,
            &wasm_bindgen::JsValue::from_str("__gfxlab"),
            &wasm_bindgen::JsValue::from_str(&json),
        );
    }
}
