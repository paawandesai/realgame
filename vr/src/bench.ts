import {
  createSystem,
  CylinderGeometry,
  DoubleSide,
  Entity,
  Grabbed,
  Group,
  LatheGeometry,
  Mesh,
  MeshBasicMaterial,
  MeshStandardMaterial,
  Object3D,
  OneHandGrabbable,
  PokeInteractable,
  PointLight,
  Pressed,
  RayInteractable,
  SphereGeometry,
  Vector2,
  Vector3,
  VisibilityState,
} from '@iwsdk/core';
import { BenchButton, Reagent } from './bench-components.js';
import { atomColor, atomRadius, atomsOf, Bowl, bowlText, Energy, mix, MixResult, NAMES, pretty } from './chem.js';
import { Label } from './label.js';
import { sound } from './sound.js';

/** Reagents on the shelf, left to right. */
export const SHELF = ['H2', 'O2', 'CH4', 'C', 'N2', 'H2O', 'Na', 'Cl2', 'Fe', 'Mg'];

/** Most molecules of one kind the bowl will hold, so a stray pinch spree stays readable. */
const BOWL_MAX = 12;
/** A released orb this close to the bowl's centre (metres) drops in. */
const DROP_RADIUS = 0.11;

// Bench layout, in metres, in the bench's own frame: origin on the desk
// surface, +z towards the player, +y up.
const SHELF_Y = 0.13;
const SHELF_Z = -0.13;
const SHELF_SPAN = 0.62;
const BOWL_POS = new Vector3(0, 0, 0.05);
const BOARD_POS = new Vector3(0, 0.33, -0.2);

const atomGeo = new SphereGeometry(1, 20, 14);
const atomMats = new Map<string, MeshStandardMaterial>();
function atomMat(el: string): MeshStandardMaterial {
  let m = atomMats.get(el);
  if (!m) {
    m = new MeshStandardMaterial({ color: atomColor(el), roughness: 0.35, metalness: 0.05 });
    atomMats.set(el, m);
  }
  return m;
}

/** A ball-and-stick-free molecule: atoms packed around the central one. */
export function moleculeMesh(formula: string, scale = 1): Group {
  const atoms = atomsOf(formula);
  const g = new Group();
  const counts = new Map<string, number>();
  for (const a of atoms) counts.set(a, (counts.get(a) ?? 0) + 1);
  // The centre is the heaviest atom that appears once (C in CH4, O in H2O).
  const center = atoms.length > 2 ? [...counts].find(([el, n]) => n === 1 && el !== 'H')?.[0] : undefined;
  const outer = center ? atoms.filter((a, i) => !(a === center && i === atoms.indexOf(center))) : atoms;
  if (center) {
    const s = new Mesh(atomGeo, atomMat(center));
    s.scale.setScalar(atomRadius(center) * scale);
    g.add(s);
  }
  const n = outer.length;
  outer.forEach((el, i) => {
    const s = new Mesh(atomGeo, atomMat(el));
    const r = atomRadius(el) * scale;
    s.scale.setScalar(r);
    if (center) {
      // Spread on a sphere around the centre (Fibonacci points).
      const d = (atomRadius(center) + r) * scale * 0.82;
      const y = n === 1 ? 0 : 1 - (2 * (i + 0.5)) / n;
      const rad = Math.sqrt(1 - y * y);
      const th = i * 2.399963;
      s.position.set(Math.cos(th) * rad * d, y * d, Math.sin(th) * rad * d);
    } else if (n > 1) {
      // A diatomic pair, or a lone atom.
      s.position.x = (i - (n - 1) / 2) * r * 1.6;
    }
    g.add(s);
  });
  return g;
}

interface ShelfOrb {
  entity: Entity;
  formula: string;
  home: Vector3;
  grabbed: boolean;
  pressed: boolean;
}

interface Flight {
  obj: Object3D;
  from: Vector3;
  to: Vector3;
  t: number;
  done: () => void;
}

