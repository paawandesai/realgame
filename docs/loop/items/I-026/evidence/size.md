# I-026 size evidence (test 13, builder measurement)

Both sides built the same way, on the same machine, into the same target dir (`/home/claude/bevy-target`), one after the other, with `bash crates/bevy-client/build-web.sh` (release, opt-level z, fat LTO, default features `bloom,webgl2`, no wasm-opt).

- **Base of record:** origin/main `4c7c9a1` (= b8fa548 plus a loop-state docs commit; identical client, sim and content).
- **Branch:** `loop/I-026` at the client and lint commits (`80721f5`).

The `brotli` CLI is not installed on this box, so brotli was measured with Node's zlib at the same settings as `brotli -q 11` (quality 11, window 24 bits). gzip is Node's zlib level 9 (build-web.sh's `gzip -9` prints a slightly different absolute number; both sides here use the same tool).

| `bevy-client_bg.wasm` | base | branch | delta |
|---|---:|---:|---:|
| raw bytes | 17,681,945 | 17,778,303 | +96,358 |
| gzip -9 bytes | 5,106,204 | 5,148,968 | +42,764 |
| **brotli q11 bytes** | **3,536,740** | **3,563,434** | **+26,694 (26.1 KiB)** |

**Gate: brotli growth at most 60 KB. Result: +26,694 bytes, pass.** Under the 40 KB line, so no twiggy report is attached. The growth is serde_json, the three embedded JSON files (reactions.json 5,088 bytes, known_reactions.json 4,761 and spells.json 2,112) and the formula parser. Splitting `ReactionBook::load` to skip `known_reactions.json` is not worth doing at this size.

Test 12 on the branch build: `test -s web/pkg-out/bevy-client.js` succeeds; `grep -cE "WebGame|reaction_book|heat_lut" web/pkg-out/bevy-client.js` = **0**.

Speed (perf-pairs, recorded only) is the tester's step; see the client lane note for whether the builder ran it.
