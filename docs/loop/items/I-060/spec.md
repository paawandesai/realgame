# I-060: One Bench

Branch `loop/one-bench`, from `loop/bench-play` (merged up to ebf8065: I-058's source audit and I-059's grade bands). I-058 and I-059 were taken, so this is I-060 (no folder on any branch).

Paawan: the Chemistry Bench is the centre of "Formula, a RealGame world". There used to be two: B in the world opened the old Formula Bench (`bench.js` panel), and the title screen opened the free-play Chemistry Bench (`bench-free.html`). Now there is one. B opens the Chemistry Bench over the world, and learning spells and building formulas to cast happen inside it.

## 1. The overlay

- `bench.js` `startBench()` hosts the Bench page. `#bench` is a full-screen `role=dialog aria-modal` that holds `iframe#bench-frame`, whose `src` is `bench-free.html?mode=story`. The frame is created on the first open and kept after that, so the next opens are instant. A "Setting up the Bench…" card with "Back to the world" shows until the page says it is ready.
- Why an iframe: the free-play page keeps its own CSS, its 3D scene and its keys. It cannot disturb the world's page, and the same page runs both modes. The page and the world are same-origin, so the page reads the world's state (`__benchState`, `__workState`, `__discState`, `__gameState`, `__storyState`) and pushes commands (`__gameIn.bench`, `__gameIn.disc`, `__gameIn.story`) through `window.parent`.
- B (or E at a Bench) opens it. Opening:
  - frees the pointer lock;
  - pauses the world (`__game.pause(true)`), as the old Bench did;
  - makes `#stage`, `#hud`, `#loading` and `#pause` inert;
  - hides the pause card under it.
