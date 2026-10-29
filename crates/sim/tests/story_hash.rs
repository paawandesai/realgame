//! Golden hashes of the v2 story grid. v3 adds species, a per-cell `tag` and
//! Trial/Lab levels, but the story must stay bit-for-bit the same (DESIGN_V3
//! §1, §12). If one of these fails, a change leaked into the story.
//!
//! The hash is FNV-1a over every cell's species, temperature bits and `aux`,
//! plus `tag` only where it is non-zero, so a story run that never sets a
//! tag hashes the same as it did before `tag` existed.

use realgame_sim::game::{Game, Tier, INPUT_RIGHT};

fn story_hash(g: &Game) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |b: u8| {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    };
    for (i, c) in g.grid.cells.iter().enumerate() {
        eat(*c as u8);
        g.grid.temp[i]
            .to_bits()
            .to_le_bytes()
            .into_iter()
            .for_each(&mut eat);
        g.grid.aux[i].to_le_bytes().into_iter().for_each(&mut eat);
    }
    h
}

fn run(tier: Tier, seed: u64, ticks: u32) -> Game {
    let mut g = Game::new(tier, seed);
    g.step_all_columns = true;
    for t in 0..ticks {
        match t {
            5 => {
                g.set_aim(120, 120, true);
                g.cast("water");
            }
            40 => {
                g.cast("2H2 + O2 -> 2H2O");
            }
            80 => {
                g.cast("2H2O -> 2H2 + O2");
            }
            140 => {
                g.cast("CO2");
            }
            _ => {}
        }
        g.tick(if (200..700).contains(&t) {
            INPUT_RIGHT
        } else {
            0
        });
    }
    g
}

/// Pinned on the v2 main commit (52941cb) before any v3 change.
const PINNED: [(Tier, u64, u64, u64); 3] = [
    (
        Tier::Sprout,
        1,
        0x4ac9_ef26_9caf_8781,
        0x8ddd_eb26_c515_2b30,
    ),
    (
        Tier::Seedling,
        7,
        0x4ac9_ef26_9caf_8781,
        0xda0e_9951_42c4_d197,
    ),
    (
        Tier::Apprentice,
        42,
        0x4ac9_ef26_9caf_8781,
        0x134c_8859_406c_133f,
    ),
];

#[test]
fn v2_story_grid_hashes_are_unchanged() {
    for (tier, seed, new, after) in PINNED {
        let g = Game::new(tier, seed);
        assert_eq!(story_hash(&g), new, "{tier:?} new game");
        assert_eq!(g.grid.content_hash(), new, "Grid::content_hash agrees");
        let g = run(tier, seed, 900);
        assert_eq!(
            story_hash(&g),
            after,
            "{tier:?} seed {seed} after 900 ticks"
        );
        assert_eq!(g.grid.content_hash(), after);
        assert!(g.grid.tag.iter().all(|&t| t == 0), "the story never tags");
        assert!(g.is_story());
    }
}
