// The realistic look's scanned ground textures (?look=new; graphics plan,
// docs/graphics/world-look-realistic.md). Loads the two stacked JPEGs
// (tex/terrain-albedo.jpg and tex/terrain-normal.jpg: one 1024 px tile per
// layer, top to bottom: grass, soil, rock, snow), lets the browser decode
// them, builds each layer's mip chain on a canvas and leaves the bytes on
// window.__terrainTex for the game (look.rs), which turns them into
// texture arrays and clears the global. ?low uses 512 px tiles.
(function () {
  const q = new URLSearchParams(location.search);
  if (q.get('look') !== 'new') return;
  const LAYERS = 4;
  const SIZE = q.has('low') ? 512 : 1024;
  const LEVELS = Math.log2(SIZE) + 1;

  async function bitmap(url) {
    const r = await fetch(url);
    if (!r.ok) throw new Error(`${url}: ${r.status}`);
    // No colour conversion: albedo stays sRGB bytes, normals stay vectors.
    return createImageBitmap(await r.blob(), { colorSpaceConversion: 'none', premultiplyAlpha: 'none' });
  }

  // All layers' mips, layer-major, RGBA8.
  function chain(bmp) {
    const tile = bmp.width;
    let per = 0;
    for (let l = 0; l < LEVELS; l++) per += Math.max(1, SIZE >> l) ** 2 * 4;
    const out = new Uint8Array(per * LAYERS);
    let at = 0;
    for (let layer = 0; layer < LAYERS; layer++) {
      let src = document.createElement('canvas');
      src.width = src.height = SIZE;
      let g = src.getContext('2d', { willReadFrequently: true });
      g.imageSmoothingQuality = 'high';
      g.drawImage(bmp, 0, layer * tile, tile, tile, 0, 0, SIZE, SIZE);
      for (let l = 0; l < LEVELS; l++) {
        const s = Math.max(1, SIZE >> l);
        if (l > 0) {
          const c = document.createElement('canvas');
          c.width = c.height = s;
          const cg = c.getContext('2d', { willReadFrequently: true });
          cg.imageSmoothingQuality = 'high';
          cg.drawImage(src, 0, 0, src.width, src.height, 0, 0, s, s);
          src = c;
          g = cg;
        }
        const px = g.getImageData(0, 0, s, s).data;
        out.set(px, at);
        at += px.length;
      }
    }
    return out;
  }

  (async () => {
    try {
      const [a, n] = await Promise.all([bitmap('tex/terrain-albedo.jpg'), bitmap('tex/terrain-normal.jpg')]);
      window.__terrainTex = { size: SIZE, layers: LAYERS, levels: LEVELS, albedo: chain(a), normal: chain(n) };
      a.close();
      n.close();
    } catch (e) {
      console.warn('ground textures not loaded:', e);
    }
  })();
})();
