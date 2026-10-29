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
}

pub const ALL: [Species; 14] = [
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
    pub color: [u8; 3],
}

pub const AIR_MOLAR_MASS: f32 = 28.97;

impl Species {
    pub fn from_u8(v: u8) -> Species {
        ALL.get(v as usize).copied().unwrap_or(Species::Air)
    }

    /// Molar masses are real values; heat capacity and conductivity are relative
    /// tuning values, ordered the way the real materials are.
    pub fn props(self) -> Props {
        use Phase::*;
        let p = |name, formula, phase, molar_mass, heat_capacity, conductivity, color| Props {
            name,
            formula,
            phase,
            molar_mass,
            heat_capacity,
            conductivity,
            color,
        };
        match self {
            Species::Air => p("air", "", Gas, AIR_MOLAR_MASS, 1.0, 0.04, [196, 222, 240]),
            Species::Stone => p("stone", "", Solid, 60.0, 2.0, 0.10, [118, 112, 108]),
            Species::Wood => p("wood", "C", Solid, 12.0, 2.0, 0.03, [122, 84, 48]),
            Species::Water => p("water", "H2O", Liquid, 18.02, 4.0, 0.15, [52, 118, 210]),
            Species::Ice => p("ice", "H2O", Solid, 18.02, 2.0, 0.20, [190, 232, 250]),
            Species::Steam => p("steam", "H2O", Gas, 18.02, 2.0, 0.05, [232, 236, 240]),
            Species::H2 => p("hydrogen", "H2", Gas, 2.016, 1.5, 0.12, [236, 200, 236]),
            Species::O2 => p("oxygen", "O2", Gas, 32.00, 1.0, 0.04, [170, 226, 236]),
            Species::CH4 => p("methane", "CH4", Gas, 16.04, 1.5, 0.05, [196, 230, 170]),
            Species::CO2 => p(
                "carbon dioxide",
                "CO2",
                Gas,
                44.01,
                1.0,
                0.03,
                [150, 156, 162],
            ),
            Species::Cl2 => p("chlorine", "Cl2", Gas, 70.90, 1.0, 0.02, [206, 222, 92]),
            Species::Flame => p("flame", "", Flame, 0.0, 1.0, 0.15, [255, 160, 40]),
            Species::Door => p("door", "", Solid, 60.0, 2.0, 0.02, [84, 76, 92]),
            Species::Crystal => p("crystal", "", Solid, 60.0, 2.0, 0.02, [168, 110, 240]),
        }
    }

    pub fn phase(self) -> Phase {
        self.props().phase
    }

    pub fn is_gas(self) -> bool {
        self.phase() == Phase::Gas
    }

    /// Something a player could walk on or bump into.
    pub fn is_solid(self) -> bool {
        self.phase() == Phase::Solid
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
