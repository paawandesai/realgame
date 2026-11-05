//! The Formula Bench in the courtyard (I-028): the lab pad landmark, the
//! bench's command queue (`window.__gameIn.bench`) and its view
//! (`window.__benchState`). Judging is `realgame_sim::bench::judge_with` and
//! the shared codex (`spells::ClientSpells::judge`); the UI is
//! `web/bench.js`. See `docs/loop/items/I-028/spec.md`, sections 1, 5 and 6.
//!
//! The pad is a landmark, never a trigger: standing on it only sets
//! `atBench` (the page lights the bench button). The bench opens only from
//! the page (`B` or the button), which pushes `open`. This module never reads
//! or writes `Game`, `Chem`, `Settings` or the cast queue, never casts and
//! never unpauses.

use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;
use realgame_sim::bench::{self as sim, BenchView, ResultView, SpellCard, Verdict};
use realgame_sim::spellbook::SpellDef;
use realgame_sim::tier::Tier;

use crate::bridge::{self, WebIn};
use crate::discover::Disc;
use crate::player::Player;
use crate::progress::{Book, Stage};
use crate::Progress;

/// The pad's centre on the ground (x, z), about 41° left of the start view.
pub const STATION: (f32, f32) = (-3.0, 14.5);
/// The pad's radius (m): `atBench` while the player's feet are this close.
pub const RADIUS: f32 = 1.2;
/// Height of the flask icon's bulb above the pad's centre (m).
const FLASK_Y: f32 = 2.2;

/// The bench's state. `pub` so I-029's worked casts and I-030's trial read
/// it (`band` in particular, kept when the bench closes).
#[derive(Resource, Default)]
pub struct BenchState {
    pub open: bool,
    /// Set only by the `band` command.
    pub band: Option<Tier>,
    /// The picked slot; 0 = none.
    pub pick: u8,
    pub result: Option<ResultView>,
    /// Bumps per judged try.
    pub n: u32,
    /// The last JSON published, so `__benchState` is written only on change.
    pub last_json: String,
    /// The one Bench (I-058): a saved discovery picked to practise (its
    /// codex index). A `try` while it is set is judged against its own
    /// reaction and counts toward its mastery only.
    pub disc: Option<usize>,
}

/// One command from `window.__gameIn.bench`.
#[derive(Debug, Clone, PartialEq)]
pub enum Cmd {
    Band(Tier),
    Open,
    Pick(u8),
    /// The slot and the rest of the line, words joined by one space.
    Try(u8, String),
    Close,
    /// `learn <codex id>`: pick a saved discovery to practise (I-058).
    Learn(String),
}

/// Parses one command line; `None` for an unknown word or a malformed
/// argument (the caller warns).
pub fn parse(line: &str) -> Option<Cmd> {
    let mut words = line.split_whitespace();
    let word = words.next()?;
    match word {
        "band" => {
            let t = Tier::from_id(words.next()?)?;
            words.next().is_none().then_some(Cmd::Band(t))
        }
        "open" => words.next().is_none().then_some(Cmd::Open),
        "close" => words.next().is_none().then_some(Cmd::Close),
        "pick" => {
            let slot = words.next()?.parse::<u8>().ok()?;
            words.next().is_none().then_some(Cmd::Pick(slot))
        }
        "try" => {
            let slot = words.next()?.parse::<u8>().ok()?;
            let text = words.collect::<Vec<_>>().join(" ");
            Some(Cmd::Try(slot, text))
        }
        "learn" => {
            let id = words.next()?;
            let ok = id.len() <= 48
                && id
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
            (ok && words.next().is_none()).then(|| Cmd::Learn(id.to_string()))
        }
        _ => None,
    }
}

/// True while the player's feet are within `RADIUS` of `STATION`
/// (horizontal distance; height is ignored).
pub fn at_bench(pos: Vec3) -> bool {
    let (dx, dz) = (pos.x - STATION.0, pos.z - STATION.1);
    dx * dx + dz * dz <= RADIUS * RADIUS && crate::bench_on()
}

