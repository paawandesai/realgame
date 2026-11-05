// Chemistry Bench free play (Bench plan Phase 1): the approved mock-up's
// bench, now judged by the real chemistry engine (crates/chem, built to
// chem.js + chem_bg.wasm by build-web.sh). Every result comes from the
// engine: a known reaction, one that needs a condition, one taught at a
// higher band, an unbalanced try, a mystery card the model predicts, or no
// reaction. Three.js r128 is vendored in vendor/ (no CDN).
//
// Saved in this browser only: the band, the codex entries found, mystery
// cards, the notebook, balanced equations and which bands have seen the
// Guide walkthrough (keys start `formula.bench.`). The Guide tab's content
// (walkthroughs, lessons, hints, challenges, everyday lines) comes from
// content/chem/guide.json, shipped as chem-guide.json.

import init, { codex_json, effects_json, judge_json, literature_key_json, names_json, show_json } from './chem.js';
import { readSpells, saveSpell, readFound, loadCommands } from './formula-save.js';
import { mountSpells } from './bench-spells.js';
import { parseFormula as parseWith, hill, sideOf, sideText } from './bench-free-lib.js';
import { createBenchScene } from './bench-scene.js';

// Element display data and everyday names, from content/chem/names.json.
/** @type {Record<string, {v:number,c:number,r:number,name:string}>} */
const EL = {};
/** @type {Record<string, string>} */
const NAMES = {};
const parseFormula = (/** @type {string} */ f) => parseWith(f, (el) => !!EL[el]);

const BANDS = ['sprout', 'seedling', 'apprentice', 'adept'];
// Grade bands (I-059): band names with the school grades they are for. The
// ids are unchanged, so progress saved by band stays valid.
const BAND_NAME = { sprout: 'Sprout (grades 6–7)', seedling: 'Seedling (grades 8–9)', apprentice: 'Apprentice (grades 10–11)', adept: 'Adept (grade 12+)' };

/** @type {Set<string>} */
const GASES = new Set();
/** @type {string[][]} */
const COND = [];
const condName = (/** @type {string} */ k) => (COND.find((c) => c[0] === k) || [k, k])[1].toLowerCase();

const subs = (/** @type {string} */ f) => f.replace(/\d/g, (d) => '₀₁₂₃₄₅₆₇₈₉'[+d]);
/** A formula for display, with subscripts but leading coefficients kept. */
const show = (/** @type {string} */ f) => subs(f);
const showEq = (/** @type {string} */ eq) => eq.split(' -> ').map((s) => s.split(' + ').map((t) => { const m = t.match(/^(\d*)(.+)$/); return m ? m[1] + subs(m[2]) : t; }).join(' + ')).join(' → ');
const hex = (/** @type {number} */ n) => '#' + n.toString(16).padStart(6, '0');
const LAB_CREDIT = 'Lab records: Open Reaction Database (Kearnes et al. 2021), CC BY-SA 4.0. <a href="https://github.com/open-reaction-database/ord-data" target="_blank" rel="noopener">ord-data</a>';
const esc = (/** @type {string} */ s) => s.replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c] || c);

const store = {
  get(/** @type {string} */ k, /** @type {any} */ d) { try { const v = JSON.parse(localStorage.getItem('formula.bench.' + k) || 'null'); return v ?? d; } catch { return d; } },
  set(/** @type {string} */ k, /** @type {any} */ v) { try { localStorage.setItem('formula.bench.' + k, JSON.stringify(v)); } catch { /* no storage */ } },
};

// ---------- story mode: the one Bench over the world (I-060) ----------
// bench.js on the world page shows this page as `bench-free.html?mode=story`
// in a same-origin frame. Then the bottles are the pack's (the world's
// `__discState.shelf`) and the air's, the band is the world's, and Escape or
// "Back to the world" close it. Opened any other way it is free play.

/**
 * The world page, when this Bench is its in-world overlay; else null.
 * @param {URLSearchParams} q
 * @returns {any}
 */
function storyHost(q) {
  if (q.get('mode') !== 'story') return null;
  try {
    const p = /** @type {any} */ (window.parent);
    return p && p !== window && p.__bench ? p : null;
  } catch {
    return null;
  }
}

/** @param {any} w @param {string} k */
const hostState = (w, k) => { try { return JSON.parse(w[k] || 'null'); } catch { return null; } };

/**
 * The story shelf: each substance the pack or the air gives, with how many
 * the pack holds (air: unlimited), and every bottle item's find line.
 * @param {any} host
 */
function storyShelf(host) {
  const ds = hostState(host, '__discState') || {};
  /** @type {Record<string, {n: number, air: boolean, labels: string[]}>} */
  const have = {};
  /** @type {Record<string, {label: string, find: string}>} */
  const find = {};
  for (const f of Array.isArray(ds.air) ? ds.air : ['O2', 'N2']) have[f] = { n: 0, air: true, labels: [] };
  for (const b of Array.isArray(ds.shelf) ? ds.shelf : []) {
    if (!find[b.species]) find[b.species] = { label: b.label, find: b.find };
    if (!(b.n > 0)) continue;
    const h = (have[b.species] ??= { n: 0, air: false, labels: [] });
    h.n += b.n;
    if (!h.labels.includes(b.label)) h.labels.push(b.label);
  }
  return { have, find, list: Object.keys(have), key: JSON.stringify([ds.air, ds.shelf]) };
}

