// HUD and input glue for the first-person prototype. See src/bridge.rs.
//
// window.__gameIn  (JS -> game): raw mouse deltas, lock and pause state, commands.
// window.__gameState (game -> JS): JSON string written every frame.
// window.__game: a small debug and test hook:
//   __game.state()          parsed state
//   __game.cmd('select 2')  commands: select N, cast, fire (a click: a queued
//                           cast), tap FX FY (fractions of
//                           the canvas, 0..1), look YAW PITCH,
//                           turn DX DY, face I, tp X Z, move RIGHT FWD SECS,
//                           tick N, reset
//   __game.lock()           request pointer lock (needs a user gesture)
//   __game.mode()           { pointMode, lockDenied, dragging }
//   __game.lastSound()      { name, n, played, peak, frame } of the last sound asked for
//   __game.hudState()       the state the HUD last drew (the DOM matches it)
// Test commands other than select, cast, fire, tap, reset and set need `?debug`.
//
// Settings (pause menu): look speed, view width, hand sway, aim help, hints,
// sound and text size. They apply live (`set sens|hfov|sway|assist` for the
// game), persist in localStorage['alchemist.settings.v1'], and the URL
// (`?sens=`, `?assist=`) beats the saved value, which beats the default.

const $ = (id) => document.getElementById(id);
const params = new URLSearchParams(location.search);
const STORE_KEY = 'alchemist.settings.v1';
// Shown after 45 s with no progress (GS-24). Tied to the fire quest: a new
// quest brings its own idle hint and slot.
const IDLE_MS = 45000;
const IDLE_HINT = 'Look for a fire. Press 2 for Water, aim at the fire and click.';
const IDLE_SLOT = 1;
// The locked hint, until the first fire is out.
const AIM_HINT = 'Aim at a fire and click.';
const START_HINT = 'Click to look around. Esc shows the controls and point-and-click mode.';
// Flame icons in the objective line, at most this many (then a plus sign).
const MAX_PIPS = 8;
// How long each hit marker stays, ms.
const MARK_MS = { hit: 250, fizzle: 300, out: 700 };
// Loudest any tone gets (about -16 dBFS): a lab of 30 must not be startled.
const PEAK = 0.15;

// Which hint shows, highest first (docs/loop/items/I-025/spec.md, item 3;
// supersedes I-019 item 9): banner > idle > note > locked > start. Nothing
// under a banner; the idle hint when it is due (the player is stuck, and it
// is newer than any note); a one-off note; how to aim (locked, until the
// first fire is out); how to start. No Hints input: the Hints setting gates
// the result downstream (setHint), and `note` comes back unchanged so status
// notes stay with Hints off.
function pickHint({ playing, idle, note, locked, extinguished, casts }) {
  if (!playing) return '';
  if (idle) return IDLE_HINT;
  if (note) return note;
  if (locked) return extinguished === 0 ? AIM_HINT : '';
  return casts > 0 ? '' : START_HINT;
}

const reducedMotion = () => {
  try { return matchMedia('(prefers-reduced-motion: reduce)').matches; } catch (e) { return false; }
};

// Effective settings: URL, then saved, then default. Every storage access
// can throw (private windows, blocked site data), so each is guarded.
function loadSettings() {
  const s = { sens: 1, hfov: 90, sway: !reducedMotion(), assist: true, hints: true, sound: true, text: 1 };
  let saved = null;
  try { saved = JSON.parse(localStorage.getItem(STORE_KEY) || 'null'); } catch (e) { saved = null; }
  if (saved && typeof saved === 'object') {
    const num = (v, lo, hi) => (typeof v === 'number' && Number.isFinite(v) ? Math.min(hi, Math.max(lo, v)) : null);
    const sens = num(saved.sens, 0.25, 3); if (sens !== null) s.sens = sens;
    const hfov = num(saved.hfov, 70, 110); if (hfov !== null) s.hfov = Math.round(hfov / 5) * 5;
    for (const k of ['sway', 'assist', 'hints', 'sound']) if (typeof saved[k] === 'boolean') s[k] = saved[k];
    if ([1, 1.5, 2].includes(saved.text)) s.text = saved.text;
  }
  const us = parseFloat(params.get('sens'));
  if (Number.isFinite(us) && us > 0) s.sens = Math.min(3, Math.max(0.25, us));
  if (params.has('assist')) s.assist = !['0', 'off', 'false'].includes(params.get('assist'));
  return s;
}
function saveSettings(s) {
  try { localStorage.setItem(STORE_KEY, JSON.stringify(s)); } catch (e) { /* storage blocked: settings last this visit only */ }
}

