# Persona panel protocol

The panel is checks.md level 5. Five personas play the first-person 3D slice one turn at a time and judge three bars: **B4** (every age can play), **B5** (fun) and **B8** (enough to play). Every panel score is **"panel" data, never "player" data**: no real child, teacher or playtester is involved (target.md, human-only).

Harness: `tests/e2e/panel/` (I-006, hardened in I-014). CLI: `node tests/e2e/panel/panel.mjs <command> --run <dir> ...`. Every command prints exactly one JSON object and exits non-zero on failure with `{"ok":false,"error":"..."}`.

## Run recipe

1. **Build once** for all five personas: `bash crates/bevy-client/build-web.sh` (with `CARGO_TARGET_DIR=/home/claude/bevy-target` and the wasm-bindgen CLI under `/home/claude/bevy-target/tools` in the cloud). Write down the artifact sha (`git rev-parse --short HEAD`).
2. **Five fresh-context agents, one per persona.** Model routing comes from config.md: haiku for `6yo` and `9yo`, sonnet for `13yo` and `teacher`, opus for `qa`. Each agent gets exactly one persona block below, the turn loop and the rubric, and nothing else: no source code, no bars, no other persona's results. See **Information barrier** below: it is enforced, not only asked for.
3. **The operator (the loop, not the persona) starts each session.** For each persona make a fresh temp dir outside the repo (`T=$(mktemp -d)`); the persona's run dir is `$T/run`, and the operator dir (truth log, daemon logs, token, result) stays in the repo under `.loop-artifacts/panel/<date>/<persona>/`. Same artifact sha and query (`?low` in the cloud) for all five:
   ```sh
   PANEL_ROLE=operator node tests/e2e/panel/panel.mjs start --run "$T/run" \
     --operator-dir .loop-artifacts/panel/<date>/<persona> --persona <id> --query '?low' --artifact <sha>
   ```
   `qa` gets `--throttle 4` by default; everyone else gets 1. Do not pass another throttle: `score` fails the panel for it unless `--allow-throttle`.
4. **Spawn the persona agent with its cwd set to `$T`**, not the repo. Give it only: the persona block, the turn loop, the rubric, the absolute path of `panel.mjs` and its run dir `$T/run`. Never give it the operator dir, the repo path as a working directory, or `PANEL_ROLE`.
5. **Turn loop.** Repeat: `observe`, look at the screenshot file and the HUD text, decide, say what you are trying to do, then `act`. Only `observe` and `act` (the CLI refuses `--truth` and `debug` without the operator role).
6. **Cap:** 400 turns or 20 panel minutes, whichever comes first. When a cap is hit, `observe` and `act` refuse with `"cap"` set (exit 3). 400 turns is enough for a 9-year-old to reach the 15 panel minutes B8 needs: at about 3 s per observe plus a short act, 150 turns would be only about 11 minutes. A persona may stop earlier by saying it would quit; its reason goes in `quit_reason`.
7. When the persona says it is done, it writes the self-report (rubric below) to `$T/answer.json`. The operator runs `stop` (a persona may run it too), then `report --run "$T/run" --operator-dir <odir> --file "$T/answer.json" --model <model>`. `report` refuses a session without a `stop` entry (or a cap). It writes `<odir>/result.json`.
8. The loop scores the five operator dirs together:
   ```sh
   node tests/e2e/panel/panel.mjs score --runs <o6> <o9> <o13> <oteacher> <oqa> --artifact <sha> --append docs/loop/scoreboard.jsonl
   ```
   (`--runs` also accepts a run dir whose `result.json` sits in it or in `<run>.operator`.)

## Information barrier

A persona only knows what the screenshot and the HUD text show. I-014 enforces this in code:

