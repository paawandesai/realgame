// Loads the WebGPU build when the browser has WebGPU and the URL does not
// ask for WebGL2 (`?api=webgl2`), else the WebGL2 build. `?api=webgpu`
// forces WebGPU (and fails without it, which the matrix records).
const api = new URLSearchParams(location.search).get('api')
  || ('gpu' in navigator ? 'webgpu' : 'webgl2');
window.__gfxlabApi = api;
const mod = await import(`./${api}/gfx-lab.js`);
try {
  await mod.default();
} catch (e) {
  // Bevy/winit throws on purpose to hand control to the browser.
  if (!String(e).includes('Using exceptions for control flow')) {
    console.error(e);
    window.__gfxlab = JSON.stringify({ error: String(e) });
  }
}
