//! The game layer's glue (I-052, Lane C): intro, objective line and arrow,
//! the quest (`realgame_sim::valley::Quest`), pack, crafting, the Invention
//! Bench and the ending. Registered only in `Mode::Voxel`.
//!
//! The page talks to it through its own queue, `window.__gameIn.story` (one
//! command string each: `intro_done`, `craft <id>`, `bind <slot> <key 6-9>`,
//! `worked_ok <spell 0-3>`, and with `?debug` `give <item> <n>` and
//! `quest <n>`), and reads `window.__storyState` (recipes, hotbar bindings,
//! practice), written only when it changes. The quest hears the world only
//! through `PlayerEvent`s and `MatterEvent`s (spec section 10).

use bevy::prelude::*;
use realgame_sim::tier::Tier;
use realgame_sim::valley::{self, practice::RunPractice, Item, Pack, Quest, QuestEvent};

use crate::bench::BenchState;
use crate::player::Player;
use crate::{MatterEvent, PlayerEvent};

/// This plugin's values for the voxel-mode state keys (spec section 8);
/// `publish` formats them. Values, not JSON fragments.
#[derive(Resource, Debug, Clone, Default)]
pub struct Keys {
    /// `quest`: the step, 0-8.
    pub quest: u8,
    /// `questLine`: the objective text.
    pub quest_line: String,
    /// `pack`: `(item, n)` per filled slot.
    pub pack: Vec<(String, u16)>,
    /// `objBearing`: degrees to the step's target, or none.
    pub obj_bearing: Option<f32>,
    /// `ending`: the ending card has shown.
    pub ending: bool,
}

/// The run's game state: quest, pack and the in-run practice counts.
#[derive(Resource, Debug, Clone)]
pub struct Story {
    pub quest: Quest,
    pub pack: Pack,
    pub practice: RunPractice,
    /// `?debug`: `give` and `quest` are allowed.
    debug: bool,
    /// The last `__storyState` written.
    last: String,
    /// The last craft: id, and the reason it was refused (empty: made).
    craft: Option<(String, String)>,
    /// The biome the player is in (`voxel_view`'s key), for its name.
    biome: String,
    /// Water has put out a burning block in the workshop (step 1).
    water_on_workshop: bool,
    /// The biome gates and their signs, as JSON (`gates`, round 2).
    pub gates_json: String,
}

impl Default for Story {
    fn default() -> Self {
        let mut pack = Pack::default();
        // Everyone starts with the Pick (key 5 selects it).
        pack.add(Item::Pick, 1);
        Story {
            quest: Quest::new(),
            pack,
            practice: RunPractice::default(),
            debug: crate::param("debug").is_some(),
            last: String::new(),
            craft: None,
            biome: "village".to_string(),
            water_on_workshop: false,
            gates_json: "[]".to_string(),
        }
    }
}

/// Where each step's target is (x, z), for the objective arrow (spec section
/// 4): workshop, ford, ice gate, coal seam, hollow, rig, Kiln.
const TARGETS: [(f32, f32); 7] = [
    (142.0, 190.0),
    (128.0, 154.0),
    (60.0, 90.0),
    (60.0, 72.0),
    (191.0, 101.0),
    (231.0, 29.0),
    (128.0, 169.0),
];

/// How fast the intro camera pans while the intro cards show, radians/s.
const INTRO_PAN: f32 = 0.08;

/// Registers the story (Voxel mode only).
pub fn plugin(app: &mut App) {
    app.add_plugins(crate::gates::plugin)
        .init_resource::<Keys>()
        .init_resource::<Story>()
        .add_systems(
            Update,
            (commands, on_event, practice_sync, intro_camera, keys)
                .chain()
                .after(crate::aim_and_cast)
                .before(crate::publish),
        );
}

