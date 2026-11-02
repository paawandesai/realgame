//! The valley's later puzzles and its ending (I-052, late lane; spec sections
//! 4-8): the dark Crystal Cave and the torch, the electrolysis rig (water
//! split 2:1 into H2 and O2), the balloon crate that lifts the Kiln Spark
//! off the ledge (only H2 lifts it), the Kiln door, the Invention Bench
//! (P5) and the ending. Registered only in `Mode::Voxel`.
//!
//! The page (`web/late.js`) sends commands on `window.__gameIn.late`
//! (`use`: the E key; `tie <o2_flask|h2_balloon>`; `invent <item,...>|<text>`;
//! `keep_exploring`) and reads `window.__lateState`, written when it changes.
//! The quest hears progress through the story's public `Quest` (Holding the
//! Kiln Spark, Invented).

use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;
use realgame_matter::{lift_n, Gas};
use realgame_sim::tier::Tier;
use realgame_sim::valley::{invent, Item, Pack, QuestEvent};
use realgame_voxel::{gen, mat, stamp, Cell, World};

use crate::bench::BenchState;
use crate::player::Player;
use crate::story::Story;
use crate::voxel_view::VoxWorld;

// ---------------------------------------------------------------------------
// Places (spec section 4; the cells come from `realgame_voxel::gen`).
// ---------------------------------------------------------------------------

/// The rig's glass cell (the Hoffman voltameter) in the cave's rig room.
pub const RIG: [f32; 3] = [231.5, 11.0, 29.5];
/// The crate's resting place on the shaft floor, and the ledge height it
/// rises to (the top of the shaft, beside the ledge at y 24).
pub const CRATE_FLOOR: [f32; 3] = [240.5, 11.0, 20.5];
pub const CRATE_TOP_Y: f32 = 23.2;
/// The Kiln Spark on the ledge.
pub const SPARK: [f32; 3] = [244.0, 25.3, 20.5];
/// In front of the Kiln door, and the Invention Bench.
pub const KILN_DOOR: [f32; 3] = [128.5, 41.0, 173.5];
pub const INVENTION: [f32; 3] = [132.5, 35.0, 200.5];
/// Where the ending puts the player to watch the relit Kiln (x, z).
pub const WATCH: (f32, f32) = (118.5, 196.5);
/// How close (m, from the feet) the player must be to use something.
pub const REACH: f32 = 3.0;

/// The rig splits water for this long (s), filling the tubes 2:1.
pub const RIG_S: f32 = 4.0;
/// The tubes' final volumes, mL (the cathode's H2 is twice the anode's O2).
pub const RIG_H2_ML: f32 = 40.0;
pub const RIG_O2_ML: f32 = 20.0;
/// The cave balloon (spec 5.7): 2 m³ carrying the crate and the Spark.
pub const BALLOON_M3: f32 = 2.0;
pub const PAYLOAD_KG: f32 = 1.5;
/// The crate's speed up (balloon) and down (the rope reeled in), m/s.
pub const RISE_MS: f32 = 1.5;
pub const LOWER_MS: f32 = 2.5;
/// A sinking balloon is untied after this long (s) and comes back.
pub const SINK_S: f32 = 3.0;

/// Ambient brightness in the open and deep in the cave (spec 5.9: dim,
/// never black), and the sun's share left under rock.
pub const AMBIENT_OPEN: f32 = 700.0;
pub const AMBIENT_CAVE: f32 = 120.0;
pub const SUN_CAVE: f32 = 0.08;

// ---------------------------------------------------------------------------
// The model (pure, tested natively).
// ---------------------------------------------------------------------------

/// The electrolysis rig.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Rig {
    Idle,
    /// Splitting water: seconds so far.
    Filling(f32),
}

/// The balloon crate in the shaft.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Crate {
    /// On the shaft floor, nothing tied.
    Floor,
    /// A balloon of this gas is tied but sinks: seconds so far.
    Sinking(Gas, f32),
    /// Rising on an H2 balloon, at height y.
    Rising(f32),
    /// Coming back down on the rope with the Spark, at height y.
    Lowering(f32),
    /// Back on the floor with the Spark inside: press E to take it.
    Loaded,
    /// The Spark has been taken.
    Empty,
}

/// The Invention Bench's last verdict, for the page.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tried {
    pub n: u32,
    pub ok: bool,
    pub relights: bool,
    pub lit: bool,
    pub reaction: String,
    pub code: String,
    pub reason: String,
}

/// The late lane's state.
#[derive(Resource, Clone, Debug)]
pub struct Late {
    pub rig: Rig,
    pub rig_uses: u32,
    pub krate: Crate,
    pub door_open: bool,
    pub kiln_lit: bool,
    /// The ending card shows (until Keep exploring).
    pub card: bool,
    /// Seconds played when the Kiln was relit.
    pub played_s: f32,
    /// Seconds since the page loaded (game time, not paused).
    pub clock: f32,
    /// What is in reach: rig, crate, door or invention.
    pub near: Option<&'static str>,
    /// The player is under rock, and has light (a torch in the pack or
    /// placed nearby).
    pub dark: bool,
    pub light: bool,
    /// The last message for the player, with a counter so the page shows
    /// it again when it repeats.
    pub say: String,
    pub say_n: u32,
    pub tried: Tried,
    /// The invention panel should open (a use at the bench).
    pub open_bench: u32,
    /// The age band's id, for the page (typed equations at Apprentice+).
    pub band: &'static str,
}

impl Default for Late {
    fn default() -> Self {
        Late {
            rig: Rig::Idle,
            rig_uses: 0,
            krate: Crate::Floor,
            door_open: false,
            kiln_lit: false,
            card: false,
            played_s: 0.0,
            clock: 0.0,
            near: None,
            dark: false,
            light: false,
            say: String::new(),
            say_n: 0,
            tried: Tried::default(),
            open_bench: 0,
            band: "seedling",
        }
    }
}

impl Late {
    fn say(&mut self, s: impl Into<String>) {
        self.say = s.into();
        self.say_n += 1;
    }

