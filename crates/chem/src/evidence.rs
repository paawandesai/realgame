//! Evidence for every codex reaction (I-058,
//! `docs/loop/items/I-058/spec.md`): where in the real world the reaction,
//! its products and what the player sees come from.
//!
//! - `textbook`: a standard reaction found in a common, openly accessible
//!   reference (OpenStax, LibreTexts, Wikipedia, RSC Education, CLEAPSS);
//! - `data`: the outcome follows from cited data tables (the solubility
//!   rules, the reactivity series, CRC enthalpies of formation) by a stated
//!   rule; `entries` names the table entries used;
//! - `unverified`: no source traced yet. The codex hides it from play
//!   (`Codex::hidden`).
//!
//! `checked` is true only when the cited page was fetched during the audit
//! and the specific claim (`claim`) was read on it. A reference cited from
//! knowledge, with its page not fetched, stays `checked: false`.
//!
//! [`csv`] writes the per-reaction table a teacher can review,
//! `content/chem/evidence.csv` (regenerate with `scripts/evidence-csv.sh`).

use serde::{Deserialize, Serialize};

use crate::show::{ShowRecord, Shows, Side, Visible};
use crate::{Codex, CodexTier, Entry, Heat};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EvidenceKind {
    Textbook,
    Data,
    Unverified,
}

impl EvidenceKind {
    /// Textbook and data evidence make a reaction playable.
    pub fn playable(self) -> bool {
        matches!(self, EvidenceKind::Textbook | EvidenceKind::Data)
    }

