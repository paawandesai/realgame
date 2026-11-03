// The pack (I-052, Lane C, spec section 6): Tab or I opens 20 slots; click a
// slot, then press 6-9 to put it on that hotbar key. It pauses the game and
// follows the Formula Bench's modal rules (aria-modal, the page behind it
// inert, Escape or Tab closes, the canvas gets the focus back). Only in the
// block world. Rust (story.rs) owns the pack; this draws `__storyState.slots`
// and sends `bind <slot> <key>` on `window.__gameIn.story`.

/** A slot's label: story.rs sends it (`valley::Item::label`), else the key. */
const label = (/** @type {{ item: string, label?: string }} */ s) => s.label || s.item.replace(/_/g, ' ');

const parse = (/** @type {string | undefined} */ raw) => {
  try { return raw ? JSON.parse(raw) : null; } catch { return null; }
};

export function startPack() {
  const w = /** @type {any} */ (window);
  const root = document.createElement('div');
  root.id = 'pack';
  root.hidden = true;
  root.setAttribute('role', 'dialog');
  root.setAttribute('aria-modal', 'true');
  root.setAttribute('aria-label', 'Pack');
  const card = document.createElement('div');
  card.className = 'pack-card';
  const head = document.createElement('div');
  head.className = 'pack-head';
  const title = document.createElement('h2');
  title.textContent = 'Pack';
  const help = document.createElement('div');
  help.className = 'pack-help';
  help.textContent = 'Pick a slot, then press 6, 7, 8 or 9';
  const close = document.createElement('button');
  close.type = 'button';
  close.className = 'pack-close';
  close.textContent = 'Close';
  head.append(title, help, close);
  const grid = document.createElement('div');
  grid.className = 'pack-grid';
  /** @type {HTMLButtonElement[]} */
  const slots = [];
  for (let i = 0; i < 20; i++) {
    const b = document.createElement('button');
    b.type = 'button';
    b.className = 'pack-slot';
    b.dataset.slot = String(i);
    b.addEventListener('click', () => select(i));
    grid.append(b);
    slots.push(b);
  }
  card.append(head, grid);
  root.append(card);
  document.body.append(root);

  let open = false;
  let picked = -1;
  let last = '';
  /** @type {Array<[HTMLElement, boolean]>} */
  let behind = [];
  let wasPaused = false;

  const draw = () => {
    const st = parse(w.__storyState);
    const items = st && Array.isArray(st.slots) ? st.slots : [];
    const hot = st && Array.isArray(st.hot) ? st.hot : [];
    const key = JSON.stringify([items, hot, picked]);
    if (key === last) return;
    last = key;
    slots.forEach((b, i) => {
      const s = items[i];
      const bound = hot.indexOf(i);
      b.classList.toggle('empty', !s);
      b.classList.toggle('picked', i === picked);
      b.dataset.item = s ? s.item : '';
      b.replaceChildren();
      if (s) {
        const name = document.createElement('span');
        name.className = 'pack-name';
        name.textContent = label(s);
        const n = document.createElement('span');
        n.className = 'pack-n';
        n.textContent = s.n > 1 ? String(s.n) : '';
        b.append(name, n);
      }
      if (bound >= 0) {
        const k = document.createElement('span');
        k.className = 'pack-key';
        k.textContent = String(bound + 6);
        b.append(k);
      }
      b.setAttribute('aria-label', s ? `${label(s)} ${s.n}${bound >= 0 ? `, key ${bound + 6}` : ''}` : `Empty slot ${i + 1}`);
    });
  };

  const select = (/** @type {number} */ i) => {
    picked = picked === i ? -1 : i;
    draw();
  };

  const otherModal = () => [...document.querySelectorAll('[aria-modal="true"]')]
    .some((el) => el !== root && !(/** @type {HTMLElement} */ (el).hidden) && /** @type {HTMLElement} */ (el).offsetParent !== null);

  const openPack = () => {
    if (open || otherModal()) return false;
    const s = parse(w.__gameState);
    if (!s || s.mode !== 'voxel') return false;
    open = true;
    picked = -1;
    last = '';
    wasPaused = !!(w.__gameIn && w.__gameIn.paused);
    try { if (document.pointerLockElement) document.exitPointerLock(); } catch { /* nothing to free */ }
    try { w.__game && w.__game.pause(true); } catch { /* no HUD */ }
    behind = /** @type {Array<[HTMLElement, boolean]>} */ (['stage', 'hud', 'loading', 'pause']
      .map((id) => document.getElementById(id))
      .filter(Boolean)
      .map((el) => [el, /** @type {HTMLElement} */ (el).inert]));
    for (const [el] of behind) el.inert = true;
    document.getElementById('pause')?.classList.add('bench-under');
    root.hidden = false;
    draw();
    slots[0]?.focus();
    return true;
  };

  const closePack = () => {
    if (!open) return false;
    open = false;
    root.hidden = true;
    for (const [el, was] of behind) el.inert = was;
    behind = [];
    document.getElementById('pause')?.classList.remove('bench-under');
    try { w.__game && w.__game.pause(wasPaused); } catch { /* no HUD */ }
    document.getElementById('game')?.focus();
    return true;
  };
  close.addEventListener('click', closePack);

  const editable = (/** @type {EventTarget | null} */ t) => {
    const el = /** @type {HTMLElement | null} */ (t);
    return !!el && !!el.tagName && (/^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName) || el.isContentEditable);
  };
  window.addEventListener('keydown', (e) => {
    if (e.ctrlKey || e.metaKey || e.altKey || editable(e.target)) return;
    if (!open) {
      if ((e.code === 'Tab' || e.code === 'KeyI') && !e.repeat && openPack()) {
        e.preventDefault();
        e.stopImmediatePropagation();
      }
      return;
    }
    // Open: the pack takes every key.
    e.stopImmediatePropagation();
    if (e.code === 'Escape' || e.code === 'Tab' || e.code === 'KeyI') {
      e.preventDefault();
      closePack();
      return;
    }
    const k = /^Digit([6-9])$/.exec(e.code);
    if (k && picked >= 0) {
      e.preventDefault();
      const s = parse(w.__storyState);
      if (s && s.slots && s.slots[picked]) {
        (w.__gameIn.story = w.__gameIn.story || []).push(`bind ${picked} ${k[1]}`);
      }
    }
  }, true);

  const tick = () => {
    if (open) draw();
    requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
  return { open: openPack, close: closePack };
}
