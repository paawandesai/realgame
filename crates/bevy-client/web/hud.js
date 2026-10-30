// HUD and input glue for the first-person prototype. See src/bridge.rs.
//
// window.__gameIn  (JS -> game): raw mouse deltas, lock and pause state, commands.
// window.__gameState (game -> JS): JSON string written every frame.
// window.__game: a small debug and test hook:
//   __game.state()          parsed state
//   __game.cmd('select 2')  commands: select N, cast, tap X Y, look YAW PITCH,
//                           turn DX DY, face I, tp X Z, move RIGHT FWD SECS,
//                           tick N, reset
//   __game.lock()           request pointer lock (needs a user gesture)
//   __game.mode()           { pointMode, lockDenied, dragging }

const $ = (id) => document.getElementById(id);
const params = new URLSearchParams(location.search);

export function startHud() {
  const canvas = $('game');
  const input = { dx: 0, dy: 0, locked: false, paused: false, fire: false, px: -1, py: -1, cmds: [] };
  window.__gameIn = input;
  let state = null;
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
  // requestPointerLock returns a promise in current browsers (and nothing in
  // older ones). Every rejection is caught: a failed lock (no user gesture,
  // or the page lost focus) just leaves the game in click-to-target mode.
  const tryLock = (opts) => {
    try {
      const p = opts ? canvas.requestPointerLock(opts) : canvas.requestPointerLock();
      return p && typeof p.then === 'function' ? p : Promise.resolve();
    } catch (e) {
      return Promise.reject(e);
    }
  };
  const lock = () => {
    canvas.focus();
    pointMode = false;
    // Raw, unaccelerated deltas where supported; plain lock otherwise.
    tryLock({ unadjustedMovement: true })
      .catch(() => tryLock())
      .then(() => { lockDenied = false; })
      .catch(() => {
        // Blocked (browser setting, iframe, or no user gesture): drag to look.
        lockDenied = true;
        $('hint').textContent = 'Mouse lock is blocked here: hold a mouse button and drag to look. Click to cast.';
        $('hint').classList.remove('hidden');
      });
  };
  const showPause = (on) => {
    input.paused = on;
    $('pause').classList.toggle('hidden', !on);
  };
  window.__game = {
    state: () => (window.__gameState ? JSON.parse(window.__gameState) : null),
    cmd,
    lock,
    pause: showPause,
    mode: () => ({ pointMode, lockDenied, dragging: !!drag }),
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
      showPause(false);
    } else if (everLocked && !(state && state.outcome !== 'playing')) {
      // Esc released the mouse: pause with the controls help.
      showPause(true);
    }
  };
  document.addEventListener('pointerlockchange', syncLock);
  const local = (e) => {
    const r = canvas.getBoundingClientRect();
    const k = canvas.clientWidth / r.width;
    return [(e.clientX - r.left) * k, (e.clientY - r.top) * k];
  };
  document.addEventListener('mousemove', (e) => {
    if (input.locked) {
      input.dx += e.movementX;
      input.dy += e.movementY;
      return;
    }
    if (drag) {
      // Drag-to-look: works with no pointer lock at all.
      input.dx += e.movementX;
      input.dy += e.movementY;
      drag.moved += Math.abs(e.movementX) + Math.abs(e.movementY);
    }
    if (e.target === canvas) [input.px, input.py] = local(e);
  });
  // While locked, buttons and the wheel are read here (not through winit) so
  // casting never depends on how the browser routes a locked pointer's events.
  document.addEventListener('pointerdown', (e) => { if (input.locked && e.button === 0) input.fire = true; });
  document.addEventListener('pointerup', (e) => { if (e.button === 0) input.fire = false; });
  document.addEventListener('wheel', (e) => {
    if (!input.locked || e.deltaY === 0) return;
    const cur = state ? state.spell : 0;
    cmd(`select ${((cur + (e.deltaY > 0 ? 1 : 3)) % 4) + 1}`);
  }, { passive: true });
  // Right-click must never open a menu (that drops the lock) or cast.
  document.addEventListener('contextmenu', (e) => e.preventDefault());
  // The game reads the mouse from the mousemove listener above (deltas while
  // locked or dragging, position while free), never from Bevy's cursor
  // events. So pointer moves are stopped before they reach winit's listeners
  // on the canvas: headless Chromium was seen to deliver tens of thousands of
  // coalesced moves per frame to them around pointer lock, growing the
  // WebAssembly heap by hundreds of MB.
  for (const t of ['pointermove', 'pointerrawupdate']) {
    window.addEventListener(t, (e) => e.stopImmediatePropagation(), { capture: true });
  }
  canvas.addEventListener('mouseleave', () => { input.px = -1; input.py = -1; });
  canvas.addEventListener('pointerdown', (e) => {
    canvas.focus();
    if (input.locked || input.paused) return;
    const [x, y] = local(e);
    if (e.pointerType !== 'mouse') {
      // Touch or pen: tap a target to cast at it.
      cmd(`tap ${x.toFixed(1)} ${y.toFixed(1)}`);
      return;
    }
    if (e.button !== 0 && e.button !== 2) return;
    // Any held left or right button drags to look until it is released.
    drag = { button: e.button, x, y, moved: 0 };
    // A left click always asks for the lock (the usual way to play), unless
    // the player chose point-and-click mode or the browser refused the lock.
    if (e.button === 0 && !pointMode && !lockDenied) lock();
  });
  document.addEventListener('pointerup', (e) => {
    if (!drag || e.button !== drag.button) return;
    const d = drag;
    drag = null;
    if (input.locked || d.button !== 0 || d.moved > 6) return;
    if (pointMode) {
      // Point and click: cast at the target under the cursor.
      cmd(`tap ${d.x.toFixed(1)} ${d.y.toFixed(1)}`);
    } else if (lockDenied) {
      // No lock available: a click casts at the crosshair.
      cmd('cast');
    }
  });
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && !input.locked) showPause(!input.paused);
  });
  $('resume').addEventListener('click', () => { showPause(false); lock(); });
  $('pointmode').addEventListener('click', () => {
    showPause(false);
    pointMode = true;
    $('hint').textContent = 'Point and click: click a glowing target to cast at it. Drag to look.';
    $('hint').classList.remove('hidden');
    canvas.focus();
  });
  document.querySelectorAll('.slot').forEach((b) => {
    b.addEventListener('pointerdown', (e) => { e.stopPropagation(); cmd(`select ${b.dataset.spell}`); canvas.focus(); });
  });

  const slots = [...document.querySelectorAll('.slot')];
  const cross = $('crosshair');
  const caption = $('caption');
  let capId = -1;
  let capTimer = 0;
  const frame = () => {
    syncLock();
    const raw = window.__gameState;
    if (raw && raw !== lastRaw) {
      lastRaw = raw;
      state = JSON.parse(raw);
      $('loading').classList.add('hidden');
      $('hud').classList.remove('hidden');
      slots.forEach((s, i) => s.classList.toggle('sel', i === state.spell));
      cross.classList.toggle('on', state.aimObj >= 0 && state.locked);
      canvas.classList.toggle('hover', pointMode && !state.locked && state.hover >= 0);
      if (!pointMode && !lockDenied) $('hint').classList.toggle('hidden', state.locked || state.casts > 0);
      if (state.locked) $('hint').classList.add('hidden');
      $('obj-line').textContent = state.outcome === 'won'
        ? 'Greenhouse saved!'
        : `Fires put out: ${Math.min(state.extinguished, state.goal)} / ${state.goal}  ·  burning now: ${state.burning}`;
      if (state.captionId !== capId) {
        capId = state.captionId;
        caption.textContent = state.caption;
        caption.classList.add('show');
        clearTimeout(capTimer);
        capTimer = setTimeout(() => caption.classList.remove('show'), 4500);
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
      if (state.outcome === 'won') {
        banner.querySelector('.big').textContent = 'You saved the greenhouse!';
        banner.querySelector('.small').textContent = `${state.extinguished} fires put out. Press R to play again.`;
      } else if (state.outcome === 'lost') {
        banner.querySelector('.big').textContent = 'The greenhouse caught fire';
        banner.querySelector('.small').textContent = 'Heat spread from bush to bush. Press R to try again.';
      }
    }
    requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);
}
