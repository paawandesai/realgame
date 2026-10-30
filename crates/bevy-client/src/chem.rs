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

    pub fn name(self) -> &'static str {
        match self {
            Spell::Fire => "Fire",
            Spell::Water => "Water",
            Spell::Frost => "Frost",
            Spell::Foam => "CO₂",
        }
    }

    pub fn formula(self) -> &'static str {
        match self {
            Spell::Fire => "CH₄ + 2O₂ → CO₂ + 2H₂O",
            Spell::Water => "H₂O absorbs heat",
            Spell::Frost => "N₂(l) boils at −196 °C",
            Spell::Foam => "CO₂ blanket, no O₂",
        }
    }

    /// The caption shown when the spell is cast.
    pub fn cast_caption(self) -> &'static str {
        match self {
            Spell::Fire => {
                "Fire spell: a gas flame. Methane burns: CH₄ + 2O₂ → CO₂ + 2H₂O, and lots of heat."
            }
            Spell::Water => {
                "Water soaks up heat and cools things below their ignition temperature."
            }
            Spell::Frost => {
                "Liquid nitrogen boils at −196 °C (77 K), pulling heat out of anything it touches."
            }
            Spell::Foam => {
                "CO₂ from an extinguisher is heavier than air: it smothers fire by pushing the oxygen away."
            }
        }
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
        };
        Obj {
            kind,
            pos,
            half,
            temp,
            burning: false,
            fuel,
            wet: 0.0,
            ice,
            wall,
            smothered: 0.0,
            counts: false,
            put_out: false,
        }
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
        matches!(self.kind, Kind::IceWall | Kind::Puddle)
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
        let neighbours = (0..objs.len())
            .map(|i| {
                (0..objs.len())
                    .filter(|&j| j != i && objs[i].gap(&objs[j]) < NEIGHBOUR_GAP)
                    .collect()
            })
            .collect();
        World {
            objs,
            neighbours,
            extinguished: 0,
            ticks: 0,
            ended: None,
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
        if self
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
                        o.ice = (o.ice - if o.wall { 0.34 } else { 0.5 }).max(0.0);
                        o.temp = 0.0;
                        if o.ice <= 0.0 {
                            o.wall = false;
                            o.kind = Kind::Puddle;
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
                if o.is_water() {
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
        if self.burning() > 0 || self.extinguished >= GOAL_FIRES {
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
