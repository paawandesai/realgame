//! One engine (I-052 lane one-engine): the in-world Formula Bench's judge
//! (`realgame_sim::bench::judge_with`, given this crate's `Codex`) and the
//! free-play Bench's codex agree on every spell whose formula is a codex
//! reaction, and spells whose reference is a phase change or a register
//! entry judge exactly as before. See
//! `docs/loop/items/I-052/lanes/one-engine/spec.md`.

use realgame_chem::{Codex, Need, Outcome};
use realgame_sim::bench::{self, codex_id, judge, judge_with, Code};
use realgame_sim::formula::parse_equation;
use realgame_sim::reactions::ReactionBook;
use realgame_sim::spellbook::{RefKind, SpellBook, SpellDef};
use realgame_sim::tier::Tier;

fn spells() -> Vec<SpellDef> {
    SpellBook::load(&ReactionBook::load()).spells
}

/// The codex reactions the shipped spells run, by spell id.
const MAPPED: [(&str, &str); 1] = [("fire", "methane_burns")];

#[test]
fn every_reaction_spell_maps_to_a_codex_reaction_it_can_teach() {
    let c = Codex::load();
    let book = ReactionBook::load();
    let s = spells();
    for d in &s {
        let got = codex_id(&c, d);
        let want = MAPPED.iter().find(|(id, _)| *id == d.id).map(|m| m.1);
        assert_eq!(got.as_deref(), want, "spell {}", d.id);
        if d.formula.is_reaction() {
            let id = got.unwrap_or_else(|| panic!("{}: a reaction spell not in the codex", d.id));
            let e = c.get(&id).unwrap();
            assert_eq!(e.check, "reviewed", "{id}");
            // The codex never locks a spell the spell book lets the band learn.
            assert!(e.band <= d.tier_min, "{}: codex band above tier_min", d.id);
            // The spell's reactions.json reference is the same reaction.
            assert_eq!(d.reaction.kind, RefKind::Reaction, "{}", d.id);
            let (_, eq) = book
                .reactions
                .iter()
                .find(|(r, _)| r.id == d.reaction.id)
                .unwrap();
            assert_eq!(
                realgame_chem::reduced(eq),
                realgame_chem::reduced(&parse_equation(&e.equation).unwrap()),
                "{}: {} and codex {id} differ",
                d.id,
                d.reaction.id
            );
        } else {
            // Water, Frost, CO2: a phase change or a register citation, not
            // a codex reaction. They keep the spell book's own judge.
            assert!(
                matches!(d.reaction.kind, RefKind::PhaseChange | RefKind::Register),
                "{}",
                d.id
            );
        }
    }
}

/// Every equation over the spell's own species with coefficients 1..=6.
fn coefficient_sweep(d: &SpellDef) -> Vec<String> {
    let eq = &d.formula.equation;
    let n = eq.reactants.len() + eq.products.len();
    let mut out = vec![];
    let mut coefs = vec![1u32; n];
    loop {
        let term = |i: usize, f: &str| {
            if coefs[i] == 1 {
                f.to_string()
            } else {
                format!("{}{f}", coefs[i])
            }
        };
        let l: Vec<String> = eq
            .reactants
            .iter()
            .enumerate()
            .map(|(i, t)| term(i, &t.formula))
            .collect();
        let r: Vec<String> = eq
            .products
            .iter()
            .enumerate()
            .map(|(i, t)| term(eq.reactants.len() + i, &t.formula))
            .collect();
        out.push(format!("{} -> {}", l.join(" + "), r.join(" + ")));
        let mut i = 0;
        loop {
            if i == n {
                return out;
            }
            coefs[i] += 1;
            if coefs[i] <= 6 {
                break;
            }
            coefs[i] = 1;
            i += 1;
        }
    }
}

#[test]
fn the_world_judge_and_the_codex_agree_on_every_mapped_spell() {
    let c = Codex::load();
    let s = spells();
    let mut oks = 0;
    for d in s.iter().filter(|d| codex_id(&c, d).is_some()) {
        let id = codex_id(&c, d).unwrap();
        for text in coefficient_sweep(d) {
            let eq = parse_equation(&text).unwrap();
            for band in Tier::ALL {
                let world = judge_with(&c, &s, d.slot, band, &text);
                // The free-play Bench, with the chamber supplying every need.
                let free = c.judge(&eq, &Need::ALL, band);
                if band < d.tier_min {
                    assert_eq!(world.code, Code::NotForBand, "{text:?}");
                    continue;
                }
                let codex_says_spell = free == Outcome::Known { id: id.clone() };
                assert_eq!(
                    world.ok, codex_says_spell,
                    "{} {band:?} {text:?}: {free:?}",
                    d.id
                );
                if matches!(free, Outcome::Unbalanced { .. }) {
                    assert_eq!(world.code, Code::Unbalanced, "{text:?}");
                }
                oks += u32::from(world.ok);
            }
        }
    }
    assert!(oks > 0, "the sweep reached no Ok");
}

/// The texts the sim crate's bench tests use, plus states, Unicode, term
/// order, fractions and near misses.
const TEXTS: &[&str] = &[
    "",
    "H2O",
    "H2O(l)",
    "H₂O(l)",
    "2H2O(l)",
    "OH2(l)",
    "H2O(g)",
    "HO",
    "N2",
    "N2(l)",
    "N2(g)",
    "CO2",
    "CO2(g)",
    "CO(g)",
    "CH4 + 2O2 -> CO2 + 2H2O",
    "CH₄ + 2O₂ → CO₂ + 2H₂O",
    "CH4(g) + 2O2(g) -> CO2(g) + 2H2O(l)",
    "2O2 + CH4 -> 2H2O + CO2",
    "2CH4 + 4O2 -> 2CO2 + 4H2O",
    "1/2CH4 + O2 -> 1/2CO2 + H2O",
    "CH4 + O2 + O2 -> CO2 + 2H2O",
    "CH4 + O2 -> CO2 + H2O",
    "CH4 + 2O2 -> CO + 2H2O",
    "2CH4 + 3O2 -> 2CO + 4H2O",
    "CH4 -> C + 2H2",
    "2H2 + O2 -> 2H2O",
    "CH4 + 2O2",
    "0CH4 + 2O2 -> CO2 + 2H2O",
    "H2O(l) -> H2O(g)",
    "Xx2",
    "H2 ++ O2",
];

#[test]
fn no_visible_change_for_the_shipped_spells() {
    let c = Codex::load();
    let s = spells();
    for d in &s {
        for band in Tier::ALL {
            for text in TEXTS
                .iter()
                .map(|t| t.to_string())
                .chain(coefficient_sweep(d))
            {
                assert_eq!(
                    judge_with(&c, &s, d.slot, band, &text),
                    judge(&s, d.slot, band, &text),
                    "{} {band:?} {text:?}",
                    d.id
                );
            }
        }
    }
    // A slot with no spell is still NotForBand with the codex.
    assert_eq!(
        judge_with(&c, &s, 9, Tier::Adept, "H2O").code,
        bench::Code::NotForBand
    );
}
