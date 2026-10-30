# Gameplay standards

The checklist the **spec critic**, the **breakers** and the **persona panel** apply to every user-facing item. It covers first-person game feel, accessibility, design for players aged 4 to 18, and learning design. It sits under `target.md` (first-person, WASD + mouse look + crosshair + click to cast). When this file conflicts with a bar in `bars.md`, the bar wins and the conflict goes in the round notes.

Each item gives the standard, why it matters, how we check it, and the source. Every source was fetched while this file was written (2026-10-30).

## Priority and how each role uses it

- **P0, must.** The critic rejects a spec that breaks a P0 or leaves one unaddressed in a screen it touches (`wrong-thing` or `incomplete`). Breakers file a P0 failure as **Must-fix**. Panel personas report it as a blocker.
- **P1, should.** The critic asks the spec either to meet it or to state why it is deferred. Breakers file it as **Should-fix**.
- **P2, nice.** It goes in the backlog. Breakers file it as a **Nit**.

How each role applies the list:

- **Spec critic:** for every screen, control, effect or text the spec adds or changes, walk the list. Name each item number the spec meets, defers or breaks.
- **Breakers:** run the check in the "Check" column for each item the change touches, and cite the item number (for example "GS-7") in the break report.
- **Persona panel:** each persona answers the items marked **[panel]** from that persona's point of view (age, reading level, hardware), and quotes the item number when reporting a problem.

Unit notes:

- Bevy's `PerspectiveProjection.fov` is **vertical**. Horizontal FOV = `2*atan(tan(v/2) * aspect)`. A vertical FOV of 60 deg gives about 91 deg horizontal at 16:9, which matches Source's 75 deg (4:3 horizontal) scaled to widescreen.
- Cloud fps is meaningless on the software GPU. Timing checks use CPU frame ms and event timestamps, not fps.

---

## A. Camera and motion comfort

**GS-1 (P0) Default FOV suits a laptop or monitor.** The default is about 90 deg horizontal at 16:9, which is a vertical `fov` of 55 to 62 deg in Bevy. It must not exceed about 100 deg horizontal.
- *Why:* a FOV far from what the eye expects on a near monitor causes motion sickness "severe enough to make play lasting more than a few minutes impossible". The guideline recommends 90 deg for a monitor and 60 deg for a TV. Source games default to 75 deg (4:3), which scales to about 91 deg at 16:9.
- *Check:* critic and breaker read the `PerspectiveProjection` value in `main.rs` and convert it to horizontal at 16:9. **Current client:** `fov: 70 deg` vertical, which is about 102 deg horizontal and fails.
- *Source:* https://gameaccessibilityguidelines.com/if-the-game-uses-field-of-view-3d-engine-only-set-an-appropriate-default-for-the-expected-viewing-environment/ ; https://developer.valvesoftware.com/wiki/Fov

**GS-2 (P1) FOV is adjustable in game.** A slider or steps cover at least about 70 to 110 deg horizontal. The value is saved (GS-14).
- *Why:* one default cannot fit every screen size and sitting distance. A lower FOV reduces "fishbowl" edge stretch and sickness.
- *Check:* breaker changes the FOV from the settings UI (not only a URL parameter), reloads, and confirms the value persists and the projection changed.
- *Source:* https://gameaccessibilityguidelines.com/if-the-game-uses-field-of-view-3d-engine-only-allow-a-means-for-it-to-be-adjusted/ ; https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/117

**GS-3 (P0) No camera shake, camera bob or motion blur unless it can be turned off.** Viewmodel (gauntlet) bob and any screen shake each get an off switch. Camera-space bob of the view itself is off by default. Shake on a cast or impact is small and optional.
- *Why:* repeated up/down or side-to-side movement that the body does not feel (camera bob, weapon sway, shake) causes a visual-vestibular mismatch. XAG 117 says to avoid these effects or let players disable them.
- *Check:* critic flags any spec that adds shake, bob, blur or sway without a toggle. Breaker turns each toggle off and confirms the transform offset is zero (bridge state or screenshot diff while walking). **Current client:** has a viewmodel bob (`player.rs`), and no toggle has been found.
- *Source:* https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/117

