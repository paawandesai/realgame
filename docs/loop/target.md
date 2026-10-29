# Target: playable public beta of the first-person 3D slice

**Done when:** every bar in bars.md passes for 3 consecutive nightly scorings. At that point all Paawan has to do is run `npm run deploy`, send the link to teachers, and read player counts and age bands on the dashboard.

## Hard rule (Paawan, 2026-10-29)
The game is a **first-person 3D game**, played like a shooter:
- WASD to move, mouse to look, a crosshair, and click to cast from the player's gauntlet.
- It is not a 2D game drawn in 3D, and it has no 3/4 or top-down camera. Any spec that drifts from this is `wrong-thing`.
- The 2D game (web/) is retired. It stays live only until the 3D slice passes its first playtest.

## Buyer and job
- **Who chooses it:** school teachers (Paawan's contacts, mixed grades and subjects), for students aged about 4 to 18.
- **Who plays:** students, with a long-term reach from K-12 to PhD.
- **Launch goal:** 10,000 kids within about 3 weeks of launch.
- **The job:** a game kids *want* to play, where real chemistry and physics are the magic. Casting and crafting follow real science, so playing teaches it.

## What doesn't exist yet
Science games for school are either quiz wrappers that bolt quizzes onto unrelated gameplay, or sandbox sims with no game loop (PhET-style). Shooters and MMOs have no correct science. Our edge:
- spells *are* real reactions, with correct formulas, temperatures and outcomes;
- in a first-person world with skill trees, free play and adventures;
- one control scheme for every age, with aim assist and click-to-target;
- it runs in a browser on school hardware.

The rules engine is deterministic Rust, and it becomes server-authoritative later.

## Human-only (the loop never does or fakes these)
- Real playtests with children, teacher outreach, beta recruitment (Paawan does this)
- Production deploy (`npm run deploy`), Cloudflare account and DNS changes
- Legal review of the privacy notice; any change to what data is collected
- Pricing, spending money, publishing or sending anything outside the repo
- Deleting remote branches or changing branch protection

## Standing assumptions
- The game is free for students. Persona panels never discuss price.
- Panel personas: a 6-year-old, a 9-year-old, a 13-year-old, a chemistry teacher, and a QA tester on a 4x-throttled CPU. All scores are "panel", never "player" data.
- Target hardware is a standard laptop or Chromebook from the last ~4 years running Chrome. A bigger download is acceptable for a better engine.
- AI-generated art is OK only if it fits the low-poly toon world.
