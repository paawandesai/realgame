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
//! - One chemistry quest runs here: melt ice with a discovered spell that
//!   gives off heat.
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
/// The live chemistry quest's line, and Sprout's.
pub const QUEST_LINE: &str =
    "Chemistry quest: melt some ice with a reaction from the Bench that gives off heat.";
pub const QUEST_LINE_SPROUT: &str = "Chemistry quest: melt some ice with a hot reaction you found!";
/// Said when the quest is done.
pub const QUEST_DONE: &str =
    "Chemistry quest done! Your reaction gave off heat and the ice melted.";
/// How long (s) a landed discovered spell can still be the cause of a melt.
const QUEST_WINDOW_S: f32 = 2.5;

/// The effect rules, bottles and quests, loaded once.
pub struct Data {
    pub effects: Vec<Effect>,
    by_id: BTreeMap<String, usize>,
    pub bottles: Bottles,
    pub quests: Quests,
}

pub fn data() -> &'static Data {
    static D: OnceLock<Data> = OnceLock::new();
    D.get_or_init(|| {
        let codex = &spells::book().codex;
        let effects = Effects::load().all(codex);
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
        }
    })
}

/// A codex reaction by index, with its effect.
pub fn entry(i: usize) -> Option<(&'static Entry, &'static Effect)> {
    Some((spells::book().codex.entries.get(i)?, data().effects.get(i)?))
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
    /// Discovered spells that gave off heat and landed: where, and when.
    recent: Vec<(Vec3, f32)>,
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
        if k >= 4 || !self.saved.iter().any(|s| s.0 == idx) {
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
                    if let Some(i) = index_of(id) {
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

    /// A discovered spell landed at `pos` at game time `now`.
    pub fn landed(&mut self, idx: usize, pos: Vec3, now: f32) {
        let Some((_, fx)) = entry(idx) else { return };
        for q in data().quests.quests.iter().filter(|q| q.live) {
            if !self.quests_done.contains(&q.id) && q.needs.met_by(fx) && q.event == d::EVENT_MELTED
            {
                self.recent.push((pos, now));
            }
        }
    }

    /// The world melted ice at `cell` at time `now`: finishes the live quest
    /// when a qualifying discovered spell landed nearby just before.
    pub fn melted(&mut self, cell: [i32; 3], now: f32) {
        self.recent.retain(|r| now - r.1 <= QUEST_WINDOW_S);
        let p = Vec3::new(
            cell[0] as f32 + 0.5,
            cell[1] as f32 + 0.5,
            cell[2] as f32 + 0.5,
        );
        for q in data()
            .quests
            .quests
            .iter()
            .filter(|q| q.live && q.event == d::EVENT_MELTED)
        {
            if self.quests_done.contains(&q.id) {
                continue;
            }
            if self.recent.iter().any(|r| r.0.distance(p) <= q.radius_m) {
                self.quests_done.push(q.id.clone());
                self.rev += 1;
                self.say = Some(QUEST_DONE.to_string());
            }
        }
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
        let quests: Vec<_> = data()
            .quests
            .quests
            .iter()
            .filter(|q| q.live)
            .map(|q| {
                let line = if band == Some(Tier::Sprout) {
                    QUEST_LINE_SPROUT
                } else {
                    QUEST_LINE
                };
                json!({ "id": q.id, "line": line, "done": self.quests_done.contains(&q.id) })
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

fn quest(
    time: Res<Time>,
    mut disc: ResMut<Disc>,
    mut game: ResMut<crate::Game>,
    mut landed: MessageReader<crate::SpellLanded>,
    mut matter: MessageReader<crate::MatterEvent>,
) {
    let now = time.elapsed_secs();
    for l in landed.read() {
        if let Some(i) = l.disc {
            disc.landed(usize::from(i), l.pos, now);
        }
    }
    for e in matter.read() {
        if let realgame_matter::MatterEvent::Melted { cell } = e.0 {
            disc.melted(cell, now);
        }
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
    fn two_builds_master_a_discovered_spell() {
        let mut d = loaded();
        let c = index_of("carbon_burns").unwrap();
        assert_eq!(d.stage(c), 1);
        d.record_build(c);
        assert_eq!(d.stage(c), 3);
        let fx = entry(c).unwrap().1;
        assert_eq!(Disc::tag(c).amount, fx.strength);
        let zinc = index_of("zinc_acid").unwrap();
        assert_eq!(
            Disc::tag(zinc).amount,
            0.0,
            "a hydrogen fizz changes nothing"
        );
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
}