    pub fn id(self) -> &'static str {
        match self {
            EvidenceKind::Textbook => "textbook",
            EvidenceKind::Data => "data",
            EvidenceKind::Unverified => "unverified",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Evidence {
    pub kind: EvidenceKind,
    /// The reference in words: work, edition, chapter or section, or the
    /// article's title. Empty only for `unverified`.
    #[serde(default)]
    pub source: String,
    /// The page, when it is on the open web.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// True only when the page was read during the audit and states `claim`.
    #[serde(default)]
    pub checked: bool,
    /// What the source states that the reaction rests on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim: Option<String>,
    /// For `data`: the table entries the outcome follows from.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entries: Vec<String>,
    /// Why an entry is unverified, or a caveat a reviewer should see.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

fn heat_word(h: Option<Heat>) -> &'static str {
    match h {
        Some(Heat::Releases) => "gives off heat",
        Some(Heat::Absorbs) => "takes in heat",
        Some(Heat::Little) => "little heat change",
        None => "not stated",
    }
}

/// The equation with each species' state, as the show record has it.
fn equation_with_states(rec: &ShowRecord) -> String {
    let side = |s: Side| {
        rec.species
            .iter()
            .filter(|x| x.side == s)
            .map(|x| {
                let c = if x.coef > 1 {
                    x.coef.to_string()
                } else {
                    String::new()
                };
                format!("{c}{}({})", x.formula, x.state.id())
            })
            .collect::<Vec<_>>()
            .join(" + ")
    };
    format!("{} -> {}", side(Side::Reactant), side(Side::Product))
}

/// What the player sees, in words, from the show record: flame, gases,
/// solids, coatings, colour changes, smoke, the note and the heat.
pub fn sees(e: &Entry, rec: &ShowRecord) -> String {
    let mut out: Vec<String> = Vec::new();
    if let Some(f) = &rec.flame {
        out.push(format!("flame: {}", f.light.described));
    }
    if rec.glow.is_some() || rec.has(Visible::Glow) {
        out.push("glows".into());
    }
    for g in rec.gases.iter().filter(|g| g.side == Side::Product) {
        let col = g
            .colour
            .as_ref()
            .filter(|c| !c.colourless())
            .map(|c| format!(", {}", c.described))
            .unwrap_or_default();
        out.push(format!("gas {}{col}", g.formula));
    }
    if rec.has(Visible::Bubbles) && !rec.gases.iter().any(|g| g.side == Side::Product) {
        out.push("bubbles".into());
    }
    for p in &rec.precipitates {
        let col = p
            .colour
            .as_ref()
            .map(|c| c.described.clone())
            .unwrap_or_else(|| "no colour given".into());
        out.push(format!("solid {} ({col})", p.formula));
    }
    for c in &rec.coatings {
        let col = c
            .colour
            .as_ref()
            .map(|c| format!(" ({})", c.described))
            .unwrap_or_default();
        out.push(format!("{}{col} coats the {}", c.formula, c.on));
    }
    for c in &rec.colour_changes {
        out.push(format!(
            "{}({}) {} {}",
            c.formula,
            c.state.id(),
            c.colour.described,
            if c.appears { "appears" } else { "goes" }
        ));
    }
    for s in &rec.smoke {
        let kind = format!("{:?}", s.kind).to_lowercase();
        let what = s
            .formula
            .as_ref()
            .map(|f| format!(" of {f}"))
            .unwrap_or_default();
        let col = s
            .colour
            .as_ref()
            .map(|c| format!(" ({})", c.described))
            .unwrap_or_default();
        out.push(format!("{kind}{what}{col}"));
    }
    for v in [Visible::Frost, Visible::Condensation] {
        if rec.has(v) {
            out.push(format!("{v:?}").to_lowercase());
        }
    }
    if let Some(n) = &rec.note {
        out.push(format!("note: {}", n.trim_end_matches('.')));
    }
    out.push(heat_word(e.heat).into());
    if let Some(f) = &rec.feel {
        out.push(format!("feels: {f}"));
    }
    out.join("; ")
}

fn field(s: &str) -> String {
    if s.contains([',', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

pub const CSV_HEADER: &str = "id,name,tier,rule,band,equation,needs,player_sees,dh_kj_per_mol,\
evidence,checked,playable,source,url,claim_or_entries,note";

/// The teacher table: one row per reaction, hidden ones included, from the
/// audit codex (`Codex::load_audit`) and its show records.
pub fn csv(codex: &Codex, shows: &Shows) -> String {
    let mut out = String::from(CSV_HEADER);
    out.push('\n');
    for e in &codex.entries {
        let rec = shows
            .get(&e.id)
            .unwrap_or_else(|| panic!("{}: no show record", e.id));
        let ev = e.evidence.as_ref();
        let kind = ev.map_or("unverified", |v| v.kind.id());
        let detail = ev
            .map(|v| {
                let mut d: Vec<String> = v.claim.iter().cloned().collect();
                d.extend(v.entries.iter().cloned());
                d.join(" | ")
            })
            .unwrap_or_default();
        let needs = e.needs.iter().map(|n| n.id()).collect::<Vec<_>>().join(" ");
        let dh = rec.dh.kj.map(|k| format!("{k:.1}")).unwrap_or_default();
        let row = [
            e.id.clone(),
            e.name.clone(),
            match e.tier {
                CodexTier::Checked => "core".into(),
                CodexTier::Rule => "rule".into(),
            },
            e.rule.clone().unwrap_or_default(),
            e.band.id().to_string(),
            equation_with_states(rec),
            needs,
            sees(e, rec),
            dh,
            kind.to_string(),
            ev.is_some_and(|v| v.checked).to_string(),
            e.playable().to_string(),
            ev.map(|v| v.source.clone()).unwrap_or_default(),
            ev.and_then(|v| v.url.clone()).unwrap_or_default(),
            detail,
            ev.and_then(|v| v.note.clone()).unwrap_or_default(),
        ];
        out.push_str(&row.iter().map(|s| field(s)).collect::<Vec<_>>().join(","));
        out.push('\n');
    }
    out
}

/// The table for the shipped data.
pub fn csv_for_content() -> String {
    let c = Codex::load_audit();
    csv(&c, &Shows::build(&c))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover::Effects;
    use realgame_sim::formula::parse_equation;
    use realgame_sim::tier::Tier;

    /// The teacher table, as committed.
    const EVIDENCE_CSV: &str = include_str!("../../../content/chem/evidence.csv");

    /// Every reaction the game offers has textbook or data evidence with a
    /// named source, and data evidence names its table entries.
    #[test]
    fn every_playable_reaction_has_evidence() {
        let c = Codex::load();
        assert!(!c.entries.is_empty());
        for e in &c.entries {
            let ev = e
                .evidence
                .as_ref()
                .unwrap_or_else(|| panic!("{}: playable with no evidence", e.id));
            assert!(ev.kind.playable(), "{}: {:?} is playable", e.id, ev.kind);
            assert!(!ev.source.trim().is_empty(), "{}: no source", e.id);
            if ev.kind == EvidenceKind::Data {
                assert!(!ev.entries.is_empty(), "{}: data with no entries", e.id);
            }
            if ev.checked {
                assert!(
                    ev.url.as_deref().is_some_and(|u| u.starts_with("https://")),
                    "{}: checked evidence names the page it was checked on",
                    e.id
                );
                assert!(ev.claim.is_some(), "{}: checked with no claim", e.id);
            }
        }
        // Every entry of the data files has an evidence field.
        let all = Codex::load_audit();
        for e in &all.entries {
            assert!(e.evidence.is_some(), "{}: no evidence field", e.id);
        }
        assert_eq!(all.entries.len(), c.entries.len() + c.hidden.len());
    }

    /// Every hidden entry of `c` is out of play: not among its entries, not
    /// found by running its reactants or judging its equation (any band, every
    /// condition), with no spell effect to save or cast.
    fn assert_out_of_play(c: &Codex) {
        let fx = Effects::load();
        let effects = fx.all(c);
        for e in &c.hidden {
            assert!(!e.playable(), "{}: hidden but playable", e.id);
            assert!(c.get(&e.id).is_none(), "{}: in the playable codex", e.id);
            let eq = parse_equation(&e.equation).unwrap();
            for band in [Tier::Sprout, Tier::Seedling, Tier::Apprentice, Tier::Adept] {
                if let Some((i, o)) = c.run(&eq.reactants, &crate::Need::ALL, band) {
                    let id = &c.entries[i].id;
                    assert_ne!(id, &e.id, "{}: runs", e.id);
                    assert!(c.get(id).unwrap().playable(), "{id}: {o:?}");
                }
                match c.judge(&eq, &crate::Need::ALL, band) {
                    crate::Outcome::Known { id }
                    | crate::Outcome::NeedsCondition { id, .. }
                    | crate::Outcome::Locked { id, .. } => {
                        assert_ne!(id, e.id, "{}: judged known", e.id);
                    }
                    crate::Outcome::Unbalanced { close, .. } => {
                        assert_ne!(close.as_deref(), Some(e.id.as_str()));
                    }
                    _ => {}
                }
            }
            assert!(
                effects.iter().all(|f| f.reaction_id != e.id),
                "{}: has an effect",
                e.id
            );
        }
        // Every effect (what can be saved and cast) is a playable reaction's.
        for f in &effects {
            assert!(c.get(&f.reaction_id).is_some_and(|e| e.playable()));
        }
    }

    /// An unverified reaction is never shown, run, judged known, given an
    /// effect, or saved, whatever the band and conditions: in the shipped
    /// codex, and with one core and one rule reaction marked unverified and
    /// one core reaction stripped of its evidence.
    #[test]
    fn no_unverified_reaction_can_be_run_or_saved() {
        let c = Codex::load();
        let all = Codex::load_audit();
        assert_eq!(
            all.entries.iter().filter(|e| !e.playable()).count(),
            c.hidden.len()
        );
        assert_out_of_play(&c);
        // The game's own copy (web.rs and the client load the same way).
        assert!(Effects::load()
            .shows()
            .records
            .iter()
            .all(|r| c.get(&r.reaction_id).is_some()));

        let unverify = |json: &str, id: &str| {
            let at = json.find(&format!("\"id\": \"{id}\"")).unwrap();
            let k = at + json[at..].find("\"kind\": \"").unwrap();
            let end = k + json[k + 9..].find('"').unwrap() + 10;
            format!("{}\"kind\": \"unverified\"{}", &json[..k], &json[end..])
        };
        let core = unverify(crate::CORE_JSON, "silver_chloride");
        let bare = core.replacen(
            "\"evidence\": {\n    \"kind\": \"textbook\"",
            "\"evidence_x\": {\n    \"kind\": \"textbook\"",
            1,
        );
        assert_ne!(bare, core, "one entry lost its evidence");
        let rules = unverify(crate::RULES_JSON, "rule_precipitate_agno3_kcl");
        let t = Codex::from_parts(&bare, Some(&rules)).unwrap();
        let gone: Vec<&str> = t.hidden.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(gone.len(), c.hidden.len() + 3, "{gone:?}");
        for id in [
            "silver_chloride",
            "rule_precipitate_agno3_kcl",
            "methane_burns",
        ] {
            assert!(gone.contains(&id), "{id} hidden");
        }
        assert_eq!(t.entries.len() + 3, c.entries.len());
        assert_out_of_play(&t);
    }

    /// `content/chem/evidence.csv` is what the generator writes today.
    #[test]
    fn the_teacher_table_is_current() {
        let want = csv_for_content();
        assert!(
            EVIDENCE_CSV == want,
            "content/chem/evidence.csv is stale: run scripts/evidence-csv.sh"
        );
        assert_eq!(
            EVIDENCE_CSV.lines().count(),
            Codex::load_audit().entries.len() + 1
        );
    }
}
