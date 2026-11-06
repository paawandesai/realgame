// The hit lens (graphics plan G4; Paawan, 2026-11-05: the Bench is the
// centrepiece). When a spell lands, the game (src/lens.rs) pushes a key on
// window.__hitLens: "rx:<reaction id>" or "mol:<formula>". This draws a
// round close-up beside the crosshair for a moment: the atoms regrouping
// (the Bench's show record, show_frame_json) or one molecule turning
// (geometry_json), with the equation underneath. three.js and the chemistry
// engine load only when the first lens is needed.
const SHOW_S = 3.2;
// ?lenshold keeps the last lens up (screenshots in slow headless runs).
const HOLD = new URLSearchParams(location.search).has('lenshold');
const SIZE = 200;

// CPK-style colours and radii (Å, scaled) for the elements the lens meets.
const EL = {
  H: [0xf2f2f2, 0.28], C: [0x2b2b2b, 0.42], N: [0x3050f8, 0.4], O: [0xe8302a, 0.38],
  S: [0xe8c22a, 0.48], Cl: [0x31c041, 0.46], Na: [0x8a5cd6, 0.52], K: [0x7b47c2, 0.56],
  Ca: [0x5ad35a, 0.52], Mg: [0x6bd16b, 0.5], Fe: [0xb5622d, 0.5], Cu: [0xc8803a, 0.48],
  Ag: [0xbfbfbf, 0.5], Zn: [0x7d80b0, 0.48], I: [0x940094, 0.56], Br: [0xa62929, 0.5],
};
const look = (el) => EL[el] || [0xd080ff, 0.42];

let ready = null;
let lens = null;

async function load() {
  if (!window.THREE) {
    await new Promise((ok, bad) => {
      const s = document.createElement('script');
      s.src = './vendor/three.min.js';
      s.onload = ok;
      s.onerror = bad;
      document.head.appendChild(s);
    });
  }
  const chem = await import('./chem.js');
  await chem.default();
  return chem;
}

function make() {
  const THREE = window.THREE;
  const wrap = document.createElement('div');
  wrap.id = 'hit-lens';
  wrap.style.cssText =
    'position:fixed;left:50%;top:50%;transform:translate(40px,-50%);width:' + SIZE + 'px;' +
    'pointer-events:none;z-index:30;opacity:0;transition:opacity .25s;text-align:center;' +
    'font:600 13px/1.3 Inter,system-ui,sans-serif;color:#f4efe6;text-shadow:0 1px 3px #000';
  const canvas = document.createElement('canvas');
  canvas.width = canvas.height = SIZE * Math.min(window.devicePixelRatio || 1, 2);
  canvas.style.cssText =
    'width:' + SIZE + 'px;height:' + SIZE + 'px;border-radius:50%;' +
    'border:2px solid rgba(232,208,150,.85);box-shadow:0 0 0 4px rgba(0,0,0,.35),0 6px 24px rgba(0,0,0,.5);' +
    'background:radial-gradient(circle at 40% 35%,#2a3340,#0c1016)';
  const caption = document.createElement('div');
  caption.style.marginTop = '6px';
  wrap.append(canvas, caption);
  document.body.appendChild(wrap);
  const renderer = new THREE.WebGLRenderer({ canvas, alpha: true, antialias: true });
  renderer.setPixelRatio(1);
  renderer.outputEncoding = THREE.sRGBEncoding;
  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(32, 1, 0.1, 100);
  camera.position.set(0, 0, 12);
  scene.add(new THREE.HemisphereLight(0xdfe8ff, 0x2a2018, 0.9));
  const key = new THREE.DirectionalLight(0xffffff, 0.9);
  key.position.set(3, 5, 6);
  scene.add(key);
  const group = new THREE.Group();
  scene.add(group);
  const sphere = new THREE.SphereGeometry(1, 20, 14);
  const rod = new THREE.CylinderGeometry(1, 1, 1, 8);
  const mats = {};
  const mat = (el) => mats[el] || (mats[el] = new THREE.MeshStandardMaterial({ color: look(el)[0], roughness: 0.35, metalness: 0.05 }));
  const bondMat = new THREE.MeshStandardMaterial({ color: 0xcfc6b8, roughness: 0.5 });
  const UP = new THREE.Vector3(0, 1, 0);

  // Draws atoms (el, pos) and bonds (a, b, order) into the group.
  function draw(atoms, bonds) {
    group.clear();
    const pts = atoms.map((a) => new THREE.Vector3(a.pos[0], a.pos[1], a.pos[2]));
    const c = pts.reduce((m, p) => m.add(p), new THREE.Vector3()).multiplyScalar(1 / Math.max(pts.length, 1));
    let r = 1;
    pts.forEach((p) => { p.sub(c); r = Math.max(r, p.length()); });
    group.scale.setScalar(3.4 / (r + 0.6));
    atoms.forEach((a, i) => {
      const m = new THREE.Mesh(sphere, mat(a.el));
      m.scale.setScalar(look(a.el)[1]);
      m.position.copy(pts[i]);
      group.add(m);
    });
    for (const b of bonds) {
      const p = pts[b.a], q = pts[b.b];
      if (!p || !q) continue;
      const d = q.clone().sub(p);
      const len = d.length();
      if (len < 1e-3 || len > 3.2) continue;
      const n = Math.max(1, Math.min(3, Math.round(b.order || 1)));
      const side = new THREE.Vector3().crossVectors(d, Math.abs(d.y) < 0.9 ? UP : new THREE.Vector3(1, 0, 0)).normalize().multiplyScalar(0.12);
      for (let k = 0; k < n; k++) {
        const m = new THREE.Mesh(rod, bondMat);
        const off = side.clone().multiplyScalar(k - (n - 1) / 2);
        m.position.copy(p).add(q).multiplyScalar(0.5).add(off);
        m.quaternion.setFromUnitVectors(UP, d.clone().normalize());
        m.scale.set(0.07, len, 0.07);
        group.add(m);
      }
    }
  }
  return { wrap, caption, renderer, scene, camera, group, draw };
}

