//! The courtyard: geometry, colliders, chemistry targets and their visuals.
//!
//! Everything is built from a handful of shared unit meshes (cube, faceted
//! sphere, cylinder, cone) scaled by `Transform`, and a small palette of
//! shared materials, so Bevy batches draws by mesh and material.
//!
//! Layout (metres, y up, the player starts at z = +18 looking toward -z):
//! the greenhouse fills the north side (x -8..8, z -22..-12); three chains of
//! bushes lead toward it, the first bush of each chain on fire. A dry hedge
//! runs along the west wall, a steam vent sits in the north-west, and in
//! the east an ice wall closes an alcove holding the lab bench (the future
//! water-splitting station). A loose puddle lies near the start.

use crate::chem::{self, Kind, Obj};
use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::math::Affine2;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use std::collections::{HashMap, HashSet};

pub const HALF: f32 = 24.0;
const WALL_H: f32 = 3.6;

/// An axis-aligned box.
#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn new(center: Vec3, half: Vec3) -> Self {
        Aabb {
            min: center - half,
            max: center + half,
        }
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Distance along the ray to the box's surface, if it hits within `max_t`.
    pub fn ray(&self, origin: Vec3, dir: Vec3, max_t: f32) -> Option<f32> {
        let inv = dir.recip();
        let t1 = (self.min - origin) * inv;
        let t2 = (self.max - origin) * inv;
        let tmin = t1.min(t2).max_element();
        let tmax = t1.max(t2).min_element();
        if tmax >= tmin.max(0.0) && tmin <= max_t {
            Some(tmin.max(0.0))
        } else {
            None
        }
    }
}

/// A solid box. `obj` ties it to a chemistry object; the box is then solid
/// only while that object says so (an ice wall stops blocking once melted).
#[derive(Clone, Copy, Debug)]
pub struct Collider {
    pub aabb: Aabb,
    pub obj: Option<usize>,
}

#[derive(Resource, Default)]
pub struct Colliders(pub Vec<Collider>);

impl Colliders {
    pub fn solid<'a>(&'a self, chem: &'a chem::World) -> impl Iterator<Item = &'a Aabb> + 'a {
        self.0.iter().filter_map(move |c| match c.obj {
            Some(i) if !solid_obj(&chem.objs[i]) => None,
            _ => Some(&c.aabb),
        })
    }
}

fn solid_obj(o: &Obj) -> bool {
    match o.kind {
        Kind::IceWall => o.wall,
        Kind::Puddle => false,
        _ => true,
    }
}

/// The chemistry world plus each object's picking box and its visuals.
#[derive(Resource)]
pub struct Chem {
    pub world: chem::World,
    pub targets: Vec<Target>,
    /// Kept for a reset (`R` or the `reset` command).
    pub initial: chem::World,
}

pub struct Target {
    pub name: &'static str,
    /// Picking box (ice wall: the standing wall; see [`Chem::bounds`]).
    pub aabb: Aabb,
    /// For an ice wall: the box of the puddle it melts into.
    pub melted_aabb: Option<Aabb>,
    /// Entities whose material shows the object's state.
    pub bodies: Vec<Entity>,
    /// Flame meshes shown while burning.
    pub flames: Vec<Entity>,
    /// Foam blanket shown while smothered.
    pub foam: Option<Entity>,
    /// Ice wall: the wall (scaled by remaining ice); puddles: the sheet.
    pub wall: Option<Entity>,
    pub puddle: Option<Entity>,
    pub mats: StateMats,
    pub last: Option<Look>,
}

impl Chem {
    pub fn bounds(&self, i: usize) -> Aabb {
        let t = &self.targets[i];
        match (t.melted_aabb, self.world.objs[i].wall) {
            (Some(m), false) => m,
            _ => t.aabb,
        }
    }
}

/// What an object looks like right now (defined in `cues`, which also
/// holds each look's emissive strength).
pub use crate::cues::Look;

pub fn look_of(o: &Obj) -> Look {
    if o.burning {
        Look::Burning
    } else if o.smothered > 0.0 {
        Look::Foamed
    } else if o.temp < -40.0 && !o.is_water() {
        Look::Frosty
    } else if o.charred() {
        Look::Charred
    } else if o.wet > 0.05 {
        Look::Wet
    } else if o.flammable() && o.temp > 120.0 {
        Look::Hot
    } else {
        Look::Normal
    }
}

#[derive(Clone)]
pub struct StateMats {
    pub normal: Handle<StandardMaterial>,
    pub hot: Handle<StandardMaterial>,
    pub burning: Handle<StandardMaterial>,
    pub charred: Handle<StandardMaterial>,
    pub wet: Handle<StandardMaterial>,
    pub frosty: Handle<StandardMaterial>,
    pub foamed: Handle<StandardMaterial>,
}

impl StateMats {
    pub fn get(&self, l: Look) -> Handle<StandardMaterial> {
        match l {
            Look::Normal => self.normal.clone(),
            Look::Hot => self.hot.clone(),
            Look::Burning => self.burning.clone(),
            Look::Charred => self.charred.clone(),
            Look::Wet => self.wet.clone(),
            Look::Frosty => self.frosty.clone(),
            Look::Foamed => self.foamed.clone(),
        }
    }
}

/// Shared meshes and materials.
#[derive(Resource, Clone)]
pub struct Kit {
    pub cube: Handle<Mesh>,
    pub ball: Handle<Mesh>,
    pub cyl: Handle<Mesh>,
    pub cone: Handle<Mesh>,
    pub water: Handle<StandardMaterial>,
    pub ice: Handle<StandardMaterial>,
    pub flame: Handle<StandardMaterial>,
    pub flame_core: Handle<StandardMaterial>,
    pub foam: Handle<StandardMaterial>,
}

