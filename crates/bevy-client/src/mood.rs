//! Light and mood per area (docs/art-bible.md, "Light"), voxel mode only.
//!
//! - **Day areas** (Village, Kiln terrace): a warm sun of about 5200 K
//!   from high side-front, a soft blue sky fill, a faint cool rim (the
//!   smooth shader's), soft shadows.
//! - **Frozen Fields:** cooler and brighter, with a low sun and long
//!   shadows.
//! - **Marsh:** hazy and lower in contrast (less sun, more fill), with a
//!   yellow-green ground fog in the lowlands (`voxel_smooth.wesl`,
//!   `mode.z`) and a closer, greener distance haze.
//! - **Crystal Cave:** dark until the torch (`late::CaveCover` dims the sun
//!   and the fill); then the torch's warm light near the player and teal
//!   point lights at the nearest crystal clusters are the only light.
//! - **Kiln:** once the hearth is relit, the whole valley warms over a few
//!   seconds: a golden sun, a warmer fill and haze.
//!
//! The area under the player sets a target mood; the live one eases
//! towards it, so walking between areas never pops.

use bevy::light::{GlobalAmbientLight, NotShadowCaster};
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use realgame_voxel::{gen, mat, Biome};

use crate::late::{CaveCover, Late};
use crate::player::Player;
use crate::voxel_view::VoxWorld;

/// One mood: sun, sky fill and haze. Colours are sRGB components.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mood {
    pub sun: Vec3,
    pub sun_lux: f32,
    /// Sun elevation and azimuth (radians; azimuth 0 is from +z, south).
    pub elevation: f32,
    pub azimuth: f32,
    pub fill: Vec3,
    pub fill_lux: f32,
    pub haze: Vec3,
    /// Fog start and end (m).
    pub fog: Vec2,
    /// Ground fog in the lowlands (0-1).
    pub ground_fog: f32,
}

impl Mood {
    fn lerp(&self, o: &Mood, t: f32) -> Mood {
        Mood {
            sun: self.sun.lerp(o.sun, t),
            sun_lux: self.sun_lux + (o.sun_lux - self.sun_lux) * t,
            elevation: self.elevation + (o.elevation - self.elevation) * t,
            azimuth: self.azimuth + (o.azimuth - self.azimuth) * t,
            fill: self.fill.lerp(o.fill, t),
            fill_lux: self.fill_lux + (o.fill_lux - self.fill_lux) * t,
            haze: self.haze.lerp(o.haze, t),
            fog: self.fog.lerp(o.fog, t),
            ground_fog: self.ground_fog + (o.ground_fog - self.ground_fog) * t,
        }
    }
}

/// The day valley: warm sun (about 5200 K) from high in the south-west,
/// behind and beside a player facing the Kiln or the north.
const DAY: Mood = Mood {
    sun: Vec3::new(1.0, 0.89, 0.76),
    sun_lux: 10_000.0,
    elevation: 0.95,
    azimuth: -0.75,
    fill: Vec3::new(0.84, 0.88, 1.0),
    fill_lux: 1_800.0,
    haze: Vec3::new(0.86, 0.92, 0.96),
    fog: Vec2::new(150.0, 230.0),
    ground_fog: 0.0,
};

/// Cooler and brighter: a low, warm sun from the south-west for long
/// shadows across the snow and on the cliff faces that look south, a
/// gentler blue fill so the snow reads white (#EEF2F5) and the shadows
/// blue, and little haze (B4 round 3: the frame read as a blue-grey wash).
const FROZEN: Mood = Mood {
    sun: Vec3::new(1.0, 0.92, 0.80),
    sun_lux: 17_000.0,
    elevation: 0.42,
    azimuth: -0.75,
    fill: Vec3::new(0.78, 0.86, 1.0),
    fill_lux: 1_700.0,
    haze: Vec3::new(0.90, 0.94, 0.98),
    fog: Vec2::new(170.0, 300.0),
    ground_fog: 0.0,
};

/// Hazy, low contrast, yellow-green; the haze is pale enough that the
/// vents and reeds still stand out against it.
const MARSH: Mood = Mood {
    sun: Vec3::new(0.98, 0.94, 0.80),
    sun_lux: 7_600.0,
    elevation: 0.75,
    azimuth: -0.6,
    fill: Vec3::new(0.82, 0.86, 0.70),
    fill_lux: 1_500.0,
    haze: Vec3::new(0.74, 0.78, 0.55),
    fog: Vec2::new(45.0, 170.0),
    ground_fog: 0.7,
};

/// Underground: the cover dims sun and fill to almost nothing; the haze is
/// dark, so the far hall falls away into black.
const CAVE: Mood = Mood {
    sun: Vec3::new(0.9, 0.9, 1.0),
    sun_lux: 6_000.0,
    elevation: 0.8,
    azimuth: -0.6,
    fill: Vec3::new(0.45, 0.52, 0.70),
    fill_lux: 650.0,
    haze: Vec3::new(0.03, 0.04, 0.05),
    fog: Vec2::new(14.0, 70.0),
    ground_fog: 0.0,
};

