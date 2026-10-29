# Security and child safety

This game is built for school-age children, so privacy and safety are requirements, not features.

## Reporting a problem

Please report security or child-safety issues privately to the maintainer through GitHub's private vulnerability reporting on this repository, not in a public issue. We aim to acknowledge reports within 3 working days.

## What the game collects

Today: nothing. The game runs entirely in the browser. It stores only the chosen age band in the browser's local storage, and makes no network requests after loading. There are no accounts, analytics, ads, third-party scripts, fonts or trackers.

When accounts arrive (teacher sign-in, class codes), the rules in `docs/ENGINEERING.md` apply: no student emails or real names, data minimisation, a published retention policy, and a written security program, as the amended COPPA rule requires.

## Protections in place

- The simulation is memory-safe Rust with `unsafe` code denied, compiled to sandboxed WebAssembly.
- Formula input is length-limited (80 characters), overflow-safe, and fuzz-tested with 20,000 random inputs in the test suite.
- Production responses carry a strict Content Security Policy and other security headers (`web/public/_headers`).
- CI blocks merges on lint warnings, failing tests, known-vulnerable dependencies and unapproved licences.
