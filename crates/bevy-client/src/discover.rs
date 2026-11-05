//! Discoveries as spells in the block world (I-053): reactions found on the
//! Chemistry Bench, saved as spells, put on a spell slot and cast with the
//! shipped spells' rules (`realgame_chem::discover`). See
//! `docs/loop/items/I-053/spec.md`.
//!
//! - A saved spell sits on a spell key (1-4) the player picks in the pack
//!   (by default the key of the shipped spell whose rule it runs); pressing
//!   that key again switches between the shipped spell and the discovery.
//!   On any key it flies and lands by its own rule (`fly`).
//! - It is built twice (worked casts, judged by the codex) before it casts
//!   instantly; a Sprout casts it without building.
//! - In the block world a cast needs a bottle (pack item) of every reactant
//!   the air does not supply, and uses one up.
//! - Five chemistry quests run here, one per world use
//!   (`content/chem/quests.json`, I-057): melt ice, smother the marsh gas
//!   fire, cool the charcoal mound's hot vent, wall the river at the ford,
//!   find the hidden mark. Each needs a discovered spell whose effect
//!   matches, landing close to the world event that finishes it.
//!
//! The page loads the save through `window.__gameIn.disc` (`found`,
//! `saved`, `slots`, `quests`, and the player's `equip <id> [key]` /
//! `unequip <key>` / `toggle <key>`) and reads `window.__discState`, whose `save`
//! it writes to localStorage (`web/formula-save.js`).

use std::collections::BTreeMap;
use std::sync::OnceLock;

use bevy::prelude::*;
use realgame_chem::discover::{self as d, Bottles, Effect, Effects, Plan, Quests};
use realgame_chem::Entry;
use realgame_sim::spellbook::SpellDef;
use realgame_sim::tier::Tier;
use realgame_sim::valley::Item;

use crate::spells;

/// Says what is missing when a cast has no bottle; `{}` is the item names.
/// `Disc::need_line` adds where to get the first of them.
pub const NEED_LINE: &str = "This spell needs {} from your pack, and you have none.";
/// The melt quest's done line (`content/chem/quests.json`), kept as a name
/// for the tests.
pub const QUEST_DONE: &str =
    "Chemistry quest done! Your reaction gave off heat and the ice melted.";

/// The effect rules, bottles and quests, loaded once.
pub struct Data {
    pub effects: Vec<Effect>,
    by_id: BTreeMap<String, usize>,
    pub bottles: Bottles,
    pub quests: Quests,
    /// The world uses' data values (`effects.json` `world`, I-056).
    pub world: d::WorldSpec,
}

pub fn data() -> &'static Data {
    static D: OnceLock<Data> = OnceLock::new();
    D.get_or_init(|| {
        let codex = &spells::book().codex;
        let fx = Effects::load();
        let world = fx.world().clone();
        let effects = fx.all(codex);
        let by_id = codex
            .entries
            .iter()
            .enumerate()
            .map(|(i, e)| (e.id.clone(), i))
            .collect();
        Data {
            effects,
            by_id,
            bottles: Bottles::load(),
            quests: Quests::load(),
            world,
        }
    })
}

/// A codex reaction by index, with its effect.
pub fn entry(i: usize) -> Option<(&'static Entry, &'static Effect)> {
    Some((spells::book().codex.entries.get(i)?, data().effects.get(i)?))
}

/// What a landed cast does in the world: its discovery's run, or `None`
/// for a shipped spell.
pub fn runs_of(disc: Option<u16>) -> Option<d::Runs> {
    disc.and_then(|i| entry(usize::from(i)))
        .map(|(_, fx)| fx.runs)
}

/// Whether a landed cast is a matter emit (a shipped spell, or a discovery
/// whose run is one of the four shipped rules). Wall, Reveal, Lift and Push
/// are carried out by `world_use` instead, so their flight's spell does not
/// also burn, douse or freeze where they land.
pub fn lands_as_matter(disc: Option<u16>) -> bool {
    runs_of(disc).is_none_or(d::Runs::matter)
}

/// The codex index of reaction `id`.
pub fn index_of(id: &str) -> Option<usize> {
    data().by_id.get(id).copied()
}

/// What a cast carries to where it lands: the emit's scale (1.0 for a
/// shipped spell) and, for a discovered spell, its codex index.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CastTag {
    pub amount: f32,
    pub disc: Option<u16>,
}

impl CastTag {
    pub const PLAIN: CastTag = CastTag {
        amount: 1.0,
        disc: None,
    };
}

