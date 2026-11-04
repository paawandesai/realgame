//! The Bench molecule renderer prototype (plan section "Bench close-up
//! rendering").
//!
//! Atoms are camera-facing quads whose fragment shader ray-traces a perfect
//! sphere, writes its true depth and lights it with Bevy's own PBR code
//! (shadows received, environment light, clearcoat). One material per
//! element, so all atoms of an element share one batch. Bonds are cylinders
//! split in two halves, each coloured by its atom, as in textbooks.
//!
//! Positions are in ångströms, from the molecule's real geometry, and drawn
//! at `SCALE` metres per ångström.

use bevy::asset::uuid_handle;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::{Shader, ShaderRef};

use crate::Features;

const SHADER: Handle<Shader> = uuid_handle!("8c1d2e3f-4a5b-4c6d-9e7f-0a1b2c3d4e5f");

/// Metres per ångström at the bench (plan: 1 Å is about 8 cm).
pub const SCALE: f32 = 0.08;
/// The quad's half-size over the sphere's radius: big enough to hold the
/// sphere's silhouette from any distance over about 1.7 radii.
const QUAD_OVER_RADIUS: f32 = 1.25;
/// Bond cylinder radius (m).
const BOND_R: f32 = 0.012;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Element {
    H,
    C,
    N,
    O,
}

impl Element {
    /// CPK colour (sRGB).
    fn color(self) -> Color {
        match self {
            Element::H => Color::srgb(0.93, 0.93, 0.93),
            Element::C => Color::srgb(0.22, 0.22, 0.24),
            Element::N => Color::srgb(0.19, 0.31, 0.97),
            Element::O => Color::srgb(0.9, 0.12, 0.1),
        }
    }
    /// Ball-and-stick radius in ångströms: a quarter of the van der Waals
    /// radius (H 1.20, C 1.70, N 1.55, O 1.52 Å).
    fn radius_a(self) -> f32 {
        0.25 * match self {
            Element::H => 1.20,
            Element::C => 1.70,
            Element::N => 1.55,
            Element::O => 1.52,
        }
    }
    const ALL: [Element; 4] = [Element::H, Element::C, Element::N, Element::O];
}

/// A molecule: atoms (element, position in Å) and bonds (atom indices, order).
pub struct Molecule {
    pub atoms: Vec<(Element, Vec3)>,
    pub bonds: Vec<(usize, usize, u8)>,
}

/// H2O: O-H 0.958 Å, H-O-H 104.5°.
pub fn water() -> Molecule {
    let half = 104.5f32.to_radians() / 2.0;
    let d = 0.958;
    Molecule {
        atoms: vec![
            (Element::O, Vec3::ZERO),
            (Element::H, Vec3::new(d * half.sin(), -d * half.cos(), 0.0)),
            (Element::H, Vec3::new(-d * half.sin(), -d * half.cos(), 0.0)),
        ],
        bonds: vec![(0, 1, 1), (0, 2, 1)],
    }
}

/// CH4: C-H 1.087 Å, tetrahedral (109.5°).
pub fn methane() -> Molecule {
    let d = 1.087 / 3f32.sqrt();
    let corners = [
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(1.0, -1.0, -1.0),
        Vec3::new(-1.0, 1.0, -1.0),
        Vec3::new(-1.0, -1.0, 1.0),
    ];
    let mut atoms = vec![(Element::C, Vec3::ZERO)];
    atoms.extend(corners.iter().map(|c| (Element::H, *c * d)));
    Molecule {
        atoms,
        bonds: (1..=4).map(|i| (0, i, 1)).collect(),
    }
}

/// CO2: C=O 1.16 Å, linear.
pub fn co2() -> Molecule {
    Molecule {
        atoms: vec![
            (Element::C, Vec3::ZERO),
            (Element::O, Vec3::new(1.16, 0.0, 0.0)),
            (Element::O, Vec3::new(-1.16, 0.0, 0.0)),
        ],
        bonds: vec![(0, 1, 2), (0, 2, 2)],
    }
}

#[derive(ShaderType, Debug, Clone, Default)]
pub struct AtomParams {
    /// x: sphere radius (m); y: quad half-size over radius.
    pub shape: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
pub struct AtomExt {
    #[uniform(100)]
    pub params: AtomParams,
}

impl MaterialExtension for AtomExt {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(SHADER)
    }
    // The quad's flat depth would be wrong in the prepass and the shadow map;
    // atoms receive shadows but cast none (a blob or a proper prepass shader
    // comes in G1).
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
}

pub type AtomMat = ExtendedMaterial<StandardMaterial, AtomExt>;

/// One atom: its world-space centre, kept facing the camera by `billboard`.
#[derive(Component)]
pub struct Atom {
    pub center: Vec3,
    pub radius: f32,
}

#[derive(Resource)]
struct AtomAssets {
    quad: Handle<Mesh>,
    mats: Vec<(Element, Handle<AtomMat>)>,
    bond_mesh: Handle<Mesh>,
    bond_mats: Vec<(Element, Handle<StandardMaterial>)>,
}

/// Molecules waiting for `AtomAssets` (spawned from `Startup`).
#[derive(Resource, Default)]
struct Pending(Vec<(Molecule, Vec3)>);