- **Two dirs per session.** The run dir (`--run`) holds only `session.json` (pid, port, persona, throttle, query, viewport: no truth, no token, no operator path) and `shots/NNNN.png`. The operator dir (`--operator-dir`, default `<run>.operator`; it may not contain, or sit inside, the run dir) holds `log.jsonl`, `daemon.log`, `daemon.out`, `start-error.json`, `operator.json` (session token, `cpu_bench_ms`) and `result.json`.
- **Operator role.** `start`, `observe --truth` and `debug` exit 1 unless `PANEL_ROLE=operator` is set, and the CLI checks this before it talks to the daemon. `observe` and `act` are the persona's commands.
- **Token.** The daemon makes a random token at start and writes it only to `operator.json`. It answers a truth `observe` or any `debug` command only when the request carries that token, so a post straight to the daemon's port does not leak state either. `debug` also still needs `PANEL_SELFTEST=1` at start.
- **Process.** Personas run with their cwd in a temp dir outside the repo and get only the CLI path (step 4). The agent shares the machine's user, so this is a barrier against accident and casual peeking, not against a model that sets out to break it; a persona transcript that reads files other than its screenshots, or sets `PANEL_ROLE`, makes its result invalid.

### Commands

| Command | What it does |
|---|---|
| `start --run <dir> [--operator-dir <odir>] [--persona <id>] [--throttle <n>] [--query <q>] [--vw 1280 --vh 720] [--base-url <url>] [--root <pkg-out>] [--artifact <sha>]` | Operator only. Spawns a detached daemon holding one Chromium page (and a static server for `crates/bevy-client/web/pkg-out` unless `--base-url`). Times a CPU benchmark in the page, writes `<odir>/operator.json` and `<dir>/session.json`, and returns once the game is running. |
| `observe --run <dir>` | Saves `<dir>/shots/NNNN.png` and returns the view: `{ok, turn, panel_s, screenshot, hud:{objective, warning, hint, caption, banner, pause, loading, hotbar:{selected, slots}, crosshair_on, cursor}, pointer_locked, stalled}`. HUD fields hold visible text only (`null` when hidden). `stalled:true` means the game's frame counter did not move for 2 s: a harness flag, not game state. |
| `observe --run <dir> --truth [--operator-dir <odir>]` | Operator only: adds the full game state. |
| `act --run <dir> <action> [args]` | Real input only (below). Returns `{ok, turn, panel_s, action, via, stalled, console_errors_new}`. |
| `stop --run <dir>` | Closes the browser and server, ends the daemon. Idempotent. |
| `report --run <dir> [--operator-dir <odir>] --file <answer.json> [--model <m>]` | Validates the self-report, merges it with the measured fields and writes `<odir>/result.json` (an I-006-layout run dir with its own `log.jsonl` gets `<dir>/result.json`). Refuses a session with no `stop` entry and no cap. |
| `score --runs <dirs...> [--append <file>] [--artifact <sha>] [--allow-throttle]` | Prints the panel line; appends it only with `--append`. |

If the daemon is gone (it exits after 60 minutes without a command, or crashed), `observe` and `act` say `daemon is gone` and the operator's `daemon.log` says which.

Actions (coordinates are viewport CSS pixels, 1280 × 720 by default):

| Action | Meaning |
|---|---|
| `key <Key> [ms]` | Hold one key (default 100 ms). Names: `w a s d`, `ArrowUp`…`ArrowRight`, `Space`, `Shift`, `Escape`, `Digit1`…`Digit9`, `r`, any single letter or digit. |
| `keys <K1+K2> <ms>` | Hold a chord, for example `w+Shift 2000`. |
| `move <dx> <dy> [steps]` | Move the mouse. With the mouse captured this turns the view. |
| `drag <x1> <y1> <x2> <y2> [ms]` | Press, move, release. |
| `click <x> <y> [left\|right]` | Click. |
| `hold <x> <y> <ms>` | Hold the left button (keeps casting). |
| `wheel <dy>` | Mouse wheel. |
| `wait <ms>` | Do nothing, at most 10000. |

`act` never sends game commands. `debug` exists only for `selftest.mjs` and works only for the operator, in a session started with `PANEL_SELFTEST=1`.

## Panel clock

Wall time on a model's turn includes model latency, which a child does not have, and the cloud's software GPU slows everything down. So time is **panel time**, which is deterministic for a given action log:

