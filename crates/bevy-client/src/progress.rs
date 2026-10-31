//! Spell progress: Unknown, Learning, Practised, Mastered, and XP (I-027).
//!
//! Pure Rust with no Bevy types and no crate imports, so it is tested with
//! `rustc --edition 2021 --test src/progress.rs` as well as `cargo test`.
//! It is the one owner of the progress rules; I-028, I-029 and I-030 call
//! the `Book` methods and never write fields. The save lives only in this
//! browser's `localStorage` (written by `web/progress.js`).

#![allow(dead_code)]

/// Stable spell ids in `Spell::ALL` order. Tied to `content/spells.json`
/// by the boot check in `main.rs` `setup` and by `test/bevy-progress.test.ts`.
pub const SPELL_IDS: [&str; 4] = ["fire", "water", "frost", "co2"];

/// XP for the first correct bench build.
pub const XP_LEARN: u32 = 10;
/// XP for a worked cast built correctly.
pub const XP_WORKED: u32 = 10;
/// XP for a wrong worked build: it fizzles kindly and costs nothing.
pub const XP_WORKED_WRONG: u32 = 0;
/// XP for an instant cast of a Mastered spell.
pub const XP_INSTANT: u32 = 2;
/// XP for an unaided correct build.
pub const XP_UNAIDED_BUILD: u32 = 20;
/// XP for a correct "predict what happens" answer.
pub const XP_PREDICT: u32 = 20;
/// Learning becomes Practised at this much XP.
pub const PRACTISED_XP: u32 = 30;
/// XP saturates here.
pub const XP_MAX: u32 = 1_000_000;

/// Unaided builds needed for mastery.
pub const BUILDS_NEEDED: u8 = 3;
/// Distinct local days those builds must span.
pub const DAYS_NEEDED: u8 = 2;
/// The longest save `decode` accepts.
pub const MAX_SAVE: usize = 512;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Stage {
    #[default]
    Unknown,
    Learning,
    Practised,
    Mastered,
}