    /// The H2 and O2 in the rig's tubes, mL (2:1 while filling).
    pub fn tubes(&self) -> (f32, f32) {
        let f = match self.rig {
            Rig::Idle if self.rig_uses > 0 => 1.0,
            Rig::Idle => 0.0,
            Rig::Filling(t) => (t / RIG_S).clamp(0.0, 1.0),
        };
        (RIG_H2_ML * f, RIG_O2_ML * f)
    }

    /// The crate's height (its base).
    pub fn crate_y(&self) -> f32 {
        match self.krate {
            Crate::Rising(y) | Crate::Lowering(y) => y,
            _ => CRATE_FLOOR[1],
        }
    }

    /// The Spark is on the ledge (not yet picked up by the crate).
    pub fn spark_on_ledge(&self) -> bool {
        matches!(
            self.krate,
            Crate::Floor | Crate::Sinking(..) | Crate::Rising(_)
        )
    }

    /// E pressed while `near` something.
    pub fn use_it(&mut self, pack: &mut Pack) -> Use {
        match self.near {
            Some("rig") => self.use_rig(pack),
            Some("crate") => self.use_crate(pack),
            Some("door") => self.use_door(pack),
            Some("invention") => {
                self.open_bench += 1;
                Use::Nothing
            }
            _ => Use::Nothing,
        }
    }

    fn use_rig(&mut self, pack: &Pack) -> Use {
        if self.dark && !self.light {
            self.say(
                "It's too dark to see the rig. Make a torch at the Formula Bench (wood + coal).",
            );
            return Use::Nothing;
        }
        if matches!(self.rig, Rig::Filling(_)) {
            return Use::Nothing;
        }
        if pack.count(Item::H2Balloon) > 0 && pack.count(Item::O2Flask) > 0 {
            self.say("You already carry a hydrogen balloon and an oxygen flask.");
            return Use::Nothing;
        }
        self.rig = Rig::Filling(0.0);
        self.say("Electricity splits water: 2H₂O → 2H₂ + O₂. Watch the tubes.");
        Use::Rig
    }

    fn use_crate(&mut self, pack: &mut Pack) -> Use {
        match self.krate {
            Crate::Floor => {
                let gas = if pack.count(Item::H2Balloon) > 0 {
                    Some("h2_balloon")
                } else if pack.count(Item::O2Flask) > 0 {
                    Some("o2_flask")
                } else {
                    None
                };
                match gas {
                    Some(_) => {
                        self.say("Choose a gas to fill the balloon.");
                        Use::ChooseGas
                    }
                    None => {
                        self.say("The Kiln Spark is up on the ledge. A balloon could lift this crate to it. Try the rig.");
                        Use::Nothing
                    }
                }
            }
            Crate::Loaded => {
                pack.add(Item::KilnSpark, 1);
                self.krate = Crate::Empty;
                self.say("You have the Kiln Spark! Take it home to the Kiln.");
                Use::TookSpark
            }
            _ => Use::Nothing,
        }
    }

    /// Ties a balloon of the pack's `item` (`O2Flask` or `H2Balloon`) to
    /// the crate. The sign of the lift (spec 5.7) decides what happens.
    pub fn tie(&mut self, pack: &mut Pack, item: Item) -> bool {
        if self.krate != Crate::Floor || self.near != Some("crate") {
            return false;
        }
        let gas = match item {
            Item::H2Balloon => Gas::H2,
            Item::O2Flask => Gas::O2,
            _ => return false,
        };
        if !pack.take(item, 1) {
            return false;
        }
        let lift = lift_n(gas, BALLOON_M3, PAYLOAD_KG);
        if lift > 0.0 {
            self.krate = Crate::Rising(CRATE_FLOOR[1]);
            self.say(format!(
                "Hydrogen is much lighter than air: the balloon pulls up with {lift:.1} N!"
            ));
        } else {
            self.krate = Crate::Sinking(gas, 0.0);
            self.say(format!(
                "Oxygen is heavier than air, so the balloon cannot lift the crate ({lift:.1} N)."
            ));
        }
        true
    }

    fn use_door(&mut self, pack: &Pack) -> Use {
        if self.door_open {
            return Use::Nothing;
        }
        if pack.count(Item::KilnSpark) == 0 {
            self.say("This old runestone door only opens for the Kiln Spark.");
            return Use::Nothing;
        }
        self.door_open = true;
        self.say("The Kiln Spark glows and the door swings open. Now invent a reaction hot enough to relight the Kiln.");
        Use::OpenDoor
    }

    /// The Invention Bench judges `items` (or `text`); a reaction hot
    /// enough, with the Spark in the pack, relights the Kiln and uses up
    /// what it burnt.
    pub fn invent(&mut self, pack: &mut Pack, band: Tier, items: &[Item], text: &str) -> bool {
        // Only what the pack holds goes on the bench.
        let mut on: Vec<Item> = Vec::new();
        for &i in items.iter().take(3) {
            if pack.count(i) > 0 && !on.contains(&i) {
                on.push(i);
            }
        }
        let typed = !text.trim().is_empty() && matches!(band, Tier::Apprentice | Tier::Adept);
        let held: Vec<Item> = if typed {
            pack.slots.iter().flatten().map(|s| s.item).collect()
        } else {
            on.clone()
        };
        let v = invent::judge(&held, band, text);
        let mut lit = false;
        let mut reason = v.reason.clone();
        if v.relights {
            if pack.count(Item::KilnSpark) == 0 {
                reason = format!(
                    "{reason} But the Kiln needs its Spark to light it. Find it in the Crystal Cave."
                );
            } else {
                // Burn one of each reactant.
                let used: Vec<Item> = if typed {
                    reactants_in(&held, v.reaction.as_deref())
                } else {
                    on.clone()
                };
                for i in used {
                    pack.take(i, 1);
                }
                pack.take(Item::KilnSpark, 1);
                lit = true;
            }
        }
        self.tried = Tried {
            n: self.tried.n + 1,
            ok: v.ok,
            relights: v.relights,
            lit,
            reaction: v.reaction.unwrap_or_default(),
            code: format!("{:?}", v.code),
            reason,
        };
        if lit {
            self.kiln_lit = true;
            self.door_open = true;
            self.card = true;
            self.played_s = self.clock;
        }
        lit
    }

