# Target: playable public beta of the first-person 3D slice

**Done when:** every bar in bars.md passes for 3 consecutive nightly scorings. At that point all Paawan has to do is run `npm run deploy`, send the link to teachers, and read player counts and age bands on the dashboard.

## Hard rule (Paawan, 2026-10-29; reworded 2026-10-30)
The game is a **first-person 3D game**. You move and look like a shooter, but the core is crafting and mastering real-chemistry spells; casting at a target is one way to use them, not the game.
- WASD to move, mouse to look, a crosshair, and click to cast from the player's gauntlet.
- It is not a 2D game drawn in 3D, and it has no 3/4 or top-down camera. Any spec that drifts from this is `wrong-thing`.
- The 2D game (web/) is retired. It stays live only until the 3D slice passes its first playtest.

## Core loop (Paawan, 2026-10-30; roadmap doc https://claude.ai/artifact/7kDsNoHNmPDVauroqLczSJ)
- Every spell moves Unknown -> Learning -> Practised -> Mastered. You learn it by building its formula at the Formula Bench, at your age band's level (Sprout 4-7 picture cards, Seedling 8-10 atom tiles, Apprentice 11-15 typed formulas, Adept 16+ equations with conditions).
- Before mastery every cast is a **worked cast** (rebuild the formula in a quick overlay, about 3-10 s). Worked casts earn XP even before mastery.
- Mastery = 3 unaided correct builds spread over 2 days + 1 "predict what happens" question in a new situation. Then the spell is an **instant cast** from the hotbar and becomes moddable. Mastery never fades; an optional keep-it-sharp check only adds a bonus.
- Crafting: free for Apprentice and Adept (the engine shows what really happens, including nothing); guided recipes for Sprout and Seedling. A spell = a real reaction + settings (amount, form, temperature, trigger); energy is the mana. Wrong or unbalanced formulas fizzle with a kind reason.
- Levels form a chain, mapped to NGSS bands: Greenhouse, Frozen Gate, Crystal Lab, Marsh, Old Forge. Each needs the right chemistry, not good aim, and at least one non-shooting use (freeze a bridge, lift with a balloon).
- Later order (Paawan): puzzles and rescues for the beta, then creatures in the beta, then other players in the same world (co-op MMO).
- Saves: browser-only for the beta, then sign-in open to any user (not class codes), then a push into schools.

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
