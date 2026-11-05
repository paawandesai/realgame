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
    /// The rest is used by the new look only (`look`, `?look=new`).
    /// The sky overhead (sRGB).
    pub sky_top: Vec3,
    /// The rim light on every silhouette (sRGB) and its strength.
    pub rim: Vec4,
    /// The ground fog's colour (sRGB) and the height it lies under (m).
    pub low_fog: Vec4,
    /// Colour grade: exposure (EV), saturation, temperature.
    pub grade: Vec3,
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
            sky_top: self.sky_top.lerp(o.sky_top, t),
            rim: self.rim.lerp(o.rim, t),
            low_fog: self.low_fog.lerp(o.low_fog, t),
            grade: self.grade.lerp(o.grade, t),
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
    sky_top: Vec3::new(0.16, 0.38, 0.80),
    rim: Vec4::new(0.35, 0.5, 0.75, 0.03),
    low_fog: Vec4::new(0.62, 0.66, 0.36, 33.4),
    grade: Vec3::new(0.0, 1.0, 0.0),
};

/// Cooler and brighter: a lowish, warm-white sun from the south-south-west
/// for long shadows across the snow and light on the cliff faces that look
/// south (the ice gate's), a soft, nearly neutral fill so the snow reads
/// white (#EEF2F5) and the shadows pale blue, and little haze (B4 round 3:
/// the frame read as a blue-grey wash).
const FROZEN: Mood = Mood {
    sun: Vec3::new(1.0, 0.96, 0.9),
    sun_lux: 15_000.0,
    elevation: 0.55,
    azimuth: -0.45,
    fill: Vec3::new(0.86, 0.9, 1.0),
    fill_lux: 2_300.0,
    haze: Vec3::new(0.90, 0.94, 0.98),
    fog: Vec2::new(170.0, 300.0),
    ground_fog: 0.0,
    sky_top: Vec3::new(0.16, 0.38, 0.80),
    rim: Vec4::new(0.35, 0.5, 0.75, 0.03),
    low_fog: Vec4::new(0.62, 0.66, 0.36, 33.4),
    grade: Vec3::new(0.0, 1.0, 0.0),
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
    sky_top: Vec3::new(0.16, 0.38, 0.80),
    rim: Vec4::new(0.35, 0.5, 0.75, 0.03),
    low_fog: Vec4::new(0.62, 0.66, 0.36, 33.4),
    grade: Vec3::new(0.0, 1.0, 0.0),
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
    sky_top: Vec3::new(0.16, 0.38, 0.80),
    rim: Vec4::new(0.35, 0.5, 0.75, 0.03),
    low_fog: Vec4::new(0.62, 0.66, 0.36, 33.4),
    grade: Vec3::new(0.0, 1.0, 0.0),
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
    sky_top: Vec3::new(0.16, 0.38, 0.80),
    rim: Vec4::new(0.35, 0.5, 0.75, 0.03),
    low_fog: Vec4::new(0.62, 0.66, 0.36, 33.4),
    grade: Vec3::new(0.0, 1.0, 0.0),
};

/// The realistic look's moods (`?look=new`; Paawan, 2026-11-04: "a game
/// that is real to this real world"). Real daylight: a near-white sun of
/// about 5500 K, blue sky fill, haze the colour of real air (the sky
/// shader's physical sky sets the sky itself); no rim or tinted grade.
const fn srgb(r: u8, g: u8, b: u8) -> Vec3 {
    Vec3::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
}

/// Village: a clear afternoon, sun in the south-west.
const NEW_VILLAGE: Mood = Mood {
    sun: Vec3::new(1.0, 0.95, 0.88),
    sun_lux: 11_000.0,
    elevation: 0.72,
    azimuth: -0.85,
    fill: Vec3::new(0.66, 0.77, 1.0),
    fill_lux: 1_900.0,
    haze: srgb(0xB4, 0xC6, 0xDC),
    fog: Vec2::new(140.0, 250.0),
    ground_fog: 0.0,
    sky_top: Vec3::ZERO,
    rim: Vec4::ZERO,
    low_fog: Vec4::new(0.85, 0.88, 0.92, 33.4),
    grade: Vec3::new(0.0, 1.0, 0.0),
};

