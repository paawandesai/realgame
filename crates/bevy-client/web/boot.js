// Loaded as an external module so the page works under script-src 'self'.
import init from './bevy-client.js';
window.__bootStart = performance.now();
init().catch((e) => {
  // Bevy/winit throws a control-flow exception on purpose to hand the
  // event loop to the browser; anything else is a real error.
  if (!String(e).includes('Using exceptions for control flow')) console.error(e);
});
