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

const $ = (id) => document.getElementById(id);
const params = new URLSearchParams(location.search);

export function startHud() {
  const canvas = $('game');
  const input = { dx: 0, dy: 0, locked: false, paused: false, fire: false, px: -1, py: -1, cmds: [] };
  window.__gameIn = input;
  let state = null;
  let lastRaw = '';
  let everLocked = false;

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
    // Raw, unaccelerated deltas where supported; plain lock otherwise.
    tryLock({ unadjustedMovement: true })
      .catch(() => tryLock())
      .catch(() => { /* stay unlocked */ });
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
    } else if (e.target === canvas) {
      [input.px, input.py] = local(e);
    }
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
  canvas.addEventListener('mouseleave', () => { input.px = -1; input.py = -1; });
  canvas.addEventListener('pointerdown', (e) => {
    canvas.focus();
    if (input.locked || input.paused) return;
    const [x, y] = local(e);
    if (e.pointerType !== 'mouse' || (state && state.hover >= 0)) {
      // Click or tap on a target: cast at it without locking the mouse.
      cmd(`tap ${x.toFixed(1)} ${y.toFixed(1)}`);
    } else if (e.button === 0) {
      lock();
    }
  });
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && !input.locked) showPause(!input.paused);
  });
  $('resume').addEventListener('click', () => { showPause(false); lock(); });
  $('pointmode').addEventListener('click', () => { showPause(false); canvas.focus(); });
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
      canvas.classList.toggle('hover', !state.locked && state.hover >= 0);
      $('hint').classList.toggle('hidden', state.locked || state.casts > 0);
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
        warn.querySelector('.text').textContent = `The greenhouse is heating up: ${state.ghTemp} °C. Wood catches fire at 300 °C!`;
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
