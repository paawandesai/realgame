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
  // Quality tier (graphics plan): 'low' drops shadows, glass refraction and
  // the high pixel ratio; 'mid' is the default; 'high' adds sharper shadows.
  /** @type {'low'|'mid'|'high'} */
  let tier = /** @type {any} */ (opts.tier || new URLSearchParams(location.search).get('tier') || 'mid');
  if (!['low', 'mid', 'high'].includes(tier)) tier = 'mid';
  const renderer = new THREE.WebGLRenderer({ canvas, antialias: tier !== 'low' });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, tier === 'low' ? 1 : tier === 'mid' ? 1.5 : 2));
  renderer.outputEncoding = THREE.sRGBEncoding;
  // Filmic tonemapping keeps bright glass and flame highlights soft.
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.toneMappingExposure = 1.05;
  renderer.shadowMap.enabled = tier !== 'low';
  renderer.shadowMap.type = THREE.PCFSoftShadowMap;
  const scene = new THREE.Scene();
  scene.background = labBackdrop();
  scene.fog = new THREE.Fog(0x0e1118, 14, 30);
  // Image-based light from a small lab drawn in code (no image files): warm
  // window, cool ceiling, dark walls. Gives atoms and glass real reflections.
  const pmrem = new THREE.PMREMGenerator(renderer);
  const envScene = labEnvironment();
  const envTex = pmrem.fromScene(envScene, 0.02).texture;
  scene.environment = envTex;
  envScene.traverse((/** @type {any} */ o) => { if (o.geometry) o.geometry.dispose(); if (o.material) o.material.dispose(); });
  pmrem.dispose();
  const camera = new THREE.PerspectiveCamera(42, 1, 0.1, 100);
  camera.position.set(0.75, 4.4, 9.4);
  const controls = new THREE.OrbitControls(camera, canvas);
  controls.target.set(0.75, 0.9, 0); controls.enableDamping = true; controls.minDistance = 3.5; controls.maxDistance = 13; controls.maxPolarAngle = 1.45;
  scene.add(new THREE.HemisphereLight(0xbcd0ff, 0x2a1c10, 0.25));
  // Key: warm window light from the left; soft shadows.
  const lamp = new THREE.SpotLight(0xffd6a0, 1.5, 30, 0.62, 0.65); lamp.position.set(-3.5, 8, 5); lamp.castShadow = true;
  lamp.shadow.mapSize.set(tier === 'high' ? 2048 : 1024, tier === 'high' ? 2048 : 1024); lamp.shadow.radius = 4; lamp.shadow.bias = -0.0004;
  scene.add(lamp); lamp.target.position.set(0.5, 0, 0); scene.add(lamp.target);
  // Cool rim from behind so atoms separate from the dark room.
  const rim = new THREE.DirectionalLight(0x8fb4ff, 0.7); rim.position.set(4, 4, -6); scene.add(rim);
  const woodTex = woodTexture(); woodTex.anisotropy = tier === 'low' ? 1 : 4;
  const wood = new THREE.MeshStandardMaterial({ color: 0xffffff, map: woodTex, roughness: 0.62, bumpMap: woodTex, bumpScale: 0.004, envMapIntensity: 0.6 });
  // The room: a dark floor and back wall, so the bench is lit inside a space.
  const roomMat = new THREE.MeshStandardMaterial({ color: 0x0b0a09, roughness: 0.95, envMapIntensity: 0.08 });
  const floor = new THREE.Mesh(new THREE.PlaneGeometry(60, 60), roomMat); floor.rotation.x = -Math.PI / 2; floor.position.y = -3.3; floor.receiveShadow = true; scene.add(floor);
  const wall = new THREE.Mesh(new THREE.PlaneGeometry(60, 20), roomMat); wall.position.set(0, 4, -6); scene.add(wall);
  const top = new THREE.Mesh(new THREE.BoxGeometry(8.4, 0.3, 3.6), wood); top.position.set(0.6, -0.15, 0); top.receiveShadow = true; top.castShadow = true; scene.add(top);
  for (const [x, z] of [[-3.3, -1.5], [4.5, -1.5], [-3.3, 1.5], [4.5, 1.5]]) { const leg = new THREE.Mesh(new THREE.BoxGeometry(0.28, 3, 0.28), wood); leg.position.set(x, -1.8, z); leg.castShadow = true; scene.add(leg); }
  const back = new THREE.Mesh(new THREE.BoxGeometry(8.4, 2.6, 0.18), new THREE.MeshStandardMaterial({ color: 0xb08a68, map: woodTex, roughness: 0.7, envMapIntensity: 0.5 })); back.position.set(0.6, 1.3, -1.9); back.receiveShadow = true; scene.add(back);
  const brass = new THREE.MeshStandardMaterial({ color: 0xe0b46a, metalness: 1, roughness: 0.28, envMapIntensity: 1.6 });
  const plate = new THREE.Mesh(new THREE.CylinderGeometry(1.5, 1.6, 0.12, 64), new THREE.MeshStandardMaterial({ color: 0x2b3448, metalness: 0.7, roughness: 0.28 }));
  plate.position.set(-1.2, 0.06, 0.2); plate.receiveShadow = true; scene.add(plate);
  const ring = new THREE.Mesh(new THREE.TorusGeometry(1.52, 0.035, 8, 64), new THREE.MeshBasicMaterial({ color: 0xe8b04a })); ring.rotation.x = Math.PI / 2; ring.position.set(-1.2, 0.13, 0.2); scene.add(ring);
  const CH = new THREE.Vector3(2.7, 1.25, 0.1);
  // The chamber: clear glass that reflects the lab (environment light, a
  // clearcoat sheen and Fresnel edges), on a brass base with a brass lid rim.
  const glassMat = tier === 'low'
    ? new THREE.MeshStandardMaterial({ color: 0xcfe6ff, transparent: true, opacity: 0.14, roughness: 0.05, metalness: 0.1, depthWrite: false })
    : new THREE.MeshPhysicalMaterial({ color: 0xe8f6ff, transparent: true, opacity: 0.16, roughness: 0.02, metalness: 0, clearcoat: 1, clearcoatRoughness: 0.02, reflectivity: 0.9, envMapIntensity: 2.2, depthWrite: false, side: THREE.DoubleSide });
  const glass = new THREE.Mesh(new THREE.BoxGeometry(2.4, 2.4, 2.0), glassMat);
  glass.position.copy(CH); glass.renderOrder = 2; scene.add(glass);
  const edges = new THREE.LineSegments(new THREE.EdgesGeometry(new THREE.BoxGeometry(2.4, 2.4, 2.0)), new THREE.LineBasicMaterial({ color: 0xd8f0ff, transparent: true, opacity: 0.35 }));
  edges.position.copy(CH); scene.add(edges);
  const base = new THREE.Mesh(new THREE.BoxGeometry(2.6, 0.1, 2.2), new THREE.MeshStandardMaterial({ color: 0x262b33, metalness: 0.4, roughness: 0.35 })); base.position.set(CH.x, CH.y - 1.25, CH.z); base.castShadow = true; base.receiveShadow = true; scene.add(base);
  // An open brass frame on top (four bars), so the chamber reads as glass, not a box with a lid.
  for (const [w, d, x, z] of [[2.5, 0.07, 0, 1.03], [2.5, 0.07, 0, -1.03], [0.07, 2.1, 1.23, 0], [0.07, 2.1, -1.23, 0]]) {
    const bar = new THREE.Mesh(new THREE.BoxGeometry(w, 0.07, d), brass); bar.position.set(CH.x + x, CH.y + 1.22, CH.z + z); bar.castShadow = true; scene.add(bar);
  }
  const chamberLight = new THREE.PointLight(0xff8a3a, 0, 6); chamberLight.position.copy(CH); scene.add(chamberLight);
  const sphereGeo = new THREE.SphereGeometry(1, tier === 'low' ? 20 : 36, tier === 'low' ? 14 : 26);
  // A half bond: unit length from y = 0 to y = 1, so each half is coloured by its own atom.
  const bondGeo = new THREE.CylinderGeometry(0.065, 0.065, 1, 14); bondGeo.translate(0, 0.5, 0);
  /** @type {Record<string, any>} */
  const atomMat = {};
  // Glossy molecular-model plastic: a clearcoat highlight over the element colour.
  const matFor = (/** @type {string} */ el) => atomMat[el] || (atomMat[el] = tier === 'low'
    ? new THREE.MeshStandardMaterial({ color: EL[el]?.c ?? 0xcccccc, roughness: 0.32, metalness: 0.0 })
    : new THREE.MeshPhysicalMaterial({ color: EL[el]?.c ?? 0xcccccc, roughness: el === 'C' ? 0.42 : 0.3, metalness: 0, clearcoat: 1, clearcoatRoughness: 0.08, envMapIntensity: 1.1 }));
  /** @type {Record<string, any>} */
  const bondMats = {};
  const bondMatFor = (/** @type {string} */ el) => bondMats[el] || (bondMats[el] = new THREE.MeshStandardMaterial({ color: EL[el]?.c ?? 0xc9ced8, roughness: 0.38, metalness: 0, envMapIntensity: 0.9 }));
  const atomMesh = (/** @type {string} */ el) => { const m = new THREE.Mesh(sphereGeo, matFor(el)); m.scale.setScalar(EL[el]?.r ?? 0.3); m.castShadow = true; return m; };
  const UP = new THREE.Vector3(0, 1, 0);
  // Bonds are drawn as two halves, each in its atom's colour (as in
  // textbook models); double and triple bonds are thinner parallel rods.
  const drawBonds = (/** @type {any} */ group, /** @type {any[]} */ pos, /** @type {Bond[]} */ bonds, /** @type {string[]} */ els) => {
    let k = 0;
    for (const b of bonds) {
      const a = pos[b.a], c = pos[b.b]; if (!a || !c) continue; const d = c.clone().sub(a); const L = d.length(); if (L < 1e-3) continue;
      const side = d.clone().cross(Math.abs(d.y) < 0.9 * L ? UP : new THREE.Vector3(1, 0, 0)).normalize().multiplyScalar(0.11);
      const dir = d.clone().normalize(); const thin = b.o > 1 ? 0.72 : 1;
      for (let o = 0; o < b.o; o++) {
        const off = side.clone().multiplyScalar(o - (b.o - 1) / 2);
        for (const half of [0, 1]) {
          const el = (half ? els[b.b] : els[b.a]) ?? '';
          let rod = group.children[k++]; if (!rod) { rod = new THREE.Mesh(bondGeo); rod.castShadow = tier !== 'low'; group.add(rod); }
          rod.material = bondMatFor(el); rod.visible = true;
          rod.position.copy(half ? c : a).add(off);
          rod.scale.set(thin, L / 2, thin);
          rod.quaternion.setFromUnitVectors(UP, half ? dir.clone().negate() : dir);
        }
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
    if (t < 0.9 || fizzle) { anim.before.visible = true; anim.after.visible = false; drawBonds(anim.before, pos, flat(anim.A), anim.A.flatMap((/** @type {any} */ m) => m.els)); }
    else if (t > 2.45) {
      anim.before.visible = false; anim.after.visible = true;
      /** @type {any[]} */ const Pp = [];
      for (const m of anim.B) m.els.forEach((/** @type {string} */ e, /** @type {number} */ i) => { const it = anim.items.find((/** @type {any} */ x) => x.to.equals(m.pos[i])); Pp.push(it ? it.mesh.position : m.pos[i]); });
      drawBonds(anim.after, Pp, flat(anim.B), anim.B.flatMap((/** @type {any} */ m) => m.els));
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
      drawBonds(plateBonds, atoms.map((a) => a.mesh.position), plateBondList, atoms.map((a) => a.el));
    } else drawBonds(plateBonds, [], [], []);
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
      controls.dispose(); envTex.dispose(); woodTex.dispose(); renderer.dispose();
    },
    /** The current quality tier ('low', 'mid' or 'high'); set it with `?tier=` or `opts.tier`. */
    get tier() { return tier; },
  };
}