impl Stage {
    pub fn as_str(self) -> &'static str {
        match self {
            Stage::Unknown => "unknown",
            Stage::Learning => "learning",
            Stage::Practised => "practised",
            Stage::Mastered => "mastered",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpellProgress {
    pub stage: Stage,
    pub xp: u32,
    pub builds: u8,
    pub build_days: u8,
    pub last_build_day: u32,
    pub predicted: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transition {
    pub spell: usize,
    pub to: Stage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    TooLong,
    Prefix,
    Fields,
    Id,
    Repeated,
    Range,
}

#[derive(Clone, Debug, Default)]
pub struct Book {
    spells: [SpellProgress; 4],
    rev: u32,
    day: u32,
}

impl Book {
    pub fn learn(&mut self, _i: usize) -> Vec<Transition> {
        Vec::new()
    }

    pub fn worked_cast(&mut self, _i: usize, _correct: bool) -> Vec<Transition> {
        Vec::new()
    }

    pub fn instant_cast(&mut self, _i: usize) -> Vec<Transition> {
        Vec::new()
    }

    pub fn unaided_build(&mut self, _i: usize) -> Vec<Transition> {
        Vec::new()
    }

    pub fn predict(&mut self, _i: usize, _correct: bool) -> Vec<Transition> {
        Vec::new()
    }

    pub fn clear(&mut self) {}

    pub fn set_day(&mut self, _d: u32) {}

    pub fn get(&self, i: usize) -> Option<&SpellProgress> {
        self.spells.get(i)
    }

    pub fn rev(&self) -> u32 {
        self.rev
    }

    pub fn encode(&self) -> String {
        "p1".to_string()
    }

    pub fn state_json(&self) -> String {
        let items: Vec<String> = self
            .spells
            .iter()
            .enumerate()
            .map(|(i, p)| {
                format!(
                    "{{\"id\":\"{}\",\"st\":\"{}\",\"xp\":{},\"builds\":{},\"days\":{},\"predicted\":{}}}",
                    SPELL_IDS[i],
                    p.stage.as_str(),
                    p.xp,
                    p.builds,
                    p.build_days,
                    p.predicted
                )
            })
            .collect();
        format!("[{}]", items.join(","))
    }
}

pub fn decode(_s: &str) -> Result<[SpellProgress; 4], DecodeError> {
    Err(DecodeError::Prefix)
}

pub fn apply_cmd(
    _b: &mut Book,
    _args: &[&str],
    _debug: bool,
) -> Result<Vec<Transition>, &'static str> {
    Ok(Vec::new())
}

/// The stable id of spell `i` (`Spell::ALL` order), or `""` out of range.
pub fn spell_id(i: usize) -> &'static str {
    SPELL_IDS.get(i).copied().unwrap_or("")
}

/// The index of a stable id. Display names (`"CO₂"`, `"Water"`) are not ids.
pub fn index_of(id: &str) -> Option<usize> {
    SPELL_IDS.iter().position(|s| *s == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(spell: usize, to: Stage) -> Transition {
        Transition { spell, to }
    }

    fn practised_water() -> Book {
        let mut b = Book::default();
        b.learn(1);
        b.worked_cast(1, true);
        b.worked_cast(1, true);
        assert_eq!(b.get(1).unwrap().stage, Stage::Practised);
        b
    }

    // 1. Stages in order.
    #[test]
    fn t01_stages_in_order() {
        let mut b = Book::default();
        for i in 0..4 {
            assert_eq!(b.get(i).unwrap().stage, Stage::Unknown);
            assert_eq!(b.get(i).unwrap().xp, 0);
        }
        assert_eq!(b.rev(), 0);
        assert_eq!(b.learn(1), vec![t(1, Stage::Learning)]);
        assert_eq!(b.get(1).unwrap().xp, 10);
        assert_eq!(b.rev(), 1);
        assert_eq!(b.learn(1), vec![]);
        assert_eq!(b.get(1).unwrap().xp, 10);
        assert_eq!(b.rev(), 1);
        assert!(Stage::Unknown < Stage::Learning);
        assert!(Stage::Learning < Stage::Practised);
        assert!(Stage::Practised < Stage::Mastered);
    }

    // 2. Worked casts earn XP before mastery.
    #[test]
    fn t02_worked_casts() {
        let mut b = Book::default();
        b.learn(1);
        assert_eq!(b.worked_cast(1, true), vec![]);
        assert_eq!(b.worked_cast(1, true), vec![t(1, Stage::Practised)]);
        assert_eq!(b.get(1).unwrap().xp, 30);
        let rev = b.rev();
        assert_eq!(b.worked_cast(1, false), vec![]);
        assert_eq!(b.get(1).unwrap().xp, 30);
        assert_eq!(b.rev(), rev);
        assert_eq!(b.worked_cast(2, true), vec![]);
        assert_eq!(b.get(2).unwrap().xp, 0);
        assert_eq!(b.rev(), rev);
        assert_eq!(b.instant_cast(1), vec![]);
        assert_eq!(b.get(1).unwrap().xp, 30);
        assert_eq!(b.rev(), rev);
    }

    // 3. The 2-day rule.
    #[test]
    fn t03_two_day_rule() {
        let mut b = practised_water();
        b.set_day(20755);
        for _ in 0..3 {
            assert_eq!(b.unaided_build(1), vec![]);
        }
        assert_eq!(b.get(1).unwrap().builds, 3);
        assert_eq!(b.get(1).unwrap().build_days, 1);
        assert_eq!(b.predict(1, true), vec![]);
        assert!(b.get(1).unwrap().predicted);
        assert_eq!(b.get(1).unwrap().stage, Stage::Practised);
        b.set_day(20756);
        assert_eq!(b.unaided_build(1), vec![t(1, Stage::Mastered)]);
        assert_eq!(b.get(1).unwrap().build_days, 2);

        for d in [0, 5] {
            let mut c = practised_water();
            c.set_day(d);
            let rev = c.rev();
            let before = *c.get(1).unwrap();
            assert_eq!(c.unaided_build(1), vec![]);
            assert_eq!(*c.get(1).unwrap(), before);
            assert_eq!(c.rev(), rev);
        }

        let rev = b.rev();
        let xp = b.get(1).unwrap().xp;
        assert_eq!(b.instant_cast(1), vec![]);
        assert_eq!(b.get(1).unwrap().xp, xp + 2);
        assert_eq!(b.rev(), rev + 1);
    }

    // 4. Predict.
    #[test]
    fn t04_predict() {
        let mut b = practised_water();
        let rev = b.rev();
        let before = *b.get(1).unwrap();
        assert_eq!(b.predict(1, false), vec![]);
        assert_eq!(*b.get(1).unwrap(), before);
        assert_eq!(b.rev(), rev);
        // Unknown.
        assert_eq!(b.predict(2, true), vec![]);
        assert_eq!(*b.get(2).unwrap(), SpellProgress::default());
        assert_eq!(b.rev(), rev);
        // Mastered.
        apply_cmd(&mut b, &["load", "p1|water:3:0:0:0:0:0"], false).unwrap();
        let rev = b.rev();
        let before = *b.get(1).unwrap();
        assert_eq!(b.predict(1, true), vec![]);
        assert_eq!(*b.get(1).unwrap(), before);
        assert_eq!(b.rev(), rev);
    }

    // 5. Two transitions in one step.
    #[test]
    fn t05_two_transitions() {
        let mut b = Book::default();
        assert!(apply_cmd(&mut b, &["load", "p1|water:1:20:3:2:20756:1"], false).is_ok());
        assert_eq!(
            *b.get(1).unwrap(),
            SpellProgress {
                stage: Stage::Learning,
                xp: 20,
                builds: 3,
                build_days: 2,
                last_build_day: 20756,
                predicted: true,
            }
        );
        assert_eq!(
            b.worked_cast(1, true),
            vec![t(1, Stage::Practised), t(1, Stage::Mastered)]
        );
        assert_eq!(b.get(1).unwrap().xp, 30);
    }

    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> u32 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (self.0 >> 33) as u32
        }
        fn below(&mut self, n: u32) -> u32 {
            self.next() % n
        }
    }

