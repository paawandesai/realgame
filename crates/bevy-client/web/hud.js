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
// Test commands other than select, cast, tap and reset need `?debug`.

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
      .then(() => { lockDenied = false; note = ''; })
      .catch(() => {
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
  const showPause = (on) => {
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
    // The free cursor position, for hover and click-to-target. Checked
    // against the canvas rectangle (not e.target), so HUD overlays and
    // synthetic events still count.
    const r = canvas.getBoundingClientRect();
    const inside = e.clientX >= r.left && e.clientX < r.right && e.clientY >= r.top && e.clientY < r.bottom;
    if (inside) [input.px, input.py] = local(e);
    else { input.px = -1; input.py = -1; }
  };
  window.addEventListener('pointermove', onMove, { capture: true });
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
  document.addEventListener('pointerdown', (e) => { if (input.locked && e.button === 0) input.fire = true; });
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
      cmd(`tap ${x.toFixed(1)} ${y.toFixed(1)}`);
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
      cmd(`tap ${d.x.toFixed(1)} ${d.y.toFixed(1)}`);
    } else if (lockDenied) {
      // No lock available: a click casts at the crosshair.
      cmd('cast');
    }
  });
  const caption = $('caption');
  let capTimer = 0;
  // Captions stay 8 s (long science lines take young readers a while).
  const showCaption = (text) => {
    caption.textContent = text;
    caption.classList.add('show');
    clearTimeout(capTimer);
    capTimer = setTimeout(() => caption.classList.remove('show'), 8000);
  };
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && !input.locked) showPause(!input.paused);
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
  const hint = $('hint');
  const setHint = (text) => {
    if (hint.textContent !== text) hint.textContent = text;
    hint.classList.toggle('hidden', !text);
  };
  let capId = -1;
  const frame = () => {
    syncLock();
    const raw = window.__gameState;
    if (raw && raw !== lastRaw) {
      lastRaw = raw;
      state = JSON.parse(raw);
      $('loading').classList.add('hidden');
      $('hud').classList.remove('hidden');
      slots.forEach((s, i) => s.classList.toggle('sel', i === state.spell));
      if (wheelSpell === state.spell) wheelSpell = null;
      cross.classList.toggle('on', state.aimObj >= 0 && state.locked);
      canvas.classList.toggle('hover', pointMode && !state.locked && state.hover >= 0);
      // Hints: how to start, then how to aim until three spells have landed.
      if (state.locked) {
        note = '';
        setHint(state.impacts < 3 ? 'Aim at a glowing target: the crosshair turns yellow. Left click casts.' : '');
      } else if (note) {
        setHint(note);
      } else {
        setHint(state.casts > 0 ? '' : 'Click to look around. Esc shows the controls and point-and-click mode.');
      }
      $('obj-line').textContent = state.outcome === 'won'
        ? 'Greenhouse saved!'
        : `Fires put out: ${Math.min(state.extinguished, state.goal)} / ${state.goal}  ·  burning now: ${state.burning}`;
      if (state.captionId !== capId) {
        capId = state.captionId;
        showCaption(state.caption);
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