/// The item digging block `mat` gives, if any (ice melts in your hand;
/// water, steam, air and fixtures give nothing).
pub fn drop_of(mat: u8) -> Option<Item> {
    use realgame_voxel::mat as m;
    match mat {
        m::AIR
        | m::ICE
        | m::WATER
        | m::STEAM
        | m::GAS_VENT
        | m::BENCH
        | m::INVENTION_BENCH
        | m::KILN_HEARTH
        | m::RUNESTONE
        | m::CLIFFSTONE => None,
        m::LEAVES | m::ASH => None,
        m::GRASS => Some(Item::Block(m::DIRT)),
        m::TORCH => Some(Item::Torch),
        _ => Some(Item::Block(mat)),
    }
}

/// Runs one command from the page's queue.
pub fn run_command(story: &mut Story, line: &str) {
    let a: Vec<&str> = line.split_whitespace().collect();
    let num = |i: usize| a.get(i).and_then(|v| v.parse::<i64>().ok());
    match a.first().copied() {
        Some("intro_done") => {
            story.quest.on(QuestEvent::IntroDone);
        }
        Some("craft") => {
            let id = a.get(1).copied().unwrap_or("");
            let why = match valley::craft(&mut story.pack, id) {
                Ok(()) => String::new(),
                Err(valley::CraftError::NotEnough { item, have, need }) => {
                    format!("not_enough {} {have} {need}", item.key())
                }
                Err(valley::CraftError::UnknownRecipe) => "unknown".to_string(),
                Err(valley::CraftError::PackFull) => "pack_full".to_string(),
            };
            story.craft = Some((id.to_string(), why));
            holding(story, None);
        }
        Some("bind") => {
            if let (Some(slot), Some(key)) = (num(1), num(2)) {
                if (0..valley::SLOTS as i64).contains(&slot) && (6..=9).contains(&key) {
                    story.pack.hot[(key - 6) as usize] = Some(slot as u8);
                }
            }
        }
        Some("worked_ok") => {
            if let Some(s) = num(1) {
                story.practice.record(s as usize);
            }
        }
        Some("give") if story.debug => {
            if let (Some(item), Some(n)) = (a.get(1).and_then(|k| Item::from_key(k)), num(2)) {
                story.pack.add(item, n.clamp(0, 640) as u16);
                holding(story, Some(item));
            }
        }
        Some("quest") if story.debug => {
            if let Some(n) = num(1) {
                story.quest = Quest::at(n.clamp(0, 8) as u8);
            }
        }
        Some("give" | "quest") => warn!("STORY needs_debug={line:?}"),
        _ => warn!("STORY unknown={line:?}"),
    }
}

/// Tells the quest what the pack holds now (`item`: just this one; `None`:
/// the coal and the Spark, the items the quest waits for).
fn holding(story: &mut Story, item: Option<Item>) {
    let items = match item {
        Some(i) => vec![i],
        None => vec![Item::Block(valley::mat::COAL_ORE), Item::KilnSpark],
    };
    for item in items {
        let total = story.pack.count(item);
        story.quest.on(QuestEvent::Holding { item, total });
    }
}

/// Runs `f` on the story with the run's one pack, `crate::Pack` (the voxel
/// view gathers into it and places from it), swapped into `story.pack` for
/// the call. Outside these calls `story.pack` is unused.
fn with_pack<R>(story: &mut Story, pack: &mut crate::Pack, f: impl FnOnce(&mut Story) -> R) -> R {
    std::mem::swap(&mut story.pack, &mut pack.0);
    let r = f(story);
    std::mem::swap(&mut story.pack, &mut pack.0);
    r
}

/// Drains `window.__gameIn.story`.
fn commands(
    mut story: ResMut<Story>,
    mut pack: ResMut<crate::Pack>,
    mut game: ResMut<crate::Game>,
) {
    let before = story.quest.step();
    for line in drain() {
        with_pack(&mut story, &mut pack, |s| run_command(s, &line));
    }
    // The welcome caption waited under the intro: say it now it has closed.
    if before == 0 && story.quest.step() == 1 && story.biome == "village" {
        game.say(crate::voxel_view::WELCOME_LINE);
    }
}