    /// One frame: the rig fills, the crate moves.
    pub fn tick(&mut self, dt: f32, pack: &mut Pack) -> Option<Use> {
        self.clock += dt;
        let mut out = None;
        if let Rig::Filling(t) = self.rig {
            let t = t + dt;
            if t >= RIG_S {
                self.rig = Rig::Idle;
                self.rig_uses += 1;
                pack.add(Item::H2Balloon, 1);
                pack.add(Item::O2Flask, 1);
                self.say(
                    "Twice as much hydrogen as oxygen! You got an H₂ balloon and an O₂ flask.",
                );
                out = Some(Use::RigDone);
            } else {
                self.rig = Rig::Filling(t);
            }
        }
        match self.krate {
            Crate::Sinking(gas, t) => {
                let t = t + dt;
                if t >= SINK_S {
                    self.krate = Crate::Floor;
                    pack.add(
                        match gas {
                            Gas::O2 => Item::O2Flask,
                            _ => Item::H2Balloon,
                        },
                        1,
                    );
                    self.say(
                        "You untie the balloon and keep the gas. Which gas is lighter than air?",
                    );
                } else {
                    self.krate = Crate::Sinking(gas, t);
                }
            }
            Crate::Rising(y) => {
                let y = y + RISE_MS * dt;
                if y >= CRATE_TOP_Y {
                    self.krate = Crate::Lowering(CRATE_TOP_Y);
                    self.say("The crate reaches the ledge and the Kiln Spark rolls in! Reeling it down...");
                    out = Some(Use::SparkInCrate);
                } else {
                    self.krate = Crate::Rising(y);
                }
            }
            Crate::Lowering(y) => {
                let y = y - LOWER_MS * dt;
                if y <= CRATE_FLOOR[1] {
                    self.krate = Crate::Loaded;
                    self.say("The crate is down. Press E to take the Kiln Spark.");
                } else {
                    self.krate = Crate::Lowering(y);
                }
            }
            _ => {}
        }
        out
    }

    /// `window.__lateState`.
    pub fn json(&self, pack: &Pack) -> String {
        let (h2, o2) = self.tubes();
        let krate = match self.krate {
            Crate::Floor => "floor",
            Crate::Sinking(..) => "sinking",
            Crate::Rising(_) => "rising",
            Crate::Lowering(_) => "lowering",
            Crate::Loaded => "loaded",
            Crate::Empty => "empty",
        };
        let t = &self.tried;
        let gases: Vec<&str> = [(Item::H2Balloon, "h2_balloon"), (Item::O2Flask, "o2_flask")]
            .iter()
            .filter(|(i, _)| pack.count(*i) > 0)
            .map(|(_, k)| *k)
            .collect();
        let bench: Vec<String> = pack
            .entries()
            .into_iter()
            .filter_map(|(k, n)| {
                let item = Item::from_key(k)?;
                invent::species(item)?;
                Some(format!(
                    "{{\"item\":\"{k}\",\"label\":\"{}\",\"n\":{n}}}",
                    esc(&item.label())
                ))
            })
            .collect();
        format!(
            "{{\"v\":1,\"near\":{},\"dark\":{},\"light\":{},\"rig\":{{\"on\":{},\"uses\":{},\"h2\":{h2:.1},\"o2\":{o2:.1}}},\"crate\":{{\"state\":\"{krate}\",\"y\":{:.2}}},\"sparkOnLedge\":{},\"gases\":[{}],\"door\":{},\"lit\":{},\"card\":{},\"played\":{:.0},\"say\":\"{}\",\"sayN\":{},\"tried\":{{\"n\":{},\"ok\":{},\"relights\":{},\"lit\":{},\"reaction\":\"{}\",\"code\":\"{}\",\"reason\":\"{}\"}},\"bench\":[{}],\"openBench\":{},\"band\":\"{}\"}}",
            self.near.map_or("null".to_string(), |n| format!("\"{n}\"")),
            self.dark,
            self.light,
            matches!(self.rig, Rig::Filling(_)),
            self.rig_uses,
            self.crate_y(),
            self.spark_on_ledge(),
            gases.iter().map(|g| format!("\"{g}\"")).collect::<Vec<_>>().join(","),
            self.door_open,
            self.kiln_lit,
            self.card,
            self.played_s,
            esc(&self.say),
            self.say_n,
            t.n,
            t.ok,
            t.relights,
            t.lit,
            esc(&t.reaction),
            esc(&t.code),
            esc(&t.reason),
            bench.join(","),
            self.open_bench,
            self.band,
        )
    }
}

/// What a use or a tick did (for sounds and the quest).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Use {
    Nothing,
    Rig,
    RigDone,
    ChooseGas,
    SparkInCrate,
    TookSpark,
    OpenDoor,
}

/// The pack items a typed reaction burns.
fn reactants_in(held: &[Item], reaction: Option<&str>) -> Vec<Item> {
    let want: &[&str] = match reaction {
        Some("carbon_combustion") => &["C", "O2"],
        Some("water_synthesis") => &["H2", "O2"],
        _ => &[],
    };
    want.iter()
        .filter_map(|s| {
            held.iter()
                .copied()
                .find(|&i| invent::species(i) == Some(*s))
        })
        .collect()
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// What the player is close enough to use (feet at `feet`).
pub fn near(feet: Vec3) -> Option<&'static str> {
    let close = |p: [f32; 3], dy: f32| {
        let d = Vec2::new(feet.x - p[0], feet.z - p[2]).length();
        d <= REACH && (feet.y - p[1]).abs() <= dy
    };
    if close(RIG, 2.5) {
        Some("rig")
    } else if close(CRATE_FLOOR, 2.5) {
        Some("crate")
    } else if close(KILN_DOOR, 2.5) {
        Some("door")
    } else if close(INVENTION, 2.5) {
        Some("invention")
    } else {
        None
    }
}

