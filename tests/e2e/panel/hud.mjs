// Everything the harness knows about the client's page, in one place: HUD
// selectors, the state global, console line formats. If PR #13 renames any of
// these, update them here (and note it in the round notes).
export const SEL = {
  canvas: '#game',
  objective: '#objective',
  objectiveTitle: '#objective .title',
  warning: '#warning',
  crosshair: '#crosshair',
  hint: '#hint',
  caption: '#caption',
  hotbar: '#hotbar',
  slot: '#hotbar .slot',
  selectedClass: 'sel',
  crosshairOnClass: 'on',
  banner: '#banner',
  pause: '#pause',
  resume: '#resume',
  pointmode: '#pointmode',
  loading: '#loading',
};

// The game writes its state as a JSON string here every frame (src/bridge.rs).
export const STATE_GLOBAL = '__gameState';

export const CONSOLE = {
  stats: /^STATS /,
  cpu: /^CPUFRAME /,
  firstFrame: /^FIRST_FRAME /,
  panic: /panicked/,
};

// Runs in the page. Returns only what a player can see: text of visible HUD
// elements (hidden, zero-opacity or zero-size elements give null), which
// hotbar slot is highlighted, whether the crosshair is lit, the mouse cursor
// shape and whether the mouse is captured. Never reads the game state.
export function readView(sel) {
  const visible = (el) => {
    if (!el) return false;
    const r = el.getBoundingClientRect();
    if (r.width === 0 || r.height === 0) return false;
    for (let e = el; e && e.nodeType === 1; e = e.parentElement) {
      const cs = getComputedStyle(e);
      if (cs.display === 'none' || cs.visibility === 'hidden' || parseFloat(cs.opacity) < 0.05) return false;
    }
    return true;
  };
  const text = (q) => {
    const el = document.querySelector(q);
    if (!visible(el)) return null;
    const t = el.innerText.replace(/\s+/g, ' ').trim();
    return t || null;
  };
  const slots = [...document.querySelectorAll(sel.slot)].filter(visible);
  const label = (el) => el.innerText.replace(/\s+/g, ' ').trim();
  const selSlot = slots.find((s) => s.classList.contains(sel.selectedClass));
  const cross = document.querySelector(sel.crosshair);
  const locked = !!document.pointerLockElement;
  const canvas = document.querySelector(sel.canvas);
  return {
    hud: {
      objective: text(sel.objective),
      warning: text(sel.warning),
      hint: text(sel.hint),
      caption: text(sel.caption),
      banner: text(sel.banner),
      pause: text(sel.pause),
      loading: text(sel.loading),
      hotbar: { selected: selSlot ? label(selSlot) : null, slots: slots.map(label) },
      crosshair_on: visible(cross) && cross.classList.contains(sel.crosshairOnClass),
      cursor: locked ? 'hidden' : (canvas ? getComputedStyle(canvas).cursor : null),
    },
    pointer_locked: locked,
    quest: (document.querySelector(sel.objectiveTitle) || {}).textContent || null,
  };
}

// The parts of the game state the log keeps after each turn (truth summary).
export function truthSummary(s, quest) {
  return {
    pos: s.pos,
    yaw: s.yaw,
    pitch: s.pitch,
    casts: s.casts,
    impacts: s.impacts,
    captionId: s.captionId,
    caption: s.caption,
    outcome: s.outcome,
    burning: s.burning,
    extinguished: s.extinguished,
    locked: s.locked,
    paused: s.paused,
    spell: s.spell,
    // One digit group per object: burning, wet, frozen, wall, smothered.
    chem: (s.objs || []).map((o) => `${+!!o.burning}${o.wet > 0.05 ? 1 : 0}${+!!o.frozen}${+!!o.wall}${o.smothered > 0 ? 1 : 0}`).join('.'),
    quest,
  };
}
