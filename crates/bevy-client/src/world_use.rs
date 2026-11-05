//! What a discovered spell does out in the world when it is not one of the
//! four shipped spells' matter rules (I-056, `docs/loop/items/I-056/spec.md`):
//!
//! - **Wall** (a solid forms): a short-lived wall of blocks where it lands,
//!   across the line from the player, that crumbles back to air after a
//!   while. Built like the ice walkway (`powers::freeze_walkway`): cells
//!   set in the block world, never inside a protected stamp.
//! - **Reveal** (a colour appears): the caches not yet found within a radius
//!   of the landing show for a while; the objective beacon (`beacon`)
//!   stands on the nearest of them.
//! - **Lift** (hydrogen): a player near the landing rises, with the H2 lift
//!   power's motion (`powers::Motion::lift`).
//! - **Push** (another gas): a player near the landing is pushed away from
//!   it, with the CO2 dash's launch (`powers::Motion::push`).
//!
//! Every size and time is a data value in `content/chem/effects.json`
//! (`world`), scaled by the spell's strength (`discover::lerp`). Nothing
//! here draws: the blocks are ordinary materials and the reveal uses the
//! existing beacon.

use bevy::prelude::*;
use realgame_chem::discover::{self as d, lerp, LiftSpec, PushSpec, RevealSpec, Runs, WallSpec};
use realgame_voxel::{mat, materials, Cell, World};

use crate::player::Player;
use crate::powers::{Built, Powers, SITES};
use crate::voxel_view::VoxWorld;

/// A wall's lowest block may sit this many cells below where it landed (it
/// drops to stand on the ground, not float).
const WALL_DROP: i32 = 3;

