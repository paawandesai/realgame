// The pack (I-052, Lane C, spec section 6): Tab or I opens 20 slots; click a
// slot, then press 6-9 to put it on that hotbar key. It pauses the game and
// follows the Formula Bench's modal rules (aria-modal, the page behind it
// inert, Escape or Tab closes, the canvas gets the focus back). Only in the
// block world. Rust (story.rs) owns the pack; this draws `__storyState.slots`
// and sends `bind <slot> <key>` on `window.__gameIn.story`.

/** A slot's label: story.rs sends it (`valley::Item::label`), else the key. */
const label = (/** @type {{ item: string, label?: string }} */ s) => s.label || s.item.replace(/_/g, ' ');

/** The shipped spell on each spell key (1-4), which a discovery shares its key with. */
const SHIPPED = ['Fire', 'Water', 'Frost', 'CO₂'];

/** A formula with subscript digits (H2O -> H₂O). */
const sub = (/** @type {string} */ f) => String(f || '').replace(/\d/g, (d) => '₀₁₂₃₄₅₆₇₈₉'[Number(d)]);

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
  // I-053: spells saved from the Chemistry Bench, the bottles the pack
  // holds, and the chemistry quest (discover.rs publishes `__discState`).
  const discBox = document.createElement('div');
  discBox.className = 'pack-disc';
  card.append(head, grid, discBox);
  root.append(card);
  document.body.append(root);
  let lastDisc = '';
  const sendDisc = (/** @type {string} */ c) => { w.__gameIn = w.__gameIn || {}; (w.__gameIn.disc = w.__gameIn.disc || []).push(c); };
  const drawDisc = () => {
    const st = parse(w.__discState);
    const ws = parse(w.__workState);
    const sprout = !!ws && ws.band === 'sprout';
    const key = JSON.stringify([st && st.saved, st && st.slots, st && st.on, st && st.bottles, st && st.quests, sprout]);
    if (key === lastDisc) return;
    lastDisc = key;
    discBox.replaceChildren();
    if (!st) return;
    const h = document.createElement('h3');
    h.textContent = 'Spells from the Bench';
    discBox.append(h);
    const saved = Array.isArray(st.saved) ? st.saved : [];
    if (!saved.length) {
      const p = document.createElement('p');
      p.className = 'pack-disc-empty';
      p.append('Find a reaction on the ');
      const a = document.createElement('a');
      a.href = 'bench-free.html';
      a.target = '_blank';
      a.rel = 'noopener';
      a.textContent = 'Chemistry Bench';
      p.append(a, ' and save it as a spell. It shows up here.');
      discBox.append(p);
    }
    if (saved.length) {
      const p = document.createElement('p');
      p.className = 'pack-disc-help';
      p.textContent = 'Pick a key for each spell. In the world, press that key, then press it again to switch back to the usual spell.';
      discBox.append(p);
    }
    for (const s of saved) {
      const row = document.createElement('div');
      row.className = 'pack-disc-row';
      row.dataset.id = s.id;
      row.dataset.kind = (s.effect && s.effect.kind) || '';
      const top = document.createElement('div');
      top.className = 'pack-disc-top';
      const name = document.createElement('span');
      name.className = 'pack-disc-name';
      name.textContent = s.name;
      const info = document.createElement('span');
      info.className = 'pack-disc-info';
      const built = s.instant ? 'casts at once' : `built ${Math.min(s.builds, s.need)} of ${s.need}`;
      const where = s.key ? `on key ${s.key}` : 'not on a key yet';
      info.textContent = s.usable === false ? 'for a higher grade band' : `${where} · ${built}`;
      top.append(name, info);
      row.append(top);
      // What it does out in the world (I-056).
      if (s.effect && s.effect.use) {
        const use = document.createElement('p');
        use.className = 'pack-disc-uses';
        use.textContent = `In the world: it ${s.effect.use}.`;
        row.append(use);
      }
      // What it uses up from the pack, and where to get it (I-053 bottles).
      for (const u of Array.isArray(s.uses) ? s.uses : []) {
        const items = Array.isArray(u.items) ? u.items.join(' or ') : '';
        // I-059: formulas show at every band, Sprout included.
        const what = `${items} (${sub(u.species)})`;
        const line = document.createElement('p');
        line.className = 'pack-disc-uses';
        line.classList.toggle('missing', !u.have);
        const verb = u.used ? 'Uses up' : 'Needs';
        line.textContent = u.have
          ? `${verb} ${what}. You have ${u.have}.`
          : `${verb} ${what}. You have none. ${u.findItem}: ${u.find}`;
        row.append(line);
      }
      // The key picker: any spell key, 1 to 4, one key per spell.
      const keys = document.createElement('div');
      keys.className = 'pack-disc-keys';
      keys.setAttribute('role', 'group');
      keys.setAttribute('aria-label', `Key for ${s.name}`);
      const lab = document.createElement('span');
      lab.className = 'pack-disc-keylabel';
      lab.textContent = 'Put on key:';
      keys.append(lab);
      for (let k = 1; k <= 4; k++) {
        const b = document.createElement('button');
        b.type = 'button';
        b.className = 'pack-disc-key';
        b.dataset.key = String(k);
        const on = s.key === k;
        b.setAttribute('aria-pressed', on ? 'true' : 'false');
        b.disabled = s.usable === false;
        const shares = SHIPPED[k - 1];
        const held = Array.isArray(st.names) && st.slots && st.slots[k - 1] && st.slots[k - 1] !== s.id ? st.names[k - 1] : '';
        b.title = on ? `On key ${k}: click to take it off` : `Put ${s.name} on key ${k}, with ${shares}${held ? ` (instead of ${held})` : ''}`;
        b.setAttribute('aria-label', b.title);
        const n = document.createElement('span');
        n.className = 'pack-disc-keyn';
        n.textContent = String(k);
        const sh = document.createElement('span');
        sh.className = 'pack-disc-keysh';
        sh.textContent = shares;
        b.append(n, sh);
        b.addEventListener('click', () => { sendDisc(on ? `unequip ${k}` : `equip ${s.id} ${k}`); lastDisc = ''; });
        keys.append(b);
      }
      if (s.key) {
        const off = document.createElement('button');
        off.type = 'button';
        off.className = 'pack-disc-use';
        off.textContent = 'Take off';
        off.addEventListener('click', () => { sendDisc(`unequip ${s.key}`); lastDisc = ''; });
        keys.append(off);
      }
      row.append(keys);
      discBox.append(row);
    }
    const bottles = Array.isArray(st.bottles) ? st.bottles : [];
    const bl = document.createElement('p');
    bl.className = 'pack-disc-bottles';
    bl.textContent = bottles.length
      ? `Bottles (things in your pack that Bench spells use up): ${bottles.map((/** @type {any} */ x) => `${x.label} (${sub(x.species)}) ×${x.n}`).join(', ')}`
      : `Bottles are things in your pack that Bench spells use up, like charcoal. You have none yet. ${st.starter || ''}`.trim();
    discBox.append(bl);
    for (const q of Array.isArray(st.quests) ? st.quests : []) {
      const p = document.createElement('p');
      p.className = 'pack-disc-quest';
      p.classList.toggle('done', !!q.done);
      p.textContent = `${q.done ? '✓ ' : ''}${q.line}`;
      discBox.append(p);
      // I-057: the hint names the kind of reaction it needs, not the answer.
      if (!q.done && q.hint) {
        const h = document.createElement('p');
        h.className = 'pack-disc-hint';
        h.textContent = String(q.hint);
        discBox.append(h);
      }
    }
  };

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
    lastDisc = '';
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
    if (open) { draw(); drawDisc(); }
    requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
  return { open: openPack, close: closePack };
}
