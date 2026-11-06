//! Bottle stashes (I-057, `docs/loop/items/I-057/spec.md`): the places in
//! the valley where the bottle items a discovered spell uses up are found,
//! so every chemistry quest can be done in story mode without the debug
//! `give`. Mining and digging were dropped (Paawan); a stash is picked up
//! by walking up to it.
//!
//! - Each stash (`content/chem/bottles.json` `stashes`) stands on a fixed
//!   feet cell, marked by an existing block (`block`: glass jars, or the
//!   runestone of the hidden mark). No new mesh or material.
//! - Walking within `REACH` of a full stash puts its items in the pack; it
//!   fills again `refill_s` seconds later.
//! - A hidden stash (the hidden mark) has no marker and cannot be picked up
//!   until a Reveal spell lands near it (`world_use`), which shows it.

use bevy::prelude::*;
use realgame_chem::discover::Stash;
use realgame_sim::valley::{Item, Pack};
use realgame_voxel::{mat, materials, Cell, World};

use crate::player::Player;
use crate::voxel_view::VoxWorld;

/// How close (m, across the ground, from the feet) the player picks a
/// stash up.
pub const REACH: f32 = 2.0;

/// Said when a stash's items go into the pack: the items, then the place.
pub const FOUND_LINE: &str = "You found {} {}.";
/// Said when the pack has no room for some of them.
pub const PACK_FULL_LINE: &str = "Your pack is full, so some were left behind.";
/// Said when a Reveal shows the hidden mark.
pub const MARK_LINE: &str = "A hidden mark appears: follow the gold light to it.";

/// The stashes in the data.
pub fn list() -> &'static [Stash] {
    &crate::discover::data().bottles.stashes
}

/// The stashes' state in this run.
#[derive(Resource, Debug, Default)]
pub struct Stashes {
    /// Game seconds (paused time does not count).
    pub now: f32,
    /// Per stash: the game time it is full again (0: full now).
    pub empty_until: Vec<f32>,
    /// Per stash: a hidden stash has been shown by a Reveal.
    pub shown: Vec<bool>,
    /// The marker blocks have been set in the world.
    pub placed: bool,
}

impl Stashes {
    fn fit(&mut self) {
        let n = list().len();
        self.empty_until.resize(n, 0.0);
        self.shown.resize(n, false);
    }

    /// Stash `i` can be seen (not hidden, or shown by a Reveal).
    pub fn visible(&self, i: usize) -> bool {
        list()
            .get(i)
            .is_some_and(|s| !s.hidden || self.shown.get(i).copied().unwrap_or(false))
    }

    /// Stash `i` can be picked up now.
    pub fn ready(&self, i: usize) -> bool {
        self.visible(i) && self.now >= self.empty_until.get(i).copied().unwrap_or(0.0)
    }

    /// The ready stash the player (feet at `feet`) stands at, if any.
    pub fn near(&self, feet: Vec3) -> Option<usize> {
        list().iter().enumerate().position(|(i, s)| {
            let c = crate::discover::centre(s.at);
            let d = Vec2::new(feet.x - c.x, feet.z - c.z).length();
            self.ready(i) && d <= REACH && (feet.y - s.at[1] as f32).abs() <= 2.0
        })
    }

    /// Puts stash `i`'s items in `pack` and empties it until it refills.
    /// Returns the caption.
    pub fn take(&mut self, i: usize, pack: &mut Pack) -> Option<String> {
        self.fit();
        let s = list().get(i)?;
        if !self.ready(i) {
            return None;
        }
        let mut names = Vec::new();
        let mut full = false;
        for (key, n) in &s.items {
            let Some(it) = Item::from_key(key) else {
                continue;
            };
            let left = pack.add(it, *n);
            full |= left > 0;
            if left < *n {
                names.push(format!("{} {}", n - left, it.label()));
            }
        }
        if names.is_empty() && !full {
            return None;
        }
        self.empty_until[i] = self.now + crate::discover::data().bottles.refill_s;
        let list = match names.split_last() {
            Some((last, [])) => last.clone(),
            Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
            None => String::new(),
        };
        let mut line = if list.is_empty() {
            String::new()
        } else {
            FOUND_LINE
                .replacen("{}", &list, 1)
                .replacen("{}", &place_at(s), 1)
        };
        if full {
            line = format!("{line} {PACK_FULL_LINE}").trim().to_string();
        }
        Some(line)
    }

    /// The hidden stashes not yet shown within `r` metres (across the
    /// ground) of `at`, nearest first: (index, distance).
    pub fn hidden_near(&self, at: Vec3, r: f32) -> Vec<(usize, f32)> {
        let mut v: Vec<(usize, f32)> = list()
            .iter()
            .enumerate()
            .filter(|(i, s)| s.hidden && !self.visible(*i))
            .map(|(i, s)| {
                let c = crate::discover::centre(s.at);
                (i, Vec2::new(c.x - at.x, c.z - at.z).length())
            })
            .filter(|x| x.1 <= r)
            .collect();
        v.sort_by(|a, b| a.1.total_cmp(&b.1));
        v
    }