    /// Runs the random walk of test 6 and hands every reached book to `f`.
    /// Returns whether spell 1 reached Mastered.
    fn walk(mut f: impl FnMut(&Book)) -> bool {
        let mut water_mastered = false;
        for seed in 0..50u64 {
            let mut r = Lcg(seed * 7919 + 1);
            let mut b = Book::default();
            let mut day = 20_000u32;
            for _ in 0..20_000 {
                let before = b.spells;
                let i = match r.below(10) {
                    0 => 4 + r.below(3) as usize,
                    _ => r.below(4) as usize,
                };
                let op = r.below(100);
                let mut cleared = false;
                let trs = match op {
                    0..=9 => b.learn(i),
                    10..=24 => b.worked_cast(i, r.below(4) != 0),
                    25..=31 => b.instant_cast(i),
                    32..=51 => b.unaided_build(i),
                    52..=71 => b.predict(i, r.below(3) != 0),
                    72..=72 => {
                        if r.below(20) == 0 {
                            b.clear();
                            cleared = true;
                        }
                        vec![]
                    }
                    _ => {
                        let k = r.below(100);
                        day = match k {
                            0..=59 => day,
                            60..=94 => day + 1,
                            95..=97 => 0,
                            _ => r.below(90_001),
                        };
                        b.set_day(day);
                        if day == 0 || !(19_000..=80_000).contains(&day) {
                            day = 20_000 + r.below(1000);
                        }
                        vec![]
                    }
                };
                if !cleared {
                    for k in 0..4 {
                        assert!(b.spells[k].stage >= before[k].stage, "stage lowered");
                        assert!(b.spells[k].xp >= before[k].xp, "xp lowered");
                    }
                    if i >= 4 {
                        assert_eq!(b.spells, before, "out of range changed the book");
                        assert!(trs.is_empty());
                    }
                }
                for tr in &trs {
                    assert_eq!(tr.spell, i);
                }
                if b.spells[1].stage == Stage::Mastered {
                    water_mastered = true;
                }
                f(&b);
            }
        }
        water_mastered
    }