/// The discovered spells of this player (loaded from the shared save).
#[derive(Resource, Debug, Default)]
pub struct Disc {
    /// Codex indices found on the Bench (`formula.bench.found`).
    pub found: Vec<usize>,
    /// Saved spells: codex index and correct builds.
    pub saved: Vec<(usize, u8)>,
    /// The saved spell on each spell slot (index 0-3).
    pub slot: [Option<usize>; 4],
    /// The slot casts its discovery (else the shipped spell).
    pub on: [bool; 4],
    pub quests_done: Vec<String>,
    /// Bumped on every change worth saving.
    pub rev: u32,
    /// The open worked cast builds this discovery.
    pub working: Option<usize>,
    /// Discovered spells that match a live quest's needs and landed: the
    /// quest, where and when.
    recent: Vec<(String, Vec3, f32)>,
    /// World events other systems reported this frame (`report`), handled
    /// by the quest system with the game time.
    pending: Vec<(&'static str, Vec3)>,
    /// A caption to say this frame.
    pub say: Option<String>,
    last_json: String,
    /// Commands arrived (the page has loaded the save).
    pub loaded: bool,
}

impl Disc {
    /// The discovery spell slot `i` casts now, if any.
    pub fn active(&self, i: usize) -> Option<usize> {
        self.slot
            .get(i)
            .copied()
            .flatten()
            .filter(|_| self.on.get(i).copied().unwrap_or(false))
    }

    pub fn builds(&self, idx: usize) -> u8 {
        self.saved.iter().find(|s| s.0 == idx).map_or(0, |s| s.1)
    }

    /// Counts one correct build.
    pub fn record_build(&mut self, idx: usize) {
        if let Some(s) = self.saved.iter_mut().find(|s| s.0 == idx) {
            s.1 = s.1.saturating_add(1);
            self.rev += 1;
        }
    }

    /// The worked-cast stage numbering (`worked::route`): 3 once built
    /// `MASTERY_BUILDS` times, else 1 (Learning).
    pub fn stage(&self, idx: usize) -> u8 {
        if self.builds(idx) >= d::MASTERY_BUILDS {
            3
        } else {
            1
        }
    }

    /// Whether `band` builds this discovery (`None` with no band).
    pub fn learnable(idx: usize, band: Option<Tier>) -> Option<bool> {
        let (e, _) = entry(idx)?;
        band.map(|b| b >= d::build_band(e))
    }

    /// The spell-book entry the Formula Bench judge checks a build against.
    pub fn def(idx: usize) -> Option<SpellDef> {
        let (e, fx) = entry(idx)?;
        Some(d::spell_def(e, fx))
    }

    /// The shipped spell (0-3) whose flight and landing a cast of discovery
    /// `idx` uses, whichever key it sits on: its rule's spell.
    pub fn fly(idx: usize) -> Option<usize> {
        entry(idx).map(|(_, fx)| usize::from(fx.runs.slot()) - 1)
    }

    /// The key (0-3) discovery `idx` sits on, if any.
    pub fn key_of(&self, idx: usize) -> Option<usize> {
        self.slot.iter().position(|s| *s == Some(idx))
    }

    /// Puts saved spell `idx` on key `k` (0-3), taking it off any other key
    /// (one key per spell) and replacing what `k` held. It is switched on.
    pub fn put(&mut self, idx: usize, k: usize) -> bool {
        let saveable = entry(idx).is_some_and(|(_, fx)| fx.saveable());
        if k >= 4 || !saveable || !self.saved.iter().any(|s| s.0 == idx) {
            return false;
        }
        for j in 0..4 {
            if j != k && self.slot[j] == Some(idx) {
                self.slot[j] = None;
                self.on[j] = false;
            }
        }
        self.slot[k] = Some(idx);
        self.on[k] = true;
        self.rev += 1;
        true
    }

    /// The cast's tag (the emit scale and the codex index).
    pub fn tag(idx: usize) -> CastTag {
        let amount = entry(idx).map_or(1.0, |(_, fx)| fx.runs.amount(fx.strength));
        CastTag {
            amount,
            disc: u16::try_from(idx).ok(),
        }
    }

    /// The cast caption: the codex's short line for young bands, else its
    /// sentence.
    pub fn caption(idx: usize, band: Option<Tier>) -> String {
        let Some((e, _)) = entry(idx) else {
            return String::new();
        };
        match (band, &e.short) {
            (Some(Tier::Sprout | Tier::Seedling) | None, Some(s)) => format!("{}: {s}", e.name),
            _ => format!("{}: {}", e.name, e.say),
        }
    }

