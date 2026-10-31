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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactions::ReactionBook;

    #[test]
    fn bands_are_ordered_lowest_first() {
        assert!(Tier::Sprout < Tier::Seedling);
        assert!(Tier::Seedling < Tier::Apprentice);
        assert!(Tier::Apprentice < Tier::Adept);
    }

    #[test]
    fn ids_round_trip_exactly() {
        for t in Tier::ALL {
            assert_eq!(Tier::from_id(t.id()), Some(t));
        }
        assert_eq!(
            Tier::ALL.map(Tier::id),
            ["sprout", "seedling", "apprentice", "adept"]
        );
        assert_eq!(Tier::from_id("Sprout"), None);
        assert_eq!(Tier::from_id(""), None);
    }

    #[test]
    fn every_reaction_tier_min_is_a_known_band() {
        let book = ReactionBook::load();
        for (r, _) in &book.reactions {
            assert!(
                Tier::from_id(&r.tier_min).is_some(),
                "reaction {} has unknown tier_min {:?}",
                r.id,
                r.tier_min
            );
        }
    }
}
