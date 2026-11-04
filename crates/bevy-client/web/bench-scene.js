// The Chemistry Bench's 3D scene (three.js r128, vendored in vendor/): the
// wooden bench, the build plate, the glass test chamber, atoms and bonds,
// and the reaction animation. Rendering only: it is told which atoms and
// bonds to draw and never decides any chemistry (bench-free.js and the
// engine in crates/chem do that). Owned by the graphics work; bench-free.js
// talks to it only through the object createBenchScene returns.

/** @type {any} */
const THREE = /** @type {any} */ (globalThis).THREE;

/** @typedef {{v:number, c:number, r:number, name:string}} ElementLook */
/** @typedef {{a:number, b:number, o:number}} Bond */
/** @typedef {{els: string[], bonds: Bond[]}} Molecule */
/** @typedef {'flame'|'bubbles'|'cloud'|'glow'|'fizzle'} Effect */

/**
 * @param {HTMLCanvasElement} canvas
 * @param {{ elements: Record<string, ElementLook> }} opts
 */
export function createBenchScene(canvas, opts) {
  const EL = opts.elements;
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
  const matFor = (/** @type {string} */ el) => atomMat[el] || (atomMat[el] = new THREE.MeshStandardMaterial({ color: EL[el]?.c ?? 0xcccccc, roughness: el === 'C' ? 0.5 : 0.3, metalness: 0.05 }));
  const atomMesh = (/** @type {string} */ el) => { const m = new THREE.Mesh(sphereGeo, matFor(el)); m.scale.setScalar(EL[el]?.r ?? 0.3); m.castShadow = true; return m; };
  const UP = new THREE.Vector3(0, 1, 0);
  const drawBonds = (/** @type {any} */ group, /** @type {any[]} */ pos, /** @type {Bond[]} */ bonds) => {
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

  // A spring layout for drawing: atoms push apart, bonds pull to a rest length.
  const relax = (/** @type {any[]} */ pos, /** @type {string[]} */ els, /** @type {Bond[]} */ bonds, /** @type {number} */ steps) => {
    for (let s = 0; s < steps; s++) {
      const f = pos.map(() => new THREE.Vector3());
      for (let i = 0; i < pos.length; i++) for (let j = i + 1; j < pos.length; j++) {
        const d = pos[j].clone().sub(pos[i]); const L = Math.max(d.length(), 0.05);
        const push = d.multiplyScalar(-0.05 / (L * L * L)); f[i].add(push); f[j].sub(push);
      }
      for (const b of bonds) {
        const d = pos[b.b].clone().sub(pos[b.a]); const L = Math.max(d.length(), 0.01);
        const rest = ((EL[els[b.a] ?? '']?.r ?? 0.3) + (EL[els[b.b] ?? '']?.r ?? 0.3)) * 1.25;
        const k = d.multiplyScalar((L - rest) * 0.25 / L); f[b.a].add(k); f[b.b].sub(k);
      }
      const c = pos.reduce((m, p) => m.add(p), new THREE.Vector3()).multiplyScalar(1 / Math.max(pos.length, 1));
      pos.forEach((p, i) => { p.add(f[i].clampLength(0, 0.15)); p.x -= c.x * 0.02; p.z -= c.z * 0.02; });
    }
    return pos;
  };
  const layout = (/** @type {Molecule} */ m) => {
    const pos = m.els.map((e, i) => new THREE.Vector3(Math.cos(i * 2.4) * 0.6, Math.sin(i * 1.7) * 0.4, Math.sin(i * 2.4) * 0.6));
    relax(pos, m.els, m.bonds, m.els.length > 30 ? 80 : 220);
    return pos;
  };

  // ---------- the plate ----------
  const PLATE = new THREE.Vector3(-1.2, 1.05, 0.2);
  /** @type {{el:string, mesh:any, p:any}[]} */
  let atoms = [];
  /** @type {Bond[]} */
  let plateBondList = [];
  let plateOk = false;
  const plateBonds = new THREE.Group(); scene.add(plateBonds);

  // ---------- the chamber ----------
  /** @type {any} */
  let anim = null; const chamberGroup = new THREE.Group(); scene.add(chamberGroup);
  const fxGroup = new THREE.Group(); scene.add(fxGroup);
  const place = (/** @type {Molecule[]} */ mols) => {
    const per = Math.ceil(Math.sqrt(mols.length));
    return mols.map((m, i) => {
      const col = i % per, row = Math.floor(i / per);
      const c = CH.clone().add(new THREE.Vector3((col - (per - 1) / 2) * (2 / per), -0.7 + row * (1.6 / per), (i % 2 ? -0.3 : 0.3)));
      const s = 0.55 * Math.min(1, 2.4 / per) * (m.els.length > 20 ? 0.5 : 1);
      const pos = layout(m); pos.forEach((p) => p.multiplyScalar(s).add(c));
      return { els: m.els, pos, bonds: m.bonds, s };
    });
  };
  const puffs = (/** @type {number} */ n, /** @type {() => number} */ color, /** @type {number} */ y0, /** @type {boolean} */ basic) => {
    for (let i = 0; i < n; i++) {
      const mat = basic ? new THREE.MeshBasicMaterial({ color: color(), transparent: true, opacity: 0.9 }) : new THREE.MeshStandardMaterial({ color: color(), transparent: true, opacity: 0.55, roughness: 0.1 });
      const m = new THREE.Mesh(new THREE.SphereGeometry(0.05 + Math.random() * 0.08, 8, 6), mat);
      m.position.copy(CH).add(new THREE.Vector3((Math.random() - 0.5) * 1.6, y0 - Math.random() * 0.3, (Math.random() - 0.5) * 1.2)); m.userData.v = 0.5 + Math.random(); fxGroup.add(m);
    }
  };
  const ease = (/** @type {number} */ x) => x * x * (3 - 2 * x);
  const flat = (/** @type {any[]} */ layouts) => { let k = 0; /** @type {Bond[]} */ const Bd = []; for (const m of layouts) { for (const b of m.bonds) Bd.push({ a: k + b.a, b: k + b.b, o: b.o }); k += m.els.length; } return Bd; };
  const stepAnim = (/** @type {number} */ dt) => {
    if (!anim) return;
    anim.t += dt; const t = anim.t; const fizzle = anim.fx === 'fizzle';
    const pos = anim.items.map((/** @type {any} */ it) => it.mesh.position);
    if (t < 0.9) anim.items.forEach((/** @type {any} */ it) => it.mesh.position.copy(it.from).add(new THREE.Vector3(Math.random() - 0.5, Math.random() - 0.5, Math.random() - 0.5).multiplyScalar(0.05 * t)));
    else if (fizzle) anim.items.forEach((/** @type {any} */ it) => it.mesh.position.copy(it.from));
    else if (t < 1.7) { const k = ease((t - 0.9) / 0.8); anim.items.forEach((/** @type {any} */ it) => it.mesh.position.copy(it.from).lerp(it.mid, k)); }
    else if (t < 2.7) { const k = ease((t - 1.7) / 1.0); anim.items.forEach((/** @type {any} */ it) => it.mesh.position.copy(it.mid).lerp(it.to, k)); }
    else anim.items.forEach((/** @type {any} */ it) => it.mesh.position.copy(it.to));
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
  /** @type {(i: number) => void} */
  let onPick = () => {};
  const ray = new THREE.Raycaster(); const ndc = new THREE.Vector2(); /** @type {number[] | null} */ let downAt = null;
  const down = (/** @type {PointerEvent} */ e) => { downAt = [e.clientX, e.clientY]; };
  const up = (/** @type {PointerEvent} */ e) => {
    if (!downAt || Math.hypot(e.clientX - (downAt[0] ?? 0), e.clientY - (downAt[1] ?? 0)) > 5) return;
    const r = canvas.getBoundingClientRect(); ndc.set(((e.clientX - r.left) / r.width) * 2 - 1, -((e.clientY - r.top) / r.height) * 2 + 1);
    ray.setFromCamera(ndc, camera); const hit = ray.intersectObjects(atoms.map((a) => a.mesh))[0];
    if (hit) onPick(atoms.findIndex((a) => a.mesh === hit.object));
  };
  canvas.addEventListener('pointerdown', down);
  canvas.addEventListener('pointerup', up);

  // ---------- loop ----------
  const resize = () => { const r = canvas.getBoundingClientRect(); const w = Math.max(1, r.width), h = Math.max(1, r.height); renderer.setSize(w, h, false); camera.aspect = w / h; camera.fov = w / h < 1 ? 68 : w / h < 1.4 ? 52 : 42; camera.updateProjectionMatrix(); };
  const ro = new ResizeObserver(resize); ro.observe(canvas); resize();
  const clock = new THREE.Clock();
  const reduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  let running = true;
  const frame = () => {
    if (!running) return;
    const dt = Math.min(clock.getDelta(), 0.05); const tt = clock.elapsedTime;
    if (atoms.length) {
      const pos = atoms.map((a) => a.p.clone().sub(PLATE));
      relax(pos, atoms.map((a) => a.el), plateBondList, 3);
      atoms.forEach((a, i) => { a.p.copy(pos[i].add(PLATE)); a.mesh.position.copy(a.p); if (!reduce) a.mesh.position.y += Math.sin(tt * 1.6) * 0.04; });
      const spin = reduce ? 0 : dt * 0.35; atoms.forEach((a) => { const v = a.p.clone().sub(PLATE); v.applyAxisAngle(UP, spin); a.p.copy(v.add(PLATE)); });
      drawBonds(plateBonds, atoms.map((a) => a.mesh.position), plateBondList);
    } else drawBonds(plateBonds, [], []);
    stepAnim(dt);
    fxGroup.children.forEach((/** @type {any} */ m) => { m.position.y += m.userData.v * dt * (anim && anim.fx === 'cloud' ? -0.3 : 1); m.material.opacity = Math.max(0, m.material.opacity - dt * 0.3); if (m.position.y > CH.y + 1.1) m.position.y = CH.y - 1.1; if (m.position.y < CH.y - 1.15) m.position.y = CH.y - 1.15; });
    ring.material.color.setHex(plateOk ? 0x5fd08a : 0xe8b04a);
    controls.update(); renderer.render(scene, camera); requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);

  return {
    /** Adds one atom to the build plate. */
    addPlateAtom(/** @type {string} */ el) {
      const mesh = atomMesh(el); scene.add(mesh);
      const p = PLATE.clone().add(new THREE.Vector3((Math.random() - 0.5) * 1.2, 0.9, (Math.random() - 0.5) * 1.2));
      mesh.position.copy(p); atoms.push({ el, mesh, p });
    },
    /** Removes the plate atom at `i` (the order atoms were added in). */
    removePlateAtom(/** @type {number} */ i) { const a = atoms[i]; if (!a) return; scene.remove(a.mesh); atoms.splice(i, 1); },
    clearPlate() { for (const a of atoms) scene.remove(a.mesh); atoms = []; },
    /** The plate's bonds (indices into the plate atoms) and whether it is a finished molecule. */
    setPlateBonds(/** @type {Bond[]} */ bonds, /** @type {boolean} */ ok) { plateBondList = bonds; plateOk = ok; },
    /** Called with the index of a plate atom the player tapped. */
    onPickPlateAtom(/** @type {(i: number) => void} */ cb) { onPick = cb; },
    /** Plays a reaction in the chamber: shake, fly apart, rejoin as `outMols`, then `fx`. */
    playReaction(/** @type {Molecule[]} */ inMols, /** @type {Molecule[]} */ outMols, /** @type {Effect} */ fx) {
      chamberGroup.clear(); fxGroup.clear();
      const A = place(inMols), B = place(outMols);
      /** @type {any[]} */
      const items = []; /** @type {Record<string, any[]>} */ const pool = {};
      A.forEach((m) => m.els.forEach((e, i) => { const mesh = atomMesh(e); mesh.scale.multiplyScalar(m.s); mesh.position.copy(m.pos[i]); chamberGroup.add(mesh); const it = { mesh, from: m.pos[i].clone(), mid: CH.clone().add(new THREE.Vector3((Math.random() - 0.5) * 1.8, (Math.random() - 0.5) * 1.6, (Math.random() - 0.5) * 1.4)), to: null }; items.push(it); (pool[e] = pool[e] || []).push(it); }));
      B.forEach((m) => m.els.forEach((e, i) => { const it = (pool[e] || []).shift(); if (it) it.to = m.pos[i].clone(); }));
      items.forEach((it) => { if (!it.to) it.to = it.from.clone(); });
      const before = new THREE.Group(), after = new THREE.Group(); chamberGroup.add(before, after);
      anim = { t: 0, items, A, B, before, after, fx };
    },
    emptyChamber() { anim = null; chamberGroup.clear(); fxGroup.clear(); chamberLight.intensity = 0; },
    dispose() {
      running = false; ro.disconnect();
      canvas.removeEventListener('pointerdown', down); canvas.removeEventListener('pointerup', up);
      controls.dispose(); renderer.dispose();
    },
  };
}
