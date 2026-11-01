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

const parse = (/** @type {string | undefined} */ raw) => {
  try { return raw ? JSON.parse(raw) : null; } catch { return null; }
};

/** @param {string} cmd */
const push = (cmd) => {
  const w = /** @type {any} */ (window);
  w.__gameIn = w.__gameIn || {};
  (w.__gameIn.story = w.__gameIn.story || []).push(cmd);
};

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
    try { w.__game && w.__game.pause(wasPaused); } catch { /* no HUD */ }
    push('intro_done');
    document.getElementById('game')?.focus();
  };

  // Any key or click skips, once the intro has shown for a moment; until
  // then, and while it shows, nothing reaches the game.
  const skip = (/** @type {Event} */ e) => {
    if (!open) return;
    e.stopImmediatePropagation();
    if (e.type === 'keydown' || e.type === 'pointerdown') e.preventDefault();
    if ((performance.now() - openedAt) / 1000 >= SKIP_AFTER_S) finish();
  };
  for (const t of ['keydown', 'keypress', 'keyup', 'pointerdown', 'click']) window.addEventListener(t, skip, true);
  play1.addEventListener('click', finish);

  // Sound cues: a chime when a quest step completes, a pickup when the pack
  // gains an item.
  let lastQuest = -1;
  let lastPack = -1;

  const tick = () => {
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
      if (s.quest > 0) done = true;
      if (lastQuest >= 1 && s.quest > lastQuest) play('bench_success');
      lastQuest = s.quest;
      const n = Array.isArray(s.pack) ? s.pack.reduce((a, p) => a + (p.n || 0), 0) : 0;
      if (lastPack >= 0 && n > lastPack) play('pickup');
      lastPack = n;
    }
    requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
}
