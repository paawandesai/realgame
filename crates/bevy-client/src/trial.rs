//! The Mastery trial and keep-it-sharp (I-030): the question bank and the
//! trial desk's flow.
//!
//! Pure Rust with no Bevy types and no crate imports, so it is tested with
//! `rustc --edition 2021 --test src/trial.rs` as well as `cargo test`. The
//! glue (judging builds, XP, mastery) lives in `bench.rs`, which calls the
//! `progress::Book` methods; this file never touches progress. The page is
//! `web/trial.js`. See `docs/loop/items/I-030/spec.md`, wave 14 revision.
//!
//! A trial: build the spell 3 times from memory (misses never cost), then
//! answer one "what happens?" question for the band. A wrong answer shows
//! the right one and why, then "Ask me again" with the choices in a new
//! order, so every player can finish. Keep it sharp: one build from memory
//! on a later day for a small bonus.

#![allow(dead_code)]

/// Correct builds a Mastery trial needs.
pub const BUILDS: u8 = 3;

/// One choice of a question. `id` is `"a"`, `"b"` or `"c"`; `icon` names a
/// Sprout picture in `ICONS` (empty above Sprout).
#[derive(Debug)]
pub struct Choice {
    pub id: &'static str,
    pub label: &'static str,
    pub icon: &'static str,
    pub correct: bool,
}

/// One "what happens?" question: one per spell and band.
/// `spell` is the progress index (0 fire, 1 water, 2 frost, 3 co2).
#[derive(Debug)]
pub struct Question {
    pub id: &'static str,
    pub spell: usize,
    pub band: &'static str,
    pub prompt: &'static str,
    pub choices: [Choice; 3],
    pub why: &'static str,
}

const fn c(id: &'static str, label: &'static str, icon: &'static str, correct: bool) -> Choice {
    Choice {
        id,
        label,
        icon,
        correct,
    }
}

/// Every Sprout picture id (drawn in `web/trial.js` `PICTURES`).
pub const ICONS: [&str; 7] = [
    "fire",
    "fire_out",
    "fire_big",
    "ice",
    "balloon_big",
    "balloon_small",
    "candle_out",
];

