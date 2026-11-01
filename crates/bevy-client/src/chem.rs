//! The chemistry rules engine: a deterministic state machine per object.
//!
//! This module has no Bevy (or any other) dependency on purpose, so it can
//! later move into a shared crate and run on a server. It steps at a fixed
//! rate ([`DT`]), visits objects in index order and has no randomness, so the
//! same spells cast at the same ticks always give the same result.
//!
//! Each [`Obj`] carries a temperature, whether it is burning, its fuel, how
//! wet it is, how much of it is ice, and how long a CO2 blanket still
//! covers it. Spells ([`apply`]) and time ([`step`]) change those numbers;
//! both report what happened as [`Event`]s, which the game turns into
//! particles, colours and captions.

/// Fixed simulation step in seconds (30 Hz).
pub const DT: f32 = 1.0 / 30.0;
/// Air temperature in the courtyard, °C.
pub const AMBIENT_C: f32 = 20.0;
/// Liquid nitrogen boils at 77 K.
pub const LN2_C: f32 = -196.0;
/// Two objects closer than this (gap between their footprints, metres)
/// heat each other when one burns.
pub const NEIGHBOUR_GAP: f32 = 2.5;

/// Heating rate from one burning neighbour, °C per second, before the
/// object's own cooling toward ambient. Tuned with [`COOLING`] for 6 to
/// 9-year-olds: the first neighbour catches fire after about a minute, and
/// unattended fires reach the greenhouse after about 2.5 minutes, with the
/// HUD warning for the last 30 s (see the `courtyard_pacing` test).
const SPREAD_HEAT: f32 = 12.0;
/// Newton cooling toward ambient, per second.
const COOLING: f32 = 0.02;
/// A burning object sits at roughly flame temperature.
const FLAME_C: f32 = 800.0;
/// How long a CO2 blanket keeps oxygen away, seconds.
const SMOTHER_S: f32 = 25.0;
/// Heat one Fire spell adds, °C. One hit lights a dry bush at room
/// temperature; a frost-chilled one (−196 °C) needs to warm up first.
const FIRE_HEAT: f32 = 300.0;
/// Ice one Frost cast adds to a level-2 stream (gameplay tuning, P-020):
/// three casts freeze it solid (0.34, 0.68, then capped at 1).
pub const STREAM_FREEZE: f32 = 0.34;
/// Ice one Fire hit removes from an ice wall or a frozen stream.
const ICE_MELT: f32 = 0.34;
/// Level 3 (I-037): how much of the shallow dip one CO2 cast fills
/// (gameplay tuning, P-021): two casts fill it.
pub const DIP_CO2_PER_CAST: f32 = 0.5;
/// How much of the deeper hollow one CO2 cast fills (gameplay tuning,
/// P-022): three casts fill it (0.34, 0.68, then capped at 1).
pub const HOLLOW_CO2_PER_CAST: f32 = 0.34;
/// How long Water or Frost knocks a gas flame down before it grows back,
/// seconds (gameplay tuning, P-023).
pub const FLICKER_S: f32 = 1.5;
/// Level 4 (I-036): how much of the balloon one split of water fills with
/// hydrogen (gameplay tuning, P-024): three casts fill it (0.34, 0.68, then
/// capped at 1).
pub const H2_PER_SPLIT: f32 = 0.34;
/// How long one split runs, seconds: the tubes fill 2:1, hold, then the gas
/// moves on (gameplay tuning, P-025; display only).
pub const SPLIT_S: f32 = 2.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A green bush: wood and leaves.
    Bush,
    /// A dry hedge: dead leaves, lights easily and burns long.
    DryHedge,
    /// A wall of ice. Melts into a puddle; refreezes as a flat ice sheet.
    IceWall,
    /// A puddle of water: freezes into an ice sheet and melts back.
    Puddle,
    /// A hot steam vent (placeholder for the water-splitting station).
    SteamVent,
    /// The lab bench: the future water-splitting station.
    LabBench,
    /// The greenhouse's wooden frame: if it catches fire, the level is lost.
    Greenhouse,
    /// Level 2: a stream too deep to wade. Solid for movement while liquid
    /// (`ice < 1`); Frost freezes it a third at a time into a bridge.
    Stream,
    /// Level 2: the lantern on its plinth, the goal. Inert.
    Lantern,
    /// Level 3: marsh gas (methane) bubbling up through the water and
    /// burning. Never runs out of fuel and never chars; Water and Frost only
    /// knock the flame down. Only a full CO2 pool in its hollow puts it out.
    GasVent,
    /// Level 3: a low place in the boardwalk (the dip or the hollow). Inert
    /// floor; CO2 cast at it or at a vent in it fills it (`co2`).
    Hollow,
    /// Level 4: the water splitter (a battery and two gas tubes). Water cast
    /// into it is split 2:1 into hydrogen and oxygen; the hydrogen goes to
    /// the balloon.
    Splitter,
    /// Level 4: the balloon tied to the crate. Holds hydrogen (`gas`); full,
    /// it lifts the crate. Fire pops the hydrogen in it.
    Balloon,
    /// Level 4: the iron crate in the doorway. Solid until the balloon lifts
    /// it onto the ledge (`lifted`).
    Crate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spell {
    Fire,
    Water,
    Frost,
    Foam,
}

impl Spell {
    pub const ALL: [Spell; 4] = [Spell::Fire, Spell::Water, Spell::Frost, Spell::Foam];

    /// The chemistry rule this spell runs, as `content/spells.json` names it
    /// in `effect`. An engine key only: never save or send it (CO2's effect
    /// is `foam`). The permanent spell id is `Spell::spell_id()` in spells.rs.
    pub fn effect_id(self) -> &'static str {
        match self {
            Spell::Fire => "fire",
            Spell::Water => "water",
            Spell::Frost => "frost",
            Spell::Foam => "foam",
        }
    }

    /// The inverse of `effect_id`; exact, lower case.
    pub fn from_effect_id(id: &str) -> Option<Spell> {
        Spell::ALL.into_iter().find(|s| s.effect_id() == id)
    }
}

/// What a spell or a tick did to an object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    Ignited,
    /// Put out by a spell (counts toward the objective).
    Extinguished(Spell),
    /// Ran out of fuel by itself.
    BurntOut,
    /// Nothing was burning but the objective was not met yet (a starting
    /// fire burnt out by itself): a new fire starts so the level can end.
    Reseeded,
    /// Ice melted completely into water.
    Melted,
    /// Ice partly melted.
    Melting,
    /// Water froze into ice.
    Froze,
    /// Level 2: Frost added ice to the stream, but it is not solid yet.
    Freezing,
    /// CO2 on ice or a stream: nothing changed (it is no fire to smother).
    NoChange,
    /// Water boiled off as steam.
    Steam,
    /// Foam now covers it: no oxygen, so it cannot burn.
    Smothered,
    /// It got wet.
    Soaked,
    /// Frost chilled it, but nothing changed phase.
    Chilled,
    /// Fire could not light it (wet, smothered or not flammable).
    Fizzle(Fizzle),
    /// The lab bench: explains the coming water-splitting station.
    Station,
    /// Level 3: Water or Frost knocked a gas flame down for [`FLICKER_S`];
    /// it keeps burning.
    Flicker(Spell),
    /// Level 3: CO2 sank into a hollow, but the pool is not full yet.
    Filling,
    /// Level 3: the hollow's CO2 pool is full and every flame in it is out.
    Pooled,
    /// Level 3: CO2 on a vent in no hollow: it spills away.
    Spilled,
    /// Level 4: water split into hydrogen and oxygen; the balloon is not
    /// full yet (or the crate is already up).
    Split,
    /// Level 4: the split filled the balloon, and it lifted the crate.
    Lifted,
    /// Level 4: Fire burnt the hydrogen in the balloon with a pop.
    Popped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fizzle {
    Wet,
    NoOxygen,
    NotFlammable,
    Charred,
    Heated,
    /// Fire aimed at the greenhouse: the spell refuses (the level is about
    /// saving it, and one stray click must not lose it).
    Greenhouse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    pub obj: usize,
    pub kind: EventKind,
}

/// What one landed spell did, for the hit marker and its sound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitKind {
    /// It changed something (soaked, lit, froze, chilled...).
    Hit,
    /// It hit a target but nothing happened.
    Fizzle,
    /// It put a fire out.
    Out,
    /// It hit no target at all.
    Miss,
}

impl HitKind {
    pub fn name(self) -> &'static str {
        match self {
            HitKind::Hit => "hit",
            HitKind::Fizzle => "fizzle",
            HitKind::Out => "out",
            HitKind::Miss => "miss",
        }
    }
}

/// Classifies the events one spell produced on a target: any fire put out
/// is `Out`; otherwise anything but a fizzle is `Hit`; only fizzles (or no
/// events) are `Fizzle`. An impact with no target is `Miss` (the caller's
/// call, since there are no events to look at).
pub fn hit_kind(events: &[Event]) -> HitKind {
    if events
        .iter()
        .any(|e| matches!(e.kind, EventKind::Extinguished(_) | EventKind::Pooled))
    {
        HitKind::Out
    } else if events
        .iter()
        .any(|e| !matches!(e.kind, EventKind::Fizzle(_)))
    {
        HitKind::Hit
    } else {
        HitKind::Fizzle
    }
}