    /// Runs one command line from the page.
    pub fn command(&mut self, line: &str) {
        let mut w = line.split_whitespace();
        let (Some(word), arg) = (w.next(), w.next().unwrap_or("-")) else {
            return;
        };
        let list = |a: &str| -> Vec<String> {
            a.split(',')
                .filter(|s| !s.is_empty() && *s != "-")
                .map(str::to_string)
                .collect()
        };
        match word {
            "found" => {
                self.loaded = true;
                for id in list(arg) {
                    if let Some(i) = index_of(&id) {
                        if !self.found.contains(&i) {
                            self.found.push(i);
                        }
                    }
                }
            }
            "saved" => {
                self.loaded = true;
                for kv in list(arg) {
                    let (id, n) = kv.split_once(':').unwrap_or((kv.as_str(), "0"));
                    let n = n.parse::<u8>().unwrap_or(0);
                    // A Bench-only reaction is never a spell (an older save
                    // may list one): it is dropped.
                    let i = index_of(id).filter(|&i| entry(i).is_some_and(|(_, fx)| fx.saveable()));
                    if let Some(i) = i {
                        match self.saved.iter_mut().find(|s| s.0 == i) {
                            Some(s) => s.1 = s.1.max(n),
                            None => self.saved.push((i, n)),
                        }
                    }
                }
            }
            "slots" => {
                // Any key holds any saved spell, each spell on one key only
                // (the first key that names it).
                for (k, id) in arg.split(',').take(4).enumerate() {
                    let i = index_of(id).filter(|i| self.saved.iter().any(|s| s.0 == *i));
                    if let Some(i) = i {
                        if self.key_of(i).is_none_or(|j| j == k) {
                            self.slot[k] = Some(i);
                        }
                    }
                }
            }
            "quests" => {
                for q in list(arg) {
                    if data().quests.quests.iter().any(|x| x.id == q)
                        && !self.quests_done.contains(&q)
                    {
                        self.quests_done.push(q);
                    }
                }
            }
            "equip" => {
                // `equip <id> [key 1-4]`: no key puts it on its rule's key.
                let Some(i) = index_of(arg).filter(|i| self.saved.iter().any(|s| s.0 == *i)) else {
                    return;
                };
                let Some((e, fx)) = entry(i) else { return };
                let k = match w.next().map(str::parse::<usize>) {
                    None => usize::from(fx.runs.slot()) - 1,
                    Some(Ok(k @ 1..=4)) => k - 1,
                    Some(_) => return,
                };
                if self.put(i, k) {
                    self.say = Some(equip_line(&e.name, k));
                }
            }
            "unequip" => {
                if let Some(k) = arg.parse::<usize>().ok().and_then(|k| k.checked_sub(1)) {
                    if k < 4 && self.slot[k].is_some() {
                        self.slot[k] = None;
                        self.on[k] = false;
                        self.rev += 1;
                    }
                }
            }
            "toggle" => {
                if let Some(k) = arg.parse::<usize>().ok().and_then(|k| k.checked_sub(1)) {
                    self.toggle(k);
                }
            }
            _ => warn!("DISC unknown={line:?}"),
        }
    }

    /// Switches slot `k` between its discovery and the shipped spell.
    pub fn toggle(&mut self, k: usize) -> bool {
        if k < 4 && self.slot[k].is_some() {
            self.on[k] = !self.on[k];
            return true;
        }
        false
    }

    /// What a cast of discovery `idx` needs from `pack` (story mode).
    pub fn plan(idx: usize, pack: &realgame_sim::valley::Pack) -> Option<Plan> {
        let (e, _) = entry(idx)?;
        let held = |key: &str| Item::from_key(key).map_or(0, |it| pack.count(it));
        Some(data().bottles.plan(e, &held))
    }

    /// The caption naming the items that supply `missing` substances, and
    /// where to get the first item of each (`bottles.json` `find`).
    pub fn need_line(missing: &[String]) -> String {
        let mut groups: Vec<String> = Vec::new();
        let mut finds: Vec<String> = Vec::new();
        for s in missing {
            let mut names: Vec<String> = Vec::new();
            for b in data().bottles.items.iter().filter(|b| &b.species == s) {
                if let Some(it) = Item::from_key(&b.item) {
                    let l = it.label();
                    if !names.contains(&l) {
                        names.push(l);
                    }
                }
            }
            match data().bottles.items.iter().find(|b| &b.species == s) {
                Some(b) => {
                    let l = Item::from_key(&b.item).map_or(b.item.clone(), |it| it.label());
                    finds.push(format!("{l}: {}", b.find));
                }
                None => names.push(s.clone()),
            }
            groups.push(names.join(" or "));
        }
        let mut line = NEED_LINE.replace("{}", &groups.join(" and "));
        for f in finds {
            line.push(' ');
            line.push_str(&f);
        }
        line
    }

    /// What discovery `idx` uses from the pack, per reactant the air does
    /// not supply: the substance, its everyday name, the items that supply
    /// it, how many the pack holds and where to get the first item.
    pub fn uses(
        idx: usize,
        pack: &realgame_sim::valley::Pack,
    ) -> Vec<realgame_chem::serde_json::Value> {
        use realgame_chem::serde_json::json;
        let Some((e, _)) = entry(idx) else {
            return Vec::new();
        };
        let all = data().bottles.plan(e, &|_| 0).missing;
        all.iter()
            .map(|s| {
                let items: Vec<_> = data()
                    .bottles
                    .items
                    .iter()
                    .filter(|b| &b.species == s)
                    .collect();
                let labels: Vec<String> = items
                    .iter()
                    .map(|b| Item::from_key(&b.item).map_or(b.item.clone(), |it| it.label()))
                    .collect();
                let have: u32 = items
                    .iter()
                    .map(|b| Item::from_key(&b.item).map_or(0, |it| pack.count(it)))
                    .sum();
                let first = items.first();
                json!({
                    "species": s,
                    "name": substance_name(s),
                    "items": labels,
                    "have": have,
                    "used": first.is_some_and(|b| b.consumed),
                    "findItem": labels.first(),
                    "find": first.map(|b| b.find.clone()),
                })
            })
            .collect()
    }