/// What the relit Kiln does to the valley's light: dusk. A low orange sun
/// from the west, a dim warm fill and orange haze, so the hearth's glow,
/// flames and sparks are the brightest things in view (B4 round 3).
const KILN_WARM: Mood = Mood {
    sun: Vec3::new(1.0, 0.56, 0.30),
    sun_lux: 5_200.0,
    elevation: 0.20,
    azimuth: -1.35,
    fill: Vec3::new(0.95, 0.68, 0.62),
    fill_lux: 750.0,
    haze: Vec3::new(0.98, 0.64, 0.42),
    fog: Vec2::new(90.0, 220.0),
    ground_fog: 0.0,
};

/// How much of the sun and fill is left deep in the cave (spec 5.9: dim,
/// never black).
const CAVE_SUN: f32 = 0.06;
const CAVE_FILL: f32 = 0.08;

/// The target mood for an area.
pub fn area_mood(b: Biome) -> Mood {
    match b {
        Biome::Village | Biome::Kiln => DAY,
        Biome::Frozen => FROZEN,
        Biome::Marsh => MARSH,
        Biome::Cave => CAVE,
    }
}

/// The live mood, and the relit Kiln's warmth (0-1).
#[derive(Resource)]
struct Live {
    mood: Mood,
    warm: f32,
    /// Fog distances at `?low` are cut to the shorter view distance.
    low: bool,
    /// The player's position last frame.
    last: Option<Vec3>,
}

/// A teal point light that sits at one of the nearest crystal clusters.
#[derive(Component)]
struct CrystalLight;

/// Crystal clusters in the cave: centre and how many crystals.
#[derive(Resource, Default)]
struct Clusters(Vec<(Vec3, u32)>);

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, setup.after(crate::setup))
        .add_systems(Update, apply.after(crate::late::LightSet));
}

