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
    /// Runs `f` on spell `i` and settles it. Bumps `rev` only when a field
    /// actually changed; an out-of-range index changes nothing.
    fn op(&mut self, i: usize, f: impl FnOnce(&mut SpellProgress, u32)) -> Vec<Transition> {
        let day = self.day;
        let Some(p) = self.spells.get_mut(i) else {
            return Vec::new();
        };
        let before = *p;
        f(p, day);
        let mut out = Vec::new();
        if p.stage != before.stage {
            out.push(Transition {
                spell: i,
                to: p.stage,
            });
        }
        out.extend(settle(i, p));
        if *p != before {
            self.rev = self.rev.wrapping_add(1);
        }
        out
    }

    /// Unknown -> Learning, `+XP_LEARN`. Nothing on any other stage.
    pub fn learn(&mut self, i: usize) -> Vec<Transition> {
        self.op(i, |p, _| {
            if p.stage == Stage::Unknown {
                p.stage = Stage::Learning;
                add_xp(p, XP_LEARN);
            }
        })
    }

    /// A worked cast on a learned spell: `+XP_WORKED` if built correctly,
    /// `+XP_WORKED_WRONG` (0) if not. Nothing on Unknown.
    pub fn worked_cast(&mut self, i: usize, correct: bool) -> Vec<Transition> {
        self.op(i, |p, _| {
            if p.stage != Stage::Unknown {
                add_xp(p, if correct { XP_WORKED } else { XP_WORKED_WRONG });
            }
        })
    }

    /// An instant cast: `+XP_INSTANT`, only on Mastered.
    pub fn instant_cast(&mut self, i: usize) -> Vec<Transition> {
        self.op(i, |p, _| {
            if p.stage == Stage::Mastered {
                add_xp(p, XP_INSTANT);
            }
        })
    }

    /// An unaided correct build on Learning or Practised, only while the day
    /// is known. It counts distinct local calendar days, as target.md's
    /// "2 days" says: builds at 11:55 pm and 12:05 am count as two days, and
    /// so does changing the device clock. Accepted on purpose: storing
    /// timestamps would keep a history of when a child played and still could
    /// not be checked against a changed clock. This is the only place that
    /// counts days, so a stricter rule (I-030) can go here.
    pub fn unaided_build(&mut self, i: usize) -> Vec<Transition> {
        self.op(i, |p, day| {
            if day != 0 && matches!(p.stage, Stage::Learning | Stage::Practised) {
                p.builds = (p.builds + 1).min(BUILDS_NEEDED);
                if day != p.last_build_day {
                    p.build_days = (p.build_days + 1).min(DAYS_NEEDED);
                    p.last_build_day = day;
                }
                add_xp(p, XP_UNAIDED_BUILD);
            }
        })
    }

    /// A "predict what happens" answer on Learning or Practised. A correct
    /// one sets `predicted` and adds `XP_PREDICT`; a wrong one changes
    /// nothing and is never penalised.
    pub fn predict(&mut self, i: usize, correct: bool) -> Vec<Transition> {
        self.op(i, |p, _| {
            if correct && matches!(p.stage, Stage::Learning | Stage::Practised) {
                p.predicted = true;
                add_xp(p, XP_PREDICT);
            }
        })
    }

    /// Every spell back to the default. The only way down; only an explicit
    /// player action may call it.
    pub fn clear(&mut self) {
        self.spells = Default::default();
        self.rev = self.rev.wrapping_add(1);
    }

    /// Today's local day number (days since 1970-01-01 in the device's time
    /// zone), from `web/progress.js`. Outside 19 000..=80 000 (years 2022 to
    /// 2189) it reads as 0, unknown. Not a progress change.
    pub fn set_day(&mut self, d: u32) {
        self.day = if (19_000..=80_000).contains(&d) { d } else { 0 };
    }

    pub fn get(&self, i: usize) -> Option<&SpellProgress> {
        self.spells.get(i)
    }

    pub fn rev(&self) -> u32 {
        self.rev
    }

    /// One line, `p1` then `|id:stage:xp:builds:build_days:last_build_day:predicted`
    /// for each spell not at the default. `rev` and `day` are not saved.
    pub fn encode(&self) -> String {
        let mut s = String::from("p1");
        for (i, p) in self.spells.iter().enumerate() {
            if *p != SpellProgress::default() {
                s.push_str(&format!(
                    "|{}:{}:{}:{}:{}:{}:{}",
                    SPELL_IDS[i],
                    p.stage as u8,
                    p.xp,
                    p.builds,
                    p.build_days,
                    p.last_build_day,
                    p.predicted as u8
                ));
            }
        }
        s
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

fn add_xp(p: &mut SpellProgress, xp: u32) {
    p.xp = p.xp.saturating_add(xp).min(XP_MAX);
}

/// Learning with enough XP becomes Practised; then Practised with 3 builds
/// over 2 days and a prediction becomes Mastered. Never lowers a stage.
fn settle(i: usize, p: &mut SpellProgress) -> Vec<Transition> {
    let mut out = Vec::new();
    if p.stage == Stage::Learning && p.xp >= PRACTISED_XP {
        p.stage = Stage::Practised;
        out.push(Transition {
            spell: i,
            to: Stage::Practised,
        });
    }
    if p.stage == Stage::Practised
        && p.builds >= BUILDS_NEEDED
        && p.build_days >= DAYS_NEEDED
        && p.predicted
    {
        p.stage = Stage::Mastered;
        out.push(Transition {
            spell: i,
            to: Stage::Mastered,
        });
    }
    out
}

/// An unsigned decimal of at most `max`; no sign, no spaces, no leading `+`.
fn num(s: &str, max: u32) -> Result<u32, DecodeError> {
    if s.is_empty() || s.len() > 10 || !s.bytes().all(|c| c.is_ascii_digit()) {
        return Err(DecodeError::Range);
    }
    match s.parse::<u64>() {
        Ok(v) if v <= max as u64 => Ok(v as u32),
        _ => Err(DecodeError::Range),
    }
}

/// Parses a save. Rejects the whole save, never part of it. The stage is
/// kept exactly as saved (no `settle`), above or below what its fields
/// allow: mastery never fades, and a load is not a progress change.
pub fn decode(s: &str) -> Result<[SpellProgress; 4], DecodeError> {
    if s.len() > MAX_SAVE {
        return Err(DecodeError::TooLong);
    }
    let mut parts = s.split('|');
    if parts.next() != Some("p1") {
        return Err(DecodeError::Prefix);
    }
    let mut out = [SpellProgress::default(); 4];
    let mut seen = [false; 4];
    for entry in parts {
        let f: Vec<&str> = entry.split(':').collect();
        if f.len() != 7 {
            return Err(DecodeError::Fields);
        }
        let i = index_of(f[0]).ok_or(DecodeError::Id)?;
        if seen[i] {
            return Err(DecodeError::Repeated);
        }
        seen[i] = true;
        let stage = match num(f[1], 3)? {
            0 => Stage::Unknown,
            1 => Stage::Learning,
            2 => Stage::Practised,
            _ => Stage::Mastered,
        };
        out[i] = SpellProgress {
            stage,
            xp: num(f[2], XP_MAX)?,
            builds: num(f[3], BUILDS_NEEDED as u32)? as u8,
            build_days: num(f[4], DAYS_NEEDED as u32)? as u8,
            last_build_day: num(f[5], 80_000)?,
            predicted: num(f[6], 1)? == 1,
        };
    }
    Ok(out)
}

/// Runs one `prog ...` command (the words after `prog`). `load` and `day`
/// are always allowed; the rest need `?debug`. On `Err` nothing changes.
pub fn apply_cmd(
    b: &mut Book,
    args: &[&str],
    debug: bool,
) -> Result<Vec<Transition>, &'static str> {
    let idx = |k: usize| -> Result<usize, &'static str> {
        args.get(k)
            .and_then(|s| s.parse::<usize>().ok())
            .ok_or("needs a spell index")
    };
    let flag = |k: usize| -> Result<bool, &'static str> {
        match args.get(k).copied() {
            Some("0") => Ok(false),
            Some("1") => Ok(true),
            _ => Err("needs 0 or 1"),
        }
    };
    let name = args.first().copied().ok_or("needs a subcommand")?;
    match name {
        "load" => {
            let s = args.get(1).ok_or("needs a save")?;
            if args.len() != 2 {
                return Err("bad save");
            }
            b.spells = decode(s).map_err(|_| "bad save")?;
            Ok(Vec::new())
        }
        "day" => {
            let d = args
                .get(1)
                .and_then(|s| s.parse::<u32>().ok())
                .ok_or("needs a day number")?;
            b.set_day(d);
            Ok(Vec::new())
        }
        "learn" | "worked" | "instant" | "build" | "predict" | "clear" if !debug => {
            Err("needs ?debug")
        }
        "learn" => Ok(b.learn(idx(1)?)),
        "worked" => {
            let (i, c) = (idx(1)?, flag(2)?);
            Ok(b.worked_cast(i, c))
        }
        "instant" => Ok(b.instant_cast(idx(1)?)),
        "build" => Ok(b.unaided_build(idx(1)?)),
        "predict" => {
            let (i, c) = (idx(1)?, flag(2)?);
            Ok(b.predict(i, c))
        }
        "clear" => {
            b.clear();
            Ok(Vec::new())
        }
        _ => Err("unknown prog command"),
    }
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
