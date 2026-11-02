// The block valley's story layer (I-052, Lane C): the intro cards over the
// cold Kiln, and the sound cues for quest progress and pickups. Rust
// (story.rs) owns the quest; this reads `__gameState` (mode, quest, pack) and
// `__storyState` (the intro text for the player's band) and sends commands on
// `window.__gameIn.story`. Only in the block world (`mode === 'voxel'`).

/** The sound library, loaded only once the block world is seen, so the
 * classic courtyard never creates its AudioContext. @type {{ play: (name: string, o?: object) => unknown } | null} */
let sfx = null;
let sfxLoading = false;
const play = (/** @type {string} */ name) => { if (sfx) sfx.play(name); };

/** Card length (s), and how long before a key or click can skip (s). */
const CARD_S = 6;
const SKIP_AFTER_S = 1;
/** Sprout reads slower: the first card holds this long. */
const SPROUT_FIRST_S = 3;
/** A gap between frames counts for at most this long (ms) toward a card's
 * time and the 1 s before a key can skip: a longer gap is a stall (the
 * first chunks meshing at load), when nothing new was on screen. A slow
 * machine still advances, by up to this much a frame. */
const STALL_MS = 500;

const parse = (/** @type {string | undefined} */ raw) => {
  try { return raw ? JSON.parse(raw) : null; } catch { return null; }
};

/** @param {string} cmd */
const push = (cmd) => {
  const w = /** @type {any} */ (window);
  w.__gameIn = w.__gameIn || {};
  (w.__gameIn.story = w.__gameIn.story || []).push(cmd);
};

/**
 * Where a world point lands on screen for the camera in `s` (the game
 * state's `cam`, `yaw` and `pitch` in degrees, `fov` the vertical field of
 * view in degrees), as pixels; null when it is behind the camera.
 * @param {number[]} p @param {any} s @param {number} w @param {number} h
 */
export function project(p, s, w, h) {
  if (!s || !Array.isArray(s.cam)) return null;
  const yaw = (s.yaw * Math.PI) / 180, pitch = (s.pitch * Math.PI) / 180;
  const d = [p[0] - s.cam[0], p[1] - s.cam[1], p[2] - s.cam[2]];
  const x1 = d[0] * Math.cos(yaw) - d[2] * Math.sin(yaw);
  const z1 = d[0] * Math.sin(yaw) + d[2] * Math.cos(yaw);
  const y2 = d[1] * Math.cos(pitch) + z1 * Math.sin(pitch);
  const z2 = -d[1] * Math.sin(pitch) + z1 * Math.cos(pitch);
  if (z2 > -0.1) return null;
  const t = Math.tan((((s.fov || 60) * Math.PI) / 180) / 2);
  const nx = x1 / -z2 / (t * (w / h)), ny = y2 / -z2 / t;
  if (Math.abs(nx) > 1.2 || Math.abs(ny) > 1.2) return null;
  return { x: ((nx + 1) / 2) * w, y: ((1 - ny) / 2) * h, dist: Math.hypot(d[0], d[1], d[2]) };
}