/// The 11 questions (spec section 3). Sources and verdicts are in
/// `content/science-sources.md`.
pub const QUESTIONS: [Question; 11] = [
    Question {
        id: "water_sprout",
        spell: 1,
        band: "sprout",
        prompt: "Lots of water goes on a campfire. What happens?",
        choices: [
            c("a", "The fire goes out", "fire_out", true),
            c("b", "The fire gets bigger", "fire_big", false),
            c("c", "The fire turns to ice", "ice", false),
        ],
        why: "Water soaks up the fire's heat. The wood cools down and stops burning.",
    },
    Question {
        id: "frost_sprout",
        spell: 2,
        band: "sprout",
        prompt: "A balloon goes into liquid nitrogen. What happens?",
        choices: [
            c("a", "It gets bigger", "balloon_big", false),
            c("b", "It catches fire", "fire", false),
            c("c", "It gets small", "balloon_small", true),
        ],
        why: "The air inside gets very cold. Cold air takes up less space.",
    },
    Question {
        id: "co2_sprout",
        spell: 3,
        band: "sprout",
        prompt: "CO₂ is poured into a jar with a candle in it. What happens?",
        choices: [
            c("a", "The flame gets bigger", "fire_big", false),
            c("b", "The flame goes out", "candle_out", true),
            c("c", "The candle turns to ice", "ice", false),
        ],
        why: "CO₂ is heavier than air. It fills the jar and keeps oxygen away from the flame.",
    },
    Question {
        id: "fire_seedling",
        spell: 0,
        band: "seedling",
        prompt: "A cold, dry pot lid is held over a methane flame. What will you see on the lid?",
        choices: [
            c("a", "Drops of water", "", true),
            c("b", "Nothing: it stays dry", "", false),
            c("c", "Little bits of ice", "", false),
        ],
        why: "Burning methane makes water: CH₄ + 2O₂ → CO₂ + 2H₂O. The water vapour cools on the lid and turns into drops.",
    },
    Question {
        id: "water_seedling",
        spell: 1,
        band: "seedling",
        prompt: "Water is poured on a pan of hot coals. What will you see?",
        choices: [
            c("a", "Steam rises and the coals cool down", "", true),
            c("b", "The coals get hotter", "", false),
            c("c", "The water turns to ice", "", false),
        ],
        why: "The coals heat the water until it boils into steam. That heat leaves the coals, so they cool.",
    },
    Question {
        id: "frost_seedling",
        spell: 2,
        band: "seedling",
        prompt: "A balloon shrank in liquid nitrogen. It is taken out into a warm room. What will happen?",
        choices: [
            c("a", "It stays small forever", "", false),
            c("b", "It grows back to its size", "", true),
            c("c", "It catches fire", "", false),
        ],
        why: "As the air inside warms up, it takes up more space again.",
    },
    Question {
        id: "co2_seedling",
        spell: 3,
        band: "seedling",
        prompt: "CO₂ is poured slowly into a box with three lit candles: low, middle and high. Which goes out first?",
        choices: [
            c("a", "The low candle", "", true),
            c("b", "The high candle", "", false),
            c("c", "None of them", "", false),
        ],
        why: "CO₂ is heavier than air. It fills the box from the bottom up and reaches the low flame first.",
    },
    Question {
        id: "fire_apprentice",
        spell: 0,
        band: "apprentice",
        prompt: "A cold, dry beaker is held upside down over a methane flame, and drops form inside it. Which explanation is best?",
        choices: [
            c("a", "The heat pulls water out of the glass.", "", false),
            c(
                "b",
                "Combustion makes water, CH₄ + 2O₂ → CO₂ + 2H₂O, and the H₂O(g) condenses on the cold glass.",
                "",
                true,
            ),
            c("c", "The flame turns oxygen into water on its own.", "", false),
        ],
        why: "Every H atom in that water came from the methane. Water vapour turns to liquid when it cools.",
    },
    Question {
        id: "water_apprentice",
        spell: 1,
        band: "apprentice",
        prompt: "A paper cup full of water is held over a candle, and the paper does not burn while water is in it. Which explanation is best?",
        choices: [
            c("a", "Wet paper is not a fuel.", "", false),
            c("b", "Flames cannot touch anything that holds water.", "", false),
            c(
                "c",
                "The water takes in the heat and stays at 100 °C or below, so the paper stays far below the temperature it needs to burn.",
                "",
                true,
            ),
        ],
        why: "Heat flows from the paper into the water. While the water boils at 100 °C, the paper beside it cannot get much hotter.",
    },
    Question {
        id: "frost_apprentice",
        spell: 2,
        band: "apprentice",
        prompt: "Liquid nitrogen poured on a table makes a white fog. Which explanation is best?",
        choices: [
            c("a", "The nitrogen is burning.", "", false),
            c("b", "The cold turns water vapour in the air into tiny drops.", "", true),
            c("c", "The table is smoking because it is burning.", "", false),
        ],
        why: "The fog is water from the air, cooled until it condenses into tiny drops.",
    },
    Question {
        id: "co2_apprentice",
        spell: 3,
        band: "apprentice",
        prompt: "A CO₂ extinguisher puts out a small fire. Which explanation is best?",
        choices: [
            c("a", "CO₂ reacts with the fuel and makes it fireproof.", "", false),
            c(
                "b",
                "It lowers the oxygen around the flame below what burning needs, and it also takes in heat.",
                "",
                true,
            ),
            c("c", "CO₂ freezes the flame solid.", "", false),
        ],
        why: "CO₂ puts fire out by lowering the oxygen concentration, and it also absorbs heat from the flame.",
    },
];

/// The question for a spell at a band. `"adept"` reads `"apprentice"`;
/// Fire at Sprout (not learnable there) and unknown bands have none.
pub fn question(spell: usize, band: &str) -> Option<&'static Question> {
    let band = if band == "adept" { "apprentice" } else { band };
    QUESTIONS
        .iter()
        .find(|q| q.spell == spell && q.band == band)
}