/// The Formula Bench on the block world's plaza (`voxel::gen::BENCH`): the
/// centre of its block, and the radius (m) within which `atBench` lights.
pub const VOXEL_STATION: (f32, f32) = (124.5, 200.5);
pub const VOXEL_RADIUS: f32 = 3.5;

/// Field Benches (small stone shrines stamped by `powers.rs`): at the
/// Village's ford approach and on the Frozen side of the Marsh gate. They
/// teach and craft the same as the plaza's, so the quest never sends a
/// player all the way back to the Village to learn the next spell.
pub const FIELD_STATIONS: [(f32, f32); 2] = [(146.5, 162.5), (118.5, 70.5)];

/// True while the player stands next to the block world's Formula Bench or
/// a field Bench.
pub fn at_voxel_bench(pos: Vec3) -> bool {
    std::iter::once(VOXEL_STATION)
        .chain(FIELD_STATIONS)
        .any(|(x, z)| {
            let (dx, dz) = (pos.x - x, pos.z - z);
            dx * dx + dz * dz <= VOXEL_RADIUS * VOXEL_RADIUS
        })
}

fn def_by_slot(spells: &[SpellDef], slot: u8) -> Option<&SpellDef> {
    spells.iter().find(|d| d.slot == slot)
}

/// The progress index of a slot (slot - 1), if the slot names a spell.
fn index_of(spells: &[SpellDef], slot: u8) -> Option<usize> {
    def_by_slot(spells, slot).map(|d| usize::from(d.slot) - 1)
}

fn stage_of(book: &Book, i: Option<usize>) -> Stage {
    i.and_then(|i| book.get(i))
        .map(|p| p.stage)
        .unwrap_or_default()
}

/// Records a judged try: bumps `n`, learns on a first correct build
/// (Unknown -> Learning only; any other stage earns nothing here) and stores
/// the result. `learned` is true only when `learn` returned a transition.
pub fn record(st: &mut BenchState, book: &mut Book, spells: &[SpellDef], slot: u8, v: Verdict) {
    st.n += 1;
    let i = index_of(spells, slot);
    let mut learned = false;
    if v.ok {
        if let Some(i) = i {
            if stage_of(book, Some(i)) == Stage::Unknown {
                learned = !book.learn(i).is_empty();
            }
        }
    }
    st.result = Some(ResultView {
        n: st.n,
        slot,
        verdict: v,
        learned,
        stage: stage_of(book, i).as_str().to_string(),
    });
}

/// The spell (0-3) a correct build just judged counts for, if `n` moved past
/// `before` with a correct verdict.
fn practice_index(st: &BenchState, before: u32) -> Option<usize> {
    let r = st
        .result
        .as_ref()
        .filter(|r| r.n > before && r.verdict.ok)?;
    usize::from(r.slot).checked_sub(1).filter(|&i| i < 4)
}

/// Applies one command. `Err` carries the reason it was ignored (warned,
/// never an error).
pub fn apply(
    st: &mut BenchState,
    book: &mut Book,
    spells: &[SpellDef],
    cmd: Cmd,
) -> Result<(), &'static str> {
    match cmd {
        Cmd::Band(t) => {
            st.band = Some(t);
            if st.open {
                st.pick = 0;
                st.disc = None;
                st.result = None;
            }
        }
        Cmd::Open => {
            st.open = true;
            st.pick = 0;
            st.disc = None;
            st.result = None;
        }
        Cmd::Close => {
            if !st.open {
                return Err("close while closed");
            }
            st.open = false;
            st.pick = 0;
            st.disc = None;
            st.result = None;
        }
        Cmd::Pick(slot) => {
            if !st.open {
                return Err("pick while closed");
            }
            let Some(band) = st.band else {
                return Err("pick before a band");
            };
            match def_by_slot(spells, slot) {
                Some(d) if sim::learnable(d, band) => {
                    st.pick = slot;
                    st.disc = None;
                    st.result = None;
                }
                _ => return Err("pick: not learnable at this band"),
            }
        }
        Cmd::Try(slot, text) => {
            if !st.open {
                return Err("try while closed");
            }
            let Some(band) = st.band else {
                return Err("try before a band");
            };
            let v = crate::spells::book().judge(spells, slot, band, &text);
            record(st, book, spells, slot, v);
        }
        Cmd::Learn(_) => return Err("learn: needs the discoveries (apply_disc)"),
    }
    Ok(())
}

