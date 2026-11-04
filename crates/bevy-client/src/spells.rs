//! The spells as data: `content/spells.json`, loaded and validated through
//! `realgame_sim` once per run.
//!
//! Bevy-free. `book()` is the only place the client parses the reaction
//! table or the spell book, and it does so once (`OnceLock`). The player text
//! that used to be string literals in `chem.rs` comes from here.

use std::sync::OnceLock;

use realgame_sim::reactions::ReactionBook;
use realgame_sim::spellbook::{SpellBook, SpellDef};

use crate::chem::Spell;

pub struct ClientSpells {
    /// Kept for later items, which show reaction text.
    #[allow(dead_code)]
    pub reactions: ReactionBook,
    /// `spells.spells[i]` is the definition of `Spell::ALL[i]` (checked at init).
    spells: SpellBook,
    /// The shared chemistry codex (`content/chem/core.json`): the bench's
    /// judge asks it about reaction spells, so the world and the free-play
    /// Bench agree (I-052 lane one-engine).
    pub codex: realgame_chem::Codex,
}

impl ClientSpells {
    /// Builds the client's book from validated content, and checks that each
    /// slot's `effect` is the chemistry rule the client numbers it as.
    fn new(reactions: ReactionBook, spells: SpellBook) -> ClientSpells {
        assert_eq!(
            spells.spells.len(),
            Spell::ALL.len(),
            "content/spells.json has {} spells; the client knows {}",
            spells.spells.len(),
            Spell::ALL.len()
        );
        for (i, def) in spells.spells.iter().enumerate() {
            let effect = Spell::from_effect_id(&def.effect);
            assert!(
                effect.is_some(),
                "spell {}: effect {:?} is not a chemistry rule the client knows",
                def.id,
                def.effect
            );
            assert_eq!(
                usize::from(def.slot),
                i + 1,
                "spell {}: slot {} out of order",
                def.id,
                def.slot
            );
            assert_eq!(
                effect,
                Some(Spell::ALL[i]),
                "spell {}: slot {} must have effect {}",
                def.id,
                def.slot,
                Spell::ALL[i].effect_id()
            );
        }
        let codex = realgame_chem::Codex::load();
        for def in &spells.spells {
            assert!(
                !def.formula.is_reaction() || realgame_sim::bench::codex_id(&codex, def).is_some(),
                "spell {}: formula has no entry in content/chem/core.json",
                def.id
            );
        }
        ClientSpells {
            reactions,
            spells,
            codex,
        }
    }

    /// The bench's judge (`realgame_sim::bench::judge_with`) with the shared
    /// codex. The only judge the client calls.
    pub fn judge(
        &self,
        spells: &[SpellDef],
        slot: u8,
        band: realgame_sim::tier::Tier,
        text: &str,
    ) -> realgame_sim::bench::Verdict {
        realgame_sim::bench::judge_with(&self.codex, spells, slot, band, text)
    }

    /// The definition of `s` (its index in `Spell::ALL`).
    pub fn def(&self, s: Spell) -> &SpellDef {
        let i = Spell::ALL
            .iter()
            .position(|x| *x == s)
            .expect("every Spell is in Spell::ALL");
        &self.spells.spells[i]
    }

    /// Every spell, in slot order.
    #[allow(dead_code)]
    pub fn all(&self) -> &[SpellDef] {
        &self.spells.spells
    }
}

/// The spell book, parsed and validated on first use.
pub fn book() -> &'static ClientSpells {
    static BOOK: OnceLock<ClientSpells> = OnceLock::new();
    BOOK.get_or_init(|| {
        let reactions = ReactionBook::load();
        let spells = SpellBook::load(&reactions);
        ClientSpells::new(reactions, spells)
    })
}

impl Spell {
    pub fn def(self) -> &'static SpellDef {
        book().def(self)
    }

    /// The permanent spell id (`fire`, `water`, `frost`, `co2`): the only
    /// key to save or send. Not `effect_id()`, which is `foam` for CO2.
    #[allow(dead_code)]
    pub fn spell_id(self) -> &'static str {
        &self.def().id
    }

    pub fn name(self) -> &'static str {
        &self.def().name
    }

    /// The formula text shown in game state (`label` in spells.json).
    pub fn formula(self) -> &'static str {
        &self.def().label
    }

    /// The caption shown when the spell is cast.
    pub fn cast_caption(self) -> &'static str {
        &self.def().cast_caption
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_map_to_the_chemistry_rules_in_order() {
        let b = book();
        for (i, s) in Spell::ALL.into_iter().enumerate() {
            assert_eq!(b.def(s).slot as usize, i + 1);
        }
        assert_eq!(
            Spell::ALL.map(Spell::spell_id),
            ["fire", "water", "frost", "co2"]
        );
        assert_eq!(Spell::Foam.name(), "CO₂");
        assert_eq!(Spell::Water.formula(), "H₂O absorbs heat");
        assert_eq!(b.all().len(), 4);
    }
}