    // 6. Mastery never fades.
    #[test]
    fn t06_never_fades() {
        assert!(walk(|_| {}), "the walk never mastered water");
    }

    fn save_ok(s: &str) -> bool {
        let Some(rest) = s.strip_prefix("p1") else {
            return false;
        };
        if s.len() > MAX_SAVE {
            return false;
        }
        rest.is_empty()
            || (rest.starts_with('|')
                && rest[1..].split('|').all(|e| {
                    !e.is_empty()
                        && e.bytes().all(|c| {
                            c.is_ascii_lowercase() || c.is_ascii_digit() || c == b':' || c == b'.'
                        })
                }))
    }

    // 7. Save round trip.
    #[test]
    fn t07_round_trip() {
        assert_eq!(Book::default().encode(), "p1");
        let mut n = 0u32;
        walk(|b| {
            n += 1;
            if n % 7 != 0 {
                return;
            }
            let s = b.encode();
            assert!(save_ok(&s), "bad save {s}");
            assert!(!s.contains("rev") && !s.contains("day"));
            assert_eq!(decode(&s), Ok(b.spells), "{s}");
        });
        let mut b = Book::default();
        b.set_day(20755);
        b.learn(1);
        assert_eq!(b.encode(), "p1|water:1:10:0:0:0:0");
    }

    // 8. Decode rejects whole.
    #[test]
    fn t08_decode_rejects_whole() {
        let long = format!("p1|water:1:0:0:0:0:0{}", "0".repeat(513 - 20));
        assert_eq!(long.len(), 513);
        let bad = [
            "",
            "p2",
            "p1|water:2:40:2:1:20756",
            "p1|water:9:0:0:0:0:0",
            "p1|water:1:0:4:0:0:0",
            "p1|water:1:0:0:3:0:0",
            "p1|water:1:0:0:0:0:2",
            "p1|lava:1:0:0:0:0:0",
            "p1|water:1:0:0:0:0:0|water:1:0:0:0:0:0",
            long.as_str(),
            "p1|water:1:-1:0:0:0:0",
            "p1|water:1:0:0:0:0:0;",
            "p1|water:1:0:0:0:0",
            "p1|water:1:0:0:0:0:0:0",
        ];
        let mut b = Book::default();
        b.learn(0);
        let before = b.spells;
        let rev = b.rev();
        for s in bad {
            assert!(decode(s).is_err(), "accepted {s:?}");
            assert!(apply_cmd(&mut b, &["load", s], false).is_err(), "{s:?}");
            assert_eq!(b.spells, before, "{s:?}");
            assert_eq!(b.rev(), rev);
        }
        let m = decode("p1|water:3:0:0:0:0:0").unwrap();
        assert_eq!(m[1].stage, Stage::Mastered);
    }

    // 8b. Stage is kept as saved in both directions (inherited by I-030).
    #[test]
    fn t08b_stage_kept_as_saved() {
        let mut b = Book::default();
        apply_cmd(&mut b, &["load", "p1|water:3:0:0:0:0:0"], false).unwrap();
        assert_eq!(b.get(1).unwrap().stage, Stage::Mastered);
        assert!(!b.get(1).unwrap().predicted);
        b.predict(1, true);
        b.worked_cast(1, true);
        assert_eq!(b.get(1).unwrap().stage, Stage::Mastered);
        let mut c = Book::default();
        apply_cmd(&mut c, &["load", "p1|water:1:20:3:2:20756:1"], false).unwrap();
        assert_eq!(c.get(1).unwrap().stage, Stage::Learning);
    }