/// Rock overhead: a solid block within 30 above the eye.
pub fn covered(world: &World, eye: Vec3) -> bool {
    let (x, z) = (eye.x.floor() as i32, eye.z.floor() as i32);
    let y0 = eye.y.floor() as i32 + 1;
    (y0..y0 + 30).any(|y| {
        let m = world.get([x, y, z]).mat;
        m != mat::AIR && realgame_voxel::materials::flags(m).solid
    })
}

/// A placed torch within `r` blocks of `p` (a small cube scan).
pub fn torch_near(world: &World, p: Vec3, r: i32) -> Option<[i32; 3]> {
    let c = [p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32];
    let mut best: Option<([i32; 3], i32)> = None;
    for dz in -r..=r {
        for dy in -r / 2..=r / 2 {
            for dx in -r..=r {
                let q = [c[0] + dx, c[1] + dy, c[2] + dz];
                if world.get(q).mat == mat::TORCH {
                    let d = dx * dx + dy * dy + dz * dz;
                    if best.is_none_or(|(_, b)| d < b) {
                        best = Some((q, d));
                    }
                }
            }
        }
    }
    best.map(|(q, _)| q)
}

/// Opens the Kiln's runestone door (its cells become air).
pub fn open_door(world: &mut World) -> usize {
    let s = stamp::KILN;
    let mut n = 0;
    for z in s.min[2]..s.max[2] {
        for y in s.min[1]..s.max[1] {
            for x in s.min[0]..s.max[0] {
                if world.get([x, y, z]).mat == mat::RUNESTONE {
                    world.set([x, y, z], Cell::AIR);
                    n += 1;
                }
            }
        }
    }
    n
}

/// Lights the hearth (the `LIT` state bit).
pub fn light_hearth(world: &mut World) {
    let mut c = world.get(gen::KILN_HEARTH);
    c.set_flag(Cell::LIT, true);
    world.set(gen::KILN_HEARTH, c);
}

/// `?biome=cave` and `?biome=kiln` (spec section 3.3): the quest step and
/// the pack the earlier steps would have given, so a judge who jumps in can
/// play the late puzzles. The cave start has coal and wood for a torch (the
/// coal step's reward) but no torch; the Kiln start has what the cave gave.
pub fn seed(biome: realgame_voxel::Biome) -> Option<(u8, Vec<(Item, u16)>)> {
    use realgame_voxel::Biome;
    let coal = Item::Block(mat::COAL_ORE);
    let wood = Item::Block(mat::WOOD);
    match biome {
        Biome::Cave => Some((6, vec![(coal, 2), (wood, 1)])),
        Biome::Kiln => Some((
            7,
            vec![
                (coal, 2),
                (Item::Torch, 1),
                (Item::O2Flask, 1),
                (Item::H2Balloon, 1),
                (Item::KilnSpark, 1),
            ],
        )),
        _ => None,
    }
}

fn seed_start(
    start: Option<Res<crate::StartBiome>>,
    mut story: ResMut<Story>,
    mut pack: ResMut<crate::Pack>,
) {
    if let Some((step, items)) = start.and_then(|b| seed(b.0)) {
        story.quest = realgame_sim::valley::Quest::at(step);
        for (item, n) in items {
            pack.0.add(item, n);
        }
    }
}

// ---------------------------------------------------------------------------
// The Bevy side.
// ---------------------------------------------------------------------------

#[derive(Component)]
struct CrateBox;
#[derive(Component)]
struct Rope;
#[derive(Component)]
struct Balloon;
#[derive(Component)]
struct SparkOrb;
#[derive(Component)]
struct TubeFill(bool);
#[derive(Component)]
struct HeldTorch;
#[derive(Component)]
struct PlacedTorch;
#[derive(Component)]
struct KilnFire;
#[derive(Component)]
struct Smoke(f32);

/// The sun's full strength, read once.
#[derive(Resource, Default)]
struct Sun(Option<f32>);

/// Registers the late lane (Voxel mode only).
pub fn plugin(app: &mut App) {
    app.init_resource::<Late>()
        .init_resource::<Sun>()
        .add_systems(Startup, (spawn, seed_start))
        .add_systems(
            Update,
            (commands, step, light, draw, publish)
                .chain()
                .after(crate::story::on_event)
                .before(crate::publish),
        );
}