/// The display order of the 3 choices: `[0, 1, 2]` rotated left by
/// `asks % 3`, so "Ask me again" moves every choice.
pub fn order(asks: u8) -> [usize; 3] {
    let k = usize::from(asks % 3);
    [k % 3, (k + 1) % 3, (k + 2) % 3]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Master,
    Sharp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Build,
    Ask,
    Answered,
    Done,
}

/// Why `Desk::start` refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refuse {
    /// A trial is already open.
    Open,
    /// Master needs Practised; Sharp needs Mastered.
    Stage,
    NotLearnable,
    /// Sharp: not ready today.
    NotReady,
    /// No question for this spell at this band (Fire at Sprout).
    NoQuestion,
}

/// What a judged build did to the trial.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Built {
    /// Not in the build phase, or another spell.
    Ignored,
    /// Wrong: nothing lost.
    Miss,
    /// Correct build `n` (1 or 2) of a Mastery trial.
    Counted(u8),
    /// The 3rd correct build: the question comes next.
    Ask,
    /// Keep it sharp: the one build was correct.
    SharpDone,
}

#[derive(Clone, Debug)]
struct Trial {
    spell: usize,
    kind: Kind,
    phase: Phase,
    built: u8,
    asks: u8,
    misses: u32,
    right: Option<bool>,
    mastered: bool,
    band: String,
}

/// The one open trial, if any.
#[derive(Default, Debug)]
pub struct Desk {
    t: Option<Trial>,
}

impl Desk {
    /// Opens a trial. Master: Practised (`stage == 2`), learnable, and a
    /// question exists for the band. Sharp: Mastered (`stage == 3`) and
    /// `sharp_ready`. Refuses while one is open.
    pub fn start(
        &mut self,
        spell: usize,
        kind: Kind,
        stage: u8,
        learnable: bool,
        sharp_ready: bool,
        band: &str,
    ) -> Result<(), Refuse> {
        if self.t.is_some() {
            return Err(Refuse::Open);
        }
        match kind {
            Kind::Master => {
                if stage != 2 {
                    return Err(Refuse::Stage);
                }
                if !learnable {
                    return Err(Refuse::NotLearnable);
                }
                if question(spell, band).is_none() {
                    return Err(Refuse::NoQuestion);
                }
            }
            Kind::Sharp => {
                if stage != 3 {
                    return Err(Refuse::Stage);
                }
                if !learnable {
                    return Err(Refuse::NotLearnable);
                }
                if !sharp_ready {
                    return Err(Refuse::NotReady);
                }
            }
        }
        self.t = Some(Trial {
            spell,
            kind,
            phase: Phase::Build,
            built: 0,
            asks: 0,
            misses: 0,
            right: None,
            mastered: false,
            band: band.to_string(),
        });
        Ok(())
    }

    /// A judged build for `spell`. Only counts in the build phase of that
    /// spell's trial; a miss never lowers `built`.
    pub fn on_build(&mut self, spell: usize, correct: bool) -> Built {
        let Some(t) = self.t.as_mut() else {
            return Built::Ignored;
        };
        if t.phase != Phase::Build || t.spell != spell {
            return Built::Ignored;
        }
        if !correct {
            t.misses = t.misses.saturating_add(1);
            return Built::Miss;
        }
        match t.kind {
            Kind::Sharp => {
                t.phase = Phase::Done;
                Built::SharpDone
            }
            Kind::Master => {
                t.built = (t.built + 1).min(BUILDS);
                if t.built >= BUILDS {
                    t.phase = Phase::Ask;
                    Built::Ask
                } else {
                    Built::Counted(t.built)
                }
            }
        }
    }

    /// The player's answer (`"a"`, `"b"` or `"c"`), only while asking.
    /// `Some(right)`, and the phase becomes Answered; anything else changes
    /// nothing and is `None`.
    pub fn on_answer(&mut self, choice: &str) -> Option<bool> {
        let t = self.t.as_mut()?;
        if t.phase != Phase::Ask || t.kind != Kind::Master {
            return None;
        }
        let q = question(t.spell, &t.band)?;
        let ch = q.choices.iter().find(|ch| ch.id == choice)?;
        t.right = Some(ch.correct);
        t.phase = Phase::Answered;
        Some(ch.correct)
    }