// Short synthesised sounds (WebAudio, no files). The context starts on the
// first key or button press, as browsers require.
function makeSound() {
  let ctx = null;
  const last = { name: '', n: 0, played: false, peak: 0, frame: -1 };
  const start = () => {
    if (ctx) { if (ctx.state === 'suspended') ctx.resume().catch(() => {}); return; }
    try {
      const AC = window.AudioContext || window.webkitAudioContext;
      if (AC) { ctx = new AC(); ctx.resume().catch(() => {}); }
    } catch (e) { ctx = null; }
  };
  window.addEventListener('pointerdown', start, { capture: true });
  window.addEventListener('keydown', start, { capture: true });
  // Each note: [type, from Hz, to Hz, start s, length s, peak gain].
  const SHAPES = {
    cast: [['sine', 520, 820, 0, 0.06, 0.08]],
    hit: [['triangle', 660, 700, 0, 0.06, 0.12], ['triangle', 880, 940, 0.06, 0.06, 0.12]],
    fizzle: [['square', 220, 110, 0, 0.15, 0.05]],
    out: [['triangle', 523, 530, 0, 0.14, PEAK], ['triangle', 659, 665, 0.15, 0.14, PEAK], ['triangle', 784, 790, 0.3, 0.15, PEAK]],
    miss: [['sine', 150, 60, 0, 0.08, 0.12]],
  };
  const play = (name, on, frame) => {
    const notes = SHAPES[name];
    last.name = name;
    last.n += 1;
    last.frame = frame;
    last.peak = Math.min(PEAK, Math.max(...notes.map((x) => x[5])));
    last.played = false;
    if (!on || !ctx) return;
    try {
      const t0 = ctx.currentTime + 0.005;
      for (const [type, f0, f1, at, len, peak] of notes) {
        const o = ctx.createOscillator();
        const g = ctx.createGain();
        o.type = type;
        o.frequency.setValueAtTime(f0, t0 + at);
        o.frequency.exponentialRampToValueAtTime(f1, t0 + at + len);
        g.gain.setValueAtTime(0.0001, t0 + at);
        g.gain.exponentialRampToValueAtTime(Math.min(PEAK, peak), t0 + at + Math.min(0.01, len / 4));
        g.gain.exponentialRampToValueAtTime(0.0001, t0 + at + len);
        o.connect(g).connect(ctx.destination);
        o.start(t0 + at);
        o.stop(t0 + at + len + 0.02);
      }
      last.played = true;
    } catch (e) { last.played = false; }
  };
  return { play, last: () => ({ ...last }) };
}