fn spawn(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let wood = materials.add(StandardMaterial {
        base_color: Color::srgb(0.62, 0.42, 0.24),
        perceptual_roughness: 0.9,
        ..default()
    });
    let rope = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.72, 0.5),
        perceptual_roughness: 1.0,
        ..default()
    });
    let balloon = materials.add(StandardMaterial {
        base_color: Color::srgba(0.75, 0.9, 1.0, 0.85),
        emissive: LinearRgba::rgb(0.05, 0.1, 0.15),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let spark = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.75, 0.3),
        emissive: LinearRgba::rgb(6.0, 3.0, 0.8),
        ..default()
    });
    let glass = materials.add(StandardMaterial {
        base_color: Color::srgba(0.85, 0.95, 1.0, 0.3),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.1,
        ..default()
    });
    let h2 = materials.add(StandardMaterial {
        base_color: Color::srgb(0.6, 0.85, 1.0),
        emissive: LinearRgba::rgb(0.3, 0.6, 1.0),
        ..default()
    });
    let o2 = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.65, 0.6),
        emissive: LinearRgba::rgb(1.0, 0.4, 0.35),
        ..default()
    });
    let flame = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.6, 0.2),
        emissive: LinearRgba::rgb(12.0, 5.0, 1.0),
        ..default()
    });
    let smoke = materials.add(StandardMaterial {
        base_color: Color::srgba(0.85, 0.85, 0.88, 0.55),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    let [cx, cy, cz] = CRATE_FLOOR;
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.9, 0.9, 0.9))),
        MeshMaterial3d(wood),
        Transform::from_xyz(cx, cy + 0.45, cz),
        CrateBox,
        NotShadowCaster,
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.03, 1.0))),
        MeshMaterial3d(rope),
        Transform::from_xyz(cx, cy + 1.0, cz),
        Rope,
        NotShadowCaster,
        NotShadowReceiver,
    ));
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.7).mesh().ico(3).expect("ico"))),
        MeshMaterial3d(balloon),
        Transform::from_xyz(cx, cy + 2.2, cz).with_scale(Vec3::new(1.0, 1.2, 1.0)),
        Visibility::Hidden,
        Balloon,
        NotShadowCaster,
        NotShadowReceiver,
    ));
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.22).mesh().ico(2).expect("ico"))),
        MeshMaterial3d(spark.clone()),
        Transform::from_xyz(SPARK[0], SPARK[1], SPARK[2]),
        SparkOrb,
        NotShadowCaster,
        NotShadowReceiver,
        PointLight {
            color: Color::srgb(1.0, 0.7, 0.3),
            intensity: 40_000.0,
            range: 8.0,
            shadow_maps_enabled: false,
            ..default()
        },
    ));
    // The rig's two tubes (H2 over the cathode on the left, O2 on the right),
    // each a glass tube with a coloured column that grows as gas collects.
    for (k, dx) in [(true, -0.25f32), (false, 0.25)] {
        let base = Vec3::new(RIG[0] + dx, RIG[1] + 1.0, RIG[2]);
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(0.12, 1.2))),
            MeshMaterial3d(glass.clone()),
            Transform::from_translation(base + Vec3::Y * 0.6),
            NotShadowCaster,
            NotShadowReceiver,
        ));
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(0.1, 1.0))),
            MeshMaterial3d(if k { h2.clone() } else { o2.clone() }),
            Transform::from_translation(base + Vec3::Y * 1.18)
                .with_scale(Vec3::new(1.0, 0.001, 1.0)),
            TubeFill(k),
            NotShadowCaster,
            NotShadowReceiver,
        ));
    }
    // The torch you carry: a warm light at the hand, off until you have one.
    commands.spawn((
        PointLight {
            color: Color::srgb(1.0, 0.72, 0.4),
            intensity: 0.0,
            range: 14.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::default(),
        HeldTorch,
    ));
    commands.spawn((
        PointLight {
            color: Color::srgb(1.0, 0.72, 0.4),
            intensity: 0.0,
            range: 12.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::default(),
        PlacedTorch,
    ));
    // The Kiln's fire and its smoke, hidden until the ending.
    let [hx, hy, hz] = gen::KILN_HEARTH;
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.6).mesh().ico(2).expect("ico"))),
        MeshMaterial3d(flame),
        Transform::from_xyz(hx as f32 + 0.5, hy as f32 + 1.2, hz as f32 + 0.5),
        Visibility::Hidden,
        KilnFire,
        NotShadowCaster,
        NotShadowReceiver,
        PointLight {
            color: Color::srgb(1.0, 0.6, 0.25),
            intensity: 0.0,
            range: 30.0,
            shadow_maps_enabled: false,
            ..default()
        },
    ));
    let puff = meshes.add(Sphere::new(0.9).mesh().ico(1).expect("ico"));
    for k in 0..8 {
        commands.spawn((
            Mesh3d(puff.clone()),
            MeshMaterial3d(smoke.clone()),
            Transform::from_xyz(128.5, 61.0, 169.5),
            Visibility::Hidden,
            Smoke(k as f32 / 8.0),
            NotShadowCaster,
            NotShadowReceiver,
        ));
    }
}

fn band(bench: Option<&BenchState>) -> Tier {
    bench.and_then(|b| b.band).unwrap_or(Tier::Seedling)
}

/// Drains `window.__gameIn.late`.
#[allow(clippy::too_many_arguments)]
fn commands(
    mut late: ResMut<Late>,
    mut pack: ResMut<crate::Pack>,
    mut story: ResMut<Story>,
    mut world: Option<ResMut<VoxWorld>>,
    mut player: ResMut<Player>,
    bench: Option<Res<BenchState>>,
) {
    for line in drain() {
        let (head, rest) = line.split_once(' ').unwrap_or((line.as_str(), ""));
        match head {
            "use" => match late.use_it(&mut pack.0) {
                Use::TookSpark => {
                    let total = pack.0.count(Item::KilnSpark);
                    story.quest.on(QuestEvent::Holding {
                        item: Item::KilnSpark,
                        total,
                    });
                }
                Use::OpenDoor => {
                    if let Some(w) = world.as_mut() {
                        open_door(&mut w.0);
                    }
                }
                _ => {}
            },
            "tie" => {
                if let Some(item) = Item::from_key(rest.trim()) {
                    late.tie(&mut pack.0, item);
                }
            }
            "invent" => {
                let (items, text) = rest.split_once('|').unwrap_or((rest, ""));
                let items: Vec<Item> = items
                    .split(',')
                    .filter_map(|k| Item::from_key(k.trim()))
                    .collect();
                let b = band(bench.as_deref());
                if late.invent(&mut pack.0, b, &items, text) {
                    story.quest.on(QuestEvent::Invented { relights: true });
                    if let Some(w) = world.as_mut() {
                        open_door(&mut w.0);
                        light_hearth(&mut w.0);
                    }
                    // Step back into the village to watch the Kiln roar
                    // and its chimney smoke.
                    let (x, z) = WATCH;
                    let y = world
                        .as_ref()
                        .map_or(36, |w| gen::ground_below(&w.0, x as i32, z as i32, 60));
                    player.pos = Vec3::new(x, y as f32, z);
                    player.vel = Vec3::ZERO;
                    player.face(Vec3::new(128.5, 52.0, 169.5));
                }
            }
            "keep_exploring" => late.card = false,
            _ => warn!("LATE unknown={line:?}"),
        }
    }
}

/// Reach, darkness and the timed parts.
fn step(
    time: Res<Time>,
    mut late: ResMut<Late>,
    mut pack: ResMut<crate::Pack>,
    player: Res<Player>,
    world: Option<Res<VoxWorld>>,
    bench: Option<Res<BenchState>>,
    mut scan: Local<(f32, bool)>,
) {
    let dt = time.delta_secs().min(0.1);
    late.band = match band(bench.as_deref()) {
        Tier::Sprout => "sprout",
        Tier::Seedling => "seedling",
        Tier::Apprentice => "apprentice",
        Tier::Adept => "adept",
    };
    late.near = near(player.pos);
    if let Some(w) = world.as_ref() {
        late.dark = covered(&w.0, player.eye());
        // A placed torch nearby counts as light too (scanned twice a second).
        scan.0 -= dt;
        if scan.0 <= 0.0 {
            scan.0 = 0.5;
            scan.1 = late.dark && torch_near(&w.0, player.pos, 8).is_some();
        }
    }
    late.light = pack.0.count(Item::Torch) > 0 || scan.1;
    late.tick(dt, &mut pack.0);
}

