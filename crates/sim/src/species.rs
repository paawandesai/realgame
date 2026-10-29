//! What each grid cell can be made of, and the physical properties the
//! simulation uses for it.

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Species {
    /// Background air: roughly 78% N2, 21% O2. It is an oxidiser, weaker than pure O2.
    Air = 0,
    Stone,
    /// Simplified to carbon fuel (see `carbon_combustion` in the reaction table).
    Wood,
    Water,
    Ice,
    Steam,
    H2,
    O2,
    CH4,
    CO2,
    Cl2,
    /// Not a substance: the visible, glowing zone where combustion is happening.
    Flame,
    /// A stone door that opens when its trial objective is met.
    Door,
    /// Charged crystal that gives the player electricity for electrolysis.
    Crystal,
    // ---- v3 (DESIGN_V3 §4.1). Appended in this order; existing ids never
    // change, because `web/src/gl.ts` and the story hashes depend on them.
    /// Magnesium metal strip. Burns white (a Flame tagged `Emitter::MgWhite`;
    /// burn progress lives in `aux`).
    Mg,
    /// Magnesium oxide: white powder ash, inert.
    MgO,
    /// Silica sand: a powder that falls and piles. Smothers flames.
    Sand,
    /// Hydrogen peroxide solution. Decomposes slowly; fast with a catalyst.
    H2O2,
    /// Catalase/yeast foam: a light solid you can stand on. Decays back to
    /// water; its timer lives in `aux`.
    Foam,
    /// Red-cabbage indicator water; pH x100 lives in `aux`.
    Indicator,
    /// Limewater, Ca(OH)2(aq).
    Limewater,
    /// Chalk, CaCO3: a fine white precipitate.
    Chalk,
}

/// What a Flame cell is coloured by (`Grid::tag`; DESIGN_V3 §4.1). Zero is
/// none, and the story never sets one.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Emitter {
    #[default]
    None = 0,
    /// Sodium: 589.0/589.6 nm yellow (NIST ASD).
    Na,
    /// Lithium: 670.8 nm red (NIST ASD).
    Li,
    /// Potassium: 766.5/769.9 nm lilac (NIST ASD).
    K,
    /// Copper (as CuCl2): blue-green molecular bands.
    Cu,
    /// Strontium (as SrCl2): red SrOH/SrCl molecular bands.
    Sr,
    /// Burning magnesium: brilliant white.
    MgWhite,
}

impl Emitter {
    pub const ALL: [Emitter; 7] = [
        Emitter::None,
        Emitter::Na,
        Emitter::Li,
        Emitter::K,
        Emitter::Cu,
        Emitter::Sr,
        Emitter::MgWhite,
    ];

    pub fn from_u8(v: u8) -> Emitter {
        Emitter::ALL
            .get(v as usize)
            .copied()
            .unwrap_or(Emitter::None)
    }

    /// The element symbol shown on salt cards (colour is never the only cue).
    pub fn element(self) -> &'static str {
        match self {
            Emitter::None => "",
            Emitter::Na => "Na",
            Emitter::Li => "Li",
            Emitter::K => "K",
            Emitter::Cu => "Cu",
            Emitter::Sr => "Sr",
            Emitter::MgWhite => "Mg",
        }
    }
}

pub const ALL: [Species; 22] = [
    Species::Air,
    Species::Stone,
    Species::Wood,
    Species::Water,
    Species::Ice,
    Species::Steam,
    Species::H2,
    Species::O2,
    Species::CH4,
    Species::CO2,
    Species::Cl2,
    Species::Flame,
    Species::Door,
    Species::Crystal,
    Species::Mg,
    Species::MgO,
    Species::Sand,
    Species::H2O2,
    Species::Foam,
    Species::Indicator,
    Species::Limewater,
    Species::Chalk,
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Solid,
    Liquid,
    Gas,
    Flame,
}

pub struct Props {
    pub name: &'static str,
    pub formula: &'static str,
    pub phase: Phase,
    /// g/mol. Decides whether a gas rises or sinks in air (air is about 29).
    pub molar_mass: f32,
    /// Relative heat capacity per cell; water's is high, so it soaks up heat.
    pub heat_capacity: f32,
    /// Relative thermal conductivity, 0..=0.25 for numerical stability.
    pub conductivity: f32,
    // How each species looks (Okabe-Ito colours and textures) is presentation
    // and lives in `palette.rs`.
}

