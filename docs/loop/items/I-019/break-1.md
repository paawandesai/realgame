# I-019 break 1 (wave-7 breaker, fresh)

Branch head 7ca36d5. Base origin/main.

## Verified
- Lanes: all 8 changed non-docs files are in the spec's lanes (cues.rs, fx.rs, main.rs, world.rs, hud.js, style.css, golden.mjs, science-sources.md). No index.html, chem.rs, clock.rs, feel.rs, player.rs edits. The only existing golden check edited is the sanctioned `keysOk`.
- Unit tests: cues 5/5, chem 19, feel 4, clock 3, motion 8 all pass. `npm test` passes.
- Build (wasm, shared target dir): OK.
- Golden `QUERY=?low` under the lock: 98/98, no console errors.
- Playwright play-through (1280x720, ?low): boot spell is Water, counter reads "Fires to put out: 3" with 3 flame pips, one click sent `fire` (clicks 0->1, casts 0->1, stream 1, no errors). Crosshair gets `on fire` with an orange ring on a burning bush (shot 03). Finder chevron+flame shows at the left edge for a fire behind (bearing -122), turning toward it clears it (shots 04, 03). Hot bushes no longer glow.

## Blocking
None.

## Should-fix
1. hud.js:667 hint priority puts the idle hint above `note`; spec 9 says banner > note > idle. Deviation is documented in the web round notes (needed to keep an I-007 check), but the spec and code disagree. Either amend the spec or reorder and fix the I-007 check setup.
2. main.rs:124 (`STREAM_THICK`) / stream colour: the Water stream is a thin pale-blue cone. Against the sky (shot 05) the far end has very low contrast and reads as a thin line, not "a stream". A 7-year-old may not notice it; consider thicker (0.2 m) or a brighter/opaque material. Golden only measures pixel change (about 0.3% of viewport), which passes with a faint stream.
3. hud.js:673 / main.rs:991: the finder points to the nearest fire even when another fire is already under the crosshair (shot 03: crosshair on a fire, arrow to the right). For a child it reads as "go somewhere else". Suppress the finder while `aimObj` is burning, or while any burning object is on screen (fireOff is only about the nearest one).

## Nits
- cues.rs:78: `Frosty` keeps emissive 0.25 (a faint self-glow on frosted objects), the same kind of implied visual claim the item removed for `Hot`. Cold objects do not glow.
- Flight cap (cues.rs, MAX_FLIGHT 0.25): CO2 at 18 m flies at about 72 m/s and Water at the same, so the 20 vs 30 m/s difference is no longer visible. No science claim is made, so only a note.
- main.rs:1076 (held path): at 4 fps a click whose release falls after the next frame sample casts twice (fire + hold), 0.3 s apart. Pre-existing hold behaviour; the seam spec only forbids two in one frame.
- The fire object is a peach cone on a red-brown base, not a flame; with the finder icon (a flame) the two do not match visually for a young child. Out of scope for this item.
- Not run by this breaker: test 21 (perf.mjs pairs, brotli delta). The tester owns it.
