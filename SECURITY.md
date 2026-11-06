# Security and child safety

This game is built for school-age children, so privacy and safety are requirements, not features.

## Reporting a problem

Please report security or child-safety issues privately to the maintainer through GitHub's private vulnerability reporting on this repository, not in a public issue. We aim to acknowledge reports within 3 working days.

## What the game collects

Anonymous play counts, and nothing that identifies a child. The full contract is in `docs/ANALYTICS.md`.

- **Sent:** when play starts, the age band picked (4 to 7, 8 to 10, or 11 and up); which rooms are cleared and whether the trial is finished; and a coarse session length (under 1 minute, 1 to 5, 5 to 15, 15 to 30, or 30 minutes and more). Each event also carries a random number the browser replaces every UTC day, so the game can count each day's players without being able to recognise anyone from one day to the next.
- **Never collected:** names, emails, ages or birth dates, anything typed into the game (formulas included), cookies, IP addresses, user agents, device fingerprints or location.
- **Where it goes:** only to the game's own address (`/api/event`), stored in the game's own Cloudflare D1 database. No third-party analytics, ads, scripts, fonts or trackers. Cloudflare is the hosting provider.
- **Opt-out:** if the browser sends Do Not Track or Global Privacy Control, nothing is sent.
- **Retention:** raw events are deleted after 90 days.
- **Who can see it:** only the aggregate dashboard, behind a secret token (optionally also Cloudflare Access).
- **On the device:** the chosen age band and the daily random number, in the browser's local storage. The game works the same if storage or the network is blocked.

When accounts arrive (teacher sign-in, class codes), the rules in `docs/ENGINEERING.md` apply: no student emails or real names, data minimisation, a published retention policy, and a written security program, as the amended COPPA rule requires.

## Protections in place

- The simulation is memory-safe Rust with `unsafe` code denied, compiled to sandboxed WebAssembly.
- Formula input is length-limited (80 characters), overflow-safe, and fuzz-tested with 20,000 random inputs in the test suite.
- Production responses carry a strict Content Security Policy and other security headers (`web/public/_headers`); the analytics Functions set their own (`no-store`, `nosniff`, `default-src 'none'`).
- The analytics endpoint accepts only four fixed event shapes from the same origin, rejects anything else, caps each device at 120 events a day, and stores no IP address or user agent. The stats endpoint compares its token in constant time and refuses to run without one.
- CI blocks merges on lint warnings, failing tests, known-vulnerable dependencies and unapproved licences.
