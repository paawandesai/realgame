# I-051: Spell physics v1 (water arcs under gravity, short fire and frost cones, CO2 lobs, sinks and spreads)

**based_on:** origin/main 5662cd7. **Lane:** Paawan's Mac (wave-14-mac). **Why:** Paawan, 2026-11-01: "the spell line is too long and only in a straight line; the idea is hyper realistic physics". Today every spell is `spell_path`, a lerp with a tiny arc, drawn as a constant rod from the hand to the crosshair, and the hit is resolved at the crosshair at any range. Moves **B5** (fun) and **B11** (chemistry, not a laser).

## Behaviour

Each spell moves like the real substance. The hit is resolved where the spell actually arrives, not at the crosshair.

| spell | motion | reach | what it hits |
|---|---|---|---|
| Water | ballistic jet from the hand under g = 9.81 m/s²: launch speed √(g·R), so R is the flat-ground range at 45°; droplets trace the parabola | R = 6 m for a tap, rising with how long the button has been held to 10 m at 0.8 s | the first object or ground the parabola meets; on bare ground, an object within 0.5 m of the splash |
| Fire | short flame cone from the hand, flickering, hot air rising | 2.5 m, half-angle 18° | the nearest object inside the cone, else nothing |
| Frost | short cold spray cone, mist sinking | 3 m, half-angle 20° | as Fire |
| CO₂ | a heavy puff lobbed under gravity, which settles and spreads along the ground (denser than air) | R = 5 m tap to 7 m held | the first object it lands on; on bare ground, the nearest object within 1.5 m of where it settles |

- **Aim assist** (the existing `assist` setting, on for every band by default): when the crosshair's assisted aim has a target, the jet and the lob are launched along the low-arc solution that lands on it (if the launch speed can reach it), and the cones turn toward it (if it is inside range and within 18° + assist of the view). With assist off, every spell goes exactly where the view points.
- **Arc preview:** while the button is held with Water or CO₂ (pointer locked), a faint dotted parabola shows the path the next cast will take, ending where it would land.
- A spell that lands on nothing does nothing (a `Miss`), as today.
- A `LAND x y z obj=N` info line is logged for every landing (the golden reads it; no state keys change).
- The debug `cast` command takes an optional hold time: `cast 0.8` casts as if held 0.8 s.

## Acceptance tests (each fails on 5662cd7)

**Rust** (`ballistics.rs`, pure):

1. `at` and `apex`: for speed v and launch angle θ from height 0, the apex height is v²sin²θ/(2g) and occurs at v sinθ/g (within 1e-4).
2. `landing_time` on flat ground: range v²sin(2θ)/g for θ in 15°..75° from height 0; from height h > 0 the jet lands later and farther.
3. `reach_for_hold`: tap (0 s) gives the minimum, 0.8 s or more the maximum, monotonic in between, NaN or negative hold gives the minimum. Water 6 → 10 m, CO₂ 5 → 7 m.
4. `aim_velocity`: for targets within reach the returned velocity has the given speed and its path passes within 1e-2 m of the target; it picks the low arc; it returns `None` beyond reach.
5. `in_cone`: points inside range and angle are in; points beyond range, behind, or outside the half-angle are out.
6. `path_hits`: a flat-ground path ends where `landing_time` says.

**Golden** (new section `spellPhysics`, in the `ONLY` map):

7. Water from far falls short: in level 1, from about 14 m from a burning bush with assist on, a Water cast lands (a `LAND` line within 9 m of the hand) and the bush keeps burning.
8. Walk closer and it works: from about 4 m, Water puts that bush out.
9. Reach grows with hold: on open ground with the view 40° up, `cast` lands 5.5-8.5 m away, and `cast 0.8` lands 9-13 m away, farther than the tap by at least 3 m. Both land on the ground (y ≈ 0).
10. Fire is short: Fire at the dry hedge from 5 m changes nothing (no ignition), and from 1.8 m lights it.
11. Frost is short: in level 2, Frost at the stream from 6 m leaves its ice at 0, and from 2 m freezes a third of it.
12. CO₂ lobs and settles: in level 3, CO₂ at the dip from where the flames stop the player fills the dip pool to 0.5.
13. No console errors.

## Science rows

- P-026 `G` 9.81 m/s² (standard gravity 9.80665 m/s², rounded): new source, NIST standard acceleration of gravity.
- P-027 water jet reach 6-10 m, P-028 CO₂ lob 5-7 m, P-029 fire cone 2.5 m, P-030 frost cone 3 m: gameplay tuning [S0]. A hand-held water jet reaching a few metres is in line with real hoses and extinguishers, but these numbers make no physical claim.
- The CO₂ puff sinking and spreading uses the existing density rows (C-035, [S5] [S8] [S26]); no new player text.

## Out of scope

- Splash physics on objects (droplets bouncing), wind, real fluid simulation, CO₂ flowing downhill across a level (the Marsh's hollows keep their fill rule).
- New state keys (key-order checks stay as they are).
- Changing any existing golden check: checks that assumed the old long straight beam are listed in the report for Paawan to decide.

## Contract

- `crates/bevy-client/src/ballistics.rs` (new, pure, no `crate::` in tests).
- `main.rs`: the cast loop computes each spell's motion, path and landing; `Projectile` and `Stream` follow the path; the arc preview; `LAND` log line; `cast <hold>` debug arg. `spell_path` goes.
- `tests/e2e/golden.mjs`: section `spellPhysics` in the `ONLY` map.
- `content/science-sources.md`: P-026..P-030 and the gravity source.