- each act adds its input duration: the `ms` of `key`, `keys`, `hold`, `drag` (default 300) and `wait`; 100 ms for `click`, `move` and `wheel`;
- each `observe` adds the persona's reading cost: `6yo` 4 s, `9yo` 3 s, `13yo` 2 s, `teacher` 3 s, `qa` 2 s (`tests/e2e/panel/personas.mjs`);
- refused acts, `start`, `stop` and `debug` add nothing.

## Game clock between turns

The daemon pauses the page's JavaScript between turns (CDP `Debugger.pause`) and resumes it for each `observe` and `act`, so the courtyard does not burn down while a model thinks. Disable with `start --freeze 0`. Consequences:

- `STATS` and `CPUFRAME` lines from a panel session include the paused gaps (huge `max_ms`). They are not perf data; perf stays in the `perf` check (B2/B3).
- In the cloud the game itself runs slower than real time (software GPU; on a loaded 4-core box we saw about 1 to 3 frames per second, and Bevy clamps each frame's step), so a 1500 ms `key w` walks about 1 m instead of about 6 m. Timings measured in the cloud are therefore pessimistic for movement-heavy goals.

## Measured fields (the harness measures these; the persona never reports them)

The daemon logs every turn to `<odir>/log.jsonl`: `turn`, `cmd`, `panel_s`, `stalled`, the view, and a truth summary (position, yaw, pitch, casts, impacts, caption, outcome, burning, extinguished, pointer lock, pause, spell, per-object chemistry flags).

- `time_to_move_s`: panel time at the end of the first act that either **holds a movement key** (`w a s d`, arrows, in `key` or `keys`) and moved the player at least 0.05 m across that act, or leaves the player more than 0.5 m from spawn; `null` if never. Why the first rule: the game clamps each frame's step and the cloud runs at about 1 to 3 frames per second, so a 1500 ms `w` walked exactly 0.5 m in the I-006 probe and a child's tap never reached 0.5 m. Any real displacement from a movement key now counts, so the 10 s limit judges whether the persona found the controls, not how fast the cloud renders.
- `time_to_cast_s`: panel time at the end of the first act that cast a spell which then **landed**: casts rose, a projectile hit (impacts rose) and the game wrote a spell-result caption (every cast also writes its own caption when it leaves the hand, which is not counted; a lone "Heat from a neighbour…" or "Oh no!" caption is not a result). A click into the sky does not count. `null` if never. I-014 kept this caption rule rather than requiring a chemistry-flag change: the flags are coarse (heating a bush that does not ignite flips none) and change on their own (fire spreading, drying), which would credit a sky cast. A result caption on a target whose flags do not change (Fire at a bush already burning) does count: the spell visibly landed.
- `minutes_played` (B8): **engaged** panel time / 60, not wall or raw panel time. An act is engaged when it is real input other than `wait` and the player's own state changed across it: moved at least 0.05 m, yaw or pitch changed at least 0.5 degrees, casts rose, the selected spell changed, pause or pointer lock toggled, or the outcome changed (fire spreading and captions do not count). Credit: an engaged act counts its full input time; any other non-`wait` act at most 0.5 s; a `wait` counts only after an engaged act, at most 3 s in total until the next engaged act; an `observe` counts its reading cost only when the act just before or just after it is engaged. So 90 x `wait 10000` (15 raw panel minutes, the I-006 probe) gives 0, and 200 x (`observe`, `key Shift 10000`) with nothing changing gives under 2 minutes.
- `panel_minutes`: raw panel time / 60 (the caps use it). `engaged_acts`: count of engaged acts. `stalled_turns`: turns with `stalled:true`.
- `quests_completed` (distinct quests won), `outcome`, `turns`, `console_errors`, `panics` (console text `panicked`), `throttle`, and `cpu_bench_ms` at the top level of the result: the fastest of seven timings of a fixed JavaScript loop on the blank page, taken at start after the CDP throttle is set and before the game loads (fastest, so a busy box does not inflate the unthrottled run) (I-006 found the software GPU hides the throttle in frame rate, so the check is on JavaScript speed).

## Persona prompts

Give each agent exactly one block, plus the turn loop and the rubric. Prices are never discussed.

Common rules (included in every block):

> You only know what the screenshot and the HUD text show you. Never guess hidden state. Before each `act`, say in one short line what you are trying to do. Use only `observe` and `act` with the CLI path you were given, and read no file except the screenshot each `observe` names. Look at the screenshot file every turn. You may stop when you would really stop playing; say why, and write your self-report to `answer.json` in your current directory.

### 6yo: six-year-old

> You are 6. You can read only short, easy words, and you skip long sentences. You have played a few simple games on a tablet, and you like tapping or clicking things that look fun. You do not know what W A S D means unless the game shows you. You get bored fast: if nothing fun happens for a while, or you cannot work out what to do, you want to stop. You like bright colours, fire and splashing water. Talk like a 6-year-old in your turn notes ("I click the fire!").

### 9yo: nine-year-old

> You are 9. You read fine but skim anything long. You have played Minecraft and Roblox, so you know WASD to walk and the mouse to look. You want to know quickly what the goal is and you like winning. You will try things for a while, but you get frustrated if the game does not tell you what to do next, and you quit when it gets boring. Talk like a 9-year-old in your turn notes.

### 13yo: thirteen-year-old

> You are 13. You play shooters and adventure games on PC and are good with mouse and keyboard. You are quick to call something "babyish" or "boring" and you notice when controls feel bad (slow turning, floaty movement, missed clicks). You will explore and try to break things. You care about whether it is actually fun, not whether it is educational. Talk like a 13-year-old in your turn notes.

### teacher: chemistry teacher

> You are a secondary-school chemistry teacher deciding whether to show this to your classes (ages 6 to 16). You are an occasional gamer at most. You read every on-screen text and check that the science is right (formulas, temperatures, what each spell does). You think about whether your youngest and least-confident students could play it without help, and whether it would hold a class's attention. Talk like a teacher in your turn notes.

### qa: QA tester (4x CPU throttle)

> You are a professional QA tester on a slow school laptop (the session runs at 4x CPU throttle). Hunt for blockers: soft-locks, states you cannot leave, controls that stop working, the pause screen, restarting with R, losing on purpose, text that overlaps or is cut off, and any `console_errors_new` above 0. Try odd inputs (Escape at odd times, the wheel, right click, holding keys). Log every problem with the turn number. Talk like a QA tester in your turn notes.

## Rubric (self-report, asked after `stop`)

Write `<dir>/answer.json`:

```json
{
  "model": "haiku",
  "clarity": 7,
  "fun": 6,
  "blockers": [{ "kind": "goal", "what": "I did not know which bush to spray", "turn": 12 }],
  "would_play_again": "maybe",
  "best": "Spraying water on the fire.",
  "worst": "Walking was slow.",
  "quotes": ["I click the fire!"],
  "stopped_because": "quit",
  "quit_reason": "I won and wanted to stop."
}
```

- `clarity` 1–10. 1 = "I didn't know what to do." 4 = "I mostly guessed." 7 = "I usually knew what to do next." 10 = "I always knew what to do."
- `fun` 1–10. 1 = "I wanted to stop." 4 = "It was OK for a minute." 7 = "I liked it and kept going." 10 = "I didn't want to stop."
- `blockers`: list of `{kind, what, turn}`, kind one of `control` (could not move, look or cast as intended), `goal` (did not know what to do), `bug`, `perf`, `text` (could not read or understand the words).
- `would_play_again`: `yes`, `no` or `maybe`.
- `best`, `worst`: one sentence each, in the persona's voice.
- `quotes`: at most 3 short lines copied word for word from your turn notes.
- `stopped_because`: `quit` or `blocked` (the harness sets `turn_cap`/`time_cap` itself; `error` if the harness failed).

Any `measured` or timing field in the answer is ignored with a warning: the harness's values always win.

## Result schema

`tests/e2e/panel/result.schema.json` (JSON Schema draft 2020-12) is the source of truth; `report` and `score` validate against it.

```
{ schema: "panel-result/1", persona: "6yo|9yo|13yo|teacher|qa", model, artifact_sha, query, throttle,
  measured: { time_to_move_s|null, time_to_cast_s|null, minutes_played, quests_completed,
              outcome: "playing|won|lost", turns, console_errors, panics },
  self: { clarity 1-10, fun 1-10, blockers: [{kind, what, turn}], would_play_again, best, worst, quotes (<=3) },
  stopped_because: "quit|turn_cap|time_cap|blocked|error", quit_reason? }
```

## Scoring

From bars.md:

- **B4** "In panel playtests each persona moves within 10 s and lands a cast within 30 s. Median clarity >= 8/10. No persona reports a control blocker."
- **B5** "Median fun >= 8/10 across the 5 personas, and each persona's fun >= 6."
- **B8** "The slice has >= 15 minutes of guided play for a 9-year-old persona (at least 3 quests, 6 spells, a free-play area and a skill-tree screen), measured by panel play time."

What `score` computes (`tests/e2e/panel/score.mjs`):

- **B4 passes** when every persona has `time_to_move_s` ≤ 10 and `time_to_cast_s` ≤ 30 (a `null` fails), the median clarity (the middle of the 5 sorted values) is ≥ 8, and no persona reports a blocker of kind `control`.
- **B5 passes** when the median fun is ≥ 8 and every persona's fun is ≥ 6.
- **Run quality first.** If any persona has `panics` > 0, `stopped_because:"error"`, fewer than 20 turns, more than 10 % of its turns stalled, or a throttle other than its default (qa 4, others 1; `--allow-throttle` lifts only this one), every bar gets `"pass":false, "reason":"run quality"` and the line lists `problems` (persona and field). A crashed or barely played session cannot judge a bar. Model-routing mismatches (qa not opus, and so on) are listed under `warnings` without failing: the operator assigns models.
- **B4 also needs `throttle_ok`**: qa's `cpu_bench_ms` divided by the 9yo's must be > 2 (a missing value fails), so "qa on a slow laptop" is really exercised.
- **B8 passes** when the 9yo's `minutes_played` (engaged time) is ≥ 15 and its `engaged_acts` is at least 2 per minute played. Its content half (≥ 3 quests, 6 spells, a free-play area, a skill tree) is not visible to the harness and is recorded as `"content_check":"manual"`.
- Fewer than 5 valid results, or results that are not the five distinct personas, give every bar `"pass":false, "reason":"incomplete panel"`. Results with different `artifact_sha` or `query` give `"reason":"mismatched runs"`. Invalid result files (for example `schema: "panel-result/0"`) are listed under `rejected` with the failing field.

Panel line appended to `scoreboard.jsonl`:

```
{"ts", "kind":"panel", "source":"panel", "artifact_sha", "query", "personas":[5 ids],
 "per_persona":{id:{ttm, ttc, clarity, fun, minutes, blockers_control}}, "problems":[...], "warnings":[...],
 "B4":{"pass", "median_clarity", "all_move_10s", "all_cast_30s", "control_blockers", "throttle_ok", "throttle_ratio"},
 "B5":{"pass", "median_fun", "min_fun"},
 "B8":{"pass", "minutes_9yo", "engaged_acts_9yo", "quests_completed_9yo", "content_check":"manual"}}
```

## Tests

- Unit tests (no browser): `node --test tests/e2e/panel/`
- Selftest (scripted robot on a real build, uses `--truth` and the debug path as the operator, and plays persona calls without the role): `node tests/e2e/panel/selftest.mjs` (env `ROOT`, `QUERY`, `OUT`). It also checks the barrier (run dir contents, refused truth through the CLI and straight to the port), the CPU benchmark ratio, and a negative control: a stub page whose frame counter never moves must give `stalled:true`.
- Selftest panel seconds are not representative of a persona: the robot waits and retries far more than a player would.

## Known harness facts

- The daemon launches new headless Chromium (`channel: "chromium"`). The default headless shell was OOM-killed in this cloud shortly after a pointer-lock click (renderer above 5 GB within a minute), so it is only a fallback.
- A press while the mouse is captured is sent in place (no move first), because a move would send a stray `movementX` and turn the view.
- When a real mouse move under pointer lock does not turn the view, `move` falls back to a synthetic `mousemove` with `movementX`/`movementY` on `document` (the same handler a real mouse feeds) and reports `"via":"synthetic"`.
