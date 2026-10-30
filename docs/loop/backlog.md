# Backlog
Ordered: top is next. One line per item.

| id | Item | Moves bar | Track | After | Status | Updated |
|----|------|-----------|-------|-------|--------|---------|
| I-001 | Land the first-person prototype (branch claude/fps-prototype): courtyard, gauntlet, 4 spells, Save the Greenhouse, aim assist. PR to main with CI covering the bevy-client crate [bootstrap:build] | B1 | product | — | merged | 2026-10-29 |
| I-002 | Smoke: Playwright golden path over the artifact (pointer lock or click-to-target, every spell, quest win, console-error check) as `tests/e2e/golden.mjs` [bootstrap:smoke] | B1 | measurement | I-001 | merged | 2026-10-29 |
| I-003 | Perf script: Brotli size, time to first frame, CPU frame ms at 1x/4x, `?low`; plus a Mac-run recipe [bootstrap:perf] | B2,B3 | measurement | I-001 | scoring | 2026-10-29 |
| I-004 | Science sources file + reviewer protocol: every in-game claim listed with a citation; lint that fails on unsourced claims | B6 | measurement | — | merged | 2026-10-29 |
| I-005 | Wire anonymous analytics into the 3D client (age picker with 3 pictures, session_start/quest_complete/session_end), build:site packages the 3D client, analytics-e2e script [bootstrap:analytics-e2e] | B7 | infra | I-001 | proposed | 2026-10-29 |
| I-006 | Persona panel harness: 5 personas drive the artifact via Playwright screenshots and actions, report time-to-move, time-to-cast, clarity, fun and minutes played | B4,B5,B8 | measurement | I-002 | scoring | 2026-10-29 |
| I-007 | Feel pass: mouse sensitivity, FOV, head bob off by default, crosshair feedback, hit markers, cast cooldown ring, recoil | B4,B5 | product | I-001 | critiqued | 2026-10-29 |
| I-008 | Content: quests 2 and 3 (water-splitting lab station, liquid-nitrogen Frost puzzle), spells 5–6, free-play area, skill-tree screen | B8,B5 | product | I-001 | critiqued | 2026-10-29 |
| I-009 | Art pass: luminous low-poly toon look (lighting, skybox, materials, CC0 kits), with a `?low` tier | B5,B2 | product | I-001 | proposed | 2026-10-29 |
| I-010 | Science lint hardening from I-004 break report: source for foam/CO2 claims (C-012/C-018), keyword-free claims and '{:' skip gap, verify S8 (US Std Atmosphere 1976) | B6 | measurement | I-004 | merged | 2026-10-29 |
| I-011 | Gameplay standards checklist (docs/loop/gameplay-standards.md) from accessibility and FPS/kids design sources; applied by critic, breakers, panel | B4,B5 | rethink | — | merged | 2026-10-30 |
| I-012 | Input robustness and win-condition fixes from I-001 round-2 breaker (pointerlockerror, movement guards, level can stall, spread farming, stray cast) | B1,B4 | product | I-001 | building (r1) | 2026-10-30 |
| I-013 | Perf tool hardening (I-003 break): Mac DPR 2, tee exit code, visibility/SwiftShader guard, requested-files size, timer TTI, sample range, throttle calibration, blank-canvas check | B2,B3 | measurement | I-003 | proposed | 2026-10-30 |
| I-014 | Panel harness hardening (I-006 break): enforce info barrier (no --truth for personas, logs outside persona dir, persona cwd outside repo), B8 minutes not gameable by idling | B4,B5,B8 | measurement | I-006 | proposed | 2026-10-30 |
| I-015 | Load and CPU budget: B3 TTI 16.7 s@4x (target 8 s), B2 CPU 33.8 ms@4x ?low (target 16 ms) per first I-003 cloud run; profile and cut (startup asset work, per-frame allocs, JSON state each frame) | B2,B3 | product | I-003 | proposed | 2026-10-30 |
| I-016 | Apply gameplay-standards P0s to the client: FOV 70°v→~59°v (GS-1), plus other P0 failures listed in gameplay-standards.md | B4,B5 | product | I-011 | proposed | 2026-10-30 |