/// Cave darkness, the held torch and the nearest placed torch.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn light(
    time: Res<Time>,
    late: Res<Late>,
    player: Res<Player>,
    world: Option<Res<VoxWorld>>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut sun: ResMut<Sun>,
    mut suns: Query<&mut DirectionalLight>,
    mut held: Query<(&mut PointLight, &mut Transform), (With<HeldTorch>, Without<PlacedTorch>)>,
    mut placed: Query<(&mut PointLight, &mut Transform), (With<PlacedTorch>, Without<HeldTorch>)>,
    mut cover: Local<f32>,
    mut scan: Local<f32>,
) {
    let dt = time.delta_secs().min(0.1);
    // Ease in and out of the cave over about a second.
    let target = if late.dark { 1.0 } else { 0.0 };
    *cover += (target - *cover) * (dt * 2.5).min(1.0);
    ambient.brightness = AMBIENT_OPEN + (AMBIENT_CAVE - AMBIENT_OPEN) * *cover;
    for mut s in &mut suns {
        let full = *sun.0.get_or_insert(s.illuminance);
        s.illuminance = full * (1.0 - (1.0 - SUN_CAVE) * *cover);
    }
    let has = late.light && late.dark;
    for (mut l, mut t) in &mut held {
        l.intensity = if has { 120_000.0 } else { 0.0 };
        t.translation = player.eye() + Vec3::Y * 0.3;
    }
    *scan -= dt;
    if *scan <= 0.0 {
        *scan = 0.5;
        let at = world
            .as_ref()
            .filter(|_| late.dark)
            .and_then(|w| torch_near(&w.0, player.pos, 12));
        for (mut l, mut t) in &mut placed {
            match at {
                Some(p) => {
                    l.intensity = 90_000.0;
                    t.translation =
                        Vec3::new(p[0] as f32 + 0.5, p[1] as f32 + 0.8, p[2] as f32 + 0.5);
                }
                None => l.intensity = 0.0,
            }
        }
    }
}