    /// "Ask me again" after a wrong answer: back to asking, choices moved.
    pub fn ask_again(&mut self) -> bool {
        match self.t.as_mut() {
            Some(t) if t.phase == Phase::Answered && t.right == Some(false) => {
                t.phase = Phase::Ask;
                t.asks = t.asks.wrapping_add(1);
                t.right = None;
                true
            }
            _ => false,
        }
    }

    /// After a right answer: Done, recording whether the spell is Mastered.
    pub fn finish(&mut self, mastered: bool) {
        if let Some(t) = self.t.as_mut() {
            if t.phase == Phase::Answered && t.right == Some(true) {
                t.phase = Phase::Done;
                t.mastered = mastered;
            }
        }
    }

    pub fn close(&mut self) {
        self.t = None;
    }

    pub fn open_spell(&self) -> Option<usize> {
        self.t.as_ref().map(|t| t.spell)
    }

    pub fn phase(&self) -> Option<Phase> {
        self.t.as_ref().map(|t| t.phase)
    }

    pub fn kind(&self) -> Option<Kind> {
        self.t.as_ref().map(|t| t.kind)
    }

    /// `"null"`, or the open trial for `window.__trialState.trial`. The
    /// correct id and the `why` line stay `null` until the player answers.
    pub fn state_json(&self) -> String {
        let Some(t) = self.t.as_ref() else {
            return "null".to_string();
        };
        let kind = match t.kind {
            Kind::Master => "master",
            Kind::Sharp => "sharp",
        };
        let phase = match t.phase {
            Phase::Build => "build",
            Phase::Ask => "ask",
            Phase::Answered => "answered",
            Phase::Done => "done",
        };
        let right = match t.right {
            None => "null",
            Some(true) => "true",
            Some(false) => "false",
        };
        let q = match (t.kind, t.phase, question(t.spell, &t.band)) {
            (Kind::Master, p, Some(q)) if p != Phase::Build => {
                let shown = matches!(p, Phase::Answered | Phase::Done);
                let choices: Vec<String> = order(t.asks)
                    .iter()
                    .map(|&k| {
                        let ch = &q.choices[k];
                        format!(
                            "{{\"id\":\"{}\",\"label\":\"{}\",\"icon\":\"{}\"}}",
                            ch.id,
                            esc(ch.label),
                            ch.icon
                        )
                    })
                    .collect();
                let answer = if shown {
                    q.choices
                        .iter()
                        .find(|ch| ch.correct)
                        .map(|ch| format!("\"{}\"", ch.id))
                        .unwrap_or_else(|| "null".into())
                } else {
                    "null".into()
                };
                let why = if shown {
                    format!("\"{}\"", esc(q.why))
                } else {
                    "null".into()
                };
                format!(
                    "{{\"id\":\"{}\",\"prompt\":\"{}\",\"choices\":[{}],\"answer\":{},\"why\":{}}}",
                    q.id,
                    esc(q.prompt),
                    choices.join(","),
                    answer,
                    why
                )
            }
            _ => "null".into(),
        };
        format!(
            "{{\"spell\":{},\"slot\":{},\"kind\":\"{}\",\"phase\":\"{}\",\"built\":{},\"need\":{},\"misses\":{},\"asks\":{},\"right\":{},\"mastered\":{},\"q\":{}}}",
            t.spell,
            t.spell + 1,
            kind,
            phase,
            t.built,
            BUILDS,
            t.misses,
            t.asks,
            right,
            t.mastered,
            q
        )
    }
}

