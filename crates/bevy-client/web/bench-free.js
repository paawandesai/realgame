// Chemistry Bench free play (Bench plan Phase 1): the approved mock-up's
// bench, now judged by the real chemistry engine (crates/chem, built to
// chem.js + chem_bg.wasm by build-web.sh). Every result comes from the
// engine: a known reaction, one that needs a condition, one taught at a
// higher band, an unbalanced try, a mystery card the model predicts, or no
// reaction. Three.js r128 is vendored in vendor/ (no CDN).
//
// Saved in this browser only: the band, the codex entries found, mystery
// cards and the notebook (keys start `formula.bench.`).

import init, { codex_json, judge_json, literature_key_json, names_json } from './chem.js';
import { parseFormula as parseWith, hill, sideOf, sideText } from './bench-free-lib.js';

// Element display data and everyday names, from content/chem/names.json.
/** @type {Record<string, {v:number,c:number,r:number,name:string}>} */
const EL = {};
/** @type {Record<string, string>} */
const NAMES = {};
const parseFormula = (/** @type {string} */ f) => parseWith(f, (el) => !!EL[el]);

/** @type {any} */
const THREE = /** @type {any} */ (window).THREE;

const BANDS = ['sprout', 'seedling', 'apprentice', 'adept'];
const BAND_NAME = { sprout: 'Sprout', seedling: 'Seedling', apprentice: 'Apprentice', adept: 'Adept' };

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
  let band = BANDS.includes(q.get('band') || '') ? /** @type {string} */ (q.get('band')) : store.get('band', 'apprentice');
  /** @type {string[]} */
  let found = store.get('found', []);
  /** @type {string[]} */
  let mysteries = store.get('mysteries', []);
  /** @type {string[]} */
  let labFinds = store.get('lab', []);
  /** @type {{t:string, eq:string, what:string, ok:boolean}[]} */
  let notes = store.get('notes', []);

  // ---------- 3D scene (from the mock-up) ----------
  const canvas = /** @type {HTMLCanvasElement} */ ($('scene'));
  const renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
  renderer.outputEncoding = THREE.sRGBEncoding;
  renderer.shadowMap.enabled = true;
  const scene = new THREE.Scene();
  scene.background = new THREE.Color(0x12161f);
  scene.fog = new THREE.Fog(0x12161f, 12, 26);
  const camera = new THREE.PerspectiveCamera(42, 1, 0.1, 100);
  camera.position.set(0.75, 4.4, 9.4);
  const controls = new THREE.OrbitControls(camera, canvas);
  controls.target.set(0.75, 0.9, 0); controls.enableDamping = true; controls.minDistance = 3.5; controls.maxDistance = 13; controls.maxPolarAngle = 1.45;
  scene.add(new THREE.HemisphereLight(0xbcd0ff, 0x3a2a18, 0.55));
  const lamp = new THREE.SpotLight(0xffd9a0, 1.6, 30, 0.6, 0.5); lamp.position.set(-2, 8, 4); lamp.castShadow = true; lamp.shadow.mapSize.set(1024, 1024); scene.add(lamp); lamp.target.position.set(0.5, 0, 0); scene.add(lamp.target);
  const rim = new THREE.DirectionalLight(0x7fa8ff, 0.5); rim.position.set(4, 3, -5); scene.add(rim);
  const wood = new THREE.MeshStandardMaterial({ color: 0x7a5232, roughness: 0.75 });
  const top = new THREE.Mesh(new THREE.BoxGeometry(8.4, 0.3, 3.6), wood); top.position.set(0.6, -0.15, 0); top.receiveShadow = true; scene.add(top);
  for (const [x, z] of [[-3.3, -1.5], [4.5, -1.5], [-3.3, 1.5], [4.5, 1.5]]) { const leg = new THREE.Mesh(new THREE.BoxGeometry(0.28, 3, 0.28), wood); leg.position.set(x, -1.8, z); scene.add(leg); }
  const back = new THREE.Mesh(new THREE.BoxGeometry(8.4, 2.6, 0.18), new THREE.MeshStandardMaterial({ color: 0x5a3c24, roughness: 0.8 })); back.position.set(0.6, 1.3, -1.9); scene.add(back);
  const plate = new THREE.Mesh(new THREE.CylinderGeometry(1.5, 1.6, 0.12, 48), new THREE.MeshStandardMaterial({ color: 0x2b3448, metalness: 0.6, roughness: 0.35 }));
  plate.position.set(-1.2, 0.06, 0.2); plate.receiveShadow = true; scene.add(plate);
  const ring = new THREE.Mesh(new THREE.TorusGeometry(1.52, 0.035, 8, 64), new THREE.MeshBasicMaterial({ color: 0xe8b04a })); ring.rotation.x = Math.PI / 2; ring.position.set(-1.2, 0.13, 0.2); scene.add(ring);
  const CH = new THREE.Vector3(2.7, 1.25, 0.1);
  const glass = new THREE.Mesh(new THREE.BoxGeometry(2.4, 2.4, 2.0), new THREE.MeshPhysicalMaterial({ color: 0x9fc3ff, transparent: true, opacity: 0.12, roughness: 0.05, metalness: 0, depthWrite: false }));
  glass.position.copy(CH); scene.add(glass);
  const edges = new THREE.LineSegments(new THREE.EdgesGeometry(new THREE.BoxGeometry(2.4, 2.4, 2.0)), new THREE.LineBasicMaterial({ color: 0x9fc3ff, transparent: true, opacity: 0.5 }));
  edges.position.copy(CH); scene.add(edges);
  const chamberLight = new THREE.PointLight(0xff8a3a, 0, 6); chamberLight.position.copy(CH); scene.add(chamberLight);
  const sphereGeo = new THREE.SphereGeometry(1, 28, 20);
  const bondGeo = new THREE.CylinderGeometry(0.07, 0.07, 1, 12);
  const bondMat = new THREE.MeshStandardMaterial({ color: 0xc9ced8, roughness: 0.4, metalness: 0.2 });
  /** @type {Record<string, any>} */
  const atomMat = {};
  const matFor = (/** @type {string} */ el) => atomMat[el] || (atomMat[el] = new THREE.MeshStandardMaterial({ color: EL[el].c, roughness: el === 'C' ? 0.5 : 0.3, metalness: 0.05 }));
  const atomMesh = (/** @type {string} */ el) => { const m = new THREE.Mesh(sphereGeo, matFor(el)); m.scale.setScalar(EL[el].r); m.castShadow = true; return m; };
  const UP = new THREE.Vector3(0, 1, 0);
  const drawBonds = (/** @type {any} */ group, /** @type {any[]} */ pos, /** @type {{a:number,b:number,o:number}[]} */ bonds) => {
    let k = 0;
    for (const b of bonds) {
      const a = pos[b.a], c = pos[b.b]; if (!a || !c) continue; const d = c.clone().sub(a); const L = d.length(); if (L < 1e-3) continue;
      const side = d.clone().cross(Math.abs(d.y) < 0.9 * L ? UP : new THREE.Vector3(1, 0, 0)).normalize().multiplyScalar(0.12);
      for (let o = 0; o < b.o; o++) {
        let rod = group.children[k++]; if (!rod) { rod = new THREE.Mesh(bondGeo, bondMat); group.add(rod); }
        rod.visible = true;
        rod.position.copy(a).add(c).multiplyScalar(0.5).add(side.clone().multiplyScalar(o - (b.o - 1) / 2));
        rod.scale.set(1, L, 1);
        rod.quaternion.setFromUnitVectors(UP, d.clone().normalize());
      }
    }
    for (; k < group.children.length; k++) group.children[k].visible = false;
  };

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
  const relax = (/** @type {any[]} */ pos, /** @type {string[]} */ els, /** @type {{a:number,b:number}[]} */ bonds, /** @type {number} */ steps) => {
    for (let s = 0; s < steps; s++) {
      const f = pos.map(() => new THREE.Vector3());
      for (let i = 0; i < pos.length; i++) for (let j = i + 1; j < pos.length; j++) {
        const d = pos[j].clone().sub(pos[i]); const L = Math.max(d.length(), 0.05);
        const push = d.multiplyScalar(-0.05 / (L * L * L)); f[i].add(push); f[j].sub(push);
      }
      for (const b of bonds) {
        const d = pos[b.b].clone().sub(pos[b.a]); const L = Math.max(d.length(), 0.01);
        const rest = (EL[els[b.a]].r + EL[els[b.b]].r) * 1.25;
        const k = d.multiplyScalar((L - rest) * 0.25 / L); f[b.a].add(k); f[b.b].sub(k);
      }
      const c = pos.reduce((m, p) => m.add(p), new THREE.Vector3()).multiplyScalar(1 / Math.max(pos.length, 1));
      pos.forEach((p, i) => { p.add(f[i].clampLength(0, 0.15)); p.x -= c.x * 0.02; p.z -= c.z * 0.02; });
    }
    return pos;
  };
  const layout = (/** @type {string[]} */ els) => {
    const { bonds } = structure(els);
    const pos = els.map((e, i) => new THREE.Vector3(Math.cos(i * 2.4) * 0.6, Math.sin(i * 1.7) * 0.4, Math.sin(i * 2.4) * 0.6));
    relax(pos, els, bonds, els.length > 30 ? 80 : 220);
    return { pos, bonds };
  };
  const atomsOf = (/** @type {string} */ f) => Object.entries(parseFormula(f) || {}).flatMap(([e, k]) => Array(k).fill(e));

  // ---------- the plate ----------
  const PLATE = new THREE.Vector3(-1.2, 1.05, 0.2);
  /** @type {{el:string, mesh:any, p:any}[]} */
  let atoms = [];
  const plateBonds = new THREE.Group(); scene.add(plateBonds);
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
  const restructure = () => { plateInfo = atoms.length ? structure(atoms.map((a) => a.el)) : { bonds: [], complete: false, left: [], parts: false }; readout(); };
  const addAtom = (/** @type {string} */ el) => {
    if (atoms.length >= 16) { setStatus('The plate holds up to 16 atoms. Use a bottle for bigger ones.'); return; }
    const mesh = atomMesh(el); scene.add(mesh);
    const p = PLATE.clone().add(new THREE.Vector3((Math.random() - 0.5) * 1.2, 0.9, (Math.random() - 0.5) * 1.2));
    mesh.position.copy(p); atoms.push({ el, mesh, p }); restructure(); tone(el);
  };
  const clearPlate = () => { for (const a of atoms) scene.remove(a.mesh); atoms = []; restructure(); };

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
      plus.addEventListener('click', () => { side[f] = Math.min(side[f] + 1, 12); drawTable(); });
      row.append(minus, plus); box.append(row);
    }
  };
  let tilt = 0;
  const drawTable = () => {
    drawSide($('leftSide'), left, 'In'); drawSide($('rightSide'), right, 'Out');
    const L = totals(left), R = totals(right); const els = [...new Set([...Object.keys(L), ...Object.keys(R)])];
    const guessing = Object.keys(right).length > 0;
    let diff = 0; let html = '<span class="h">Atom</span><span class="h">In</span><span class="h">Out</span><span class="h"></span>';
    for (const e of els) { const a = L[e] || 0, b = R[e] || 0; diff += (a - b); html += `<span>${e}</span><span>${a}</span><span>${b}</span><span class="${a === b ? 'ok' : 'no'}">${a === b ? '✓' : '✗'}</span>`; }
    $('tally').innerHTML = guessing && els.length ? html : '<span class="empty" style="grid-column:1/-1">Fill in “out” and the scale weighs the atoms on each side.</span>';
    tilt = guessing ? Math.max(-14, Math.min(14, diff * 1.5)) : 0;
  };
  const putOn = (/** @type {Record<string, number>} */ side, /** @type {string} */ f) => { side[f] = Math.min((side[f] || 0) + 1, 12); drawTable(); };
  $('toLeft').addEventListener('click', () => { if (plateOk()) { putOn(left, plateFormula()); clearPlate(); } });
  $('toRight').addEventListener('click', () => { if (plateOk()) { putOn(right, plateFormula()); clearPlate(); } });
  $('clear').addEventListener('click', clearPlate);

  // ---------- shelves ----------
  const visible = (/** @type {any} */ r) => rank(r.band) <= rank(band);
  const drawShelves = () => {
    /** @type {Set<string>} */
    const species = new Set();
    for (const r of codex.reactions) if (visible(r)) for (const f of Object.keys(sideOf(r.equation.split(' -> ')[0]))) species.add(f);
    const list = [...species].sort((a, b) => (NAMES[a] || a).localeCompare(NAMES[b] || b));
    const box = $('bottles'); box.innerHTML = '';
    for (const f of list) {
      const b = document.createElement('button'); b.type = 'button'; b.className = 'bottle';
      const dots = atomsOf(f).slice(0, 8).map((e) => `<i style="background:${hex(EL[e].c)}"></i>`).join('');
      b.innerHTML = `<span class="dots">${dots}</span><span><b>${show(f)}</b><small>${esc(NAMES[f] || '')}</small></span>`;
      b.setAttribute('aria-label', `${NAMES[f] || f} (${f}): put on the table`);
      b.addEventListener('click', () => putOn(left, f));
      box.append(b);
    }
    $('bottleNote').textContent = `${list.length} substances for ${BAND_NAME[/** @type {keyof typeof BAND_NAME} */ (band)]}. Tap one to put it on “in”.`;
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
    $('scaleBox').hidden = rank(band) < rank('seedling');
  };
  const condBox = $('conds');
  for (const [k, label] of COND) {
    const b = document.createElement('button'); b.type = 'button'; b.className = 'cond'; b.textContent = label; b.setAttribute('aria-pressed', 'false'); b.dataset.cond = k;
    b.addEventListener('click', () => { if (conds.has(k)) conds.delete(k); else conds.add(k); b.setAttribute('aria-pressed', String(conds.has(k))); });
    condBox.append(b);
  }
  const setBand = (/** @type {string} */ b) => {
    band = b; store.set('band', b);
    document.querySelectorAll('.band button').forEach((x) => x.setAttribute('aria-pressed', String(/** @type {HTMLElement} */ (x).dataset.band === b)));
    drawShelves(); drawCodex();
  };
  document.querySelectorAll('.band button').forEach((x) => x.addEventListener('click', () => setBand(/** @type {string} */ (/** @type {HTMLElement} */ (x).dataset.band))));
  $('typeForm').addEventListener('submit', (ev) => {
    ev.preventDefault(); const v = /** @type {HTMLInputElement} */ ($('typeIn')).value.trim(); if (!v) return;
    if (/->|→|=/.test(v)) { runChamber(v.replace('→', '->')); return; }
    const c = parseFormula(v);
    if (!c) { setStatus('Use element symbols and numbers, like H2O or Ca(OH)2.'); return; }
    putOn(left, written[hill(c)] || v.replace(/\s/g, ''));
  });

  // ---------- tabs, codex, notebook ----------
  document.querySelectorAll('.tabs button').forEach((t) => t.addEventListener('click', () => {
    const name = /** @type {HTMLElement} */ (t).dataset.tab;
    document.querySelectorAll('.tabs button').forEach((x) => x.setAttribute('aria-selected', String(x === t)));
    document.querySelectorAll('.tabpane').forEach((p) => { /** @type {HTMLElement} */ (p).hidden = /** @type {HTMLElement} */ (p).dataset.pane !== name; });
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
          ? `<div class="entry found"><b>${esc(r.name)}</b><div class="eq">${showEq(r.equation)}</div><div>${esc(describe(r))}</div></div>`
          : `<div class="entry"><b>?</b><div>${r.needs.length ? 'Needs ' + r.needs.map(condName).join(' and ') + '.' : 'Needs nothing special.'}</div></div>`;
      }
      html += '</details>';
    }
    if (mysteries.length) html += `<details class="fam" open><summary><b>Mystery cards</b> <span>${mysteries.length}</span></summary>${mysteries.map((m) => `<div class="entry mystery"><div class="eq">${showEq(m)}</div><div>Predicted by the model, not confirmed.</div></div>`).join('')}</details>`;
    if (labFinds.length) html += `<details class="fam" open><summary><b>Lab records</b> <span>${labFinds.length}</span></summary>${labFinds.map((m) => `<div class="entry found"><div class="eq">${showEq(plain(m))}</div><div>Recorded by real chemists.</div></div>`).join('')}</details>`;
    html += `<p class="note">${LAB_CREDIT}</p>`;
    $('codex').innerHTML = html;
  };
  const drawNotes = () => {
    $('notes').innerHTML = notes.length ? notes.map((x) => `<div class="entry${x.ok ? ' found' : ''}"><div class="eq">${showEq(x.eq)}</div><div>${esc(x.what)}</div></div>`).join('') : '<p class="note">Nothing yet.</p>';
  };
  const note = (/** @type {string} */ eq, /** @type {string} */ what, /** @type {boolean} */ ok) => { notes = [{ t: new Date().toISOString(), eq, what, ok }, ...notes].slice(0, 40); store.set('notes', notes); drawNotes(); };

  // ---------- running the chamber ----------
  const result = (/** @type {string} */ cls, /** @type {string} */ html) => { const r = $('result'); r.hidden = false; r.className = 'result ' + cls; r.innerHTML = html; r.scrollIntoView({ block: 'nearest', behavior: 'smooth' }); };
  /** @type {Record<string, string>} */
  const HEAT = names.heat;
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
  const runChamber = async (/** @type {string} */ input) => {
    const raw = JSON.parse(judge_json(input, [...conds].join(','), band));
    const [lt, rt] = input.split('->').map((s) => s.trim());
    const inSide = sideOf(lt); const guess = rt ? sideOf(rt) : null;
    if (raw.kind === 'parse_error') { result('bad', `<p>${esc(raw.message)}</p>`); return; }
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
      if (isNew) { found = [...found, r.id]; store.set('found', found); drawCodex(); }
      result('good', `<p><b>${esc(r.name)}!</b> ${esc(describe(r))}</p><p class="eq">${showEq(eqText)}</p>${r.heat ? `<p>${HEAT[r.heat]}</p>` : ''}<p>${isNew ? 'New codex entry!' : 'Already in your codex.'}</p>`);
      note(eqText, r.name, true); animate(inSide, out, fxFor(r, out)); return;
    }
    if (raw.kind === 'unbalanced') {
      const near = raw.close ? `<p>You’re close: these really do react. ${rank(band) <= rank('seedling') ? 'Try changing how many of each you use.' : 'Change the numbers in front.'}</p>` : '<p>Atoms are never made or destroyed, so both sides need the same atoms.</p>';
      result('bad', `<p><b>The scale isn’t level.</b> ${raw.report.map(esc).join(' ')}</p>${near}`);
      note(input, 'Not balanced.', false); animate(inSide, inSide, 'fizzle'); return;
    }
    if (raw.kind === 'mystery' || raw.kind === 'no_reaction') {
      const rec = await labRecord(input);
      if (rec) {
        const [key, times] = rec; const eq = plain(key); const [, outText] = eq.split(' -> ');
        const isNew = !labFinds.includes(key);
        if (isNew) { labFinds = [...labFinds, key]; store.set('lab', labFinds); drawCodex(); }
        const surprise = raw.kind === 'no_reaction' && guess ? '<p>Our simple model didn’t expect this one, but the lab record says it happened.</p>' : '';
        result('lab', `<p><b>Lab record!</b> It’s not in the codex, but real chemists have recorded ${guess ? 'this reaction' : 'these turning into something'} ${times === 1 ? 'once' : times + ' times'}:</p><p class="eq">${showEq(eq)}</p>${surprise}<p class="credit">Formulas only, so it could be a different molecule with the same atoms. ${LAB_CREDIT}</p><p>${isNew ? 'New lab record in your codex!' : 'Already in your codex.'}</p>`);
        note(eq, 'Lab record (published reaction).', true); animate(inSide, sideOf(outText ?? ''), 'glow'); return;
      }
    }
    if (raw.kind === 'mystery' && guess) {
      if (!mysteries.includes(input)) { mysteries = [...mysteries, input]; store.set('mysteries', mysteries); drawCodex(); }
      result('mystery', `<p><b>Mystery card!</b> This isn’t in our reaction data. The model says it might hold together:</p><ul>${raw.checks.map((/** @type {string} */ c) => `<li>${esc(c)}</li>`).join('')}</ul><p>Predicted by the model, not confirmed. Real chemists would test it in a lab.</p>`);
      note(input, 'Mystery card (predicted, not confirmed).', true); animate(inSide, guess, 'glow'); return;
    }
    result('bad', `<p><b>Fizzle.</b> ${esc(raw.reason || 'Nothing happens.')}</p>${guess ? '' : '<p>Try other bottles, another condition, or write your guess in “out”.</p>'}`);
    note(input, raw.reason || 'Nothing happened.', false); animate(inSide, inSide, 'fizzle');
  };
  $('test').addEventListener('click', () => {
    if (!Object.keys(left).length) { result('bad', '<p>Put at least one substance on “in” first.</p>'); return; }
    const input = Object.keys(right).length ? `${sideText(left)} -> ${sideText(right)}` : sideText(left);
    runChamber(input);
  });
  const emptyChamber = () => { anim = null; chamberGroup.clear(); fxGroup.clear(); chamberLight.intensity = 0; };
  $('emptyChamber').addEventListener('click', () => { emptyChamber(); $('result').hidden = true; });
  $('clearTable').addEventListener('click', () => { for (const s of [left, right]) for (const k of Object.keys(s)) delete s[k]; emptyChamber(); $('result').hidden = true; drawTable(); });

  // Chamber animation: molecules shake, atoms fly apart, then rejoin as the products.
  /** @type {any} */
  let anim = null; const chamberGroup = new THREE.Group(); scene.add(chamberGroup);
  const fxGroup = new THREE.Group(); scene.add(fxGroup);
  const molLayouts = (/** @type {Record<string, number>} */ side) => {
    const list = Object.entries(side).flatMap(([f, n]) => Array(Math.min(n, 6)).fill(f)).slice(0, 12);
    const per = Math.ceil(Math.sqrt(list.length));
    return list.map((f, i) => {
      const els = atomsOf(f); const col = i % per, row = Math.floor(i / per);
      const c = CH.clone().add(new THREE.Vector3((col - (per - 1) / 2) * (2 / per), -0.7 + row * (1.6 / per), (i % 2 ? -0.3 : 0.3)));
      const s = 0.55 * Math.min(1, 2.4 / per) * (els.length > 20 ? 0.5 : 1);
      const { pos, bonds } = layout(els); pos.forEach((p) => p.multiplyScalar(s).add(c));
      return { els, pos, bonds, s };
    });
  };
  const animate = (/** @type {Record<string, number>} */ inSide, /** @type {Record<string, number>} */ outSide, /** @type {string} */ fx) => {
    chamberGroup.clear(); fxGroup.clear();
    const A = molLayouts(inSide), B = molLayouts(outSide);
    /** @type {any[]} */
    const items = []; /** @type {Record<string, any[]>} */ const pool = {};
    A.forEach((m) => m.els.forEach((e, i) => { const mesh = atomMesh(e); mesh.scale.multiplyScalar(m.s); mesh.position.copy(m.pos[i]); chamberGroup.add(mesh); const it = { mesh, from: m.pos[i].clone(), mid: CH.clone().add(new THREE.Vector3((Math.random() - 0.5) * 1.8, (Math.random() - 0.5) * 1.6, (Math.random() - 0.5) * 1.4)), to: null }; items.push(it); (pool[e] = pool[e] || []).push(it); }));
    B.forEach((m) => m.els.forEach((e, i) => { const it = (pool[e] || []).shift(); if (it) it.to = m.pos[i].clone(); }));
    items.forEach((it) => { if (!it.to) it.to = it.from.clone(); });
    const before = new THREE.Group(), after = new THREE.Group(); chamberGroup.add(before, after);
    anim = { t: 0, items, A, B, before, after, fx };
  };
  const puffs = (/** @type {number} */ n, /** @type {() => number} */ color, /** @type {number} */ y0, /** @type {boolean} */ basic) => {
    for (let i = 0; i < n; i++) {
      const mat = basic ? new THREE.MeshBasicMaterial({ color: color(), transparent: true, opacity: 0.9 }) : new THREE.MeshStandardMaterial({ color: color(), transparent: true, opacity: 0.55, roughness: 0.1 });
      const m = new THREE.Mesh(new THREE.SphereGeometry(0.05 + Math.random() * 0.08, 8, 6), mat);
      m.position.copy(CH).add(new THREE.Vector3((Math.random() - 0.5) * 1.6, y0 - Math.random() * 0.3, (Math.random() - 0.5) * 1.2)); m.userData.v = 0.5 + Math.random(); fxGroup.add(m);
    }
  };
  const ease = (/** @type {number} */ x) => x * x * (3 - 2 * x);
  const stepAnim = (/** @type {number} */ dt) => {
    if (!anim) return;
    anim.t += dt; const t = anim.t; const fizzle = anim.fx === 'fizzle';
    const pos = anim.items.map((/** @type {any} */ it) => it.mesh.position);
    if (t < 0.9) anim.items.forEach((/** @type {any} */ it) => it.mesh.position.copy(it.from).add(new THREE.Vector3(Math.random() - 0.5, Math.random() - 0.5, Math.random() - 0.5).multiplyScalar(0.05 * t)));
    else if (fizzle) anim.items.forEach((/** @type {any} */ it) => it.mesh.position.copy(it.from));
    else if (t < 1.7) { const k = ease((t - 0.9) / 0.8); anim.items.forEach((/** @type {any} */ it) => it.mesh.position.copy(it.from).lerp(it.mid, k)); }
    else if (t < 2.7) { const k = ease((t - 1.7) / 1.0); anim.items.forEach((/** @type {any} */ it) => it.mesh.position.copy(it.mid).lerp(it.to, k)); }
    else anim.items.forEach((/** @type {any} */ it) => it.mesh.position.copy(it.to));
    const flat = (/** @type {any[]} */ layouts) => { let k = 0; /** @type {{a:number,b:number,o:number}[]} */ const Bd = []; for (const m of layouts) { for (const b of m.bonds) Bd.push({ a: k + b.a, b: k + b.b, o: b.o }); k += m.els.length; } return Bd; };
    if (t < 0.9 || fizzle) { anim.before.visible = true; anim.after.visible = false; drawBonds(anim.before, pos, flat(anim.A)); }
    else if (t > 2.45) {
      anim.before.visible = false; anim.after.visible = true;
      /** @type {any[]} */ const Pp = [];
      for (const m of anim.B) m.els.forEach((/** @type {string} */ e, /** @type {number} */ i) => { const it = anim.items.find((/** @type {any} */ x) => x.to.equals(m.pos[i])); Pp.push(it ? it.mesh.position : m.pos[i]); });
      drawBonds(anim.after, Pp, flat(anim.B));
    } else { anim.before.visible = false; anim.after.visible = false; }
    if (!fizzle && t > 2.7 && !anim.fired) {
      anim.fired = true;
      if (anim.fx === 'flame') puffs(40, () => (Math.random() < 0.5 ? 0xffb03a : 0x5aa0ff), -1.0, true);
      else if (anim.fx === 'bubbles') puffs(30, () => 0xcfe6ff, -1.1, false);
      else if (anim.fx === 'cloud') puffs(40, () => 0xf2f2f2, -0.2, false);
    }
    chamberLight.color.setHex(anim.fx === 'flame' ? 0xff8a3a : anim.fx === 'glow' ? 0x9fd0ff : 0xbfe0ff);
    chamberLight.intensity = fizzle ? 0 : t > 2.7 ? Math.max(0, 2.4 - (t - 2.7) * 0.5) : 0;
  };

  // ---------- picking atoms off the plate ----------
  const ray = new THREE.Raycaster(); const ndc = new THREE.Vector2(); /** @type {number[] | null} */ let downAt = null;
  canvas.addEventListener('pointerdown', (e) => { downAt = [e.clientX, e.clientY]; });
  canvas.addEventListener('pointerup', (e) => {
    if (!downAt || Math.hypot(e.clientX - downAt[0], e.clientY - downAt[1]) > 5) return;
    const r = canvas.getBoundingClientRect(); ndc.set(((e.clientX - r.left) / r.width) * 2 - 1, -((e.clientY - r.top) / r.height) * 2 + 1);
    ray.setFromCamera(ndc, camera); const hit = ray.intersectObjects(atoms.map((a) => a.mesh))[0];
    if (hit) { const i = atoms.findIndex((a) => a.mesh === hit.object); scene.remove(atoms[i].mesh); atoms.splice(i, 1); restructure(); }
  });
  /** @type {AudioContext | null} */
  let ac = null;
  const tone = (/** @type {string} */ el) => { try { ac = ac || new AudioContext(); const o = ac.createOscillator(), g = ac.createGain(); o.frequency.value = 300 + (Object.keys(EL).indexOf(el) % 12) * 60; g.gain.setValueAtTime(0.05, ac.currentTime); g.gain.exponentialRampToValueAtTime(0.0001, ac.currentTime + 0.18); o.connect(g).connect(ac.destination); o.start(); o.stop(ac.currentTime + 0.2); } catch { /* no sound */ } };

  // ---------- loop ----------
  const resize = () => { const r = canvas.getBoundingClientRect(); const w = Math.max(1, r.width), h = Math.max(1, r.height); renderer.setSize(w, h, false); camera.aspect = w / h; camera.fov = w / h < 1 ? 68 : w / h < 1.4 ? 52 : 42; camera.updateProjectionMatrix(); };
  new ResizeObserver(resize).observe(canvas); resize();
  const clock = new THREE.Clock();
  const reduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  const beam = $('beam'); let beamNow = 0;
  const frame = () => {
    const dt = Math.min(clock.getDelta(), 0.05); const tt = clock.elapsedTime;
    if (atoms.length) {
      const pos = atoms.map((a) => a.p.clone().sub(PLATE));
      relax(pos, atoms.map((a) => a.el), plateInfo.bonds, 3);
      atoms.forEach((a, i) => { a.p.copy(pos[i].add(PLATE)); a.mesh.position.copy(a.p); if (!reduce) a.mesh.position.y += Math.sin(tt * 1.6) * 0.04; });
      const spin = reduce ? 0 : dt * 0.35; atoms.forEach((a) => { const v = a.p.clone().sub(PLATE); v.applyAxisAngle(UP, spin); a.p.copy(v.add(PLATE)); });
      drawBonds(plateBonds, atoms.map((a) => a.mesh.position), plateInfo.bonds);
    } else drawBonds(plateBonds, [], []);
    stepAnim(dt);
    fxGroup.children.forEach((/** @type {any} */ m) => { m.position.y += m.userData.v * dt * (anim && anim.fx === 'cloud' ? -0.3 : 1); m.material.opacity = Math.max(0, m.material.opacity - dt * 0.3); if (m.position.y > CH.y + 1.1) m.position.y = CH.y - 1.1; if (m.position.y < CH.y - 1.15) m.position.y = CH.y - 1.15; });
    beamNow += (tilt - beamNow) * Math.min(1, dt * 4); beam.setAttribute('transform', `rotate(${(-beamNow).toFixed(2)} 150 60)`);
    ring.material.color.setHex(plateOk() ? 0x5fd08a : 0xe8b04a);
    controls.update(); renderer.render(scene, camera); requestAnimationFrame(frame);
  };

  setBand(band);
  drawTable(); drawNotes(); restructure();
  $('loading').hidden = true;
  /** @type {any} */ (window).__benchFree = { codex, run: runChamber, left, right, conds, found: () => found, setBand };
  requestAnimationFrame(frame);
}

if (typeof document !== 'undefined' && document.getElementById('scene')) {
  main().catch((e) => {
    const l = document.getElementById('loading');
    if (l) l.textContent = 'The bench could not start: ' + (e && e.message ? e.message : e);
  });
}
