# I-042 (proposed id; the leader confirms it when filing the backlog row): spell_learned / spell_mastered analytics events

**Split from I-027 revision 2** (critique change 6). Held behind **ACTION-1** (human-only legal review). The loop opens this item's PR as a draft and does not merge it. It needs I-027 main and I-022 merged first. It reads only I-027's published `progress` key; it changes no Rust.

**Bars moved:** B7 (Deploy-ready with analytics), only after ACTION-1: `spell_mastered` becomes countable on the dashboard's data path. Without approval nothing changes.

## Analytics proposal (needs ACTION-1 before any of it merges)

The minimal events, in the existing pipeline and the existing format:

```json
{"v":1,"type":"spell_learned","band":"seedling","spell":1,"id":"9f2c4e0b7a1d4c3e8b6f0a2d4e6c8b1a"}
{"v":1,"type":"spell_mastered","band":"seedling","spell":1,"id":"9f2c4e0b7a1d4c3e8b6f0a2d4e6c8b1a"}
```

- `spell` is an integer index, 0 to 15, into the game's fixed spell list (the `progress` array order). It is the same kind of value as `quest`, and it is stored in the existing `room` column. No new column, no free text, nothing typed. A built formula is never sent.
- **Sent when:** the 500 ms analytics poll sees a spell's `st` change between two states of the same session, to `learning` from `unknown` (`spell_learned`) or to `mastered` from any other stage (`spell_mastered`). Never on the first state of a session (a loaded save is not a new event). At most once per spell per type per session.
- **Why both:** `spell_mastered` is the B9 count ("players who master a spell, by band"). `spell_learned` is its denominator ("of those who learned one"). If only one is approved, keep `spell_mastered`.
- **What it reveals, honestly:** with the daily id, the server can see that one device learned or mastered spell N on that UTC day, the same linkage `quest_complete` already has. Because mastery needs 2 days, `spell_mastered` implies the device played on an earlier day. The day is not sent, and the daily id still cannot link days.
- **Edge:** with storage blocked, progress resets on each load, so the same device can send `spell_learned` again after a reload. The dashboard counts are then events, not unique learners. The cap (120 per device per day) is unaffected (at most 8 such events per session with 4 spells).
- **What the held lane changes:** `analytics.js` (`EXTRA_KEYS` gains both types with `['spell']`, `eventBody` validates `spell`, new pure `progressEvents(prev, state, sent)`, and `createSession.poll` sends them); `functions/_lib/analytics.ts` (`EVENT_TYPES`, `KEYS_BY_TYPE`, `MAX_SPELL = 15`, and `spell` validated and stored in `room`); `migrations/0003_spell_events.sql` (the same table rebuild as 0002, with the two types added to the `CHECK`, same columns, `STRICT`); `docs/ANALYTICS.md` (event table, examples, "On the device" gains `rg.progress.v1`, which never leaves the device); `SECURITY.md` (one line each); tests. The dashboard is unchanged (follow-up).
- **On-device key, also in ACTION-1 for the notice text only:** `rg.progress.v1` holds per-spell stage, XP, build counts, one local day number per spell and the predict flag. It is not sent anywhere, like the existing `alchemist.settings.v1`. Storing it does not wait on review (it is not collected); describing it in the privacy notice does.


## Acceptance tests (numbered as in I-027 rev 1, so earlier references still match)

Written and run in this item's draft PR.

26. `test/bevy-analytics.test.ts`: `eventBody` accepts both types with `spell` 0 and 15, and rejects `spell` 16, −1, 1.5, `"1"`, a missing `spell`, an extra key, and `quest` in place of `spell`. `progressEvents(prev, state, sent)`: `unknown → learning` gives `[['spell_learned', i]]`; `practised → mastered` and `learning → mastered` give `spell_mastered`; `prev === null` gives none; a repeat after `clear` and relearn in the same session gives none; a state without `progress` gives none.
27. `test/handlers.test.ts` (or the existing server test file for `validateEvent`): the same accept and reject cases on the server, `spell` stored as `room`, and the old five types unchanged.
28. Migration 0003 applies with `npm run db:migrate:local` over a database holding 0002 rows, keeps every row, and rejects an insert with type `spell_tamed`.
29. `tests/e2e/analytics-e2e.mjs` (append only): a scripted run with `?debug` that does `prog learn 1` and the mastery commands of test 19 sends exactly one `spell_learned` and one `spell_mastered` with `spell: 1`, and D1 holds them with `room = 1`. A run with DNT set sends neither.


## Lane

| lane | owns | builds |
|------|------|--------|
| analytics (**held: draft PR on `loop/I-027-analytics`, not merged by the loop; needs ACTION-1 and I-022 merged**) | `crates/bevy-client/web/analytics.js`, `functions/_lib/analytics.ts`, `migrations/0003_spell_events.sql` (new), `test/bevy-analytics.test.ts`, the server validation test file, `tests/e2e/analytics-e2e.mjs` (append only), `docs/ANALYTICS.md`, `SECURITY.md` | Tests 26–29 first, then the changes in "Analytics proposal". Opens a draft PR titled "I-027 analytics: spell_learned / spell_mastered (needs Paawan's review)" and stops. Must not touch `dashboard/**`, `hud.js`, `age.js`, `boot.js` or Rust |

## ACTIONs (human-only)

- **ACTION-1 (Paawan, legal review; blocks only the held analytics lane).** Approve, trim or reject: (a) two new event types `spell_learned` and `spell_mastered` with one field `spell` (integer 0–15, stored in the existing `room` column), shapes in "Analytics proposal"; (b) migration `0003_spell_events.sql`, which widens the `type` `CHECK` and adds no column; (c) the privacy-notice text in `docs/ANALYTICS.md` and `SECURITY.md` for these events and for the on-device `rg.progress.v1` key. Recommended minimum if you want only one: `spell_mastered`. Until you answer, nothing new is collected, and the main item ships without it.