async function main() {
  const $ = (/** @type {string} */ id) => /** @type {HTMLElement} */ (document.getElementById(id));
  await init();
  const names = JSON.parse(names_json());
  for (const [k, e] of Object.entries(names.elements)) { const x = /** @type {any} */ (e); EL[k] = { v: x.bonds, c: parseInt(x.color.slice(1), 16), r: x.radius, name: x.name }; }
  Object.assign(NAMES, names.substances);
  for (const g of names.gases) GASES.add(g);
  COND.push(...names.conditions);
  /** @type {{families: string[], reactions: any[]}} */
  const codex = JSON.parse(codex_json());
  const byId = Object.fromEntries(codex.reactions.map((r) => [r.id, r]));
  const rank = (/** @type {string} */ b) => BANDS.indexOf(b);

  // Formula as the codex writes it, by atom counts (so a built CH3COOH reads as vinegar).
  /** @type {Record<string, string>} */
  const written = {};
  for (const r of codex.reactions) for (const s of r.equation.split(' -> ')) for (const f of Object.keys(sideOf(s))) {
    const c = parseFormula(f); if (c) written[hill(c)] = written[hill(c)] || f;
  }

  const q = new URLSearchParams(location.search);
  const story = storyHost(q);
  const hostBand = () => { try { const b = story && story.__bench.band(); return BANDS.includes(b) ? b : null; } catch { return null; } };
  let band = story ? hostBand() || 'apprentice' : BANDS.includes(q.get('band') || '') ? /** @type {string} */ (q.get('band')) : store.get('band', 'apprentice');
  /** @type {string[]} */
  let found = store.get('found', []);
  /** @type {string[]} */
  let mysteries = store.get('mysteries', []);
  /** @type {string[]} */
  let labFinds = store.get('lab', []);
  /** @type {{t:string, eq:string, what:string, ok:boolean}[]} */
  let notes = store.get('notes', []);
  /** Reactions the player balanced by writing both sides (Guide challenges). @type {string[]} */
  let balanced = store.get('balanced', []);

  // ---------- grade bands (I-059) ----------
  // Sprout (grades 6-7) sees formulas, with a word equation under each
  // equation, and never balances by hand: its result card offers "Show me
  // how" instead. Sprout and Seedling (grades 8-9) get tap-to-read (an
  // accessibility option), colour-coded atom counts, a "show me" for
  // balancing and safety badges in plain words. See
  // docs/loop/items/I-059/spec.md.
  const isSprout = () => band === 'sprout';
  const isYoung = () => rank(band) <= rank('seedling');
  const nameOf = (/** @type {string} */ f) => NAMES[f] || f;
  /** A word equation: each formula's everyday name. */
  const wordEq = (/** @type {string} */ eq) => eq.split(' -> ').map((side) => side.split(' + ').map((t) => nameOf(t.replace(/^\d+/, '').trim())).join(' + ')).join(' → ');
  /** An equation as this band reads it: formulas, with the word equation under it at Sprout. */
  const eqFor = (/** @type {string} */ eq) => (isSprout() ? `${showEq(eq)}<br><small>${esc(wordEq(eq))}</small>` : showEq(eq));
  /** Conditions a Sprout never needs: hidden to keep the panel small. */
  const SPROUT_HIDES = new Set(['electricity', 'pressure']);
  /** @type {Record<string, string>} */
  const YOUNG_COND = names.conditions_young || {};
  const HAZARD_YOUNG = { fire: '🔥 Fire and heat: only try this with a grown-up.', lab: '🧪 Dangerous for real: only trained chemists, in a proper lab.' };
  const HAZARD_OLDER = { fire: '🔥 Lab safety: flames or strong heat. Only with supervision, goggles and a fire-safe bench.', lab: '🧪 Lab only: reactive, poisonous or explosive for real. Trained chemists with proper equipment only.' };
  const hazardText = (/** @type {string} */ h) => (isYoung() ? HAZARD_YOUNG : HAZARD_OLDER)[/** @type {'fire'|'lab'} */ (h)];

  // ---------- story mode helpers (I-060) ----------
  /** @type {ReturnType<typeof mountSpells> | null} */
  let spells = null;
  let shelfKey = '';
  const storyShelfKey = () => (story ? storyShelf(story).key : '');
  /** A bottle's count line: how many the pack holds, or that the air gives it. */
  const storyLabel = (/** @type {HTMLElement} */ b, /** @type {{n: number, air: boolean, labels: string[]} | undefined} */ h) => {
    if (!h) return;
    const tag = document.createElement('span'); tag.className = 'have';
    tag.textContent = h.air ? 'from the air' : `${h.labels.join(' or ')} ×${h.n}`;
    b.append(tag);
    shelfKey = storyShelfKey();
  };
  const storyNote = (/** @type {ReturnType<typeof storyShelf>} */ sh) => {
    shelfKey = sh.key;
    const fromPack = sh.list.filter((f) => !sh.have[f]?.air).length;
    return fromPack
      ? 'Your bottles come from your pack and the air. Running a reaction here does not use them up; casting it as a spell does.'
      : 'Only the air’s gases so far. Find bottles in the world (your pack, Tab, says where) and they show up here.';
  };
  /** Story mode: "in" may only hold what the pack or the air gives. */
  const storyMissing = (/** @type {Record<string, number>} */ inSide) => {
    if (!story) return '';
    const sh = storyShelf(story);
    const gone = Object.keys(inSide).filter((f) => !sh.have[f]);
    if (!gone.length) return '';
    const lines = gone.map((f) => {
      const how = sh.find[f];
      return `<p><b>${esc(`${nameOf(f)} (${f})`)}</b>: ${how ? `${esc(how.label)}. ${esc(how.find)}` : 'nothing in the valley gives this one yet. Try it in free play.'}</p>`;
    }).join('');
    return `<p><b>You don’t have that in your pack yet.</b> In the story, bottles come from the world.</p>${lines}`;
  };
  /** Story mode: tell the world the save changed (it also hears the storage event). */
  const storySync = () => {
    if (!story) return;
    try {
      story.__gameIn = story.__gameIn || {};
      const cmds = loadCommands(readSpells(localStorage), readFound(localStorage));
      (story.__gameIn.disc = story.__gameIn.disc || []).push(...cmds);
    } catch { /* the world reads the save on its next load */ }
  };
  let openedOnce = false;
  const askBand = () => {
    const box = $('bandAsk'); const row = $('bandAskRow'); row.innerHTML = '';
    for (const b of BANDS) {
      const x = document.createElement('button'); x.type = 'button'; x.className = 'btn'; x.dataset.band = b;
      x.textContent = BAND_NAME[/** @type {keyof typeof BAND_NAME} */ (b)];
      // The world's age picker draws a picture per band; reuse it here.
      try { const pic = story?.document.querySelector(`#age button[data-band="${b}"] svg`); if (pic) x.prepend(pic.cloneNode(true)); } catch { /* no picture */ }
      x.addEventListener('click', () => { box.hidden = true; setBand(b, true); showTab('spells'); });
      row.append(x);
    }
    box.hidden = false;
    /** @type {HTMLElement | null} */ (row.querySelector('button'))?.focus();
  };
  const setupStory = () => {
    const chip = $('modeChip');
    if (!story) return;
    chip.dataset.mode = 'story';
    chip.textContent = 'Story: your bottles come from your pack';
    document.body.dataset.mode = 'story';
    const back = $('toGame');
    back.addEventListener('click', (e) => { e.preventDefault(); story.__bench.close(); });
    document.addEventListener('keydown', (e) => {
      if (e.key !== 'Escape') return;
      e.preventDefault();
      story.__bench.close('escape');
    });
    if (story.__bench.fixed()) document.querySelectorAll('.band button').forEach((x) => { /** @type {HTMLButtonElement} */ (x).disabled = true; });
    /** @type {any} */ (window).__benchStory = {
      opened: () => {
        const hb = hostBand();
        if (hb && hb !== band) setBand(hb);
        drawShelves();
        if (!openedOnce) { openedOnce = true; showTab('spells'); }
        spells?.reset();
        if (!hb) askBand();
      },
      closed: () => { try { speechSynthesis.cancel(); } catch { /* no speech */ } },
    };
  };

  // ---------- 3D scene (bench-scene.js) ----------
  const scene3d = createBenchScene(/** @type {HTMLCanvasElement} */ ($('scene')), { elements: EL });

  // Bonds from the atoms alone: heavy atoms chain, hydrogens fill, then bond
  // orders rise where both ends still have room. A picture, not a claim.
  const structure = (/** @type {string[]} */ els) => {
    const n = els.length, bonds = /** @type {{a:number,b:number,o:number}[]} */ ([]), used = new Array(n).fill(0);
    const free = (/** @type {number} */ i) => EL[els[i]].v - used[i];
    const join = (/** @type {number} */ a, /** @type {number} */ b) => { bonds.push({ a, b, o: 1 }); used[a]++; used[b]++; };
    const heavy = els.map((e, i) => i).filter((i) => els[i] !== 'H').sort((a, b) => EL[els[b]].v - EL[els[a]].v);
    /** @type {number[]} */
    const placed = [];
    for (const i of heavy) {
      const to = placed.filter((j) => free(j) > 0).sort((a, b) => free(b) - free(a))[0];
      if (to !== undefined && free(i) > 0) join(i, to);
      placed.push(i);
    }
    for (const h of els.map((e, i) => i).filter((i) => els[i] === 'H')) {
      const to = [...placed].filter((j) => j !== h && free(j) > 0).sort((a, b) => Number(els[a] === 'H') - Number(els[b] === 'H') || free(b) - free(a))[0];
      if (to !== undefined && free(h) > 0) join(h, to);
      placed.push(h);
    }
    let grew = true;
    while (grew) { grew = false; for (const b of bonds) if (b.o < 3 && free(b.a) > 0 && free(b.b) > 0) { b.o++; used[b.a]++; used[b.b]++; grew = true; } }
    const seen = new Set([0]), st = [0];
    while (st.length) { const i = /** @type {number} */ (st.pop()); for (const b of bonds) { const j = b.a === i ? b.b : b.b === i ? b.a : -1; if (j >= 0 && !seen.has(j)) { seen.add(j); st.push(j); } } }
    const left = els.map((e, i) => i).filter((i) => free(i) > 0);
    return { bonds, complete: n > 0 && left.length === 0 && seen.size === n, left: left.map((i) => els[i]), parts: seen.size < n };
  };
  const atomsOf = (/** @type {string} */ f) => Object.entries(parseFormula(f) || {}).flatMap(([e, k]) => Array(k).fill(e));

  // ---------- the plate ----------
  /** @type {{el:string}[]} */
  let atoms = [];
  let plateInfo = { bonds: /** @type {{a:number,b:number,o:number}[]} */ ([]), complete: false, left: /** @type {string[]} */ ([]), parts: false };
  const plateFormula = () => { const c = atoms.reduce((m, a) => ((m[a.el] = (m[a.el] || 0) + 1), m), /** @type {Record<string, number>} */ ({})); const h = hill(c); return written[h] || h; };
  const plateKnown = () => atoms.length > 0 && !!written[hill(atoms.reduce((m, a) => ((m[a.el] = (m[a.el] || 0) + 1), m), /** @type {Record<string, number>} */ ({})))];
  const plateOk = () => plateInfo.complete || plateKnown();
  const setStatus = (/** @type {string} */ t, /** @type {boolean} */ ok = false) => { const s = $('status'); s.textContent = t; s.classList.toggle('ok', ok); };
  const readout = () => {
    const f = plateFormula();
    $('formula').innerHTML = `${atoms.length ? show(f) : '–'}<small>${atoms.length ? esc(NAMES[f] || 'no common name') : 'empty plate'}</small>`;
    const ok = plateOk();
    /** @type {HTMLButtonElement} */ ($('toLeft')).disabled = !ok; /** @type {HTMLButtonElement} */ ($('toRight')).disabled = !ok;
    if (!atoms.length) setStatus('Put atoms on the plate, or take a bottle from the shelf.');
    else if (ok) setStatus(plateKnown() ? 'A real substance. Add it to the table.' : 'Every bond is filled. This molecule is complete.', true);
    else if (plateInfo.parts) setStatus('These atoms can’t all join into one molecule. Take one off or add another.');
    else setStatus(`Not finished: ${[...new Set(plateInfo.left)].map((e) => e + ' still has room for a bond').join(', ')}.`);
  };
  const restructure = () => { plateInfo = atoms.length ? structure(atoms.map((a) => a.el)) : { bonds: [], complete: false, left: [], parts: false }; scene3d.setPlateBonds(plateInfo.bonds, plateOk()); readout(); };
  const addAtom = (/** @type {string} */ el) => {
    if (atoms.length >= 16) { setStatus('The plate holds up to 16 atoms. Use a bottle for bigger ones.'); return; }
    scene3d.addPlateAtom(el); atoms.push({ el }); restructure(); tone(el);
  };
  const clearPlate = () => { scene3d.clearPlate(); atoms = []; restructure(); };
  scene3d.onPickPlateAtom((i) => { if (i < 0 || i >= atoms.length) return; scene3d.removePlateAtom(i); atoms.splice(i, 1); restructure(); });

  // ---------- the table ----------
  /** @type {Record<string, number>} */
  const left = {};
  /** @type {Record<string, number>} */
  const right = {};
  const conds = new Set();
  const totals = (/** @type {Record<string, number>} */ side) => { /** @type {Record<string, number>} */ const t = {}; for (const [f, n] of Object.entries(side)) for (const [e, k] of Object.entries(parseFormula(f) || {})) t[e] = (t[e] || 0) + k * n; return t; };
  const drawSide = (/** @type {HTMLElement} */ box, /** @type {Record<string, number>} */ side, /** @type {string} */ label) => {
    box.innerHTML = `<span class="lbl">${label}</span>`;
    const keys = Object.keys(side);
    if (!keys.length) box.insertAdjacentHTML('beforeend', `<span class="empty">${label === 'In' ? 'Take a bottle, or build a molecule.' : 'Optional: your guess.'}</span>`);
    for (const f of keys) {
      const row = document.createElement('div'); row.className = 'mol';
      row.innerHTML = `<strong title="${esc(NAMES[f] || f)}"><span class="coef">${side[f] > 1 ? side[f] : ''}</span>${show(f)}</strong>`;
      const minus = document.createElement('button'); minus.type = 'button'; minus.textContent = '−'; minus.setAttribute('aria-label', `One less ${f}`);
      const plus = document.createElement('button'); plus.type = 'button'; plus.textContent = '+'; plus.setAttribute('aria-label', `One more ${f}`);
      minus.addEventListener('click', () => { side[f]--; if (side[f] <= 0) delete side[f]; drawTable(); });
      plus.addEventListener('click', () => { side[f] = Math.min(side[f] + 1, 40); drawTable(); });
      row.append(minus, plus); box.append(row);
    }
  };
  let tilt = 0;
  const drawTable = () => {
    drawSide($('leftSide'), left, 'In'); drawSide($('rightSide'), right, 'Out');
    const L = totals(left), R = totals(right); const els = [...new Set([...Object.keys(L), ...Object.keys(R)])];
    const guessing = Object.keys(right).length > 0;
    let diff = 0; let html = '<span class="h">Atom</span><span class="h">In</span><span class="h">Out</span><span class="h"></span>';
    for (const e of els) { const a = L[e] || 0, b = R[e] || 0; diff += (a - b); html += `<span>${isYoung() ? `<i class="dot" style="background:${hex(EL[e]?.c ?? 0xcccccc)}"></i>${esc(EL[e]?.name ?? e)}` : e}</span><span>${a}</span><span>${b}</span><span class="${a === b ? 'ok' : 'no'}">${a === b ? '✓' : '✗'}</span>`; }
    $('tally').innerHTML = guessing && els.length ? html : '<span class="empty" style="grid-column:1/-1">Fill in “out” and the scale weighs the atoms on each side.</span>';
    tilt = guessing ? Math.max(-14, Math.min(14, diff * 1.5)) : 0;
    previewIn();
  };
  const putOn = (/** @type {Record<string, number>} */ side, /** @type {string} */ f) => { side[f] = Math.min((side[f] || 0) + 1, 40); drawTable(); };
  $('toLeft').addEventListener('click', () => { if (plateOk()) { putOn(left, plateFormula()); clearPlate(); } });
  $('toRight').addEventListener('click', () => { if (plateOk()) { putOn(right, plateFormula()); clearPlate(); } });
  $('clear').addEventListener('click', clearPlate);

  // ---------- shelves ----------
  const visible = (/** @type {any} */ r) => rank(r.band) <= rank(band);
  const drawShelves = () => {
    /** @type {Set<string>} */
    const species = new Set();
    for (const r of codex.reactions) if (visible(r)) for (const f of Object.keys(sideOf(r.equation.split(' -> ')[0]))) species.add(f);
    const shelf = story ? storyShelf(story) : null;
    const list = (shelf ? shelf.list : [...species]).sort((a, b) => (NAMES[a] || a).localeCompare(NAMES[b] || b));
    const box = $('bottles'); box.innerHTML = '';
    for (const f of list) {
      const b = document.createElement('button'); b.type = 'button'; b.className = 'bottle';
      const dots = atomsOf(f).slice(0, 8).map((e) => `<i style="background:${hex(EL[e].c)}"></i>`).join('');
      b.innerHTML = `<span class="dots">${dots}</span><span><b>${show(f)}</b><small>${esc(NAMES[f] || '')}</small></span>`;
      b.setAttribute('aria-label', `${NAMES[f] || f} (${f}): put on the table`);
      b.dataset.formula = f;
      b.addEventListener('click', () => { putOn(left, f); guideDid('tap_bottle', f); });
      if (shelf) storyLabel(b, shelf.have[f]);
      box.append(b);
    }
    $('bottleNote').textContent = shelf ? storyNote(shelf) : `${list.length} substances for ${BAND_NAME[/** @type {keyof typeof BAND_NAME} */ (band)]}. Tap one to put it on “in”.`;
    $('elementBox').hidden = !!story;
    const els = new Set(['H', 'C', 'N', 'O']); for (const f of list) for (const e of atomsOf(f)) els.add(e);
    const jars = $('jars'); jars.innerHTML = '';
    for (const k of Object.keys(EL).filter((e) => els.has(e))) {
      const e = EL[k]; const b = document.createElement('button'); b.type = 'button'; b.className = 'jar';
      b.innerHTML = `<i class="ball" style="background:${hex(e.c)};--glow:${hex(e.c)}66"></i><b>${k}</b><span>${e.name}</span>`;
      b.setAttribute('aria-label', `${e.name} (${k})`);
      b.addEventListener('click', () => addAtom(k));
      jars.append(b);
    }
    $('typeBox').hidden = rank(band) < rank('apprentice');
    document.body.dataset.band = band;
    for (const b of /** @type {NodeListOf<HTMLElement>} */ (document.querySelectorAll('.cond'))) {
      const k = b.dataset.cond || '';
      b.hidden = isSprout() && SPROUT_HIDES.has(k);
      if (b.hidden && conds.has(k)) { conds.delete(k); b.setAttribute('aria-pressed', 'false'); }
      b.textContent = (isYoung() && YOUNG_COND[k]) || (COND.find((c) => c[0] === k) || [k, k])[1];
    }
    $('scaleBox').hidden = rank(band) < rank('seedling');
  };
  const condBox = $('conds');
  for (const [k, label] of COND) {
    const b = document.createElement('button'); b.type = 'button'; b.className = 'cond'; b.textContent = label; b.setAttribute('aria-pressed', 'false'); b.dataset.cond = k;
    b.addEventListener('click', () => { if (conds.has(k)) conds.delete(k); else conds.add(k); b.setAttribute('aria-pressed', String(conds.has(k))); if (conds.has(k)) guideDid('condition', k); });
    condBox.append(b);
  }
  const setBand = (/** @type {string} */ b, user = false) => {
    band = b;
    if (!story) store.set('band', b);
    else if (user) story.__bench.setBand(b);
    document.querySelectorAll('.band button').forEach((x) => x.setAttribute('aria-pressed', String(/** @type {HTMLElement} */ (x).dataset.band === b)));
    stopWalk(); drawShelves(); drawCodex(); drawTable(); drawNotes(); autoWalk();
    spells?.reset();
  };
  document.querySelectorAll('.band button').forEach((x) => x.addEventListener('click', () => setBand(/** @type {string} */ (/** @type {HTMLElement} */ (x).dataset.band), true)));
  $('typeForm').addEventListener('submit', (ev) => {
    ev.preventDefault(); const v = /** @type {HTMLInputElement} */ ($('typeIn')).value.trim(); if (!v) return;
    guideDid('type', v);
    if (/->|→|=/.test(v)) { runChamber(v.replace('→', '->')); return; }
    const c = parseFormula(v);
    if (!c) { setStatus('Use element symbols and numbers, like H2O or Ca(OH)2.'); return; }
    putOn(left, written[hill(c)] || v.replace(/\s/g, ''));
  });

  // ---------- tabs, codex, notebook ----------
  const showTab = (/** @type {string} */ name) => {
    document.querySelectorAll('.tabs button').forEach((x) => x.setAttribute('aria-selected', String(/** @type {HTMLElement} */ (x).dataset.tab === name)));
    document.querySelectorAll('.tabpane').forEach((p) => { /** @type {HTMLElement} */ (p).hidden = /** @type {HTMLElement} */ (p).dataset.pane !== name; });
    document.body.dataset.tab = name;
    if (name === 'spells') spells?.draw();
  };
  document.querySelectorAll('.tabs button').forEach((t) => t.addEventListener('click', () => {
    const name = /** @type {HTMLElement} */ (t).dataset.tab || 'bench';
    showTab(name); guideDid('open_tab', name);
  }));
  /** @type {Record<string, string>} */
  const FAMILY = names.families;
  const describe = (/** @type {any} */ r) => (rank(band) <= rank('seedling') && r.short ? r.short : r.say);
  // Lab records use Hill formulas (NH3 is H3N); show the everyday way of
  // writing a formula when we know one, or the way the player wrote it.
  /** @type {Map<string, string>} */
  const usual = new Map();
  const learnUsual = (/** @type {string} */ f) => { const a = parseFormula(f); if (a) { const h = hill(a); if (!usual.has(h)) usual.set(h, f); } };
  for (const f of Object.keys(names.substances)) learnUsual(f);
  const plain = (/** @type {string} */ eq) => eq.split(' -> ').map((side) => side.split(' + ').map((t) => { const m = t.match(/^(\d*)(.+)$/); return m ? m[1] + (usual.get(m[2] ?? '') ?? m[2]) : t; }).join(' + ')).join(' -> ');
  // ---------- discoveries become spells (I-053) ----------
  // A found reaction can be saved as a spell for the game (formula-save.js,
  // shared with the world). Free play keeps unlimited bottles; the world's
  // story mode needs the bottles the player has found.
  // I-056: every spell does something in the world (`use`); a reaction
  // with nothing to do out there is Bench only and says why.
  /** @type {Record<string, {kind: string, runs: string, use?: string, bench_only?: string | null}>} */
  const EFFECTS = JSON.parse(effects_json());
  const KIND_WORD = { fire: 'a fire spell', water: 'a splash spell', frost: 'a cold spell', foam: 'a fizz spell', cloud: 'a cloud spell', glow: 'a warm glow spell', wall: 'a wall spell', reveal: 'a reveal spell' };
  /** @type {Record<string, {id: string, name: string, text: string}>} */
  const RULES = Object.fromEntries((codex.rules || []).map((/** @type {any} */ x) => [x.id, x]));
  /** "Predicted by rule: <the rule in plain words>" for a rule-tier entry, else "". */
  const ruleHtml = (/** @type {any} */ r) => {
    if (r.tier !== 'rule') return '';
    const x = RULES[r.rule];
    return `<div class="rule">Predicted by rule${x ? ` (${esc(x.name)}): ${esc(x.text)}` : '.'}</div>`;
  };
  const saveSpellButton = (/** @type {any} */ r) => {
    const b = document.createElement('button'); b.type = 'button'; b.className = 'btn save-spell';
    const fx = EFFECTS[r.id];
    if (fx?.bench_only) {
      b.disabled = true; b.textContent = `Bench only: ${fx.bench_only}`; b.title = fx.bench_only;
      return b;
    }
    const kind = fx ? KIND_WORD[/** @type {keyof typeof KIND_WORD} */ (fx.kind)] || '' : '';
    const done = () => { b.disabled = true; b.textContent = 'Saved! Practise it in the Spells tab, then put it on a key.'; };
    if (r.id in readSpells(localStorage).saved) done();
    else {
      b.textContent = kind ? `✨ Save as a spell (${kind})` : '✨ Save as a spell';
      if (fx?.use) b.textContent += `: it ${fx.use}`;
      b.addEventListener('click', () => { if (saveSpell(localStorage, r.id)) { done(); storySync(); } });
    }
    return b;
  };
  const drawCodex = () => {
    const seen = new Set(found);
    const vis = codex.reactions.filter(visible);
    const n = vis.filter((r) => seen.has(r.id)).length;
    $('codexCount').textContent = `${n}/${vis.length}`;
    $('codexNote').textContent = `${n} of ${vis.length} found at ${BAND_NAME[/** @type {keyof typeof BAND_NAME} */ (band)]}. Real chemistry has far more. Anything else you find is checked against published lab records, or becomes a mystery card.`;
    let html = '';
    for (const fam of codex.families) {
      const rs = vis.filter((r) => r.family === fam); if (!rs.length) continue;
      const got = rs.filter((r) => seen.has(r.id)).length;
      html += `<details class="fam"><summary><b>${esc(FAMILY[fam] || fam)}</b> <span>${got} of ${rs.length}</span></summary>`;
      for (const r of rs) {
        html += seen.has(r.id)
          ? `<div class="entry found" data-id="${esc(r.id)}"><b>${esc(r.name)}</b><div class="eq">${eqFor(r.equation)}</div><div>${esc(describe(r))}</div>${ruleHtml(r)}${isYoung() && guide?.where_you_see_it[r.id] ? `<div class="where">👀 ${esc(guide.where_you_see_it[r.id])}</div>` : ''}</div>`
          : `<div class="entry"><b>?</b><div>${r.needs.length ? 'Needs ' + r.needs.map(condName).join(' and ') + '.' : 'Needs nothing special.'}</div></div>`;
      }
      html += '</details>';
    }
    if (mysteries.length) html += `<details class="fam" open><summary><b>Mystery cards</b> <span>${mysteries.length}</span></summary>${mysteries.map((m) => `<div class="entry mystery"><div class="eq">${eqFor(m)}</div><div>Predicted by the model, not confirmed.</div></div>`).join('')}</details>`;
    if (labFinds.length) html += `<details class="fam" open><summary><b>Lab records</b> <span>${labFinds.length}</span></summary>${labFinds.map((m) => `<div class="entry found"><div class="eq">${eqFor(plain(m))}</div><div>Recorded by real chemists.</div></div>`).join('')}</details>`;
    html += `<p class="note">${LAB_CREDIT}</p>`;
    $('codex').innerHTML = html;
    for (const el of $('codex').querySelectorAll('.entry.found[data-id]')) { const r = byId[/** @type {HTMLElement} */ (el).dataset.id || '']; if (r) el.append(saveSpellButton(r)); }
    drawGuide();
  };
  const drawNotes = () => {
    $('notes').innerHTML = notes.length ? notes.map((x) => `<div class="entry${x.ok ? ' found' : ''}"><div class="eq">${eqFor(x.eq)}</div><div>${esc(x.what)}</div></div>`).join('') : '<p class="note">Nothing yet.</p>';
  };
  const note = (/** @type {string} */ eq, /** @type {string} */ what, /** @type {boolean} */ ok) => { notes = [{ t: new Date().toISOString(), eq, what, ok }, ...notes].slice(0, 40); store.set('notes', notes); drawNotes(); };

  // ---------- Sprout's "Show me how" (I-059) ----------
  // Sprout sees balanced equations but never balances by hand: this counts
  // each kind of atom on both sides of the equation it just ran.
  const showHow = (/** @type {string} */ eq) => {
    if (!isSprout()) return null;
    const [l = {}, r = {}] = eq.split(' -> ').map((s) => totals(sideOf(s)));
    const b = document.createElement('button'); b.type = 'button'; b.className = 'btn'; b.textContent = 'Show me how';
    b.addEventListener('click', () => {
      const rows = Object.keys(l).map((e) => `<li>${esc(EL[e]?.name ?? e)} (${e}): ${l[e]} in, ${r[e] || 0} out</li>`).join('');
      const box = document.createElement('div'); box.className = 'how';
      box.innerHTML = `<p><b>How it balances.</b> Count each kind of atom on both sides. The big numbers in front make the counts match:</p><ul>${rows}</ul>`;
      b.replaceWith(box);
    });
    return b;
  };

  // ---------- running the chamber ----------
  /** A "read it to me" button for Sprout and Seedling (plays only when tapped), or null. */
  const sayButton = (/** @type {string} */ text) => {
    if (!isYoung() || !('speechSynthesis' in window)) return null;
    const say = document.createElement('button'); say.type = 'button'; say.className = 'btn say'; say.textContent = '🔊 Read it to me';
    say.addEventListener('click', () => { speechSynthesis.cancel(); const u = new SpeechSynthesisUtterance(text); u.rate = 0.9; speechSynthesis.speak(u); });
    return say;
  };
  const result = (/** @type {string} */ cls, /** @type {string} */ html) => {
    const r = $('result'); r.hidden = false; r.className = 'result ' + cls; r.innerHTML = html;
    // Read-aloud for young players: plays only when tapped (voiceovers stay off by default).
    const say = sayButton(r.innerText);
    if (say) r.append(say);
    r.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
  };
  /** @type {Record<string, string>} */
  const HEAT = names.heat;
  // ---------- what you would really see (reaction show records, I-054) ----------
  // Each codex reaction's show record (crates/chem/src/show.rs) says what a
  // real bench shows: states, sourced colours, the visible changes, ΔH and
  // speed. The card says it in the band's words: plain words for Sprout and
  // Seedling, states from Apprentice, ΔH at Adept.
  /** @type {Record<string, any>} */
  const SHOWS = {};
  const showOf = (/** @type {string} */ id) => (SHOWS[id] ??= JSON.parse(show_json(id)));
  const listWords = (/** @type {string[]} */ xs) => (xs.length <= 1 ? xs.join('') : `${xs.slice(0, -1).join(', ')} and ${xs[xs.length - 1]}`);
  const STATE_WORD = { s: 'solid', l: 'liquid', g: 'gas', aq: 'dissolved' };
  /** A substance's everyday name without its bracketed second name ("carbon (charcoal)" is carbon). */
  const bare = (/** @type {string} */ f) => nameOf(f).replace(/ \(.*\)$/, '');
  /** "a" or "an" before a word. */
  const an = (/** @type {string} */ w) => `${/^[aeiou]/i.test(w) ? 'an' : 'a'} ${w}`;
  /** A gas's name: names.json names some substances by their solution (HCl is hydrochloric acid); as a gas it is hydrogen chloride. */
  const GAS_NAME = { HCl: 'hydrogen chloride' };
  const gasName = (/** @type {string} */ f) => GAS_NAME[/** @type {keyof typeof GAS_NAME} */ (f)] || nameOf(f);
  /** A glow's colour word from its temperature (no temperature: an externally heated solid). */
  const glowWord = (/** @type {any} */ t) => (!t ? 'a glow from the hot solid' : t.k < 1200 ? 'a red-hot glow' : t.k < 1800 ? 'an orange-hot glow' : 'a white-hot glow');
  /** The visible changes of a show record, as short phrases. Sprout gets the two plainest. */
  const seeParts = (/** @type {any} */ s, young = false) => {
    /** @type {string[]} */
    const parts = [];
    const add = (/** @type {string} */ x) => { if (!parts.includes(x)) parts.push(x); };
    // A colour as an adjective ("blue copper sulfate"); none when colourless, or when it repeats the name ("shiny silver" silver).
    const tone = (/** @type {any} */ c, /** @type {string} */ f = '') => (c && !c.described.startsWith('colourless') && !(f && c.described.includes(bare(f))) ? `${c.described} ` : '');
    for (const v of s.visible) {
      if (v === 'flame') add(s.flame && s.flame.emitter !== 'unknown' ? `${an(s.flame.described)} flame` : 'a flame');
      else if (v === 'glow') add(glowWord(s.glow?.temp));
      else if (v === 'bubbles') { const g = s.gases.filter((/** @type {any} */ x) => x.side === 'product').map((/** @type {any} */ x) => gasName(x.formula).replace(/ \(.*\)$/, '')); add(g.length ? `bubbles of ${listWords(g)}` : 'bubbles'); }
      else if (v === 'precipitate') for (const x of s.precipitates) add(`${an(`${tone(x.colour)}solid`)} (${bare(x.formula)}) ${x.grain === 'grainy' ? 'settling to the bottom' : 'clouding the liquid'}`);
      else if (v === 'coating') for (const x of s.coatings) add(`${tone(x.colour, x.formula)}${bare(x.formula)} coating the ${x.on === 'electrode' ? 'electrode' : bare(x.on)}`);
      else if (v === 'colour_change') {
        const gone = s.colour_changes.filter((/** @type {any} */ c) => !c.appears);
        const comes = s.colour_changes.filter((/** @type {any} */ c) => c.appears);
        if (young) {
          const c = gone[0] || comes[0];
          if (c) add(`the ${c.colour.described} colour ${c.appears ? 'appearing' : 'fading'}`);
        } else if (gone.length || comes.length) add(`a colour change: ${listWords([...gone.map((/** @type {any} */ c) => `${tone(c.colour, c.formula)}${bare(c.formula)} going`), ...comes.map((/** @type {any} */ c) => `${tone(c.colour, c.formula)}${bare(c.formula)} appearing`)])}`);
      } else if (v === 'smoke') for (const x of s.smoke) add(x.kind === 'vapour' ? `${tone(x.colour)}${bare(x.formula)} vapour` : `${tone(x.colour)}${x.kind}`);
      else if (v === 'frost') add('frost');
      else if (v === 'shimmer') add('the hot air shimmering above it');
      else if (v === 'condensation') add('droplets forming on the cool glass');
    }
    return young ? parts.slice(0, 2) : parts;
  };
  const DRIFT = { rises: 'so it rises', sinks: 'so it sinks', mixes: 'close enough that it mixes with the air instead of rising or sinking' };
  /** The "what you would see" lines of the result card, for this band. */
  const seeHtml = (/** @type {any} */ s) => {
    if (!s) return '';
    const parts = seeParts(s, isSprout());
    // Phrases that hold their own list are kept apart with semicolons.
    const seen = parts.some((x) => /, | and /.test(x)) ? parts.join('; ') : listWords(parts);
    let h = parts.length ? `<p class="see"><b>What you’d see:</b> ${esc(seen)}.</p>` : '';
    if (s.note) h += `<p class="see">${esc(s.note)}</p>`;
    if (s.sped_up) h += `<p class="see">For real this takes ${esc(s.timescale)}, so the Bench shows it sped up.</p>`;
    if (rank(band) >= rank('apprentice')) {
      const st = s.species.map((/** @type {any} */ x) => `${x.coef > 1 ? x.coef : ''}${subs(x.formula)}(${x.state})`);
      const nr = s.species.filter((/** @type {any} */ x) => x.side === 'reactant').length;
      h += `<p class="see eq">${st.slice(0, nr).join(' + ')} → ${st.slice(nr).join(' + ')}</p>`;
      h += `<p class="see">States: ${esc(listWords([...new Set(s.species.map((/** @type {any} */ x) => `${x.state} = ${STATE_WORD[/** @type {keyof typeof STATE_WORD} */ (x.state)]}`))]))}.</p>`;
    }
    if (rank(band) >= rank('adept')) {
      if (s.dh.kj !== null) h += `<p class="see">ΔH = ${s.dh.kj > 0 ? '+' : '−'}${Math.abs(s.dh.kj).toFixed(1)} kJ per mole of reaction as written${s.dh.dh_estimated ? ' (estimated)' : ''}, from standard enthalpies of formation.</p>`;
      const t = s.flame?.temp || s.glow?.temp;
      if (t) h += `<p class="see">Up to about ${Math.round(t.k / 100) * 100} K: an estimated upper bound (no heat lost).</p>`;
      if (s.flame?.typical_k) h += `<p class="see">Its glowing soot is nearer ${Math.round(s.flame.typical_k / 100) * 100} K, which gives the yellow.</p>`;
      for (const g of s.gases.filter((/** @type {any} */ x) => x.side === 'product')) h += `<p class="see">${esc(gasName(g.formula).replace(/ \(.*\)$/, ''))}: ${g.density_rel_air.toFixed(2)} times as dense as air, ${DRIFT[/** @type {keyof typeof DRIFT} */ (g.drift)]}.</p>`;
    }
    return h;
  };
  const fxFor = (/** @type {any} */ r, /** @type {Record<string, number>} */ out) => (names.effects[r.family] === 'flame' ? 'flame' : Object.keys(out).some((f) => GASES.has(f)) ? 'bubbles' : names.effects[r.family] || 'glow');
  // Lab records: published reactions (Open Reaction Database), fetched the
  // first time a try isn't in the codex. Keys come from the engine.
  /** @type {Promise<{byKey: Record<string, [number, string]>, byLeft: Map<string, [string, number][]>} | null> | null} */
  let labLoad = null;
  const loadLab = () => (labLoad ??= fetch('chem-literature.json').then((r) => (r.ok ? r.json() : null)).then((doc) => {
    if (!doc) return null;
    /** @type {Record<string, [number, string]>} */
    const byKey = doc.reactions;
    /** @type {Map<string, [string, number][]>} */
    const byLeft = new Map();
    for (const [k, [n]] of Object.entries(byKey)) {
      const l = k.split(' -> ')[0] ?? k;
      const list = byLeft.get(l) ?? [];
      list.push([k, n]); byLeft.set(l, list);
    }
    for (const list of byLeft.values()) list.sort((a, b) => b[1] - a[1]);
    return { byKey, byLeft };
  }).catch(() => null));
  /** The lab record for `input`: [equation, times recorded], or null. */
  const labRecord = async (/** @type {string} */ input) => {
    const lab = await loadLab(); if (!lab) return null;
    for (const t of input.split(/->|\+/)) learnUsual(t.trim().replace(/^\d+/, ''));
    const k = JSON.parse(literature_key_json(input));
    if (k.key) { const hit = lab.byKey[k.key]; return hit ? [k.key, hit[0]] : null; }
    const first = k.left ? lab.byLeft.get(k.left)?.[0] : undefined;
    return first ?? null;
  };
  /** @type {any} */
  let lastRaw = null;
  const runChamber = async (/** @type {string} */ input) => {
    await judgeAndShow(input);
    afterRun(input, lastRaw);
  };
  const judgeAndShow = async (/** @type {string} */ input) => {
    const raw = JSON.parse(judge_json(input, [...conds].join(','), band));
    lastRaw = raw;
    const [lt, rt] = input.split('->').map((s) => s.trim());
    const inSide = sideOf(lt); const guess = rt ? sideOf(rt) : null;
    if (raw.kind === 'parse_error') { result('bad', `<p>${esc(raw.message)}</p>`); return; }
    const missing = storyMissing(inSide);
    if (missing) { result('bad', missing); note(lt, 'Not in your pack yet.', false); animate(inSide, inSide, 'fizzle'); return; }
    if (raw.kind === 'known' || raw.kind === 'needs_condition' || raw.kind === 'locked') {
      const r = byId[raw.id]; const [el, er] = r.equation.split(' -> ').map(sideOf);
      const f0 = Object.keys(el)[0]; const k = (inSide[f0] || el[f0]) / el[f0];
      /** @type {Record<string, number>} */
      const out = {}; for (const [f, n] of Object.entries(er)) out[f] = Number.isInteger(n * k) ? n * k : n;
      const eqText = `${lt} -> ${sideText(out)}`;
      if (raw.kind === 'locked') {
        result('info', `<p><b>A real reaction!</b> You’ll learn this one at ${BAND_NAME[/** @type {keyof typeof BAND_NAME} */ (raw.band)]}.</p>`);
        note(lt, 'A real reaction for an older band.', false); animate(inSide, inSide, 'fizzle'); return;
      }
      if (raw.kind === 'needs_condition') {
        result('bad', `<p><b>Real reaction, but nothing happens yet.</b> It needs ${raw.missing.map(condName).join(' and ')}.</p><p>Turn that on under Conditions and run it again.</p>`);
        note(lt, `Needs ${raw.missing.map(condName).join(' and ')}.`, false); animate(inSide, inSide, 'fizzle'); return;
      }
      const isNew = !found.includes(r.id);
      if (rt && !balanced.includes(r.id)) { balanced = [...balanced, r.id]; store.set('balanced', balanced); }
      if (isNew) { found = [...found, r.id]; store.set('found', found); }
      drawCodex();
      const show = showOf(r.id);
      result('good', `<p><b>${esc(r.name)}!</b> ${esc(describe(r))}</p><p class="eq">${eqFor(eqText)}</p>${ruleHtml(r)}${seeHtml(show)}${show?.feel ? `<p>${esc(show.feel)}</p>` : r.heat ? `<p>${HEAT[r.heat]}</p>` : ''}${r.hazard ? `<p class="hazard">${hazardText(r.hazard)}</p>` : ''}<p>${isNew ? 'New codex entry!' : 'Already in your codex.'}</p>`);
      $('result').append(saveSpellButton(r));
      const how = showHow(eqText); if (how) $('result').append(how);
      note(eqText, r.name, true); animate(inSide, out, fxFor(r, out), show); return;
    }
    if (raw.kind === 'unbalanced') {
      const near = raw.close ? `<p>You’re close: these really do react. ${rank(band) <= rank('seedling') ? 'Try changing how many of each you use.' : 'Change the numbers in front.'}</p>` : '<p>Atoms are never made or destroyed, so both sides need the same atoms.</p>';
      result('bad', `<p><b>The scale isn’t level.</b> ${raw.report.map(esc).join(' ')}</p>${near}`);
      const closeR = raw.close ? byId[raw.close] : null;
      if (closeR && isYoung()) {
        const show1 = document.createElement('button'); show1.type = 'button'; show1.className = 'btn'; show1.textContent = 'Show me how';
        show1.addEventListener('click', () => {
          const [l, rr] = closeR.equation.split(' -> ').map(sideOf);
          for (const s of [left, right]) for (const k of Object.keys(s)) delete s[k];
          Object.assign(left, l); Object.assign(right, rr); drawTable();
          result('info', '<p><b>Here’s the level way.</b> Count each colour on both sides of the scale: they match.</p><p>Now run the chamber.</p>');
        });
        $('result').prepend(show1);
      }
      note(input, 'Not balanced.', false); animate(inSide, inSide, 'fizzle'); return;
    }
    if (raw.kind === 'mystery' || raw.kind === 'no_reaction') {
      const rec = await labRecord(input);
      if (rec) {
        const [key, times] = rec; const eq = plain(key); const [, outText] = eq.split(' -> ');
        const isNew = !labFinds.includes(key);
        if (isNew) { labFinds = [...labFinds, key]; store.set('lab', labFinds); drawCodex(); }
        const surprise = raw.kind === 'no_reaction' && guess ? '<p>Our simple model didn’t expect this one, but the lab record says it happened.</p>' : '';
        result('lab', `<p><b>Lab record!</b> It’s not in the codex, but real chemists have recorded ${guess ? 'this reaction' : 'these turning into something'} ${times === 1 ? 'once' : times + ' times'}:</p><p class="eq">${eqFor(eq)}</p>${surprise}<p class="credit">Formulas only, so it could be a different molecule with the same atoms. ${LAB_CREDIT}</p><p>${isNew ? 'New lab record in your codex!' : 'Already in your codex.'}</p>`);
        note(eq, 'Lab record (published reaction).', true); animate(inSide, sideOf(outText ?? ''), 'glow'); return;
      }
    }
    if (raw.kind === 'mystery' && guess) {
      if (!mysteries.includes(input)) { mysteries = [...mysteries, input]; store.set('mysteries', mysteries); drawCodex(); }
      if (isYoung()) { result('mystery', '<p><b>Mystery card!</b> Nobody has tried this on our bench yet. It might work, but no one has checked.</p>'); note(input, 'Mystery card (predicted, not confirmed).', true); animate(inSide, guess, 'glow'); return; }
      result('mystery', `<p><b>Mystery card!</b> This isn’t in our reaction data. The model says it might hold together:</p><ul>${raw.checks.map((/** @type {string} */ c) => `<li>${esc(c)}</li>`).join('')}</ul><p>Predicted by the model, not confirmed. Real chemists would test it in a lab.</p>`);
      note(input, 'Mystery card (predicted, not confirmed).', true); animate(inSide, guess, 'glow'); return;
    }
    result('bad', `<p><b>Fizzle.</b> ${esc(raw.reason || 'Nothing happens.')}</p>${guess ? '' : isSprout() ? '<p>Try other bottles, or another switch.</p>' : '<p>Try other bottles, another condition, or write your guess in “out”.</p>'}`);
    note(input, raw.reason || 'Nothing happened.', false); animate(inSide, inSide, 'fizzle');
  };
  $('test').addEventListener('click', () => {
    guideDid('run', true);
    if (!Object.keys(left).length) { result('bad', '<p>Put at least one substance on “in” first.</p>'); return; }
    const input = Object.keys(right).length ? `${sideText(left)} -> ${sideText(right)}` : sideText(left);
    runChamber(input);
  });
  const emptyChamber = () => { scene3d.emptyChamber(); chamberMols = 0; previewKey = ''; ran = false; drawChamberTag(); };
  // After a run this puts back what is on "in", ready to run again.
  $('emptyChamber').addEventListener('click', () => { emptyChamber(); $('result').hidden = true; previewIn(); });
  $('clearTable').addEventListener('click', () => { for (const s of [left, right]) for (const k of Object.keys(s)) delete s[k]; emptyChamber(); $('result').hidden = true; drawTable(); });

  // The chamber animation is drawn by bench-scene.js; here each side becomes
  // up to 12 molecules with their drawn bonds.
  const mols = (/** @type {Record<string, number>} */ side) => Object.entries(side).flatMap(([f, n]) => Array(Math.min(n, 6)).fill(f)).slice(0, 12).map((f) => { const els = atomsOf(f); return { els, bonds: structure(els).bonds }; });
  // The show record goes to the scene as an optional 4th argument (the
  // graphics thread draws from it; today's scene ignores it).
  const animate = (/** @type {Record<string, number>} */ inSide, /** @type {Record<string, number>} */ outSide, /** @type {any} */ fx, /** @type {any} */ show = undefined) => {
    const a = mols(inSide); /** @type {any} */ (scene3d).playReaction(a, mols(outSide), fx, show); chamberMols = a.length; previewKey = sideText(left); ran = true; drawChamberTag();
  };

  // What is on "in" always shows in the 3D chamber: one molecule per count
  // (up to 6 of each and 12 in all, as in a run), standing still until Run
  // plays the reaction. Clear the table (or taking everything off) empties
  // it. A small tag on the view names what is there, in the band's words.
  let chamberMols = 0;
  let previewKey = '';
  /** The chamber shows a run's result, not "in": the tag waits for the next change. */
  let ran = false;
  const drawChamberTag = () => {
    const tag = $('chamberTag'); const keys = Object.keys(left);
    tag.hidden = ran || !keys.length;
    if (tag.hidden) { tag.textContent = ''; return; }
    const what = keys.map((f) => `${(left[f] ?? 0) > 1 ? left[f] : ''}${show(f)}`).join(' + ');
    const total = Object.values(left).reduce((n, k) => n + k, 0);
    tag.innerHTML = `In the chamber: <b>${what}</b>${total > chamberMols ? ` <small>(showing ${chamberMols})</small>` : ''}`;
  };
  const previewIn = () => {
    const key = sideText(left);
    if (key === previewKey) { drawChamberTag(); return; }
    previewKey = key; ran = false;
    if (!Object.keys(left).length) { scene3d.emptyChamber(); chamberMols = 0; }
    else { const a = mols(left); scene3d.playReaction(a, a, 'fizzle'); chamberMols = a.length; }
    drawChamberTag();
  };

  /** @type {AudioContext | null} */
  let ac = null;
  const tone = (/** @type {string} */ el) => { try { ac = ac || new AudioContext(); const o = ac.createOscillator(), g = ac.createGain(); o.frequency.value = 300 + (Object.keys(EL).indexOf(el) % 12) * 60; g.gain.setValueAtTime(0.05, ac.currentTime); g.gain.exponentialRampToValueAtTime(0.0001, ac.currentTime + 0.18); o.connect(g).connect(ac.destination); o.start(); o.stop(ac.currentTime + 0.2); } catch { /* no sound */ } };

  // ---------- the Guide (content/chem/guide.json, shipped as chem-guide.json) ----------
  // A first-visit walkthrough per band, lessons with "Try it", a hint ladder
  // after a failed run, challenge cards and an everyday line per codex card.
  /** @type {any} */
  let guide = null;
  const G = () => guide?.bands?.[band];
  /** @type {{band: string, i: number, pending: boolean} | null} */
  let walk = null;
  const TAB_OF = /** @type {Record<string, string>} */ ({ codex: 'codex', notebook: 'notes', guide: 'guide' });
  const squash = (/** @type {string} */ s) => s.replace(/\s+/g, '').replace('→', '->');
  /** Formulas in guide prose shown with subscripts and arrows, at every band (I-059). */
  const prose = (/** @type {string} */ s) => {
    const h = esc(s).replace(/&lt;=&gt;/g, '⇌').replace(/-&gt;/g, '→');
    return h.split(' ').map((w) => {
      const m = w.match(/^(\d*)([A-Za-z0-9()]+)([.,;:]?)$/);
      return m && /\d/.test(m[2] ?? '') && /[A-Z]/.test(m[2] ?? '') && parseFormula(m[2] ?? '') ? `${m[1]}${subs(m[2] ?? '')}${m[3]}` : w;
    }).join(' ');
  };
  const stepAction = (/** @type {any} */ step) => /** @type {[string, any]} */ (Object.entries(step.do)[0]);
  const isRunStep = (/** @type {any} */ step) => { const [k, v] = stepAction(step); return k === 'run' || (k === 'type' && /->/.test(String(v))); };
  const targetOf = (/** @type {any} */ step) => {
    const [k, v] = stepAction(step);
    if (k === 'tap_bottle') return [...document.querySelectorAll('.bottle')].find((b) => /** @type {HTMLElement} */ (b).dataset.formula === v) || null;
    if (k === 'condition') return document.querySelector(`.cond[data-cond="${v}"]`);
    if (k === 'run') return $('test');
    if (k === 'type') return $('typeIn');
    if (k === 'open_tab') return document.querySelector(`.tabs button[data-tab="${TAB_OF[v] || v}"]`);
    return null;
  };
  const unmark = () => document.querySelectorAll('.guide-target').forEach((x) => x.classList.remove('guide-target'));
  const coach = (/** @type {string} */ step, /** @type {string} */ html, /** @type {HTMLElement[]} */ buttons) => {
    const box = $('coach'); box.hidden = false;
    $('coachStep').textContent = step;
    $('coachSay').innerHTML = html;
    const row = $('coachRow'); row.innerHTML = '';
    const say = sayButton($('coachSay').innerText); if (say) row.append(say);
    row.append(...buttons);
  };
  const button = (/** @type {string} */ label, /** @type {() => void} */ fn) => { const b = document.createElement('button'); b.type = 'button'; b.className = 'btn'; b.textContent = label; b.addEventListener('click', fn); return b; };
  const showStep = () => {
    const g = G(); if (!walk || !g) return;
    const step = g.walkthrough[walk.i];
    unmark();
    const [k] = stepAction(step);
    if (k === 'condition' || k === 'run') showTab('bench');
    // A switch the player already turned on counts: no need to turn it off and on again.
    if (k === 'condition' && conds.has(stepAction(step)[1])) { walk.i++; if (walk.i < g.walkthrough.length) showStep(); return; }
    const t = targetOf(step);
    if (t) { t.classList.add('guide-target'); t.scrollIntoView({ block: 'nearest', behavior: 'smooth' }); }
    coach(`Step ${walk.i + 1} of ${g.walkthrough.length}`, prose(step.say), []);
  };
  const clearBench = () => {
    for (const s of [left, right]) for (const k of Object.keys(s)) delete s[k];
    conds.clear();
    document.querySelectorAll('.cond').forEach((b) => b.setAttribute('aria-pressed', 'false'));
    emptyChamber(); $('result').hidden = true; drawTable();
  };
  const startWalk = () => {
    const g = G(); if (!g?.walkthrough?.length) return;
    clearBench(); showTab('bench');
    walk = { band, i: 0, pending: false };
    showStep();
  };
  const stopWalk = () => { walk = null; unmark(); $('coach').hidden = true; };
  const doneWalk = () => { store.set('guide.' + band, true); drawGuide(); };
  const autoWalk = () => { if (!story && guide && !store.get('guide.' + band, false)) startWalk(); };
  $('coachSkip').addEventListener('click', () => { doneWalk(); stopWalk(); });
  /** The player did something the walkthrough may be waiting for. */
  const guideDid = (/** @type {string} */ kind, /** @type {any} */ value) => {
    const g = G(); if (!walk || !g) return;
    const step = g.walkthrough[walk.i]; const [k, v] = stepAction(step);
    if (k !== kind) return;
    const hit = k === 'type' ? squash(String(v)) === squash(String(value)) : k === 'open_tab' ? (TAB_OF[v] || v) === value : k === 'run' ? true : v === value;
    if (!hit) return;
    if (isRunStep(step)) { walk.pending = true; return; }
    walk.i++; showStep();
  };
  /** After every run: the walkthrough's last step, and the hint ladder after a failure. */
  const afterRun = (/** @type {string} */ input, /** @type {any} */ raw) => {
    if (raw && $('result').classList.contains('bad') && raw.kind !== 'parse_error') offerHint(familyFor(raw, input));
    const g = G();
    if (!walk || !walk.pending || !g) return;
    walk.pending = false;
    if (raw?.kind === 'known') {
      walk.i++;
      if (walk.i < g.walkthrough.length) { showStep(); return; }
      doneWalk(); walk = null; unmark();
      const r = byId[raw.id];
      coach('', `<b>🎉 ${esc(r.name)}!</b> ${esc(GUIDE_DONE)}`, [button('Open the Guide', () => { $('coach').hidden = true; showTab('guide'); }), button('Close', () => { $('coach').hidden = true; })]);
      return;
    }
    coach(`Step ${walk.i + 1} of ${g.walkthrough.length}`, `${prose(g.walkthrough[walk.i].say)}<br><i>${esc(GUIDE_RETRY)}</i>`, [button('Start again', startWalk)]);
  };
  const GUIDE_DONE = 'It is in your Codex now. The Guide tab has lessons and challenges.';
  const GUIDE_RETRY = 'Not quite yet. Check the table and the switches, then try again.';

  // Hint ladder: nudge, then idea, then the answer, per family.
  /** @type {Map<string, number>} */
  const hintLevel = new Map();
  const familyFor = (/** @type {any} */ raw, /** @type {string} */ input) => {
    const known = byId[raw.id || raw.close || ''];
    if (known) return known.family;
    const mine = Object.keys(sideOf((input.split('->')[0] ?? '').trim()));
    let best = null, score = 0;
    for (const r of codex.reactions) {
      if (!visible(r)) continue;
      const theirs = Object.keys(sideOf(r.equation.split(' -> ')[0]));
      const n = mine.filter((f) => theirs.includes(f)).length;
      if (n > score) { best = r.family; score = n; }
    }
    return best;
  };
  const offerHint = (/** @type {string | null} */ fam) => {
    const lines = fam ? G()?.hints?.[fam] : null; if (!lines || !fam) return;
    const box = document.createElement('div'); box.className = 'hints';
    const list = document.createElement('ol');
    const more = button('💡 Hint', () => { hintLevel.set(fam, Math.min(lines.length, (hintLevel.get(fam) || 0) + 1)); draw(); });
    const draw = () => {
      const n = hintLevel.get(fam) || 0;
      list.innerHTML = lines.slice(0, n).map((/** @type {string} */ l) => `<li>${prose(l)}</li>`).join('');
      more.textContent = n ? '💡 Another hint' : '💡 Hint';
      more.hidden = n >= lines.length;
      box.querySelector('.say')?.remove();
      const say = n ? sayButton(list.innerText) : null; if (say) box.append(say);
    };
    box.append(list, more); $('result').append(box); draw();
  };

  // Lessons: "Try it" loads the reactants and conditions onto the table.
  const tryLesson = (/** @type {any} */ l) => {
    stopWalk(); clearBench();
    for (const f of l.try.reactants) left[f] = (left[f] || 0) + 1;
    for (const c of l.try.conditions) conds.add(c);
    document.querySelectorAll('.cond').forEach((b) => b.setAttribute('aria-pressed', String(conds.has(/** @type {HTMLElement} */ (b).dataset.cond))));
    drawTable(); showTab('bench');
    result('info', `<p><b>${esc(l.title)}</b></p><p>${esc(TRY_READY)}</p>`);
  };
  const TRY_READY = 'Everything is on the table. Run the chamber!';

  // Challenges: progress from the codex entries found (and equations balanced).
  const progress = (/** @type {any} */ goal) => {
    const got = new Set(found);
    const vis = codex.reactions.filter(visible);
    if (goal.kind === 'find_family') return [vis.some((r) => r.family === goal.family && got.has(r.id)) ? 1 : 0, 1];
    if (goal.kind === 'find_any_of') return [goal.ids.some((/** @type {string} */ id) => got.has(id)) ? 1 : 0, 1];
    if (goal.kind === 'find_all_of') return [goal.ids.filter((/** @type {string} */ id) => got.has(id)).length, goal.ids.length];
    if (goal.kind === 'find_count') return [Math.min(goal.count, vis.filter((r) => (!goal.family || r.family === goal.family) && (!goal.ids || goal.ids.includes(r.id)) && got.has(r.id)).length), goal.count];
    if (goal.kind === 'balance') return [balanced.includes(goal.reaction) ? 1 : 0, 1];
    return [0, 1];
  };
  const drawGuide = () => {
    const box = $('guide');
    const g = G();
    if (!g) { box.innerHTML = `<p class="note">${guide === false ? 'The guide could not load.' : 'Loading the guide…'}</p>`; return; }
    box.innerHTML = '';
    const start = button(store.get('guide.' + band, false) ? 'Do the walkthrough again' : 'Start the walkthrough', startWalk);
    start.classList.add('primary');
    const sec = (/** @type {string} */ title) => { const s = document.createElement('section'); s.innerHTML = `<h2>${title}</h2>`; box.append(s); return s; };
    if (story) sec('Walkthrough').append(Object.assign(document.createElement('p'), { className: 'note', textContent: 'The walkthrough needs every bottle, so it runs in free play: open the Chemistry Bench from the title screen.' }));
    else sec('Walkthrough').append(start);
    const ch = sec('Challenges');
    for (const c of g.challenges) {
      const [n, of] = progress(c.goal);
      const d = document.createElement('div'); d.className = 'challenge' + (n >= of ? ' done' : '');
      d.innerHTML = `<b>${n >= of ? '✓ ' : ''}${prose(c.title)}</b><div class="bar" role="progressbar" aria-valuemin="0" aria-valuemax="${of}" aria-valuenow="${n}" aria-label="${esc(c.title)}"><i style="width:${Math.round((100 * n) / of)}%"></i></div><small>${n} of ${of}</small>`;
      ch.append(d);
    }
    const ls = sec('Lessons');
    for (const l of g.lessons) {
      const d = document.createElement('div'); d.className = 'lesson';
      // I-057: a lesson's `world` points at the chemistry quest that uses it.
      const world = l.world && l.world.say ? `<p class="world">${prose(l.world.say)}</p>` : '';
      // I-059: an Adept lesson also names its AP Chemistry topic.
      const ap = l.ap ? ` <small class="ngss" title="AP Chemistry Course and Exam Description topic">AP Chemistry ${esc(l.ap)}</small>` : '';
      d.innerHTML = `<b>${prose(l.title)}</b><p>${prose(l.idea)}</p>${world}<small class="ngss" title="Next Generation Science Standards">NGSS ${esc(l.ngss)}</small>${ap}`;
      const row = document.createElement('div'); row.className = 'row';
      if (l.try) row.append(button('Try it', () => tryLesson(l)));
      const say = sayButton(`${l.title}. ${l.idea}${l.world && l.world.say ? ` ${l.world.say}` : ''}`); if (say) row.append(say);
      d.append(row); ls.append(d);
    }
  };
  fetch('chem-guide.json').then((r) => (r.ok ? r.json() : null)).then((doc) => { guide = doc || false; drawCodex(); autoWalk(); }).catch(() => { guide = false; drawGuide(); });

  // ---------- loop ----------
  const beam = $('beam'); let beamNow = 0; let last = performance.now();
  const frame = (/** @type {number} */ now) => {
    const dt = Math.min((now - last) / 1000, 0.05); last = now;
    beamNow += (tilt - beamNow) * Math.min(1, dt * 4); beam.setAttribute('transform', `rotate(${(-beamNow).toFixed(2)} 150 60)`);
    requestAnimationFrame(frame);
  };

  // ---------- the Spells tab (bench-spells.js, I-060) ----------
  spells = mountSpells({
    box: $('spellbook'),
    host: story,
    storage: localStorage,
    band: () => band,
    byId,
    effects: EFFECTS,
    judge: (input, have, b) => JSON.parse(judge_json(input, have, b)),
    atoms: (f) => { const c = parseFormula(f); return c ? /** @type {Array<[string, number]>} */ (Object.entries(c)) : null; },
    eq: eqFor,
    onSaved: storySync,
  });
  setInterval(() => {
    if (story && storyShelfKey() !== shelfKey) drawShelves();
    spells?.tick();
  }, 400);
  setupStory();

  setBand(band);
  drawTable(); drawNotes(); restructure();
  $('loading').hidden = true;
  /** @type {any} */ (window).__benchFree = { codex, run: runChamber, left, right, conds, found: () => found, setBand, walk: () => walk, guide: () => guide, sceneCount: () => chamberMols + atoms.length, chamberCount: () => chamberMols, plateCount: () => atoms.length, mode: story ? 'story' : 'free', spells: () => spells?.state(), showTab };
  if (story) story.__bench.ready();
  requestAnimationFrame(frame);
}

if (typeof document !== 'undefined' && document.getElementById('scene')) {
  main().catch((e) => {
    const l = document.getElementById('loading');
    if (l) l.textContent = 'The bench could not start: ' + (e && e.message ? e.message : e);
  });
}