/// The one Bench (I-058): practising a saved discovery. `learn <id>` picks
/// one the band builds (Seedling and up, its codex band or older); a `try`
/// while it is picked is judged by the same judge as a worked cast of it
/// (`judge_with` against `Disc::def`) and a correct one counts one build
/// (two builds, then it casts instantly). It never touches the shipped
/// spells' book or practice. `None`: not a discovery command (`apply`
/// handles it).
pub fn apply_disc(
    st: &mut BenchState,
    disc: &mut Disc,
    cmd: &Cmd,
) -> Option<Result<(), &'static str>> {
    match cmd {
        Cmd::Learn(id) => Some(learn(st, disc, id)),
        Cmd::Try(slot, text) if st.disc.is_some() => Some(try_disc(st, disc, *slot, text)),
        _ => None,
    }
}

fn learn(st: &mut BenchState, disc: &Disc, id: &str) -> Result<(), &'static str> {
    if !st.open {
        return Err("learn while closed");
    }
    let Some(band) = st.band else {
        return Err("learn before a band");
    };
    let Some(i) = crate::discover::index_of(id).filter(|i| disc.saved.iter().any(|s| s.0 == *i))
    else {
        return Err("learn: not a saved discovery");
    };
    if Disc::learnable(i, Some(band)) != Some(true) {
        return Err("learn: this band does not build it");
    }
    let Some(def) = Disc::def(i) else {
        return Err("learn: no spell for it");
    };
    st.disc = Some(i);
    st.pick = def.slot;
    st.result = None;
    Ok(())
}

fn try_disc(
    st: &mut BenchState,
    disc: &mut Disc,
    slot: u8,
    text: &str,
) -> Result<(), &'static str> {
    if !st.open {
        return Err("try while closed");
    }
    let (Some(band), Some(i)) = (st.band, st.disc) else {
        return Err("try before a band");
    };
    let Some(def) = Disc::def(i) else {
        return Err("try: no spell for it");
    };
    if slot != def.slot {
        return Err("try: not the picked discovery");
    }
    let v = crate::spells::book().judge(std::slice::from_ref(&def), def.slot, band, text);
    st.n += 1;
    if v.ok {
        disc.record_build(i);
    }
    st.result = Some(ResultView {
        n: st.n,
        slot,
        verdict: v,
        learned: false,
        stage: disc_stage(disc, i).to_string(),
    });
    Ok(())
}

/// A discovery's stage word: Mastered (casts instantly) once built
/// `MASTERY_BUILDS` times, else Learning.
fn disc_stage(disc: &Disc, i: usize) -> &'static str {
    if disc.stage(i) >= 3 {
        Stage::Mastered.as_str()
    } else {
        Stage::Learning.as_str()
    }
}

/// The view `__benchState` carries: closed, or the full open view.
pub fn view(st: &BenchState, book: &Book, spells: &[SpellDef], at: bool) -> BenchView {
    if !st.open {
        return BenchView::closed(at, st.band);
    }
    let mut v = BenchView::closed(at, st.band);
    v.open = true;
    if let Some(band) = st.band {
        v.spells = Some(
            spells
                .iter()
                .filter(|d| sim::learnable(d, band))
                .map(|d| SpellCard {
                    slot: d.slot,
                    id: d.id.clone(),
                    name: d.name.clone(),
                    formula: d.formula.text.clone(),
                    stage: stage_of(book, index_of(spells, d.slot))
                        .as_str()
                        .to_string(),
                    prompt: sim::prompt(d, band).unwrap_or_default().to_string(),
                })
                .collect(),
        );
        if st.pick != 0 && st.disc.is_none() {
            v.pick = Some(st.pick);
            if band < Tier::Apprentice {
                v.recipe = def_by_slot(spells, st.pick).map(sim::recipe);
            }
        }
    }
    v.result = st.result.clone();
    v
}

