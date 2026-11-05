//! Reaction show records (I-054, Bench plan phase B1).
//!
//! For every codex reaction, a record of what really happens on a lab
//! bench, built from measured data so the renderer computes the look rather
//! than inventing it:
//!
//! - **states** of every species at the reaction's conditions (s, l, g, aq);
//! - **colours** from the sourced substance colour table
//!   (`content/chem/substances.json`); a hex is a rendering of a sourced
//!   description, flagged `hex_estimated`;
//! - the **visible changes** (flame, bubbles, precipitate, colour change,
//!   glow, smoke, frost, shimmer, condensation);
//! - **ΔH** in kJ per mole of reaction as written, from CRC/NBS enthalpies of
//!   formation, with each term's source (or the heat class, `dh_estimated`);
//! - **speed**, flame and glow temperatures (adiabatic, flagged estimated),
//!   the emitter of a flame, gas densities relative to air, precipitate
//!   grain, what each product does at room temperature;
//! - a **geometry** (conformer id) per species (`content/chem/geometry.json`)
//!   and an **atom story** of bond break and form events, `illustrative`.
//!
//! [`frame`] turns a record and the geometry into the atoms, bonds, events
//! and effect at a time `t` in 0..1, for the Bench scene and the graphics
//! thread's G1/G2 per-frame type. See `docs/loop/items/I-054/spec.md`.

use std::collections::{BTreeMap, BTreeSet};

use realgame_sim::formula::{parse_equation, Equation};
use serde::{Deserialize, Serialize};

use crate::{Codex, Entry, Heat, Need};

pub const SUBSTANCES_JSON: &str = include_str!("../../../content/chem/substances.json");
pub const GEOMETRY_JSON: &str = include_str!("../../../content/chem/geometry.json");
pub const SHOW_JSON: &str = include_str!("../../../content/chem/show.json");

/// Molar mass of dry air, g/mol (CRC Handbook, "Composition of dry air").
pub const AIR_MOLAR_MASS: f64 = 28.96;
/// Room temperature for "what the product does afterwards", K.
pub const ROOM_K: f64 = 298.15;
/// Moles of N2 that come with each mole of O2 when a reaction runs in air.
pub const AIR_N2_PER_O2: f64 = 3.76;
/// The gas constant, J/(mol K).
const R: f64 = 8.314_462_618;
/// |ΔH| (kJ per mole of reaction) at which a spell reaches full strength.
pub const FULL_STRENGTH_KJ: f64 = 800.0;
/// A reaction whose |ΔH| is at most this many kJ per mole of reaction as
/// written counts as "little heat change".
pub const LITTLE_KJ: f64 = 25.0;

// ---------------------------------------------------------------------------
// Data files
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    S,
    L,
    G,
    Aq,
}