pub const AIR_MOLAR_MASS: f32 = 28.97;

impl Species {
    pub fn from_u8(v: u8) -> Species {
        ALL.get(v as usize).copied().unwrap_or(Species::Air)
    }

    /// Molar masses are real values; heat capacity and conductivity are relative
    /// tuning values, ordered the way the real materials are.
    #[inline]
    pub fn props(self) -> &'static Props {
        &PROPS[self as usize]
    }

    #[inline]
    pub fn phase(self) -> Phase {
        PHASE[self as usize]
    }

    #[inline]
    pub fn is_gas(self) -> bool {
        matches!(self.phase(), Phase::Gas)
    }

    /// Something a player could walk on or bump into.
    #[inline]
    pub fn is_solid(self) -> bool {
        matches!(self.phase(), Phase::Solid)
    }

    /// Species with a per-cell rule in the chemistry pass (phase changes,
    /// burning); everything else only reacts when one of these touches it.
    #[inline]
    pub fn reacts_alone(self) -> bool {
        matches!(
            self,
            Species::Water
                | Species::Ice
                | Species::Steam
                | Species::Wood
                | Species::H2
                | Species::CH4
                | Species::Flame
        )
    }

    /// Solids that fall and pile like sand (Sand, MgO, Chalk). They are
    /// `Phase::Solid` everywhere else (walkable, stop bolts); the grid's
    /// movement pass gives them a powder rule (Track A).
    #[inline]
    pub fn is_powder(self) -> bool {
        matches!(self, Species::Sand | Species::MgO | Species::Chalk)
    }

    /// Added in v3 (id 14 and up). The story never places these.
    #[inline]
    pub fn is_v3(self) -> bool {
        self as u8 >= Species::Mg as u8
    }

    #[inline]
    pub fn conductivity(self) -> f32 {
        CONDUCT[self as usize]
    }

    #[inline]
    pub fn heat_capacity(self) -> f32 {
        HEATCAP[self as usize]
    }

    /// Gases that can supply oxygen for burning.
    pub fn oxidiser_strength(self) -> f32 {
        match self {
            Species::O2 => 1.0,
            Species::Air => 0.21,
            _ => 0.0,
        }
    }

    /// The reagent jar a gathered cell goes into.
    pub fn jar(self) -> Option<&'static str> {
        match self {
            Species::Water | Species::Ice | Species::Steam => Some("H2O"),
            Species::H2 => Some("H2"),
            Species::O2 => Some("O2"),
            Species::CH4 => Some("CH4"),
            Species::CO2 => Some("CO2"),
            _ => None,
        }
    }

    /// The species a released jar becomes in the world.
    pub fn from_jar(formula: &str) -> Option<Species> {
        match formula {
            "H2O" => Some(Species::Water),
            "H2" => Some(Species::H2),
            "O2" => Some(Species::O2),
            "CH4" => Some(Species::CH4),
            "CO2" => Some(Species::CO2),
            "Cl2" => Some(Species::Cl2),
            _ => None,
        }
    }
}

const fn p(
    name: &'static str,
    formula: &'static str,
    phase: Phase,
    molar_mass: f32,
    heat_capacity: f32,
    conductivity: f32,
) -> Props {
    Props {
        name,
        formula,
        phase,
        molar_mass,
        heat_capacity,
        conductivity,
    }
}

