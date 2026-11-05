// The one Bench's Spells tab (I-060, docs/loop/items/I-060/spec.md): what the
// old Formula Bench did, inside the Chemistry Bench.
//
// - The four starting spells (Fire, Water, Frost, CO2): Learn by building the
//   formula at the band's level, then Practise; the stage, the practice count
//   and XP show as the world keeps them. In story mode (the Bench over the
//   world) every build goes to the game's Formula Bench queue
//   (`window.__gameIn.bench` on the world page: `pick`, `try`), which
//   bench.rs judges with judge_with and the shared codex, exactly as before.
//   Free play has no world running, so it shows the saved progress
//   (`rg.progress.v1`) and says where to learn them.
// - Spells saved from the Bench (discoveries): Practise builds the reaction's
//   equation; two correct builds and it casts at once (Sprout casts without
//   building). Story mode sends `learn <id>` then `try`, judged by bench.rs
//   against the reaction (the same judge as a worked cast); free play judges
//   with the chemistry engine and counts the build in the shared save
//   (formula-save.js). Put on key: any spell key 1 to 4.
// - Reactions found but not saved yet, with Save as a spell.
// - The chemistry quests (done or not), and in story mode the old Bench's
//   "Make things" (crafting from the pack).
//
// The builder is bench.js mountBuilder, the same one the worked casts use.

import { mountBuilder, reasonFor, pretty, rise, speak, wakeAudio, STAGE_WORD, ICONS } from './bench.js';
import { readSpells, readFound, recordBuild, putOnKey, readProgress, saveSpell, MASTERY_BUILDS } from './formula-save.js';

/** The starting spells, in key order (crates/bevy-client/src/progress.rs SPELL_IDS). */
const SHIPPED = [
  { id: 'fire', name: 'Fire', slot: 1 },
  { id: 'water', name: 'Water', slot: 2 },
  { id: 'frost', name: 'Frost', slot: 3 },
  { id: 'co2', name: 'CO2', slot: 4 },
];
const RANK = /** @type {Record<string, number>} */ ({ sprout: 0, seedling: 1, apprentice: 2, adept: 3 });
const BAND_NAME = /** @type {Record<string, string>} */ ({ sprout: 'Sprout', seedling: 'Seedling', apprentice: 'Apprentice', adept: 'Adept' });

/** @param {any} w @param {string} k */
const hostJson = (w, k) => { try { return JSON.parse(w[k] || 'null'); } catch { return null; } };

/**
 * @typedef {{
 *   box: HTMLElement,
 *   host: any,
 *   storage: any,
 *   band: () => string,
 *   byId: Record<string, any>,
 *   effects: Record<string, any>,
 *   judge: (input: string, conds: string, band: string) => any,
 *   atoms: (formula: string) => Array<[string, number]> | null,
 *   eq: (equation: string) => string,
 *   onSaved: () => void,
 * }} SpellsOpts
 */

/**
 * Mounts the Spells tab into `o.box`.
 * @param {SpellsOpts} o
 */