/// `__benchState`: the view, and while a discovery is picked (I-058) its
/// card after the shipped spells', its recipe, and `"disc": <codex id>`.
pub fn view_json(
    st: &BenchState,
    book: &Book,
    spells: &[SpellDef],
    at: bool,
    disc: &Disc,
) -> String {
    let mut v = view(st, book, spells, at);
    let picked = st
        .disc
        .filter(|_| st.open)
        .and_then(|i| Some((i, Disc::def(i)?, st.band?)));
    let Some((i, def, band)) = picked else {
        return v.to_json();
    };
    if let Some(cards) = v.spells.as_mut() {
        cards.push(SpellCard {
            slot: def.slot,
            id: def.id.clone(),
            name: def.name.clone(),
            formula: def.formula.text.clone(),
            stage: disc_stage(disc, i).to_string(),
            prompt: sim::prompt(&def, band).unwrap_or_default().to_string(),
        });
    }
    v.pick = Some(def.slot);
    v.recipe = (band < Tier::Apprentice).then(|| sim::recipe(&def));
    let mut json = v.to_json();
    json.pop();
    json.push_str(&format!(
        ",\"disc\":{}}}",
        realgame_chem::serde_json::to_string(&def.id).unwrap_or_else(|_| "null".into())
    ));
    json
}

/// `Startup`: inserts `BenchState` (so `main.rs` needs no `init_resource`)
/// and spawns the lab pad and its flask icon. Nothing is added to
/// `Colliders` or the chemistry targets: the player stands on the pad and
/// spells pass through the flask.
pub fn spawn_station(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(BenchState::default());

    let lab = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.72, 0.95),
        emissive: LinearRgba::rgb(0.05, 0.12, 0.2),
        perceptual_roughness: 0.6,
        ..default()
    });
    let rim = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.96, 0.98),
        perceptual_roughness: 0.7,
        ..default()
    });
    let (x, z) = STATION;
    // The white rim is a slightly wider, slightly lower disc under the pad.
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(RADIUS + 0.1, 0.03))),
        MeshMaterial3d(rim),
        Transform::from_xyz(x, 0.015, z),
        NotShadowCaster,
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(RADIUS, 0.04))),
        MeshMaterial3d(lab.clone()),
        Transform::from_xyz(x, 0.02, z),
        NotShadowCaster,
    ));
    // The flask icon: a bulb and a short neck, static (no animation).
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.25).mesh().ico(2).expect("ico"))),
        MeshMaterial3d(lab.clone()),
        Transform::from_xyz(x, FLASK_Y, z),
        NotShadowCaster,
        NotShadowReceiver,
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.08, 0.25))),
        MeshMaterial3d(lab),
        Transform::from_xyz(x, FLASK_Y + 0.32, z),
        NotShadowCaster,
        NotShadowReceiver,
    ));
}