// ---------- generated looks (no image files: everything is drawn in code) ----------

/** A soft vertical gradient behind the bench: dark room, a faint warm glow low left. */
function labBackdrop() {
  const c = document.createElement('canvas'); c.width = 16; c.height = 256;
  const g = /** @type {CanvasRenderingContext2D} */ (c.getContext('2d'));
  const grad = g.createLinearGradient(0, 0, 0, 256);
  grad.addColorStop(0, '#07090d'); grad.addColorStop(0.55, '#141821'); grad.addColorStop(1, '#1d1a1a');
  g.fillStyle = grad; g.fillRect(0, 0, 16, 256);
  const t = new THREE.CanvasTexture(c); t.encoding = THREE.sRGBEncoding;
  return t;
}

/**
 * A tiny lab for image-based light: a dark box with a warm window, a cool
 * ceiling panel and a dim teal glow, rendered once into a PMREM.
 */
function labEnvironment() {
  const env = new THREE.Scene();
  const box = new THREE.Mesh(new THREE.BoxGeometry(20, 10, 20), new THREE.MeshBasicMaterial({ color: 0x15171c, side: THREE.BackSide }));
  box.position.y = 3; env.add(box);
  const panel = (/** @type {number} */ color, /** @type {number} */ w, /** @type {number} */ h, /** @type {number[]} */ p, /** @type {number[]} */ r) => {
    const m = new THREE.Mesh(new THREE.PlaneGeometry(w, h), new THREE.MeshBasicMaterial({ color, side: THREE.DoubleSide }));
    m.position.set(p[0], p[1], p[2]); m.rotation.set(r[0], r[1], r[2]); env.add(m);
  };
  panel(0xfff0d8, 6, 4, [-9.9, 4, 3], [0, Math.PI / 2, 0]); // window, left
  panel(0x9fb8ff, 8, 3, [0, 7.9, 0], [Math.PI / 2, 0, 0]); // ceiling softbox
  panel(0x2a6f66, 10, 1.5, [0, 1, -9.9], [0, 0, 0]); // teal strip on the back wall
  panel(0x443322, 20, 20, [0, -1.9, 0], [-Math.PI / 2, 0, 0]); // warm floor bounce
  return env;
}