/// Frozen Fields: a low winter sun, long shadows, cold clear air with a
/// little mist over the river.
const NEW_FROZEN: Mood = Mood {
    sun: Vec3::new(1.0, 0.94, 0.86),
    sun_lux: 13_000.0,
    elevation: 0.38,
    azimuth: -0.45,
    fill: Vec3::new(0.66, 0.78, 1.0),
    fill_lux: 2_400.0,
    haze: srgb(0xC4, 0xD2, 0xE2),
    fog: Vec2::new(150.0, 260.0),
    ground_fog: 0.3,
    sky_top: Vec3::ZERO,
    rim: Vec4::ZERO,
    low_fog: Vec4::new(0.88, 0.91, 0.95, 33.4),
    grade: Vec3::new(0.0, 1.0, 0.0),
};

/// Marsh: overcast and humid; soft light from the whole sky, grey-white
/// mist lying on the water.
const NEW_MARSH: Mood = Mood {
    sun: Vec3::new(0.96, 0.96, 0.94),
    sun_lux: 4_200.0,
    elevation: 0.9,
    azimuth: -0.6,
    fill: Vec3::new(0.80, 0.83, 0.86),
    fill_lux: 2_800.0,
    haze: srgb(0xB0, 0xB4, 0xAE),
    fog: Vec2::new(40.0, 170.0),
    ground_fog: 0.75,
    sky_top: Vec3::ZERO,
    rim: Vec4::ZERO,
    low_fog: Vec4::new(0.80, 0.82, 0.80, 33.4),
    grade: Vec3::new(0.0, 0.92, 0.0),
};

/// Crystal Cave: dark rock; only the crystals and the torch give light.
const NEW_CAVE: Mood = Mood {
    sun: Vec3::new(0.9, 0.9, 1.0),
    sun_lux: 6_000.0,
    elevation: 0.8,
    azimuth: -0.6,
    fill: Vec3::new(0.45, 0.52, 0.62),
    fill_lux: 650.0,
    haze: Vec3::new(0.02, 0.03, 0.035),
    fog: Vec2::new(14.0, 70.0),
    ground_fog: 0.0,
    sky_top: Vec3::ZERO,
    rim: Vec4::ZERO,
    low_fog: Vec4::new(0.3, 0.3, 0.3, 33.4),
    grade: Vec3::new(0.0, 1.0, 0.0),
};

/// Kiln terrace: the same afternoon as the Village, a little dust in the
/// air over the clay.
const NEW_KILN: Mood = Mood {
    haze: srgb(0xBE, 0xC4, 0xCC),
    ground_fog: 0.15,
    low_fog: Vec4::new(0.80, 0.76, 0.70, 33.4),
    ..NEW_VILLAGE
};

/// The relit Kiln: real evening light, a low orange sun.
const NEW_KILN_WARM: Mood = Mood {
    sun: Vec3::new(1.0, 0.62, 0.36),
    sun_lux: 6_000.0,
    elevation: 0.16,
    azimuth: -1.35,
    fill: Vec3::new(0.62, 0.62, 0.78),
    fill_lux: 900.0,
    haze: srgb(0xD8, 0xA8, 0x88),
    fog: Vec2::new(110.0, 240.0),
    ground_fog: 0.0,
    sky_top: Vec3::ZERO,
    rim: Vec4::ZERO,
    low_fog: Vec4::new(0.80, 0.66, 0.56, 33.4),
    grade: Vec3::new(0.0, 1.0, 0.0),
};