/// `Update`, after `control` and before `publish`: drains the bench queue,
/// judges tries, learns on a first correct build and publishes the view
/// only when its JSON changed.
pub fn run(
    mut web: ResMut<WebIn>,
    mut bench: ResMut<BenchState>,
    mut progress: ResMut<Progress>,
    player: Res<Player>,
    voxel: Option<Res<crate::voxel_view::VoxWorld>>,
    mut working: Option<ResMut<crate::Working>>,
    mut disc: ResMut<Disc>,
) {
    let spells = crate::spells::book().all();
    for line in std::mem::take(&mut web.bench) {
        let before = bench.n;
        let mut shipped = true;
        let ignored = match parse(&line) {
            Some(cmd) => match apply_disc(&mut bench, &mut disc, &cmd) {
                Some(r) => {
                    shipped = false;
                    r
                }
                None => apply(&mut bench, &mut progress.0, spells, cmd),
            },
            None => Err("unknown or malformed command"),
        };
        if let Err(why) = ignored {
            warn!("bench: ignored {line:?}: {why}");
        }
        // Paawan's playtest (2026-11-04): every correct build at the Bench
        // ("Build another") counts toward Practised, like a worked cast. A
        // discovery's build counts toward its own mastery only.
        if let (true, Some(i), Some(w)) =
            (shipped, practice_index(&bench, before), working.as_mut())
        {
            w.cast_log.push((i, true));
        }
    }
    let at = if voxel.is_some() {
        at_voxel_bench(player.pos)
    } else {
        at_bench(player.pos)
    };
    let json = view_json(&bench, &progress.0, spells, at, &disc);
    if json != bench.last_json {
        bridge::publish_bench(&json);
        bench.last_json = json;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spells() -> &'static [SpellDef] {
        crate::spells::book().all()
    }

    #[test]
    fn parses_the_five_words() {
        assert_eq!(parse("band seedling"), Some(Cmd::Band(Tier::Seedling)));
        assert_eq!(parse("band adept"), Some(Cmd::Band(Tier::Adept)));
        assert_eq!(parse("open"), Some(Cmd::Open));
        assert_eq!(parse("close"), Some(Cmd::Close));
        assert_eq!(parse("pick 2"), Some(Cmd::Pick(2)));
        assert_eq!(
            parse("try 1  CH4 +  2O2 -> CO2 + 2H2O"),
            Some(Cmd::Try(1, "CH4 + 2O2 -> CO2 + 2H2O".into()))
        );
        assert_eq!(parse("try 2"), Some(Cmd::Try(2, String::new())));
        assert_eq!(parse("try 2 \"\\"), Some(Cmd::Try(2, "\"\\".into())));
    }

    #[test]
    fn malformed_commands_are_none() {
        for s in [
            "",
            "bench open",
            "band",
            "band Sprout",
            "band toddler",
            "band sprout x",
            "open now",
            "pick",
            "pick x",
            "pick 300",
            "pick 1 2",
            "try",
            "try x H2O",
            "Open",
        ] {
            assert_eq!(parse(s), None, "{s:?}");
        }
    }

    #[test]
    fn the_block_worlds_bench_lights_within_its_radius() {
        assert!(at_voxel_bench(Vec3::new(124.5, 35.0, 202.5)));
        assert!(at_voxel_bench(Vec3::new(126.0, 35.0, 201.5)));
        assert!(
            !at_voxel_bench(Vec3::new(128.5, 35.0, 205.5)),
            "spawn is not at the bench"
        );
    }

    #[test]
    fn at_bench_is_horizontal_distance_within_the_radius() {
        assert!(at_bench(Vec3::new(STATION.0, 0.0, STATION.1)));
        assert!(at_bench(Vec3::new(STATION.0, 5.0, STATION.1)));
        assert!(at_bench(Vec3::new(STATION.0 + 1.19, 0.0, STATION.1)));
        assert!(!at_bench(Vec3::new(STATION.0 + 1.21, 0.0, STATION.1)));
        assert!(!at_bench(Player::default().pos), "not at the start pose");
    }

    #[test]
    fn closed_view_carries_band_once_set_and_ignores_closed_commands() {
        let mut st = BenchState::default();
        let mut book = Book::default();
        let s = spells();
        assert_eq!(
            view(&st, &book, s, false).to_json(),
            "{\"v\":1,\"atBench\":false,\"open\":false}"
        );
        assert!(apply(&mut st, &mut book, s, Cmd::Pick(2)).is_err());
        assert!(apply(&mut st, &mut book, s, Cmd::Try(2, "H2O".into())).is_err());
        assert!(apply(&mut st, &mut book, s, Cmd::Close).is_err());
        assert_eq!(st.n, 0);
        apply(&mut st, &mut book, s, Cmd::Band(Tier::Seedling)).unwrap();
        assert_eq!(
            view(&st, &book, s, true).to_json(),
            "{\"v\":1,\"atBench\":true,\"open\":false,\"band\":\"seedling\"}"
        );
    }

    #[test]
    fn open_without_band_ignores_pick_and_try() {
        let mut st = BenchState::default();
        let mut book = Book::default();
        let s = spells();
        apply(&mut st, &mut book, s, Cmd::Open).unwrap();
        assert!(apply(&mut st, &mut book, s, Cmd::Pick(2)).is_err());
        assert!(apply(&mut st, &mut book, s, Cmd::Try(2, "H2O".into())).is_err());
        assert_eq!(
            view(&st, &book, s, false).to_json(),
            "{\"v\":1,\"atBench\":false,\"open\":true}"
        );
    }

    #[test]
    fn band_while_open_clears_pick_and_result_and_close_keeps_band() {
        let mut st = BenchState::default();
        let mut book = Book::default();
        let s = spells();
        apply(&mut st, &mut book, s, Cmd::Band(Tier::Apprentice)).unwrap();
        apply(&mut st, &mut book, s, Cmd::Open).unwrap();
        st.pick = 2;
        record(&mut st, &mut book, s, 2, Verdict::default());
        apply(&mut st, &mut book, s, Cmd::Band(Tier::Sprout)).unwrap();
        assert_eq!((st.pick, st.result.is_none()), (0, true));
        apply(&mut st, &mut book, s, Cmd::Close).unwrap();
        assert_eq!(st.band, Some(Tier::Sprout));
        assert!(!st.open);
    }

    #[test]
    fn a_correct_build_learns_once_and_repeats_earn_nothing() {
        let mut st = BenchState::default();
        let mut book = Book::default();
        let s = spells();
        let ok = Verdict {
            ok: true,
            code: sim::Code::Ok,
            ..Verdict::default()
        };
        // A wrong build changes nothing.
        record(&mut st, &mut book, s, 2, Verdict::default());
        let r = st.result.clone().unwrap();
        assert_eq!(
            (r.n, r.slot, r.learned, r.stage.as_str()),
            (1, 2, false, "unknown")
        );
        assert_eq!(book.get(1).unwrap().stage, Stage::Unknown);
        // The first correct build moves Unknown -> Learning (+10 XP).
        record(&mut st, &mut book, s, 2, ok.clone());
        let r = st.result.clone().unwrap();
        assert_eq!((r.n, r.learned, r.stage.as_str()), (2, true, "learning"));
        let xp = book.get(1).unwrap().xp;
        // Five more earn nothing here.
        for k in 0..5 {
            record(&mut st, &mut book, s, 2, ok.clone());
            let r = st.result.clone().unwrap();
            assert_eq!((r.n, r.learned), (3 + k, false));
            assert_eq!(book.get(1).unwrap().xp, xp);
        }
        // Other spells are untouched.
        assert_eq!(book.get(0).unwrap().stage, Stage::Unknown);
    }

    #[test]
    fn slots_outside_the_book_never_panic() {
        let mut st = BenchState::default();
        let mut book = Book::default();
        let s = spells();
        let ok = Verdict {
            ok: true,
            code: sim::Code::Ok,
            ..Verdict::default()
        };
        for slot in [0, 5, 255] {
            record(&mut st, &mut book, s, slot, ok.clone());
            let r = st.result.clone().unwrap();
            assert!(!r.learned);
            assert_eq!(r.stage, "unknown");
        }
    }

    #[test]
    fn open_view_carries_cards_with_formula_and_stage() {
        // Depends on the sim lane's `learnable`/`prompt`/`recipe`; on the
        // contract stub `learnable` is false, so the list is empty.
        let mut st = BenchState::default();
        let mut book = Book::default();
        let s = spells();
        apply(&mut st, &mut book, s, Cmd::Band(Tier::Seedling)).unwrap();
        apply(&mut st, &mut book, s, Cmd::Open).unwrap();
        let v = view(&st, &book, s, false);
        let cards = v.spells.expect("open with a band lists spells");
        for c in &cards {
            let d = def_by_slot(s, c.slot).unwrap();
            assert_eq!(c.formula, d.formula.text);
            assert_eq!(c.stage, "unknown");
        }
        assert!(v.pick.is_none() && v.recipe.is_none() && v.result.is_none());
    }

    #[test]
    fn learn_parses_a_codex_id_only() {
        assert_eq!(
            parse("learn carbon_burns"),
            Some(Cmd::Learn("carbon_burns".into()))
        );
        for bad in ["learn", "learn Carbon", "learn a b", "learn a-b"] {
            assert_eq!(parse(bad), None, "{bad:?}");
        }
    }

    /// I-058: the one Bench practises a saved discovery: picked with
    /// `learn`, judged against its own reaction, a correct build counts
    /// toward its two builds, and the shipped spells are never touched.
    #[test]
    fn the_bench_practises_a_saved_discovery_twice_then_it_is_instant() {
        let mut st = BenchState::default();
        let mut book = Book::default();
        let mut disc = Disc::default();
        let s = spells();
        let learn = Cmd::Learn("carbon_burns".into());
        // Closed, no band, or not saved: ignored.
        assert!(apply_disc(&mut st, &mut disc, &learn).unwrap().is_err());
        apply(&mut st, &mut book, s, Cmd::Band(Tier::Seedling)).unwrap();
        apply(&mut st, &mut book, s, Cmd::Open).unwrap();
        assert!(apply_disc(&mut st, &mut disc, &learn).unwrap().is_err());
        disc.command("saved carbon_burns:0");
        let i = crate::discover::index_of("carbon_burns").unwrap();
        // A shipped-spell command is not a discovery command.
        assert!(apply_disc(&mut st, &mut disc, &Cmd::Pick(2)).is_none());
        apply_disc(&mut st, &mut disc, &learn).unwrap().unwrap();
        assert_eq!(st.disc, Some(i));
        let slot = Disc::def(i).unwrap().slot;
        let json = view_json(&st, &book, s, false, &disc);
        assert!(json.contains("\"disc\":\"carbon_burns\""), "{json}");
        assert!(
            json.contains("\"recipe\""),
            "Seedling gets the recipe: {json}"
        );
        // A wrong build counts nothing; a try on another slot is refused.
        let wrong = Cmd::Try(slot, "C + O2 -> CO".into());
        apply_disc(&mut st, &mut disc, &wrong).unwrap().unwrap();
        assert_eq!(disc.builds(i), 0);
        assert!(!st.result.as_ref().unwrap().verdict.ok);
        let other = if slot == 1 { 2 } else { 1 };
        let off = Cmd::Try(other, "C + O2 -> CO2".into());
        assert!(apply_disc(&mut st, &mut disc, &off).unwrap().is_err());
        // Two correct builds: mastered (instant), with its own count only.
        let ok = Cmd::Try(slot, "C + O2 -> CO2".into());
        apply_disc(&mut st, &mut disc, &ok).unwrap().unwrap();
        assert_eq!(disc.builds(i), 1);
        assert_eq!(st.result.as_ref().unwrap().stage, "learning");
        apply_disc(&mut st, &mut disc, &ok).unwrap().unwrap();
        assert_eq!(disc.builds(i), 2);
        let r = st.result.clone().unwrap();
        assert!(r.verdict.ok && !r.learned);
        assert_eq!(r.stage, "mastered");
        assert_eq!(disc.stage(i), 3);
        for k in 0..4 {
            assert_eq!(book.get(k).unwrap().stage, Stage::Unknown, "spell {k}");
        }
        // Picking a shipped spell, or closing, drops the discovery.
        apply(&mut st, &mut book, s, Cmd::Pick(2)).unwrap();
        assert_eq!(st.disc, None);
        assert!(!view_json(&st, &book, s, false, &disc).contains("\"disc\""));
        apply_disc(&mut st, &mut disc, &learn).unwrap().unwrap();
        apply(&mut st, &mut book, s, Cmd::Close).unwrap();
        assert_eq!(st.disc, None);
    }

    #[test]
    fn a_sprout_never_builds_a_discovery_at_the_bench() {
        let mut st = BenchState::default();
        let mut book = Book::default();
        let mut disc = Disc::default();
        disc.command("saved carbon_burns:0");
        let s = spells();
        apply(&mut st, &mut book, s, Cmd::Band(Tier::Sprout)).unwrap();
        apply(&mut st, &mut book, s, Cmd::Open).unwrap();
        let learn = Cmd::Learn("carbon_burns".into());
        assert!(apply_disc(&mut st, &mut disc, &learn).unwrap().is_err());
        assert_eq!(st.disc, None);
    }
}