/// Hears what the world and the player did.
pub fn on_event(
    mut story: ResMut<Story>,
    mut pack: ResMut<crate::Pack>,
    player_pos: Res<Player>,
    mut player: MessageReader<PlayerEvent>,
    mut matter: MessageReader<MatterEvent>,
    fire: Option<Res<crate::blockchem::Keys>>,
) {
    let story = &mut *story;
    for e in player.read() {
        match e {
            // The voxel view already put the drop in the pack (or refused
            // the dig when it was full); the quest hears what it holds now.
            PlayerEvent::Dug { mat } => {
                if let Some(item) = drop_of(*mat) {
                    with_pack(story, &mut pack, |s| holding(s, Some(item)));
                }
            }
            // The view took the placed block out of the pack.
            PlayerEvent::Placed { .. } => {}
            // Crossing the ford is a position check (`crossing_progress`):
            // entering the ford's box happens on the water, not across it.
            PlayerEvent::InBox { .. }
            | PlayerEvent::EnteredBiome { .. }
            | PlayerEvent::UsedRig
            | PlayerEvent::UsedLever
            | PlayerEvent::UsedKilnDoor => {}
        }
    }
    let events: Vec<_> = matter.read().map(|e| e.0).collect();
    let burning = fire.as_ref().map_or(u32::MAX, |k| k.workshop_burning);
    fire_progress(story, &events, burning);
    let gate_ice = fire.as_ref().map_or(u32::MAX, |k| k.gate_ice);
    crossing_progress(story, player_pos.pos, gate_ice);
    let vents = fire.as_ref().map_or(u32::MAX, |k| k.vents_lit);
    hollow_progress(story, player_pos.pos, vents);
}

/// Step 1 (P1): remembers water putting out a workshop block, and tells
/// the quest once nothing in the workshop burns (`workshop_burning`, from
/// `blockchem`) after water did.
pub fn fire_progress(
    story: &mut Story,
    events: &[realgame_matter::MatterEvent],
    workshop_burning: u32,
) {
    use realgame_matter::{By, MatterEvent as M};
    for e in events {
        if let M::Extinguished {
            cell,
            by: By::Water,
        } = e
        {
            if realgame_voxel::stamp::WORKSHOP.contains(*cell) {
                story.water_on_workshop = true;
            }
        }
    }
    if workshop_burning == 0 && story.water_on_workshop {
        story.quest.on(QuestEvent::WorkshopOut { by_water: true });
    }
}

/// Run practice (Paawan, 2026-11-01): each correct worked cast this frame
/// (or a cast with nothing to build) counts toward its spell; three make it
/// Practised for this run, which turns its worked cast into a one-step
/// confirm (`Working::practised`) and opens the biome gate that needs it.
fn practice_sync(
    mut story: ResMut<Story>,
    mut working: ResMut<crate::Working>,
    mut game: ResMut<crate::Game>,
) {
    for (spell, ok) in std::mem::take(&mut working.cast_log) {
        if ok && story.practice.record(spell) {
            let name = crate::chem::Spell::ALL.get(spell).map_or("", |s| s.name());
            game.say(&format!(
                "{name}: Practised! Its cast is now one quick confirm."
            ));
        }
    }
    for i in 0..4 {
        working.practised[i] = story.practice.practised(i);
    }
}

/// Steps 2 and 3 (P2): across the ford is standing on its north bank (z
/// below the water, x in the ford's columns); the gate is melted when its
/// stamp box holds no ice (`gate_ice`, from `blockchem`).
pub fn crossing_progress(story: &mut Story, pos: Vec3, gate_ice: u32) {
    let (x0, x1) = realgame_voxel::gen::FORD_X;
    let across = pos.z < realgame_voxel::gen::WATER_Z.0 as f32
        && pos.z > (realgame_voxel::gen::CANYON_Z.0 - 4) as f32
        && pos.x >= x0 as f32
        && pos.x < x1 as f32;
    if across {
        story.quest.on(QuestEvent::CrossedFord);
    }
    if gate_ice == 0 {
        story.quest.on(QuestEvent::GateMelted);
    }
}