pub fn plugin(app: &mut App) {
    app.world_mut()
        .resource_mut::<Assets<Shader>>()
        .insert(
            SHADER.id(),
            Shader::from_wesl(include_str!("atoms.wesl"), "realgame/atoms.wesl"),
        )
        .ok();
    app.add_plugins(MaterialPlugin::<AtomMat>::default())
        .init_resource::<Pending>()
        .add_systems(PreStartup, make_assets)
        .add_systems(PostStartup, spawn_pending)
        .add_systems(PostUpdate, billboard.before(TransformSystems::Propagate));
}

/// Queues a molecule centred at `at` (m).
pub fn spawn_molecule(commands: &mut Commands, m: &Molecule, at: Vec3) {
    let m = Molecule {
        atoms: m.atoms.clone(),
        bonds: m.bonds.clone(),
    };
    commands.queue(move |world: &mut World| {
        world.resource_mut::<Pending>().0.push((m, at));
    });
}

fn make_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut atom_mats: ResMut<Assets<AtomMat>>,
    mut std_mats: ResMut<Assets<StandardMaterial>>,
    features: Res<Features>,
) {
    let clearcoat = features.on("clearcoat");
    let mats = Element::ALL
        .iter()
        .map(|&e| {
            let r = e.radius_a() * SCALE;
            let h = atom_mats.add(ExtendedMaterial {
                base: StandardMaterial {
                    base_color: e.color(),
                    perceptual_roughness: 0.3,
                    reflectance: 0.5,
                    clearcoat: if clearcoat { 1.0 } else { 0.0 },
                    clearcoat_perceptual_roughness: 0.08,
                    ..default()
                },
                extension: AtomExt {
                    params: AtomParams {
                        shape: Vec4::new(r, QUAD_OVER_RADIUS, 0.0, 0.0),
                    },
                },
            });
            (e, h)
        })
        .collect();
    let bond_mats = Element::ALL
        .iter()
        .map(|&e| {
            (
                e,
                std_mats.add(StandardMaterial {
                    base_color: e.color(),
                    perceptual_roughness: 0.35,
                    ..default()
                }),
            )
        })
        .collect();
    commands.insert_resource(AtomAssets {
        // A 2 x 2 quad (half-size 1): the transform's scale sets its size.
        quad: meshes.add(Rectangle::new(2.0, 2.0)),
        mats,
        // Unit height along Y, centred: scaled to each half bond's length.
        bond_mesh: meshes.add(Cylinder::new(BOND_R, 1.0).mesh().resolution(12)),
        bond_mats,
    });
}

fn spawn_pending(mut commands: Commands, mut pending: ResMut<Pending>, assets: Res<AtomAssets>) {
    for (m, at) in pending.0.drain(..) {
        let world = |p: Vec3| at + p * SCALE;
        for &(e, p) in &m.atoms {
            let mat = assets.mats.iter().find(|(x, _)| *x == e).unwrap().1.clone();
            let r = e.radius_a() * SCALE;
            commands.spawn((
                Mesh3d(assets.quad.clone()),
                MeshMaterial3d(mat),
                Transform::from_translation(world(p)),
                Atom {
                    center: world(p),
                    radius: r,
                },
                NotShadowCaster,
            ));
        }
        for &(a, b, order) in &m.bonds {
            let (ea, pa) = m.atoms[a];
            let (eb, pb) = m.atoms[b];
            let (wa, wb) = (world(pa), world(pb));
            let axis = (wb - wa).normalize();
            // Offset direction for double and triple bonds.
            let side = axis.any_orthonormal_vector() * BOND_R * 2.4;
            let offsets: Vec<Vec3> = match order {
                2 => vec![side * 0.5, -side * 0.5],
                3 => vec![side, Vec3::ZERO, -side],
                _ => vec![Vec3::ZERO],
            };
            let mid = (wa + wb) / 2.0;
            for off in offsets {
                for (e, from, to) in [(ea, wa, mid), (eb, mid, wb)] {
                    let mat = assets
                        .bond_mats
                        .iter()
                        .find(|(x, _)| *x == e)
                        .unwrap()
                        .1
                        .clone();
                    let len = (to - from).length();
                    commands.spawn((
                        Mesh3d(assets.bond_mesh.clone()),
                        MeshMaterial3d(mat),
                        Transform::from_translation((from + to) / 2.0 + off)
                            .with_rotation(Quat::from_rotation_arc(Vec3::Y, axis))
                            .with_scale(Vec3::new(
                                if order > 1 { 0.7 } else { 1.0 },
                                len,
                                if order > 1 { 0.7 } else { 1.0 },
                            )),
                        NotShadowReceiver,
                    ));
                }
            }
        }
    }
}

/// Turns each atom's quad to face the camera, with the same axes the shader
/// rebuilds: +Z towards the camera, +X = cross(world up, +Z).
fn billboard(
    cam: Query<&GlobalTransform, With<Camera3d>>,
    mut atoms: Query<(&Atom, &mut Transform)>,
) {
    let Some(cam) = cam.iter().next() else {
        return;
    };
    let eye = cam.translation();
    for (atom, mut t) in &mut atoms {
        let n = (eye - atom.center).normalize_or(Vec3::Z);
        *t = Transform::from_translation(atom.center)
            .looking_to(-n, Vec3::Y)
            .with_scale(Vec3::splat(atom.radius * QUAD_OVER_RADIUS));
    }
}
