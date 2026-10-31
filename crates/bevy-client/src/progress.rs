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