impl EventKind {
    /// The one-line, scientifically accurate caption for this event.
    pub fn caption(self) -> &'static str {
        match self {
            EventKind::Ignited => {
                "It caught fire: hot enough to reach its ignition temperature, with oxygen around."
            }
            EventKind::Extinguished(Spell::Water) => {
                "Fire out! Water cooled it below its ignition temperature."
            }
            EventKind::Extinguished(Spell::Frost) => {
                "Fire out! Liquid nitrogen at −196 °C cooled it far below ignition."
            }
            EventKind::Extinguished(Spell::Foam) => {
                "Fire out! The CO₂ smothered it: no oxygen, no fire."
            }
            EventKind::Extinguished(Spell::Fire) => "Fire out!",
            EventKind::BurntOut => "It burnt out: the fuel is used up.",
            EventKind::Reseeded => "Another bush is on fire! Put it out before it spreads.",
            EventKind::Melted => "The ice melted at 0 °C: H₂O(s) → H₂O(l). Now it is a puddle.",
            EventKind::Melting => "Melting... it takes a lot of heat to turn ice into water.",
            EventKind::Froze => {
                "Frozen! Liquid nitrogen at −196 °C (77 K) froze the water: H₂O(l) → H₂O(s)."
            }
            EventKind::Freezing => {
                "Freezing: H₂O(l) → H₂O(s). The nitrogen drops glide on their own gas (the Leidenfrost effect), so it takes a few casts."
            }
            EventKind::NoChange => {
                "CO₂ keeps oxygen away. That does not melt ice or freeze water here."
            }
            EventKind::Steam => "Hot water boils at 100 °C and turns into steam: H₂O(l) → H₂O(g).",
            EventKind::Smothered => {
                "A CO₂ blanket covers it: no oxygen can reach it, so it cannot burn."
            }
            EventKind::Soaked => "Soaked: it must dry out (boil off at 100 °C) before it can burn.",
            EventKind::Chilled => {
                "Brrr! Liquid nitrogen boils away at −196 °C as it chills the surface."
            }
            EventKind::Fizzle(Fizzle::Wet) => {
                "Sizzle! The water must boil away at 100 °C before it can burn."
            }
            EventKind::Fizzle(Fizzle::NoOxygen) => {
                "No oxygen under the CO₂ blanket, so the fire cannot start."
            }
            EventKind::Fizzle(Fizzle::NotFlammable) => "That does not burn.",
            EventKind::Fizzle(Fizzle::Greenhouse) => {
                "Not the greenhouse! We are here to save it. Aim at the burning bushes."
            }
            EventKind::Fizzle(Fizzle::Charred) => "Already burnt: no fuel left.",
            EventKind::Fizzle(Fizzle::Heated) => {
                "Heating up... a little more heat and it will catch fire."
            }
            EventKind::Station => {
                "Water-splitting station (coming soon): 2H₂O → 2H₂ + O₂ with electricity."
            }
            EventKind::Flicker(Spell::Frost) => {
                "Liquid nitrogen boils into N₂ gas. Once it warms up, N₂ is about as heavy as air, so it mixes away and the methane keeps burning."
            }
            EventKind::Flicker(_) => {
                "Marsh gas is methane (CH₄). It keeps bubbling up through the water, so a splash only knocks the flame down for a moment."
            }
            EventKind::Filling => {
                "CO₂ (44 g/mol) is denser than air (29 g/mol), so it pools low, like an invisible pond."
            }
            EventKind::Pooled => {
                "The CO₂ pool pushed the air out of the hollow. Without O₂, CH₄ + 2O₂ → CO₂ + 2H₂O cannot happen."
            }
            EventKind::Spilled => "No hollow here: the CO₂ spills away downhill.",
            EventKind::Split => {
                "2H₂O → 2H₂ + O₂: the hydrogen tube fills twice as fast as the oxygen tube."
            }
            EventKind::Lifted => {
                "H₂ (2 g/mol) is about 14 times lighter than air (29 g/mol). The balloon floats, and the crate goes with it."
            }
            EventKind::Popped => {
                "Pop! 2H₂ + O₂ → 2H₂O. Hydrogen is flammable, so the balloon burst. Split more water to fill it again."
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Obj {
    pub kind: Kind,
    /// Footprint centre (x, z) and half extents, metres. Used for neighbours.
    pub pos: [f32; 2],
    pub half: [f32; 2],
    pub temp: f32,
    pub burning: bool,
    /// Seconds of burning left. 0 for something that cannot burn or is charred.
    pub fuel: f32,
    /// Surface water, 0 (dry) to 1 (soaked).
    pub wet: f32,
    /// Water bodies: how much is ice, 0 (liquid) to 1 (solid).
    pub ice: f32,
    /// Ice wall still standing (false once it melted into a puddle).
    pub wall: bool,
    /// Seconds of CO2 blanket left.
    pub smothered: f32,
    /// Burning since the level started or because fire spread to it (not
    /// lit by the player): putting it out counts toward the objective.
    pub counts: bool,
    /// A spell put a fire out here at some point: the reseed never relights
    /// it (the player saved it).
    pub put_out: bool,
    /// Level 3 hollows: how full of CO2, 0 (none) to 1 (over the flames).
    pub co2: f32,
    /// Level 3 hollows: how much one CO2 cast adds (0 for everything else).
    pub co2_step: f32,
    /// Level 3 vents: seconds left of a flame knocked down by Water or Frost.
    pub flicker: f32,
    /// Level 4 balloon: how full of hydrogen, 0 (limp) to 1 (lifts the crate).
    pub gas: f32,
    /// Level 4 crate: lifted onto the ledge (out of the doorway).
    pub lifted: bool,
    /// Level 4 splitter: seconds left of the current split (tube display).
    pub run: f32,
}

impl Obj {
    pub fn new(kind: Kind, pos: [f32; 2], half: [f32; 2]) -> Self {
        let (fuel, ice, wall, temp) = match kind {
            Kind::Bush => (150.0, 0.0, false, AMBIENT_C),
            Kind::DryHedge => (90.0, 0.0, false, AMBIENT_C),
            Kind::Greenhouse => (300.0, 0.0, false, AMBIENT_C),
            Kind::IceWall => (0.0, 1.0, true, -5.0),
            Kind::Puddle => (0.0, 0.0, false, AMBIENT_C),
            Kind::SteamVent => (0.0, 0.0, false, 110.0),
            Kind::LabBench => (0.0, 0.0, false, AMBIENT_C),
            Kind::Stream => (0.0, 0.0, false, AMBIENT_C),
            Kind::Lantern => (0.0, 0.0, false, AMBIENT_C),
            Kind::GasVent => (0.0, 0.0, false, FLAME_C),
            Kind::Hollow => (0.0, 0.0, false, AMBIENT_C),
            Kind::Splitter | Kind::Balloon | Kind::Crate => (0.0, 0.0, false, AMBIENT_C),
        };
        Obj {
            kind,
            pos,
            half,
            temp,
            // Marsh gas keeps coming up: a vent is alight from the start.
            burning: kind == Kind::GasVent,
            fuel,
            wet: 0.0,
            ice,
            wall,
            smothered: 0.0,
            counts: false,
            put_out: false,
            co2: 0.0,
            co2_step: 0.0,
            flicker: 0.0,
            gas: 0.0,
            lifted: false,
            run: 0.0,
        }
    }

    /// A level-3 hollow (footprint centre and half extents) that `per_cast`
    /// of CO2 fills each cast.
    pub fn hollow(pos: [f32; 2], half: [f32; 2], per_cast: f32) -> Self {
        Obj {
            co2_step: per_cast,
            ..Obj::new(Kind::Hollow, pos, half)
        }
    }

    /// Whether the point (x, z) lies on this object's footprint.
    pub fn covers(&self, p: [f32; 2]) -> bool {
        (p[0] - self.pos[0]).abs() <= self.half[0] && (p[1] - self.pos[1]).abs() <= self.half[1]
    }

    pub fn flammable(&self) -> bool {
        matches!(self.kind, Kind::Bush | Kind::DryHedge | Kind::Greenhouse)
    }

    /// Ignition temperature, °C (wood and dry leaves light around 250 to 300 °C).
    pub fn ignition_c(&self) -> f32 {
        match self.kind {
            Kind::DryHedge => 230.0,
            _ => 300.0,
        }
    }

    pub fn is_water(&self) -> bool {
        matches!(self.kind, Kind::IceWall | Kind::Puddle | Kind::Stream)
    }

    pub fn frozen(&self) -> bool {
        self.is_water() && self.ice > 0.0
    }

    pub fn charred(&self) -> bool {
        self.flammable() && !self.burning && self.fuel <= 0.0
    }

    /// Footprint gap to another object (0 when they overlap).
    pub fn gap(&self, o: &Obj) -> f32 {
        let dx = ((self.pos[0] - o.pos[0]).abs() - self.half[0] - o.half[0]).max(0.0);
        let dz = ((self.pos[1] - o.pos[1]).abs() - self.half[1] - o.half[1]).max(0.0);
        (dx * dx + dz * dz).sqrt()
    }
}

/// All the objects plus precomputed neighbour lists and the objective.
#[derive(Clone, Debug, Default)]
pub struct World {
    pub objs: Vec<Obj>,
    pub neighbours: Vec<Vec<usize>>,
    /// Fires put out by spells: only ones burning since the start or lit by
    /// spreading heat, never ones the player lit.
    pub extinguished: u32,
    pub ticks: u64,
    /// The outcome once the level is won or lost. Latched: nothing changes
    /// it again (only a fresh World, on reset).
    pub ended: Option<Outcome>,
    /// How this level is won.
    pub goal: Goal,
    /// Level 3: the hollow each gas vent stands in (`None` for anything
    /// else, or a vent outside every hollow).
    pub hollow_of: Vec<Option<usize>>,
}

/// The hollow whose footprint holds gas vent `i`'s centre (the first, if
/// footprints overlap). `None` for anything but a vent.
pub fn hollow_of(objs: &[Obj], i: usize) -> Option<usize> {
    if objs[i].kind != Kind::GasVent {
        return None;
    }
    objs.iter()
        .position(|h| h.kind == Kind::Hollow && h.covers(objs[i].pos))
}

/// How a level is won.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Goal {
    /// Level 1: put out `GOAL_FIRES` fires (the reseed keeps it winnable).
    #[default]
    Fires,
    /// Level 2: reach a place. Never won by the rules on their own; the game
    /// calls [`World::reach`]. No reseed.
    Reach,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Playing,
    Won,
    Lost,
}

/// Fires to put out to save the greenhouse.
pub const GOAL_FIRES: u32 = 3;

impl World {
    pub fn new(objs: Vec<Obj>) -> Self {
        World::with_goal(objs, Goal::Fires)
    }

    pub fn with_goal(objs: Vec<Obj>, goal: Goal) -> Self {
        let neighbours = (0..objs.len())
            .map(|i| {
                (0..objs.len())
                    .filter(|&j| j != i && objs[i].gap(&objs[j]) < NEIGHBOUR_GAP)
                    .collect()
            })
            .collect();
        let hollow_of = (0..objs.len()).map(|i| hollow_of(&objs, i)).collect();
        World {
            hollow_of,
            objs,
            neighbours,
            extinguished: 0,
            ticks: 0,
            ended: None,
            goal,
        }
    }

    /// The player reached the goal (level 2's lantern): latches `Won`.
    pub fn reach(&mut self) {
        if self.ended.is_none() {
            self.ended = Some(Outcome::Won);
        }
    }

    pub fn ignite(&mut self, i: usize) {
        let o = &mut self.objs[i];
        o.burning = true;
        o.temp = FLAME_C;
        o.counts = true;
    }

    /// The greenhouse's temperature, and whether anything next to it is on
    /// fire (the HUD warns while it is).
    pub fn greenhouse_risk(&self) -> Option<(f32, bool)> {
        let i = self.objs.iter().position(|o| o.kind == Kind::Greenhouse)?;
        let near = self.neighbours[i].iter().any(|&j| self.objs[j].burning);
        Some((self.objs[i].temp, near))
    }

    pub fn burning(&self) -> usize {
        self.objs.iter().filter(|o| o.burning).count()
    }

    pub fn outcome(&self) -> Outcome {
        self.ended.unwrap_or_else(|| self.live_outcome())
    }

    /// Latch a won or lost outcome the first time it happens.
    fn latch(&mut self) {
        if self.ended.is_none() {
            let o = self.live_outcome();
            if o != Outcome::Playing {
                self.ended = Some(o);
            }
        }
    }

    fn live_outcome(&self) -> Outcome {
        if self.goal == Goal::Reach {
            // Won only through `reach`, which latches.
            Outcome::Playing
        } else if self
            .objs
            .iter()
            .any(|o| o.kind == Kind::Greenhouse && o.burning)
        {
            Outcome::Lost
        } else if self.extinguished >= GOAL_FIRES && self.burning() == 0 {
            Outcome::Won
        } else {
            Outcome::Playing
        }
    }

    /// A spell lands on object `i`.
    /// Does nothing once the level is won or lost (a spell still in flight
    /// must not change the result).
    pub fn apply(&mut self, i: usize, spell: Spell, events: &mut Vec<Event>) {
        if self.ended.is_some() {
            return;
        }
        self.apply_inner(i, spell, events);
        self.latch();
    }

    fn apply_inner(&mut self, i: usize, spell: Spell, events: &mut Vec<Event>) {
        if matches!(self.objs[i].kind, Kind::GasVent | Kind::Hollow) {
            self.apply_marsh(i, spell, events);
            return;
        }
        if matches!(
            self.objs[i].kind,
            Kind::Splitter | Kind::Balloon | Kind::Crate
        ) {
            self.apply_lab(i, spell, events);
            return;
        }
        let mut ev = |kind| events.push(Event { obj: i, kind });
        let o = &mut self.objs[i];
        if o.kind == Kind::LabBench {
            ev(EventKind::Station);
            return;
        }
        match spell {
            Spell::Fire => {
                if o.is_water() {
                    if o.ice > 0.0 {
                        let step = if o.wall || o.kind == Kind::Stream {
                            ICE_MELT
                        } else {
                            0.5
                        };
                        o.ice = (o.ice - step).max(0.0);
                        o.temp = 0.0;
                        if o.ice <= 1e-4 {
                            o.ice = 0.0;
                            // A stream stays a stream; a wall slumps into a puddle.
                            if o.kind == Kind::IceWall {
                                o.wall = false;
                                o.kind = Kind::Puddle;
                            }
                            o.temp = 5.0;
                            ev(EventKind::Melted);
                        } else {
                            ev(EventKind::Melting);
                        }
                    } else {
                        o.temp = 100.0;
                        ev(EventKind::Steam);
                    }
                } else if o.kind == Kind::SteamVent {
                    o.temp = o.temp.max(110.0);
                    ev(EventKind::Steam);
                } else if !o.flammable() {
                    ev(EventKind::Fizzle(Fizzle::NotFlammable));
                } else if o.kind == Kind::Greenhouse {
                    ev(EventKind::Fizzle(Fizzle::Greenhouse));
                } else if o.burning {
                    o.temp = FLAME_C;
                } else if o.fuel <= 0.0 {
                    ev(EventKind::Fizzle(Fizzle::Charred));
                } else if o.smothered > 0.0 {
                    ev(EventKind::Fizzle(Fizzle::NoOxygen));
                } else if o.wet > 0.0 {
                    // The flame's heat goes into boiling the water off first.
                    o.wet = (o.wet - 0.5).max(0.0);
                    o.temp = 100.0;
                    ev(EventKind::Fizzle(Fizzle::Wet));
                } else {
                    // The flame heats it from whatever temperature it has now:
                    // something chilled by liquid nitrogen must warm up first.
                    o.temp += FIRE_HEAT;
                    if o.temp >= o.ignition_c() {
                        o.burning = true;
                        o.temp = FLAME_C;
                        // Lit by the player: putting it out does not count.
                        o.counts = false;
                        ev(EventKind::Ignited);
                    } else {
                        ev(EventKind::Fizzle(Fizzle::Heated));
                    }
                }
            }
            Spell::Water => {
                if o.kind == Kind::SteamVent {
                    ev(EventKind::Steam);
                    return;
                }
                if o.is_water() {
                    ev(EventKind::Soaked);
                    return;
                }
                let was = o.burning;
                // Water holds the temperature at 100 °C while it boils off.
                o.temp = o.temp.min(100.0);
                o.wet = 1.0;
                if was {
                    o.burning = false;
                    ev(EventKind::Extinguished(Spell::Water));
                    self.extinguished += u32::from(std::mem::take(&mut o.counts));
                    o.put_out = true;
                } else {
                    ev(EventKind::Soaked);
                }
            }
            Spell::Frost => {
                let was = o.burning;
                if o.kind == Kind::SteamVent {
                    o.temp = 0.0;
                    ev(EventKind::Chilled);
                    return;
                }
                o.temp = LN2_C;
                if o.kind == Kind::IceWall {
                    // Cold does not rebuild a wall: it only chills the ice.
                    ev(EventKind::Chilled);
                } else if o.kind == Kind::Stream {
                    if o.ice < 1.0 {
                        o.ice = (o.ice + STREAM_FREEZE).min(1.0);
                        if o.ice > 1.0 - 1e-4 {
                            o.ice = 1.0;
                            ev(EventKind::Froze);
                        } else {
                            ev(EventKind::Freezing);
                        }
                    } else {
                        ev(EventKind::Chilled);
                    }
                } else if o.is_water() {
                    if o.ice < 1.0 {
                        o.ice = 1.0;
                        ev(EventKind::Froze);
                    } else {
                        ev(EventKind::Chilled);
                    }
                } else if was {
                    o.burning = false;
                    ev(EventKind::Extinguished(Spell::Frost));
                    self.extinguished += u32::from(std::mem::take(&mut o.counts));
                    o.put_out = true;
                } else {
                    // Any surface water freezes too.
                    ev(EventKind::Chilled);
                }
            }
            Spell::Foam => {
                if matches!(o.kind, Kind::Stream | Kind::IceWall) {
                    ev(EventKind::NoChange);
                    return;
                }
                if !o.flammable() {
                    ev(EventKind::Smothered);
                    return;
                }
                let was = o.burning;
                o.smothered = SMOTHER_S;
                o.burning = false;
                if was {
                    ev(EventKind::Extinguished(Spell::Foam));
                    self.extinguished += u32::from(std::mem::take(&mut o.counts));
                    o.put_out = true;
                } else {
                    ev(EventKind::Smothered);
                }
            }
        }
    }

    /// Level 3: a spell lands on a gas vent or a hollow's floor. CO2 fills
    /// the hollow it lands in (the same rise from a vent or the floor) and
    /// at 1 smothers every vent in it at once; Water and Frost only knock
    /// burning flames down; Fire relights nothing under a full pool.
    fn apply_marsh(&mut self, i: usize, spell: Spell, events: &mut Vec<Event>) {
        let hollow = if self.objs[i].kind == Kind::Hollow {
            Some(i)
        } else {
            self.hollow_of[i]
        };
        // The vents this spell reaches: the one hit, or all in the hollow.
        let vents: Vec<usize> = if self.objs[i].kind == Kind::GasVent {
            vec![i]
        } else {
            (0..self.objs.len())
                .filter(|&j| self.hollow_of[j] == Some(i))
                .collect()
        };
        let lit: Vec<usize> = vents
            .iter()
            .copied()
            .filter(|&j| self.objs[j].burning)
            .collect();
        let mut ev = |obj, kind| events.push(Event { obj, kind });
        match spell {
            Spell::Foam => {
                let Some(h) = hollow else {
                    // No low place to hold it: the gas spills away.
                    ev(i, EventKind::Spilled);
                    return;
                };
                let o = &mut self.objs[h];
                if o.co2 < 1.0 {
                    o.co2 = (o.co2 + o.co2_step).min(1.0);
                    if o.co2 > 1.0 - 1e-4 {
                        o.co2 = 1.0;
                    }
                }
                if o.co2 < 1.0 {
                    ev(h, EventKind::Filling);
                    return;
                }
                // Full: the pool covers every flame in the hollow.
                for j in 0..self.objs.len() {
                    if self.hollow_of[j] == Some(h) {
                        let v = &mut self.objs[j];
                        v.burning = false;
                        v.smothered = f32::MAX;
                        v.flicker = 0.0;
                    }
                }
                ev(h, EventKind::Pooled);
            }
            Spell::Water | Spell::Frost => {
                if lit.is_empty() {
                    ev(
                        i,
                        if spell == Spell::Water {
                            EventKind::Soaked
                        } else {
                            EventKind::Chilled
                        },
                    );
                    return;
                }
                for j in lit {
                    self.objs[j].flicker = FLICKER_S;
                }
                ev(i, EventKind::Flicker(spell));
            }
            Spell::Fire => {
                if !lit.is_empty() {
                    ev(i, EventKind::NoChange);
                } else if vents.is_empty() {
                    ev(i, EventKind::Fizzle(Fizzle::NotFlammable));
                } else if vents.iter().any(|&j| self.objs[j].smothered > 0.0) {
                    ev(i, EventKind::Fizzle(Fizzle::NoOxygen));
                } else {
                    // Out but not smothered (only a test makes one): relight.
                    for j in vents {
                        let v = &mut self.objs[j];
                        v.burning = true;
                        v.temp = FLAME_C;
                        v.counts = false;
                    }
                    ev(i, EventKind::Ignited);
                }
            }
        }
    }

    /// Level 4 (I-036): a spell lands on the water splitter, the balloon or
    /// the crate. Water splits water at the splitter: the hydrogen fills the
    /// balloon a step at a time, and once it is full the balloon lifts the
    /// crate (for good). Fire pops the hydrogen in the balloon. Nothing else
    /// changes anything.
    fn apply_lab(&mut self, i: usize, spell: Spell, events: &mut Vec<Event>) {
        let balloon = self.objs.iter().position(|o| o.kind == Kind::Balloon);
        let crate_ = self.objs.iter().position(|o| o.kind == Kind::Crate);
        let mut ev = |kind| events.push(Event { obj: i, kind });
        match (self.objs[i].kind, spell) {
            (Kind::Splitter, Spell::Water) => {
                self.objs[i].run = SPLIT_S;
                let Some(b) = balloon else {
                    ev(EventKind::Split);
                    return;
                };
                let o = &mut self.objs[b];
                o.gas = (o.gas + H2_PER_SPLIT).min(1.0);
                if o.gas > 1.0 - 1e-4 {
                    o.gas = 1.0;
                }
                let full = o.gas >= 1.0;
                match crate_ {
                    Some(c) if full && !self.objs[c].lifted => {
                        self.objs[c].lifted = true;
                        ev(EventKind::Lifted);
                    }
                    _ => ev(EventKind::Split),
                }
            }
            (Kind::Splitter, Spell::Fire) => ev(EventKind::Fizzle(Fizzle::NotFlammable)),
            (Kind::Splitter, Spell::Frost) => ev(EventKind::Chilled),
            (Kind::Splitter, Spell::Foam) => ev(EventKind::Smothered),
            (Kind::Balloon, Spell::Fire) => {
                let o = &mut self.objs[i];
                if o.gas > 0.0 {
                    // 2H2 + O2 -> 2H2O: the hydrogen burns with the air's
                    // oxygen. The crate, if lifted, stays on its ledge.
                    o.gas = 0.0;
                    ev(EventKind::Popped);
                } else {
                    ev(EventKind::Fizzle(Fizzle::NotFlammable));
                }
            }
            (Kind::Balloon, Spell::Foam) => ev(EventKind::NoChange),
            (Kind::Balloon, Spell::Water) => ev(EventKind::Soaked),
            (Kind::Balloon, Spell::Frost) => ev(EventKind::Chilled),
            _ => ev(EventKind::NoChange),
        }
    }

    /// One fixed step of [`DT`] seconds: burning, spreading heat, drying,
    /// cooling and the CO2 blanket wearing off.
    pub fn step(&mut self, events: &mut Vec<Event>) {
        if self.ended.is_some() {
            return;
        }
        self.ticks += 1;
        let n = self.objs.len();
        // Heat each object receives from burning neighbours this step, and
        // whether any of those fires counts toward the objective (a fire
        // spread from one the player lit does not).
        let mut heat = vec![0.0f32; n];
        let mut from_counted = vec![false; n];
        for (i, h) in heat.iter_mut().enumerate() {
            for &j in &self.neighbours[i] {
                if self.objs[j].burning {
                    // Closer neighbours heat more: full at 0 m, half at the edge.
                    let g = self.objs[i].gap(&self.objs[j]);
                    *h += SPREAD_HEAT * (1.0 - 0.5 * g / NEIGHBOUR_GAP);
                    from_counted[i] |= self.objs[j].counts;
                }
            }
        }
        for (i, o) in self.objs.iter_mut().enumerate() {
            let mut ev = |kind| events.push(Event { obj: i, kind });
            // Level 3: gas vents burn on their own fuel, which never runs
            // out; a full CO2 pool keeps them out for good. Hollows are inert.
            if o.kind == Kind::GasVent {
                o.flicker = (o.flicker - DT).max(0.0);
                if o.smothered > 0.0 {
                    o.burning = false;
                }
                if o.burning {
                    o.temp = FLAME_C;
                }
                continue;
            }
            if o.kind == Kind::Hollow {
                continue;
            }
            // Level 4: only the splitter's run (the tube display) changes
            // with time; the balloon keeps its gas and the crate its place.
            if matches!(o.kind, Kind::Splitter | Kind::Balloon | Kind::Crate) {
                o.run = (o.run - DT).max(0.0);
                continue;
            }
            // The CO2 blanket stays until the object has cooled below its
            // ignition temperature, so a smothered fire cannot come back
            // the moment the blanket lifts.
            if !o.flammable() || o.temp < o.ignition_c() {
                o.smothered = (o.smothered - DT).max(0.0);
            }
            if o.burning {
                o.fuel -= DT;
                o.temp = FLAME_C;
                if o.smothered > 0.0 {
                    o.burning = false;
                } else if o.fuel <= 0.0 {
                    o.fuel = 0.0;
                    o.burning = false;
                    ev(EventKind::BurntOut);
                }
                continue;
            }
            if o.kind == Kind::SteamVent {
                // The vent is fed by hot water underground: it warms back up.
                o.temp += (110.0 - o.temp) * 0.2 * DT;
                continue;
            }
            o.temp += (heat[i] + (AMBIENT_C - o.temp) * COOLING) * DT;
            if o.is_water() {
                // Ice holds at 0 °C or below while it is ice.
                if o.ice > 0.0 {
                    o.temp = o.temp.min(0.0);
                    if heat[i] > 0.0 {
                        o.ice = (o.ice - heat[i] * 0.002 * DT).max(0.0);
                        if o.ice <= 0.0 {
                            o.wall = false;
                            o.kind = Kind::Puddle;
                            ev(EventKind::Melted);
                        }
                    }
                }
                continue;
            }
            // Wet things stay at 100 °C until the water has boiled off.
            if o.wet > 0.0 {
                if o.temp > 100.0 {
                    o.wet = (o.wet - (o.temp - 100.0) * 0.004 * DT).max(0.0);
                    o.temp = 100.0;
                } else {
                    o.wet = (o.wet - 0.01 * DT).max(0.0);
                }
            }
            if o.flammable()
                && o.fuel > 0.0
                && o.wet <= 0.0
                && o.smothered <= 0.0
                && o.temp >= o.ignition_c()
            {
                o.burning = true;
                o.temp = FLAME_C;
                // Spread fire: putting it out counts if the fire it came
                // from counts.
                o.counts = from_counted[i];
                ev(EventKind::Ignited);
            }
        }
        self.reseed(events);
        self.latch();
    }

    /// Keeps the level winnable: when nothing burns and fewer than
    /// [`GOAL_FIRES`] counted fires were put out (a starting fire burnt out
    /// by itself, say), the bush farthest from the greenhouse that still has
    /// fuel and that no spell has put out catches fire. With no such bush,
    /// the greenhouse is safe and the level is won.
    fn reseed(&mut self, events: &mut Vec<Event>) {
        if self.goal == Goal::Reach || self.burning() > 0 || self.extinguished >= GOAL_FIRES {
            return;
        }
        let gh = self
            .objs
            .iter()
            .find(|o| o.kind == Kind::Greenhouse)
            .map_or([0.0, 0.0], |o| o.pos);
        let far = |o: &Obj| (o.pos[0] - gh[0]).powi(2) + (o.pos[1] - gh[1]).powi(2);
        let pick = (0..self.objs.len())
            .filter(|&i| {
                let o = &self.objs[i];
                o.kind == Kind::Bush && o.fuel > 0.0 && !o.put_out
            })
            .max_by(|&a, &b| far(&self.objs[a]).total_cmp(&far(&self.objs[b])));
        match pick {
            Some(i) => {
                self.objs[i].wet = 0.0;
                self.ignite(i);
                events.push(Event {
                    obj: i,
                    kind: EventKind::Reseeded,
                });
            }
            None => self.extinguished = self.extinguished.max(GOAL_FIRES),
        }
    }
}

/// Geometry of the lab-bench electrolysis prop (a Hoffman voltameter).
///
/// 2H2O(l) -> 2H2(g) + O2(g): at the same temperature and pressure the
/// hydrogen fills exactly twice the volume of the oxygen (Avogadro). Both
/// tubes have the same diameter, so the gas column heights are 2:1. This is
/// the only source of the prop's tube and column sizes; `world.rs` draws what
/// [`tubes`] returns. Units are metres. Diameters are the cylinder mesh's
/// `scale.x` (the mesh is radius 0.5, height 1.0).
pub mod electrolysis {
    /// Centre height of each glass tube.
    pub const TUBE_Y: f32 = 1.6;
    /// Height of each glass tube.
    pub const TUBE_H: f32 = 1.1;
    /// Outer diameter of each glass tube.
    pub const TUBE_D: f32 = 0.25;
    /// Gap between the glass ends and the water / gas inside.
    pub const INSET: f32 = 0.025;
    /// Diameter of the water and gas columns (inside the glass).
    pub const FILL_D: f32 = 0.2;
    /// Distance of each tube from the battery along z.
    pub const TUBE_DZ: f32 = 0.9;
    /// The cathode tube collects hydrogen.
    pub const CATHODE_DZ: f32 = -TUBE_DZ;
    /// The anode tube collects oxygen.
    pub const ANODE_DZ: f32 = TUBE_DZ;
    /// Height of the oxygen column.
    pub const GAS_O2: f32 = 0.3;
    /// Height of the hydrogen column: exactly twice the oxygen.
    pub const GAS_H2: f32 = 2.0 * GAS_O2;

    /// One cylinder: centre y, height and diameter.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Piece {
        pub y: f32,
        pub h: f32,
        pub d: f32,
    }

    /// One tube: its z offset from the battery and its three pieces.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Tube {
        pub dz: f32,
        pub glass: Piece,
        pub water: Piece,
        pub gas: Piece,
    }