fn flat(mut m: Mesh) -> Mesh {
    m.duplicate_vertices();
    m.compute_flat_normals();
    m
}

/// Tiny deterministic hash for procedural textures.
fn hash(x: u32, y: u32) -> f32 {
    let mut h = x.wrapping_mul(0x27d4_eb2d) ^ y.wrapping_mul(0x1656_67b1) ^ 0x9e37_79b9;
    h ^= h >> 15;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    (h & 0xffff) as f32 / 65535.0
}

/// A small repeating greyscale texture (the material's colour tints it).
fn texture(images: &mut Assets<Image>, size: u32, f: impl Fn(u32, u32) -> f32) -> Handle<Image> {
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let v = (f(x, y).clamp(0.0, 1.0) * 255.0) as u8;
            data.extend_from_slice(&[v, v, v, 255]);
        }
    }
    let mut img = Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        ..ImageSamplerDescriptor::linear()
    });
    images.add(img)
}

/// Stone blocks: two courses per tile, offset every other course, with soft
/// mortar lines and a little per-block brightness variation.
fn stone_texture(images: &mut Assets<Image>) -> Handle<Image> {
    texture(images, 64, |x, y| {
        let row = y / 32;
        let shift = if row % 2 == 1 { 16 } else { 0 };
        let col = ((x + shift) % 64) / 32;
        let (bx, by) = ((x + shift) % 32, y % 32);
        if bx < 2 || by < 2 {
            0.62
        } else {
            0.84 + 0.16 * hash(col + row * 7, 3) + 0.03 * hash(x, y)
        }
    })
}

/// Grass: soft blotches and speckles so the ground is never one flat colour.
fn grass_texture(images: &mut Assets<Image>) -> Handle<Image> {
    texture(images, 64, |x, y| {
        let blotch = hash(x / 16, y / 16) * 0.5 + hash(x / 8, y / 8) * 0.5;
        0.84 + 0.12 * blotch + 0.05 * hash(x, y)
    })
}

fn textured(
    materials: &mut Assets<StandardMaterial>,
    c: Color,
    tex: &Handle<Image>,
    repeat: Vec2,
) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: c,
        base_color_texture: Some(tex.clone()),
        uv_transform: Affine2::from_scale(repeat),
        perceptual_roughness: 0.9,
        reflectance: 0.2,
        ..default()
    })
}

fn toon(materials: &mut Assets<StandardMaterial>, c: Color) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: c,
        perceptual_roughness: 0.9,
        reflectance: 0.2,
        ..default()
    })
}

fn glow(materials: &mut Assets<StandardMaterial>, c: Color, k: f32) -> Handle<StandardMaterial> {
    let l = c.to_linear();
    materials.add(StandardMaterial {
        base_color: c,
        emissive: LinearRgba::rgb(l.red * k, l.green * k, l.blue * k),
        perceptual_roughness: 0.6,
        ..default()
    })
}

pub fn make_kit(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Kit {
    Kit {
        cube: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        ball: meshes.add(flat(Sphere::new(0.5).mesh().ico(1).expect("ico"))),
        cyl: meshes.add(flat(Cylinder::new(0.5, 1.0).mesh().resolution(10).build())),
        cone: meshes.add(flat(Cone::new(0.5, 1.0).mesh().resolution(7).build())),
        water: materials.add(StandardMaterial {
            base_color: Color::srgba(0.25, 0.55, 0.95, 0.85),
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 0.08,
            reflectance: 0.6,
            ..default()
        }),
        ice: materials.add(StandardMaterial {
            base_color: Color::srgb(0.72, 0.92, 1.0),
            emissive: LinearRgba::rgb(0.06, 0.16, 0.24),
            perceptual_roughness: 0.15,
            reflectance: 0.7,
            ..default()
        }),
        flame: glow(materials, Color::srgb(1.0, 0.45, 0.08), 6.0),
        flame_core: glow(materials, Color::srgb(1.0, 0.85, 0.3), 9.0),
        foam: toon(materials, Color::srgb(0.97, 0.98, 1.0)),
    }
}

fn state_mats(materials: &mut Assets<StandardMaterial>, base: Color, wet: Color) -> StateMats {
    StateMats {
        normal: toon(materials, base),
        // A heating bush looks scorched, not lit: no emissive below ignition.
        hot: glow(
            materials,
            Color::srgb(0.4, 0.25, 0.13),
            crate::cues::look_emissive(Look::Hot),
        ),
        burning: glow(
            materials,
            Color::srgb(0.55, 0.2, 0.05),
            crate::cues::look_emissive(Look::Burning),
        ),
        charred: toon(materials, Color::srgb(0.12, 0.1, 0.1)),
        wet: materials.add(StandardMaterial {
            base_color: wet,
            perceptual_roughness: 0.2,
            reflectance: 0.6,
            ..default()
        }),
        frosty: glow(
            materials,
            Color::srgb(0.8, 0.93, 1.0),
            crate::cues::look_emissive(Look::Frosty),
        ),
        foamed: toon(materials, Color::srgb(0.9, 0.93, 0.95)),
    }
}

/// `?low` only (I-024 Part R): one shared white `toon` material for the
/// untextured palette scenery, with each piece's colour moved into a
/// vertex-colour copy of its kit mesh. Every palette switch on WebGL2
/// rebinds the material group's textures and samplers; one material removes
/// those switches. Vertex colour x white base is the same product as the
/// palette base colour, so the picture does not change.
struct LowShare {
    shared: Handle<StandardMaterial>,
    /// Palette material -> its linear base colour.
    palette: HashMap<AssetId<StandardMaterial>, LinearRgba>,
    /// (kit mesh, colour bits) -> vertex-colour copy of that mesh.
    cache: HashMap<(AssetId<Mesh>, [u32; 4]), Handle<Mesh>>,
    /// Palette pieces spawned through `Builder::piece`; the end-of-`build`
    /// post-pass swaps those no `Target` references.
    candidates: Vec<(Entity, Handle<Mesh>, Handle<StandardMaterial>)>,
    /// Swapped entities.
    pieces: usize,
}

impl LowShare {
    fn new(
        materials: &mut Assets<StandardMaterial>,
        palette: &[(&Handle<StandardMaterial>, Color)],
    ) -> Self {
        LowShare {
            shared: toon(materials, Color::WHITE),
            palette: palette
                .iter()
                .map(|(h, c)| (h.id(), c.to_linear()))
                .collect(),
            cache: HashMap::new(),
            candidates: Vec::new(),
            pieces: 0,
        }
    }

    /// Swaps every candidate no `Target` references to the shared material
    /// and a vertex-colour copy of its mesh. Kit meshes are never edited in
    /// place (streams and projectiles draw them at runtime).
    fn apply(&mut self, commands: &mut Commands, meshes: &mut Assets<Mesh>, targets: &[Target]) {
        let mut used: HashSet<Entity> = HashSet::new();
        for t in targets {
            used.extend(t.bodies.iter().copied());
            used.extend(t.flames.iter().copied());
            used.extend(t.foam);
            used.extend(t.wall);
            used.extend(t.puddle);
        }
        for (e, mesh, mat) in std::mem::take(&mut self.candidates) {
            if used.contains(&e) {
                continue;
            }
            let Some(&c) = self.palette.get(&mat.id()) else {
                continue;
            };
            let key = (
                mesh.id(),
                [
                    c.red.to_bits(),
                    c.green.to_bits(),
                    c.blue.to_bits(),
                    c.alpha.to_bits(),
                ],
            );
            let coloured = match self.cache.get(&key) {
                Some(h) => h.clone(),
                None => {
                    let Some(src) = meshes.get(&mesh) else {
                        continue;
                    };
                    let mut m = src.clone();
                    let n = m.count_vertices();
                    m.insert_attribute(
                        Mesh::ATTRIBUTE_COLOR,
                        vec![[c.red, c.green, c.blue, c.alpha]; n],
                    );
                    let h = meshes.add(m);
                    self.cache.insert(key, h.clone());
                    h
                }
            };
            commands
                .entity(e)
                .insert((Mesh3d(coloured), MeshMaterial3d(self.shared.clone())));
            self.pieces += 1;
        }
    }
}

struct Builder<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    kit: Kit,
    colliders: Vec<Collider>,
    low: Option<LowShare>,
}