/// What the world uses are doing now.
#[derive(Resource, Debug, Default)]
pub struct Uses {
    /// Game seconds (paused time does not count).
    pub now: f32,
    /// Standing walls: their cells, material and when they crumble.
    pub walls: Vec<Wall>,
    /// Revealed caches (indices into `powers::SITES`), nearest to the
    /// landing first, and when they hide again.
    pub reveal: Vec<usize>,
    pub reveal_until: f32,
    /// The orb the beacon stands on while a reveal lasts.
    pub reveal_at: Option<Vec3>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Wall {
    pub cells: Vec<[i32; 3]>,
    /// What each cell held before (air, or water it plugged), put back when
    /// it crumbles.
    pub was: Vec<Cell>,
    pub mat: u8,
    pub until: f32,
}

impl Uses {
    /// Where the beacon points while a reveal lasts (x, z).
    pub fn revealed(&self) -> Option<(f32, f32)> {
        (self.now < self.reveal_until)
            .then_some(self.reveal_at)
            .flatten()
            .map(|p| (p.x, p.z))
    }
}

/// The wall material named in the data (ash if unknown).
pub fn wall_mat(spec: &WallSpec) -> u8 {
    materials::by_name(&spec.material).unwrap_or(mat::ASH)
}

/// The wall's width and height (blocks) for a spell of `strength`.
pub fn wall_size(spec: &WallSpec, strength: f32) -> (i32, i32) {
    let f = |r: [u8; 2]| lerp([f32::from(r[0]), f32::from(r[1])], strength).round() as i32;
    (f(spec.width).max(1), f(spec.height).max(1))
}

fn open(w: &World, p: [i32; 3]) -> bool {
    let m = w.get(p).mat;
    (w.can_place(p) && matches!(m, mat::AIR | mat::WATER)) || (m == mat::WATER && in_ford(p))
}

/// The ford's water (`stamp::FORD`, P2a) is a protected puzzle stamp, but
/// a wall may stand in its water for a while (I-057: the wall quest),
/// like the Frost path; it gives the water back when it crumbles. Its
/// banks and bed stay protected.
pub fn in_ford(p: [i32; 3]) -> bool {
    realgame_voxel::stamp::FORD.contains(p)
}

/// Builds a wall `width` x `height` of `m` where a spell landed at `pos` on
/// a face with normal `n`, across the line from `from` (the player): the
/// cell in front of the face, dropped to stand on the ground, then
/// `width` cells sideways and `height` up. Only air or water cells outside
/// protected stamps (or water in the ford) are filled. Returns the wall
/// (its `until` is for the caller to set); `Wall::wet` counts the water it
/// plugged (I-057: a wall in the river).
pub fn build_wall(
    w: &mut World,
    pos: Vec3,
    n: IVec3,
    from: Vec3,
    width: i32,
    height: i32,
    m: u8,
) -> Wall {
    let inside = pos - n.as_vec3() * 0.01;
    let hit = inside.floor().as_ivec3();
    let mut base = hit + n;
    // A cast stops on water (`voxel_cast`): landing on the surface, the
    // wall stands in the water itself.
    if !open(w, base.into()) && w.get(hit.into()).mat == mat::WATER && open(w, hit.into()) {
        base = hit;
    }
    let mut wall = Wall {
        cells: Vec::new(),
        was: Vec::new(),
        mat: m,
        until: 0.0,
    };
    if !open(w, base.into()) {
        return wall;
    }
    for _ in 0..WALL_DROP {
        let below = base - IVec3::Y;
        if open(w, below.into()) {
            base = below;
        } else {
            break;
        }
    }
    let fwd = Vec2::new(pos.x - from.x, pos.z - from.z);
    // Sideways: across the line of the throw, snapped to a block axis.
    let side = if fwd.x.abs() >= fwd.y.abs() {
        IVec3::Z
    } else {
        IVec3::X
    };
    let lo = -(width - 1) / 2;
    for k in lo..lo + width {
        for y in 0..height {
            let p: [i32; 3] = (base + side * k + IVec3::Y * y).into();
            if open(w, p) {
                wall.was.push(w.get(p));
                w.set(p, Cell::new(m));
                wall.cells.push(p);
            }
        }
    }
    wall
}

/// The middle of a wall's cells.
pub fn middle(cells: &[[i32; 3]]) -> Vec3 {
    let sum: Vec3 = cells.iter().map(|c| crate::discover::centre(*c)).sum();
    sum / cells.len().max(1) as f32
}

/// What a Reveal landing at `at` shows, nearest first: the unfound caches
/// (indices into `SITES`) and the hidden stashes (I-057: the hidden mark)
/// within its radius, and where the beacon stands (the nearest of all).
pub fn reveal_targets(
    built: &[Built],
    found: &[bool],
    stashes: &crate::stash::Stashes,
    at: Vec3,
    spec: &RevealSpec,
    strength: f32,
) -> (Vec<usize>, Vec<usize>, Option<Vec3>) {
    let caches = hidden_near(built, found, at, spec, strength);
    let marks = stashes.hidden_near(at, lerp(spec.radius_m, strength));
    let flat = |p: Vec3| Vec2::new(p.x - at.x, p.z - at.z).length();
    let cache = caches.first().and_then(|i| {
        built
            .iter()
            .find(|b| b.site.id == SITES[*i].id)
            .map(|b| b.orb)
    });
    let mark = marks
        .first()
        .and_then(|m| crate::stash::list().get(m.0))
        .map(|s| crate::discover::centre(s.at));
    let beacon = match (cache, mark) {
        (Some(c), Some(m)) => Some(if flat(m) < flat(c) { m } else { c }),
        (c, m) => c.or(m),
    };
    (caches, marks.into_iter().map(|m| m.0).collect(), beacon)
}

/// Crumbles a wall: its cells that still hold its material go back to what
/// they held, air or the water it plugged (a block the player broke or
/// something else replaced is left alone).
pub fn crumble(w: &mut World, wall: &Wall) -> usize {
    let mut n = 0;
    for (i, &p) in wall.cells.iter().enumerate() {
        if w.get(p).mat == wall.mat {
            w.set(p, wall.was.get(i).copied().unwrap_or(Cell::AIR));
            n += 1;
        }
    }
    n
}

impl Wall {
    /// How many of its cells held water (it plugged them).
    pub fn wet(&self) -> usize {
        self.was.iter().filter(|c| c.mat == mat::WATER).count()
    }
}

/// The caches not yet found within the reveal radius of `at` (flat
/// metres), nearest first: indices into `SITES`.
pub fn hidden_near(
    built: &[Built],
    found: &[bool],
    at: Vec3,
    spec: &RevealSpec,
    strength: f32,
) -> Vec<usize> {
    let r = lerp(spec.radius_m, strength);
    let mut v: Vec<(f32, usize)> = built
        .iter()
        .filter_map(|b| {
            let i = SITES.iter().position(|s| s.id == b.site.id)?;
            let d = Vec2::new(b.orb.x - at.x, b.orb.z - at.z).length();
            (!found.get(i).copied().unwrap_or(false) && d <= r).then_some((d, i))
        })
        .collect();
    v.sort_by(|a, b| a.0.total_cmp(&b.0));
    v.into_iter().map(|x| x.1).collect()
}

/// Whether the player stands within `r` metres of the landing.
fn near(player: &Player, at: Vec3, r: f32) -> bool {
    (player.pos + Vec3::Y * 0.9).distance(at) <= r
}

/// A Lift landing: lifts a grounded player near it. True when it did.
pub fn lift(
    pw: &mut Powers,
    player: &mut Player,
    at: Vec3,
    spec: &LiftSpec,
    strength: f32,
) -> bool {
    near(player, at, spec.radius_m) && pw.motion.lift(player, lerp(spec.blocks, strength))
}

/// A Push landing: pushes a player near it away. True when it did.
pub fn push(
    pw: &mut Powers,
    player: &mut Player,
    at: Vec3,
    spec: &PushSpec,
    strength: f32,
) -> bool {
    if !near(player, at, spec.radius_m) {
        return false;
    }
    pw.motion.push(player, at, lerp(spec.speed, strength));
    true
}

pub const WALL_LINE: &str = "A wall of solid forms where it landed. It will crumble soon.";
pub const WALL_NONE: &str = "No room for a wall there. Aim at open ground.";
pub const REVEAL_LINE: &str = "The colour shows a hidden cache: follow the gold light.";
/// Said when a wall stands in the river's water (I-057).
pub const WALL_WATER_LINE: &str = "A wall of solid forms in the water and holds it back.";
pub const REVEAL_NONE: &str = "The colour shows nothing hidden near here.";
pub const LIFT_LINE: &str = "The hydrogen lifts you up!";
pub const LIFT_MISS: &str = "The hydrogen floats away. Cast it at your feet to ride it up.";
pub const PUSH_LINE: &str = "The burst of gas pushes you!";
pub const PUSH_MISS: &str = "The gas puffs out. Cast it close by to be pushed.";

pub fn plugin(app: &mut App) {
    app.init_resource::<Uses>().add_systems(
        Update,
        run.after(crate::aim_and_cast).before(crate::publish),
    );
}

#[allow(clippy::too_many_arguments)]
fn run(
    time: Res<Time>,
    mut uses: ResMut<Uses>,
    mut landed: MessageReader<crate::SpellLanded>,
    mut game: ResMut<crate::Game>,
    mut player: ResMut<Player>,
    mut pw: ResMut<Powers>,
    mut disc: ResMut<crate::discover::Disc>,
    mut stashes: ResMut<crate::stash::Stashes>,
    vox: Option<ResMut<VoxWorld>>,
) {
    let Some(mut vox) = vox else {
        landed.clear();
        return;
    };
    if !game.paused {
        uses.now += time.delta_secs().min(0.1);
    }
    let now = uses.now;
    let spec = &crate::discover::data().world;
    for l in landed.read() {
        let Some(runs) = crate::discover::runs_of(l.disc) else {
            continue;
        };
        let s = l.amount;
        let line = match runs {
            Runs::Wall if l.normal != [0, 0, 0] => {
                let (wd, ht) = wall_size(&spec.wall, s);
                let m = wall_mat(&spec.wall);
                let n = IVec3::new(l.normal[0].into(), l.normal[1].into(), l.normal[2].into());
                let mut wall = build_wall(&mut vox.0, l.pos, n, player.pos, wd, ht, m);
                if wall.cells.is_empty() {
                    WALL_NONE
                } else {
                    let wet = wall.wet();
                    if wet > 0 {
                        disc.report(d::EVENT_WALLED_WATER, middle(&wall.cells));
                    }
                    wall.until = now + spec.wall.seconds;
                    uses.walls.push(wall);
                    if wet > 0 {
                        WALL_WATER_LINE
                    } else {
                        WALL_LINE
                    }
                }
            }
            Runs::Wall => WALL_NONE,
            Runs::Reveal => {
                let (caches, marks, at) =
                    reveal_targets(&pw.built, &pw.found, &stashes, l.pos, &spec.reveal, s);
                for &i in &marks {
                    stashes.show(i, &mut vox.0);
                }
                if at.is_some() {
                    uses.reveal = caches;
                    uses.reveal_at = at;
                    uses.reveal_until = now + spec.reveal.seconds;
                    disc.report(d::EVENT_REVEALED, l.pos);
                    if marks.is_empty() {
                        REVEAL_LINE
                    } else {
                        crate::stash::MARK_LINE
                    }
                } else {
                    REVEAL_NONE
                }
            }
            Runs::Lift => {
                if lift(&mut pw, &mut player, l.pos, &spec.lift, s) {
                    LIFT_LINE
                } else {
                    LIFT_MISS
                }
            }
            Runs::Push => {
                if push(&mut pw, &mut player, l.pos, &spec.push, s) {
                    PUSH_LINE
                } else {
                    PUSH_MISS
                }
            }
            _ => continue,
        };
        game.say(line);
    }
    // Walls crumble; a reveal ends.
    let (old, keep): (Vec<Wall>, Vec<Wall>) = std::mem::take(&mut uses.walls)
        .into_iter()
        .partition(|w| now >= w.until);
    for w in &old {
        crumble(&mut vox.0, w);
    }
    uses.walls = keep;
    if now >= uses.reveal_until {
        uses.reveal.clear();
        uses.reveal_at = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover::{data, entry, index_of};
    use realgame_voxel::Aabb;

    fn ground(w: &mut World, y: i32) {
        for x in 0..32 {
            for z in 0..32 {
                w.set([x, y, z], Cell::new(mat::STONE));
            }
        }
    }

    #[test]
    fn a_wall_stands_across_the_throw_and_crumbles() {
        let spec = &data().world.wall;
        let m = wall_mat(spec);
        assert_eq!(m, mat::ASH, "the data names a real block");
        let mut w = World::new();
        ground(&mut w, 10);
        // Thrown along -z, landing on the top of the ground at (16, 11, 16).
        let at = Vec3::new(16.5, 11.0, 16.5);
        let (wd, ht) = wall_size(spec, 1.0);
        assert_eq!((wd, ht), (5, 3), "full strength: the data's largest");
        assert_eq!(wall_size(spec, 0.0), (1, 1));
        let built = build_wall(&mut w, at, IVec3::Y, Vec3::new(16.5, 11.0, 26.0), wd, ht, m);
        let cells = built.cells.clone();
        assert_eq!(cells.len(), 15);
        assert_eq!(built.wet(), 0, "on dry ground");
        // Across the throw (x), standing on the ground (y 11 to 13).
        for p in &cells {
            assert_eq!(p[2], 16);
            assert!(
                (14..=18).contains(&p[0]) && (11..=13).contains(&p[1]),
                "{p:?}"
            );
            assert_eq!(w.get(*p).mat, mat::ASH);
            assert!(materials::flags(w.get(*p).mat).solid, "it blocks");
        }
        // Thrown along x, it runs along z.
        let side = build_wall(
            &mut w,
            Vec3::new(8.5, 11.0, 8.5),
            IVec3::Y,
            Vec3::new(0.0, 11.0, 8.5),
            3,
            1,
            m,
        )
        .cells;
        assert!(
            side.iter().all(|p| p[0] == 8) && side.len() == 3,
            "{side:?}"
        );
        // It crumbles back to air, leaving a block the player changed.
        w.set(cells[0], Cell::new(mat::DIRT));
        assert_eq!(crumble(&mut w, &built), 14);
        assert_eq!(w.get(cells[0]).mat, mat::DIRT);
        assert!(cells[1..].iter().all(|p| w.get(*p).mat == mat::AIR));
    }

    #[test]
    fn a_wall_drops_to_the_ground_plugs_water_and_spares_stamps() {
        let mut w = World::new();
        ground(&mut w, 5);
        // Hitting a cliff face (normal +x) high up: it drops to stand.
        for y in 6..12 {
            w.set([10, y, 10], Cell::new(mat::STONE));
        }
        let cells = build_wall(
            &mut w,
            Vec3::new(11.0, 8.5, 10.5),
            IVec3::X,
            Vec3::new(20.0, 8.0, 10.5),
            1,
            1,
            mat::ASH,
        )
        .cells;
        assert_eq!(cells, vec![[11, 6, 10]], "dropped onto the ground");
        // Water is plugged.
        w.set([20, 6, 20], Cell::new(mat::WATER));
        let plug = build_wall(
            &mut w,
            Vec3::new(20.5, 6.0, 20.5),
            IVec3::Y,
            Vec3::new(20.5, 6.0, 30.0),
            1,
            1,
            mat::ASH,
        );
        assert_eq!(plug.cells, vec![[20, 6, 20]]);
        assert_eq!(plug.wet(), 1, "it counts the water it plugged");
        // It gives the water back when it crumbles.
        assert_eq!(crumble(&mut w, &plug), 1);
        assert_eq!(w.get([20, 6, 20]).mat, mat::WATER);
        // A protected stamp is never built into.
        w.protect(Aabb {
            min: [0, 0, 0],
            max: [4, 20, 4],
        });
        let cells = build_wall(
            &mut w,
            Vec3::new(2.5, 6.0, 2.5),
            IVec3::Y,
            Vec3::new(2.5, 6.0, 10.0),
            3,
            2,
            mat::ASH,
        )
        .cells;
        assert!(cells.is_empty(), "{cells:?}");
    }

    #[test]
    fn a_reveal_shows_only_unfound_caches_within_its_radius() {
        let spec = &data().world.reveal;
        let mut w = World::new();
        let built: Vec<Built> = SITES
            .iter()
            .map(|s| crate::powers::stamp(&mut w, *s))
            .collect();
        let vault = SITES.iter().position(|s| s.id == "vault").unwrap();
        let ledge = SITES.iter().position(|s| s.id == "ledge").unwrap();
        let at = built[vault].orb - Vec3::new(3.0, 0.0, 0.0);
        let mut found = [false; crate::powers::N_SITES];
        // Weak: a small radius takes in the vault only.
        let v = hidden_near(&built, &found, at, spec, 0.0);
        assert_eq!(v.first(), Some(&vault), "nearest first");
        // Strong: the plaza tower (about 26 m away) shows too.
        let d = Vec2::new(built[ledge].orb.x - at.x, built[ledge].orb.z - at.z).length();
        assert!(d > spec.radius_m[0] && d < spec.radius_m[1], "{d}");
        let v = hidden_near(&built, &found, at, spec, 1.0);
        assert!(v.contains(&vault) && v.contains(&ledge), "{v:?}");
        assert!(!hidden_near(&built, &found, at, spec, 0.0).contains(&ledge));
        // A found cache is not hidden.
        found[vault] = true;
        assert!(!hidden_near(&built, &found, at, spec, 1.0).contains(&vault));
        // The beacon points at it while the reveal lasts.
        let mut u = Uses {
            reveal: vec![ledge],
            reveal_at: Some(built[ledge].orb),
            reveal_until: spec.seconds,
            ..default()
        };
        assert_eq!(u.revealed(), Some((built[ledge].orb.x, built[ledge].orb.z)));
        u.now = spec.seconds;
        assert_eq!(u.revealed(), None, "it hides again");
    }

    #[test]
    fn hydrogen_lifts_and_a_gas_burst_pushes_a_player_nearby() {
        let spec = &data().world;
        let mut pw = Powers::default();
        let mut p = Player {
            pos: Vec3::new(10.0, 5.0, 10.0),
            grounded: true,
            ..default()
        };
        // Cast at the feet: a lift of the data's blocks.
        assert!(lift(
            &mut pw,
            &mut p,
            Vec3::new(10.5, 5.0, 10.0),
            &spec.lift,
            1.0
        ));
        assert_eq!(pw.motion.lift_to, Some(5.0 + spec.lift.blocks[1]));
        assert!(p.vel.y > 0.0 && !p.grounded);
        // Too far: nothing.
        let mut pw = Powers::default();
        let mut p2 = Player {
            pos: Vec3::new(10.0, 5.0, 10.0),
            grounded: true,
            ..default()
        };
        let far = Vec3::new(10.0 + spec.lift.radius_m + 2.0, 5.0, 10.0);
        assert!(!lift(&mut pw, &mut p2, far, &spec.lift, 1.0));
        assert!(p2.grounded && pw.motion.lift_to.is_none());
        // A burst at +x pushes the player towards -x at the data's speed.
        let mut p3 = Player {
            pos: Vec3::new(10.0, 5.0, 10.0),
            grounded: true,
            ..default()
        };
        assert!(push(
            &mut pw,
            &mut p3,
            Vec3::new(12.0, 5.0, 10.0),
            &spec.push,
            0.0
        ));
        assert!((p3.vel.x + spec.push.speed[0]).abs() < 1e-4, "{:?}", p3.vel);
        assert!(p3.vel.z.abs() < 1e-4 && p3.vel.y > 0.0);
        assert!(!push(&mut pw, &mut p2, far, &spec.push, 1.0));
    }

    #[test]
    fn the_world_uses_come_from_the_codex() {
        // Spells carry their use to the landing: a precipitate walls, a
        // displacement reveals, a metal in acid lifts, a peroxide pushes.
        for (id, runs) in [
            ("silver_chloride", Runs::Wall),
            ("iron_copper_sulfate", Runs::Reveal),
            ("zinc_acid", Runs::Lift),
            ("peroxide_splits", Runs::Push),
            ("carbon_burns", Runs::Fire),
        ] {
            let i = index_of(id).unwrap();
            assert_eq!(entry(i).unwrap().1.runs, runs, "{id}");
            assert_eq!(
                crate::discover::lands_as_matter(u16::try_from(i).ok()),
                runs.matter(),
                "{id}"
            );
        }
    }

    /// I-057: a Reveal also shows the hidden mark, and the beacon stands
    /// on the nearest hidden thing.
    #[test]
    fn a_reveal_shows_the_hidden_mark_too() {
        let spec = &data().world.reveal;
        let mut w = World::new();
        let built: Vec<Built> = SITES
            .iter()
            .map(|s| crate::powers::stamp(&mut w, *s))
            .collect();
        let mut found = [true; crate::powers::N_SITES];
        let st = crate::stash::Stashes::default();
        let i = crate::stash::list().iter().position(|s| s.hidden).unwrap();
        let mark = crate::discover::centre(crate::stash::list()[i].at);
        // Every cache found: only the mark shows, and the beacon is on it.
        let at = mark + Vec3::new(5.0, 0.0, 0.0);
        let (caches, marks, beacon) = reveal_targets(&built, &found, &st, at, spec, 0.5);
        assert!(caches.is_empty());
        assert_eq!(marks, vec![i]);
        assert_eq!(beacon, Some(mark));
        // Too far for the weakest reveal: nothing.
        let far = mark + Vec3::new(spec.radius_m[0] + 3.0, 0.0, 0.0);
        let (_, marks, beacon) = reveal_targets(&built, &found, &st, far, spec, 0.0);
        assert!(marks.is_empty() && beacon.is_none());
        // With the Ember Vault unfound, the nearer of the two gets the beacon.
        let vault = SITES.iter().position(|s| s.id == "vault").unwrap();
        found[vault] = false;
        let near_vault = built[vault].orb + Vec3::new(1.0, 0.0, 0.0);
        let (caches, _, beacon) = reveal_targets(&built, &found, &st, near_vault, spec, 1.0);
        assert_eq!(caches.first(), Some(&vault));
        assert_eq!(beacon, Some(built[vault].orb));
    }

    /// I-057: a wall spell cast at the river at the ford stands in the
    /// water, in the wall quest's place.
    #[test]
    fn a_wall_in_the_river_at_the_ford_plugs_its_water() {
        let mut w = realgame_voxel::gen::generate(realgame_voxel::DEFAULT_SEED);
        let q = data()
            .quests
            .quests
            .iter()
            .find(|q| q.event == d::EVENT_WALLED_WATER)
            .unwrap();
        let spec = &data().world.wall;
        let (wd, ht) = wall_size(spec, 0.63);
        // Landing on the water's surface (or the bed under it) from the
        // village bank.
        for (pos, n) in [
            (Vec3::new(128.5, 31.0, 154.5), IVec3::Y),
            (Vec3::new(126.5, 28.0, 153.5), IVec3::Y),
        ] {
            let mut w2 = w.clone();
            let wall = build_wall(
                &mut w2,
                pos,
                n,
                Vec3::new(128.5, 35.0, 163.0),
                wd,
                ht,
                wall_mat(spec),
            );
            assert!(wall.wet() > 0, "{pos}: {:?}", wall.cells);
            assert!(q.in_place(middle(&wall.cells).into()), "{pos}");
            // Only the ford's water: never its bed or banks.
            assert!(wall.cells.iter().all(|p| in_ford(*p)), "{:?}", wall.cells);
            assert!(wall
                .was
                .iter()
                .all(|c| c.mat == mat::WATER || c.mat == mat::AIR));
            // It gives the river its water back.
            crumble(&mut w2, &wall);
            for (p, c) in wall.cells.iter().zip(&wall.was) {
                assert_eq!(w2.get(*p), *c);
            }
        }
        // On the dry bank it is no river wall.
        let bank = build_wall(
            &mut w,
            Vec3::new(128.5, 35.0, 165.5),
            IVec3::Y,
            Vec3::new(128.5, 35.0, 172.0),
            wd,
            ht,
            wall_mat(spec),
        );
        assert_eq!(bank.wet(), 0);
    }
}