/// Step 5 (P3): every vent in the methane hollow is out at once
/// (`vents_lit`, from `blockchem`) while the player is in the hollow (its
/// stamp box, along the ground): the burning gas no longer blocks the way.
pub fn hollow_progress(story: &mut Story, pos: Vec3, vents_lit: u32) {
    let h = realgame_voxel::stamp::HOLLOW;
    let inside = pos.x >= h.min[0] as f32
        && pos.x < h.max[0] as f32
        && pos.z >= h.min[2] as f32
        && pos.z < h.max[2] as f32
        && pos.y < (h.max[1] + 2) as f32;
    if vents_lit == 0 && inside {
        story.quest.on(QuestEvent::CrossedHollow);
    }
}

/// The intro's slow camera pan over the valley while the cards show.
fn intro_camera(time: Res<Time>, story: Res<Story>, mut player: ResMut<Player>) {
    if story.quest.step() == 0 {
        player.yaw += INTRO_PAN * time.delta_secs().min(0.1);
    }
}

/// Fills the state keys and `__storyState`.
fn keys(
    mut story: ResMut<Story>,
    mut pack: ResMut<crate::Pack>,
    keys: ResMut<Keys>,
    player: Res<Player>,
    bench: Option<Res<BenchState>>,
    view: Option<Res<crate::voxel_view::Keys>>,
) {
    with_pack(&mut story, &mut pack, |s| {
        fill_keys(s, keys, &player, bench, view)
    });
}

fn fill_keys(
    story: &mut Story,
    mut keys: ResMut<Keys>,
    player: &Player,
    bench: Option<Res<BenchState>>,
    view: Option<Res<crate::voxel_view::Keys>>,
) {
    story.biome = view.map_or("village".to_string(), |v| v.biome.to_string());
    let band: Option<Tier> = bench.as_ref().and_then(|b| b.band);
    let step = story.quest.step();
    keys.quest = step;
    let line = story.quest.line(band);
    if keys.quest_line != line {
        keys.quest_line = line.to_string();
    }
    keys.pack = story
        .pack
        .entries()
        .into_iter()
        .map(|(k, n)| (k.to_string(), n))
        .collect();
    keys.obj_bearing = (1..=7).contains(&step).then(|| {
        let (tx, tz) = TARGETS[step as usize - 1];
        let eye = player.eye();
        crate::cues::bearing_deg(eye.x, eye.z, player.yaw, tx, tz)
    });
    keys.ending = step > valley::STEPS;
    let json = state_json(story, band);
    if json != story.last {
        publish(&json);
        story.last = json;
    }
}

