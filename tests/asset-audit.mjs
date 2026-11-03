// Asset audit for the Game Week rule "every asset is generated for the game".
// Scans a built web folder (default crates/bevy-client/web/pkg-out) and fails if
// it ships any image, model, audio, video or font file, or if the page code
// loads anything from another host. Only code, the wasm, styles and our own
// hand-written SVG favicon may ship; every texture, mesh, sound and tune is
// made at run time by our code.
//
//   node tests/asset-audit.mjs [dir]
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, extname, relative } from 'node:path';

const dir = process.argv[2] || 'crates/bevy-client/web/pkg-out';
const ALLOWED = new Set(['.html', '.css', '.js', '.wasm', '.json', '.txt', '.webmanifest']);
const OWN_SVG = new Set(['favicon.svg']);
// Hosts the page may talk to at run time: none for assets. Same-origin /api
// calls (anonymous analytics) are relative URLs and never match.
const EXTERNAL = /\b(?:src|href)\s*=\s*["']https?:\/\/|url\(\s*["']?https?:\/\/|@import\s+["']?https?:\/\/|fetch\(\s*["']https?:\/\/|new Audio\(|\.(?:png|jpe?g|gif|webp|ogg|mp3|wav|m4a|glb|gltf|fbx|obj|ktx2|woff2?|ttf|otf)["'?]/gi;

const problems = [];
const files = [];
const walk = (d) => {
  for (const name of readdirSync(d)) {
    const p = join(d, name);
    if (statSync(p).isDirectory()) walk(p);
    else files.push(p);
  }
};
walk(dir);

for (const p of files) {
  const rel = relative(dir, p);
  const ext = extname(p).toLowerCase();
  if (ext === '.svg' && OWN_SVG.has(rel)) continue;
  if (!ALLOWED.has(ext)) { problems.push(`ships a non-code file: ${rel}`); continue; }
  if (ext === '.wasm') {
    // An image or model compiled into the binary (include_bytes!, or an
    // engine feature such as a tonemapping lookup texture) counts too.
    // Format-sniffing tables mention the magic bytes alone, so look for the
    // structure that follows a real file.
    const bin = readFileSync(p);
    const embedded = [
      ['PNG image', Buffer.from('\x89PNG\r\n\x1a\n\0\0\0\rIHDR', 'latin1')],
      ['KTX2 texture', Buffer.from('\xabKTX 20\xbb\r\n\x1a\n', 'latin1')],
      ['binary glTF model', Buffer.from('glTF\x02\0\0\0', 'latin1')],
      ['Ogg audio', Buffer.from('OggS\0', 'latin1')],
      ['WAV audio', Buffer.from('WAVEfmt ', 'latin1')],
    ];
    for (const [what, magic] of embedded) {
      if (bin.indexOf(magic) >= 0) problems.push(`${rel} embeds a pre-made ${what}`);
    }
    continue;
  }
  if (['.html', '.css', '.js'].includes(ext)) {
    const text = readFileSync(p, 'utf8');
    for (const m of text.matchAll(EXTERNAL)) {
      const line = text.slice(0, m.index).split('\n').length;
      problems.push(`${rel}:${line} loads an outside asset: ${m[0]}`);
    }
  }
}

console.log(`asset audit: ${files.length} files in ${dir}`);
if (problems.length) {
  for (const p of problems) console.log(`  FAIL ${p}`);
  process.exit(1);
}
console.log('  PASS no pre-made images, models, audio, video or fonts; nothing loaded from another host');