// Subscripts the counts in a formula (H2O → H₂O), not a leading coefficient.
const sub = (f) => f.replace(/([A-Za-z)\]])(\d+)/g, (_, a, d) => a + d.split('').map((c) => '₀₁₂₃₄₅₆₇₈₉'[+c]).join(''));

function equation(show) {
  const side = (s) => show.species
    .filter((x) => x.side === s)
    .map((x) => (x.coef > 1 ? x.coef : '') + sub(x.formula))
    .join(' + ');
  return side('reactant') + ' → ' + side('product');
}


async function play(key) {
  if (!ready) ready = load();
  let chem;
  try {
    chem = await ready;
  } catch (e) {
    console.warn('hit lens unavailable:', e);
    return;
  }
  if (!lens) lens = make();
  const [kind, id] = [key.slice(0, key.indexOf(':')), key.slice(key.indexOf(':') + 1)];
  let frame;
  let text;
  try {
    if (kind === 'rx') {
      const show = JSON.parse(chem.show_json(id));
      text = equation(show);
      frame = (t) => JSON.parse(chem.show_frame_json(id, t));
    } else {
      const g = JSON.parse(chem.geometry_json(id));
      const atoms = g.elements.map((el, i) => ({ el, pos: g.positions[i] }));
      const bonds = g.bonds.map(([a, b, order]) => ({ a, b, order }));
      text = sub(id);
      frame = () => ({ atoms, bonds });
    }
    frame(0);
  } catch (e) {
    console.warn('hit lens: no record for', key, e);
    return;
  }
  lens.caption.textContent = text;
  lens.wrap.style.opacity = '1';
  const t0 = performance.now();
  const tick = () => {
    const s = (performance.now() - t0) / 1000;
    const t = Math.min(1, Math.max(0, (s - 0.3) / (SHOW_S - 0.9)));
    const f = frame(t);
    lens.draw(f.atoms, f.bonds);
    lens.group.rotation.y = s * 0.6;
    lens.group.rotation.x = 0.35;
    lens.renderer.render(lens.scene, lens.camera);
    if (s < SHOW_S) requestAnimationFrame(tick);
    else if (!HOLD) lens.wrap.style.opacity = '0';
  };
  requestAnimationFrame(tick);
}

// The game pushes keys; take the newest each frame (older ones are stale).
let busyUntil = 0;
function poll() {
  const q = window.__hitLens;
  if (Array.isArray(q) && q.length) {
    const key = q[q.length - 1];
    q.length = 0;
    if (performance.now() >= busyUntil) {
      busyUntil = performance.now() + SHOW_S * 1000;
      play(key);
    }
  }
  requestAnimationFrame(poll);
}
requestAnimationFrame(poll);
window.__hitLensPlay = play;
