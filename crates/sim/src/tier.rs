//! Age bands for the 3D game: who may learn or craft which spell.
//!
//! Four bands, lowest first. This is not the 2D game's `game::Tier`, which has
//! three bands and silently maps unknown text to `Apprentice`; `from_id` here
//! is exact and returns `None` for anything it does not know.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Sprout,
    Seedling,
    Apprentice,
    Adept,
}

impl Tier {
    pub const ALL: [Tier; 4] = [Tier::Sprout, Tier::Seedling, Tier::Apprentice, Tier::Adept];

    /// The content id (`"sprout"`, `"seedling"`, `"apprentice"`, `"adept"`).
    pub fn id(self) -> &'static str {
        match self {
            Tier::Sprout => "sprout",
            Tier::Seedling => "seedling",
            Tier::Apprentice => "apprentice",
            Tier::Adept => "adept",
        }
    }

    /// Exact, lower-case only.
    pub fn from_id(s: &str) -> Option<Tier> {
        Tier::ALL.into_iter().find(|t| t.id() == s)
    }
}