/// Indexed by `Species as usize`; the order must match the enum (tested).
static PROPS: [Props; ALL.len()] = {
    use Phase::*;
    [
        p("air", "", Gas, AIR_MOLAR_MASS, 1.0, 0.04),
        p("stone", "", Solid, 60.0, 2.0, 0.10),
        p("wood", "C", Solid, 12.0, 2.0, 0.03),
        p("water", "H2O", Liquid, 18.02, 4.0, 0.15),
        p("ice", "H2O", Solid, 18.02, 2.0, 0.20),
        p("steam", "H2O", Gas, 18.02, 2.0, 0.05),
        p("hydrogen", "H2", Gas, 2.016, 1.5, 0.12),
        p("oxygen", "O2", Gas, 32.00, 1.0, 0.04),
        p("methane", "CH4", Gas, 16.04, 1.5, 0.05),
        p("carbon dioxide", "CO2", Gas, 44.01, 1.0, 0.03),
        p("chlorine", "Cl2", Gas, 70.90, 1.0, 0.02),
        p("flame", "", Flame, 0.0, 1.0, 0.15),
        p("door", "", Solid, 60.0, 2.0, 0.02),
        p("crystal", "", Solid, 60.0, 2.0, 0.02),
        // v3. Molar masses are real (IUPAC atomic weights); heat capacity
        // and conductivity are first-cut tuning values for Track A.
        p("magnesium", "Mg", Solid, 24.305, 1.0, 0.25),
        p("magnesium oxide", "MgO", Solid, 40.30, 1.5, 0.05),
        p("sand", "SiO2", Solid, 60.08, 1.5, 0.03),
        p("hydrogen peroxide", "H2O2", Liquid, 34.01, 3.5, 0.12),
        p("foam", "", Solid, 18.02, 1.0, 0.02),
        p("indicator", "", Liquid, 18.02, 4.0, 0.15),
        p("limewater", "Ca(OH)2", Liquid, 18.02, 4.0, 0.15),
        p("chalk", "CaCO3", Solid, 100.09, 1.5, 0.04),
    ]
};

/// Hot-loop copies of single columns of `PROPS`, so the grid passes touch one
/// small array instead of a whole `Props` row.
static PHASE: [Phase; ALL.len()] = {
    let mut out = [Phase::Gas; ALL.len()];
    let mut i = 0;
    while i < ALL.len() {
        out[i] = PROPS[i].phase;
        i += 1;
    }
    out
};

static CONDUCT: [f32; ALL.len()] = {
    let mut out = [0.0; ALL.len()];
    let mut i = 0;
    while i < ALL.len() {
        out[i] = PROPS[i].conductivity;
        i += 1;
    }
    out
};

static HEATCAP: [f32; ALL.len()] = {
    let mut out = [0.0; ALL.len()];
    let mut i = 0;
    while i < ALL.len() {
        out[i] = PROPS[i].heat_capacity;
        i += 1;
    }
    out
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_line_up_with_the_enum() {
        let names = [
            "air",
            "stone",
            "wood",
            "water",
            "ice",
            "steam",
            "hydrogen",
            "oxygen",
            "methane",
            "carbon dioxide",
            "chlorine",
            "flame",
            "door",
            "crystal",
            "magnesium",
            "magnesium oxide",
            "sand",
            "hydrogen peroxide",
            "foam",
            "indicator",
            "limewater",
            "chalk",
        ];
        for (i, s) in ALL.iter().enumerate() {
            assert_eq!(*s as usize, i);
            assert_eq!(Species::from_u8(i as u8), *s);
            assert_eq!(s.props().name, names[i]);
            assert_eq!(s.phase(), s.props().phase);
            assert_eq!(s.conductivity(), s.props().conductivity);
            assert_eq!(s.heat_capacity(), s.props().heat_capacity);
        }
        assert!(Species::Stone.is_solid() && Species::H2.is_gas());
        assert!(!Species::Water.is_solid() && !Species::Water.is_gas());
    }
}

#[cfg(test)]
mod gpu_ids {
    use super::*;

    /// `web/src/gl.ts` hard-codes these ids and the solid set in its shaders.
    #[test]
    fn ids_used_by_the_webgl_shaders() {
        assert_eq!(Species::Flame as u8, 11);
        assert_eq!(Species::Crystal as u8, 13);
        assert_eq!(Species::Mg as u8, 14);
        assert_eq!(Species::Chalk as u8, 21);
        let solid: Vec<u8> = ALL
            .iter()
            .filter(|s| s.is_solid())
            .map(|&s| s as u8)
            .collect();
        assert_eq!(solid, vec![1, 2, 4, 12, 13, 14, 15, 16, 18, 21]);
    }
}
