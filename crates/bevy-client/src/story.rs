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
    app.init_resource::<Keys>()
        .init_resource::<Story>()
        .add_systems(
            Update,
            (commands, on_event, intro_camera, keys)
                .chain()
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
fn commands(mut story: ResMut<Story>, mut pack: ResMut<crate::Pack>) {
    for line in drain() {
        with_pack(&mut story, &mut pack, |s| run_command(s, &line));
    }
}

/// Hears what the world and the player did.
pub fn on_event(
    mut story: ResMut<Story>,
    mut pack: ResMut<crate::Pack>,
    player_pos: Res<Player>,
    mut player: MessageReader<PlayerEvent>,
    mut matter: MessageReader<MatterEvent>,
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
            PlayerEvent::InBox { stamp } => {
                if *stamp == realgame_voxel::stamp::FORD.id && player_pos.pos.z < 150.0 {
                    story.quest.on(QuestEvent::CrossedFord);
                }
            }
            PlayerEvent::EnteredBiome { .. }
            | PlayerEvent::UsedRig
            | PlayerEvent::UsedLever
            | PlayerEvent::UsedKilnDoor => {}
        }
    }
    // Matter events reach the quest on Tuesday (spec section 12).
    for _ in matter.read() {}
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
        "{{\"v\":1,\"step\":{},\"biomeName\":\"{}\",\"intro\":[{}],\"recipes\":[{}],\"slots\":[{}],\"hot\":[{}],\"practised\":[{}],\"craft\":{craft}}}",
        story.quest.step(),
        valley::biome_name(&story.biome),
        intro.join(","),
        recipes.join(","),
        slots.join(","),
        hot.join(","),
        practised.join(",")
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