export function startHud() {
  const canvas = $('game');
  const input = { dx: 0, dy: 0, locked: false, paused: false, fire: false, px: -1, py: -1, cmds: [] };
  window.__gameIn = input;
  let state = null;
  // The state the HUD has finished drawing (for tests that compare the DOM
  // with the state it shows).
  let drawn = null;
  let lastRaw = '';
  let everLocked = false;
  // "Point and click mode" (from the pause menu): mouse clicks cast at the
  // target under the cursor instead of locking. Touch always works this way.
  let pointMode = false;
  // Set when the browser refuses pointer lock: hold a mouse button and drag
  // to look instead.
  let lockDenied = false;
  // A mouse button held while unlocked: drag-to-look.
  let drag = null;
  // A one-off hint (lock refused, point-and-click) that replaces the usual
  // ones until the pointer locks.
  let note = '';
  // The spell the wheel last asked for, until the game reports it (two wheel
  // ticks in one frame must not pick the same spell).
  let wheelSpell = null;
  // The last few captions, for the pause card and the C key.
  const history = [];

  // Render scale: `?low` renders at 0.75, `?scale=0.5` etc. The canvas is laid
  // out smaller (so its drawing buffer is smaller) and scaled up by CSS.
  const low = params.has('low') && !['0', 'false'].includes(params.get('low'));
  let scale = parseFloat(params.get('scale') || (low ? '0.75' : '1'));
  if (!(scale >= 0.25 && scale <= 1)) scale = 1;
  if (scale < 1) {
    // Bevy sets the canvas itself to 100% of its parent, so scale the parent.
    const stage = $('stage');
    stage.style.right = 'auto';
    stage.style.bottom = 'auto';
    stage.style.width = `${scale * 100}vw`;
    stage.style.height = `${scale * 100}vh`;
    stage.style.transform = `scale(${1 / scale})`;
  }

  const cmd = (c) => input.cmds.push(c);
  // The lock request in flight: settled by the promise requestPointerLock
  // returns (current Chrome and Firefox), or else by the pointerlockchange /
  // pointerlockerror events (Safari and older Firefox return undefined and
  // only report a refusal through pointerlockerror).
  let pending = null;
  // A short click made while the lock was refused before and a new request is
  // still open: it casts only if that request is refused too.
  let castIfRefused = false;
  const settle = (ok) => {
    const p = pending;
    pending = null;
    if (p) (ok ? p.resolve : p.reject)();
  };
  document.addEventListener('pointerlockerror', () => { if (pending && !pending.promised) settle(false); });
  const tryLock = (opts) => new Promise((resolve, reject) => {
    pending = { resolve, reject, promised: false };
    try {
      const p = opts ? canvas.requestPointerLock(opts) : canvas.requestPointerLock();
      if (p && typeof p.then === 'function') {
        pending.promised = true;
        p.then(() => settle(true), () => settle(false));
      } else {
        // No promise and no event within 1.5 s: treat it as refused.
        const mine = pending;
        setTimeout(() => { if (pending === mine) settle(false); }, 1500);
      }
    } catch (e) {
      settle(false);
    }
  });
  const lock = () => {
    canvas.focus();
    pointMode = false;
    // Raw, unaccelerated deltas where supported; plain lock otherwise.
    tryLock({ unadjustedMovement: true })
      .catch(() => tryLock())
      .then(() => { lockDenied = false; note = ''; castIfRefused = false; })
      .catch(() => {
        if (castIfRefused) cmd('cast');
        castIfRefused = false;
        if (everLocked) {
          // Browsers refuse a new lock for about a second after Esc.
          note = 'Click again to lock the mouse.';
          return;
        }
        // Blocked (browser setting, iframe, or no user gesture): drag to look.
        lockDenied = true;
        note = 'Mouse lock is blocked here: hold a mouse button and drag to look. Click to cast.';
      });
  };
  const sound = makeSound();
  const settings = loadSettings();
  // The idle clock (GS-24): restarts on progress, reset, pause and resume.
  let idleSince = performance.now();
  const showPause = (on) => {
    if (on !== input.paused) idleSince = performance.now();
    input.paused = on;
    $('pause').classList.toggle('hidden', !on);
    if (on) {
      // Recent messages, newest first, so nothing is missed.
      const list = $('history');
      list.replaceChildren(...history.map((t) => {
        const li = document.createElement('li');
        li.textContent = t;
        return li;
      }));
      $('history-box').classList.toggle('hidden', history.length === 0);
    }
  };
  window.__game = {
    state: () => (window.__gameState ? JSON.parse(window.__gameState) : null),
    cmd,
    lock,
    pause: showPause,
    mode: () => ({ pointMode, lockDenied, dragging: !!drag }),
    lastSound: sound.last,
    hudState: () => drawn,
    pickHint,
  };

  // Polled every frame as well as on the event (some browsers, headless
  // Chromium among them, do not always fire pointerlockchange).
  const syncLock = () => {
    const locked = document.pointerLockElement === canvas;
    if (locked === input.locked) return;
    input.locked = locked;
    input.fire = false;
    if (locked) {
      everLocked = true;
      settle(true);
      // A lock that arrives late (after the refusal timeout) still wins.
      lockDenied = false;
      note = '';
      castIfRefused = false;
      showPause(false);
    } else if (everLocked && !(state && state.outcome !== 'playing')) {
      // Esc released the mouse: pause with the controls help.
      showPause(true);
    }
  };
  document.addEventListener('pointerlockchange', syncLock);
  // One event's movement: a number, and not one of the huge jumps some
  // browsers report on the first event after the lock.
  const delta = (v) => (Number.isFinite(v) ? Math.max(-200, Math.min(200, v)) : 0);
  // An event's movement, including the moves the browser merged into it
  // (with a button held, Chrome merges pointer moves to one per frame, and
  // the merged event's own movementX can hold only the last of them, which
  // made aiming while holding fire turn far too slowly). When the parts
  // are there, their sum is used (clamped like a single event).
  const movement = (e) => {
    const parts = typeof e.getCoalescedEvents === 'function' ? e.getCoalescedEvents() : [];
    if (parts.length > 1) {
      let sx = 0;
      let sy = 0;
      for (const c of parts) { sx += delta(c.movementX); sy += delta(c.movementY); }
      return [delta(sx), delta(sy)];
    }
    return [delta(e.movementX), delta(e.movementY)];
  };
  // Set once pointermove has reported movement while locked.
  let moveWorks = false;
  // Where the event is on the canvas, as fractions (0..1) of its width and
  // height: the game builds the ray from these, whatever the pixel scale.
  const local = (e) => {
    const r = canvas.getBoundingClientRect();
    return [(e.clientX - r.left) / r.width, (e.clientY - r.top) / r.height];
  };
  // All mouse movement is read here, from pointermove in the capture phase
  // on window, and then stopped so it never reaches winit's listeners on the
  // canvas. Two reasons:
  // - mousemove is not usable: winit calls preventDefault on pointerdown,
  //   and per the Pointer Events spec the browser then sends no compatibility
  //   mouse events (mousemove included) while a button is held. Drag-to-look,
  //   aiming while holding fire, and hover all froze that way.
  // - headless Chromium was seen to deliver tens of thousands of coalesced
  //   moves per frame to winit around pointer lock, growing the WebAssembly
  //   heap by hundreds of MB. The game never reads Bevy's cursor events.
  const onMove = (e) => {
    e.stopImmediatePropagation();
    const [dx, dy] = movement(e);
    if (input.locked) {
      if (dx || dy) moveWorks = true;
      input.dx += dx;
      input.dy += dy;
      return;
    }
    if (drag) {
      // Drag-to-look: works with no pointer lock at all.
      input.dx += dx;
      input.dy += dy;
      drag.moved += Math.abs(dx) + Math.abs(dy);
    }
    // The free cursor position, for hover and click-to-target. Checked
    // against the canvas rectangle (not e.target), so synthetic events
    // count; the hotbar and the pause card do not.
    const r = canvas.getBoundingClientRect();
    const inside = e.clientX >= r.left && e.clientX < r.right && e.clientY >= r.top && e.clientY < r.bottom;
    const overUi = !!(e.target && e.target.closest && e.target.closest('#hotbar, #pause'));
    if (inside && !overUi && !input.paused) [input.px, input.py] = local(e);
    else { input.px = -1; input.py = -1; }
  };
  window.addEventListener('pointermove', onMove, { capture: true });
  // Fallback for browsers whose pointermove carries no movement under lock:
  // mousemove deltas count only until pointermove has reported any movement
  // (never both, so nothing is counted twice).
  document.addEventListener('mousemove', (e) => {
    if (!input.locked || moveWorks) return;
    input.dx += delta(e.movementX);
    input.dy += delta(e.movementY);
  });
  // The cursor left the window: nothing is hovered.
  document.documentElement.addEventListener('pointerleave', () => { input.px = -1; input.py = -1; });
  // pointerrawupdate carries the same moves again, and the browser only
  // produces it while someone listens. winit listens (for raw motion the
  // game never reads on the web), so listeners for it are not registered at
  // all: in headless Chromium under pointer lock, its mere presence grew the
  // renderer process by hundreds of MB a second until the tab crashed.
  // (startHud runs before the WebAssembly starts, so this covers winit.)
  const addListener = EventTarget.prototype.addEventListener;
  EventTarget.prototype.addEventListener = function (type, ...rest) {
    if (type === 'pointerrawupdate') return undefined;
    return addListener.call(this, type, ...rest);
  };
  // While locked, buttons and the wheel are read here (not through winit) so
  // casting never depends on how the browser routes a locked pointer's events.
  // Each locked left press also sends one `fire` (a queued cast), so a click
  // whose press and release both fall between two game frames still casts
  // once; holding keeps casting through `input.fire`. No isTrusted check:
  // the tests click with synthetic events.
  document.addEventListener('pointerdown', (e) => {
    if (!input.locked || e.button !== 0) return;
    input.fire = true;
    cmd('fire');
  });
  document.addEventListener('pointerup', (e) => { if (e.button === 0) input.fire = false; });
  document.addEventListener('wheel', (e) => {
    if (!input.locked || e.deltaY === 0) return;
    const cur = wheelSpell ?? (state ? state.spell : 0);
    wheelSpell = (cur + (e.deltaY > 0 ? 1 : 3)) % 4;
    cmd(`select ${wheelSpell + 1}`);
  }, { passive: true });
  // Right-click must never open a menu (that drops the lock) or cast.
  document.addEventListener('contextmenu', (e) => e.preventDefault());
  canvas.addEventListener('pointerdown', (e) => {
    canvas.focus();
    if (input.locked || input.paused) return;
    const [x, y] = local(e);
    if (e.pointerType !== 'mouse') {
      // Touch or pen: tap a target to cast at it.
      cmd(`tap ${x.toFixed(4)} ${y.toFixed(4)}`);
      return;
    }
    if (e.button !== 0 && e.button !== 2) return;
    // Any held left or right button drags to look until it is released.
    drag = { button: e.button, x, y, moved: 0 };
    // A left click always asks for the lock (the usual way to play), unless
    // the player chose point-and-click mode. If the browser refused it
    // before, it is asked again (a short click then also casts, below).
    if (e.button === 0 && !pointMode) lock();
  });
  document.addEventListener('pointerup', (e) => {
    if (!drag || e.button !== drag.button) return;
    const d = drag;
    drag = null;
    if (input.locked || d.button !== 0 || d.moved > 6) return;
    if (pointMode) {
      // Point and click: cast at the target under the cursor.
      cmd(`tap ${d.x.toFixed(4)} ${d.y.toFixed(4)}`);
    } else if (lockDenied) {
      // No lock available: a click casts at the crosshair. If this click's
      // own lock request is still open, cast only once it is refused.
      if (pending) castIfRefused = true;
      else cmd('cast');
    }
  });
  const caption = $('caption');
  let capTimer = 0;
  // Captions stay 8 s (long science lines take young readers a while).
  const showCaption = (text) => {
    caption.textContent = text;
    caption.classList.add('show');
    clearTimeout(capTimer);
    capTimer = setTimeout(() => {
      caption.classList.remove('show');
      const pic = document.getElementById('capicon');
      if (pic) pic.classList.add('hidden');
    }, 8000);
  };
  // Pause now: free the mouse and show the pause card in the same handler
  // (under automation Esc does not release the lock by itself; the later
  // pointerlockchange then finds the game already paused).
  const pauseNow = () => {
    if (document.pointerLockElement) {
      try { document.exitPointerLock(); } catch (err) { /* nothing to free */ }
    }
    showPause(true);
  };
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      if (input.locked) pauseNow();
      else showPause(!input.paused);
    }
    // P (by key position, so AZERTY works too) pauses, and resumes like the
    // Resume button; if the lock is refused the game still unpauses.
    if (e.code === 'KeyP' && !e.repeat) {
      if (input.paused) { showPause(false); lock(); } else pauseNow();
    }
    // C shows the last message again.
    if ((e.key === 'c' || e.key === 'C') && !e.repeat && history.length) showCaption(history[0]);
  });
  $('resume').addEventListener('click', () => { showPause(false); lock(); });
  $('pointmode').addEventListener('click', () => {
    showPause(false);
    pointMode = true;
    note = 'Point and click: click a glowing target to cast at it. Drag to look.';
    canvas.focus();
  });
  document.querySelectorAll('.slot').forEach((b) => {
    b.addEventListener('pointerdown', (e) => { e.stopPropagation(); cmd(`select ${b.dataset.spell}`); canvas.focus(); });
  });

  const slots = [...document.querySelectorAll('.slot')];
  const cross = $('crosshair');
  const hudEl = $('hud');
  const hint = $('hint');
  // Hints off (settings): coaching hints are hidden. Status notes (`note`)
  // and the start line (unlocked, nothing cast yet) stay: they say how to
  // play at all, not how to play better. The idle hint is gated in updateIdle.
  // The start line is judged from `state.locked`, the same source frame()
  // picks the hint from, so the frame between Esc and Bevy's next state
  // cannot let the locked aim hint through (I-044).
  const hintAllowed = (text) => settings.hints || !text || text === note
    || (!!state && !state.locked && state.casts === 0);
  const setHint = (text) => {
    const shown = hintAllowed(text) ? text : '';
    if (hint.textContent !== shown) hint.textContent = shown;
    hint.classList.toggle('hidden', !shown);
  };

  // Settings block, built here (the page's CSP allows no inline styles and
  // index.html stays as it is), just before the Resume button. Native
  // controls, so it works from the keyboard too.
  const el = (tag, props) => Object.assign(document.createElement(tag), props);
  const box = el('div', { id: 'settings' });
  box.append(el('h2', { textContent: 'Settings' }));
  const row = (name, control, value) => {
    const label = el('label', { className: `set-row ${control.type === 'checkbox' ? 'check' : ''}` });
    label.append(el('span', { className: 'set-name', textContent: name }), control);
    if (value) label.append(value);
    box.append(label);
  };
  const sensIn = el('input', { type: 'range', id: 'sens', min: '0.25', max: '3', step: '0.05' });
  const sensVal = el('span', { id: 'sens-val', className: 'set-val' });
  const fovIn = el('input', { type: 'range', id: 'fov', min: '70', max: '110', step: '5' });
  const fovVal = el('span', { id: 'fov-val', className: 'set-val' });
  const checks = {};
  for (const [k, id] of [['sway', 'sway'], ['assist', 'assist'], ['hints', 'hints'], ['sound', 'sound']]) {
    checks[k] = el('input', { type: 'checkbox', id });
  }
  const textIn = el('select', { id: 'textsize' });
  for (const [v, t] of [['1', '100%'], ['1.5', '150%'], ['2', '200%']]) textIn.append(el('option', { value: v, textContent: t }));
  row('Look speed', sensIn, sensVal);
  row('View width', fovIn, fovVal);
  row('Hand sway', checks.sway);
  row('Aim help', checks.assist);
  row('Hints', checks.hints);
  row('Sound', checks.sound);
  row('Text size', textIn);
  $('resume').before(box);
  const showSettings = () => {
    sensIn.value = String(settings.sens);
    sensVal.textContent = settings.sens.toFixed(2);
    fovIn.value = String(settings.hfov);
    fovVal.textContent = `${settings.hfov}°`;
    for (const k of Object.keys(checks)) checks[k].checked = settings[k];
    textIn.value = String(settings.text);
    document.documentElement.style.setProperty('--ui', String(settings.text));
  };
  // The game's share of the settings, as commands.
  const send = (k) => {
    if (k === 'sens') cmd(`set sens ${settings.sens}`);
    if (k === 'hfov') cmd(`set hfov ${settings.hfov}`);
    if (k === 'sway') cmd(`set sway ${settings.sway ? 1 : 0}`);
    if (k === 'assist') cmd(`set assist ${settings.assist ? 1 : 0}`);
  };
  const changed = (k) => { showSettings(); send(k); saveSettings(settings); };
  sensIn.addEventListener('input', () => {
    const v = parseFloat(sensIn.value);
    if (Number.isFinite(v)) settings.sens = Math.min(3, Math.max(0.25, v));
    changed('sens');
  });
  fovIn.addEventListener('input', () => {
    const v = parseFloat(fovIn.value);
    if (Number.isFinite(v)) settings.hfov = Math.min(110, Math.max(70, Math.round(v / 5) * 5));
    changed('hfov');
  });
  for (const k of Object.keys(checks)) {
    checks[k].addEventListener('change', () => {
      settings[k] = checks[k].checked;
      changed(k);
      if (k === 'hints' && !settings.hints) updateIdle(performance.now());
    });
  }
  textIn.addEventListener('change', () => {
    const v = parseFloat(textIn.value);
    if ([1, 1.5, 2].includes(v)) settings.text = v;
    changed('text');
  });
  showSettings();

  // Crosshair: a ring that empties over the cooldown, and a short pulse on
  // each cast (kept for at least two HUD frames, so it is always drawn).
  cross.append(el('b', { className: 'cd' }));
  let pulseAt = null;
  const pulse = (frame) => {
    cross.classList.remove('pulse');
    void cross.offsetWidth; // restart the animation
    cross.classList.add('pulse');
    pulseAt = { t: performance.now(), frame };
  };

  // Hit marker: X for a hit, slashed circle for a fizzle, a big star for a
  // fire put out. A miss shows none (it has its own sound).
  const hm = el('div', { id: 'hitmarker' });
  const hx = el('div', { className: 'hm-x' });
  for (let k = 0; k < 4; k++) hx.append(el('i', {}));
  hm.append(hx, el('div', { className: 'hm-slash' }), el('div', { className: 'hm-star' }));
  $('hud').append(hm);

  // The pause card's controls table gains the pause keys.
  {
    const table = document.querySelector('#pause table');
    if (table) {
      const tr = el('tr', {});
      tr.append(el('td', { textContent: 'Pause' }), el('td', { textContent: 'Esc or P' }));
      (table.tBodies[0] || table).append(tr);
    }
  }

  // The arrow to the nearest fire when it is off screen: a chevron and a
  // flame at the left or right edge, no text (GS-21, GS-24).
  const finder = el('div', { id: 'firefinder', className: 'hidden left' });
  finder.append(el('span', { className: 'ff-chev' }), el('span', { className: 'ff-flame' }));
  $('hud').append(finder);
  // I-035: the Sprout picture next to a level-2 caption (snowflake, flame on
  // a drop, or a cross), from the state's capIcon.
  const capIcon = el('div', { id: 'capicon', className: 'hidden', ariaHidden: 'true' });
  $('hud').append(capIcon);

  // The objective: one number, and a flame per fire left (for players who
  // cannot read yet). The plus sign after MAX_PIPS is drawn by CSS, so the
  // line's text stays "Fires to put out: N".
  const objLine = $('obj-line');
  const objText = document.createTextNode('');
  const pips = el('span', { className: 'fire-pips' });
  objLine.replaceChildren(objText, pips);
  let pipCount = -1;
  let pipMore = null;
  const setObjective = (text, n) => {
    if (objText.data !== text) objText.data = text;
    const shown = Math.min(n, MAX_PIPS);
    const more = n > MAX_PIPS;
    if (shown === pipCount && more === pipMore) return;
    pipCount = shown;
    pipMore = more;
    const kids = [];
    for (let k = 0; k < shown; k++) kids.push(el('span', { className: 'fire-pip' }));
    if (more) kids.push(el('span', { className: 'fire-more' }));
    pips.replaceChildren(...kids);
  };
  let hmTimer = 0;
  const mark = (kind, s) => {
    const r = canvas.getBoundingClientRect();
    let x = r.left + r.width / 2;
    let y = r.top + r.height / 2;
    // Pointer free (point and click): at the impact. Locked: at the crosshair.
    if (!s.locked && s.hitX >= 0 && s.hitY >= 0) {
      x = r.left + s.hitX * r.width;
      y = r.top + s.hitY * r.height;
    }
    hm.style.left = `${x.toFixed(1)}px`;
    hm.style.top = `${y.toFixed(1)}px`;
    hm.dataset.frame = String(s.frame);
    hm.className = `show kind-${kind}`;
    clearTimeout(hmTimer);
    hmTimer = setTimeout(() => hm.classList.remove('show'), MARK_MS[kind]);
    hmShown = { t: performance.now(), ms: MARK_MS[kind] };
  };
  // On slow frames the timer can run late: a HUD frame also takes the marker
  // down when the next frame would be closer to its end time than this one.
  let hmShown = null;
  let lastFrameAt = performance.now();
  const expireMark = (now, dt) => {
    if (!hmShown) return;
    if (now - hmShown.t + dt / 2 >= hmShown.ms || !hm.classList.contains('show')) {
      hm.classList.remove('show');
      hmShown = null;
    }
  };

  // Idle hint (GS-24): after 45 s of play with no progress, say what to do
  // and make the Water slot glow (for players who cannot read yet).
  let idleOn = false;
  function updateIdle(now) {
    const on = !!state && settings.hints && state.level === 1 && state.outcome === 'playing' && !state.paused && !input.paused && now - idleSince >= IDLE_MS;
    if (on === idleOn) return;
    idleOn = on;
    slots[IDLE_SLOT].classList.toggle('nudge', on);
    if (!on && hint.textContent === IDLE_HINT) setHint('');
  }

  let prev = null;
  let capId = -1;
  // I-035: game time of the level-1 win (null while not won), for the exit arrow.
  let wonAt = null;
  const EXIT_ARROW_DELAY_S = 3;
  const frame = () => {
    syncLock();
    const raw = window.__gameState;
    if (raw && raw !== lastRaw) {
      lastRaw = raw;
      state = JSON.parse(raw);
      $('loading').classList.add('hidden');
      $('hud').classList.remove('hidden');
      const now = performance.now();
      expireMark(now, now - lastFrameAt);
      lastFrameAt = now;
      if (!prev) {
        // The game booted with the URL's settings only: send the effective ones.
        for (const k of ['sens', 'hfov', 'sway', 'assist']) send(k);
      } else {
        // Feedback in the same HUD frame the change is first seen (GS-12, GS-16).
        if (state.casts > prev.casts) {
          pulse(state.frame);
          sound.play('cast', settings.sound, state.frame);
        }
        if (state.impacts > prev.impacts && MARK_MS[state.hitKind] !== undefined) mark(state.hitKind, state);
        if (state.impacts > prev.impacts && state.hitKind !== 'none') sound.play(state.hitKind, settings.sound, state.frame);
        // Progress, a reset, or a pause restarts the idle clock.
        if (state.impacts !== prev.impacts || state.extinguished !== prev.extinguished || state.tick < prev.tick
          || state.casts < prev.casts || state.paused !== prev.paused) idleSince = now;
      }
      prev = state;
      if (pulseAt && now - pulseAt.t >= 150 && state.frame > pulseAt.frame + 1) {
        cross.classList.remove('pulse');
        pulseAt = null;
      }
      const cd = state.cooldownMax > 0 ? Math.min(1, Math.max(0, state.cooldown / state.cooldownMax)) : 0;
      cross.style.setProperty('--cd', cd.toFixed(3));
      cross.classList.toggle('cooling', state.cooldown > 0);
      slots.forEach((s, i) => s.classList.toggle('sel', i === state.spell));
      if (wheelSpell === state.spell) wheelSpell = null;
      const burningAt = (i) => i >= 0 && !!state.objs[i] && state.objs[i].burning;
      cross.classList.toggle('on', state.aimObj >= 0 && state.locked);
      // A second, outer ring when the target is on fire (a shape cue, GS-19).
      cross.classList.toggle('fire', state.locked && burningAt(state.aimObj));
      const hovering = pointMode && !state.locked && state.hover >= 0;
      canvas.classList.toggle('hover', hovering);
      canvas.classList.toggle('hover-fire', hovering && burningAt(state.hover));
      // Hints: banner > idle > note > locked > start (pickHint, I-025).
      const playing = state.outcome === 'playing';
      if (state.locked) note = '';
      updateIdle(now);
      // Levels 2 and 3 have no courtyard fires (level 3's gas flames need
      // CO2, not Water), so the fire aiming hint never applies there.
      const extinguished = state.level >= 2 ? 1 : state.extinguished;
      setHint(pickHint({ playing, idle: idleOn, note, locked: state.locked, extinguished, casts: state.casts }));
      // The arrow to the nearest fire, while it is off screen and no fire is
      // in view or aimed at (I-025).
      // I-035: after the courtyard is saved, the same arrow points at the open
      // exit arch, once the win banner has had 3 s of game time on its own
      // (nothing shows under a banner as it appears, I-025 test 18).
      // I-037: the same after the Frozen Gate is opened, toward its forward arch.
      const exitAhead = (state.level === 1 && state.exitOpen) || (state.level === 2 && state.nextOpen);
      if (state.outcome !== 'won' || state.level === 3) wonAt = null;
      else if (wonAt === null) wonAt = state.gameTime;
      const findExit = exitAhead && state.outcome === 'won' && wonAt !== null && state.gameTime - wonAt >= EXIT_ARROW_DELAY_S
        && state.fireOff && !state.paused && !input.paused;
      const findOn = (playing && state.fireOff && !state.fireInView && state.nearFire >= 0 && !state.paused && !input.paused) || findExit;
      finder.classList.toggle('hidden', !findOn);
      finder.classList.toggle('exit', findExit);
      if (findOn) {
        const right = state.fireBearing > 0;
        finder.classList.toggle('right', right);
        finder.classList.toggle('left', !right);
      }
      const objTitle = document.querySelector('#objective .title');
      const titleText = state.level === 3 ? 'The Marsh' : state.level === 2 ? 'The Frozen Gate' : 'Save the Greenhouse';
      if (objTitle && objTitle.textContent !== titleText) objTitle.textContent = titleText;
      if (state.level === 3) setObjective(state.outcome === 'won' ? 'Marsh cleared!' : 'Put out the gas flames, cross the boardwalk, reach the lantern.', 0);
      else if (state.level === 2) setObjective(state.outcome === 'won' ? 'Gate opened!' : 'Cross the stream, melt the gate, reach the lantern.', 0);
      else if (state.outcome === 'won') setObjective('Greenhouse saved!', 0);
      else setObjective(`Fires to put out: ${state.firesLeft}`, state.firesLeft);
      if (state.captionId !== capId) {
        capId = state.captionId;
        showCaption(state.caption);
        const icon = ['freeze', 'melt', 'wrong', 'gas'].includes(state.capIcon) ? state.capIcon : '';
        capIcon.dataset.icon = icon;
        capIcon.classList.toggle('hidden', !icon);
        if (state.caption && history[0] !== state.caption) {
          history.unshift(state.caption);
          history.length = Math.min(history.length, 6);
        }
      }
      // Warn while a fire burns next to the greenhouse: how close it is to
      // its ignition temperature (300 °C).
      const warn = $('warning');
      const danger = state.outcome === 'playing' && (state.ghDanger || state.ghTemp > 60);
      warn.classList.toggle('hidden', !danger);
      if (danger) {
        warn.querySelector('.text').textContent = `The greenhouse is heating up: ${state.ghTemp} °C. Wood can catch fire at about 300 °C!`;
        warn.querySelector('.fill').style.width = `${Math.min(100, Math.max(0, (state.ghTemp - 20) / 2.8)).toFixed(0)}%`;
      }
      const banner = $('banner');
      banner.classList.toggle('hidden', state.outcome === 'playing');
      banner.classList.toggle('lost', state.outcome === 'lost');
      if (state.outcome === 'won' && state.level === 3) {
        banner.querySelector('.big').textContent = 'You cleared the Marsh!';
        banner.querySelector('.small').textContent = 'CO₂ is heavier than air: it filled the hollows and smothered the flames. Walk to the arch to go back. Press R to play again.';
      } else if (state.outcome === 'won' && state.level === 2) {
        banner.querySelector('.big').textContent = 'You opened the Frozen Gate!';
        banner.querySelector('.small').textContent = 'Freeze to cross, melt to open: water changed state both ways. Walk to the glowing arch to visit the Marsh. Press R to play again.';
      } else if (state.outcome === 'won') {
        banner.querySelector('.big').textContent = 'You saved the greenhouse!';
        banner.querySelector('.small').textContent = 'Walk to the glowing arch behind you to visit the Frozen Gate. Press R to play again.';
      } else if (state.outcome === 'lost') {
        banner.querySelector('.big').textContent = 'The greenhouse caught fire';
        banner.querySelector('.small').textContent = 'Heat spread from bush to bush. Press R to try again.';
      }
      // The state frame this HUD frame has drawn (for the tests).
      hudEl.dataset.frame = String(state.frame);
      drawn = state;
    }
    requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);
}