    /// Takes the plan's consumed bottles from the pack.
    pub fn take(plan: &Plan, pack: &mut realgame_sim::valley::Pack) {
        for key in &plan.take {
            if let Some(it) = Item::from_key(key) {
                pack.take(it, 1);
            }
        }
    }

    /// A discovered spell landed at `pos` at game time `now`: it may be
    /// the cause of a quest's event for a while.
    pub fn landed(&mut self, idx: usize, pos: Vec3, now: f32) {
        let Some((_, fx)) = entry(idx) else { return };
        for q in data().quests.quests.iter().filter(|q| q.live) {
            if !self.quests_done.contains(&q.id) && q.needs.met_by(fx) {
                self.recent.push((q.id.clone(), pos, now));
            }
        }
    }

    /// Another system saw world event `ev` at `at` (handled by `event` in
    /// the quest system, with the game time).
    pub fn report(&mut self, ev: &'static str, at: Vec3) {
        self.pending.push((ev, at));
    }

    /// The world reported event `ev` at `at`, at game time `now`: finishes
    /// each live quest waiting on it whose matching discovered spell landed
    /// within its radius and window, when the event is in the quest's place.
    pub fn event(&mut self, ev: &str, at: Vec3, now: f32) {
        let quests = &data().quests.quests;
        self.recent.retain(|r| {
            quests
                .iter()
                .find(|q| q.id == r.0)
                .is_some_and(|q| now - r.2 <= q.window_s)
        });
        for q in quests.iter().filter(|q| q.live && q.event == ev) {
            if self.quests_done.contains(&q.id) || !q.in_place(at.into()) {
                continue;
            }
            let hit = self
                .recent
                .iter()
                .any(|r| r.0 == q.id && r.1.distance(at) <= q.radius_m && r.2 <= now + 1e-3);
            if hit {
                self.quests_done.push(q.id.clone());
                self.recent.retain(|r| r.0 != q.id);
                self.rev += 1;
                self.say = Some(q.done.clone());
            }
        }
    }

    /// The world melted ice at `cell` at time `now`.
    pub fn melted(&mut self, cell: [i32; 3], now: f32) {
        self.event(d::EVENT_MELTED, centre(cell), now);
    }