export function startStory() {
  const w = /** @type {any} */ (window);
  const root = document.createElement('div');
  root.id = 'intro';
  root.hidden = true;
  root.setAttribute('role', 'dialog');
  root.setAttribute('aria-modal', 'true');
  root.setAttribute('aria-label', 'The story so far');
  const cards = [0, 1, 2].map((k) => {
    const card = document.createElement('div');
    card.className = `intro-card intro-card-${k}`;
    const pic = document.createElement('div');
    pic.className = `intro-pic intro-pic-${k}`;
    pic.setAttribute('aria-hidden', 'true');
    const text = document.createElement('p');
    text.className = 'intro-text';
    card.append(pic, text);
    root.append(card);
    return card;
  });
  const play1 = document.createElement('button');
  play1.type = 'button';
  play1.className = 'intro-play';
  play1.textContent = 'Just play';
  const dots = document.createElement('div');
  dots.className = 'intro-dots';
  dots.setAttribute('aria-hidden', 'true');
  for (let k = 0; k < 3; k++) dots.append(document.createElement('span'));
  root.append(dots, play1);
  document.body.append(root);

  let open = false;
  let done = false;
  let openedAt = 0;
  let cardAt = 0;
  let lastTick = performance.now();
  /** The part of the gap since the last frame that does not count (ms). */
  const stall = () => Math.max(0, performance.now() - lastTick - STALL_MS);
  let card = 0;
  let sprout = false;
  /** @type {Array<[HTMLElement, boolean]>} */
  let behind = [];
  let wasPaused = false;

  const show = (/** @type {number} */ k) => {
    card = k;
    cardAt = performance.now();
    root.dataset.card = String(k);
    cards.forEach((c, i) => c.classList.toggle('on', i === k));
    [...dots.children].forEach((d, i) => d.classList.toggle('on', i === k));
  };
  const setText = (/** @type {string[]} */ lines) => {
    lines.forEach((t, i) => { const p = cards[i]?.querySelector('.intro-text'); if (p && p.textContent !== t) p.textContent = t; });
  };

  const openIntro = (/** @type {any} */ story, /** @type {string | null} */ band) => {
    open = true;
    sprout = band === 'sprout';
    openedAt = performance.now();
    if (story && Array.isArray(story.intro)) setText(story.intro);
    wasPaused = !!(w.__gameIn && w.__gameIn.paused);
    try { w.__game && w.__game.pause(true); } catch { /* no HUD */ }
    behind = /** @type {Array<[HTMLElement, boolean]>} */ (['stage', 'hud', 'loading', 'pause']
      .map((id) => document.getElementById(id))
      .filter(Boolean)
      .map((el) => [el, /** @type {HTMLElement} */ (el).inert]));
    for (const [el] of behind) el.inert = true;
    document.getElementById('pause')?.classList.add('bench-under');
    // The welcome caption waits under the intro (story.rs says it again
    // once the intro closes), so the two never overlap.
    document.body.classList.add('intro-open');
    root.hidden = false;
    show(0);
    play1.focus();
  };

  const finish = () => {
    if (!open) return;
    open = false;
    done = true;
    root.hidden = true;
    for (const [el, was] of behind) el.inert = was;
    behind = [];
    document.getElementById('pause')?.classList.remove('bench-under');
    document.body.classList.remove('intro-open');
    try { w.__game && w.__game.pause(wasPaused); } catch { /* no HUD */ }
    push('intro_done');
    document.getElementById('game')?.focus();
  };

  // Any key or click skips, once the intro has shown for a moment; until
  // then, and while it shows, nothing reaches the game. The key that skips
  // also reaches the game (W starts walking, 5 takes the Pick); a click
  // that skips does not (it would lock the pointer or cast).
  const skip = (/** @type {Event} */ e) => {
    if (!open) return;
    // "Just play" is a button the player chose: it works at once, by
    // mouse, touch or keyboard (the 1 s guard is for stray keys only). Its
    // press is let through to the button, and its click closes the intro.
    const onPlay = e.target instanceof Node && play1.contains(e.target);
    if (onPlay) {
      e.stopPropagation();
      if (e.type === 'click') finish();
      return;
    }
    const ready = (performance.now() - openedAt - stall()) / 1000 >= SKIP_AFTER_S;
    e.stopImmediatePropagation();
    if (ready && e.type === 'keydown') {
      // The event is aimed at the intro (the game was inert), so after
      // closing it the key is sent again to the game's canvas.
      finish();
      const k = /** @type {KeyboardEvent} */ (e);
      document.getElementById('game')?.dispatchEvent(new KeyboardEvent('keydown', {
        key: k.key, code: k.code, location: k.location, repeat: k.repeat, bubbles: true, cancelable: true,
        shiftKey: k.shiftKey, ctrlKey: k.ctrlKey, altKey: k.altKey, metaKey: k.metaKey,
      }));
      e.preventDefault();
      return;
    }
    if (e.type === 'keydown' || e.type === 'pointerdown') e.preventDefault();
    if (ready) finish();
  };
  for (const t of ['keydown', 'keypress', 'keyup', 'pointerdown', 'click']) window.addEventListener(t, skip, true);
  play1.addEventListener('click', finish);

  // The biome gates' signs (round 2): a board over each shut gate, drawn
  // where the gate is on screen while it is near and in view.
  /** @type {Record<string, HTMLElement>} */
  const signs = {};
  const drawSigns = (/** @type {any} */ s, /** @type {any} */ story) => {
    const gates = story && Array.isArray(story.gates) ? story.gates : [];
    for (const g of gates) {
      let el = signs[g.id];
      if (!el) {
        el = document.createElement('div');
        el.className = 'gate-sign';
        el.dataset.gate = g.id;
        el.setAttribute('role', 'note');
        document.body.append(el);
        signs[g.id] = el;
      }
      if (el.textContent !== g.text) el.textContent = g.text;
      const at = !g.open && !open ? project(g.at, s, innerWidth, innerHeight) : null;
      const show = !!at && at.dist < 40;
      el.hidden = !show;
      if (show && at) {
        el.style.left = `${Math.round(at.x)}px`;
        el.style.top = `${Math.round(at.y)}px`;
        el.style.opacity = String(Math.max(0.35, Math.min(1, 1.4 - at.dist / 40)));
      }
    }
  };

  // Sound cues: a chime when a quest step completes, a pickup when the pack
  // gains an item.
  let lastQuest = -1;
  let lastDonePush = performance.now();
  let lastPack = -1;
  let lastBurning = -1;

  const tick = () => {
    const now = performance.now();
    if (open && now - lastTick > STALL_MS) { const g = now - lastTick - STALL_MS; openedAt += g; cardAt += g; }
    lastTick = now;
    const s = parse(w.__gameState);
    if (s && s.mode === 'voxel') {
      if (!sfx && !sfxLoading) {
        sfxLoading = true;
        import('./sfx.js').then((m) => { sfx = m; }).catch(() => {});
      }
      const story = parse(w.__storyState);
      const band = parse(w.__benchState)?.band || null;
      if (!done && !open && s.quest === 0 && story) openIntro(story, band);
      if (open) {
        if (story && Array.isArray(story.intro)) setText(story.intro);
        const hold = card === 0 && sprout ? Math.max(CARD_S, SPROUT_FIRST_S) : CARD_S;
        if ((performance.now() - cardAt) / 1000 >= hold) {
          if (card < 2) show(card + 1);
          else finish();
        }
      }
      drawSigns(s, story);
      if (s.quest > 0) done = true;
      // Closed, but the game has not heard it yet (a command lost before
      // the game read its queue): say it again, once a second, so the
      // intro never comes back and the first objective always shows.
      else if (done && !open && now - lastDonePush > 1000) { lastDonePush = now; push('intro_done'); }
      if (lastQuest >= 1 && s.quest > lastQuest) play('bench_success');
      lastQuest = s.quest;
      // A fire put out: the hiss (round 2, P1).
      if (typeof s.burning === 'number') {
        if (lastBurning >= 0 && s.burning < lastBurning) play('hiss');
        lastBurning = s.burning;
      }
      const n = Array.isArray(s.pack) ? s.pack.reduce((a, p) => a + (p.n || 0), 0) : 0;
      if (lastPack >= 0 && n > lastPack) play('pickup');
      lastPack = n;
    }
    requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
}