fn setup(mut commands: Commands, settings: Res<crate::Settings>, world: Option<Res<VoxWorld>>) {
    commands.insert_resource(Live {
        mood: DAY,
        warm: 0.0,
        low: settings.low,
        last: None,
    });
    // Crystal clusters: the cave's crystals binned on a 6 m grid.
    let mut bins: Vec<([i32; 3], Vec3, u32)> = Vec::new();
    if let Some(w) = world.as_ref() {
        for z in 10..70 {
            for x in 188..248 {
                for y in 1..27 {
                    if w.0.get([x, y, z]).mat != mat::CRYSTAL {
                        continue;
                    }
                    let key = [x.div_euclid(6), y.div_euclid(6), z.div_euclid(6)];
                    let p = Vec3::new(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5);
                    match bins.iter_mut().find(|b| b.0 == key) {
                        Some(b) => {
                            b.1 += p;
                            b.2 += 1;
                        }
                        None => bins.push((key, p, 1)),
                    }
                }
            }
        }
    }
    commands.insert_resource(Clusters(
        bins.into_iter()
            .filter(|b| b.2 >= 2)
            .map(|b| (b.1 / b.2 as f32, b.2))
            .collect(),
    ));
    let n = if settings.low { 3 } else { 5 };
    for _ in 0..n {
        commands.spawn((
            PointLight {
                color: Color::srgb(0.25, 1.0, 0.85),
                intensity: 0.0,
                range: 16.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::default(),
            CrystalLight,
            NotShadowCaster,
        ));
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn apply(
    time: Res<Time>,
    player: Res<Player>,
    late: Option<Res<Late>>,
    cave: Option<Res<CaveCover>>,
    mut live: ResMut<Live>,
    clusters: Res<Clusters>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut suns: Query<(&mut DirectionalLight, &mut Transform), Without<CrystalLight>>,
    mut fogs: Query<&mut DistanceFog>,
    mut crystal: Query<
        (&mut PointLight, &mut Transform),
        (With<CrystalLight>, Without<DirectionalLight>),
    >,
    chunks: Option<Res<crate::voxel_view::SmoothHandles>>,
    mut smooth: ResMut<Assets<crate::voxel_tex::SmoothMat>>,
    sky: Query<&MeshMaterial3d<crate::sky::SkyMat>>,
    mut sky_mats: ResMut<Assets<crate::sky::SkyMat>>,
) {
    let dt = time.delta_secs().min(0.1);
    let p = player.pos;
    let b = gen::biome_at([p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32]);
    let target = area_mood(b);
    let lit = late.as_ref().is_some_and(|l| l.kiln_lit);
    live.warm += ((if lit { 1.0 } else { 0.0 }) - live.warm) * (dt * 0.6).min(1.0);
    // A jump of more than 20 m in a frame (a respawn or a teleport) takes
    // the new area's light at once instead of easing in from the old one.
    let jumped = live.last.is_some_and(|q: Vec3| q.distance(p) > 20.0);
    live.last = Some(p);
    let k = if jumped { 1.0 } else { (dt * 1.2).min(1.0) };
    let mood = live.mood.lerp(&target, k);
    live.mood = mood;
    let mut m = mood.lerp(
        &KILN_WARM,
        live.warm * if b == Biome::Cave { 0.0 } else { 1.0 },
    );
    let cover = cave.map_or(0.0, |c| c.0);
    m.sun_lux *= 1.0 - (1.0 - CAVE_SUN) * cover;
    m.fill_lux *= 1.0 - (1.0 - CAVE_FILL) * cover;

    let srgb = |v: Vec3| Color::srgb(v.x, v.y, v.z);
    for (mut s, mut t) in &mut suns {
        s.illuminance = m.sun_lux;
        s.color = srgb(m.sun);
        let (se, ce) = m.elevation.sin_cos();
        let (sa, ca) = m.azimuth.sin_cos();
        let from = Vec3::new(sa * ce, se, ca * ce);
        // Only when it moves: a new sun transform re-renders its shadows.
        if t.translation.distance(from) > 1e-3 {
            *t = Transform::from_translation(from).looking_at(Vec3::ZERO, Vec3::Y);
        }
    }
    ambient.color = srgb(m.fill);
    ambient.brightness = m.fill_lux;
    let (fs, fe) = if live.low {
        (m.fog.x * 0.8, (m.fog.y * 0.83).min(192.0))
    } else {
        (m.fog.x, m.fog.y)
    };
    for mut f in &mut fogs {
        f.color = srgb(m.haze);
        f.falloff = FogFalloff::Linear { start: fs, end: fe };
    }
    // The ground fog and the sky's haze follow the mood.
    if let Some(h) = chunks.as_ref() {
        // (Touch an asset only when it changes: a changed material is
        // prepared and uploaded again.)
        for id in h.0.iter() {
            let stale = smooth
                .get(id)
                .is_some_and(|mat| (mat.extension.mode.z - m.ground_fog).abs() > 0.01);
            if let Some(mut mat) = stale.then(|| smooth.get_mut(id)).flatten() {
                mat.extension.mode.z = m.ground_fog;
            }
        }
    }
    let haze = m.haze.extend(cover);
    let (se, ce) = m.elevation.sin_cos();
    let (sa, ca) = m.azimuth.sin_cos();
    let sun = Vec4::new(sa * ce, se, ca * ce, live.warm);
    for handle in &sky {
        let stale = sky_mats.get(&handle.0).is_some_and(|s| {
            (s.params.haze - haze).abs().max_element() > 0.005
                || (s.params.sun - sun).abs().max_element() > 0.005
        });
        if let Some(mut s) = stale.then(|| sky_mats.get_mut(&handle.0)).flatten() {
            s.params.haze = haze;
            s.params.sun = sun;
        }
    }
    // Teal light at the nearest crystal clusters, only in the dark.
    let mut near: Vec<&(Vec3, u32)> = clusters.0.iter().collect();
    near.sort_by(|a, b| {
        a.0.distance_squared(p)
            .partial_cmp(&b.0.distance_squared(p))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for (i, (mut l, mut t)) in crystal.iter_mut().enumerate() {
        match near.get(i) {
            Some((c, n)) if cover > 0.01 && c.distance(p) < 30.0 => {
                // Teal light round each cluster, enough to colour the rock
                // round it (art bible: crystals light their surroundings).
                l.intensity = 60_000.0 * (*n as f32).min(6.0) * cover;
                t.translation = *c;
            }
            _ => l.intensity = 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_area_has_its_own_light() {
        let (day, frozen, marsh, cave) = (
            area_mood(Biome::Village),
            area_mood(Biome::Frozen),
            area_mood(Biome::Marsh),
            area_mood(Biome::Cave),
        );
        // Frozen: cooler, brighter, lower sun.
        assert!(frozen.sun.z > day.sun.z && frozen.sun_lux > day.sun_lux);
        assert!(frozen.elevation < day.elevation);
        // Marsh: less sun against more fill (lower contrast), green haze,
        // ground fog, closer fog.
        assert!(marsh.sun_lux / marsh.fill_lux < day.sun_lux / day.fill_lux);
        assert!(marsh.haze.y > marsh.haze.z && marsh.ground_fog > 0.5);
        assert!(marsh.fog.x < day.fog.x);
        // Cave: dark haze. Day: warm sun.
        assert!(cave.haze.length() < 0.1);
        assert!(day.sun.x > day.sun.z);
        // The relit Kiln warms the light.
        assert!(KILN_WARM.sun.z < day.sun.z && KILN_WARM.fill.x > day.fill.x);
    }
}