- If a pointer lock asked for before B lands after the Bench opened, it is freed again and the world stays paused.
- To close, press Esc (in the page or the world) or "Back to the world" (the page's `#toGame`). Closing restores the pause state from before. The canvas gets focus, and the next click locks the pointer again, as in the game.
- The one modal gate stays: B does nothing while another `aria-modal` dialog shows.
- The title screen's "Chemistry Bench" link still opens `bench-free.html` in free play.

## 2. Story and free play

| | story (`?mode=story`, from the world) | free play (title screen) |
|---|---|---|
| chip under the title | "Story: your bottles come from your pack" | "Free play: every bottle you want" |
| bottles | the pack's bottle items (`content/bottles.json`) and the air (O₂, N₂), each labelled "Iron ×4" or "from the air" | every bottle |
| element jars | hidden (the story's atoms come from the world) | shown |
| a reaction whose bottles are not in the pack | refused: "You don't have that in your pack yet", with where to find each missing bottle | runs |
| running a reaction | **uses nothing up** | uses nothing up |
| band | the world's (`?band=`, the grade picker, or the page's band question). A class link's `?band=` fixes it. | the page's own, saved |
| Guide walkthroughs | run in free play (they need every bottle); the story says so | run |

Running a reaction on the Bench uses no bottles. I-053 says a *cast* uses its bottles up (the world's `Disc` does this when a saved spell is cast), so the Bench is where you try things for free, and the world is where they cost.

## 3. What moved from the old Bench

The new Spells tab (`web/bench-spells.js`, `mountSpells`) does everything the old Bench panel did, and more:

- **Starting spells** (Fire, Water, Frost, CO₂). Each shows its stage badge (New, Learning, Practised, Mastered), its key, XP and practice count. Learn or Practise opens the same formula builder (`bench.js` `mountBuilder`: Sprout picture recipe and atom cards, Seedling scale and tiles, Apprentice typed formula), and the game's Rust judge (`bench.rs` `judge_with`, `spells::book().judge`) decides.
  - A fizzle shows the band's reason (`reasonFor`). At Sprout it adds the picture diff; at Seedling it adds the before/after counts.
  - The done panel shows the done line, the stage badge, "You learned X! Cast it with key N.", the Apprentice simplest-form and coefficient lines, and the progress to Practised.
  - Speech, the 48 px and 18 px sizes (Sprout 96, 120 and 24), the text size setting, arrow-key focus and reduced motion all work as on the old Bench.
- **Spells you saved** (I-053 and I-056 discoveries).
  - Learn or Practise at the Bench goes through the new `learn <codex id>` Bench command. Rust judges a saved discovery and records its builds (`Disc::record_build`, `MASTERY_BUILDS = 2`).
  - Put on key 1–4 (`equip` and `unequip` to the world's discoveries, or `putOnKey` in free play).
  - The tab also shows what the spell does in the world, what it uses up, and where to find the bottles.
- **Found, not saved yet**, with a Save button.
- **Quests**, the same list the world shows.
- **Make things** (story only), the pack recipes (I-052).

What was retired: the old `#bench` panel views (band chooser, spell list, build, fizzle, done, make) and the `?debug` `__bench.fake()` hook. `bench.js` keeps its pure helpers and `mountBuilder` (the worked-cast builder `worked.js` still uses them).

## 4. One save

| what | key |
|---|---|
| discoveries | `formula.bench.found` |
| saved spells, build counts, keys and quests | `formula.spells.v1` |
| the starting spells' stages and XP | `rg.progress.v1` |

The world (`discover.js`) and the Bench page read and write the same keys. The world reloads on `storage` events. Free play reads the starting spells' progress read-only (`readProgress`), and counts saved spells' builds with `recordBuild` (merges keep the larger count).

## 5. Tests

- `tests/e2e/one-bench.mjs` (new). B opens the one Bench in story mode with the pack's bottles. Fire is learned by building its formula. Closing the Bench puts Fire on the hotbar as Learning. The title screen's Bench is free play and shows Fire as Learning from the same save.
- `tests/e2e/golden.mjs` `formulaBench` (tests 12–28b and round 2) and `valleyPuzzles` P1 now drive the Bench inside the frame. Every check is kept: the old selectors map onto the one Bench's, the size, text and contrast checks measure the Spells tab, and the animation probe builds Water for real instead of using the retired `fake()`.
- `golden.mjs` test 17 now loads with `voice: true` saved. Spoken lines have been opt-in since db6ddf1 ("Read lines aloud"), so its three speech checks were failing before this item too. They now check what is spoken when voice is on, and the sound-off check still checks that nothing is.
- `tests/e2e/analytics-e2e.mjs` test 15 reads the chooser and cards from the frame.
- Unit tests: `test/bench-spells.test.ts` (shared-save helpers), and `test/bench-free.test.ts` and `test/bench-layout.test.ts` (the new files and the Spells tab).

## 6. Adept in the world's grade picker

Paawan approved Adept as a fourth choice in the world's grade picker (I-059 had left it for a separate decision because of analytics).

- **The picker.** `index.html` `#age` now has four pictures: "Sprout · Grades 6–7", "Seedling · Grades 8–9", "Apprentice · Grades 10–11" (relabelled from "Grade 10+") and "Adept · Grade 12+". `age.js` expects four buttons, and the arrows and Tab move between all four. The Bench's band question copies the picker's pictures, so it also offers four.
- **The world.** Adept is its own band (`Tier::Adept`). `bench.js` `bandFrom` no longer reads Adept as Apprentice, and `startBench` already sent all four bands. Only the formula builder's layout (`bandOf`, used by `mountBuilder` and `reasonFor`) builds Adept the way it builds Apprentice, by typing.
- **Analytics.** `adept` is a fourth anonymous band value, and nothing personal is added:
  - `functions/_lib/analytics.ts` `BANDS` and `zeroBands`;
  - `crates/bevy-client/web/analytics.js` `BANDS`;
  - `migrations/0003_adept_band.sql`, which rebuilds `events` with the wider `band` CHECK (same columns, same CHECKs, STRICT kept);
  - the dashboard: `dashboard/dashboard.js`, `dashboard/index.html` and `dashboard/dashboard.css`;
  - `docs/ANALYTICS.md`.

  The Classic (2D) game still sends only its three bands.
- **Tests.** Tests that pinned three bands now pin four:
  - `test/handlers.test.ts`, `test/quest.test.ts` and `test/validate.test.ts` (band counts, plus a new migration 0003 test and a new adept-event test);
  - `test/bevy-analytics.test.ts`;
  - `test/bench.test.ts` (`bandFrom`);
  - `tests/e2e/analytics-e2e.mjs` (four picker buttons, the arrows reach Adept, the Bench's chooser has four);
  - `tests/e2e/golden.mjs` (the Bench's band question has four 160 px pictures).