    // 9. Commands and the debug gate.
    #[test]
    fn t09_commands() {
        let mut b = Book::default();
        assert_eq!(
            apply_cmd(&mut b, &["learn", "1"], false),
            Err("needs ?debug")
        );
        assert_eq!(*b.get(1).unwrap(), SpellProgress::default());
        assert_eq!(
            apply_cmd(&mut b, &["learn", "1"], true),
            Ok(vec![t(1, Stage::Learning)])
        );
        assert!(apply_cmd(&mut b, &["day", "20755"], false).is_ok());
        assert!(apply_cmd(&mut b, &["load", "p1"], false).is_ok());
        assert_eq!(*b.get(1).unwrap(), SpellProgress::default());
        let rev = b.rev();
        assert!(apply_cmd(&mut b, &["load", "p1|frost:1:10:0:0:0:0"], false).is_ok());
        assert_eq!(b.rev(), rev);
        assert_eq!(b.get(2).unwrap().stage, Stage::Learning);
        for bad in [
            &["nope"][..],
            &["learn"],
            &["learn", "x"],
            &["worked", "1", "2"],
            &[],
            &["load"],
            &["day"],
            &["day", "x"],
        ] {
            let before = b.spells;
            assert!(apply_cmd(&mut b, bad, true).is_err(), "{bad:?}");
            assert_eq!(b.spells, before);
            assert_eq!(b.rev(), rev);
        }
        for gated in [
            &["worked", "1", "1"][..],
            &["instant", "1"],
            &["build", "1"],
            &["predict", "1", "1"],
            &["clear"],
        ] {
            assert_eq!(apply_cmd(&mut b, gated, false), Err("needs ?debug"));
        }
        // Each debug command reaches its operation.
        assert!(apply_cmd(&mut b, &["worked", "2", "1"], true).is_ok());
        assert_eq!(b.get(2).unwrap().xp, 20);
        assert!(apply_cmd(&mut b, &["build", "2"], true).is_ok());
        assert_eq!(b.get(2).unwrap().builds, 1);
        assert!(apply_cmd(&mut b, &["predict", "2", "1"], true).is_ok());
        assert!(b.get(2).unwrap().predicted);
        assert!(apply_cmd(&mut b, &["clear"], true).is_ok());
        assert_eq!(*b.get(2).unwrap(), SpellProgress::default());
    }

    // 10. Ids.
    #[test]
    fn t10_ids() {
        let ids: Vec<&str> = (0..4).map(spell_id).collect();
        assert_eq!(ids, ["fire", "water", "frost", "co2"]);
        for (i, id) in ids.iter().enumerate() {
            assert_eq!(index_of(id), Some(i));
        }
        assert_eq!(index_of("CO₂"), None);
        assert_eq!(index_of("Water"), None);
    }

    /// A small JSON checker for test 11 (no serde in a plain `rustc` test).
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
                    if s[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
                Some(i + 1)
            }
            b't' if s[i..].starts_with(b"true") => Some(i + 4),
            b'f' if s[i..].starts_with(b"false") => Some(i + 5),
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

    // 11. state_json.
    #[test]
    fn t11_state_json() {
        let one = |id: &str| {
            format!("{{\"id\":\"{id}\",\"st\":\"unknown\",\"xp\":0,\"builds\":0,\"days\":0,\"predicted\":false}}")
        };
        let want = format!(
            "[{},{},{},{}]",
            one("fire"),
            one("water"),
            one("frost"),
            one("co2")
        );
        assert_eq!(Book::default().state_json(), want);
        assert!(parses(&want));
        let mut n = 0u32;
        walk(|b| {
            n += 1;
            if n % 13 == 0 {
                let j = b.state_json();
                assert!(parses(&j), "{j}");
            }
        });
    }
}