impl State {
    pub fn id(self) -> &'static str {
        match self {
            State::S => "s",
            State::L => "l",
            State::G => "g",
            State::Aq => "aq",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Colour {
    /// The colour as the source describes it ("pale yellow-green").
    pub described: String,
    /// An sRGB rendering of the description, when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hex: Option<String>,
    /// Always true when `hex` is present: the hex renders a description.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hex_estimated: bool,
    pub src: String,
}

impl Colour {
    pub fn colourless(&self) -> bool {
        self.described == "colourless"
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Dhf {
    /// kJ/mol at 298.15 K, 1 bar.
    pub v: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verified: Option<bool>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub estimated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub how: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ions: Option<BTreeMap<String, u32>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Grain {
    /// A fine or gelatinous suspension that settles slowly.
    Cloudy,
    /// Crystals or a metal deposit that settle fast.
    Grainy,
}

/// One heat-capacity segment: [Tmin K, Tmax K, A, B, C, D, E] with
/// Cp = A + B T + C T^2 + D T^3 + E / T^2 (J/(mol K)).
pub type CpSeg = [f64; 7];

#[derive(Clone, Debug, Deserialize)]
pub struct Substance {
    #[serde(default)]
    pub cas: Option<String>,
    pub name: String,
    pub molar_mass: f64,
    #[serde(default)]
    pub dhf: BTreeMap<State, Dhf>,
    #[serde(default)]
    pub mp_k: Option<f64>,
    #[serde(default)]
    pub bp_k: Option<f64>,
    #[serde(default)]
    pub density_kg_m3: Option<f64>,
    #[serde(default)]
    pub phys_src: Option<String>,
    #[serde(default)]
    pub hfus_kj: Option<f64>,
    #[serde(default)]
    pub hvap_kj: Option<f64>,
    #[serde(default)]
    pub cp: BTreeMap<State, Vec<CpSeg>>,
    #[serde(default)]
    pub cp_src: Option<String>,
    pub state: State,
    pub soluble: bool,
    #[serde(default)]
    pub colour: BTreeMap<State, Colour>,
    #[serde(default)]
    pub grain: Option<Grain>,
}

#[derive(Deserialize)]
struct SubstFile {
    sources: BTreeMap<String, String>,
    substances: BTreeMap<String, Substance>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Geometry {
    pub id: String,
    pub elements: Vec<String>,
    /// Ångströms, centred on the centroid.
    pub positions: Vec<[f64; 3]>,
    /// [i, j, order]; order 0 is an ionic contact.
    pub bonds: Vec<(usize, usize, f64)>,
    pub lattice: bool,
    pub src: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spacing_a: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
}

impl Geometry {
    /// Radius of the unit around its centroid, Å (at least half its lattice
    /// spacing).
    fn radius(&self) -> f64 {
        let r = self
            .positions
            .iter()
            .map(|p| (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt())
            .fold(0.0, f64::max);
        r.max(self.spacing_a.unwrap_or(0.0) / 2.0).max(0.6)
    }
}

#[derive(Deserialize)]
struct GeoFile {
    sources: BTreeMap<String, String>,
    species: BTreeMap<String, Geometry>,
}

/// The geometry of every codex species.
pub struct Geometries {
    pub species: BTreeMap<String, Geometry>,
    pub sources: BTreeMap<String, String>,
}

impl Geometries {
    pub fn load() -> Geometries {
        let f: GeoFile =
            serde_json::from_str(GEOMETRY_JSON).expect("content/chem/geometry.json is valid");
        Geometries {
            species: f.species,
            sources: f.sources,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Speed {
    Instant,
    Fast,
    Slow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visible {
    Flame,
    Glow,
    Bubbles,
    Precipitate,
    ColourChange,
    Smoke,
    Frost,
    Shimmer,
    Condensation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Medium {
    /// In water: dissolved species are aq.
    Water,
    /// In air: O2 comes with N2.
    Air,
    /// Neither: a dry mix, a melt, or pure gases.
    Dry,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Emitter {
    pub described: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hex: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lines_nm: Vec<f64>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub blackbody: bool,
    pub src: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fix {
    #[serde(default)]
    flame: Option<String>,
    #[serde(default)]
    glow: bool,
    #[serde(default)]
    add: Vec<Visible>,
    #[serde(default)]
    smoke: Vec<Colour>,
    #[serde(default)]
    states: BTreeMap<String, State>,
    #[serde(default)]
    speed: Option<(Speed, String)>,
    #[serde(default)]
    medium: Option<Medium>,
    #[serde(default)]
    note: Option<String>,
    /// Why ΔH and the codex heat class disagree, when they do.
    #[serde(default)]
    dh_note: Option<String>,
    #[allow(dead_code)]
    why: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ShowFile {
    #[allow(dead_code)]
    version: u32,
    #[allow(dead_code)]
    about: String,
    check: String,
    sources: BTreeMap<String, String>,
    aqueous_families: Vec<String>,
    aqueous_reactions: Vec<String>,
    speed_by_family: BTreeMap<String, (Speed, String)>,
    emitters: BTreeMap<String, Emitter>,
    reactions: BTreeMap<String, Fix>,
}

// ---------------------------------------------------------------------------
// The record
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Reactant,
    Product,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AfterRoom {
    Condenses,
    Freezes,
    Evaporates,
    None,
}

#[derive(Clone, Debug, Serialize)]
pub struct SpeciesShow {
    pub formula: String,
    pub coef: u32,
    pub side: Side,
    pub state: State,
    /// g/mol.
    pub molar_mass: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colour: Option<Colour>,
    /// The geometry id in `content/chem/geometry.json`.
    pub conformer: String,
    pub lattice: bool,
    /// What a product does once it is back at room temperature.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at_room: Option<AfterRoom>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DhTerm {
    pub formula: String,
    /// Signed: products positive, reactants negative.
    pub coef: i32,
    /// The phase whose ΔHf was used.
    pub state: State,
    pub dhf: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub estimated: bool,
    /// False when entered from a printed table, not checked against a
    /// machine-readable copy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Enthalpy {
    /// kJ per mole of reaction as written; `None` when a species has no
    /// value (then `heat` falls back to the codex heat class).
    pub kj: Option<f64>,
    /// True when `kj` is missing or any term is estimated.
    pub dh_estimated: bool,
    /// -1 takes in heat, 0 little, +1 gives out heat.
    pub heat: i8,
    pub terms: Vec<DhTerm>,
    /// Species whose ΔHf at the record's state was missing, so the standard
    /// state's value was used.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub state_fallbacks: Vec<String>,
    /// Whether the sign agrees with the codex heat class (None: the codex
    /// gives no class).
    pub agrees: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Temperature {
    pub k: f64,
    pub estimated: bool,
    pub basis: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Flame {
    pub emitter: String,
    #[serde(flatten)]
    pub light: Emitter,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temp: Option<Temperature>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Glow {
    /// Incandescence: the renderer colours it as a blackbody at `temp`.
    pub blackbody: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temp: Option<Temperature>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Opacity {
    /// A colourless gas: only a shimmer shows it.
    Clear,
    /// A coloured gas: see-through, tinted.
    Tinted,
    /// Smoke: solid particles that hide what is behind.
    Dense,
}

#[derive(Clone, Debug, Serialize)]
pub struct GasShow {
    pub formula: String,
    pub side: Side,
    /// Molar mass / 28.96: below 1 rises in air, above 1 sinks.
    pub density_rel_air: f64,
    pub rises: bool,
    pub opacity: Opacity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colour: Option<Colour>,
    /// What it becomes on cool glass (steam: a white mist of droplets).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condenses_to: Option<Colour>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SmokeShow {
    /// The solid the smoke is made of (absent for soot).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formula: Option<String>,
    pub opacity: Opacity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colour: Option<Colour>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SolidShow {
    pub formula: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colour: Option<Colour>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grain: Option<Grain>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ColourChange {
    pub formula: String,
    pub state: State,
    /// true: this colour appears (a product); false: it goes (a reactant).
    pub appears: bool,
    pub colour: Colour,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EventKind {
    Break,
    Form,
}

/// One atom of the atom story: where it starts (reactant term, unit, atom)
/// and where it ends (product term, unit, atom), indices into the equation's
/// terms, the coefficient copy and the geometry's atoms.
#[derive(Clone, Debug, Serialize)]
pub struct StoryAtom {
    pub el: String,
    pub from: [usize; 3],
    pub to: [usize; 3],
}

#[derive(Clone, Debug, Serialize)]
pub struct StoryEvent {
    pub kind: EventKind,
    /// Story atom indices.
    pub a: usize,
    pub b: usize,
    pub order: f64,
    /// Start and duration, as fractions of the whole show (0..1).
    pub start: f64,
    pub duration: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct AtomStory {
    /// Always true: the order and timing are a picture of which bonds change,
    /// not a computed mechanism.
    pub illustrative: bool,
    pub atoms: Vec<StoryAtom>,
    pub events: Vec<StoryEvent>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ShowRecord {
    pub reaction_id: String,
    pub medium: Medium,
    pub species: Vec<SpeciesShow>,
    pub visible: Vec<Visible>,
    /// What there is to see in words, when the visible list is short.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub dh: Enthalpy,
    pub speed: Speed,
    /// Slow reactions are shown sped up, with a label saying so.
    pub sped_up: bool,
    /// How long it really takes ("days", "seconds").
    pub timescale: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flame: Option<Flame>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub glow: Option<Glow>,
    pub gases: Vec<GasShow>,
    pub smoke: Vec<SmokeShow>,
    pub precipitates: Vec<SolidShow>,
    pub colour_changes: Vec<ColourChange>,
    pub atom_story: AtomStory,
    pub check: String,
}

impl ShowRecord {
    pub fn has(&self, v: Visible) -> bool {
        self.visible.contains(&v)
    }

    /// The colour a spell shows: the flame's for a fire, otherwise the first
    /// product with a rendered colour (a precipitate, coloured gas or
    /// solution).
    pub fn tint(&self, fire: bool) -> Option<String> {
        if fire {
            return self.flame.as_ref().and_then(|f| f.light.hex.clone());
        }
        self.species
            .iter()
            .filter(|s| s.side == Side::Product && s.formula != "H2O")
            .find_map(|s| {
                s.colour
                    .as_ref()
                    .filter(|c| !c.colourless())
                    .and_then(|c| c.hex.clone())
            })
    }
}

/// Spell strength from |ΔH| (documented scale, I-054 spec section 6):
/// `ln(1 + |ΔH|) / ln(1 + 800)`, clamped to 0.1..1, so a burning (several
/// hundred kJ) is a full-strength spell and a few kJ is a faint one. A
/// reaction that takes in heat only while it is given heat, light or
/// electricity chills little, so it counts a quarter.
pub fn strength_of(dh_kj: f64, driven_cold: bool) -> f32 {
    let s = ((1.0 + dh_kj.abs()).ln() / (1.0 + FULL_STRENGTH_KJ).ln()).clamp(0.1, 1.0);
    let s = if driven_cold { s * 0.25 } else { s };
    ((s * 100.0).round() / 100.0).max(0.05) as f32
}

// ---------------------------------------------------------------------------
// Building the records
// ---------------------------------------------------------------------------

/// Every codex reaction's show record, with the data it was built from.
pub struct Shows {
    pub records: Vec<ShowRecord>,
    pub substances: BTreeMap<String, Substance>,
    pub geometry: Geometries,
    /// Every source key used by substances.json, show.json and geometry.json.
    pub sources: BTreeMap<String, String>,
    by_id: BTreeMap<String, usize>,
}

impl Shows {
    pub fn load() -> Shows {
        Shows::build(&Codex::load())
    }

    pub fn build(codex: &Codex) -> Shows {
        let subs: SubstFile =
            serde_json::from_str(SUBSTANCES_JSON).expect("content/chem/substances.json is valid");
        let fixes: ShowFile =
            serde_json::from_str(SHOW_JSON).expect("content/chem/show.json is valid");
        let geometry = Geometries::load();
        let mut sources = subs.sources.clone();
        sources.extend(fixes.sources.clone());
        sources.extend(geometry.sources.clone());
        let none = Fix::default();
        let records: Vec<ShowRecord> = codex
            .entries
            .iter()
            .map(|e| {
                let fix = fixes.reactions.get(&e.id).unwrap_or(&none);
                build_record(e, fix, &fixes, &subs.substances, &geometry)
            })
            .collect();
        let by_id = records
            .iter()
            .enumerate()
            .map(|(i, r)| (r.reaction_id.clone(), i))
            .collect();
        Shows {
            records,
            substances: subs.substances,
            geometry,
            sources,
            by_id,
        }
    }

    pub fn get(&self, id: &str) -> Option<&ShowRecord> {
        self.by_id.get(id).map(|&i| &self.records[i])
    }

    /// The reasons show.json gives for a ΔH sign that disagrees with the
    /// codex heat class, by reaction id.
    pub fn dh_notes(&self) -> BTreeMap<String, String> {
        self.records
            .iter()
            .filter_map(|r| r.dh.note.clone().map(|n| (r.reaction_id.clone(), n)))
            .collect()
    }
}

fn substance<'a>(subs: &'a BTreeMap<String, Substance>, f: &str) -> &'a Substance {
    subs.get(f)
        .unwrap_or_else(|| panic!("substances.json has no {f}"))
}

fn is_hot(e: &Entry, fix: &Fix) -> bool {
    e.family == "combustion"
        || e.needs.contains(&Need::Heat)
        || e.needs.contains(&Need::Spark)
        || fix.flame.is_some()
        || fix.glow
}

fn state_of(f: &str, side: Side, sub: &Substance, medium: Medium, hot: bool, fix: &Fix) -> State {
    if let Some(&s) = fix.states.get(f) {
        return s;
    }
    let mut s = sub.state;
    if medium == Medium::Water && f != "H2O" {
        let has_aq = sub.dhf.contains_key(&State::Aq);
        s = match (sub.state, side) {
            (State::G, Side::Product) => State::G,
            (_, _) if sub.soluble && has_aq => State::Aq,
            (st, _) => st,
        };
    }
    if hot && side == Side::Product && s == State::L && sub.bp_k.is_some_and(|b| b <= 400.0) {
        s = State::G;
    }
    s
}

fn after_room(state: State, sub: &Substance) -> AfterRoom {
    let mp = sub.mp_k.unwrap_or(0.0);
    match state {
        State::G if sub.bp_k.is_some_and(|b| b > ROOM_K) || mp > ROOM_K => {
            if mp > ROOM_K {
                AfterRoom::Freezes
            } else {
                AfterRoom::Condenses
            }
        }
        State::L if mp > ROOM_K => AfterRoom::Freezes,
        State::L if sub.bp_k.is_some_and(|b| b < 360.0) => AfterRoom::Evaporates,
        _ => AfterRoom::None,
    }
}

/// ΔHf of `sub` in `state`, or in its standard state when that phase has no
/// value (the bool says a fallback was used).
fn dhf_for(sub: &Substance, state: State) -> Option<(State, &Dhf, bool)> {
    if let Some(d) = sub.dhf.get(&state) {
        return Some((state, d, false));
    }
    let order = [sub.state, State::S, State::L, State::G, State::Aq];
    order
        .iter()
        .find_map(|s| sub.dhf.get(s).map(|d| (*s, d, true)))
}

fn heat_class(h: Option<Heat>) -> i8 {
    match h {
        Some(Heat::Releases) => 1,
        Some(Heat::Absorbs) => -1,
        _ => 0,
    }
}

fn enthalpy(
    e: &Entry,
    eq: &Equation,
    species: &[SpeciesShow],
    subs: &BTreeMap<String, Substance>,
    fix: &Fix,
) -> Enthalpy {
    let mut terms = Vec::new();
    let mut fallbacks = Vec::new();
    let mut total = 0.0;
    let mut missing = false;
    for s in species {
        let sub = substance(subs, &s.formula);
        let sign = if s.side == Side::Product { 1 } else { -1 };
        match dhf_for(sub, s.state) {
            Some((st, d, fb)) => {
                if fb {
                    fallbacks.push(format!("{}({})", s.formula, s.state.id()));
                }
                total += sign as f64 * s.coef as f64 * d.v;
                terms.push(DhTerm {
                    formula: s.formula.clone(),
                    coef: sign * s.coef as i32,
                    state: st,
                    dhf: d.v,
                    src: d.src.clone(),
                    estimated: d.estimated,
                    verified: d.verified,
                });
            }
            None => missing = true,
        }
    }
    let _ = eq;
    let kj = (!missing).then(|| (total * 10.0).round() / 10.0);
    let class = heat_class(e.heat);
    let heat = match kj {
        Some(v) if v.abs() <= LITTLE_KJ => 0,
        Some(v) if v < 0.0 => 1,
        Some(_) => -1,
        None => class,
    };
    let agrees = match (e.heat, kj) {
        (None, _) | (_, None) => None,
        (Some(Heat::Little), Some(v)) => Some(v.abs() <= LITTLE_KJ),
        (Some(Heat::Releases), Some(v)) => Some(v < 0.0),
        (Some(Heat::Absorbs), Some(v)) => Some(v > 0.0),
    };
    Enthalpy {
        kj,
        dh_estimated: missing || terms.iter().any(|t| t.estimated),
        heat,
        terms,
        state_fallbacks: fallbacks,
        agrees,
        note: fix.dh_note.clone(),
    }
}

/// H(T) - H(298.15 K) of one mole of `sub`, starting in `phase` at 298 K,
/// J/mol. Condensed phases melt at the melting point (adding the heat of
/// fusion); the caller caps T at the boiling point. Returns whether the
/// heat capacity was estimated (no NIST data: 3R per atom for a condensed
/// phase, (3n - 1.5)R for an n-atom gas).
fn rise(sub: &Substance, atoms: u32, phase: State, t: f64) -> (f64, bool) {
    let n = atoms.max(1) as f64;
    let integrate = |segs: &[CpSeg], a: f64, b: f64| -> f64 {
        let h = |s: &CpSeg, x: f64| {
            s[2] * x + s[3] * x * x / 2.0 + s[4] * x.powi(3) / 3.0 + s[5] * x.powi(4) / 4.0
                - s[6] / x
        };
        let mut sum = 0.0;
        let (mut x, mut i) = (a, 0);
        while x < b - 1e-9 {
            // The segment covering x (the first one below its range, the
            // last one past the end).
            while i + 1 < segs.len() && x >= segs[i][1] {
                i += 1;
            }
            let end = if i + 1 < segs.len() {
                segs[i][1].min(b)
            } else {
                b
            };
            sum += h(&segs[i], end) - h(&segs[i], x);
            x = end;
        }
        sum
    };
    let gas_cp = if n <= 1.0 {
        2.5 * R
    } else {
        (3.0 * n - 1.5) * R
    };
    match phase {
        State::G | State::Aq => match sub.cp.get(&State::G) {
            Some(segs) => (integrate(segs, ROOM_K, t), false),
            None => (gas_cp * (t - ROOM_K), true),
        },
        State::S | State::L => {
            let mut est = false;
            let solid = |a: f64, b: f64, est: &mut bool| match sub.cp.get(&State::S) {
                Some(s) => integrate(s, a, b),
                None => match sub.cp.get(&State::L) {
                    Some(l) => integrate(l, a, b),
                    None => {
                        *est = true;
                        3.0 * R * n * (b - a)
                    }
                },
            };
            let liquid = |a: f64, b: f64, est: &mut bool| match sub.cp.get(&State::L) {
                Some(l) => integrate(l, a, b),
                None => match sub.cp.get(&State::S) {
                    Some(s) => integrate(s, a, b),
                    None => {
                        *est = true;
                        3.0 * R * n * (b - a)
                    }
                },
            };
            let mp = sub.mp_k.unwrap_or(f64::INFINITY);
            if phase == State::L || mp <= ROOM_K {
                return (liquid(ROOM_K, t, &mut est), est);
            }
            if t <= mp {
                return (solid(ROOM_K, t, &mut est), est);
            }
            let fus = match sub.hfus_kj {
                Some(h) => h * 1000.0,
                None => {
                    est = true;
                    0.0
                }
            };
            let h = solid(ROOM_K, mp, &mut est) + fus + liquid(mp, t, &mut est);
            (h, est)
        }
    }
}

/// Adiabatic temperature of the products, K: the heat the reaction gives
/// out (−ΔH) warms its own products (plus the nitrogen of the air, when it
/// burns in air) with no heat lost and no dissociation, so it is an upper
/// bound. Capped at the lowest boiling point of a condensed product.
fn adiabatic(
    dh: &Enthalpy,
    species: &[SpeciesShow],
    subs: &BTreeMap<String, Substance>,
    in_air: bool,
) -> Option<Temperature> {
    let q = -dh.kj? * 1000.0;
    if q <= 0.0 {
        return None;
    }
    let o2: u32 = species
        .iter()
        .filter(|s| s.side == Side::Reactant && s.formula == "O2")
        .map(|s| s.coef)
        .sum();
    let n2 = if in_air {
        AIR_N2_PER_O2 * o2 as f64
    } else {
        0.0
    };
    let n2_sub = substance(subs, "N2");
    // Products in the phase their ΔHf was taken in.
    let prods: Vec<(&Substance, f64, State, u32, &str)> = dh
        .terms
        .iter()
        .filter(|t| t.coef > 0)
        .map(|t| {
            let sub = substance(subs, &t.formula);
            let atoms = realgame_sim::formula::parse_formula(&t.formula)
                .map(|a| a.values().sum())
                .unwrap_or(1);
            (sub, t.coef as f64, t.state, atoms, t.formula.as_str())
        })
        .collect();
    let mut cap = 6000.0_f64;
    let mut capped_by = None;
    for (sub, _, st, _, f) in &prods {
        if matches!(st, State::S | State::L) {
            if let Some(bp) = sub.bp_k {
                if bp < cap {
                    cap = bp;
                    capped_by = Some(*f);
                }
            }
        }
    }
    let mut estimated_cp = BTreeSet::new();
    let total = |t: f64, est: &mut BTreeSet<String>| {
        let mut h = 0.0;
        for (sub, n, st, atoms, f) in &prods {
            let (r, e) = rise(sub, *atoms, *st, t);
            if e {
                est.insert(f.to_string());
            }
            h += n * r;
        }
        h + n2 * rise(n2_sub, 2, State::G, t).0
    };
    let (mut lo, mut hi) = (ROOM_K, cap);
    let capped = total(cap, &mut estimated_cp) < q;
    let k = if capped {
        cap
    } else {
        for _ in 0..60 {
            let mid = (lo + hi) / 2.0;
            if total(mid, &mut estimated_cp) < q {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        (lo + hi) / 2.0
    };
    let mut basis = format!(
        "adiabatic: stoichiometric {}, no heat lost and no dissociation, so an upper bound (heat capacities: NIST Shomate)",
        if in_air && o2 > 0 {
            "in air (O2 + 3.76 N2)"
        } else {
            "in the reactants alone"
        }
    );
    if capped {
        if let Some(f) = capped_by {
            basis.push_str(&format!("; capped at the boiling point of {f}"));
        }
    }
    if !estimated_cp.is_empty() {
        let list: Vec<String> = estimated_cp.into_iter().collect();
        basis.push_str(&format!(
            "; heat capacity estimated (3R per atom) for {}",
            list.join(", ")
        ));
    }
    Some(Temperature {
        k: (k / 10.0).round() * 10.0,
        estimated: true,
        basis,
    })
}

fn build_record(
    e: &Entry,
    fix: &Fix,
    file: &ShowFile,
    subs: &BTreeMap<String, Substance>,
    geo: &Geometries,
) -> ShowRecord {
    let eq = parse_equation(&e.equation).expect("codex equations parse");
    let medium = fix.medium.unwrap_or_else(|| {
        if file.aqueous_families.contains(&e.family) || file.aqueous_reactions.contains(&e.id) {
            Medium::Water
        } else if eq.reactants.iter().any(|t| t.formula == "O2") {
            Medium::Air
        } else {
            Medium::Dry
        }
    });
    let hot = is_hot(e, fix);
    let mut species = Vec::new();
    for (side, terms) in [
        (Side::Reactant, &eq.reactants),
        (Side::Product, &eq.products),
    ] {
        for t in terms.iter() {
            let sub = substance(subs, &t.formula);
            let state = state_of(&t.formula, side, sub, medium, hot, fix);
            let g = geo
                .species
                .get(&t.formula)
                .unwrap_or_else(|| panic!("geometry.json has no {}", t.formula));
            species.push(SpeciesShow {
                formula: t.formula.clone(),
                coef: t.coef,
                side,
                state,
                molar_mass: sub.molar_mass,
                colour: sub.colour.get(&state).cloned(),
                conformer: g.id.clone(),
                lattice: g.lattice,
                at_room: (side == Side::Product).then(|| after_room(state, sub)),
            });
        }
    }
    let dh = enthalpy(e, &eq, &species, subs, fix);
    let in_air = medium == Medium::Air;
    let temp = || adiabatic(&dh, &species, subs, in_air);
    let flame = fix.flame.as_ref().map(|k| Flame {
        emitter: k.clone(),
        light: file
            .emitters
            .get(k)
            .unwrap_or_else(|| panic!("show.json has no emitter {k}"))
            .clone(),
        temp: temp(),
    });
    let glow = fix.glow.then(|| Glow {
        blackbody: true,
        temp: temp(),
    });
    let products = || species.iter().filter(|s| s.side == Side::Product);
    let reactants = || species.iter().filter(|s| s.side == Side::Reactant);
    let mut visible = BTreeSet::new();
    if flame.is_some() {
        visible.insert(Visible::Flame);
    }
    if glow.is_some() {
        visible.insert(Visible::Glow);
    }
    let in_water =
        medium == Medium::Water || reactants().any(|s| matches!(s.state, State::Aq | State::L));
    if in_water && products().any(|s| s.state == State::G) {
        visible.insert(Visible::Bubbles);
    }
    // In water, a product that stays solid comes out of the solution (the
    // soluble ones are aq by the state rules).
    let precipitates: Vec<SolidShow> = if medium == Medium::Water {
        products()
            .filter(|s| s.state == State::S)
            .map(|s| SolidShow {
                formula: s.formula.clone(),
                colour: s.colour.clone(),
                grain: substance(subs, &s.formula).grain,
            })
            .collect()
    } else {
        Vec::new()
    };
    if !precipitates.is_empty() {
        visible.insert(Visible::Precipitate);
    }
    // Colour changes: a coloured species that is there before and not after
    // (or the other way round).
    let coloured = |s: &SpeciesShow| s.colour.as_ref().filter(|c| !c.colourless()).cloned();
    let before: BTreeSet<String> = reactants()
        .filter_map(coloured)
        .map(|c| c.described)
        .collect();
    let after: BTreeSet<String> = products()
        .filter_map(coloured)
        .map(|c| c.described)
        .collect();
    let mut colour_changes = Vec::new();
    for s in &species {
        if let Some(c) = coloured(s) {
            let other = if s.side == Side::Reactant {
                &after
            } else {
                &before
            };
            if !other.contains(&c.described) {
                colour_changes.push(ColourChange {
                    formula: s.formula.clone(),
                    state: s.state,
                    appears: s.side == Side::Product,
                    colour: c,
                });
            }
        }
    }
    if !colour_changes.is_empty() {
        visible.insert(Visible::ColourChange);
    }
    let solid_product = products().any(|s| s.state == State::S);
    if (flame.is_some() && solid_product)
        || (medium != Medium::Water && solid_product && reactants().all(|s| s.state == State::G))
    {
        visible.insert(Visible::Smoke);
    }
    if flame.is_some() || glow.is_some() {
        visible.insert(Visible::Shimmer);
    }
    if hot && products().any(|s| s.state == State::G && s.at_room == Some(AfterRoom::Condenses)) {
        visible.insert(Visible::Condensation);
    }
    visible.extend(fix.add.iter().copied());
    let mut smoke: Vec<SmokeShow> = fix
        .smoke
        .iter()
        .map(|c| SmokeShow {
            formula: None,
            opacity: Opacity::Dense,
            colour: Some(c.clone()),
        })
        .collect();
    if visible.contains(&Visible::Smoke) {
        for s in products().filter(|s| {
            s.state == State::S || (s.state == State::G && s.at_room == Some(AfterRoom::Freezes))
        }) {
            let sub = substance(subs, &s.formula);
            smoke.push(SmokeShow {
                formula: Some(s.formula.clone()),
                opacity: Opacity::Dense,
                colour: sub.colour.get(&State::S).or(s.colour.as_ref()).cloned(),
            });
        }
        if e.id == "so3_water" {
            // The fog is droplets of the acid made.
            smoke.push(SmokeShow {
                formula: Some("H2SO4".into()),
                opacity: Opacity::Dense,
                colour: Some(mist()),
            });
        }
    }
    let gases = species
        .iter()
        .filter(|s| s.state == State::G)
        .map(|s| {
            let d = ((s.molar_mass / AIR_MOLAR_MASS) * 1000.0).round() / 1000.0;
            let tinted = s.colour.as_ref().is_some_and(|c| !c.colourless());
            GasShow {
                formula: s.formula.clone(),
                side: s.side,
                density_rel_air: d,
                rises: d < 1.0,
                opacity: if tinted {
                    Opacity::Tinted
                } else {
                    Opacity::Clear
                },
                colour: s.colour.clone(),
                condenses_to: (s.formula == "H2O" && s.side == Side::Product).then(mist),
            }
        })
        .collect();
    let (speed, timescale) = fix
        .speed
        .clone()
        .or_else(|| file.speed_by_family.get(&e.family).cloned())
        .unwrap_or((Speed::Fast, "seconds".into()));
    let atom_story = story(&eq, geo);
    ShowRecord {
        reaction_id: e.id.clone(),
        medium,
        species,
        visible: visible.into_iter().collect(),
        note: fix.note.clone(),
        dh,
        speed,
        sped_up: speed == Speed::Slow,
        timescale,
        flame,
        glow,
        gases,
        smoke,
        precipitates,
        colour_changes,
        atom_story,
        check: file.check.clone(),
    }
}

fn mist() -> Colour {
    Colour {
        described: "white mist of droplets".into(),
        hex: Some("#f4f4f4".into()),
        hex_estimated: true,
        src: "MIST".into(),
    }
}

// ---------------------------------------------------------------------------
// The atom story
// ---------------------------------------------------------------------------

struct Unit<'a> {
    term: usize,
    copy: usize,
    geo: &'a Geometry,
}

fn units<'a>(terms: &[realgame_sim::formula::Term], geo: &'a Geometries) -> Vec<Unit<'a>> {
    let mut out = Vec::new();
    for (i, t) in terms.iter().enumerate() {
        let g = &geo.species[&t.formula];
        for c in 0..t.coef as usize {
            out.push(Unit {
                term: i,
                copy: c,
                geo: g,
            });
        }
    }
    out
}

/// Flattened atoms of one side: (element, [term, copy, atom]) plus bonds as
/// (flat i, flat j, order).
type Flat = (Vec<(String, [usize; 3])>, Vec<(usize, usize, f64)>);

fn flatten(us: &[Unit]) -> Flat {
    let mut atoms = Vec::new();
    let mut bonds = Vec::new();
    for u in us {
        let off = atoms.len();
        for (k, el) in u.geo.elements.iter().enumerate() {
            atoms.push((el.clone(), [u.term, u.copy, k]));
        }
        for &(a, b, o) in &u.geo.bonds {
            bonds.push((off + a, off + b, o));
        }
    }
    (atoms, bonds)
}

fn neighbours(n: usize, bonds: &[(usize, usize, f64)]) -> Vec<Vec<usize>> {
    let mut adj = vec![Vec::new(); n];
    for &(a, b, _) in bonds {
        adj[a].push(b);
        adj[b].push(a);
    }
    adj
}

/// Maps every product atom to a reactant atom of the same element, greedily
/// keeping bonds that survive: a product atom next to an already mapped one
/// takes, when it can, a reactant atom bonded to that one's match.
fn map_atoms(r: &Flat, p: &Flat) -> Vec<usize> {
    let (ra, rb) = r;
    let (pa, pb) = p;
    let radj = neighbours(ra.len(), rb);
    let padj = neighbours(pa.len(), pb);
    let mut map: Vec<Option<usize>> = vec![None; pa.len()];
    let mut used = vec![false; ra.len()];
    let heavy = |el: &str| el != "H";
    let sig = |adj: &[Vec<usize>], atoms: &[(String, [usize; 3])], i: usize| {
        let mut v: Vec<&str> = adj[i].iter().map(|&j| atoms[j].0.as_str()).collect();
        v.sort_unstable();
        v.join(",")
    };
    loop {
        // A frontier atom: unmapped, next to a mapped product atom.
        let frontier =
            (0..pa.len()).find(|&i| map[i].is_none() && padj[i].iter().any(|&j| map[j].is_some()));
        let (pi, cand) = match frontier {
            Some(pi) => {
                let anchors: Vec<usize> = padj[pi].iter().filter_map(|&j| map[j]).collect();
                let near = (0..ra.len()).find(|&ri| {
                    !used[ri]
                        && ra[ri].0 == pa[pi].0
                        && anchors.iter().any(|&a| radj[a].contains(&ri))
                });
                let same_unit = || {
                    (0..ra.len()).find(|&ri| {
                        !used[ri]
                            && ra[ri].0 == pa[pi].0
                            && anchors
                                .iter()
                                .any(|&a| ra[a].1[0] == ra[ri].1[0] && ra[a].1[1] == ra[ri].1[1])
                    })
                };
                (pi, near.or_else(same_unit))
            }
            None => {
                // A new seed: the first unmapped heavy atom (else any).
                let Some(pi) = (0..pa.len())
                    .find(|&i| map[i].is_none() && heavy(&pa[i].0))
                    .or_else(|| (0..pa.len()).find(|&i| map[i].is_none()))
                else {
                    break;
                };
                let want = sig(&padj, pa, pi);
                let best = (0..ra.len())
                    .filter(|&ri| !used[ri] && ra[ri].0 == pa[pi].0)
                    .max_by_key(|&ri| (sig(&radj, ra, ri) == want, std::cmp::Reverse(ri)));
                (pi, best)
            }
        };
        let ri = cand.or_else(|| (0..ra.len()).find(|&ri| !used[ri] && ra[ri].0 == pa[pi].0));
        let ri = ri.expect("both sides hold the same atoms");
        used[ri] = true;
        map[pi] = Some(ri);
    }
    map.into_iter()
        .map(|m| m.expect("every product atom is mapped"))
        .collect()
}

fn story(eq: &Equation, geo: &Geometries) -> AtomStory {
    let r = flatten(&units(&eq.reactants, geo));
    let p = flatten(&units(&eq.products, geo));
    let map = map_atoms(&r, &p);
    // Story atoms are the reactant atoms, in order.
    let mut to = vec![[0usize; 3]; r.0.len()];
    let mut inv = vec![0usize; r.0.len()];
    for (pi, &ri) in map.iter().enumerate() {
        to[ri] = p.0[pi].1;
        inv[ri] = pi;
    }
    let atoms: Vec<StoryAtom> =
        r.0.iter()
            .enumerate()
            .map(|(i, (el, from))| StoryAtom {
                el: el.clone(),
                from: *from,
                to: to[i],
            })
            .collect();
    let key = |a: usize, b: usize| (a.min(b), a.max(b));
    let rset: BTreeSet<(usize, usize)> = r.1.iter().map(|&(a, b, _)| key(a, b)).collect();
    let pset: BTreeSet<(usize, usize)> = p.1.iter().map(|&(a, b, _)| key(map[a], map[b])).collect();
    let breaks: Vec<(usize, usize, f64)> =
        r.1.iter()
            .filter(|&&(a, b, _)| !pset.contains(&key(a, b)))
            .copied()
            .collect();
    let forms: Vec<(usize, usize, f64)> =
        p.1.iter()
            .map(|&(a, b, o)| (map[a], map[b], o))
            .filter(|&(a, b, _)| !rset.contains(&key(a, b)))
            .collect();
    let _ = inv;
    let spread = |n: usize, i: usize, from: f64, span: f64| {
        if n <= 1 {
            from + span / 2.0
        } else {
            from + span * i as f64 / (n - 1) as f64
        }
    };
    let round = |x: f64| (x * 1000.0).round() / 1000.0;
    let mut events = Vec::new();
    for (i, &(a, b, o)) in breaks.iter().enumerate() {
        events.push(StoryEvent {
            kind: EventKind::Break,
            a,
            b,
            order: o,
            start: round(spread(breaks.len(), i, 0.10, 0.30)),
            duration: 0.15,
        });
    }
    for (i, &(a, b, o)) in forms.iter().enumerate() {
        events.push(StoryEvent {
            kind: EventKind::Form,
            a,
            b,
            order: o,
            start: round(spread(forms.len(), i, 0.50, 0.30)),
            duration: 0.15,
        });
    }
    AtomStory {
        illustrative: true,
        atoms,
        events,
    }
}

// ---------------------------------------------------------------------------
// Frames
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize)]
pub struct FrameAtom {
    pub el: String,
    /// Ångströms.
    pub pos: [f32; 3],
}

#[derive(Clone, Debug, Serialize)]
pub struct FrameBond {
    pub a: usize,
    pub b: usize,
    pub order: f32,
    /// 1 at rest; above 1 while a bond stretches before it breaks or while a
    /// new bond pulls in.
    pub stretch: f32,
}

#[derive(Clone, Debug, Serialize)]
pub struct FrameEvent {
    pub kind: EventKind,
    pub a: usize,
    pub b: usize,
    /// 0..1 through the event.
    pub progress: f32,
}

#[derive(Clone, Debug, Serialize)]
pub struct FrameEffect {
    pub reaction_id: String,
    pub kind: crate::discover::EffectKind,
    pub heat: i8,
    pub strength: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tint: Option<String>,
    /// Each visible change with its intensity at this time (0..1).
    pub visible: Vec<(Visible, f32)>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Frame {
    pub t: f32,
    pub atoms: Vec<FrameAtom>,
    pub bonds: Vec<FrameBond>,
    /// Events under way at `t`.
    pub events: Vec<FrameEvent>,
    pub effect: FrameEffect,
}

/// Places each unit of a side in a row along x, centred on the origin, and
/// returns every flattened atom's position.
fn layout(terms: &[realgame_sim::formula::Term], geo: &Geometries) -> Vec<[f64; 3]> {
    let us = units(terms, geo);
    let gap = 1.2;
    let widths: Vec<f64> = us.iter().map(|u| 2.0 * u.geo.radius() + gap).collect();
    let total: f64 = widths.iter().sum();
    // Wrap long rows into a square-ish grid.
    let n = us.len().max(1);
    let cols = (n as f64).sqrt().ceil() as usize;
    let cell = widths.iter().copied().fold(0.0, f64::max);
    let _ = total;
    let mut out = Vec::new();
    for (i, u) in us.iter().enumerate() {
        let (cx, cz) = ((i % cols) as f64, (i / cols) as f64);
        let rows = n.div_ceil(cols);
        let c = [
            (cx - (cols as f64 - 1.0) / 2.0) * cell,
            0.0,
            (cz - (rows as f64 - 1.0) / 2.0) * cell,
        ];
        for p in &u.geo.positions {
            out.push([p[0] + c[0], p[1] + c[1], p[2] + c[2]]);
        }
    }
    out
}

fn smooth(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// The atoms, bonds, events and effect of `rec` at time `t` (0..1): bonds
/// stretch, then break; atoms travel to their product places; new bonds
/// pull in; the visible changes rise once the products form.
pub fn frame(rec: &ShowRecord, geo: &Geometries, fx: &crate::discover::Effect, t: f32) -> Frame {
    let tt = t.clamp(0.0, 1.0) as f64;
    let eq = rec_equation(rec);
    let from = layout(&eq.reactants, geo);
    let to_flat = layout(&eq.products, geo);
    let pr = flatten(&units(&eq.products, geo));
    let index_of: BTreeMap<[usize; 3], usize> =
        pr.0.iter().enumerate().map(|(i, (_, k))| (*k, i)).collect();
    let story = &rec.atom_story;
    let n = story.atoms.len();
    // Each atom moves between the end of its first break and the end of its
    // last form.
    let mut t0 = vec![f64::INFINITY; n];
    let mut t1 = vec![f64::NEG_INFINITY; n];
    for ev in &story.events {
        for x in [ev.a, ev.b] {
            match ev.kind {
                EventKind::Break => t0[x] = t0[x].min(ev.start + ev.duration),
                EventKind::Form => t1[x] = t1[x].max(ev.start + ev.duration),
            }
        }
    }
    let mut pos: Vec<[f64; 3]> = (0..n)
        .map(|i| {
            let a = from[i];
            let b = to_flat[index_of[&story.atoms[i].to]];
            let s0 = if t0[i].is_finite() { t0[i] } else { 0.35 };
            let s1 = if t1[i].is_finite() { t1[i] } else { 0.85 };
            let s1 = if s1 <= s0 { s0 + 0.1 } else { s1 };
            let u = smooth((tt - s0) / (s1 - s0));
            [
                a[0] + (b[0] - a[0]) * u,
                a[1] + (b[1] - a[1]) * u,
                a[2] + (b[2] - a[2]) * u,
            ]
        })
        .collect();
    let progress = |ev: &StoryEvent| ((tt - ev.start) / ev.duration).clamp(0.0, 1.0);
    // Stretch: a breaking bond's atoms are pushed apart (up to 0.4 Å) during
    // the event, relaxing over the next 0.1.
    for ev in story.events.iter().filter(|e| e.kind == EventKind::Break) {
        let end = ev.start + ev.duration;
        let d = if tt < ev.start {
            0.0
        } else if tt <= end {
            0.4 * progress(ev)
        } else {
            0.4 * (1.0 - (tt - end) / 0.1).max(0.0)
        };
        if d > 0.0 {
            let (a, b) = (pos[ev.a], pos[ev.b]);
            let v = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-6);
            for k in 0..3 {
                pos[ev.a][k] -= v[k] / len * d / 2.0;
                pos[ev.b][k] += v[k] / len * d / 2.0;
            }
        }
    }
    let atoms = (0..n)
        .map(|i| FrameAtom {
            el: story.atoms[i].el.clone(),
            pos: [pos[i][0] as f32, pos[i][1] as f32, pos[i][2] as f32],
        })
        .collect();
    // Bonds: reactant bonds until they break; product bonds once they form.
    let rflat = flatten(&units(&eq.reactants, geo));
    let key = |a: usize, b: usize| (a.min(b), a.max(b));
    let event_of: BTreeMap<((usize, usize), EventKind), &StoryEvent> = story
        .events
        .iter()
        .map(|e| ((key(e.a, e.b), e.kind), e))
        .collect();
    let inv: BTreeMap<[usize; 3], usize> = story
        .atoms
        .iter()
        .enumerate()
        .map(|(i, a)| (a.to, i))
        .collect();
    let mut bonds = Vec::new();
    let mut seen = BTreeSet::new();
    for &(a, b, o) in &rflat.1 {
        let k = key(a, b);
        match event_of.get(&(k, EventKind::Break)) {
            Some(ev) if tt > ev.start + ev.duration => {}
            Some(ev) => bonds.push(FrameBond {
                a,
                b,
                order: o as f32,
                stretch: (1.0 + 0.5 * progress(ev)) as f32,
            }),
            None => {
                seen.insert(k);
                bonds.push(FrameBond {
                    a,
                    b,
                    order: o as f32,
                    stretch: 1.0,
                });
            }
        }
    }
    for &(pa, pb, o) in &pr.1 {
        let (a, b) = (inv[&pr.0[pa].1], inv[&pr.0[pb].1]);
        let k = key(a, b);
        if seen.contains(&k) {
            // Kept from the reactants: its order switches half way.
            if tt >= 0.5 {
                if let Some(bd) = bonds.iter_mut().find(|x| key(x.a, x.b) == k) {
                    bd.order = o as f32;
                }
            }
            continue;
        }
        if let Some(ev) = event_of.get(&(k, EventKind::Form)) {
            if tt >= ev.start {
                bonds.push(FrameBond {
                    a,
                    b,
                    order: o as f32,
                    stretch: (1.6 - 0.6 * progress(ev)) as f32,
                });
            }
        }
    }
    let events = story
        .events
        .iter()
        .filter(|e| tt >= e.start && tt <= e.start + e.duration)
        .map(|e| FrameEvent {
            kind: e.kind,
            a: e.a,
            b: e.b,
            progress: progress(e) as f32,
        })
        .collect();
    let visible = rec
        .visible
        .iter()
        .map(|&v| {
            let (a, b) = match v {
                Visible::Flame | Visible::Glow | Visible::Shimmer => (0.45, 0.6),
                _ => (0.55, 0.9),
            };
            (v, smooth((tt - a) / (b - a)) as f32)
        })
        .collect();
    Frame {
        t: tt as f32,
        atoms,
        bonds,
        events,
        effect: FrameEffect {
            reaction_id: rec.reaction_id.clone(),
            kind: fx.kind,
            heat: fx.heat,
            strength: fx.strength,
            tint: fx.tint.clone(),
            visible,
        },
    }
}

fn rec_equation(rec: &ShowRecord) -> Equation {
    let side = |s: Side| {
        rec.species
            .iter()
            .filter(|x| x.side == s)
            .map(|x| {
                format!(
                    "{}{}",
                    if x.coef > 1 {
                        x.coef.to_string()
                    } else {
                        String::new()
                    },
                    x.formula
                )
            })
            .collect::<Vec<_>>()
            .join(" + ")
    };
    parse_equation(&format!(
        "{} -> {}",
        side(Side::Reactant),
        side(Side::Product)
    ))
    .expect("a record's equation parses")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover::Effects;

    fn shows() -> Shows {
        Shows::load()
    }

    #[test]
    fn every_codex_reaction_has_a_complete_show_record() {
        let c = Codex::load();
        let s = shows();
        assert_eq!(s.records.len(), c.entries.len());
        for (e, r) in c.entries.iter().zip(&s.records) {
            assert_eq!(r.reaction_id, e.id);
            let eq = parse_equation(&e.equation).unwrap();
            assert_eq!(
                r.species.len(),
                eq.reactants.len() + eq.products.len(),
                "{}",
                e.id
            );
            assert!(
                !r.visible.is_empty() || r.note.is_some(),
                "{}: nothing visible and no note",
                e.id
            );
            assert!(!r.timescale.is_empty(), "{}", e.id);
            assert_eq!(r.sped_up, r.speed == Speed::Slow, "{}", e.id);
            for sp in &r.species {
                assert!(!sp.conformer.is_empty(), "{}: {}", e.id, sp.formula);
                if sp.side == Side::Product {
                    assert!(sp.at_room.is_some(), "{}: {}", e.id, sp.formula);
                }
            }
            if r.has(Visible::Flame) {
                let f = r
                    .flame
                    .as_ref()
                    .unwrap_or_else(|| panic!("{}: flame", e.id));
                assert!(!f.light.described.is_empty());
                let t = f
                    .temp
                    .as_ref()
                    .unwrap_or_else(|| panic!("{}: flame temperature", e.id));
                assert!(
                    t.estimated && (900.0..=6000.0).contains(&t.k),
                    "{}: {}",
                    e.id,
                    t.k
                );
            }
            if r.has(Visible::Precipitate) {
                assert!(!r.precipitates.is_empty(), "{}", e.id);
                for p in &r.precipitates {
                    assert!(p.grain.is_some(), "{}: {} has no grain", e.id, p.formula);
                }
            }
            if r.has(Visible::ColourChange) {
                assert!(
                    !r.colour_changes.is_empty() || r.has(Visible::Flame),
                    "{}",
                    e.id
                );
            }
            assert!(r.atom_story.illustrative);
            assert!(!r.atom_story.events.is_empty(), "{}: no atom story", e.id);
            for ev in &r.atom_story.events {
                assert!(ev.start >= 0.0 && ev.start + ev.duration <= 1.0, "{}", e.id);
                assert!(ev.a < r.atom_story.atoms.len() && ev.b < r.atom_story.atoms.len());
            }
            // Atoms are conserved by the story.
            let ra: usize = r
                .species
                .iter()
                .filter(|x| x.side == Side::Reactant)
                .map(|x| x.coef as usize * s.geometry.species[&x.formula].elements.len())
                .sum();
            assert_eq!(ra, r.atom_story.atoms.len(), "{}", e.id);
        }
    }

    #[test]
    fn every_colour_and_enthalpy_has_a_source_or_is_flagged() {
        let s = shows();
        for (f, sub) in &s.substances {
            for (st, c) in &sub.colour {
                assert!(
                    s.sources.contains_key(&c.src),
                    "{f}({st:?}): colour source {}",
                    c.src
                );
                if let Some(h) = &c.hex {
                    assert!(c.hex_estimated, "{f}: a hex is a rendering, flag it");
                    assert!(
                        h.len() == 7
                            && h.starts_with('#')
                            && h[1..].chars().all(|c| c.is_ascii_hexdigit()),
                        "{f}: {h}"
                    );
                }
            }
            for (st, d) in &sub.dhf {
                match &d.src {
                    Some(src) => assert!(s.sources.contains_key(src), "{f}({st:?}): {src}"),
                    None => assert!(d.estimated && d.how.is_some(), "{f}({st:?}): no source"),
                }
            }
            assert!(!sub.dhf.is_empty(), "{f}: no enthalpy of formation");
        }
        for r in &s.records {
            for c in r
                .species
                .iter()
                .filter_map(|x| x.colour.as_ref())
                .chain(r.smoke.iter().filter_map(|x| x.colour.as_ref()))
            {
                assert!(
                    s.sources.contains_key(&c.src),
                    "{}: {}",
                    r.reaction_id,
                    c.src
                );
            }
            if let Some(f) = &r.flame {
                assert!(s.sources.contains_key(&f.light.src), "{}", r.reaction_id);
            }
            match r.dh.kj {
                Some(_) => {
                    for t in &r.dh.terms {
                        assert!(
                            t.src.is_some() || t.estimated,
                            "{}: {}",
                            r.reaction_id,
                            t.formula
                        );
                    }
                    assert_eq!(r.dh.dh_estimated, r.dh.terms.iter().any(|t| t.estimated));
                }
                None => assert!(r.dh.dh_estimated, "{}", r.reaction_id),
            }
        }
    }

    /// The ΔH sign agrees with the codex heat class, or show.json says why
    /// it does not (each one is listed in the I-054 spec).
    #[test]
    fn the_enthalpy_sign_agrees_with_the_heat_class() {
        let s = shows();
        let mut bad = Vec::new();
        for r in &s.records {
            if r.dh.agrees == Some(false) && r.dh.note.is_none() {
                bad.push(format!("{} {:?}", r.reaction_id, r.dh.kj));
            }
        }
        assert!(bad.is_empty(), "unexplained disagreements: {bad:?}");
        // Spot checks against textbook values (kJ per mole of reaction).
        let kj = |id: &str| s.get(id).unwrap().dh.kj.unwrap();
        assert!(
            (kj("methane_burns") + 802.3).abs() < 1.5,
            "{}",
            kj("methane_burns")
        );
        assert!(
            (kj("neutralise_hcl_naoh") + 55.8).abs() < 1.0,
            "{}",
            kj("neutralise_hcl_naoh")
        );
        assert!(
            (kj("limestone_roast") - 178.3).abs() < 1.0,
            "{}",
            kj("limestone_roast")
        );
        assert!((kj("thermite") + 851.5).abs() < 1.0, "{}", kj("thermite"));
    }

    #[test]
    fn gas_densities_relative_to_air_are_right() {
        let s = shows();
        let gas = |id: &str, f: &str| {
            s.get(id)
                .unwrap()
                .gases
                .iter()
                .find(|g| g.formula == f)
                .unwrap_or_else(|| panic!("{id}: {f}"))
                .clone()
        };
        let h2 = gas("zinc_acid", "H2");
        assert!(h2.density_rel_air < 0.1 && h2.rises);
        let co2 = gas("chalk_acid", "CO2");
        assert!((co2.density_rel_air - 1.52).abs() < 0.01 && !co2.rises);
        assert!(gas("water_splits", "O2").density_rel_air > 1.0);
        let no2 = gas("no_oxidises", "NO2");
        assert_eq!(no2.opacity, Opacity::Tinted);
        let steam = gas("methane_burns", "H2O");
        assert!(steam.rises && steam.opacity == Opacity::Clear && steam.condenses_to.is_some());
        let cl2 = gas("salt_electrolysis", "Cl2");
        assert!(cl2.density_rel_air > 2.4 && cl2.colour.as_ref().unwrap().hex.is_some());
    }

    #[test]
    fn geometry_covers_every_species() {
        let c = Codex::load();
        let g = Geometries::load();
        for f in &c.known_species {
            let geo = g
                .species
                .get(f)
                .unwrap_or_else(|| panic!("{f}: no geometry"));
            assert!(g.sources.contains_key(&geo.src), "{f}: {}", geo.src);
            assert_eq!(geo.elements.len(), geo.positions.len(), "{f}");
            let atoms = realgame_sim::formula::parse_formula(f).unwrap();
            let mut got = BTreeMap::new();
            for e in &geo.elements {
                *got.entry(e.clone()).or_insert(0u32) += 1;
            }
            assert_eq!(got, atoms, "{f}: atoms");
            for &(a, b, o) in &geo.bonds {
                assert!(
                    a < geo.elements.len() && b < geo.elements.len() && a != b,
                    "{f}"
                );
                assert!(
                    [0.0, 1.0, 1.5, 2.0, 3.0].contains(&o),
                    "{f}: bond order {o}"
                );
            }
        }
        // Computed bond lengths are close to experiment.
        let len = |f: &str, i: usize, j: usize| {
            let p = &g.species[f].positions;
            ((p[i][0] - p[j][0]).powi(2)
                + (p[i][1] - p[j][1]).powi(2)
                + (p[i][2] - p[j][2]).powi(2))
            .sqrt()
        };
        let bond = |f: &str| {
            let (a, b, _) = g.species[f].bonds[0];
            len(f, a, b)
        };
        assert!((bond("H2O") - 0.958).abs() < 0.03, "{}", bond("H2O"));
        assert!((bond("CO2") - 1.160).abs() < 0.03, "{}", bond("CO2"));
        assert!((bond("N2") - 1.098).abs() < 0.03, "{}", bond("N2"));
        assert!((bond("Cl2") - 1.988).abs() < 0.06, "{}", bond("Cl2"));
        assert!(g.species["NaCl"].lattice && !g.species["CH4"].lattice);
    }

    #[test]
    fn the_shows_follow_the_chemistry() {
        let s = shows();
        let r = |id: &str| s.get(id).unwrap();
        let ag = r("silver_chloride");
        assert!(ag.has(Visible::Precipitate));
        assert_eq!(ag.precipitates[0].formula, "AgCl");
        assert_eq!(ag.precipitates[0].grain, Some(Grain::Cloudy));
        let st = |id: &str, f: &str| r(id).species.iter().find(|x| x.formula == f).unwrap().state;
        assert_eq!(st("silver_chloride", "AgNO3"), State::Aq);
        assert_eq!(st("silver_chloride", "AgCl"), State::S);
        assert_eq!(st("methane_burns", "H2O"), State::G);
        assert_eq!(st("neutralise_hcl_naoh", "H2O"), State::L);
        assert_eq!(st("salt_electrolysis", "NaCl"), State::L);
        let m = r("methane_burns");
        assert!(m.has(Visible::Flame) && m.has(Visible::Condensation) && m.has(Visible::Shimmer));
        let t = m.flame.as_ref().unwrap().temp.as_ref().unwrap().k;
        // Measured adiabatic methane-air: 2226 K; no dissociation runs high.
        assert!((2200.0..2400.0).contains(&t), "{t}");
        assert!(r("chalk_acid").has(Visible::Bubbles));
        assert!(r("iron_copper_sulfate").has(Visible::ColourChange));
        assert!(r("magnesium_burns").has(Visible::Smoke));
        let rust = r("iron_rusts");
        assert!(rust.sped_up && rust.speed == Speed::Slow);
        assert_eq!(r("neutralise_hcl_naoh").speed, Speed::Instant);
        let na = r("salt_from_elements").flame.as_ref().unwrap();
        assert_eq!(na.light.lines_nm, vec![589.0, 589.6]);
        let after = |id: &str, f: &str| {
            r(id)
                .species
                .iter()
                .find(|x| x.formula == f)
                .unwrap()
                .at_room
        };
        assert_eq!(after("methane_burns", "H2O"), Some(AfterRoom::Condenses));
        assert_eq!(after("thermite", "Fe"), Some(AfterRoom::Freezes));
        assert_eq!(after("chlorine_bromide", "KCl"), Some(AfterRoom::None));
        // A story breaks O=O and H-H and forms O-H for hydrogen burning.
        let h = r("hydrogen_burns");
        let breaks = h
            .atom_story
            .events
            .iter()
            .filter(|e| e.kind == EventKind::Break)
            .count();
        let forms = h
            .atom_story
            .events
            .iter()
            .filter(|e| e.kind == EventKind::Form)
            .count();
        assert_eq!((breaks, forms), (3, 4));
    }

    #[test]
    fn a_frame_shows_bonds_stretch_break_and_form() {
        let s = shows();
        let fx = Effects::load();
        let c = Codex::load();
        let e = c.get("hydrogen_burns").unwrap();
        let rec = s.get("hydrogen_burns").unwrap();
        let f = |t: f32| frame(rec, &s.geometry, &fx.of(e), t);
        let start = f(0.0);
        assert_eq!(start.atoms.len(), 6);
        // 2 H-H and 1 O=O at the start, none formed yet.
        assert_eq!(start.bonds.len(), 3);
        assert!(start.bonds.iter().all(|b| b.stretch == 1.0));
        assert!(start.effect.visible.iter().all(|&(_, i)| i == 0.0));
        // Part way through a break: stretched.
        let ev = rec.atom_story.events[0].clone();
        let mid = f((ev.start + ev.duration / 2.0) as f32);
        let b = mid
            .bonds
            .iter()
            .find(|b| (b.a, b.b) == (ev.a, ev.b) || (b.b, b.a) == (ev.a, ev.b))
            .unwrap();
        assert!(b.stretch > 1.0);
        assert!(mid.events.iter().any(|x| x.kind == EventKind::Break));
        // At the end: 4 O-H bonds, all at rest, the flame at full.
        let end = f(1.0);
        assert_eq!(end.bonds.len(), 4);
        assert!(end.bonds.iter().all(|b| b.stretch == 1.0));
        assert!(end
            .effect
            .visible
            .iter()
            .any(|&(v, i)| v == Visible::Flame && i == 1.0));
        assert_eq!(end.effect.reaction_id, "hydrogen_burns");
        // Every O ends up bonded to two H.
        for (i, a) in end.atoms.iter().enumerate() {
            if a.el == "O" {
                assert_eq!(end.bonds.iter().filter(|b| b.a == i || b.b == i).count(), 2);
            }
        }
        // Positions move continuously.
        let p0 = f(0.5).atoms[0].pos;
        let p1 = f(0.51).atoms[0].pos;
        let d: f32 = (0..3).map(|k| (p0[k] - p1[k]).powi(2)).sum::<f32>().sqrt();
        assert!(d < 0.5, "{d}");
        // A frame serialises with its contract fields.
        let v = serde_json::to_value(&end).unwrap();
        for k in ["t", "atoms", "bonds", "events", "effect"] {
            assert!(v.get(k).is_some(), "{k}");
        }
        // Every record's frames hold together.
        for (e, r) in c.entries.iter().zip(&s.records) {
            for t in [0.0, 0.3, 0.6, 1.0] {
                let fr = frame(r, &s.geometry, &fx.of(e), t);
                assert_eq!(fr.atoms.len(), r.atom_story.atoms.len(), "{}", e.id);
                assert!(
                    fr.atoms.iter().all(|a| a.pos.iter().all(|x| x.is_finite())),
                    "{}",
                    e.id
                );
            }
        }
    }

    #[test]
    fn strength_follows_the_documented_scale() {
        assert_eq!(strength_of(-890.0, false), 1.0);
        assert_eq!(strength_of(0.0, false), 0.1);
        assert!(strength_of(-55.8, false) > 0.5 && strength_of(-55.8, false) < 0.7);
        assert!(strength_of(178.3, true) < strength_of(31.2, false));
    }
}