    /// A tube at `dz` with a gas column `gas` metres tall collected at the
    /// top, over water filling the rest.
    pub fn tube(dz: f32, gas: f32) -> Tube {
        let inner_bot = TUBE_Y - TUBE_H / 2.0 + INSET;
        let inner_top = TUBE_Y + TUBE_H / 2.0 - INSET;
        let gas_bot = inner_top - gas;
        Tube {
            dz,
            glass: Piece {
                y: TUBE_Y,
                h: TUBE_H,
                d: TUBE_D,
            },
            water: Piece {
                y: (inner_bot + gas_bot) / 2.0,
                h: gas_bot - inner_bot,
                d: FILL_D,
            },
            gas: Piece {
                y: (gas_bot + inner_top) / 2.0,
                h: gas,
                d: FILL_D,
            },
        }
    }

    /// The hydrogen (cathode) tube, then the oxygen (anode) tube.
    pub fn tubes() -> [Tube; 2] {
        [tube(CATHODE_DZ, GAS_H2), tube(ANODE_DZ, GAS_O2)]
    }

    /// Level 4 (I-036): the H2 and O2 gas column heights shown while a split
    /// runs, with `run` seconds of [`super::SPLIT_S`] left. Both columns grow
    /// (the first 40 %), hold at the full heights, then empty together (the
    /// last 20 %, as the hydrogen goes on to the balloon), so the hydrogen
    /// column is always exactly twice the oxygen column.
    pub fn column(run: f32) -> (f32, f32) {
        let p = 1.0 - (run / super::SPLIT_S).clamp(0.0, 1.0);
        let f = if p < 0.4 {
            p / 0.4
        } else if p <= 0.8 {
            1.0
        } else {
            (1.0 - p) / 0.2
        };
        let f = f.clamp(0.0, 1.0);
        (GAS_H2 * f, GAS_O2 * f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effect_ids_round_trip() {
        assert_eq!(
            Spell::ALL.map(Spell::effect_id),
            ["fire", "water", "frost", "foam"]
        );
        for s in Spell::ALL {
            assert_eq!(Spell::from_effect_id(s.effect_id()), Some(s));
        }
        assert_eq!(Spell::from_effect_id("co2"), None);
        assert_eq!(Spell::from_effect_id("Fire"), None);
    }

    fn row() -> World {
        // Three bushes 3 m apart (gap 1.2 m) and an isolated puddle.
        World::new(vec![
            Obj::new(Kind::Bush, [0.0, 0.0], [0.9, 0.9]),
            Obj::new(Kind::Bush, [3.0, 0.0], [0.9, 0.9]),
            Obj::new(Kind::Bush, [6.0, 0.0], [0.9, 0.9]),
            Obj::new(Kind::Puddle, [20.0, 0.0], [1.0, 1.0]),
        ])
    }

    fn run(w: &mut World, secs: f32) -> Vec<Event> {
        let mut ev = Vec::new();
        for _ in 0..(secs / DT) as usize {
            w.step(&mut ev);
        }
        ev
    }

    #[test]
    fn neighbours_by_gap() {
        let w = row();
        assert_eq!(w.neighbours[0], vec![1]);
        assert_eq!(w.neighbours[1], vec![0, 2]);
        assert!(w.neighbours[3].is_empty());
    }

    #[test]
    fn fire_spreads_slowly_then_burns_out() {
        let mut w = row();
        w.ignite(0);
        run(&mut w, 20.0);
        assert!(!w.objs[1].burning, "spread must be slow");
        let ev = run(&mut w, 60.0);
        assert!(w.objs[1].burning);
        assert!(ev.contains(&Event {
            obj: 1,
            kind: EventKind::Ignited
        }));
        run(&mut w, 400.0);
        assert!(w.objs.iter().all(|o| !o.burning));
        assert!(w.objs[0].charred());
    }

    #[test]
    fn water_and_foam_extinguish_and_count() {
        let mut w = row();
        w.ignite(0);
        w.ignite(2);
        let mut ev = Vec::new();
        w.apply(0, Spell::Water, &mut ev);
        w.apply(2, Spell::Foam, &mut ev);
        assert_eq!(w.extinguished, 2);
        assert_eq!(w.burning(), 0);
        // Wet and smothered things resist fire.
        w.apply(0, Spell::Fire, &mut ev);
        assert!(!w.objs[0].burning);
        w.apply(2, Spell::Fire, &mut ev);
        assert_eq!(ev.last().unwrap().kind, EventKind::Fizzle(Fizzle::NoOxygen));
        // A dry one lights.
        w.apply(1, Spell::Fire, &mut ev);
        assert!(w.objs[1].burning);
    }

    #[test]
    fn ice_melts_and_refreezes() {
        let mut w = World::new(vec![Obj::new(Kind::IceWall, [0.0, 0.0], [0.5, 4.0])]);
        let mut ev = Vec::new();
        for _ in 0..3 {
            w.apply(0, Spell::Fire, &mut ev);
        }
        assert_eq!(w.objs[0].kind, Kind::Puddle);
        assert!(!w.objs[0].wall && !w.objs[0].frozen());
        w.apply(0, Spell::Frost, &mut ev);
        assert!(w.objs[0].frozen());
        assert!(!w.objs[0].wall, "refreezes flat, as a sheet");
        assert_eq!(ev.last().unwrap().kind, EventKind::Froze);
    }

    #[test]
    fn win_and_lose() {
        let mut w = World::new(vec![
            Obj::new(Kind::Bush, [0.0, 0.0], [0.9, 0.9]),
            Obj::new(Kind::Bush, [10.0, 0.0], [0.9, 0.9]),
            Obj::new(Kind::Bush, [20.0, 0.0], [0.9, 0.9]),
            Obj::new(Kind::Greenhouse, [30.0, 0.0], [1.0, 1.0]),
        ]);
        for i in 0..3 {
            w.ignite(i);
        }
        let mut ev = Vec::new();
        w.apply(0, Spell::Water, &mut ev);
        w.apply(1, Spell::Frost, &mut ev);
        assert_eq!(w.outcome(), Outcome::Playing);
        w.apply(2, Spell::Foam, &mut ev);
        assert_eq!(w.outcome(), Outcome::Won);
        // Latched: nothing after the win changes it (a spell in flight).
        w.apply(1, Spell::Fire, &mut ev);
        w.ignite(3);
        assert_eq!(w.outcome(), Outcome::Won);
        assert!(!w.objs[1].burning, "spells do nothing once the level ended");

        let mut l = World::new(vec![
            Obj::new(Kind::Bush, [0.0, 0.0], [0.9, 0.9]),
            Obj::new(Kind::Greenhouse, [2.5, 0.0], [0.5, 0.5]),
        ]);
        l.ignite(0);
        run(&mut l, 300.0);
        assert_eq!(l.outcome(), Outcome::Lost);
        // Water landing on the burning greenhouse afterwards changes nothing.
        l.apply(1, Spell::Water, &mut ev);
        assert_eq!(l.outcome(), Outcome::Lost);
        assert!(l.objs[1].burning);
    }

    #[test]
    fn chilled_must_warm_before_it_burns() {
        let mut w = row();
        let mut ev = Vec::new();
        w.apply(0, Spell::Frost, &mut ev);
        w.apply(0, Spell::Fire, &mut ev);
        assert!(!w.objs[0].burning, "−196 °C + one flame is not enough");
        assert_eq!(ev.last().unwrap().kind, EventKind::Fizzle(Fizzle::Heated));
        w.apply(0, Spell::Fire, &mut ev);
        assert!(w.objs[0].burning);
    }

    #[test]
    fn foam_fire_stays_out() {
        // A bush and a hedge, each smothered by CO2. A far bush is there for
        // the reseed to light, so the level keeps running while they cool;
        // the reseed must never pick the bushes the player put out.
        for kind in [Kind::Bush, Kind::DryHedge] {
            let mut w = World::new(vec![
                Obj::new(kind, [0.0, 0.0], [0.9, 0.9]),
                Obj::new(Kind::Bush, [40.0, 0.0], [0.9, 0.9]),
            ]);
            w.ignite(0);
            let mut ev = Vec::new();
            w.apply(0, Spell::Foam, &mut ev);
            let ev = run(&mut w, 300.0);
            assert!(
                !w.objs[0].burning
                    && !ev.iter().any(|e| e.obj == 0
                        && matches!(e.kind, EventKind::Ignited | EventKind::Reseeded)),
                "{kind:?}: a smothered fire must not relight by itself"
            );
            assert!(w.objs[0].temp < w.objs[0].ignition_c(), "{kind:?}");
        }
    }

    #[test]
    fn reseed_skips_bushes_the_player_saved() {
        // The smothered bush is the farthest from the greenhouse, and its
        // blanket wears off long before the check: still not relit.
        let mut w = World::new(vec![
            Obj::new(Kind::Bush, [40.0, 0.0], [0.9, 0.9]),
            Obj::new(Kind::Bush, [10.0, 0.0], [0.9, 0.9]),
            Obj::new(Kind::Greenhouse, [-30.0, 0.0], [1.0, 1.0]),
        ]);
        w.ignite(0);
        let mut ev = Vec::new();
        w.apply(0, Spell::Foam, &mut ev);
        let ev = run(&mut w, 120.0);
        let re: Vec<_> = ev
            .iter()
            .filter(|e| e.kind == EventKind::Reseeded)
            .collect();
        assert_eq!(re.len(), 1);
        assert_eq!(re[0].obj, 1);
        assert!(!w.objs[0].burning);
    }

    #[test]
    fn only_spreading_or_starting_fires_count() {
        let mut w = row();
        let mut ev = Vec::new();
        // Farming: light the bush yourself and put it out, three times.
        for _ in 0..3 {
            w.objs[1].wet = 0.0;
            w.objs[1].temp = AMBIENT_C;
            w.apply(1, Spell::Fire, &mut ev);
            assert!(w.objs[1].burning);
            w.apply(1, Spell::Frost, &mut ev);
        }
        assert_eq!(w.extinguished, 0);
        // A fire that was burning from the start counts, once.
        w.ignite(0);
        w.apply(0, Spell::Water, &mut ev);
        w.apply(0, Spell::Water, &mut ev);
        assert_eq!(w.extinguished, 1);
    }

    #[test]
    fn spread_from_a_player_fire_does_not_count() {
        let mut w = row();
        let mut ev = Vec::new();
        w.apply(0, Spell::Fire, &mut ev);
        assert!(w.objs[0].burning && !w.objs[0].counts);
        run(&mut w, 80.0);
        assert!(w.objs[1].burning, "it spread");
        assert!(!w.objs[1].counts);
        w.apply(1, Spell::Water, &mut ev);
        assert_eq!(w.extinguished, 0);
        // From a starting fire, it does count.
        let mut w = row();
        w.ignite(0);
        run(&mut w, 80.0);
        assert!(w.objs[1].counts);
    }

    #[test]
    fn level_always_resolves_when_a_fire_burns_out() {
        let bush = |x| Obj::new(Kind::Bush, [x, 0.0], [0.9, 0.9]);
        let mut w = World::new(vec![
            bush(0.0),
            bush(10.0),
            bush(20.0),
            Obj::new(Kind::Greenhouse, [-30.0, 0.0], [1.0, 1.0]),
        ]);
        w.ignite(0);
        w.ignite(1);
        w.objs[1].fuel = 1.0;
        let mut ev = Vec::new();
        w.apply(0, Spell::Water, &mut ev);
        // Bush 1 burns out: nothing burns, 1 of 3 counted.
        let ev = run(&mut w, 2.0);
        assert!(ev.iter().any(|e| e.kind == EventKind::BurntOut));
        let re: Vec<_> = ev
            .iter()
            .filter(|e| e.kind == EventKind::Reseeded)
            .collect();
        assert_eq!(re.len(), 1);
        assert_eq!(re[0].obj, 2, "farthest from the greenhouse");
        let mut ev = Vec::new();
        for _ in 0..10 {
            if let Some(i) = w.objs.iter().position(|o| o.burning) {
                w.apply(i, Spell::Frost, &mut ev);
            }
            run(&mut w, 1.0);
            if w.outcome() != Outcome::Playing {
                break;
            }
        }
        assert_eq!(w.outcome(), Outcome::Won);
    }

    #[test]
    fn fire_will_not_light_the_greenhouse() {
        let mut w = World::new(vec![Obj::new(Kind::Greenhouse, [0.0, 0.0], [8.0, 0.15])]);
        let mut ev = Vec::new();
        w.apply(0, Spell::Fire, &mut ev);
        assert!(!w.objs[0].burning);
        assert_eq!(w.outcome(), Outcome::Playing);
        assert_eq!(
            ev.last().unwrap().kind,
            EventKind::Fizzle(Fizzle::Greenhouse)
        );
    }

    /// The courtyard's bush chains (positions as in world.rs): unattended,
    /// the greenhouse must survive the first two minutes and fall later.
    #[test]
    fn courtyard_pacing() {
        let bush = |x, z| Obj::new(Kind::Bush, [x, z], [0.9, 0.9]);
        let mut w = World::new(vec![
            bush(-13.5, -3.0),
            bush(-12.0, -6.5),
            bush(-9.6, -10.2),
            bush(13.5, -3.0),
            bush(12.0, -6.5),
            bush(9.6, -10.2),
            bush(0.0, -5.0),
            bush(0.0, -9.2),
            Obj::new(Kind::Greenhouse, [0.0, -12.0], [8.0, 0.15]),
        ]);
        for i in [0, 3, 6] {
            w.ignite(i);
        }
        let mut first_spread = None;
        let mut warned = None;
        let mut lost = None;
        for t in 0..(600.0 / DT) as usize {
            let mut ev = Vec::new();
            w.step(&mut ev);
            let secs = t as f32 * DT;
            if first_spread.is_none() && w.burning() > 3 {
                first_spread = Some(secs);
            }
            if warned.is_none() && w.greenhouse_risk().unwrap().1 {
                warned = Some(secs);
            }
            if w.outcome() == Outcome::Lost {
                lost = Some(secs);
                break;
            }
        }
        let (s, wn, l) = (first_spread.unwrap(), warned.unwrap(), lost.unwrap());
        eprintln!("pacing: first spread {s:.0} s, warning {wn:.0} s, lost {l:.0} s");
        assert!(s > 30.0, "first spread after {s} s");
        assert!(l > 120.0, "greenhouse lost after {l} s");
        assert!(
            l - wn > 25.0,
            "warning {wn} s only {} s before the loss",
            l - wn
        );
    }

    #[test]
    fn deterministic() {
        let mut a = row();
        let mut b = row();
        a.ignite(0);
        b.ignite(0);
        let ea = run(&mut a, 40.0);
        let eb = run(&mut b, 40.0);
        assert_eq!(ea, eb);
        assert_eq!(a.objs, b.objs);
    }

    #[test]
    fn hit_kind_classifies_impacts() {
        let ev = |k: &[EventKind]| -> Vec<Event> {
            k.iter().map(|&kind| Event { obj: 0, kind }).collect()
        };
        assert_eq!(hit_kind(&ev(&[EventKind::Soaked])), HitKind::Hit);
        assert_eq!(hit_kind(&ev(&[EventKind::Station])), HitKind::Hit);
        assert_eq!(
            hit_kind(&ev(&[EventKind::Fizzle(Fizzle::Wet)])),
            HitKind::Fizzle
        );
        assert_eq!(hit_kind(&[]), HitKind::Fizzle);
        assert_eq!(
            hit_kind(&ev(&[
                EventKind::Fizzle(Fizzle::Wet),
                EventKind::Extinguished(Spell::Water)
            ])),
            HitKind::Out
        );
        for k in [EventKind::Ignited, EventKind::Froze, EventKind::Chilled] {
            assert_eq!(hit_kind(&ev(&[k])), HitKind::Hit, "{k:?}");
        }
    }

    #[test]
    fn electrolysis_gas_is_two_to_one() {
        use electrolysis::tubes;
        let [h2, o2] = tubes();
        assert!((h2.gas.h / o2.gas.h - 2.0).abs() < 1e-5);
    }

    #[test]
    fn electrolysis_columns_fit_tube() {
        use electrolysis::{tubes, INSET};
        for t in tubes() {
            assert!(t.water.h > 0.05 && t.gas.h > 0.05);
            let glass_bot = t.glass.y - t.glass.h / 2.0;
            let glass_top = t.glass.y + t.glass.h / 2.0;
            let water_bot = t.water.y - t.water.h / 2.0;
            let water_top = t.water.y + t.water.h / 2.0;
            let gas_bot = t.gas.y - t.gas.h / 2.0;
            let gas_top = t.gas.y + t.gas.h / 2.0;
            assert!(water_bot >= glass_bot - 1e-5, "water below glass");
            assert!(gas_top <= glass_top + 1e-5, "gas above glass");
            assert!((water_top - gas_bot).abs() < 1e-5, "gap or overlap");
            assert!((t.water.h + t.gas.h - (t.glass.h - 2.0 * INSET)).abs() < 1e-5);
            assert!(t.water.d < t.glass.d && t.gas.d < t.glass.d);
        }
    }

    #[test]
    fn electrolysis_h2_at_cathode() {
        use electrolysis::tubes;
        let [h2, o2] = tubes();
        assert!(h2.dz < 0.0);
        assert!(o2.dz > 0.0);
        assert_eq!(h2.dz, -o2.dz);
        assert!(h2.gas.h > o2.gas.h);
    }

    #[test]
    fn electrolysis_difference_readable() {
        use electrolysis::tubes;
        let [h2, o2] = tubes();
        assert!(h2.gas.h - o2.gas.h >= 0.2);
    }

    // ---- I-035 (level 2, the Frozen Gate): acceptance tests 1 and 2 ----

    fn level2_world() -> World {
        World::with_goal(
            vec![
                Obj::new(Kind::Stream, [0.0, 6.0], [24.0, 2.0]),
                Obj::new(Kind::IceWall, [0.0, -8.0], [2.0, 0.5]),
                Obj::new(Kind::Lantern, [0.0, -17.0], [0.4, 0.4]),
            ],
            Goal::Reach,
        )
    }

    fn kinds(ev: &[Event]) -> Vec<EventKind> {
        ev.iter().map(|e| e.kind).collect()
    }

    #[test]
    fn i035_stream_freezes_in_three_frost_casts_and_melts_back() {
        // CI compiles chem.rs alone (rustc --test), so mirror world::solid_obj for a stream.
        let solid = |w: &World| w.objs[0].ice < 1.0;
        let mut w = level2_world();
        assert!(w.objs[0].is_water());
        assert_eq!(w.objs[0].ice, 0.0);
        assert!(solid(&w), "a liquid stream is too deep to wade");
        // Wrong chemistry on the liquid stream never changes its ice.
        for (spell, want) in [
            (Spell::Fire, EventKind::Steam),
            (Spell::Water, EventKind::Soaked),
            (Spell::Foam, EventKind::NoChange),
        ] {
            let mut ev = Vec::new();
            w.apply(0, spell, &mut ev);
            assert_eq!(kinds(&ev), vec![want], "{spell:?} on the liquid stream");
            assert_eq!(
                w.objs[0].ice, 0.0,
                "{spell:?} must not change the stream's ice"
            );
            assert!(solid(&w));
        }
        // Three Frost casts: 0.34, 0.68, then capped at 1.0.
        for (ice, want) in [
            (0.34, EventKind::Freezing),
            (0.68, EventKind::Freezing),
            (1.0, EventKind::Froze),
        ] {
            let mut ev = Vec::new();
            w.apply(0, Spell::Frost, &mut ev);
            assert_eq!(kinds(&ev), vec![want]);
            assert!(
                (w.objs[0].ice - ice).abs() < 1e-5,
                "ice {} want {ice}",
                w.objs[0].ice
            );
            assert_eq!(solid(&w), ice < 1.0, "solid while ice < 1 (ice {ice})");
        }
        assert_eq!(w.objs[0].ice, 1.0);
        assert!(w.objs[0].frozen());
        // Frost on the frozen stream only chills it.
        let mut ev = Vec::new();
        w.apply(0, Spell::Frost, &mut ev);
        assert_eq!(kinds(&ev), vec![EventKind::Chilled]);
        assert_eq!(w.objs[0].ice, 1.0);
        // Water and CO2 on the frozen stream change nothing either.
        for spell in [Spell::Water, Spell::Foam] {
            let mut ev = Vec::new();
            w.apply(0, spell, &mut ev);
            assert_eq!(w.objs[0].ice, 1.0, "{spell:?} on the bridge");
        }
        // Fire melts it back, 0.34 a hit, solid again from the first hit.
        for (ice, want) in [
            (0.66, EventKind::Melting),
            (0.32, EventKind::Melting),
            (0.0, EventKind::Melted),
        ] {
            let mut ev = Vec::new();
            w.apply(0, Spell::Fire, &mut ev);
            assert_eq!(kinds(&ev), vec![want]);
            assert!(
                (w.objs[0].ice - ice).abs() < 1e-5,
                "ice {} want {ice}",
                w.objs[0].ice
            );
            assert!(solid(&w), "solid again from the first Fire hit (ice {ice})");
            assert_eq!(w.objs[0].kind, Kind::Stream, "a stream stays a stream");
        }
        // ...and Frost freezes it again: the level cannot be soft-locked.
        for _ in 0..3 {
            w.apply(0, Spell::Frost, &mut Vec::new());
        }
        assert!(!solid(&w));
    }

    #[test]
    fn i035_frost_and_co2_do_not_change_an_ice_wall() {
        let mut w = level2_world();
        let mut ev = Vec::new();
        w.apply(1, Spell::Frost, &mut ev);
        assert_eq!(kinds(&ev), vec![EventKind::Chilled]);
        assert!(w.objs[1].wall);
        assert_eq!(w.objs[1].ice, 1.0);
        // A partly melted wall is chilled, not refrozen (it does not take the
        // water-body "set ice = 1" path).
        w.apply(1, Spell::Fire, &mut Vec::new());
        assert!((w.objs[1].ice - 0.66).abs() < 1e-5);
        let mut ev = Vec::new();
        w.apply(1, Spell::Frost, &mut ev);
        assert_eq!(kinds(&ev), vec![EventKind::Chilled]);
        assert!((w.objs[1].ice - 0.66).abs() < 1e-5);
        assert!(w.objs[1].wall);
        // CO2 and Water: nothing changes.
        for (spell, want) in [
            (Spell::Foam, EventKind::NoChange),
            (Spell::Water, EventKind::Soaked),
        ] {
            let before = w.objs[1].clone();
            let mut ev = Vec::new();
            w.apply(1, spell, &mut ev);
            assert_eq!(kinds(&ev), vec![want]);
            assert_eq!(w.objs[1].ice, before.ice);
            assert_eq!(w.objs[1].wall, before.wall);
        }
        // Fire still melts it in 3 hits in all (existing rule).
        w.apply(1, Spell::Fire, &mut Vec::new());
        let mut ev = Vec::new();
        w.apply(1, Spell::Fire, &mut ev);
        assert_eq!(kinds(&ev), vec![EventKind::Melted]);
        assert!(!w.objs[1].wall);
    }

    #[test]
    fn i035_reach_goal_never_wins_or_reseeds_by_itself() {
        let mut w = level2_world();
        assert_eq!(World::new(vec![]).goal, Goal::Fires);
        assert_eq!(w.goal, Goal::Reach);
        let mut ev = Vec::new();
        for _ in 0..20_000 {
            w.step(&mut ev);
            assert_eq!(w.outcome(), Outcome::Playing);
        }
        assert!(
            !ev.iter().any(|e| e.kind == EventKind::Reseeded),
            "no reseed under Goal::Reach"
        );
        assert_eq!(w.ended, None);
        assert_eq!(w.extinguished, 0);
        w.reach();
        assert_eq!(w.outcome(), Outcome::Won);
        assert_eq!(w.ended, Some(Outcome::Won));
        let before = w.objs.clone();
        let mut ev = Vec::new();
        for s in Spell::ALL {
            w.apply(0, s, &mut ev);
            w.apply(1, s, &mut ev);
        }
        assert!(ev.is_empty(), "apply after the win does nothing");
        assert_eq!(w.objs, before);
        w.reach();
        assert_eq!(w.outcome(), Outcome::Won, "still latched");
    }

    #[test]
    fn i035_a_fires_world_with_no_bushes_still_wins_at_once() {
        // Level 1's rule is unchanged: with no bush to reseed, it is won.
        let mut w = World::new(vec![Obj::new(Kind::Puddle, [0.0, 0.0], [1.0, 1.0])]);
        w.step(&mut Vec::new());
        assert_eq!(w.outcome(), Outcome::Won);
    }
    // ---- I-037 (level 3, the Marsh): acceptance tests 1, 2 and 3 ----

    /// A dip (0.5 a cast) holding 3 vents, a hollow (0.34 a cast) holding 4
    /// vents, and 1 vent outside both. Indices: 0 dip, 1..=3 dip vents,
    /// 4 hollow, 5..=8 hollow vents, 9 the outside vent, 10 the lantern.
    fn marsh_world() -> World {
        let mut objs = vec![Obj::hollow([0.0, 10.0], [1.6, 2.5], DIP_CO2_PER_CAST)];
        for x in [-1.0, 0.0, 1.0] {
            objs.push(Obj::new(Kind::GasVent, [x, 10.0], [0.5, 0.4]));
        }
        objs.push(Obj::hollow([0.0, -8.0], [2.0, 3.0], HOLLOW_CO2_PER_CAST));
        for (x, z) in [(-1.5, -7.0), (1.5, -7.0), (-1.5, -9.0), (1.5, -9.0)] {
            objs.push(Obj::new(Kind::GasVent, [x, z], [1.55, 0.4]));
        }
        objs.push(Obj::new(Kind::GasVent, [8.0, 0.0], [0.5, 0.5]));
        objs.push(Obj::new(Kind::Lantern, [0.0, -17.0], [0.4, 0.4]));
        World::with_goal(objs, Goal::Reach)
    }
    const DIP: usize = 0;
    const DIP_VENTS: [usize; 3] = [1, 2, 3];
    const HOLLOW: usize = 4;
    const HOLLOW_VENTS: [usize; 4] = [5, 6, 7, 8];
    const OUTSIDE: usize = 9;

    fn lit(w: &World, vents: &[usize]) -> usize {
        vents.iter().filter(|&&i| w.objs[i].burning).count()
    }

    #[test]
    fn i037_gas_vents_burn_forever_and_shrug_off_water_frost_and_fire() {
        let mut w = marsh_world();
        assert_eq!(w.hollow_of[1], Some(DIP));
        assert_eq!(w.hollow_of[5], Some(HOLLOW));
        assert_eq!(w.hollow_of[OUTSIDE], None);
        let all: Vec<usize> = DIP_VENTS
            .into_iter()
            .chain(HOLLOW_VENTS)
            .chain([OUTSIDE])
            .collect();
        assert_eq!(lit(&w, &all), 8, "every vent starts burning");
        for &i in &all {
            assert!(!w.objs[i].flammable(), "a vent is not a flammable solid");
        }
        let mut ev = Vec::new();
        for _ in 0..20_000 {
            w.step(&mut ev);
        }
        assert!(
            !ev.iter().any(|e| e.kind == EventKind::BurntOut),
            "gas vents never burn out"
        );
        assert_eq!(lit(&w, &all), 8);
        for &i in &all {
            assert!(!w.objs[i].charred());
        }
        // Water and Frost knock a flame down; it keeps burning.
        for spell in [Spell::Water, Spell::Frost] {
            let before = w.extinguished;
            let mut ev = Vec::new();
            w.apply(2, spell, &mut ev);
            assert_eq!(kinds(&ev), vec![EventKind::Flicker(spell)], "{spell:?}");
            assert!(w.objs[2].burning, "{spell:?} leaves the vent burning");
            assert_eq!(w.extinguished, before);
            assert!(w.objs[2].flicker > 0.0 && w.objs[2].flicker <= FLICKER_S);
            // The flicker wears off.
            for _ in 0..((FLICKER_S / DT) as usize + 2) {
                w.step(&mut Vec::new());
            }
            assert_eq!(w.objs[2].flicker, 0.0);
            assert!(w.objs[2].burning);
        }
        // Fire on a burning vent changes nothing.
        let before = w.objs.clone();
        let mut ev = Vec::new();
        w.apply(2, Spell::Fire, &mut ev);
        assert_eq!(kinds(&ev), vec![EventKind::NoChange]);
        assert_eq!(w.objs, before);
    }

    #[test]
    fn i037_co2_pools_low_and_smothers_a_whole_hollow_at_once() {
        let mut w = marsh_world();
        // The dip: 0.5, then 1.0 (Filling, Pooled).
        let mut ev = Vec::new();
        w.apply(2, Spell::Foam, &mut ev);
        assert_eq!(kinds(&ev), vec![EventKind::Filling]);
        assert_eq!(ev[0].obj, DIP, "the event is on the hollow");
        assert_eq!(w.objs[DIP].co2, 0.5);
        assert_eq!(lit(&w, &DIP_VENTS), 3, "half a pool is below the flames");
        let mut ev = Vec::new();
        w.apply(DIP, Spell::Foam, &mut ev);
        assert_eq!(kinds(&ev), vec![EventKind::Pooled]);
        assert_eq!(ev[0].obj, DIP);
        assert_eq!(w.objs[DIP].co2, 1.0);
        assert_eq!(lit(&w, &DIP_VENTS), 0, "every dip flame is out");
        assert_eq!(lit(&w, &HOLLOW_VENTS), 4, "the other hollow still burns");
        assert!(w.objs[OUTSIDE].burning);
        // At 1 already: Pooled again, nothing changes.
        let before = w.objs.clone();
        let mut ev = Vec::new();
        w.apply(1, Spell::Foam, &mut ev);
        assert_eq!(kinds(&ev), vec![EventKind::Pooled]);
        assert_eq!(w.objs, before);
        // The hollow: 0.34, 0.68, 1.0 (Filling, Filling, Pooled), alternating
        // vent and floor hits: the same rise either way.
        for (k, (at, pool, want)) in [
            (5, 0.34, EventKind::Filling),
            (HOLLOW, 0.68, EventKind::Filling),
            (8, 1.0, EventKind::Pooled),
        ]
        .into_iter()
        .enumerate()
        {
            let mut ev = Vec::new();
            w.apply(at, Spell::Foam, &mut ev);
            assert_eq!(kinds(&ev), vec![want], "cast {k}");
            assert_eq!(ev[0].obj, HOLLOW);
            assert!(
                (w.objs[HOLLOW].co2 - pool).abs() < 1e-5,
                "pool {} want {pool}",
                w.objs[HOLLOW].co2
            );
            assert_eq!(
                lit(&w, &HOLLOW_VENTS),
                if pool < 1.0 { 4 } else { 0 },
                "no vent goes out before the pool is full (cast {k})"
            );
        }
        assert_eq!(w.objs[HOLLOW].co2, 1.0);
        assert!(w.objs[OUTSIDE].burning);
        // Smothered vents stay out, and Fire cannot relight them.
        let mut ev = Vec::new();
        for _ in 0..20_000 {
            w.step(&mut ev);
        }
        assert_eq!(lit(&w, &DIP_VENTS) + lit(&w, &HOLLOW_VENTS), 0);
        assert!(!ev.iter().any(|e| e.kind == EventKind::Ignited));
        for i in DIP_VENTS.into_iter().chain(HOLLOW_VENTS) {
            let mut ev = Vec::new();
            w.apply(i, Spell::Fire, &mut ev);
            assert_eq!(
                kinds(&ev),
                vec![EventKind::Fizzle(Fizzle::NoOxygen)],
                "Fire on smothered vent {i}"
            );
            assert!(!w.objs[i].burning);
        }
        // CO2 on the vent outside every hollow spills away.
        let before = w.objs.clone();
        let mut ev = Vec::new();
        w.apply(OUTSIDE, Spell::Foam, &mut ev);
        assert_eq!(kinds(&ev), vec![EventKind::Spilled]);
        assert_eq!(w.objs, before);
    }

    #[test]
    fn i037_a_cast_at_the_floor_fills_like_a_cast_at_a_vent() {
        let mut a = marsh_world();
        let mut b = marsh_world();
        a.apply(5, Spell::Foam, &mut Vec::new());
        b.apply(HOLLOW, Spell::Foam, &mut Vec::new());
        assert_eq!(a.objs, b.objs);
        assert!((a.objs[HOLLOW].co2 - HOLLOW_CO2_PER_CAST).abs() < 1e-6);
    }

    #[test]
    fn i037_a_marsh_never_wins_loses_or_reseeds_by_itself() {
        let mut w = marsh_world();
        let mut ev = Vec::new();
        for _ in 0..20_000 {
            w.step(&mut ev);
            assert_eq!(w.outcome(), Outcome::Playing);
        }
        assert_eq!(w.ended, None);
        assert!(!ev.iter().any(|e| e.kind == EventKind::Reseeded));
        assert_eq!(w.burning(), 8);
        w.reach();
        assert_eq!(w.outcome(), Outcome::Won);
    }

    // ---- I-036 (level 4, the Crystal Lab) ----

    const SPLITTER: usize = 0;
    const BALLOON: usize = 1;
    const CRATE: usize = 2;

    /// The lab's chemistry: the splitter, the balloon, the crate and the
    /// lantern, the way level 4 lays them out.
    fn lab() -> World {
        World::with_goal(
            vec![
                Obj::new(Kind::Splitter, [-4.0, 8.0], [1.2, 0.5]),
                Obj::new(Kind::Balloon, [1.0, -3.5], [2.5, 1.0]),
                Obj::new(Kind::Crate, [0.0, -4.0], [1.2, 0.5]),
                Obj::new(Kind::Lantern, [0.0, -16.0], [0.4, 0.4]),
            ],
            Goal::Reach,
        )
    }

    fn cast(w: &mut World, i: usize, s: Spell) -> Vec<EventKind> {
        let mut ev = Vec::new();
        w.apply(i, s, &mut ev);
        ev.into_iter().map(|e| e.kind).collect()
    }

    #[test]
    fn i036_three_splits_fill_the_balloon_and_lift_the_crate() {
        let mut w = lab();
        let want = [
            (0.34, EventKind::Split, false),
            (0.68, EventKind::Split, false),
            (1.0, EventKind::Lifted, true),
        ];
        for (gas, kind, up) in want {
            let ev = cast(&mut w, SPLITTER, Spell::Water);
            assert_eq!(ev, vec![kind]);
            assert!(
                (w.objs[BALLOON].gas - gas).abs() < 1e-6,
                "gas {}",
                w.objs[BALLOON].gas
            );
            assert_eq!(w.objs[CRATE].lifted, up);
            assert!(
                (w.objs[SPLITTER].run - SPLIT_S).abs() < 1e-6,
                "a split runs"
            );
        }
        assert_eq!(w.objs[BALLOON].gas, 1.0, "snapped to exactly full");
        // A fourth split changes nothing.
        let before = w.objs.clone();
        assert_eq!(cast(&mut w, SPLITTER, Spell::Water), vec![EventKind::Split]);
        assert_eq!(w.objs[BALLOON].gas, before[BALLOON].gas);
        assert!(w.objs[CRATE].lifted);
        assert_eq!(w.outcome(), Outcome::Playing);
    }

    #[test]
    fn i036_wrong_spells_change_nothing() {
        let mut w = lab();
        assert_eq!(
            cast(&mut w, SPLITTER, Spell::Fire),
            vec![EventKind::Fizzle(Fizzle::NotFlammable)]
        );
        assert_eq!(
            cast(&mut w, SPLITTER, Spell::Frost),
            vec![EventKind::Chilled]
        );
        assert_eq!(
            cast(&mut w, SPLITTER, Spell::Foam),
            vec![EventKind::Smothered]
        );
        assert_eq!(w.objs[BALLOON].gas, 0.0);
        assert_eq!(w.objs[SPLITTER].run, 0.0, "only water runs it");
        // One split, then the wrong spells on the balloon keep its gas.
        cast(&mut w, SPLITTER, Spell::Water);
        let gas = w.objs[BALLOON].gas;
        assert_eq!(
            cast(&mut w, BALLOON, Spell::Foam),
            vec![EventKind::NoChange]
        );
        assert_eq!(cast(&mut w, BALLOON, Spell::Water), vec![EventKind::Soaked]);
        assert_eq!(
            cast(&mut w, BALLOON, Spell::Frost),
            vec![EventKind::Chilled]
        );
        assert_eq!(w.objs[BALLOON].gas, gas);
        for s in Spell::ALL {
            assert_eq!(
                cast(&mut w, CRATE, s),
                vec![EventKind::NoChange],
                "{s:?} on the crate"
            );
        }
        assert!(!w.objs[CRATE].lifted);
        assert_eq!(w.objs[BALLOON].gas, gas);
    }

    #[test]
    fn i036_fire_pops_the_hydrogen_and_the_crate_stays_up() {
        let mut w = lab();
        assert_eq!(
            cast(&mut w, BALLOON, Spell::Fire),
            vec![EventKind::Fizzle(Fizzle::NotFlammable)],
            "nothing to burn in an empty balloon"
        );
        cast(&mut w, SPLITTER, Spell::Water);
        assert_eq!(cast(&mut w, BALLOON, Spell::Fire), vec![EventKind::Popped]);
        assert_eq!(w.objs[BALLOON].gas, 0.0);
        assert!(!w.objs[CRATE].lifted);
        // Refill, lift, then pop: the crate stays on the ledge.
        for _ in 0..3 {
            cast(&mut w, SPLITTER, Spell::Water);
        }
        assert!(w.objs[CRATE].lifted);
        assert_eq!(cast(&mut w, BALLOON, Spell::Fire), vec![EventKind::Popped]);
        assert_eq!(w.objs[BALLOON].gas, 0.0);
        assert!(w.objs[CRATE].lifted, "no soft-lock: the crate stays up");
        assert_eq!(
            hit_kind(&[Event {
                obj: BALLOON,
                kind: EventKind::Popped
            }]),
            HitKind::Hit
        );
        assert_eq!(
            hit_kind(&[Event {
                obj: SPLITTER,
                kind: EventKind::Lifted
            }]),
            HitKind::Hit
        );
    }

    #[test]
    fn i036_a_lab_never_wins_loses_or_reseeds_by_itself() {
        let mut w = lab();
        cast(&mut w, SPLITTER, Spell::Water);
        assert!(w.objs[SPLITTER].run > 0.0);
        let ev = run(&mut w, 20_000.0 * DT);
        assert!(ev.is_empty(), "{ev:?}");
        assert_eq!(w.outcome(), Outcome::Playing);
        assert_eq!(w.objs[SPLITTER].run, 0.0, "the split ran down");
        assert!((w.objs[BALLOON].gas - H2_PER_SPLIT).abs() < 1e-6);
        w.reach();
        assert_eq!(w.outcome(), Outcome::Won);
    }

    #[test]
    fn i036_split_columns_are_always_two_to_one() {
        use electrolysis::{column, GAS_H2, GAS_O2};
        assert_eq!(column(0.0), (0.0, 0.0));
        assert_eq!(column(SPLIT_S), (0.0, 0.0));
        let mut full = false;
        for k in 0..=250 {
            let run = SPLIT_S * k as f32 / 250.0;
            let (h2, o2) = column(run);
            assert!((h2 - 2.0 * o2).abs() < 1e-6, "run {run}: {h2} vs {o2}");
            assert!((0.0..=GAS_H2 + 1e-6).contains(&h2) && (0.0..=GAS_O2 + 1e-6).contains(&o2));
            full |= (h2 - GAS_H2).abs() < 1e-6 && (o2 - GAS_O2).abs() < 1e-6;
        }
        assert!(
            full,
            "the columns reach the full 2:1 heights while the split holds"
        );
    }
}