/// The target mood for an area in the new look.
pub fn area_mood_new(b: Biome) -> Mood {
    match b {
        Biome::Village => NEW_VILLAGE,
        Biome::Kiln => NEW_KILN,
        Biome::Frozen => NEW_FROZEN,
        Biome::Marsh => NEW_MARSH,
        Biome::Cave => NEW_CAVE,
    }
}

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
    /// The new look (`?look=new`).
    new: bool,
}

/// What the new look (`look`) updates each frame.
#[derive(bevy::ecs::system::SystemParam)]
pub struct NewLookParams<'w, 's> {
    grades: Query<'w, 's, &'static mut bevy::render::view::ColorGrading>,
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

fn setup(
    mut commands: Commands,
    settings: Res<crate::Settings>,
    world: Option<Res<VoxWorld>>,
    look: Option<Res<crate::look::NewLook>>,
) {
    let new = look.is_some_and(|l| l.0);
    commands.insert_resource(Live {
        mood: if new { NEW_VILLAGE } else { DAY },
        warm: 0.0,
        low: settings.low,
        last: None,
        new,
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
    mut new_look: NewLookParams,
) {
    let dt = time.delta_secs().min(0.1);
    let p = player.pos;
    let b = gen::biome_at([p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32]);
    let target = if live.new {
        area_mood_new(b)
    } else {
        area_mood(b)
    };
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
        if live.new { &NEW_KILN_WARM } else { &KILN_WARM },
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
        let rim = m.rim;
        let low_fog = m.low_fog;
        for id in h.0.iter() {
            let stale = smooth.get(id).is_some_and(|mat| {
                (mat.extension.mode.z - m.ground_fog).abs() > 0.01
                    || (mat.extension.rim - rim).abs().max_element() > 0.004
                    || (mat.extension.low_fog - low_fog).abs().max_element() > 0.004
            });
            if let Some(mut mat) = stale.then(|| smooth.get_mut(id)).flatten() {
                mat.extension.mode.z = m.ground_fog;
                mat.extension.rim = rim;
                mat.extension.low_fog = low_fog;
            }
        }
    }
    let haze = m.haze.extend(cover);
    let (se, ce) = m.elevation.sin_cos();
    let (sa, ca) = m.azimuth.sin_cos();
    let sun = Vec4::new(sa * ce, se, ca * ce, live.warm);
    let top = m.sky_top.extend(if live.new { 1.0 } else { 0.0 });
    for handle in &sky {
        let stale = sky_mats.get(&handle.0).is_some_and(|s| {
            (s.params.haze - haze).abs().max_element() > 0.005
                || (s.params.sun - sun).abs().max_element() > 0.005
                || (s.params.top - top).abs().max_element() > 0.005
        });
        if let Some(mut s) = stale.then(|| sky_mats.get_mut(&handle.0)).flatten() {
            s.params.haze = haze;
            s.params.sun = sun;
            s.params.top = top;
        }
    }
    if live.new {
        crate::look::apply(&m, &mut new_look.grades);
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
                l.intensity = 45_000.0 * (*n as f32).min(6.0) * cover;
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

    #[test]
    fn the_realistic_look_uses_real_daylight() {
        let v = area_mood_new(Biome::Village);
        let f = area_mood_new(Biome::Frozen);
        let m = area_mood_new(Biome::Marsh);
        let c = area_mood_new(Biome::Cave);
        // A near-white sun (no stylised tint) and a blue sky fill.
        for r in [v, f] {
            assert!(r.sun.min_element() > 0.8 && r.fill.z > r.fill.x);
            assert!(r.haze.z > r.haze.x, "air hazes blue");
            assert_eq!(r.rim.w, 0.0);
        }
        // Winter sun is low; the marsh is overcast (fill close to sun) and misty.
        assert!(f.elevation < v.elevation);
        assert!(m.fill_lux / m.sun_lux > v.fill_lux / v.sun_lux && m.ground_fog > 0.5);
        assert!(c.haze.length() < 0.1);
        assert!(NEW_KILN_WARM.sun.z < v.sun.z && NEW_KILN_WARM.elevation < v.elevation);
    }
}
