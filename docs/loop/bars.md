# Bars

| id | Bar | Pass when | Checked by | Level |
|----|-----|-----------|------------|-------|
| B1 | Golden path works | Scripted first-person playthrough (move, look, select each spell, cast, complete every quest in the slice) passes 3/3 runs with 0 console errors and 0 panics | `smoke` command on the artifact | 0 |
| B2 | Smooth on real hardware | On Paawan's Mac (M5 Pro, Chrome), WebGL2 p5 fps >= 55 over 60 s of the golden path. In cloud headless at 4x CPU throttle, the median CPU frame is <= 16 ms with `?low` | Mac Remote Control run + `perf` script | 2 |
| B3 | Fast to load | Brotli download <= 15 MB total. Time to first interactive frame <= 8 s at 4x CPU throttle in cloud headless | `perf` script | 2 |
| B4 | Every age can play | In panel playtests each persona moves within 10 s and lands a cast within 30 s. Median clarity >= 8/10. No persona reports a control blocker | Persona panel (5 personas, fresh contexts) | 5 |
| B5 | Fun | Median fun >= 8/10 across the 5 personas, and each persona's fun >= 6 | Persona panel | 5 |
| B6 | Science is right | Science reviewer (a model that did not write the content) checks every formula, temperature and outcome shown in game against a cited source: 0 errors, 100% of claims sourced in `content/science-sources.md` | Science review agent + source file lint | 2 |
| B7 | Deploy-ready with analytics | `npm run build:site` packages the 3D client. Local `wrangler pages dev` receives session_start (with age band), quest_complete and session_end from a scripted run, and the dashboard query returns them. Privacy rules (no IP/UA, DNT/GPC honoured) still pass tests | `analytics-e2e` script | 1 |
| B8 | Enough to play | The slice has >= 15 minutes of guided play for a 9-year-old persona (at least 3 quests, 6 spells, a free-play area and a skill-tree screen), measured by panel play time | Persona panel | 5 |