/// A JSON string body: escapes `"`, `\` and control characters.
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small JSON checker (no serde in a plain `rustc` test).
    fn json_value(s: &[u8], mut i: usize) -> Option<usize> {
        let ws = |s: &[u8], mut i: usize| {
            while i < s.len() && s[i].is_ascii_whitespace() {
                i += 1;
            }
            i
        };
        i = ws(s, i);
        match *s.get(i)? {
            b'[' => {
                i = ws(s, i + 1);
                if s.get(i) == Some(&b']') {
                    return Some(i + 1);
                }
                loop {
                    i = ws(s, json_value(s, i)?);
                    match s.get(i)? {
                        b',' => i += 1,
                        b']' => return Some(i + 1),
                        _ => return None,
                    }
                }
            }
            b'{' => {
                i = ws(s, i + 1);
                if s.get(i) == Some(&b'}') {
                    return Some(i + 1);
                }
                loop {
                    i = ws(s, i);
                    if s.get(i) != Some(&b'"') {
                        return None;
                    }
                    i = ws(s, json_value(s, i)?);
                    if s.get(i) != Some(&b':') {
                        return None;
                    }
                    i = ws(s, json_value(s, i + 1)?);
                    match s.get(i)? {
                        b',' => i += 1,
                        b'}' => return Some(i + 1),
                        _ => return None,
                    }
                }
            }
            b'"' => {
                i += 1;
                while *s.get(i)? != b'"' {
                    if s[i] < 0x20 {
                        return None;
                    }
                    if s[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
                Some(i + 1)
            }
            b't' if s[i..].starts_with(b"true") => Some(i + 4),
            b'f' if s[i..].starts_with(b"false") => Some(i + 5),
            b'n' if s[i..].starts_with(b"null") => Some(i + 4),
            b'0'..=b'9' => {
                while i < s.len() && s[i].is_ascii_digit() {
                    i += 1;
                }
                Some(i)
            }
            _ => None,
        }
    }

    fn parses(s: &str) -> bool {
        json_value(s.as_bytes(), 0) == Some(s.len())
    }

    fn correct_id(spell: usize, band: &str) -> &'static str {
        question(spell, band)
            .unwrap()
            .choices
            .iter()
            .find(|c| c.correct)
            .unwrap()
            .id
    }

    fn wrong_id(spell: usize, band: &str) -> &'static str {
        question(spell, band)
            .unwrap()
            .choices
            .iter()
            .find(|c| !c.correct)
            .unwrap()
            .id
    }

    // R4. The bank.
    #[test]
    fn r4_bank() {
        assert_eq!(QUESTIONS.len(), 11);
        let mut ids: Vec<&str> = QUESTIONS.iter().map(|q| q.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 11, "ids are unique");
        for spell in 0..4 {
            for band in ["sprout", "seedling", "apprentice"] {
                let want = !(spell == 0 && band == "sprout");
                let q = question(spell, band);
                assert_eq!(q.is_some(), want, "{spell} {band}");
                if let Some(q) = q {
                    assert_eq!((q.spell, q.band), (spell, band));
                    assert_eq!(
                        q.id,
                        format!("{}_{}", ["fire", "water", "frost", "co2"][spell], band)
                    );
                }
            }
            let adept = question(spell, "adept").map(|q| q.id);
            assert_eq!(adept, question(spell, "apprentice").map(|q| q.id));
            assert!(question(spell, "toddler").is_none());
            assert!(question(spell, "").is_none());
        }
        assert!(question(0, "sprout").is_none());
        assert!(question(4, "seedling").is_none());
        let text_ok = |s: &str| {
            let n = s.chars().count();
            (1..=220).contains(&n) && !s.chars().any(|c| c == '"' || c == '\\' || c.is_control())
        };
        let mut correct_letters = Vec::new();
        for q in &QUESTIONS {
            assert_eq!(
                q.choices.iter().filter(|c| c.correct).count(),
                1,
                "{}",
                q.id
            );
            let mut cids: Vec<&str> = q.choices.iter().map(|c| c.id).collect();
            cids.sort();
            assert_eq!(cids, ["a", "b", "c"], "{}", q.id);
            assert!(text_ok(q.prompt), "{} prompt", q.id);
            assert!(text_ok(q.why), "{} why", q.id);
            for ch in &q.choices {
                assert!(text_ok(ch.label), "{} {}", q.id, ch.id);
                if q.band == "sprout" {
                    assert!(
                        ICONS.contains(&ch.icon),
                        "{} {} icon {:?}",
                        q.id,
                        ch.id,
                        ch.icon
                    );
                } else {
                    assert_eq!(ch.icon, "", "{} {}", q.id, ch.id);
                }
            }
            correct_letters.push(q.choices.iter().find(|c| c.correct).unwrap().id);
        }
        correct_letters.dedup();
        assert!(
            correct_letters.len() > 1,
            "correct ids are not all the same letter"
        );
        let all_same = correct_letters.windows(2).all(|w| w[0] == w[1]);
        assert!(!all_same);
    }

    #[test]
    fn r4_order_rotates() {
        assert_eq!(order(0), [0, 1, 2]);
        assert_eq!(order(1), [1, 2, 0]);
        assert_eq!(order(2), [2, 0, 1]);
        assert_eq!(order(3), [0, 1, 2]);
        for a in 0..=254u8 {
            let mut o = order(a);
            assert_ne!(order(a), order(a + 1));
            o.sort();
            assert_eq!(o, [0, 1, 2]);
        }
    }

    // R5. The desk, master path.
    #[test]
    fn r5_desk_master() {
        let mut d = Desk::default();
        assert_eq!(d.state_json(), "null");
        assert_eq!(
            d.start(1, Kind::Master, 1, true, false, "sprout"),
            Err(Refuse::Stage)
        );
        assert_eq!(
            d.start(1, Kind::Master, 3, true, false, "sprout"),
            Err(Refuse::Stage)
        );
        assert_eq!(
            d.start(1, Kind::Master, 0, true, false, "sprout"),
            Err(Refuse::Stage)
        );
        assert_eq!(
            d.start(1, Kind::Master, 2, false, false, "sprout"),
            Err(Refuse::NotLearnable)
        );
        assert_eq!(
            d.start(0, Kind::Master, 2, true, false, "sprout"),
            Err(Refuse::NoQuestion)
        );
        assert_eq!(d.phase(), None);
        assert_eq!(d.start(1, Kind::Master, 2, true, false, "sprout"), Ok(()));
        assert_eq!(
            d.start(2, Kind::Master, 2, true, false, "sprout"),
            Err(Refuse::Open)
        );
        assert_eq!(d.open_spell(), Some(1));
        assert_eq!(d.phase(), Some(Phase::Build));
        let j = d.state_json();
        assert!(parses(&j), "{j}");
        assert!(j.contains("\"phase\":\"build\""), "{j}");
        assert!(j.contains("\"q\":null"), "{j}");
        assert!(
            j.contains("\"slot\":2") && j.contains("\"kind\":\"master\""),
            "{j}"
        );

        assert_eq!(d.on_build(2, true), Built::Ignored, "another spell");
        assert_eq!(d.on_build(1, true), Built::Counted(1));
        assert_eq!(d.on_build(1, true), Built::Counted(2));
        assert_eq!(d.on_build(1, false), Built::Miss);
        let j = d.state_json();
        assert!(
            j.contains("\"built\":2") && j.contains("\"misses\":1"),
            "{j}"
        );
        assert_eq!(d.on_answer("a"), None, "no answer while building");
        assert_eq!(d.on_build(1, true), Built::Ask);
        assert_eq!(d.phase(), Some(Phase::Ask));
        assert_eq!(d.on_build(1, true), Built::Ignored);
        assert_eq!(d.on_build(1, false), Built::Ignored);
        assert_eq!(d.on_answer("z"), None);
        assert_eq!(d.phase(), Some(Phase::Ask));
        assert!(!d.ask_again(), "nothing to ask again yet");

        let j = d.state_json();
        assert!(parses(&j), "{j}");
        assert!(j.contains("\"id\":\"water_sprout\""), "{j}");
        assert!(
            j.contains("\"answer\":null") && j.contains("\"why\":null"),
            "{j}"
        );
        assert!(j.contains("\"icon\":\"fire_out\""), "{j}");
        assert!(j.contains("\"right\":null"), "{j}");

        let ok = correct_id(1, "sprout");
        let bad = wrong_id(1, "sprout");
        assert_eq!(d.on_answer(bad), Some(false));
        assert_eq!(d.phase(), Some(Phase::Answered));
        let j = d.state_json();
        assert!(parses(&j), "{j}");
        assert!(j.contains(&format!("\"answer\":\"{ok}\"")), "{j}");
        assert!(j.contains("\"right\":false"), "{j}");
        assert!(!j.contains("\"why\":null"), "{j}");
        d.finish(true);
        assert_eq!(
            d.phase(),
            Some(Phase::Answered),
            "finish needs a right answer"
        );
        assert_eq!(d.on_answer(ok), None, "answered: no second answer");

        let first: Vec<String> = choice_ids(&d);
        assert!(d.ask_again());
        assert_eq!(d.phase(), Some(Phase::Ask));
        assert_ne!(order(1), order(0));
        let again: Vec<String> = choice_ids(&d);
        assert_ne!(first, again, "the order changed");
        let j = d.state_json();
        assert!(
            j.contains("\"answer\":null") && j.contains("\"asks\":1"),
            "{j}"
        );

        assert_eq!(d.on_answer(ok), Some(true));
        d.finish(true);
        assert_eq!(d.phase(), Some(Phase::Done));
        let j = d.state_json();
        assert!(parses(&j), "{j}");
        assert!(
            j.contains("\"phase\":\"done\"") && j.contains("\"mastered\":true"),
            "{j}"
        );
        assert!(j.contains(&format!("\"answer\":\"{ok}\"")), "{j}");
        d.close();
        assert_eq!(d.state_json(), "null");
        assert_eq!(d.open_spell(), None);
    }

    fn choice_ids(d: &Desk) -> Vec<String> {
        let j = d.state_json();
        let at = j.find("\"choices\":[").unwrap();
        j[at..]
            .split("{\"id\":\"")
            .skip(1)
            .take(3)
            .map(|s| s[..1].to_string())
            .collect()
    }

    #[test]
    fn r5_every_question_serialises_and_hides_its_answer() {
        for q in &QUESTIONS {
            let mut d = Desk::default();
            d.start(q.spell, Kind::Master, 2, true, false, q.band)
                .unwrap();
            for _ in 0..3 {
                d.on_build(q.spell, true);
            }
            let j = d.state_json();
            assert!(parses(&j), "{j}");
            assert!(j.contains("\"answer\":null"), "{j}");
            let ok = q.choices.iter().find(|c| c.correct).unwrap().id;
            assert_eq!(d.on_answer(ok), Some(true));
            let j = d.state_json();
            assert!(parses(&j), "{j}");
        }
    }

    // R6. The desk, sharp path.
    #[test]
    fn r6_desk_sharp() {
        let mut d = Desk::default();
        assert_eq!(
            d.start(1, Kind::Sharp, 2, true, true, "sprout"),
            Err(Refuse::Stage)
        );
        assert_eq!(
            d.start(1, Kind::Sharp, 3, true, false, "sprout"),
            Err(Refuse::NotReady)
        );
        assert_eq!(d.start(1, Kind::Sharp, 3, true, true, "sprout"), Ok(()));
        let j = d.state_json();
        assert!(parses(&j), "{j}");
        assert!(
            j.contains("\"kind\":\"sharp\"") && j.contains("\"q\":null"),
            "{j}"
        );
        assert_eq!(d.on_build(1, false), Built::Miss);
        assert_eq!(d.phase(), Some(Phase::Build));
        assert_eq!(d.on_build(1, false), Built::Miss);
        assert_eq!(d.on_build(1, true), Built::SharpDone);
        assert_eq!(d.phase(), Some(Phase::Done));
        assert_eq!(d.on_build(1, true), Built::Ignored);
        assert_eq!(d.on_answer("a"), None);
        let j = d.state_json();
        assert!(
            parses(&j) && j.contains("\"q\":null") && j.contains("\"misses\":2"),
            "{j}"
        );
        // Fire at Sprout has no question but a sharp check needs none.
        let mut f = Desk::default();
        assert_eq!(f.start(0, Kind::Sharp, 3, true, true, "seedling"), Ok(()));
    }

    #[test]
    fn esc_escapes_quotes_backslashes_and_controls() {
        assert_eq!(esc("a\"b\\c\n"), "a\\\"b\\\\c\\u000a");
        assert!(parses(&format!("\"{}\"", esc("x\"\\\u{1}y"))));
    }
}
