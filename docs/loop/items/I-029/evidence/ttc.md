# I-029 W18: time to cast, per band

These are the `WORKEDTTC` rows from the round-2 run of golden `ONLY=workedCasts,frozenGate,formulaBench` (`QUERY=?low`, headless Chromium, 1366 x 768). That run scored 123/125. W18 passed on all five rows and on both tripwires.

How to read the columns (arbiter r2):
- `castFrames` counts game frames from the Make press to the first recorder row where `casts` has gone up by 1. The Make press is the last left `pointerdown` before that row, or Enter at Apprentice. The bar is 2 or fewer.
- `sameFrame` means the cast row is also the first row with `work === null` after the press.
- `ttcMs` is wall time from the opening click to the cast. It is shown for information only, because it mostly measures the harness's page round trips at 1.5-3 fps. It has no bar.
- `estS` is `1.0 + inputs x perInput[band]`. The cap is 10 s.

| band | spell | inputs | openFrames | castFrames | sameFrame | ttcMs (info) | open-frame CPU ms | estS | gap px | min piece px |
|---|---|---|---|---|---|---|---|---|---|---|
| sprout | water | 4 | 1 | 1 | yes | 9520 | 14.2 | 7.0 | 29 | 64 |
| seedling | water | 4 | 1 | 1 | yes | 7162 | 15.7 | 4.6 | 29 | 56 |
| seedling | fire | 3 | 1 | 1 | yes | 9551 | 11.0 | 3.7 | 29 | 48 |
| apprentice | water | 7 | 1 | 1 | yes | 11744 | 9.6 | 3.45 | 29 | 48 |
| apprentice | fire | 24 | 1 | 1 | yes | 36961 | 7.6 | 9.4 | 29 | 48 |

N1, the casts in a Water-only level-1 completion, is 3 (won). The level-1 chore tripwire:
- Sprout: 3 x 7.0 = 21 s, at or under 60 s.
- Seedling: 3 x 4.6 = 13.8 s, at or under 60 s.

Apprentice Fire is 24 typed characters (estS 9.4, just under the 10 s cap). This is break 1's should-fix 4, and it is left to the critic.