**GS-4 (P0) The camera never moves on its own unless the player can turn that off.** This covers scripted turns, auto-centering and aim-assist rotation. The player's mouse is the only thing that rotates the view by default. Aim assist is the exception: it may nudge the view, but only while it is enabled (GS-10), and it must be toggleable at runtime.
- *Why:* XAG 117 asks for a toggle to disable automatic camera movement. Unexpected camera motion is a listed cybersickness factor.
- *Check:* breaker idles 10 s with no input, near a target and away from one, and confirms that yaw and pitch are unchanged. With assist off, the camera never moves without input.
- *Source:* https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/117 ; https://arxiv.org/pdf/1611.06292

**GS-5 (P0) Frame pacing is steady.** No hitch over 50 ms during the golden path at 4x throttle with `?low`, excluding the first 5 s of load and shader compile (B3 covers load). Bar B2 sets the median.
- *Why:* dropped frames and delays are linked directly to discomfort ("It is crucial to ensure that the VR application does not drop frames or produces delays"). Hitches also break aim.
- *Check:* `perf` script reports the p99 and max CPU frame ms alongside the median.
- *Source:* https://arxiv.org/pdf/1611.06292

**GS-6 (P0) No seizure-risk flashing.** No more than 3 flashes per second over 20% or more of the screen. A flash is a luminance change of 10% or more where the darker state is below 0.8. Saturated red (R/(R+G+B) >= 0.8) has a stricter threshold. No high-contrast striped patterns over 20% of the screen. Cast, explosion and warning effects must meet these limits. A splash warning does not replace them.
- *Why:* photosensitive epilepsy. XAG 118 says to remove triggering content and test every game, including games without intentional flashing.
- *Check:* breaker records a burst of frames (headless screenshots at fixed sim steps) through each spell's cast and impact and the fire warning. A script computes mean relative luminance per frame and the fraction of pixels that changed by 10% or more, and counts the flashes in any 1 s window. Critic rejects specs with strobe or flash effects that have no stated frequency or area. The current `#warning` pulse runs at 1 Hz, which is within the limit.
- *Source:* https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/118

**GS-7 (P1) Players can pause at any time, and the game suggests breaks.** Esc or focus loss pauses everything. Nothing advances while paused. At natural stopping points (end of a quest), the game offers a clean exit with progress saved.
- *Why:* discomfort rises with play time, so the guidelines advise allowing pauses and suggesting breaks. XAG 108 says single-player games are "pausable at any time". The AAP recommends limiting screen use to 1 hour a day for ages 2 to 5 and setting consistent limits for ages 6 and up.
- *Check:* breaker pauses mid-cast and mid-fire, waits 30 s, then resumes, and checks that no timer, fire spread or score has advanced. The panel 6-year-old confirms that pausing is findable. **[panel]**
- *Source:* https://arxiv.org/pdf/1611.06292 ; https://devdocs.xbox.com/build/game-principles/accessibility/xag-deep-dives/xag-108-difficulty-options.md ; https://www.news-medical.net/news/20161021/AAP-releases-new-policy-recommendations-to-help-families-maintain-healthy-media-diet-for-kids.aspx

## B. Controls and input

**GS-8 (P0) The default look sensitivity is comfortable, and it is adjustable in game by at least ±50%.** The sensitivity setting is in the settings UI, not only `?sens=`. The default should fall near 20 to 80 cm per 360 deg for a typical 800 to 1600 CPI mouse, and must still be usable on a Chromebook trackpad.
- *Why:* in a CHI 2022 study, targeting performance was best between 20 and 80 cm/360. Too low makes turning slow. Too high costs precision and adds corrective movements. XAG 107 asks for in-game sensitivity adjustment of at least ±50% of the default for each analog input, including the mouse.
- *Check:* critic or breaker computes cm/360 from `LOOK_PER_PX` x `sens` at 800 CPI (about 315 px/cm). **Current client:** 0.0022 rad/px works out to about 9 cm/360 at 800 CPI, which is faster than the study's best range. Breaker then changes the sensitivity in the UI and confirms that the yaw-per-pixel ratio changes. Panel QA persona tries a trackpad. **[panel]**
- *Source:* https://ar5iv.labs.arxiv.org/html/2203.12050 ; https://devdocs.xbox.com/build/game-principles/accessibility/xag-deep-dives/xag-107-input.md