impl Builder<'_, '_, '_> {
    fn piece(
        &mut self,
        mesh: &Handle<Mesh>,
        mat: &Handle<StandardMaterial>,
        t: Transform,
    ) -> Entity {
        let e = self
            .commands
            .spawn((Mesh3d(mesh.clone()), MeshMaterial3d(mat.clone()), t))
            .id();
        if let Some(low) = &mut self.low {
            if low.palette.contains_key(&mat.id()) {
                low.candidates.push((e, mesh.clone(), mat.clone()));
            }
        }
        e
    }

    /// A solid box: drawn and collided.
    fn block(&mut self, center: Vec3, size: Vec3, mat: &Handle<StandardMaterial>) -> Entity {
        self.colliders.push(Collider {
            aabb: Aabb::new(center, size * 0.5),
            obj: None,
        });
        let cube = self.kit.cube.clone();
        self.piece(
            &cube,
            mat,
            Transform::from_translation(center).with_scale(size),
        )
    }

    fn tree(
        &mut self,
        x: f32,
        z: f32,
        s: f32,
        trunk: &Handle<StandardMaterial>,
        leaf: &Handle<StandardMaterial>,
    ) {
        let (cyl, ball, cone) = (
            self.kit.cyl.clone(),
            self.kit.ball.clone(),
            self.kit.cone.clone(),
        );
        self.piece(
            &cyl,
            trunk,
            Transform::from_xyz(x, 1.1 * s, z).with_scale(Vec3::new(0.45 * s, 2.2 * s, 0.45 * s)),
        );
        self.piece(
            &cone,
            leaf,
            Transform::from_xyz(x, 3.2 * s, z).with_scale(Vec3::new(2.8 * s, 2.6 * s, 2.8 * s)),
        );
        self.piece(
            &ball,
            leaf,
            Transform::from_xyz(x, 2.3 * s, z).with_scale(Vec3::splat(2.6 * s)),
        );
        self.colliders.push(Collider {
            aabb: Aabb::new(
                Vec3::new(x, 1.5 * s, z),
                Vec3::new(0.3 * s, 1.5 * s, 0.3 * s),
            ),
            obj: None,
        });
    }

    /// Hidden flame meshes for a burning object.
    fn flames(&mut self, spots: &[(Vec3, f32)]) -> Vec<Entity> {
        let (cone, flame, core) = (
            self.kit.cone.clone(),
            self.kit.flame.clone(),
            self.kit.flame_core.clone(),
        );
        let mut out = Vec::new();
        for &(p, s) in spots {
            let e = self
                .commands
                .spawn((
                    Mesh3d(cone.clone()),
                    MeshMaterial3d(flame.clone()),
                    Transform::from_translation(p).with_scale(Vec3::new(1.0, 1.6, 1.0) * s),
                    Visibility::Hidden,
                    NotShadowCaster,
                    NotShadowReceiver,
                ))
                .with_children(|c| {
                    c.spawn((
                        Mesh3d(cone.clone()),
                        MeshMaterial3d(core.clone()),
                        Transform::from_xyz(0.0, -0.12, 0.0).with_scale(Vec3::splat(0.55)),
                        NotShadowCaster,
                        NotShadowReceiver,
                    ));
                })
                .id();
            out.push(e);
        }
        out
    }

    fn foam_blob(&mut self, p: Vec3, s: Vec3) -> Entity {
        let (ball, foam) = (self.kit.ball.clone(), self.kit.foam.clone());
        self.commands
            .spawn((
                Mesh3d(ball),
                MeshMaterial3d(foam),
                Transform::from_translation(p).with_scale(s),
                Visibility::Hidden,
                NotShadowCaster,
            ))
            .id()
    }
}