/// Moves the crate, rope, balloon, Spark, tube columns, Kiln fire and smoke.
#[allow(clippy::type_complexity)]
fn draw(
    time: Res<Time>,
    late: Res<Late>,
    mut q: ParamSet<(
        Query<&mut Transform, With<CrateBox>>,
        Query<&mut Transform, With<Rope>>,
        Query<(&mut Transform, &mut Visibility), With<Balloon>>,
        Query<(&mut Transform, &mut Visibility), With<SparkOrb>>,
        Query<(&mut Transform, &TubeFill)>,
        Query<(&mut Visibility, &mut PointLight), With<KilnFire>>,
        Query<(&mut Transform, &mut Visibility, &mut Smoke)>,
    )>,
) {
    let y = late.crate_y();
    let [cx, _, cz] = CRATE_FLOOR;
    for mut t in &mut q.p0() {
        t.translation = Vec3::new(cx, y + 0.45, cz);
    }
    // The rope runs from the crate up to the pulley at the shaft's top.
    let top = CRATE_TOP_Y + 1.6;
    for mut t in &mut q.p1() {
        let len = (top - (y + 0.9)).max(0.05);
        t.translation = Vec3::new(cx, y + 0.9 + len / 2.0, cz);
        t.scale = Vec3::new(1.0, len, 1.0);
    }
    let tied = matches!(late.krate, Crate::Sinking(..) | Crate::Rising(_));
    let sink = match late.krate {
        Crate::Sinking(_, s) => (s * 2.0).min(1.0),
        _ => 0.0,
    };
    for (mut t, mut v) in &mut q.p2() {
        *v = if tied {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        // A sinking balloon sags down beside the crate.
        t.translation = Vec3::new(cx + 0.6 * sink, y + 2.2 - 1.6 * sink, cz);
    }
    let spark_at = match late.krate {
        Crate::Lowering(_) | Crate::Loaded => Some(Vec3::new(cx, y + 1.05, cz)),
        Crate::Empty => None,
        _ => Some(Vec3::from_array(SPARK)),
    };
    let bob = (time.elapsed_secs() * 2.0).sin() * 0.06;
    for (mut t, mut v) in &mut q.p3() {
        match spark_at {
            Some(p) if !late.kiln_lit => {
                *v = Visibility::Visible;
                t.translation = p + Vec3::Y * bob;
            }
            _ => *v = Visibility::Hidden,
        }
    }
    let (h2, o2) = late.tubes();
    for (mut t, fill) in &mut q.p4() {
        let f = if fill.0 {
            h2 / RIG_H2_ML
        } else {
            o2 / RIG_H2_ML
        };
        // Gas collects at the top of each tube and pushes the water down.
        let h = f.max(0.001);
        t.scale = Vec3::new(1.0, h, 1.0);
        t.translation.y = RIG[1] + 1.0 + 1.18 - h / 2.0;
    }
    for (mut v, mut l) in &mut q.p5() {
        *v = if late.kiln_lit {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        let flicker = 1.0 + 0.15 * (time.elapsed_secs() * 13.0).sin();
        l.intensity = if late.kiln_lit {
            900_000.0 * flicker
        } else {
            0.0
        };
    }
    let dt = time.delta_secs().min(0.1);
    for (mut t, mut v, mut s) in &mut q.p6() {
        if !late.kiln_lit {
            *v = Visibility::Hidden;
            continue;
        }
        *v = Visibility::Visible;
        s.0 = (s.0 + dt * 0.18) % 1.0;
        let k = s.0;
        t.translation = Vec3::new(128.5 + k * 3.0, 61.0 + k * 14.0, 169.5 - k * 1.5);
        t.scale = Vec3::splat(0.6 + k * 2.4);
    }
}

fn publish(late: Res<Late>, pack: Res<crate::Pack>, mut last: Local<String>) {
    let json = late.json(&pack.0);
    if *last != json {
        write_state(&json);
        *last = json;
    }
}

#[cfg(target_arch = "wasm32")]
fn drain() -> Vec<String> {
    use wasm_bindgen::JsValue;
    use web_sys::js_sys::{Array, Reflect};
    let Some(win) = web_sys::window() else {
        return Vec::new();
    };
    let get =
        |o: &JsValue, k: &str| Reflect::get(o, &JsValue::from_str(k)).unwrap_or(JsValue::UNDEFINED);
    let gi = get(&win.into(), "__gameIn");
    let q = get(&gi, "late");
    if !Array::is_array(&q) {
        return Vec::new();
    }
    let arr = Array::from(&q);
    if arr.length() == 0 {
        return Vec::new();
    }
    let out = arr.iter().filter_map(|v| v.as_string()).collect();
    let _ = Reflect::set(&gi, &"late".into(), &Array::new());
    out
}

#[cfg(not(target_arch = "wasm32"))]
fn drain() -> Vec<String> {
    Vec::new()
}

#[cfg(target_arch = "wasm32")]
fn write_state(json: &str) {
    use wasm_bindgen::JsValue;
    if let Some(win) = web_sys::window() {
        let _ =
            web_sys::js_sys::Reflect::set(&win, &"__lateState".into(), &JsValue::from_str(json));
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn write_state(_json: &str) {}

#[cfg(test)]
mod tests {
    use super::*;
    use realgame_sim::valley;

    fn run(late: &mut Late, pack: &mut Pack, secs: f32) {
        for _ in 0..(secs / 0.05).ceil() as usize {
            late.tick(0.05, pack);
        }
    }

    fn at(late: &mut Late, what: &'static str) {
        late.near = Some(what);
    }

    #[test]
    fn the_rig_needs_light_in_the_dark() {
        let mut late = Late::default();
        let mut pack = Pack::default();
        at(&mut late, "rig");
        late.dark = true;
        assert_eq!(late.use_it(&mut pack), Use::Nothing);
        assert!(late.say.contains("too dark"), "{}", late.say);
        assert_eq!(late.rig, Rig::Idle);
        late.light = true;
        assert_eq!(late.use_it(&mut pack), Use::Rig);
    }

    #[test]
    fn the_rig_fills_two_to_one_and_gives_both_gases() {
        let mut late = Late::default();
        let mut pack = Pack::default();
        at(&mut late, "rig");
        late.use_it(&mut pack);
        run(&mut late, &mut pack, RIG_S / 2.0);
        let (h2, o2) = late.tubes();
        assert!(h2 > 0.0 && (h2 - 2.0 * o2).abs() < 0.01, "{h2} {o2}");
        assert_eq!(pack.count(Item::H2Balloon), 0, "not before it is done");
        run(&mut late, &mut pack, RIG_S);
        assert_eq!(late.tubes(), (RIG_H2_ML, RIG_O2_ML));
        assert_eq!(pack.count(Item::H2Balloon), 1);
        assert_eq!(pack.count(Item::O2Flask), 1);
        // Carrying both, the rig does not run again.
        assert_eq!(late.use_it(&mut pack), Use::Nothing);
    }

    #[test]
    fn o2_does_not_lift_the_crate_and_h2_brings_the_spark_down() {
        let mut late = Late::default();
        let mut pack = Pack::default();
        pack.add(Item::H2Balloon, 1);
        pack.add(Item::O2Flask, 1);
        // Not in reach: nothing happens.
        assert!(!late.tie(&mut pack, Item::O2Flask));
        at(&mut late, "crate");
        assert_eq!(late.use_it(&mut pack), Use::ChooseGas);
        assert!(late.tie(&mut pack, Item::O2Flask));
        assert_eq!(pack.count(Item::O2Flask), 0);
        for _ in 0..50 {
            late.tick(0.05, &mut pack);
            assert_eq!(late.crate_y(), CRATE_FLOOR[1], "O2 never lifts it");
        }
        run(&mut late, &mut pack, SINK_S);
        assert_eq!(late.krate, Crate::Floor);
        assert_eq!(pack.count(Item::O2Flask), 1, "the flask comes back");
        assert!(late.spark_on_ledge());
        assert!(late.tie(&mut pack, Item::H2Balloon));
        run(&mut late, &mut pack, 2.0);
        assert!(late.crate_y() > CRATE_FLOOR[1] + 2.0, "H2 lifts it");
        run(&mut late, &mut pack, 20.0);
        assert_eq!(late.krate, Crate::Loaded);
        assert!(!late.spark_on_ledge());
        assert_eq!(late.use_it(&mut pack), Use::TookSpark);
        assert_eq!(pack.count(Item::KilnSpark), 1);
        assert_eq!(late.krate, Crate::Empty);
    }

    #[test]
    fn the_door_and_the_invention_bench_need_the_spark() {
        let mut late = Late::default();
        let mut pack = Pack::default();
        let coal = Item::Block(valley::mat::COAL_ORE);
        pack.add(coal, 2);
        at(&mut late, "door");
        assert_eq!(late.use_it(&mut pack), Use::Nothing);
        assert!(!late.door_open);
        // Coal alone fizzles, with the kind reason.
        assert!(!late.invent(&mut pack, Tier::Seedling, &[coal], ""));
        assert!(late.tried.reason.contains("oxygen"), "{:?}", late.tried);
        // Coal + O2 is hot enough, but the Spark is missing.
        pack.add(Item::O2Flask, 1);
        assert!(!late.invent(&mut pack, Tier::Seedling, &[coal, Item::O2Flask], ""));
        assert!(late.tried.relights && !late.tried.lit);
        assert!(late.tried.reason.contains("Spark"), "{:?}", late.tried);
        assert_eq!(pack.count(Item::O2Flask), 1, "nothing used up");
        // An item you do not have is not on the bench.
        pack.add(Item::KilnSpark, 1);
        assert!(!late.invent(
            &mut pack,
            Tier::Seedling,
            &[Item::H2Balloon, Item::O2Flask],
            ""
        ));
        assert_eq!(late.use_it(&mut pack), Use::OpenDoor);
        assert!(late.door_open);
        late.clock = 600.0;
        assert!(late.invent(&mut pack, Tier::Seedling, &[coal, Item::O2Flask], ""));
        assert!(late.kiln_lit && late.card);
        assert_eq!(late.played_s, 600.0);
        assert_eq!(pack.count(Item::O2Flask), 0, "the oxygen burnt");
        assert_eq!(pack.count(coal), 1, "one coal burnt");
        let j = late.json(&pack);
        assert!(
            j.contains("\"lit\":true") && j.contains("\"card\":true"),
            "{j}"
        );
    }

    #[test]
    fn apprentice_types_the_reaction() {
        let mut late = Late::default();
        let mut pack = Pack::default();
        pack.add(Item::H2Balloon, 1);
        pack.add(Item::O2Flask, 1);
        pack.add(Item::KilnSpark, 1);
        assert!(!late.invent(&mut pack, Tier::Apprentice, &[], "H2 + O2 -> H2O"));
        assert_eq!(late.tried.code, "Unbalanced");
        assert!(late.invent(&mut pack, Tier::Apprentice, &[], "2H2 + O2 -> 2H2O"));
        assert_eq!(late.tried.reaction, "water_synthesis");
        assert_eq!(pack.count(Item::H2Balloon), 0);
        assert_eq!(pack.count(Item::O2Flask), 0);
    }

    #[test]
    fn the_cave_is_covered_and_the_door_opens() {
        let mut w = gen::generate(7);
        assert!(
            covered(&w, Vec3::new(RIG[0], RIG[1] + 1.6, RIG[2] - 2.0)),
            "the rig room is under rock"
        );
        assert!(
            !covered(&w, Vec3::new(128.5, 36.6, 205.5)),
            "the village spawn is open sky"
        );
        assert!(open_door(&mut w) > 0, "the Kiln has a runestone door");
        assert_eq!(open_door(&mut w), 0);
        light_hearth(&mut w);
        assert!(w.get(gen::KILN_HEARTH).has(Cell::LIT));
        // The crate and the ledge are where the stamps put them.
        let air = |p: [i32; 3]| w.get(p).mat == mat::AIR;
        assert!(
            air([240, 11, 20]) && air([240, 23, 20]),
            "the shaft is open to the top"
        );
        assert!(!air([240, 10, 20]), "the shaft has a floor");
        assert!(
            !air([244, 24, 20]) && air([244, 25, 20]),
            "the ledge holds the Spark"
        );
        assert!(air([242, 23, 20]), "the shaft opens onto the ledge");
    }

    /// A walker 2 blocks tall who can step up 1 block and drop any height
    /// can get from the cave mouth to the rig and into the balloon shaft.
    #[test]
    fn the_cave_route_is_walkable() {
        use std::collections::{HashSet, VecDeque};
        let w = gen::generate(7);
        let solid = |p: [i32; 3]| realgame_voxel::materials::flags(w.get(p).mat).solid;
        let stand = |p: [i32; 3]| {
            !solid(p) && !solid([p[0], p[1] + 1, p[2]]) && solid([p[0], p[1] - 1, p[2]])
        };
        // Drop to the floor below a free cell.
        let fall = |mut p: [i32; 3]| {
            while p[1] > 1 && !solid([p[0], p[1] - 1, p[2]]) {
                p[1] -= 1;
            }
            p
        };
        let m = gen::CAVE_MOUTH;
        let start = fall([m[0], m[1] + 1, m[2]]);
        assert!(stand(start), "the cave mouth has a floor: {start:?}");
        let goal = |p: [i32; 3]| p;
        let rig = goal([231, 11, 27]);
        let shaft = goal([240, 11, 20]);
        let mut seen: HashSet<[i32; 3]> = HashSet::new();
        let mut q = VecDeque::from([start]);
        seen.insert(start);
        while let Some(p) = q.pop_front() {
            for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                for dy in [1, 0] {
                    let n = [p[0] + dx, p[1] + dy, p[2] + dz];
                    // Stepping up needs head room above the start too.
                    if dy == 1 && solid([p[0], p[1] + 2, p[2]]) {
                        continue;
                    }
                    if !solid(n) && !solid([n[0], n[1] + 1, n[2]]) {
                        let f = fall(n);
                        if stand(f)
                            && (180..250).contains(&f[0])
                            && (5..80).contains(&f[2])
                            && seen.insert(f)
                        {
                            q.push_back(f);
                        }
                        break;
                    }
                }
            }
        }
        assert!(
            seen.contains(&rig),
            "the rig room is reachable from the cave mouth"
        );
        assert!(
            seen.contains(&shaft),
            "the balloon shaft is reachable from the cave mouth"
        );
    }

    /// The Spark is reached by balloon only: the ledge, the shaft and the
    /// rock above and around them cannot be dug or built in.
    #[test]
    fn the_ledge_cannot_be_dug_or_built_around() {
        let w = gen::generate(7);
        for y in 24..60 {
            for p in [[244, y, 20], [240, y, 20], [246, y, 21], [237, y, 19]] {
                assert!(!w.can_break(p) && !w.can_place(p), "{p:?} is protected");
            }
        }
    }

    #[test]
    fn biome_starts_seed_the_quest_and_pack() {
        use realgame_voxel::Biome;
        let (step, items) = seed(Biome::Cave).expect("cave");
        assert_eq!(step, 6);
        assert!(
            !items.iter().any(|(i, _)| *i == Item::Torch),
            "the torch is the cave's first job"
        );
        let mut pack = Pack::default();
        for (i, n) in items {
            pack.add(i, n);
        }
        assert!(realgame_sim::valley::can_craft(&pack, "torch"));
        let (step, items) = seed(Biome::Kiln).expect("kiln");
        assert_eq!(step, 7);
        assert!(items.contains(&(Item::KilnSpark, 1)) && items.contains(&(Item::O2Flask, 1)));
        assert_eq!(seed(Biome::Village), None);
    }
}