**GS-9 (P1) Controls are simple, remappable and need no held chords.**
- Movement accepts WASD and the arrow keys. Keys are bound by physical `code`, so ZQSD on AZERTY works.
- Players can remap keys in game. Hints and the controls screen update to match.
- Invert-Y is available.
- No action needs a long hold: sprint and similar actions get a toggle option.
- No action needs two keys pressed at the same time beyond move plus look.
- *Why:* XAG 107 asks for in-game remapping, per-axis inversion, toggles instead of long holds, and no simultaneous button presses. GAG lists remapping and simple controls as basic.
- *Check:* breaker plays the golden path with arrow keys only, then with an AZERTY layout emulated by dispatching `code` values. Critic flags any new chorded input or hold-only input.
- *Source:* https://devdocs.xbox.com/build/game-principles/accessibility/xag-deep-dives/xag-107-input.md ; https://gameaccessibilityguidelines.com/full-list/

**GS-10 (P0) Aim assist is on by default, can be adjusted at runtime, and does not take control from the player.** It works with a mouse:
- Magnetism or friction slows the crosshair over a target, or the cast bends toward the nearest valid target inside a small cone.
- It weakens with distance and stops beyond the spell's range.
- It never snaps the view more than a few degrees.
- Players can toggle it, or set it to a level, from settings without losing progress.
- *Why:* the Game Accessibility Guidelines list assist modes such as auto-aim for players with limited speed or precision and for less experienced players. The target bar promises "one control scheme for every age, with aim assist". Established designs (Halo magnetism, Insomniac's magnetism, centering and friction) show that radius-limited, distance-falloff assist works.
- *Check:* breaker runs a scripted near-miss (a cast aimed 2 to 3 deg off target) with assist on, which should hit, and with it off, which should miss. Breaker checks the far-target falloff and confirms the toggle takes effect without a reload. The 6-year-old persona must land a cast within 30 s (B4). **[panel]**
- *Source:* https://gameaccessibilityguidelines.com/include-assist-modes-such-as-auto-aim-and-assisted-steering/ ; https://halopedia.org/Magnetism ; https://www.gdcvault.com/play/1017942/Techniques-for-Building-Aim-Assist

**GS-11 (P0) The youngest players have a path that needs only the mouse.** Ages 4 to 8 must be able to finish the first quest with mouse look and click, plus at most one movement key. Click-to-target is how they do it: clicking a highlighted target faces it and casts. Coordinated WASD plus mouse aim is not required until a later, optional challenge.
- *Why:* NN/g's observations of children:
  - Ages 3 to 5 have very limited fine motor skills.
  - Ages 6 to 8 have mastered clicking and simple keyboard use, but long drags and precise coordination fail.
  - Complex keyboard-and-mouse coordination arrives around ages 9 to 12.
- *Check:* breaker completes quest 1 using only mouse events and a single movement key. The 6-year-old persona reports no control blocker (B4). **[panel]**
- *Source:* https://www.nngroup.com/articles/children-ux-physical-development/

**GS-12 (P0) Input reaches the screen within one frame.** A mouse move or click changes the camera or starts the cast effect no later than 2 rendered frames after the event (target: the next frame). The game adds no buffering. Click to visible cast feedback takes 100 ms or less (GS-16).
- *Why:* players perceive delays well under 100 ms. In one study the median detection threshold was about 51 to 90 ms, and the most sensitive players noticed 26 to 40 ms. Lower system latency measurably speeds first-person targeting.
- *Check:* in Playwright, timestamp the injected `mousemove`/`mousedown` and poll bridge state (yaw, cast id) on each `requestAnimationFrame`. Pass when the change appears within 2 frames of the event, measured in frames, not wall ms, on the software GPU.
- *Source:* https://cms.simula.no/sites/default/files/publications/files/paper_5.pdf ; https://research.nvidia.com/publication/2019-11_Latency-of-30

**GS-13 (P1) Every menu can be used from the keyboard, and the settings are reachable from the first screen.** Pointer lock is released on Esc. The pause menu, settings and spell selection all work with keys (Tab/arrows plus Enter, or number keys), and focus order is predictable.
- *Why:* XAG 112 asks that the UI be "fully navigable by keyboard" and that accessibility settings be reachable from launch. GAG says the game should start without navigating "multiple levels of menus".
- *Check:* breaker operates start, pause, resume, settings and spell select with the keyboard only.
- *Source:* https://devdocs.xbox.com/build/game-principles/accessibility/xag-deep-dives/xag-112-ui-navigation.md ; https://gameaccessibilityguidelines.com/full-list/

**GS-14 (P1) Settings are remembered.** Sensitivity, FOV, assist, bob and shake toggles, volumes and text size all persist across reloads. Store them in `localStorage` wrapped in try/catch, so the game still works when storage is blocked.
- *Why:* "Ensure that all settings are saved/remembered" is a GAG basic.
- *Check:* breaker changes each setting, reloads and checks it was kept. Then breaker blocks storage, for example in a private context, and confirms the game boots with defaults.
- *Source:* https://gameaccessibilityguidelines.com/full-list/

## C. Game feel and feedback

**GS-15 (P0) Every cast and hit gives feedback on at least two channels.** Visual plus audio, at least. A hit and a miss must look and sound different. The "target in range" state uses the crosshair's shape, not only its colour.
- *Why:* XAG 103 says important cues should use at least two of: text, symbols, colour, sound, haptics. It also says no cue should depend on colour or on sound alone.
- *Check:* breaker mutes audio and checks that hit and miss are still distinguishable on screen. Breaker then views a greyscale screenshot and checks that the crosshair's on-target state is still visible.
- *Source:* https://devdocs.xbox.com/build/game-principles/accessibility/xag-deep-dives/xag-103-additional-cues.md

**GS-16 (P1) Impacts feel solid.** Visual and audio feedback on a hit fire in the same frame, with sound and visuals in sync. A short hit-stop or emphasis (a pause, flash or scale pop of 50 to 100 ms, within the GS-6 limits) marks a successful reaction. "Game feel" means real-time control, a simulated space, and polish that emphasises interactions.
- *Why:* in a study of action games, hit stop, coherent sound and camera emphasis were the features that most affected "impact feel". Mismatched or delayed audio made hits feel "soft and powerless".
- *Check:* breaker logs the frame of the impact event against the frames where the VFX spawned and the audio played. They must be the same frame. Panel personas rate whether casting "feels powerful". **[panel]**
- *Source:* https://arxiv.org/pdf/2208.06155 ; https://en.wikipedia.org/wiki/Game_feel

**GS-17 (P0) Feedback is explicit enough for under-6s.** A success shows a large, clear reaction: a character or object reacts visibly, a cheer or spoken line plays, and a big icon appears. Subtle faces or small sounds alone are not enough.
- *Why:* NN/g found that "Kids under 6 failed to notice" subtle character feedback. It recommends exaggerated expressions or spoken statements.
- *Check:* the 6-year-old persona explains what happened after each first-time event. If they cannot, it is a fail. **[panel]**
- *Source:* https://www.nngroup.com/articles/kids-cognition/

## D. Seeing, hearing and reading

**GS-18 (P0) Text is readable.** HUD and body text is at least 18 px at a 1080p viewport and scales up to 200%. Use a plain sans-serif. Text contrast is at least 4.5:1 against the lowest-contrast part of what is behind it, and at least 3:1 for large text (36 px or more).
- *Why:* XAG 101 sets a PC minimum of 18 px with scaling to 200%. XAG 102 sets 4.5:1 for normal text and 3:1 for large text.
- *Check:* breaker measures computed `font-size` and contrast of each HUD element (DOM plus a screenshot sample of the scene behind it) at 1920x1080. **Current client:** `.slot .formula` 11 px, `.slot .key` 12 px, `.slot .name` and `#obj-line` 15 px, and `#hint` 16 px all fail.
- *Source:* https://devdocs.xbox.com/build/game-principles/accessibility/xag-deep-dives/xag-101-text-display.md ; https://devdocs.xbox.com/build/game-principles/accessibility/xag-deep-dives/xag-102-contrast.md

**GS-19 (P0) Nothing essential depends on colour alone.** Spells, elements and states each have a distinct icon or shape as well as a colour. Avoid red/green as the only contrast. Offer a crosshair colour or style choice (P1).
- *Why:* about 8% of men and 0.5% of women have a colour vision deficiency, mostly red/green. "Ensure no essential information is conveyed by a fixed colour alone" is a GAG basic. XAG 103 asks for a choice of crosshair colours.
- *Check:* breaker runs the HUD and scene screenshots through a deuteranopia/protanopia filter and greyscale, and confirms that every spell slot and target state can still be told apart.
- *Source:* https://bugnet.io/blog/colorblind-friendly-game-design ; https://devdocs.xbox.com/build/game-principles/accessibility/xag-deep-dives/xag-103-additional-cues.md

**GS-20 (P1) Audio has separate volume controls and is never the only cue.** There are separate sliders or mutes for music, sound effects and voice, plus a master mute. A mono option is P2. Every important sound (fire warning, quest done) has a visual equivalent. Any speech is captioned, and captions can be turned on before the first sound plays.
- *Why:* XAG 105 asks for separate volume controls and a mono downmix. GAG lists "no essential information by sound alone" and "subtitles for all important speech" as basics.
- *Check:* breaker plays the golden path muted and checks it can still be completed. Breaker confirms the sliders exist and persist (GS-14).
- *Source:* https://devdocs.xbox.com/build/game-principles/accessibility/xag-deep-dives/xag-105-audio-accessibility.md ; https://gameaccessibilityguidelines.com/full-list/

**GS-21 (P0) Language is simple, and pre-readers are not blocked by text.**
- Instructions are short, at a 6th-grade reading level or below.
- No instruction is conveyed by text alone. Each is reinforced with a visual demonstration, icon or voice.
- Players read prompts at their own pace: there is no auto-dismiss, or the time can be extended to at least 10x (GS-24).
- *Why:* 14% of US/UK adults have a reading age below 11. NN/g recommends a 6th-grade level or lower for teens. Children aged 4 to 6 are often pre-readers. GAG says no essential instruction should be conveyed by text alone.
- *Check:* critic checks new copy (sentence length and word choice; a readability score of grade 6 or below as a guide). The 6-year-old persona plays with "can't read yet" and must still move and cast (B4). **[panel]**
- *Source:* https://gameaccessibilityguidelines.com/use-simple-clear-language/ ; https://www.nngroup.com/articles/usability-of-websites-for-teenagers/ ; https://gameaccessibilityguidelines.com/full-list/

**GS-22 (P1) Click targets are big enough.** Every clickable UI element is at least 24x24 CSS px. Primary actions (start, resume, spell slots, quest buttons) are at least 40 CSS px, about 1 cm.
- *Why:* WCAG 2.2 sets a 24x24 CSS px minimum. NN/g saw 7-year-olds fail a 5 mm close button, and says about 1 cm targets work from about age 9.
- *Check:* breaker reads the bounding boxes of all buttons at 1920x1080 and at a 1366x768 Chromebook size.
- *Source:* https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html ; https://www.nngroup.com/articles/children-ux-physical-development/

## E. Onboarding and objectives

**GS-23 (P0) Players learn by playing, in a safe space.** The first minute teaches move, look and cast by doing them in a place where the player cannot fail. Each new mechanic follows the same pattern: introduce it safely, develop it, twist it, then move on. A static controls screen does not count as a tutorial. The tutorial can be replayed on demand.
- *Why:* the Game Accessibility Guidelines say interactive tutorials reduce load on short-term memory compared with static instructions. Mario level design introduces each mechanic "in a safe spot where you can't exactly fail while learning it". XAG 109 says static control screens "are not tutorials" and tutorials should be available on demand.
- *Check:* critic rejects specs that teach a new mechanic only through text or a controls panel. Breaker confirms that nothing in the first area can damage the player or fail the quest. Bar B4 sets the time to first move (10 s or less) and first cast (30 s or less). **[panel]**
- *Source:* https://gameaccessibilityguidelines.com/include-interactive-tutorials/ ; https://www.engadget.com/2015/03/17/super-mario-3d-world-design/ ; https://devdocs.xbox.com/build/game-principles/accessibility/xag-deep-dives/xag-109-objective-clarity.md

**GS-24 (P0) The current objective and the controls are always available.**
- The objective is specific and actionable ("Put out the fire on the cart with Water", not "Help out"), and shows numbers where they apply ("2/3 fires").
- A controls reminder is one key away (or shown on pause).
- When the player makes no progress for a while (about 45 to 60 s), a hint or marker points the way.
- Players can turn hints down.
- *Why:* XAG 109 calls for prescriptive task text, numeric progress, and optional waypoints after a period of no progress. GAG lists objective and control reminders during play.
- *Check:* breaker idles at quest start for 90 s and confirms a hint appears. Critic checks that new objective text is specific. Each persona states their current goal at any moment. **[panel]**
- *Source:* https://devdocs.xbox.com/build/game-principles/accessibility/xag-deep-dives/xag-109-objective-clarity.md ; https://gameaccessibilityguidelines.com/full-list/

**GS-25 (P1) No time pressure the player did not choose.** UI and reading have no time limits. Timed challenges (fire spreading) can be slowed or turned off with an assist (game speed or a "calm" option) without losing progress, and players can practise them without failing.
- *Why:* XAG 116 says to use time limits only when essential and to let players extend them to at least 10x. GAG lists a game-speed option as basic and "a means of practicing without failure" as intermediate.
- *Check:* breaker enables the assist and confirms the fire quest's timer slows or stops. Critic flags any new countdown that has no assist.
- *Source:* https://devdocs.xbox.com/build/game-principles/accessibility/xag-deep-dives/xag-116-time-limits.md ; https://gameaccessibilityguidelines.com/full-list/

**GS-26 (P1) Difficulty can be changed at any time without losing progress, and assists are offered respectfully.** At minimum the game offers aim assist, game speed and hint frequency. XAG prefers four or more presets in the long run. Labels describe what each option changes and never shame the player (no "baby mode").
- *Why:* XAG 108 says players should be able to change difficulty "at any point without losing progress". Celeste's Assist Mode is the reference for respectful framing.
- *Check:* breaker changes the assists mid-quest and confirms the quest state is kept. Critic reviews the labels. The 13-year-old persona must not find the wording babyish. **[panel]**
- *Source:* https://devdocs.xbox.com/build/game-principles/accessibility/xag-deep-dives/xag-108-difficulty-options.md ; https://gameaccessibilityguidelines.com/celeste-assist-mode

## F. Age-appropriate and ethical design

**GS-27 (P0) Content and tone suit the youngest players without talking down to teens.** Targets are objects and creatures, never people. There is no gore. Tone and art are friendly but not babyish. Copy avoids calling players "kids".
- *Why:* Google's Teacher Approved criteria require all content to meet "appropriateness standards for your app's youngest target age group" and rate design, visuals, appeal and enrichment. NN/g found that teens "don't want overly childish content", prefer neutral graphics, and are put off by the word "kid".
- *Check:* critic reviews new assets and copy. The 6-year-old and the teacher check appropriateness, and the 13-year-old checks tone. **[panel]**
- *Source:* https://google.play/business/programs/teacherapproved/ ; https://www.nngroup.com/articles/usability-of-websites-for-teenagers/

**GS-28 (P0) No manipulative engagement design.** The game has none of the following:
- daily-reward or streak lures ("come back tomorrow");
- fake countdowns;
- characters pleading or guilting the player to keep playing;
- delayed or hidden close buttons;
- auto-continue into the next quest without a choice;
- purchases, ads, or external links a child can reach without an adult step.
- *Why:* a JAMA Network Open study found manipulative design in 80% of apps used by young children: lures to prolong play, parasocial pressure, fake time pressure, and navigation constraints. The ICO Children's Code targets nudge techniques and "sticky features" that extend engagement. Apple's kids guidance puts links out of the app and purchases behind a parental gate.
- *Check:* critic rejects any spec that adds one of these patterns. Breaker searches the UI for external `href`s and unprompted auto-advance.
- *Source:* https://www.michiganmedicine.org/health-lab/design-tricks-commonly-used-monetize-young-childrens-app-use ; https://www.esrb.org/privacy-certified-blog/the-icos-age-appropriate-design-code-uses-detrimental-to-children/ ; https://developer.apple.com/app-store/kids-apps/?utm_id=DP

**GS-29 (P2) Accessibility features are documented for teachers.** A short page or section lists the controls, assists and settings, in plain language.
- *Why:* XAG 121 and the GAG basics ask games to publish accessibility features so people can judge fit before they play. Our buyer is a teacher choosing for a mixed class.
- *Check:* the page exists, and its list matches the settings UI.
- *Source:* https://learn.microsoft.com/en-au/gaming/accessibility/xbox-accessibility-guidelines/121

## G. Learning design

**GS-30 (P0) The science is the mechanic, not a quiz bolted on.** Succeeding in play requires applying the real reaction: picking the spell whose chemistry fits the situation is how the player wins. No quiz screens interrupt play to test facts that the gameplay does not use.
- *Why:* in Habgood and Ainsworth's *Zombie Division* study, children learned more from the intrinsically integrated version, and given a free choice they played it "seven times longer" than the version with a separate quiz.
- *Check:* critic asks of each new quest: "Can the player win without using the science?" If yes, the spec is `wrong-thing`. The teacher persona confirms that the decision being made is the science. **[panel]**
- *Source:* https://www.kqed.org/mindshift/20765/whats-the-secret-sauce-to-a-great-educational-game

**GS-31 (P1) Play keeps minds on the learning goal.**
- *Minds-on:* the player predicts or chooses, not just clicks.
- *Engaged:* there are no unrelated side games or distractions.
- *Meaningful:* reactions connect to things players know, like rust, fire and ice.
- *Socially usable:* it can be discussed in class, and the teacher can see what was learned.
- *Why:* the "four pillars" (active, engaged, meaningful, socially interactive) from the science-of-learning review of educational apps, which notes that "side games unrelated to the goal pull attention away".
- *Check:* critic checks that each new feature serves a stated learning goal. The teacher persona scores each pillar. **[panel]**
- *Source:* https://www.psychologicalscience.org/publications/educational-apps.html

**GS-32 (P1) Abstract notation is backed by concrete visuals for young players.** Formulas and numbers (H2O, "2/3", degrees C) always appear next to a visible, concrete effect: the fire shrinking, ice forming, a counter filling. Players aged 4 to 8 never need a formula to know what to do.
- *Why:* NN/g saw children struggle with abstract symbols: a 7-year-old did not understand what "2" meant, and a 5-year-old found that a switch dropped coins but did not understand the game's objective. Young children build on concrete mental models.
- *Check:* the 6-year-old persona explains what each spell does without reading the formula. **[panel]**
- *Source:* https://www.nngroup.com/articles/kids-cognition/

---

## Known gaps in the current client (claude/fps-prototype, read 2026-10-30)

These are starting points for backlog items, not verdicts. A breaker confirms each one on the artifact.

| Item | Observation |
|------|-------------|
| GS-1 | `fov: 70 deg` vertical, about 102 deg horizontal at 16:9 (above about 90 to 100) |
| GS-2, GS-8, GS-10, GS-14 | sensitivity and assist only via URL (`?sens=`, `?assist=`); no settings UI; FOV not adjustable; nothing persisted |
| GS-3 | viewmodel bob present; no toggle seen |
| GS-8 | default about 9 cm/360 at 800 CPI (faster than the 20 to 80 cm study range) |
| GS-18 | HUD text from 11 to 16 px in several places |
| GS-20 | no volume sliders seen in the HUD |