    /// `window.__discState`.
    pub fn state_json(&self, band: Option<Tier>, pack: &realgame_sim::valley::Pack) -> String {
        use realgame_chem::serde_json::{self, json};
        let id = |i: usize| entry(i).map_or(String::new(), |(e, _)| e.id.clone());
        let saved: Vec<_> = self
            .saved
            .iter()
            .filter_map(|&(i, n)| {
                let (e, fx) = entry(i)?;
                let plan = Disc::plan(i, pack)?;
                Some(json!({
                    "id": e.id,
                    "name": e.name,
                    "equation": e.equation,
                    "band": e.band.id(),
                    "usable": band.is_none_or(|b| d::usable(e, b)),
                    "builds": n,
                    "need": d::MASTERY_BUILDS,
                    "instant": band.is_some_and(|b| d::instant(e, b, n)),
                    "effect": fx,
                    "slot": fx.runs.slot(),
                    "key": self.key_of(i).map(|k| k + 1),
                    "uses": Disc::uses(i, pack),
                    "missing": plan.missing,
                    "bottles": plan.missing.is_empty(),
                }))
            })
            .collect();
        let bottles: Vec<_> = data()
            .bottles
            .items
            .iter()
            .filter_map(|b| {
                let n = Item::from_key(&b.item).map_or(0, |it| pack.count(it));
                (n > 0).then(|| {
                    let label = Item::from_key(&b.item).map_or(b.item.clone(), |it| it.label());
                    json!({ "item": b.item, "label": label, "species": b.species, "n": n, "find": b.find })
                })
            })
            .collect();
        let sprout = band == Some(Tier::Sprout);
        let quests: Vec<_> = data()
            .quests
            .quests
            .iter()
            .filter(|q| q.live)
            .map(|q| {
                let (line, hint) = if sprout {
                    (&q.line_sprout, &q.hint_sprout)
                } else {
                    (&q.line, &q.hint)
                };
                json!({
                    "id": q.id,
                    "line": line,
                    "hint": hint,
                    "place": q.place,
                    "done": self.quests_done.contains(&q.id),
                })
            })
            .collect();
        let slots: Vec<Option<String>> = self.slot.iter().map(|s| s.map(id)).collect();
        let names: Vec<Option<String>> = self
            .slot
            .iter()
            .map(|s| s.and_then(entry).map(|(e, _)| e.name.clone()))
            .collect();
        let kinds: Vec<Option<&str>> = self
            .slot
            .iter()
            .map(|s| s.and_then(entry).map(|(_, fx)| fx.kind.id()))
            .collect();
        let saved_map: serde_json::Map<String, serde_json::Value> =
            self.saved.iter().map(|&(i, n)| (id(i), json!(n))).collect();
        // Where to get the first bottle (charcoal): the pack says it when
        // the pack holds none.
        let starter = data().bottles.items.first().map(|b| {
            let l = Item::from_key(&b.item).map_or(b.item.clone(), |it| it.label());
            format!("{l}: {}", b.find)
        });
        json!({
            "v": 1,
            "rev": self.rev,
            "starter": starter,
            "loaded": self.loaded,
            "found": self.found.iter().map(|&i| id(i)).collect::<Vec<_>>(),
            "saved": saved,
            "slots": slots,
            "names": names,
            "kinds": kinds,
            "on": self.on,
            "bottles": bottles,
            "quests": quests,
            "save": { "v": 1, "saved": saved_map, "slots": slots, "quests": self.quests_done },
        })
        .to_string()
    }
}

/// The centre of block `cell`.
pub fn centre(cell: [i32; 3]) -> Vec3 {
    Vec3::new(
        cell[0] as f32 + 0.5,
        cell[1] as f32 + 0.5,
        cell[2] as f32 + 0.5,
    )
}

/// Said when a discovery goes on key `k` (0-3).
pub fn equip_line(name: &str, k: usize) -> String {
    let shipped = crate::chem::Spell::ALL.get(k).map_or("", |s| s.name());
    format!(
        "{name} is on key {n}. Press {n} again to switch between {name} and {shipped}.",
        n = k + 1
    )
}

/// A substance's everyday name (`content/chem/names.json`), else its formula.
pub fn substance_name(formula: &str) -> String {
    static NAMES: OnceLock<realgame_chem::serde_json::Value> = OnceLock::new();
    let names = NAMES.get_or_init(|| {
        realgame_chem::serde_json::from_str(realgame_chem::NAMES_JSON).unwrap_or_default()
    });
    names["substances"][formula]
        .as_str()
        .map_or_else(|| formula.to_string(), str::to_string)
}

/// Registered in voxel mode: the page's commands, the quest check and the
/// state.
pub fn plugin(app: &mut App) {
    app.add_systems(Update, commands.before(crate::control))
        .add_systems(
            Update,
            (quest, publish_state)
                .chain()
                .after(crate::blockchem::drive)
                .before(crate::publish),
        );
}

fn commands(mut disc: ResMut<Disc>) {
    for line in drain() {
        disc.command(&line);
    }
}

#[allow(clippy::too_many_arguments)]
fn quest(
    time: Res<Time>,
    mut disc: ResMut<Disc>,
    mut game: ResMut<crate::Game>,
    late: Option<Res<crate::late::Late>>,
    mut landed: MessageReader<crate::SpellLanded>,
    mut matter: MessageReader<crate::MatterEvent>,
) {
    use realgame_matter::{By, MatterEvent as M};
    let now = time.elapsed_secs();
    for l in landed.read() {
        if let Some(i) = l.disc {
            disc.landed(usize::from(i), l.pos, now);
        }
        // Something cold landing in the charcoal mound's hot vent cools it:
        // the shipped Frost or a discovery that runs Frost.
        let cold = match runs_of(l.disc) {
            Some(r) => r == d::Runs::Frost,
            None => l.spell == crate::chem::Spell::Frost,
        };
        if cold
            && late
                .as_ref()
                .is_some_and(|lt| lt.vent_hot() && crate::late::in_vent(l.pos))
        {
            disc.event(d::EVENT_COOLED, l.pos, now);
        }
    }
    for e in matter.read() {
        match e.0 {
            M::Melted { cell } => disc.melted(cell, now),
            M::Smothered { vent } => disc.event(d::EVENT_SMOTHERED, centre(vent), now),
            M::Extinguished {
                cell,
                by: By::NoOxygen,
            } => disc.event(d::EVENT_SMOTHERED, centre(cell), now),
            _ => {}
        }
    }
    for (ev, at) in std::mem::take(&mut disc.pending) {
        disc.event(ev, at, now);
    }
    if let Some(s) = disc.say.take() {
        game.say(&s);
    }
}

fn publish_state(
    mut disc: ResMut<Disc>,
    bench: Res<crate::bench::BenchState>,
    pack: Res<crate::Pack>,
) {
    let json = disc.state_json(bench.band, &pack.0);
    if json != disc.last_json {
        publish(&json);
        disc.last_json = json;
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
    let q = get(&gi, "disc");
    if !Array::is_array(&q) {
        return Vec::new();
    }
    let arr = Array::from(&q);
    if arr.length() == 0 {
        return Vec::new();
    }
    let out = arr.iter().filter_map(|v| v.as_string()).collect();
    let _ = Reflect::set(&gi, &"disc".into(), &Array::new());
    out
}

#[cfg(not(target_arch = "wasm32"))]
fn drain() -> Vec<String> {
    Vec::new()
}

#[cfg(target_arch = "wasm32")]
fn publish(json: &str) {
    use wasm_bindgen::JsValue;
    if let Some(win) = web_sys::window() {
        let _ =
            web_sys::js_sys::Reflect::set(&win, &"__discState".into(), &JsValue::from_str(json));
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn publish(_json: &str) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn loaded() -> Disc {
        let mut d = Disc::default();
        d.command("found carbon_burns,chalk_acid,nonsense");
        d.command("saved carbon_burns:1,chalk_acid:0,nonsense:3");
        d
    }

    #[test]
    fn the_save_loads_and_unknown_ids_are_dropped() {
        let d = loaded();
        assert_eq!(d.found.len(), 2);
        assert_eq!(d.saved.len(), 2);
        let c = index_of("carbon_burns").unwrap();
        assert_eq!(d.builds(c), 1);
        let mut d = d;
        d.command("saved carbon_burns:0");
        assert_eq!(d.builds(c), 1, "a load never lowers builds");
        // A key holds any saved spell (the player picks the key), each
        // spell on one key only; an unsaved or unknown id is dropped.
        d.command("slots chalk_acid,-,haber,carbon_burns");
        assert_eq!(d.slot, [index_of("chalk_acid"), None, None, Some(c)]);
        let mut d2 = loaded();
        d2.command("slots carbon_burns,carbon_burns,-,-");
        assert_eq!(d2.slot, [Some(c), None, None, None], "one key per spell");
    }

    #[test]
    fn the_player_picks_the_key_and_the_spell_keeps_its_own_flight() {
        let mut d = loaded();
        let c = index_of("carbon_burns").unwrap();
        let chalk = index_of("chalk_acid").unwrap();
        // A fire-type reaction on key 3 (Frost's key).
        d.command("equip carbon_burns 3");
        assert_eq!(d.active(2), Some(c));
        assert_eq!(d.active(0), None, "not on its rule's key");
        assert_eq!(Disc::fly(c), Some(0), "it still flies and lands as Fire");
        assert_eq!(
            d.say.as_deref(),
            Some("Charcoal burns is on key 3. Press 3 again to switch between Charcoal burns and Frost.")
        );
        // Moving it to key 2 takes it off key 3.
        d.command("equip carbon_burns 2");
        assert_eq!(d.key_of(c), Some(1));
        assert_eq!(d.slot[2], None);
        assert!(!d.on[2]);
        // Another spell on the same key replaces it.
        d.command("equip chalk_acid 2");
        assert_eq!(d.active(1), Some(chalk));
        assert_eq!(d.key_of(c), None);
        assert_eq!(Disc::fly(chalk), Some(3), "a CO2 fizz flies as CO2");
        // Bad keys change nothing.
        let rev = d.rev;
        d.command("equip carbon_burns 5");
        d.command("equip carbon_burns 0");
        d.command("equip carbon_burns x");
        assert_eq!(d.rev, rev);
        assert_eq!(d.key_of(c), None);
        // The state says which key each saved spell is on.
        let json = d.state_json(Some(Tier::Seedling), &realgame_sim::valley::Pack::default());
        let v: realgame_chem::serde_json::Value =
            realgame_chem::serde_json::from_str(&json).unwrap();
        let row = |id: &str| {
            v["saved"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["id"] == id)
                .unwrap()
                .clone()
        };
        assert_eq!(row("chalk_acid")["key"], 2);
        assert!(row("carbon_burns")["key"].is_null());
        assert_eq!(row("carbon_burns")["slot"], 1, "the suggested key");
    }

    #[test]
    fn equip_puts_a_spell_on_its_rules_slot_and_the_key_switches() {
        let mut d = loaded();
        let c = index_of("carbon_burns").unwrap();
        d.command("equip carbon_burns");
        assert_eq!(d.active(0), Some(c));
        assert!(d.say.is_some());
        d.command("toggle 1");
        assert_eq!(d.active(0), None, "the shipped Fire again");
        assert!(d.toggle(0));
        assert_eq!(d.active(0), Some(c));
        d.command("equip haber");
        assert_eq!(d.active(3), None, "not saved: nothing equipped");
        d.command("unequip 1");
        assert_eq!(d.active(0), None);
        assert!(!d.toggle(0));
    }

    #[test]
    fn a_bench_only_reaction_is_never_saved_or_equipped() {
        let haber = index_of("haber").unwrap();
        let fx = entry(haber).unwrap().1;
        assert!(!fx.saveable());
        assert!(fx
            .bench_only
            .as_deref()
            .is_some_and(|w| w.contains("ammonia")));
        let mut d = loaded();
        d.command("saved haber:2");
        assert!(d.saved.iter().all(|s| s.0 != haber), "dropped on load");
        d.saved.push((haber, 2));
        assert!(!d.put(haber, 0), "never on a key");
        d.command("equip haber 1");
        assert_eq!(d.active(0), None);
    }

    #[test]
    fn two_builds_master_a_discovered_spell() {
        let mut d = loaded();
        let c = index_of("carbon_burns").unwrap();
        assert_eq!(d.stage(c), 1);
        d.record_build(c);
        assert_eq!(d.stage(c), 3);
        let fx = entry(c).unwrap().1;
        assert_eq!(Disc::tag(c).amount, fx.strength);
        // A hydrogen fizz lifts (I-056): it carries its strength, and its
        // landing is the world's, not a matter emit.
        let zinc = index_of("zinc_acid").unwrap();
        let zfx = entry(zinc).unwrap().1;
        assert_eq!(zfx.runs, d::Runs::Lift);
        assert_eq!(Disc::tag(zinc).amount, zfx.strength);
        assert!(!lands_as_matter(Disc::tag(zinc).disc));
        assert!(lands_as_matter(Disc::tag(c).disc));
        assert!(lands_as_matter(None), "a shipped spell");
        assert_eq!(Disc::def(c).unwrap().slot, 1);
    }

    #[test]
    fn a_cast_needs_its_bottles_and_uses_them() {
        let c = index_of("carbon_burns").unwrap();
        let mut pack = realgame_sim::valley::Pack::default();
        let plan = Disc::plan(c, &pack).unwrap();
        assert_eq!(plan.missing, vec!["C".to_string()]);
        let line = Disc::need_line(&plan.missing);
        assert!(line.contains("Coal") && line.contains("Charcoal"), "{line}");
        // It says where to get it, in plain words: the charcoal mound.
        assert!(
            line.contains("Charcoal: Heat wood with very little air")
                && line.contains("vent on the charcoal mound (Frozen Fields)"),
            "{line}"
        );
        assert!(!line.to_lowercase().contains("dig"), "{line}");
        // The pack lists what the spell uses and where it comes from.
        let uses = Disc::uses(c, &pack);
        assert_eq!(uses.len(), 1, "oxygen comes from the air");
        assert_eq!(uses[0]["species"], "C");
        assert_eq!(uses[0]["name"], "carbon (charcoal)");
        assert_eq!(uses[0]["have"], 0);
        assert_eq!(uses[0]["findItem"], "Charcoal");
        assert!(uses[0]["find"].as_str().unwrap().contains("Frozen Fields"));
        pack.add(Item::Block(realgame_sim::valley::mat::COAL_ORE), 2);
        assert_eq!(Disc::uses(c, &pack)[0]["have"], 2);
        let plan = Disc::plan(c, &pack).unwrap();
        assert!(plan.missing.is_empty());
        Disc::take(&plan, &mut pack);
        assert_eq!(
            pack.count(Item::Block(realgame_sim::valley::mat::COAL_ORE)),
            1
        );
    }

    #[test]
    fn the_melt_quest_needs_a_hot_discovered_spell_nearby() {
        let mut d = loaded();
        let c = index_of("carbon_burns").unwrap();
        let cold = index_of("baking_soda_vinegar").unwrap();
        d.landed(cold, Vec3::new(10.0, 30.0, 10.0), 1.0);
        d.melted([10, 30, 10], 1.2);
        assert!(
            d.quests_done.is_empty(),
            "a reaction that takes heat in does not count"
        );
        d.landed(c, Vec3::new(10.0, 30.0, 10.0), 2.0);
        d.melted([40, 30, 40], 2.2);
        assert!(d.quests_done.is_empty(), "too far");
        d.melted([10, 30, 10], 9.0);
        assert!(d.quests_done.is_empty(), "too late");
        d.landed(c, Vec3::new(10.0, 30.0, 10.0), 10.0);
        d.melted([11, 30, 10], 10.6);
        assert_eq!(d.quests_done, vec!["melt_with_reaction".to_string()]);
        assert_eq!(d.say.as_deref(), Some(QUEST_DONE));
        let json = d.state_json(Some(Tier::Seedling), &realgame_sim::valley::Pack::default());
        let v: realgame_chem::serde_json::Value =
            realgame_chem::serde_json::from_str(&json).unwrap();
        assert_eq!(v["quests"][0]["done"], true);
        assert_eq!(v["save"]["quests"][0], "melt_with_reaction");
        assert_eq!(v["save"]["saved"]["carbon_burns"], 1);
        assert_eq!(v["saved"][0]["effect"]["kind"], "fire");
    }

    /// A live quest by id.
    fn quest(id: &str) -> &'static d::ChemQuest {
        data().quests.quests.iter().find(|q| q.id == id).unwrap()
    }

    /// Where each quest's event happens in the valley (its place).
    fn place_of(q: &d::ChemQuest) -> Vec3 {
        q.at.map_or(Vec3::new(100.0, 35.0, 200.0), Vec3::from)
    }

    /// I-057: every live quest completes only when a matching discovered
    /// spell lands close to its event, in time, in its place.
    #[test]
    fn each_quest_completes_only_with_a_matching_discovered_spell() {
        // (quest, a reaction that solves it, one that does not)
        let cases = [
            ("melt_with_reaction", "carbon_burns", "baking_soda_vinegar"),
            ("smother_with_co2", "limestone_roast", "zinc_acid"),
            ("cool_the_vent", "baking_soda_vinegar", "limestone_roast"),
            (
                "wall_in_the_river",
                "silver_chloride",
                "iron_copper_sulfate",
            ),
            (
                "reveal_the_hidden_mark",
                "iron_copper_sulfate",
                "silver_chloride",
            ),
        ];
        let live: Vec<_> = data().quests.quests.iter().filter(|q| q.live).collect();
        assert_eq!(live.len(), cases.len(), "one case per live quest");
        for (id, good, bad) in cases {
            let q = quest(id);
            let at = place_of(q);
            let good = index_of(good).unwrap();
            let bad = index_of(bad).unwrap();
            let done = |d: &Disc| d.quests_done.contains(&q.id);
            let mut d = Disc::default();
            // A shipped spell never calls `landed`: the event alone does
            // nothing.
            d.event(&q.event, at, 1.0);
            assert!(!done(&d), "{id}: the event alone");
            // A discovered spell that does not match.
            d.landed(bad, at, 2.0);
            d.event(&q.event, at, 2.1);
            assert!(!done(&d), "{id}: a spell that does not match");
            // Another quest's event.
            d.landed(good, at, 3.0);
            for ev in d::EVENTS.iter().filter(|e| **e != q.event) {
                d.event(ev, at, 3.1);
            }
            assert!(!done(&d), "{id}: another event");
            // Too far from the landing.
            let far = at + Vec3::new(q.radius_m + 1.0, 0.0, 0.0);
            let mut d = Disc::default();
            d.landed(good, far, 1.0);
            d.event(&q.event, at, 1.1);
            assert!(!done(&d), "{id}: too far");
            // Too late.
            d.landed(good, at, 5.0);
            d.event(&q.event, at, 5.0 + q.window_s + 0.5);
            assert!(!done(&d), "{id}: too late");
            // Out of its place (a quest with a place).
            if let (Some(_), Some(r)) = (q.at, q.near_m) {
                let away = at + Vec3::new(r + 5.0, 0.0, 0.0);
                d.landed(good, away, 20.0);
                d.event(&q.event, away, 20.1);
                assert!(!done(&d), "{id}: not in its place");
            }
            // The matching spell, close by and in time.
            d.landed(good, at, 30.0);
            d.event(
                &q.event,
                at + Vec3::new(0.5, 0.0, 0.0),
                30.0 + q.window_s * 0.5,
            );
            assert!(done(&d), "{id}: done");
            assert_eq!(d.say.as_deref(), Some(q.done.as_str()));
            let rev = d.rev;
            d.landed(good, at, 31.0);
            d.event(&q.event, at, 31.1);
            assert_eq!(d.quests_done.len(), 1, "{id}: done once");
            assert_eq!(d.rev, rev);
        }
    }

    /// A reported event is handled in the quest system (`pending`).
    #[test]
    fn reported_events_wait_for_the_quest_system() {
        let mut d = Disc::default();
        let q = quest("wall_in_the_river");
        let at = place_of(q);
        d.landed(index_of("silver_chloride").unwrap(), at, 1.0);
        d.report(d::EVENT_WALLED_WATER, at);
        assert!(d.quests_done.is_empty());
        for (ev, p) in std::mem::take(&mut d.pending) {
            d.event(ev, p, 1.2);
        }
        assert_eq!(d.quests_done, vec![q.id.clone()]);
    }

    /// Quest state goes into the shared save and comes back from it.
    #[test]
    fn the_save_round_trips_quest_state() {
        let mut d = loaded();
        for (id, r) in [
            ("smother_with_co2", "limestone_roast"),
            ("reveal_the_hidden_mark", "iron_copper_sulfate"),
        ] {
            let q = quest(id);
            d.landed(index_of(r).unwrap(), place_of(q), 1.0);
            d.event(&q.event, place_of(q), 1.5);
        }
        let json = d.state_json(Some(Tier::Seedling), &realgame_sim::valley::Pack::default());
        let v: realgame_chem::serde_json::Value =
            realgame_chem::serde_json::from_str(&json).unwrap();
        let saved: Vec<String> = v["save"]["quests"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap().to_string())
            .collect();
        assert_eq!(saved, vec!["smother_with_co2", "reveal_the_hidden_mark"]);
        // Every live quest is listed with its line, hint and place.
        let list = v["quests"].as_array().unwrap();
        assert_eq!(list.len(), 5);
        for row in list {
            let q = quest(row["id"].as_str().unwrap());
            assert_eq!(row["line"], q.line.as_str());
            assert_eq!(row["hint"], q.hint.as_str());
            assert_eq!(row["place"], q.place.as_str());
            assert_eq!(row["done"], d.quests_done.contains(&q.id));
        }
        // A new run loads it (formula-save.js sends `quests a,b`), and
        // unknown ids are dropped.
        let mut back = Disc::default();
        back.command(&format!("quests {},nonsense", saved.join(",")));
        assert_eq!(back.quests_done, d.quests_done);
        let again = back.state_json(Some(Tier::Sprout), &realgame_sim::valley::Pack::default());
        let w: realgame_chem::serde_json::Value =
            realgame_chem::serde_json::from_str(&again).unwrap();
        assert_eq!(w["save"]["quests"], v["save"]["quests"]);
        // Sprout gets its own wording.
        let row = &w["quests"][1];
        assert_eq!(row["line"], quest("smother_with_co2").line_sprout.as_str());
        assert_eq!(row["hint"], quest("smother_with_co2").hint_sprout.as_str());
    }
}