    /// Shows hidden stash `i` (a Reveal landed near it): its marker block
    /// appears and it can be picked up.
    pub fn show(&mut self, i: usize, w: &mut World) {
        self.fit();
        if let Some(s) = list().get(i) {
            self.shown[i] = true;
            place_marker(w, s);
        }
    }
}

/// The stash's place as a phrase after "You found ...": "at the shelf
/// by ...", or the place as written when it starts with a preposition.
fn place_at(s: &Stash) -> String {
    let p = s.place.as_str();
    if ["by ", "beside ", "at ", "in ", "near "]
        .iter()
        .any(|w| p.starts_with(w))
    {
        p.to_string()
    } else {
        format!("at {p}")
    }
}

/// Sets stash `s`'s marker block on its feet cell, when that cell is open
/// air outside protected stamps. True when it set it.
pub fn place_marker(w: &mut World, s: &Stash) -> bool {
    let m = materials::by_name(&s.block).unwrap_or(mat::GLASS);
    if w.can_place(s.at) && w.get(s.at).mat == mat::AIR {
        w.set(s.at, Cell::new(m));
        return true;
    }
    w.get(s.at).mat == m
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Stashes>()
        .add_systems(Update, run.before(crate::publish));
}

fn run(
    time: Res<Time>,
    mut st: ResMut<Stashes>,
    mut game: ResMut<crate::Game>,
    player: Res<Player>,
    mut pack: ResMut<crate::Pack>,
    vox: Option<ResMut<VoxWorld>>,
) {
    let Some(mut vox) = vox else { return };
    st.fit();
    if !st.placed {
        for (i, s) in list().iter().enumerate() {
            if st.visible(i) {
                place_marker(&mut vox.0, s);
            }
        }
        st.placed = true;
    }
    if game.paused {
        return;
    }
    st.now += time.delta_secs().min(0.1);
    if let Some(i) = st.near(player.pos) {
        if let Some(line) = st.take(i, &mut pack.0) {
            game.say(&line);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use realgame_voxel::{gen, DEFAULT_SEED};

    fn idx(id: &str) -> usize {
        list().iter().position(|s| s.id == id).unwrap()
    }

    #[test]
    fn every_stash_stands_on_open_ground_in_the_valley() {
        let w = gen::generate(DEFAULT_SEED);
        assert!(list().len() >= 5);
        for s in list() {
            let solid = |p: [i32; 3]| materials::flags(w.get(p).mat).solid;
            let [x, y, z] = s.at;
            assert!(solid([x, y - 1, z]), "{}: no ground under it", s.id);
            assert!(!solid(s.at) && !solid([x, y + 1, z]), "{}: not open", s.id);
            assert!(w.can_place(s.at), "{}: inside a protected stamp", s.id);
            assert!(
                materials::by_name(&s.block).is_some(),
                "{}: {}",
                s.id,
                s.block
            );
            for (k, _) in &s.items {
                assert!(Item::from_key(k).is_some(), "{}: {k}", s.id);
            }
        }
    }

    #[test]
    fn walking_up_to_a_stash_fills_the_pack_and_it_refills() {
        let mut st = Stashes::default();
        st.fit();
        let i = idx("kitchen");
        let s = &list()[i];
        let feet = crate::discover::centre(s.at) + Vec3::new(1.2, -0.5, 0.0);
        assert_eq!(st.near(feet), Some(i));
        assert_eq!(st.near(feet + Vec3::new(6.0, 0.0, 0.0)), None, "too far");
        let mut pack = Pack::default();
        let line = st.take(i, &mut pack).unwrap();
        assert_eq!(pack.count(Item::from_key("vinegar").unwrap()), 4);
        assert_eq!(pack.count(Item::from_key("baking_soda").unwrap()), 4);
        assert!(
            line.starts_with("You found 4 Vinegar, 4 Baking soda and 4 Salt at the shelf"),
            "{line}"
        );
        // Empty until it refills.
        assert_eq!(st.near(feet), None);
        assert!(st.take(i, &mut pack).is_none());
        st.now += crate::discover::data().bottles.refill_s;
        assert_eq!(st.near(feet), Some(i));
    }

    #[test]
    fn the_hidden_mark_shows_only_after_a_reveal() {
        let mut w = gen::generate(DEFAULT_SEED);
        let mut st = Stashes::default();
        st.fit();
        let i = idx("hidden_mark");
        let s = &list()[i];
        let at = crate::discover::centre(s.at);
        assert!(!st.visible(i));
        assert_eq!(st.near(at), None, "not before it is shown");
        assert_eq!(st.hidden_near(at + Vec3::new(10.0, 0.0, 0.0), 20.0)[0].0, i);
        assert!(st
            .hidden_near(at + Vec3::new(30.0, 0.0, 0.0), 20.0)
            .is_empty());
        st.show(i, &mut w);
        assert!(st.visible(i) && st.hidden_near(at, 20.0).is_empty());
        assert_eq!(w.get(s.at).mat, mat::RUNESTONE, "its marker appears");
        assert_eq!(st.near(at + Vec3::new(1.0, -0.5, 0.0)), Some(i));
    }
}