/// `window.__storyState`: the recipes with what the pack has, the hotbar
/// bindings, the practice counts and the last craft.
pub fn state_json(story: &Story, band: Option<Tier>) -> String {
    let recipes: Vec<String> = valley::RECIPES
        .iter()
        .map(|r| {
            let needs: Vec<String> = r
                .needs
                .iter()
                .map(|(item, n)| {
                    format!(
                        "{{\"item\":\"{}\",\"n\":{n},\"have\":{}}}",
                        item.key(),
                        story.pack.count(*item)
                    )
                })
                .collect();
            format!(
                "{{\"id\":\"{}\",\"can\":{},\"needs\":[{}],\"gives\":{{\"item\":\"{}\",\"n\":{}}}}}",
                r.id,
                valley::can_craft(&story.pack, r.id),
                needs.join(","),
                r.gives.0.key(),
                r.gives.1
            )
        })
        .collect();
    let hot: Vec<String> = story
        .pack
        .hot
        .iter()
        .map(|h| h.map_or("null".to_string(), |s| s.to_string()))
        .collect();
    let slots: Vec<String> = story
        .pack
        .slots
        .iter()
        .map(|s| {
            s.map_or("null".to_string(), |s| {
                format!(
                    "{{\"item\":\"{}\",\"label\":\"{}\",\"n\":{}}}",
                    s.item.key(),
                    s.item.label(),
                    s.n
                )
            })
        })
        .collect();
    let practised: Vec<String> = (0..4)
        .map(|i| story.practice.practised(i).to_string())
        .collect();
    let craft = story
        .craft
        .as_ref()
        .map_or("null".to_string(), |(id, why)| {
            format!(
                "{{\"id\":\"{id}\",\"ok\":{},\"why\":\"{why}\"}}",
                why.is_empty()
            )
        });
    let intro: Vec<String> = valley::intro(band)
        .iter()
        .map(|t| format!("\"{}\"", t.replace('\\', "\\\\").replace('"', "\\\"")))
        .collect();
    format!(
        "{{\"v\":1,\"step\":{},\"biomeName\":\"{}\",\"intro\":[{}],\"recipes\":[{}],\"slots\":[{}],\"hot\":[{}],\"practised\":[{}],\"craft\":{craft},\"gates\":{}}}",
        story.quest.step(),
        valley::biome_name(&story.biome),
        intro.join(","),
        recipes.join(","),
        slots.join(","),
        hot.join(","),
        practised.join(","),
        story.gates_json
    )
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
    let q = get(&gi, "story");
    if !Array::is_array(&q) {
        return Vec::new();
    }
    let arr = Array::from(&q);
    if arr.length() == 0 {
        return Vec::new();
    }
    let out = arr.iter().filter_map(|v| v.as_string()).collect();
    let _ = Reflect::set(&gi, &"story".into(), &Array::new());
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
            web_sys::js_sys::Reflect::set(&win, &"__storyState".into(), &JsValue::from_str(json));
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn publish(_json: &str) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valley_material_ids_match_the_voxel_table() {
        use realgame_voxel::mat;
        assert_eq!(&valley::mat::NAMES[..mat::COUNT], &mat::NAMES[..]);
        assert_eq!(valley::mat::DIRT, mat::DIRT);
        assert_eq!(valley::mat::STONE, mat::STONE);
        assert_eq!(valley::mat::WOOD, mat::WOOD);
        assert_eq!(valley::mat::COAL_ORE, mat::COAL_ORE);
        assert_eq!(
            valley::mat::FURNACE as usize,
            mat::COUNT,
            "the furnace is appended"
        );
    }

    #[test]
    fn commands_drive_the_quest_pack_and_crafting() {
        let mut s = Story {
            debug: true,
            ..Story::default()
        };
        assert_eq!(s.pack.count(Item::Pick), 1, "everyone starts with the Pick");
        run_command(&mut s, "intro_done");
        assert_eq!(s.quest.step(), 1);
        run_command(&mut s, "craft torch");
        assert_eq!(
            s.craft.as_ref().map(|c| c.1.as_str()),
            Some("not_enough wood 0 1")
        );
        run_command(&mut s, "give wood 1");
        run_command(&mut s, "give coal 1");
        run_command(&mut s, "craft torch");
        assert_eq!(s.craft.as_ref().map(|c| c.1.as_str()), Some(""));
        assert_eq!(s.pack.count(Item::Torch), 4);
        run_command(&mut s, "bind 3 7");
        assert_eq!(s.pack.hot, [None, Some(3), None, None]);
        run_command(&mut s, "bind 3 5");
        assert_eq!(s.pack.hot, [None, Some(3), None, None], "only keys 6-9");
        // Practice: three correct worked casts of Water.
        for _ in 0..3 {
            run_command(&mut s, "worked_ok 1");
        }
        assert!(s.practice.practised(1));
        let j = state_json(&s, None);
        assert!(j.contains("\"intro\":[\"The Great Kiln"), "{j}");
        assert!(j.contains("\"practised\":[false,true,false,false]"), "{j}");
        assert!(j.contains("{\"id\":\"torch\",\"can\":false"), "{j}");
        // Without ?debug, give and quest do nothing.
        let mut plain = Story {
            debug: false,
            ..Story::default()
        };
        run_command(&mut plain, "give coal 5");
        run_command(&mut plain, "quest 5");
        assert_eq!(plain.pack.count(Item::Block(valley::mat::COAL_ORE)), 0);
        assert_eq!(plain.quest.step(), 0);
    }

    /// Round 2 (P1): step 1 completes once no block in the workshop burns
    /// and water put at least one of them out; not before, and not when
    /// the fire went out some other way.
    #[test]
    fn the_workshop_step_completes_when_water_puts_its_fire_out() {
        use realgame_matter::{By, MatterEvent as M};
        let in_shop = [142, 35, 188];
        let elsewhere = [10, 40, 10];
        let mut s = Story {
            quest: Quest::at(1),
            ..Story::default()
        };
        // Water put one cell out, but the store still burns.
        fire_progress(
            &mut s,
            &[M::Extinguished {
                cell: in_shop,
                by: By::Water,
            }],
            5,
        );
        assert_eq!(s.quest.step(), 1, "still burning");
        // Out, with water earlier: done, and the objective moves on.
        fire_progress(&mut s, &[], 0);
        assert_eq!(s.quest.step(), 2, "the workshop is out");
        // Out without water (smothered, or outside the workshop): not done.
        let mut t = Story {
            quest: Quest::at(1),
            ..Story::default()
        };
        fire_progress(
            &mut t,
            &[
                M::Extinguished {
                    cell: in_shop,
                    by: By::NoOxygen,
                },
                M::Extinguished {
                    cell: elsewhere,
                    by: By::Water,
                },
            ],
            0,
        );
        assert_eq!(t.quest.step(), 1, "water never touched the workshop fire");
    }

    /// Round 2 (P2): step 2 completes when the player stands on the north
    /// bank of the ford (z < 150, in the ford's x range); step 3 when the
    /// ice gate holds no ice.
    #[test]
    fn crossing_the_ford_and_melting_the_gate_move_the_objective_on() {
        let mut s = Story {
            quest: Quest::at(2),
            ..Story::default()
        };
        crossing_progress(&mut s, Vec3::new(128.5, 32.0, 155.0), 12);
        assert_eq!(s.quest.step(), 2, "still on the water");
        crossing_progress(&mut s, Vec3::new(60.5, 40.0, 140.0), 12);
        assert_eq!(s.quest.step(), 2, "north, but not by the ford");
        crossing_progress(&mut s, Vec3::new(128.5, 32.0, 149.4), 12);
        assert_eq!(s.quest.step(), 3, "across the ford");
        crossing_progress(&mut s, Vec3::new(60.5, 36.0, 96.0), 3);
        assert_eq!(s.quest.step(), 3, "ice left in the gate");
        crossing_progress(&mut s, Vec3::new(60.5, 36.0, 96.0), 0);
        assert_eq!(s.quest.step(), 4, "the gate melted");
    }

    /// Round 2 (P3): step 5 completes when no vent in the hollow burns and
    /// the player is in the hollow (on its boardwalk or floor).
    #[test]
    fn the_hollow_step_completes_when_every_vent_is_out_and_the_player_is_in_it() {
        let mut s = Story {
            quest: Quest::at(5),
            ..Story::default()
        };
        hollow_progress(&mut s, Vec3::new(191.5, 29.0, 100.5), 2);
        assert_eq!(s.quest.step(), 5, "two vents still burn");
        hollow_progress(&mut s, Vec3::new(170.5, 33.0, 100.5), 0);
        assert_eq!(
            s.quest.step(),
            5,
            "out, but the player is not in the hollow"
        );
        hollow_progress(&mut s, Vec3::new(191.5, 29.0, 100.5), 0);
        assert_eq!(s.quest.step(), 6, "smothered and crossed");
    }

    #[test]
    fn digging_drops_items_and_coal_finishes_the_coal_step() {
        use realgame_voxel::mat;
        assert_eq!(drop_of(mat::COAL_ORE), Some(Item::Block(mat::COAL_ORE)));
        assert_eq!(drop_of(mat::GRASS), Some(Item::Block(mat::DIRT)));
        assert_eq!(drop_of(mat::ICE), None, "ice melts in your hand");
        assert_eq!(drop_of(mat::RUNESTONE), None);
        let mut s = Story {
            debug: true,
            quest: Quest::at(4),
            ..Story::default()
        };
        s.pack.add(Item::Block(mat::COAL_ORE), 1);
        holding(&mut s, Some(Item::Block(mat::COAL_ORE)));
        assert_eq!(s.quest.step(), 5);
    }
}
