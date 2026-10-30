# I-011: Gameplay standards (doc item)

Base: `origin/main`. Client read for the "known gaps" table: `claude/fps-prototype` (PR #13).

## What it is

A doc-only item. The planner wrote `docs/loop/gameplay-standards.md` directly. It is a numbered checklist (GS-1 to GS-32) of first-person game-feel, accessibility, ages 4 to 18 design, ethical design and learning-design standards. Each item has a standard, a why, a check, and a source URL that was fetched during research. Items are marked P0, P1 or P2, and the doc says how each role uses them:
- the spec critic walks the list for every screen or control a spec touches;
- breakers cite GS numbers and file P0 as Must-fix and P1 as Should-fix;
- persona-panel prompts include the **[panel]** items.

The main sources are the Game Accessibility Guidelines, the Xbox Accessibility Guidelines (101 to 103, 105, 107 to 109, 112, 116 to 118, 121), WCAG 2.2, Nielsen Norman Group research on children and teens, Apple's Kids guidance, Google's Teacher Approved criteria, the AAP media guidance, research on FPS sensitivity (CHI 2022), latency and impact feel, Habgood and Ainsworth on intrinsic integration, the "four pillars" of educational apps, and a JAMA study of manipulative design.

## Bar

Supports B4 (every age can play), B5 (fun) and B8 (enough to play) by giving the critic, breakers and panel shared, sourced criteria. No bar's pass rule changes.

## No client change

The file ends with a "known gaps" table (FOV 70 deg vertical, about 102 deg horizontal; URL-only sensitivity and assist settings; viewmodel bob with no toggle; HUD text from 11 to 16 px). Each gap is a candidate backlog item, to be confirmed by a breaker on the artifact.

## Critic asks

- Is every source real, and does it say what the item claims?
- Is every check runnable by our roles: headless Playwright on a software GPU (CPU frame ms, not fps), or a persona panel?
- Do any thresholds conflict with bars.md? The bars win.

## Follow-up (not in this item)

- Wire the GS list into the critic, breaker and persona-panel prompts: update the I-006 harness and the critic template.
