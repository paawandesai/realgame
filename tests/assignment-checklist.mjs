// Game Week Option 1 checklist, read from the game's own sources so it fails
// if content is removed: 3+ connected areas, 3+ puzzles, the ending, and the
// asset audit. The 10-15 minute play time is measured by the golden
// `playthrough` section, not here.
//
//   node tests/assignment-checklist.mjs
import { readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';

const read = (p) => readFileSync(p, 'utf8');
const results = [];
const check = (name, ok, detail = '') => results.push({ name, ok, detail });

const valley = read('crates/sim/src/valley/mod.rs');
const areas = [...valley.matchAll(/"(\w+)" => "(The [^"]+)"/g)].map((m) => m[2]);
check('3+ connected areas', areas.length >= 3, `${areas.length}: ${areas.join(', ')}`);

const late = read('crates/bevy-client/web/late.js');
const block = late.match(/export const PUZZLES = \[([\s\S]*?)\];/);
const puzzles = block ? [...block[1].matchAll(/'([^']+)'/g)].map((m) => m[1]) : [];
check('3+ puzzles', puzzles.length >= 3, `${puzzles.length}: ${puzzles.join('; ')}`);

check('an ending', /You relit the Great Kiln!/.test(late), 'ending card title in late.js');
check('abilities open new paths', /Practised/.test(read('crates/bevy-client/src/gates.rs')), 'biome gates need a Practised spell');

let audit = true;
try { execFileSync('node', ['tests/asset-audit.mjs'], { stdio: 'pipe' }); } catch { audit = false; }
check('all assets generated (asset audit)', audit, 'node tests/asset-audit.mjs');

for (const r of results) console.log(`  ${r.ok ? 'PASS' : 'FAIL'} ${r.name}${r.detail ? ` (${r.detail})` : ''}`);
process.exit(results.every((r) => r.ok) ? 0 : 1);
