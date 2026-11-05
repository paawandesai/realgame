// The title screen (Wednesday polish): "Formula, a RealGame world", a Play
// button and a short controls card, over the valley while it loads. The
// story intro waits under it (story.js checks `body.title-open`), so the
// first thing a player sees is the name and the one button to press.
//
// Automated browsers (navigator.webdriver: the goldens, perf and analytics
// runs) skip it, so every existing test still starts at the intro; `?title`
// shows it there too, for its own golden.

/** Rows of the controls card: [keys, what they do]. @type {[string, string][]} */
export const CONTROLS = [
  ['W A S D', 'Walk'],
  ['Mouse', 'Look around'],
  ['Click', 'Cast the spell in your hand'],
  ['1 2 3 4', 'Pick a spell'],
  ['Space', 'Jump'],
  ['E', 'Use what is in front of you'],
  ['B', 'Open the Chemistry Bench'],
  ['Esc', 'Pause and see all controls'],
];

/** Whether this page load shows the title (exported for its test). */
export function wantsTitle(/** @type {string} */ search, /** @type {boolean} */ webdriver) {
  const q = new URLSearchParams(search);
  if (q.has('title')) return true;
  if (q.has('notitle')) return false;
  return !webdriver;
}

export function startTitle() {
  const w = /** @type {any} */ (window);
  if (!wantsTitle(location.search, !!navigator.webdriver)) return;

  const root = document.createElement('div');
  root.id = 'title';
  root.setAttribute('role', 'dialog');
  root.setAttribute('aria-modal', 'true');
  root.setAttribute('aria-labelledby', 'title-name');

  const head = document.createElement('div');
  head.className = 'title-head';
  const name = document.createElement('h1');
  name.id = 'title-name';
  name.textContent = 'Formula';
  const sub = document.createElement('p');
  sub.className = 'title-sub';
  sub.textContent = 'a RealGame world';
  head.append(name, sub);

  const play = document.createElement('button');
  play.type = 'button';
  play.className = 'title-play';
  play.textContent = 'Loading…';
  play.disabled = true;

  const card = document.createElement('div');
  card.className = 'title-controls';
  const h = document.createElement('h2');
  h.textContent = 'Controls';
  const list = document.createElement('dl');
  for (const [k, what] of CONTROLS) {
    const dt = document.createElement('dt');
    for (const part of k.split(' ')) {
      const kbd = document.createElement('kbd');
      kbd.textContent = part;
      dt.append(kbd);
    }
    const dd = document.createElement('dd');
    dd.textContent = what;
    list.append(dt, dd);
  }
  card.append(h, list);

  // Bench plan Phase 1: the Chemistry Bench on its own, no world around it.
  const bench = document.createElement('a');
  bench.className = 'title-bench';
  bench.href = 'bench-free.html' + location.search;
  bench.textContent = 'Chemistry Bench';

  root.append(head, play, bench, card);
  document.body.append(root);
  document.body.classList.add('title-open');

  let ready = false;
  const close = () => {
    if (!ready || root.hidden) return;
    root.hidden = true;
    document.body.classList.remove('title-open');
    window.removeEventListener('keydown', keys, true);
    document.getElementById('game')?.focus();
  };
  // While the title shows, keys belong to it: Enter or Space plays, and
  // nothing walks the player around behind the card.
  const keys = (/** @type {KeyboardEvent} */ e) => {
    if (root.hidden) return;
    if (e.target === play && (e.key === 'Enter' || e.key === ' ')) return; // the button's own click
    if (e.target === bench && e.key === 'Enter') return; // the link's own navigation
    e.stopImmediatePropagation();
    if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); close(); }
  };
  window.addEventListener('keydown', keys, true);
  play.addEventListener('click', close);

  // Play lights up once the game is running.
  const wait = () => {
    if (w.__gameState) {
      ready = true;
      play.disabled = false;
      play.textContent = 'Play';
      play.focus();
      return;
    }
    setTimeout(wait, 200);
  };
  wait();
}