/// Spawns the courtyard. Returns the colliders and the chemistry objects.
/// `low` (the `?low` tier) draws the untextured palette scenery with one
/// shared material (see [`LowShare`]).
pub fn build(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    low: bool,
) -> (Kit, Colliders, Chem) {
    let kit = make_kit(meshes, materials);
    commands.insert_resource(kit.clone());
    let stone_tex = stone_texture(images);
    let grass_tex = grass_texture(images);
    let grass = textured(
        materials,
        Color::srgb(0.46, 0.84, 0.38),
        &grass_tex,
        Vec2::splat(25.0),
    );
    // Stone tiles are 2 m wide with two 0.9 m courses each.
    let wall_tint = Color::srgb(0.84, 0.8, 0.92);
    let wall_long = textured(materials, wall_tint, &stone_tex, Vec2::new(25.0, 2.0));
    let wall_short = textured(materials, wall_tint, &stone_tex, Vec2::new(3.25, 2.0));
    let trim_c = Color::srgb(0.6, 0.55, 0.72);
    let cap_c = Color::srgb(0.95, 0.92, 1.0);
    let path_c = Color::srgb(0.93, 0.84, 0.66);
    let stone_c = Color::srgb(0.78, 0.74, 0.86);
    let stone_dark_c = Color::srgb(0.56, 0.52, 0.68);
    let wood_c = Color::srgb(0.62, 0.4, 0.24);
    let white_c = Color::srgb(0.96, 0.96, 0.98);
    let leaf_c = Color::srgb(0.25, 0.7, 0.42);
    let leaf2_c = Color::srgb(0.5, 0.82, 0.3);
    let metal_c = Color::srgb(0.32, 0.34, 0.4);
    let trim = toon(materials, trim_c);
    let cap = toon(materials, cap_c);
    let path = toon(materials, path_c);
    let stone = toon(materials, stone_c);
    let stone_dark = toon(materials, stone_dark_c);
    let wood = toon(materials, wood_c);
    let white = toon(materials, white_c);
    let leaf = toon(materials, leaf_c);
    let leaf2 = toon(materials, leaf2_c);
    let lamp = glow(materials, Color::srgb(1.0, 0.86, 0.5), 4.0);
    let metal = toon(materials, metal_c);
    let low_share = low.then(|| {
        LowShare::new(
            materials,
            &[
                (&trim, trim_c),
                (&cap, cap_c),
                (&path, path_c),
                (&stone, stone_c),
                (&stone_dark, stone_dark_c),
                (&wood, wood_c),
                (&white, white_c),
                (&leaf, leaf_c),
                (&leaf2, leaf2_c),
                (&metal, metal_c),
            ],
        )
    });
    let vent_hot = glow(materials, Color::srgb(1.0, 0.5, 0.2), 2.5);
    let flower = [
        glow(materials, Color::srgb(1.0, 0.4, 0.6), 0.4),
        glow(materials, Color::srgb(1.0, 0.85, 0.2), 0.4),
        glow(materials, Color::srgb(0.6, 0.5, 1.0), 0.4),
    ];
    let liquid = [
        glow(materials, Color::srgb(0.3, 1.0, 0.6), 2.0),
        glow(materials, Color::srgb(0.4, 0.7, 1.0), 2.0),
    ];
    let glass = materials.add(StandardMaterial {
        base_color: Color::srgba(0.75, 0.95, 1.0, 0.22),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.05,
        reflectance: 0.8,
        ..default()
    });
    // Collected gas in the electrolysis tubes: a warm pale yellow so it reads
    // apart from the blue water and the pale lavender alcove wall.
    let gas = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.85, 0.35, 0.6),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.4,
        ..default()
    });

    let mut b = Builder {
        commands,
        kit: kit.clone(),
        colliders: Vec::new(),
        low: low_share,
    };
    let cube = kit.cube.clone();
    let ball = kit.ball.clone();
    let cyl = kit.cyl.clone();

    // Ground and paths.
    b.piece(
        &cube,
        &grass,
        Transform::from_xyz(0.0, -0.25, 0.0).with_scale(Vec3::new(
            2.0 * HALF + 2.0,
            0.5,
            2.0 * HALF + 2.0,
        )),
    );
    b.piece(
        &cube,
        &path,
        Transform::from_xyz(0.0, 0.01, 3.0).with_scale(Vec3::new(3.0, 0.02, 30.0)),
    );
    b.piece(
        &cube,
        &path,
        Transform::from_xyz(0.0, 0.012, 8.0).with_scale(Vec3::new(40.0, 0.02, 3.0)),
    );

    // Perimeter walls with pillars and lanterns. Each wall is the same box
    // (long side along x), turned for east and west so the stone texture runs
    // the same way on all four; a darker plinth and a light cap break up the
    // face.
    let wl = 2.0 * HALF + 2.0;
    for (c, turn) in [
        (Vec3::new(0.0, WALL_H / 2.0, -HALF - 0.5), false),
        (Vec3::new(0.0, WALL_H / 2.0, HALF + 0.5), false),
        (Vec3::new(-HALF - 0.5, WALL_H / 2.0, 0.0), true),
        (Vec3::new(HALF + 0.5, WALL_H / 2.0, 0.0), true),
    ] {
        let size = Vec3::new(wl, WALL_H, 1.0);
        let rot = Quat::from_rotation_y(if turn {
            std::f32::consts::FRAC_PI_2
        } else {
            0.0
        });
        let half = if turn {
            Vec3::new(0.5, WALL_H / 2.0, wl / 2.0)
        } else {
            size * 0.5
        };
        b.colliders.push(Collider {
            aabb: Aabb::new(c, half),
            obj: None,
        });
        b.piece(
            &cube,
            &wall_long,
            Transform::from_translation(c)
                .with_rotation(rot)
                .with_scale(size),
        );
        b.piece(
            &cube,
            &trim,
            Transform::from_translation(Vec3::new(c.x, 0.25, c.z))
                .with_rotation(rot)
                .with_scale(Vec3::new(wl, 0.5, 1.16)),
        );
        b.piece(
            &cube,
            &cap,
            Transform::from_translation(Vec3::new(c.x, WALL_H + 0.08, c.z))
                .with_rotation(rot)
                .with_scale(Vec3::new(wl, 0.16, 1.3)),
        );
    }
    for k in 0..7 {
        let a = -HALF + k as f32 * 8.0;
        for (x, z) in [(a, -HALF), (a, HALF), (-HALF, a), (HALF, a)] {
            b.piece(
                &cube,
                &stone_dark,
                Transform::from_xyz(x, WALL_H / 2.0 + 0.3, z).with_scale(Vec3::new(
                    1.4,
                    WALL_H + 0.6,
                    1.4,
                )),
            );
            b.piece(
                &ball,
                &lamp,
                Transform::from_xyz(x, WALL_H + 0.9, z).with_scale(Vec3::splat(0.55)),
            );
        }
    }

    // Greenhouse: glass panes on a white frame, a wooden knee wall at the
    // front (that is what can catch fire), a door gap, two trees inside.
    let (gx0, gx1, gz0, gz1, gh) = (-8.0f32, 8.0f32, -22.0f32, -12.0f32, 4.0f32);
    let pane = |b: &mut Builder, c: Vec3, s: Vec3| {
        b.colliders.push(Collider {
            aabb: Aabb::new(c, s * 0.5),
            obj: None,
        });
        b.commands.spawn((
            Mesh3d(cube.clone()),
            MeshMaterial3d(glass.clone()),
            Transform::from_translation(c).with_scale(s),
            NotShadowCaster,
        ));
    };
    pane(
        &mut b,
        Vec3::new(gx0, gh / 2.0, (gz0 + gz1) / 2.0),
        Vec3::new(0.1, gh, gz1 - gz0),
    );
    pane(
        &mut b,
        Vec3::new(gx1, gh / 2.0, (gz0 + gz1) / 2.0),
        Vec3::new(0.1, gh, gz1 - gz0),
    );
    pane(
        &mut b,
        Vec3::new(0.0, gh / 2.0, gz0),
        Vec3::new(gx1 - gx0, gh, 0.1),
    );
    // Front: glass above the knee wall, either side of the door.
    for (x0, x1) in [(gx0, -1.5f32), (1.5f32, gx1)] {
        let cx = (x0 + x1) / 2.0;
        let w = x1 - x0;
        pane(
            &mut b,
            Vec3::new(cx, 0.9 + (gh - 0.9) / 2.0, gz1),
            Vec3::new(w, gh - 0.9, 0.1),
        );
    }
    // Lintel over the door.
    b.piece(
        &cube,
        &white,
        Transform::from_xyz(0.0, 3.0, gz1).with_scale(Vec3::new(3.2, 0.2, 0.25)),
    );
    pane(&mut b, Vec3::new(0.0, 3.5, gz1), Vec3::new(3.0, 1.0, 0.1));
    // Roof: two glass slopes up to a ridge along x.
    let ridge = 1.8f32;
    let half_d = (gz1 - gz0) / 2.0;
    let slope = (ridge / half_d).atan();
    let len = (ridge * ridge + half_d * half_d).sqrt();
    for (sz, sign) in [(gz1 - half_d / 2.0, 1.0f32), (gz0 + half_d / 2.0, -1.0f32)] {
        b.commands.spawn((
            Mesh3d(cube.clone()),
            MeshMaterial3d(glass.clone()),
            Transform::from_xyz(0.0, gh + ridge / 2.0, sz)
                .with_rotation(Quat::from_rotation_x(slope * sign))
                .with_scale(Vec3::new(gx1 - gx0 + 0.2, 0.08, len)),
            NotShadowCaster,
        ));
    }
    // Frame: posts and beams.
    for k in 0..=4 {
        let x = gx0 + k as f32 * 4.0;
        for z in [gz0, gz1] {
            b.piece(
                &cube,
                &white,
                Transform::from_xyz(x, gh / 2.0, z).with_scale(Vec3::new(0.22, gh, 0.22)),
            );
        }
    }
    for k in 1..=2 {
        let z = gz0 + k as f32 * (gz1 - gz0) / 3.0;
        for x in [gx0, gx1] {
            b.piece(
                &cube,
                &white,
                Transform::from_xyz(x, gh / 2.0, z).with_scale(Vec3::new(0.22, gh, 0.22)),
            );
        }
    }
    for z in [gz0, gz1] {
        b.piece(
            &cube,
            &white,
            Transform::from_xyz(0.0, gh, z).with_scale(Vec3::new(gx1 - gx0 + 0.3, 0.25, 0.25)),
        );
    }
    b.piece(
        &cube,
        &white,
        Transform::from_xyz(0.0, gh + ridge, (gz0 + gz1) / 2.0).with_scale(Vec3::new(
            gx1 - gx0 + 0.3,
            0.25,
            0.25,
        )),
    );
    // Trees and flower beds inside.
    b.tree(-4.0, -18.0, 1.0, &wood, &leaf);
    b.tree(4.0, -19.0, 0.85, &wood, &leaf2);
    for (k, x) in [-5.5f32, -2.0, 2.0, 5.5].into_iter().enumerate() {
        b.block(Vec3::new(x, 0.3, -14.2), Vec3::new(2.4, 0.6, 1.2), &wood);
        for j in 0..3 {
            b.piece(
                &ball,
                &flower[(k + j) % 3],
                Transform::from_xyz(x - 0.7 + j as f32 * 0.7, 0.8, -14.2)
                    .with_scale(Vec3::splat(0.45)),
            );
        }
    }

    // Decor: two courtyard trees, benches, a fountain.
    b.tree(-18.0, 19.0, 1.2, &wood, &leaf);
    b.tree(18.5, 18.5, 1.1, &wood, &leaf2);
    b.tree(-19.0, -18.0, 1.0, &wood, &leaf2);
    for (x, z) in [(-6.0, 14.0), (6.0, 14.0)] {
        b.block(Vec3::new(x, 0.45, z), Vec3::new(2.4, 0.15, 0.7), &wood);
        b.piece(
            &cube,
            &stone_dark,
            Transform::from_xyz(x - 1.0, 0.2, z).with_scale(Vec3::new(0.2, 0.4, 0.6)),
        );
        b.piece(
            &cube,
            &stone_dark,
            Transform::from_xyz(x + 1.0, 0.2, z).with_scale(Vec3::new(0.2, 0.4, 0.6)),
        );
    }
    b.colliders.push(Collider {
        aabb: Aabb::new(Vec3::new(0.0, 0.35, 3.0), Vec3::new(1.6, 0.35, 1.6)),
        obj: None,
    });
    b.piece(
        &cyl,
        &stone,
        Transform::from_xyz(0.0, 0.35, 3.0).with_scale(Vec3::new(3.2, 0.7, 3.2)),
    );
    b.piece(
        &cyl,
        &kit.water,
        Transform::from_xyz(0.0, 0.71, 3.0).with_scale(Vec3::new(2.7, 0.02, 2.7)),
    );
    b.piece(
        &cyl,
        &stone,
        Transform::from_xyz(0.0, 1.2, 3.0).with_scale(Vec3::new(0.5, 1.6, 0.5)),
    );
    b.piece(
        &ball,
        &liquid[1],
        Transform::from_xyz(0.0, 2.1, 3.0).with_scale(Vec3::splat(0.6)),
    );

    // East alcove for the lab bench, closed by the ice wall.
    b.block(
        Vec3::new(20.75, WALL_H / 2.0, 2.7),
        Vec3::new(6.5, WALL_H, 0.6),
        &wall_short,
    );
    b.block(
        Vec3::new(20.75, WALL_H / 2.0, 9.3),
        Vec3::new(6.5, WALL_H, 0.6),
        &wall_short,
    );
    b.piece(
        &cube,
        &stone_dark,
        Transform::from_xyz(20.75, WALL_H + 0.1, 6.0).with_scale(Vec3::new(6.5, 0.2, 7.2)),
    );

    // --- Chemistry objects -------------------------------------------------
    let mut objs: Vec<Obj> = Vec::new();
    let mut targets: Vec<Target> = Vec::new();

    let bush_mats = state_mats(
        materials,
        Color::srgb(0.2, 0.66, 0.3),
        Color::srgb(0.1, 0.42, 0.28),
    );
    let hedge_mats = state_mats(
        materials,
        Color::srgb(0.82, 0.7, 0.32),
        Color::srgb(0.52, 0.46, 0.26),
    );
    let gh_mats = state_mats(
        materials,
        Color::srgb(0.62, 0.4, 0.24),
        Color::srgb(0.42, 0.28, 0.2),
    );
    let plain = |m: Handle<StandardMaterial>| StateMats {
        normal: m.clone(),
        hot: m.clone(),
        burning: m.clone(),
        charred: m.clone(),
        wet: m.clone(),
        frosty: m.clone(),
        foamed: m,
    };

    let mut add = |objs: &mut Vec<Obj>, t: Target, o: Obj| {
        objs.push(o);
        targets.push(t);
        objs.len() - 1
    };

    // Bushes: three chains toward the greenhouse; the first of each burns.
    let bushes = [
        (-13.5f32, -3.0f32, true),
        (-12.0, -6.5, false),
        (-9.6, -10.2, false),
        (13.5, -3.0, true),
        (12.0, -6.5, false),
        (9.6, -10.2, false),
        (0.0, -5.0, true),
        (0.0, -9.2, false),
    ];
    let mut burning_at_start = Vec::new();
    for (x, z, lit) in bushes {
        let r = 0.9;
        let mut bodies = Vec::new();
        for (dx, dy, dz, s) in [
            (0.0, 0.75, 0.0, 1.0f32),
            (0.45, 0.5, 0.3, 0.7),
            (-0.4, 0.55, -0.25, 0.75),
        ] {
            bodies.push(b.piece(
                &ball,
                &bush_mats.normal,
                Transform::from_xyz(x + dx, dy, z + dz).with_scale(Vec3::new(1.9, 1.6, 1.9) * s),
            ));
        }
        let flames = b.flames(&[
            (Vec3::new(x, 1.9, z), 1.0),
            (Vec3::new(x + 0.45, 1.5, z + 0.3), 0.7),
        ]);
        let foam = Some(b.foam_blob(Vec3::new(x, 0.95, z), Vec3::new(2.1, 1.5, 2.1)));
        let aabb = Aabb::new(Vec3::new(x, 0.8, z), Vec3::new(r + 0.05, 0.8, r + 0.05));
        b.colliders.push(Collider { aabb, obj: None });
        let i = add(
            &mut objs,
            Target {
                name: "bush",
                aabb,
                melted_aabb: None,
                bodies,
                flames,
                foam,
                wall: None,
                puddle: None,
                mats: bush_mats.clone(),
                last: None,
            },
            Obj::new(Kind::Bush, [x, z], [r, r]),
        );
        if lit {
            burning_at_start.push(i);
        }
    }

    // The greenhouse's wooden knee wall: the thing to protect.
    {
        let mut bodies = Vec::new();
        for (x0, x1) in [(gx0, -1.5f32), (1.5f32, gx1)] {
            let c = Vec3::new((x0 + x1) / 2.0, 0.45, gz1);
            let s = Vec3::new(x1 - x0, 0.9, 0.3);
            b.colliders.push(Collider {
                aabb: Aabb::new(c, s * 0.5),
                obj: None,
            });
            bodies.push(b.piece(
                &cube,
                &gh_mats.normal,
                Transform::from_translation(c).with_scale(s),
            ));
        }
        let flames = b.flames(&[
            (Vec3::new(-5.0, 1.4, gz1), 1.3),
            (Vec3::new(5.0, 1.4, gz1), 1.3),
            (Vec3::new(-2.5, 1.2, gz1), 1.0),
            (Vec3::new(2.5, 1.2, gz1), 1.0),
        ]);
        add(
            &mut objs,
            Target {
                name: "greenhouse",
                aabb: Aabb::new(Vec3::new(0.0, 0.45, gz1), Vec3::new(8.0, 0.45, 0.15)),
                melted_aabb: None,
                bodies,
                flames,
                foam: None,
                wall: None,
                puddle: None,
                mats: gh_mats,
                last: None,
            },
            Obj::new(Kind::Greenhouse, [0.0, gz1], [8.0, 0.15]),
        );
    }

    // Dry hedge along the west wall.
    {
        let (x, z, hl) = (-21.8f32, 6.0f32, 6.0f32);
        let mut bodies = vec![b.piece(
            &cube,
            &hedge_mats.normal,
            Transform::from_xyz(x, 0.8, z).with_scale(Vec3::new(1.4, 1.6, 2.0 * hl)),
        )];
        for k in 0..5 {
            bodies.push(
                b.piece(
                    &ball,
                    &hedge_mats.normal,
                    Transform::from_xyz(x, 1.6, z - hl + 1.2 + k as f32 * 2.4)
                        .with_scale(Vec3::new(1.5, 0.9, 2.2)),
                ),
            );
        }
        let flames = b.flames(&[
            (Vec3::new(x, 2.2, z - 4.0), 1.0),
            (Vec3::new(x, 2.3, z), 1.1),
            (Vec3::new(x, 2.2, z + 4.0), 1.0),
        ]);
        let foam = Some(b.foam_blob(Vec3::new(x, 1.0, z), Vec3::new(1.8, 1.6, 2.0 * hl + 0.4)));
        let aabb = Aabb::new(Vec3::new(x, 1.0, z), Vec3::new(0.75, 1.0, hl));
        b.colliders.push(Collider { aabb, obj: None });
        add(
            &mut objs,
            Target {
                name: "dry hedge",
                aabb,
                melted_aabb: None,
                bodies,
                flames,
                foam,
                wall: None,
                puddle: None,
                mats: hedge_mats,
                last: None,
            },
            Obj::new(Kind::DryHedge, [x, z], [0.7, hl]),
        );
    }

    // Ice wall across the alcove mouth; it melts into a puddle in front.
    {
        let (x, z, hz, h) = (17.6f32, 6.0f32, 3.0f32, 3.2f32);
        let wall = b.piece(
            &cube,
            &kit.ice,
            Transform::from_xyz(x, h / 2.0, z).with_scale(Vec3::new(0.9, h, 2.0 * hz)),
        );
        let puddle = b
            .commands
            .spawn((
                Mesh3d(cyl.clone()),
                MeshMaterial3d(kit.water.clone()),
                Transform::from_xyz(x - 0.6, 0.03, z).with_scale(Vec3::new(4.2, 0.04, 6.4)),
                Visibility::Hidden,
                NotShadowCaster,
            ))
            .id();
        let aabb = Aabb::new(Vec3::new(x, h / 2.0, z), Vec3::new(0.45, h / 2.0, hz));
        b.colliders.push(Collider {
            aabb,
            obj: Some(objs.len()),
        });
        add(
            &mut objs,
            Target {
                name: "ice wall",
                aabb,
                melted_aabb: Some(Aabb::new(
                    Vec3::new(x - 0.6, 0.15, z),
                    Vec3::new(2.1, 0.25, 3.2),
                )),
                bodies: vec![],
                flames: vec![],
                foam: None,
                wall: Some(wall),
                puddle: Some(puddle),
                mats: plain(kit.ice.clone()),
                last: None,
            },
            Obj::new(Kind::IceWall, [x, z], [0.45, hz]),
        );
    }

    // A loose puddle near the start.
    {
        let (x, z, r) = (7.0f32, 11.0f32, 1.8f32);
        let puddle = b
            .commands
            .spawn((
                Mesh3d(cyl.clone()),
                MeshMaterial3d(kit.water.clone()),
                Transform::from_xyz(x, 0.03, z).with_scale(Vec3::new(2.0 * r, 0.04, 2.0 * r * 0.8)),
                NotShadowCaster,
            ))
            .id();
        add(
            &mut objs,
            Target {
                name: "puddle",
                aabb: Aabb::new(Vec3::new(x, 0.15, z), Vec3::new(r, 0.25, r * 0.8)),
                melted_aabb: None,
                bodies: vec![],
                flames: vec![],
                foam: None,
                wall: None,
                puddle: Some(puddle),
                mats: plain(kit.water.clone()),
                last: None,
            },
            Obj::new(Kind::Puddle, [x, z], [r, r * 0.8]),
        );
    }

    // Steam vent: a metal ring around a glowing hot core.
    {
        let (x, z) = (-15.0f32, 14.0f32);
        let ring = b.piece(
            &cyl,
            &metal,
            Transform::from_xyz(x, 0.15, z).with_scale(Vec3::new(2.0, 0.3, 2.0)),
        );
        let core = b.piece(
            &cyl,
            &vent_hot,
            Transform::from_xyz(x, 0.2, z).with_scale(Vec3::new(1.2, 0.32, 1.2)),
        );
        let aabb = Aabb::new(Vec3::new(x, 0.5, z), Vec3::new(1.0, 0.5, 1.0));
        b.colliders.push(Collider {
            aabb: Aabb::new(Vec3::new(x, 0.15, z), Vec3::new(1.0, 0.15, 1.0)),
            obj: None,
        });
        let _ = ring;
        add(
            &mut objs,
            Target {
                name: "steam vent",
                aabb,
                melted_aabb: None,
                bodies: vec![core],
                flames: vec![],
                foam: None,
                wall: None,
                puddle: None,
                mats: StateMats {
                    frosty: toon(materials, Color::srgb(0.5, 0.6, 0.7)),
                    ..plain(vent_hot.clone())
                },
                last: None,
            },
            Obj::new(Kind::SteamVent, [x, z], [1.0, 1.0]),
        );
    }

    // Lab bench in the alcove: the water-splitting station placeholder.
    {
        let (x, z) = (21.8f32, 6.0f32);
        b.block(Vec3::new(x, 0.95, z), Vec3::new(1.4, 0.12, 3.6), &wood);
        for (dx, dz) in [(-0.6, -1.6), (0.6, -1.6), (-0.6, 1.6), (0.6, 1.6)] {
            b.piece(
                &cube,
                &wood,
                Transform::from_xyz(x + dx, 0.45, z + dz).with_scale(Vec3::new(0.12, 0.9, 0.12)),
            );
        }
        // Electrolysis (Hoffman voltameter): 2H2O -> 2H2 + O2, so the H2 gas
        // column (cathode side) is exactly twice the O2 column (anode side).
        // Geometry and the 2:1 test live in chem::electrolysis.
        b.piece(
            &cube,
            &metal,
            Transform::from_xyz(x, 1.15, z).with_scale(Vec3::new(0.6, 0.3, 0.5)),
        );
        for t in chem::electrolysis::tubes() {
            b.piece(
                &cyl,
                &glass,
                Transform::from_xyz(x, t.glass.y, z + t.dz)
                    .with_scale(Vec3::new(t.glass.d, t.glass.h, t.glass.d)),
            );
            b.piece(
                &cyl,
                &liquid[1],
                Transform::from_xyz(x, t.water.y, z + t.dz)
                    .with_scale(Vec3::new(t.water.d, t.water.h, t.water.d)),
            );
            b.piece(
                &cyl,
                &gas,
                Transform::from_xyz(x, t.gas.y, z + t.dz)
                    .with_scale(Vec3::new(t.gas.d, t.gas.h, t.gas.d)),
            );
        }
        b.piece(
            &ball,
            &liquid[0],
            Transform::from_xyz(x - 0.2, 1.2, z + 0.4).with_scale(Vec3::splat(0.3)),
        );
        add(
            &mut objs,
            Target {
                name: "lab bench",
                aabb: Aabb::new(Vec3::new(x, 1.2, z), Vec3::new(0.75, 1.0, 1.8)),
                melted_aabb: None,
                bodies: vec![],
                flames: vec![],
                foam: None,
                wall: None,
                puddle: None,
                mats: plain(wood.clone()),
                last: None,
            },
            Obj::new(Kind::LabBench, [x, z], [0.7, 1.8]),
        );
    }

    let mut world = chem::World::new(objs);
    for i in burning_at_start {
        world.ignite(i);
    }
    let colliders = Colliders(std::mem::take(&mut b.colliders));
    if let Some(mut share) = b.low.take() {
        let candidates = share.candidates.len();
        share.apply(b.commands, meshes, &targets);
        info!(
            "LOW_SHARED_MAT colours={} pieces={} meshes={} candidates={} target_skipped={}",
            share.palette.len(),
            share.pieces,
            share.cache.len(),
            candidates,
            candidates - share.pieces
        );
    }
    (
        kit,
        colliders,
        Chem {
            initial: world.clone(),
            world,
            targets,
        },
    )
}

/// Nearest thing the ray hits: distance, and the chemistry object if any.
pub fn raycast(
    chem: &Chem,
    colliders: &Colliders,
    origin: Vec3,
    dir: Vec3,
    max_t: f32,
) -> (f32, Option<usize>) {
    let mut best = (max_t, None);
    if dir.y < -1e-4 {
        let t = -origin.y / dir.y;
        if t < best.0 {
            best = (t, None);
        }
    }
    for a in colliders.solid(&chem.world) {
        if let Some(t) = a.ray(origin, dir, best.0) {
            if t < best.0 {
                best = (t, None);
            }
        }
    }
    for i in 0..chem.targets.len() {
        if let Some(t) = chem.bounds(i).ray(origin, dir, best.0 + 0.05) {
            // Targets win ties with their own collider.
            if t <= best.0 + 0.05 {
                best = (t.min(best.0), Some(i));
            }
        }
    }
    best
}