export function mountSpells(o) {
  const doc = /** @type {Document} */ (o.box.ownerDocument);
  const host = o.host;
  const story = !!host;
  /** @param {string} tag @param {string} [cls] @param {string} [text] */
  const el = (tag, cls, text) => {
    const e = /** @type {HTMLElement} */ (doc.createElement(tag));
    if (cls) e.className = cls;
    if (text !== undefined) e.textContent = text;
    return e;
  };
  /** @param {string} label @param {() => void} fn @param {string} [cls] */
  const button = (label, fn, cls = '') => {
    const b = /** @type {HTMLButtonElement} */ (el('button', `btn${cls ? ' ' + cls : ''}`, label));
    b.type = 'button';
    b.addEventListener('click', () => { wakeAudio(); fn(); });
    return b;
  };
  const rank = () => RANK[o.band()] ?? 2;
  // The world's text size (Settings: 1, 1.5 or 2) scales this tab as it does
  // the world's panels: the world page's --ui, else the saved setting.
  const uiScale = () => {
    try {
      if (story) { const v = parseFloat(host.getComputedStyle(host.document.documentElement).getPropertyValue('--ui')); if ([1, 1.5, 2].includes(v)) return v; }
      const t = JSON.parse(o.storage.getItem('alchemist.settings.v1') || 'null')?.text;
      return [1, 1.5, 2].includes(t) ? t : 1;
    } catch { return 1; }
  };
  const scale = () => o.box.style.setProperty('--ui', String(uiScale()));
  // Arrow keys move between the tab's controls (the old Bench's rule); a
  // tile or coefficient takes Up and Down itself, and a text box keeps them.
  o.box.addEventListener('keydown', (e) => {
    if (e.defaultPrevented || !String(e.key).startsWith('Arrow')) return;
    const t = /** @type {HTMLElement} */ (e.target);
    if (t && t.tagName === 'INPUT') return;
    const list = /** @type {HTMLElement[]} */ ([...o.box.querySelectorAll('button, input')]).filter((x) => !(/** @type {HTMLButtonElement} */ (x).disabled) && x.getClientRects().length > 0);
    const i = list.indexOf(t);
    if (i < 0 || !list.length) return;
    e.preventDefault();
    const d = e.key === 'ArrowRight' || e.key === 'ArrowDown' ? 1 : -1;
    list[(i + d + list.length) % list.length]?.focus();
  });
  const sprout = () => o.band() === 'sprout';
  const benchPush = (/** @type {string} */ c) => { try { host.__gameIn = host.__gameIn || {}; (host.__gameIn.bench = host.__gameIn.bench || []).push(c); } catch { /* the world is gone */ } };
  const discPush = (/** @type {string} */ c) => { try { host.__gameIn = host.__gameIn || {}; (host.__gameIn.disc = host.__gameIn.disc || []).push(c); } catch { /* the world is gone */ } };
  /** @type {Promise<any> | null} */
  let questDoc = null;
  /** @type {any} */
  let quests = null;
  const loadQuests = () => (questDoc ??= fetch('chem-quests.json').then((r) => (r.ok ? r.json() : null)).then((d) => { quests = d; lastKey = ''; return d; }).catch(() => null));

  // ---------- what each mode knows ----------
  /** The starting spells: stage, XP, practice and whether this band builds it. */
  const shipped = () => {
    if (story) {
      const bench = hostJson(host, '__benchState') || {};
      const ws = hostJson(host, '__workState') || {};
      const gs = hostJson(host, '__gameState') || {};
      const prog = Array.isArray(gs.progress) ? gs.progress : [];
      const cards = Array.isArray(bench.spells) ? bench.spells : [];
      return SHIPPED.map((s, i) => {
        const card = cards.find((/** @type {any} */ c) => c.slot === s.slot && c.id === s.id);
        const p = prog.find((/** @type {any} */ x) => x.id === s.id) || {};
        return {
          ...s,
          stage: (card && card.stage) || p.st || 'unknown',
          xp: Number(p.xp) || 0,
          casts: Array.isArray(ws.casts) ? Number(ws.casts[i]) || 0 : 0,
          need: Number(ws.need) || 0,
          learnable: !!card,
        };
      });
    }
    const p = readProgress(o.storage);
    return SHIPPED.map((s) => ({ ...s, stage: p[s.id]?.stage || 'unknown', xp: p[s.id]?.xp || 0, casts: 0, need: 0, learnable: false }));
  };
  /**
   * Saved spells: id, name, equation, band, builds, key, use, usable.
   * @returns {Array<{ id: string, name: string, equation: string, band: string, builds: number, key: number, use?: string, usable: boolean, uses: any[] }>}
   */
  const saved = () => {
    if (story) {
      const ds = hostJson(host, '__discState');
      if (!ds || !Array.isArray(ds.saved)) return [];
      return ds.saved.map((/** @type {any} */ s) => ({
        id: s.id, name: s.name, equation: s.equation, band: s.band, builds: Number(s.builds) || 0, key: s.key || 0,
        use: s.effect && s.effect.use, usable: s.usable !== false, uses: Array.isArray(s.uses) ? s.uses : [],
      }));
    }
    const save = readSpells(o.storage);
    return Object.entries(save.saved).filter(([id]) => o.byId[id]).map(([id, n]) => {
      const r = o.byId[id];
      return { id, name: r.name, equation: r.equation, band: r.band, builds: n, key: save.slots.indexOf(id) + 1, use: o.effects[id]?.use, usable: (RANK[r.band] ?? 0) <= rank(), uses: [] };
    });
  };
  /** @returns {string[]} */
  const foundIds = () => {
    if (story) { const ds = hostJson(host, '__discState'); if (ds && Array.isArray(ds.found)) return ds.found; }
    return readFound(o.storage);
  };
  /** The quests: id, line, hint, done. */
  const questList = () => {
    if (story) {
      const ds = hostJson(host, '__discState');
      return ds && Array.isArray(ds.quests) ? ds.quests : [];
    }
    const done = new Set(readSpells(o.storage).quests);
    if (!quests) { void loadQuests(); return []; }
    return (quests.quests || []).filter((/** @type {any} */ q) => q.live).map((/** @type {any} */ q) => ({
      id: q.id, line: sprout() ? q.line_sprout : q.line, hint: sprout() ? q.hint_sprout : q.hint, done: done.has(q.id),
    }));
  };
  /** A discovery's lowest building band (crates/chem discover::build_band): its own band, at least Seedling. */
  const builds = (/** @type {{ band: string }} */ s) => rank() >= Math.max(RANK[s.band] ?? 0, 1);

  // ---------- the list ----------
  let view = /** @type {'list' | 'build'} */ ('list');
  let lastKey = '';
  const stateKey = () => JSON.stringify([o.band(), shipped(), saved(), foundIds(), questList(), story ? hostJson(host, '__storyState')?.recipes || null : null, readSpells(o.storage)]);

  const badge = (/** @type {string} */ stage) => {
    const b = el('span', 'sb-stage', STAGE_WORD[stage] || 'New');
    b.dataset.stage = stage;
    return b;
  };

  const drawList = () => {
    view = 'list';
    destroyBuilder();
    scale();
    o.box.dataset.band = o.band();
    o.box.replaceChildren();
    const head = el('p', 'note sb-intro', story
      ? 'Build a spell’s formula here to learn it. Every right build is practice: practise enough and it casts at once.'
      : 'This is free play. Practise the spells you saved and pick their keys here. The starting spells are learned at the Bench in the world: press B there.');
    o.box.append(head);

    // Starting spells.
    const sec1 = el('section', 'sb-sec');
    sec1.append(el('h2', '', 'Starting spells'));
    const list = shipped();
    for (const s of list) {
      const row = el('div', 'sb-spell');
      row.dataset.spell = s.id;
      row.dataset.slot = String(s.slot);
      const icon = el('span', `icon ${ICONS[s.id] || 'water'} sb-icon`);
      icon.setAttribute('aria-hidden', 'true');
      const name = el('b', 'sb-name', pretty(s.name));
      const top = el('div', 'sb-top');
      top.append(icon, name, badge(s.stage), el('span', 'sb-key', `Key ${s.slot}`));
      row.append(top);
      const facts = [];
      if (s.xp) facts.push(`${s.xp} XP`);
      if (story && s.need && s.stage !== 'unknown' && s.stage !== 'mastered') {
        const n = Math.min(s.casts, s.need);
        facts.push(n >= s.need ? 'Practised: it casts at once.' : `Practice: ${n} of ${s.need} builds, then it casts at once.`);
      }
      if (facts.length) row.append(el('p', 'sb-facts', facts.join(' · ')));
      if (story && s.learnable) {
        const act = el('div', 'row');
        act.append(button(s.stage === 'unknown' ? 'Learn' : 'Practise', () => startBuild({ kind: 'shipped', id: s.id, slot: s.slot, name: s.name }), s.stage === 'unknown' ? 'primary sb-learn' : 'sb-practise'));
        row.append(act);
      } else if (story && s.id === 'fire' && sprout()) {
        row.append(el('p', 'sb-facts', 'Fire is a starting spell at Sprout: you can cast it already. You build its equation from Seedling up.'));
      } else if (story) {
        row.append(el('p', 'sb-facts', `You learn ${pretty(s.name)} at a higher grade band.`));
      }
      sec1.append(row);
    }
    o.box.append(sec1);

    // Saved spells (discoveries).
    const sec2 = el('section', 'sb-sec');
    sec2.append(el('h2', '', 'Spells you saved'));
    const mine = saved();
    if (!mine.length) sec2.append(el('p', 'note', 'None yet. Find a reaction on the Bench, then save it as a spell.'));
    for (const s of mine) {
      const row = el('div', 'sb-spell sb-disc');
      row.dataset.id = s.id;
      const top = el('div', 'sb-top');
      const instant = sprout() || s.builds >= MASTERY_BUILDS || !builds(s);
      top.append(el('b', 'sb-name', s.name), badge(s.builds >= MASTERY_BUILDS ? 'mastered' : s.builds > 0 ? 'learning' : 'unknown'));
      row.append(top);
      if (!sprout()) {
        const eq = el('div', 'eq');
        eq.innerHTML = o.eq(s.equation);
        row.append(eq);
      }
      if (s.use) row.append(el('p', 'sb-facts', `In the world: it ${s.use}.`));
      const forBand = BAND_NAME[s.band] || 'a higher grade band';
      if (!s.usable) row.append(el('p', 'sb-facts', `This reaction is for ${forBand} and up.`));
      else if (sprout()) row.append(el('p', 'sb-facts', 'At Sprout it casts at once: no building needed.'));
      else if (s.builds >= MASTERY_BUILDS) row.append(el('p', 'sb-facts', `Built ${s.builds} times: it casts at once.`));
      else if (builds(s)) row.append(el('p', 'sb-facts', `Built ${s.builds} of ${MASTERY_BUILDS}. Build it ${MASTERY_BUILDS - s.builds} more time${MASTERY_BUILDS - s.builds === 1 ? '' : 's'} and it casts at once.`));
      for (const u of s.uses) {
        const items = Array.isArray(u.items) ? u.items.join(' or ') : '';
        row.append(el('p', `sb-facts sb-uses${u.have ? '' : ' missing'}`, u.have ? `Casting ${u.used ? 'uses up' : 'needs'} ${items}. You have ${u.have}.` : `Casting needs ${items}. You have none yet. ${u.find || ''}`.trim()));
      }
      const act = el('div', 'row sb-keys');
      if (s.usable && !instant) act.append(button(s.builds ? 'Practise' : 'Learn', () => startBuild({ kind: 'disc', id: s.id, slot: 0, name: s.name }), s.builds ? 'sb-practise' : 'primary sb-learn'));
      if (s.usable) {
        act.append(el('span', 'sb-keylabel', 'Put on key:'));
        for (let k = 1; k <= 4; k++) {
          const on = s.key === k;
          const b = button(String(k), () => putKey(s.id, on ? 0 : k, s.key), 'sb-keybtn');
          b.dataset.key = String(k);
          b.setAttribute('aria-pressed', on ? 'true' : 'false');
          b.setAttribute('aria-label', on ? `On key ${k}: take it off` : `Put ${s.name} on key ${k}, with ${pretty(SHIPPED[k - 1]?.name || '')}`);
          act.append(b);
        }
      }
      row.append(act);
      sec2.append(row);
    }
    o.box.append(sec2);

    // Found reactions not saved yet.
    const savedIds = new Set(mine.map((s) => s.id));
    const can = foundIds().map((id) => o.byId[id]).filter((r) => r && !savedIds.has(r.id) && o.effects[r.id] && !o.effects[r.id].bench_only && (RANK[r.band] ?? 0) <= rank());
    if (can.length) {
      const sec3 = el('section', 'sb-sec');
      sec3.append(el('h2', '', 'Found, not saved yet'));
      for (const r of can) {
        const row = el('div', 'sb-spell sb-found');
        row.dataset.id = r.id;
        row.append(el('b', 'sb-name', r.name));
        const use = o.effects[r.id]?.use;
        if (use) row.append(el('p', 'sb-facts', `In the world: it ${use}.`));
        const act = el('div', 'row');
        act.append(button('Save as a spell', () => { if (saveSpell(o.storage, r.id)) { o.onSaved(); lastKey = ''; draw(); } }, 'sb-save'));
        row.append(act);
        sec3.append(row);
      }
      o.box.append(sec3);
    }

    // Chemistry quests.
    const qs = questList();
    if (qs.length) {
      const sec4 = el('section', 'sb-sec');
      sec4.append(el('h2', '', `Chemistry quests (${qs.filter((/** @type {any} */ q) => q.done).length} of ${qs.length} done)`));
      for (const q of qs) {
        const p = el('div', `sb-quest${q.done ? ' done' : ''}`);
        p.dataset.quest = q.id;
        p.append(el('p', '', `${q.done ? '✓ ' : ''}${q.line || q.id}`));
        if (!q.done && q.hint) p.append(el('p', 'sb-facts', String(q.hint)));
        sec4.append(p);
      }
      if (!story) sec4.append(el('p', 'note', 'Quests are done in the world, with a spell you saved here.'));
      o.box.append(sec4);
    }

    // Story mode: the old Bench's "Make things" (story.rs recipes).
    const recipes = story ? hostJson(host, '__storyState')?.recipes : null;
    if (Array.isArray(recipes) && hostJson(host, '__gameState')?.mode === 'voxel') {
      const sec5 = el('section', 'sb-sec');
      sec5.append(el('h2', '', 'Make things'));
      const itemName = (/** @type {string} */ key) => /** @type {Record<string, string>} */ ({ coal_ore: 'coal', torch: 'torches', furnace: 'furnace' })[key] || key.replace(/_/g, ' ');
      for (const r of recipes) {
        const needs = r.needs.map((/** @type {any} */ n) => `${n.n} ${itemName(n.item)} (you have ${n.have})`).join(' + ');
        const b = button(`${r.gives.n} ${itemName(r.gives.item)}: ${needs}`, () => {
          if (!r.can) return;
          host.__gameIn = host.__gameIn || {};
          (host.__gameIn.story = host.__gameIn.story || []).push(`craft ${r.id}`);
        }, `sb-craft${r.can ? '' : ' short'}`);
        b.dataset.recipe = r.id;
        b.setAttribute('aria-disabled', r.can ? 'false' : 'true');
        sec5.append(b);
      }
      o.box.append(sec5);
    }
    lastKey = stateKey();
    focusStart();
  };
  // The keyboard starts here when nothing else has focus (the Bench just
  // opened over the world): the first control of the tab.
  const focusStart = () => {
    const ae = o.box.ownerDocument.activeElement;
    if ((!ae || ae === o.box.ownerDocument.body || !ae.isConnected) && o.box.getClientRects().length) {
      /** @type {HTMLElement | null} */ (o.box.querySelector('button'))?.focus();
    }
  };

  const putKey = (/** @type {string} */ id, /** @type {number} */ k, /** @type {number} */ was) => {
    if (story) discPush(k ? `equip ${id} ${k}` : `unequip ${was}`);
    else putOnKey(o.storage, id, k);
    lastKey = '';
    setTimeout(draw, 120);
  };

  // ---------- the build view ----------
  /** @type {{ destroy: () => void } | null} */
  let builder = null;
  /** @type {{ kind: 'shipped' | 'disc', id: string, slot: number, name: string, card?: any, recipe?: any, mounted?: boolean, seenN: number, awaiting: boolean, sentAt: number, tries: number } | null} */
  let job = null;
  /** @type {HTMLElement | null} */
  let feedback = null;
  /** @type {HTMLElement | null} */
  let mountBox = null;
  const destroyBuilder = () => { try { builder?.destroy(); } catch { /* gone */ } builder = null; };

  /** @param {{ kind: 'shipped' | 'disc', id: string, slot: number, name: string }} what */
  const startBuild = (what) => {
    view = 'build';
    destroyBuilder();
    const bench = story ? hostJson(host, '__benchState') : null;
    job = { ...what, seenN: Number(bench?.result?.n) || 0, awaiting: false, sentAt: 0, tries: 0 };
    o.box.replaceChildren();
    const wrap = el('div', 'sb-build');
    wrap.append(button('All spells', () => { if (story) benchPush('open'); job = null; drawList(); }, 'sb-back'));
    mountBox = el('div', 'sb-mount');
    mountBox.append(el('p', 'note', 'Getting the bench ready…'));
    feedback = el('div', 'sb-feedback');
    feedback.setAttribute('aria-live', 'polite');
    wrap.append(mountBox, feedback);
    o.box.append(wrap);
    if (story) benchPush(what.kind === 'shipped' ? `pick ${what.slot}` : `learn ${what.id}`);
    else mountLocal();
    tick();
  };

  /** Free play: a discovery's card and recipe from the codex. */
  const mountLocal = () => {
    if (!job) return;
    const r = o.byId[job.id];
    if (!r) return;
    const b = o.band();
    /** @type {any[]} */
    const recipe = [];
    r.equation.split(' -> ').forEach((/** @type {string} */ side, /** @type {number} */ k) => {
      for (const t of side.split(' + ')) {
        const m = t.trim().match(/^(\d*)(.+)$/);
        if (!m) continue;
        const formula = m[2] ?? '';
        recipe.push({ side: k, coef: Number(m[1]) || 1, formula, atoms: o.atoms(formula) || [], state: null });
      }
    });
    job.card = { slot: 1, id: r.id, name: r.name, formula: r.equation, stage: 'learning', prompt: b === 'seedling' ? `${r.name}: change the numbers until the scale is level.` : `${r.name}: write the balanced equation.` };
    job.recipe = b === 'seedling' ? recipe : null;
    mountNow();
  };

  const mountNow = () => {
    if (!job || !mountBox || !job.card) return;
    job.mounted = true;
    const j = job;
    builder = mountBuilder(mountBox, {
      band: o.band(),
      spell: j.card,
      recipe: j.recipe || null,
      prompt: j.card.prompt,
      onMake: (text) => {
        if (j.awaiting && performance.now() - j.sentAt < 2000) return;
        if (story) {
          j.awaiting = true;
          j.sentAt = performance.now();
          benchPush(`try ${j.card.slot} ${text}`.trimEnd());
        } else judgeLocal(text);
      },
    });
    const first = /** @type {HTMLElement | null} */ (mountBox.querySelector('#bench-input') || mountBox.querySelector('[data-token]') || mountBox.querySelector('button'));
    first?.focus();
    if (j.card.prompt) speak(j.card.prompt);
  };

  /** Free play: the chemistry engine judges a discovery's build. */
  const judgeLocal = (/** @type {string} */ text) => {
    if (!job) return;
    const r = o.byId[job.id];
    let raw = null;
    try { raw = o.judge(text, (r.needs || []).join(','), o.band()); } catch { raw = null; }
    if (raw && raw.kind === 'known' && raw.id === job.id) {
      const n = recordBuild(o.storage, job.id);
      o.onSaved();
      showDone({ ok: true, code: 'ok', slot: 1 }, n);
      return;
    }
    job.tries++;
    let line = 'Not yet: that is not this reaction. Look at the equation and try again.';
    if (raw && raw.kind === 'known' && o.byId[raw.id]) line = `That makes ${o.byId[raw.id].name}. This one is ${r.name}.`;
    else if (raw && raw.kind === 'unbalanced') line = `The scale is not level. ${(raw.report || []).join(' ')}`.trim();
    else if (raw && raw.kind === 'parse_error') line = String(raw.message || line);
    showFizzle(line);
  };

  /** A ball with its letter (bench.js draws the same in the builder). */
  const ball = (/** @type {string} */ e) => { const b = el('span', `bench-ball el-${e}`, e); b.dataset.el = e; return b; };
  /**
   * The fizzle: the band's reason, and (as the old Bench did) a picture diff
   * per element at Sprout (the balls built against the balls wanted) or the
   * before and after counts at Seedling.
   * @param {string} line @param {any} [r]
   */
  const showFizzle = (line, r) => {
    if (!feedback) return;
    const box = el('div', 'sb-fizzle');
    const mark = el('span', 'sb-mark cross');
    mark.setAttribute('aria-hidden', 'true');
    box.append(mark, el('p', 'sb-reason', line));
    const diffs = (r && Array.isArray(r.diffs)) ? r.diffs : [];
    const recipe = job && Array.isArray(job.recipe) ? job.recipe : [];
    if (r && o.band() === 'sprout' && r.code === 'atoms_off') {
      /** @type {Record<string, [number, number]>} */
      const rows = {};
      /** @type {string[]} */
      const order = [];
      for (const t of recipe) for (const [e, n] of t.atoms) { rows[e] = [n, n]; if (!order.includes(e)) order.push(e); }
      for (const [e, have, want] of diffs) { rows[e] = [have, want]; if (!order.includes(e)) order.push(e); }
      const list = el('div', 'bench-diffs');
      for (const e of order) {
        const [have, want] = rows[e] || [0, 0];
        const row = el('div', 'bench-diff');
        row.dataset.el = e;
        row.dataset.ok = String(have === want);
        const h = el('span', 'bench-diff-have');
        for (let k = 0; k < have; k++) h.append(ball(e));
        const wnt = el('span', 'bench-diff-want');
        for (let k = 0; k < want; k++) wnt.append(ball(e));
        const m = el('span', `bench-mark sb-mark ${have === want ? 'tick' : 'cross'}`);
        m.setAttribute('aria-hidden', 'true');
        row.append(h, wnt, m);
        list.append(row);
      }
      box.append(list);
    } else if (r && o.band() === 'seedling' && r.code === 'unbalanced') {
      const list = el('div', 'bench-diffs');
      for (const [e, l, rt] of diffs) {
        const row = el('div', 'bench-diff', `${e}: ${l} before, ${rt} after`);
        row.dataset.el = e;
        row.dataset.ok = 'false';
        list.append(row);
      }
      box.append(list);
    }
    feedback.replaceChildren(box);
    speak(line);
  };

  /** @param {any} r the judged result @param {number} [builtNow] free play's new count */
  const showDone = (r, builtNow) => {
    if (!job) return;
    const j = job;
    destroyBuilder();
    const names = { [r.slot]: { name: pretty(j.name), formula: j.card?.formula || '' } };
    const line = reasonFor(o.band(), { ...r, code: 'ok' }, names, 0);
    o.box.replaceChildren();
    const wrap = el('div', 'sb-done');
    const icon = el('span', `icon ${ICONS[j.kind === 'shipped' ? j.id : ''] || 'water'} sb-done-icon`);
    icon.setAttribute('aria-hidden', 'true');
    const tick = el('span', 'sb-mark tick big');
    tick.setAttribute('aria-hidden', 'true');
    wrap.append(icon, tick, el('p', 'sb-done-line', line));
    if (r.learned) wrap.append(badge(r.stage || 'learning'), el('p', 'sb-done-extra', `You learned ${pretty(j.name)}! Cast it with key ${j.slot}.`));
    // The old Bench's Apprentice lines, for Apprentice and Adept (Adept builds as Apprentice does).
    if (rank() >= 2) {
      if (r.simplest) wrap.append(el('p', 'sb-done-extra', `Simplest form: ${pretty(r.simplest)}`));
      const coef = r.coef || 0;
      const formula = pretty(names[r.slot]?.formula || '');
      if (coef > 1) wrap.append(el('p', 'sb-done-extra', `A ${coef} in front would mean ${coef} molecules. One molecule is ${formula}.`));
    }
    const count = el('p', 'sb-done-count', '');
    wrap.append(count);
    const fill = () => {
      if (j.kind === 'disc') {
        const n = builtNow ?? (saved().find((s) => s.id === j.id)?.builds ?? 0);
        count.textContent = n >= MASTERY_BUILDS ? `${j.name}: mastered! It casts at once now.` : `${j.name}: built ${n} of ${MASTERY_BUILDS}. ${MASTERY_BUILDS - n} more and it casts at once.`;
        return;
      }
      const s = shipped().find((x) => x.id === j.id);
      if (!s || !s.need) return;
      const n = Math.min(s.casts, s.need);
      count.textContent = n >= s.need ? `${pretty(j.name)}: Practised! It casts at once now.` : `${pretty(j.name)}: ${n} of ${s.need} builds to Practised. ${s.need - n} more and it casts at once.`;
    };
    fill();
    for (const ms of [150, 400, 900, 1600]) setTimeout(fill, ms);
    const row = el('div', 'row sb-actions');
    const again = button('Build it again', () => { if (story) benchPush('open'); startBuild({ kind: j.kind, id: j.id, slot: j.slot, name: j.name }); }, 'primary sb-again');
    row.append(again, button('All spells', () => { if (story) benchPush('open'); job = null; drawList(); }, 'sb-back'));
    wrap.append(row);
    o.box.append(wrap);
    job = { ...j, mounted: false, awaiting: false };
    view = 'build';
    again.focus();
    rise();
    speak(line);
  };

  // ---------- polling (story mode follows the world's state) ----------
  const tick = () => {
    if (view === 'list') {
      if (stateKey() !== lastKey) drawList();
      else focusStart();
      return;
    }
    if (!story || !job) return;
    const st = hostJson(host, '__benchState');
    if (!st || !st.open) return;
    if (!job.mounted && mountBox && mountBox.isConnected) {
      const wantRecipe = rank() < 2;
      const card = job.kind === 'disc'
        ? (st.disc === job.id && Array.isArray(st.spells) ? st.spells.find((/** @type {any} */ c) => c.id === job?.id) : null)
        : (!st.disc && st.pick === job.slot && Array.isArray(st.spells) ? st.spells.find((/** @type {any} */ c) => c.slot === job?.slot) : null);
      if (card && st.pick === card.slot && (!wantRecipe || st.recipe)) {
        job.card = card;
        job.recipe = st.recipe || null;
        job.slot = card.slot;
        mountNow();
      }
      return;
    }
    const r = st.result;
    if (job.mounted && job.awaiting && r && typeof r.n === 'number' && r.n > job.seenN && r.slot === job.card?.slot) {
      job.seenN = r.n;
      job.awaiting = false;
      if (r.ok) showDone(r);
      else {
        if (r.code === 'atoms_off' || r.code === 'unbalanced') job.tries++;
        const names = { [r.slot]: { name: pretty(job.name), formula: job.card?.formula || '' } };
        showFizzle(reasonFor(o.band(), r, names, job.tries), r);
      }
    }
  };

  const draw = () => { if (view === 'list') drawList(); };
  return {
    draw,
    tick,
    /** Back to the list (a band change, the Bench opening again). */
    reset: () => { job = null; drawList(); },
    state: () => ({ view, job: job ? { kind: job.kind, id: job.id, mounted: !!job.mounted } : null }),
  };
}