export class BenchSystem extends createSystem({
  reagents: { required: [Reagent] },
  heldReagents: { required: [Reagent, Grabbed] },
  pressedReagents: { required: [Reagent, Pressed] },
  pressedButtons: { required: [BenchButton, Pressed] },
}) {
  private root!: Group;
  private rootEntity!: Entity;
  private orbs: ShelfOrb[] = [];
  private bowl: Bowl = {};
  private bowlContents = new Group();
  private bowlLabel!: Label;
  private board!: Label;
  private flights: Flight[] = [];
  private flash!: Mesh;
  private flashLight!: PointLight;
  private flashT = 1;
  private placed = false;
  private tmp = new Vector3();
  private tmp2 = new Vector3();
  private lastResult: MixResult | null = null;

  init(): void {
    this.root = new Group();
    this.rootEntity = this.world.createTransformEntity(this.root);
    this.buildMat();
    this.buildBowl();
    this.buildShelf();
    this.buildButtons();
    this.buildBoard();
    this.placeForDesktop();

    this.queries.heldReagents.subscribe('qualify', (e) => {
      const orb = this.orbFor(e);
      if (orb) orb.grabbed = true;
    });
    this.queries.heldReagents.subscribe('disqualify', (e) => {
      const orb = this.orbFor(e);
      if (!orb) return;
      const obj = e.object3D!;
      obj.getWorldPosition(this.tmp);
      this.bowlWorld(this.tmp2);
      if (this.tmp.distanceTo(this.tmp2) < DROP_RADIUS) this.addToBowl(orb.formula);
      this.sendHome(orb);
    });
    // Point and pinch (or click on desktop) sends the orb flying into the bowl,
    // so the bench also works from a distance and without fine finger control.
    this.queries.pressedReagents.subscribe('qualify', (e) => {
      const orb = this.orbFor(e);
      if (orb) {
        orb.pressed = true;
        orb.grabbed = false;
      }
    });
    this.queries.pressedReagents.subscribe('disqualify', (e) => {
      const orb = this.orbFor(e);
      if (!orb || !orb.pressed) return;
      orb.pressed = false;
      if (orb.grabbed) return;
      this.flyToBowl(orb.formula, e.object3D!);
    });
    this.queries.pressedButtons.subscribe('qualify', (e) => {
      const action = e.getValue(BenchButton, 'action') as string;
      const obj = e.object3D!;
      obj.position.y -= 0.006;
      setTimeout(() => (obj.position.y += 0.006), 140);
      sound.click();
      if (action === 'empty') this.emptyBowl();
      else this.react(action as Energy);
    });

    this.cleanupFuncs.push(
      this.world.visibilityState.subscribe((v) => {
        if (v === VisibilityState.Visible && !this.placed) this.placeInFrontOfHead();
        if (v === VisibilityState.NonImmersive) {
          this.placed = false;
          this.placeForDesktop();
        }
      }),
    );
  }

  update(delta: number, time: number): void {
    // Shelf orbs bob gently so they read as "pick me up".
    for (const o of this.orbs) {
      if (o.grabbed) continue;
      const obj = o.entity.object3D!;
      obj.position.y = o.home.y + Math.sin(time * 1.6 + o.home.x * 9) * 0.004;
      obj.rotation.y += delta * 0.4;
    }
    this.bowlContents.rotation.y += delta * 0.3;
    for (let i = this.flights.length - 1; i >= 0; i--) {
      const f = this.flights[i];
      f.t = Math.min(1, f.t + delta * 2.6);
      const k = f.t * f.t * (3 - 2 * f.t);
      f.obj.position.lerpVectors(f.from, f.to, k);
      f.obj.position.y += Math.sin(Math.PI * f.t) * 0.08;
      if (f.t >= 1) {
        this.flights.splice(i, 1);
        f.done();
      }
    }
    if (this.flashT < 1) {
      this.flashT = Math.min(1, this.flashT + delta * 1.8);
      const s = 0.02 + this.flashT * 0.18;
      this.flash.scale.setScalar(s);
      (this.flash.material as MeshBasicMaterial).opacity = 0.55 * (1 - this.flashT);
      this.flashLight.intensity = 3 * (1 - this.flashT);
      this.flash.visible = this.flashT < 1;
    }
  }

  // ---- placement -------------------------------------------------------

  /** In a headset: a comfortable reach in front of a seated player, just below the eyes. */
  private placeInFrontOfHead(): void {
    const head = this.player.head;
    head.updateWorldMatrix(true, false);
    head.getWorldPosition(this.tmp);
    head.getWorldDirection(this.tmp2);
    // Object3D.getWorldDirection gives +z; the camera looks down -z.
    const fwd = new Vector2(-this.tmp2.x, -this.tmp2.z);
    if (fwd.lengthSq() < 1e-4) fwd.set(0, -1);
    fwd.normalize();
    this.root.position.set(this.tmp.x + fwd.x * 0.42, this.tmp.y - 0.4, this.tmp.z + fwd.y * 0.42);
    this.root.rotation.set(0, Math.atan2(-fwd.x, -fwd.y), 0);
    this.placed = true;
  }

  /** In a desktop browser (no headset) the camera looks down at the bench like a seated player. */
  private placeForDesktop(): void {
    this.root.position.set(0, 0.7, 0);
    this.root.rotation.set(0, 0, 0);
    this.world.camera.position.set(0, 1.12, 0.52);
    this.world.camera.lookAt(0, 0.84, -0.1);
  }

  // ---- building --------------------------------------------------------

  private buildMat(): void {
    // A soft glowing mat marks the bench on the real desk (passthrough shows the desk itself).
    const mat = new Mesh(
      new CylinderGeometry(0.4, 0.4, 0.004, 64),
      new MeshStandardMaterial({ color: 0x1b2a4a, transparent: true, opacity: 0.55, roughness: 0.9 }),
    );
    mat.scale.set(1, 1, 0.55);
    mat.position.y = 0.002;
    this.root.add(mat);
    const rim = new Mesh(
      new CylinderGeometry(0.405, 0.405, 0.002, 64, 1, true),
      new MeshBasicMaterial({ color: 0x6fd3ff, transparent: true, opacity: 0.7, side: DoubleSide }),
    );
    rim.scale.set(1, 1, 0.55);
    rim.position.y = 0.004;
    this.root.add(rim);
  }

  private buildBowl(): void {
    const pts: Vector2[] = [];
    for (let i = 0; i <= 12; i++) {
      const a = (i / 12) * (Math.PI / 2);
      pts.push(new Vector2(0.02 + Math.sin(a) * 0.065, 0.07 - Math.cos(a) * 0.065));
    }
    pts.unshift(new Vector2(0, 0.005));
    const bowl = new Mesh(
      new LatheGeometry(pts, 48),
      new MeshStandardMaterial({
        color: 0x9fd8ff,
        transparent: true,
        opacity: 0.35,
        roughness: 0.05,
        metalness: 0.1,
        side: DoubleSide,
      }),
    );
    bowl.position.copy(BOWL_POS);
    this.root.add(bowl);
    this.bowlContents.position.copy(BOWL_POS).add(new Vector3(0, 0.035, 0));
    this.root.add(this.bowlContents);

    this.bowlLabel = new Label(0.24, 0.045, { bg: 'rgba(10,20,40,0.75)', font: 64 });
    // In front of the bowl, tilted up at a seated player's eyes.
    this.bowlLabel.mesh.position.copy(BOWL_POS).add(new Vector3(0, 0.02, 0.105));
    this.bowlLabel.mesh.rotation.x = -0.9;
    this.root.add(this.bowlLabel.mesh);
    this.bowlLabel.set('empty');

    this.flash = new Mesh(
      new SphereGeometry(1, 24, 16),
      new MeshBasicMaterial({ color: 0xffa040, transparent: true, opacity: 0, depthWrite: false }),
    );
    this.flash.position.copy(BOWL_POS).add(new Vector3(0, 0.05, 0));
    this.flash.visible = false;
    this.root.add(this.flash);
    this.flashLight = new PointLight(0xffa040, 0, 0.8);
    this.flashLight.position.copy(this.flash.position);
    this.root.add(this.flashLight);
  }

  private buildShelf(): void {
    SHELF.forEach((formula, i) => {
      const t = i / (SHELF.length - 1);
      const x = (t - 0.5) * SHELF_SPAN;
      // A gentle arc so every orb is the same reach away.
      const z = SHELF_Z + Math.pow((t - 0.5) * 2, 2) * 0.07;
      const home = new Vector3(x, SHELF_Y, z);
      const obj = moleculeMesh(formula, 1.25);
      // An invisible sphere makes the orb easy to hit with a finger or ray.
      const hit = new Mesh(new SphereGeometry(0.03, 12, 8), new MeshBasicMaterial({ visible: false }));
      obj.add(hit);
      obj.position.copy(home);
      const entity = this.world.createTransformEntity(obj, this.rootEntity);
      entity.addComponent(Reagent, { formula, slot: i });
      entity.addComponent(OneHandGrabbable, { rotate: true, translate: true });
      entity.addComponent(RayInteractable);
      this.orbs.push({ entity, formula, home, grabbed: false, pressed: false });

      const label = new Label(0.07, 0.03, { font: 80, bg: 'rgba(10,20,40,0.7)' });
      label.set(`#${pretty(formula)}\n${NAMES[formula] ?? ''}`);
      label.mesh.position.set(x, SHELF_Y - 0.055, z + 0.005);
      label.mesh.rotation.x = -0.3;
      this.root.add(label.mesh);
    });
  }

  private buildButtons(): void {
    const defs: { action: string; text: string; color: number; z: number }[] = [
      { action: 'spark', text: 'Spark', color: 0xff8a2a, z: -0.02 },
      { action: 'zap', text: 'Zap', color: 0x3fb6ff, z: 0.06 },
      { action: 'empty', text: 'Empty', color: 0x8a8f99, z: 0.14 },
    ];
    for (const d of defs) {
      const btn = new Mesh(
        new CylinderGeometry(0.026, 0.028, 0.016, 32),
        new MeshStandardMaterial({ color: d.color, emissive: d.color, emissiveIntensity: 0.35, roughness: 0.4 }),
      );
      btn.position.set(0.26, 0.012, d.z);
      const e = this.world.createTransformEntity(btn, this.rootEntity);
      e.addComponent(BenchButton, { action: d.action });
      e.addComponent(RayInteractable);
      e.addComponent(PokeInteractable);
      const label = new Label(0.07, 0.022, { font: 110, bg: 'rgba(10,20,40,0.7)' });
      label.set(d.text);
      label.mesh.position.set(0.335, 0.006, d.z);
      label.mesh.rotation.x = -Math.PI / 2;
      this.root.add(label.mesh);
    }
  }

  private buildBoard(): void {
    this.board = new Label(0.52, 0.17, { bg: 'rgba(8,16,34,0.82)', font: 54 });
    this.board.mesh.position.copy(BOARD_POS);
    this.board.mesh.rotation.x = -0.12;
    this.root.add(this.board.mesh);
    this.board.set('#Formula Bench\nPinch a reagent and drop it in the bowl. Then press Spark (heat) or Zap (electricity).');
  }

  // ---- bowl ------------------------------------------------------------

  private orbFor(e: Entity): ShelfOrb | undefined {
    return this.orbs.find((o) => o.entity === e);
  }

  private bowlWorld(out: Vector3): Vector3 {
    this.root.updateWorldMatrix(true, false);
    return out.copy(BOWL_POS).add(new Vector3(0, 0.04, 0)).applyMatrix4(this.root.matrixWorld);
  }

  private sendHome(orb: ShelfOrb): void {
    const obj = orb.entity.object3D!;
    // The grab handle moved the orb in world space; reset it in the bench frame.
    if (obj.parent !== this.root) this.root.attach(obj);
    obj.position.copy(orb.home);
    obj.quaternion.identity();
    orb.grabbed = false;
  }

  private flyToBowl(formula: string, from: Object3D): void {
    const ghost = moleculeMesh(formula, 1.25);
    ghost.position.copy(from.position);
    this.root.add(ghost);
    this.flights.push({
      obj: ghost,
      from: from.position.clone(),
      to: BOWL_POS.clone().add(new Vector3(0, 0.04, 0)),
      t: 0,
      done: () => {
        this.root.remove(ghost);
        this.addToBowl(formula);
      },
    });
  }

  private addToBowl(formula: string): void {
    const n = this.bowl[formula] ?? 0;
    if (n >= BOWL_MAX) {
      this.board.set(`The bowl can hold ${BOWL_MAX} ${pretty(formula)} at most.`);
      return;
    }
    this.bowl[formula] = n + 1;
    sound.drop();
    this.refreshBowl();
  }

  private emptyBowl(): void {
    this.bowl = {};
    this.lastResult = null;
    this.refreshBowl();
    this.board.set('#Bowl emptied\nTry a new mix.');
  }

  private refreshBowl(): void {
    this.bowlLabel.set(bowlText(this.bowl));
    this.bowlContents.clear();
    const items: string[] = [];
    for (const [f, n] of Object.entries(this.bowl)) for (let i = 0; i < n; i++) items.push(f);
    const shown = items.slice(0, 16);
    shown.forEach((f, i) => {
      const m = moleculeMesh(f, 0.8);
      const a = i * 2.399963;
      const r = 0.012 + 0.04 * Math.sqrt((i + 0.5) / Math.max(shown.length, 1));
      m.position.set(Math.cos(a) * r, (i % 3) * 0.008, Math.sin(a) * r);
      this.bowlContents.add(m);
    });
  }

  private react(energy: Energy): void {
    const r = mix(this.bowl, energy);
    this.lastResult = r;
    const color = energy === 'zap' ? 0x6fd3ff : 0xffa040;
    const worked = r.status === 'exact' || r.status === 'partial';
    if (worked) {
      this.bowl = { ...r.left };
      for (const [f, n] of Object.entries(r.made)) this.bowl[f] = (this.bowl[f] ?? 0) + n;
      this.refreshBowl();
      this.burst(color);
      if (r.status === 'exact') sound.success();
      else sound.partial();
    } else {
      this.burst(0x777777);
      sound.fizzle();
    }
    this.board.set(this.describe(r));
  }

  private describe(r: MixResult): string {
    const s = r.spell;
    switch (r.status) {
      case 'exact':
      case 'partial': {
        const dh =
          s?.delta_h_kj != null
            ? s.delta_h_kj < 0
              ? ` Releases ${Math.abs(s.delta_h_kj).toFixed(0)} kJ of heat.`
              : ` Takes in ${s.delta_h_kj.toFixed(0)} kJ of energy.`
            : '';
        const head = r.status === 'exact' ? 'Perfect ratio!' : 'It worked, with leftovers';
        return `#${head}\n${pretty(s!.equation)}\n${s!.name}.${dh}`;
      }
      case 'short':
        return `#Almost!\n${pretty(r.message.replace(/^Almost! /, ''))}`;
      case 'wrongenergy':
        return `#Nothing happens\n${pretty(r.message.replace(/^Nothing happens. /, ''))}`;
      case 'nothing':
        return `#No reaction\n${pretty(r.message)}`;
      default:
        return `#Bowl is empty\n${r.message}`;
    }
  }

  private burst(color: number): void {
    (this.flash.material as MeshBasicMaterial).color.setHex(color);
    this.flashLight.color.setHex(color);
    this.flashT = 0;
    this.flash.visible = true;
  }

  /** For tests and the debug console. */
  get state(): { bowl: Bowl; last: MixResult | null } {
    return { bowl: { ...this.bowl }, last: this.lastResult };
  }
}
