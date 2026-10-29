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
/// object's own cooling toward ambient.
const SPREAD_HEAT: f32 = 30.0;
/// Newton cooling toward ambient, per second.
const COOLING: f32 = 0.05;
/// A burning object sits at roughly flame temperature.
const FLAME_C: f32 = 800.0;
/// How long a CO2 blanket keeps oxygen away, seconds.
const SMOTHER_S: f32 = 25.0;

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
            Spell::Foam => "Foam",
        }
    }

    pub fn formula(self) -> &'static str {
        match self {
            Spell::Fire => "CH4 + 2O2 → CO2 + 2H2O",
            Spell::Water => "H2O absorbs heat",
            Spell::Frost => "N2(l) boils at −196 °C",
            Spell::Foam => "CO2 blanket, no O2",
        }
    }

    /// The caption shown when the spell is cast.
    pub fn cast_caption(self) -> &'static str {
        match self {
            Spell::Fire => "Combustion: CH4 + 2O2 → CO2 + 2H2O, and lots of heat.",
            Spell::Water => {
                "Water soaks up heat and cools things below their ignition temperature."
            }
            Spell::Frost => {
                "Liquid nitrogen boils at −196 °C (77 K), pulling heat out of anything it touches."
            }
            Spell::Foam => "CO2 is heavier than air: it smothers fire by pushing the oxygen away.",
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
                "Fire out! The CO2 foam smothered it: no oxygen, no fire."
            }
            EventKind::Extinguished(Spell::Fire) => "Fire out!",
            EventKind::BurntOut => "It burnt out: the fuel is used up.",
            EventKind::Melted => "The ice melted at 0 °C: H2O(s) → H2O(l). Now it is a puddle.",
            EventKind::Melting => {
                "Melting... ice needs a lot of heat to turn into water (latent heat)."
            }
            EventKind::Froze => {
                "Frozen! Liquid nitrogen at −196 °C (77 K) froze the water: H2O(l) → H2O(s)."
            }
            EventKind::Steam => "Hot water boils at 100 °C and turns into steam: H2O(l) → H2O(g).",
            EventKind::Smothered => {
                "CO2 foam covers it: no oxygen can reach it, so it cannot burn."
            }
            EventKind::Soaked => "Soaked: it must dry out (boil off at 100 °C) before it can burn.",
            EventKind::Chilled => {
                "Brrr! Liquid nitrogen boils away at −196 °C as it chills the surface."
            }
            EventKind::Fizzle(Fizzle::Wet) => {
                "Sizzle! The water must boil away at 100 °C before it can burn."
            }
            EventKind::Fizzle(Fizzle::NoOxygen) => {
                "No oxygen under the CO2 blanket, so the fire cannot start."
            }
            EventKind::Fizzle(Fizzle::NotFlammable) => "That does not burn.",
            EventKind::Fizzle(Fizzle::Charred) => "Already burnt: no fuel left.",
            EventKind::Fizzle(Fizzle::Heated) => {
                "Heating up... a little more heat and it will catch fire."
            }
            EventKind::Station => {
                "Water-splitting station (coming soon): 2H2O → 2H2 + O2 with electricity."
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
}

impl Obj {
    pub fn new(kind: Kind, pos: [f32; 2], half: [f32; 2]) -> Self {
        let (fuel, ice, wall, temp) = match kind {
            Kind::Bush => (60.0, 0.0, false, AMBIENT_C),
            Kind::DryHedge => (45.0, 0.0, false, AMBIENT_C),
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
    /// Fires put out by spells.
    pub extinguished: u32,
    pub ticks: u64,
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
        }
    }

    pub fn ignite(&mut self, i: usize) {
        let o = &mut self.objs[i];
        o.burning = true;
        o.temp = FLAME_C;
    }

    pub fn burning(&self) -> usize {
        self.objs.iter().filter(|o| o.burning).count()
    }

    pub fn outcome(&self) -> Outcome {
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
    pub fn apply(&mut self, i: usize, spell: Spell, events: &mut Vec<Event>) {
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
                    // A flame is about 900 °C at its tip.
                    o.temp = o.temp.max(AMBIENT_C) + 600.0;
                    if o.temp >= o.ignition_c() {
                        o.burning = true;
                        o.temp = FLAME_C;
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
                    self.extinguished += 1;
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
                    self.extinguished += 1;
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
                    self.extinguished += 1;
                } else {
                    ev(EventKind::Smothered);
                }
            }
        }
    }

    /// One fixed step of [`DT`] seconds: burning, spreading heat, drying,
    /// cooling and the CO2 blanket wearing off.
    pub fn step(&mut self, events: &mut Vec<Event>) {
        self.ticks += 1;
        let n = self.objs.len();
        // Heat each object receives from burning neighbours this step.
        let mut heat = vec![0.0f32; n];
        for (i, h) in heat.iter_mut().enumerate() {
            for &j in &self.neighbours[i] {
                if self.objs[j].burning {
                    // Closer neighbours heat more: full at 0 m, half at the edge.
                    let g = self.objs[i].gap(&self.objs[j]);
                    *h += SPREAD_HEAT * (1.0 - 0.5 * g / NEIGHBOUR_GAP);
                }
            }
        }
        for (i, o) in self.objs.iter_mut().enumerate() {
            let mut ev = |kind| events.push(Event { obj: i, kind });
            o.smothered = (o.smothered - DT).max(0.0);
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
                ev(EventKind::Ignited);
            }
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
        run(&mut w, 5.0);
        assert!(!w.objs[1].burning, "spread must be slow");
        let ev = run(&mut w, 40.0);
        assert!(w.objs[1].burning);
        assert!(ev.contains(&Event {
            obj: 1,
            kind: EventKind::Ignited
        }));
        run(&mut w, 120.0);
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
        w.ignite(3);
        assert_eq!(w.outcome(), Outcome::Lost);
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