/** Dark-oak grain: streaks along the board with darker knots of noise. */
function woodTexture() {
  const W = 512, H = 128;
  const c = document.createElement('canvas'); c.width = W; c.height = H;
  const g = /** @type {CanvasRenderingContext2D} */ (c.getContext('2d'));
  g.fillStyle = '#6f4a2c'; g.fillRect(0, 0, W, H);
  let seed = 7; const rnd = () => ((seed = (seed * 16807) % 2147483647) / 2147483647);
  for (let i = 0; i < 140; i++) {
    const y = rnd() * H, a = 0.05 + rnd() * 0.12, dark = rnd() < 0.6;
    g.strokeStyle = dark ? `rgba(40,24,12,${a})` : `rgba(160,112,70,${a})`;
    g.lineWidth = 0.6 + rnd() * 2.2; g.beginPath();
    const f = 0.004 + rnd() * 0.01, amp = 1 + rnd() * 4, ph = rnd() * 6;
    for (let x = 0; x <= W; x += 8) { const yy = y + Math.sin(x * f + ph) * amp; if (x) g.lineTo(x, yy); else g.moveTo(x, yy); }
    g.stroke();
  }
  const t = new THREE.CanvasTexture(c);
  t.encoding = THREE.sRGBEncoding; t.wrapS = THREE.MirroredRepeatWrapping; t.wrapT = THREE.RepeatWrapping; t.repeat.set(2, 1);
  return t;
}
