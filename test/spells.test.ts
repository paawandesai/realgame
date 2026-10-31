// The seam between content/spells.json and the 3D client (I-026 acceptance
// tests 16 and 19). The client reads its spell text from spells.json through
// crates/sim; these checks keep the static hotbar, the chemistry rules'
// effect ids, LN2_C and the science register in step with that file, and pin
// that only spells.rs parses the books, once.
import assert from "node:assert/strict";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

const REPO = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const read = (p: string) => readFileSync(join(REPO, p), "utf8");

type Ref = { kind: "reaction" | "phase_change" | "register"; id: string; role: string };
type SpellDef = {
  id: string;
  slot: number;
  effect: string;
  name: string;
  label: string;
  hotbar_label: string;
  cast_caption: string;
  formula: string;
  tier_min: string;
  reaction: Ref;
};

const PLAYER_TEXT = ["name", "label", "hotbar_label", "cast_caption"] as const;

const spells = (JSON.parse(read("content/spells.json")) as { spells: SpellDef[] }).spells
  .slice()
  .sort((a, b) => a.slot - b.slot);
const reactions = JSON.parse(read("content/reactions.json")) as {
  reactions: Array<{ id: string }>;
  phase_changes: Array<{ id: string; temperature_c: number }>;
};
const chem = read("crates/bevy-client/src/chem.rs");

/** The science lint's normalize (test/science-sources.test.ts), kept identical. */
function normalize(s: string): string {
  return s
    .normalize("NFC")
    .replace(/[₀-₉]/g, (c) => String(c.charCodeAt(0) - 0x2080))
    .replace(/−/g, "-")
    .replace(/\s+/g, " ")
    .trim();
}

const collapse = (s: string) => s.replace(/\s+/g, " ").trim();

test("spells.json slots are 1..n", () => {
  assert.deepEqual(
    spells.map((s) => s.slot),
    spells.map((_, i) => i + 1),
  );
});

test("the static hotbar in index.html matches spells.json", () => {
  const html = read("crates/bevy-client/web/index.html");
  const slots = [...html.matchAll(/<button class="slot" data-spell="(\d+)">([\s\S]*?)<\/button>/g)];
  assert.equal(slots.length, spells.length, "one .slot button per spell");
  slots.forEach((m, i) => {
    const s = spells[i]!;
    const span = (cls: string) => {
      const t = new RegExp(`<span class="${cls}">([^<]*)</span>`).exec(m[2]!);
      assert.ok(t, `slot ${m[1]} has a .${cls} span`);
      return collapse(t[1]!);
    };
    assert.equal(Number(m[1]), s.slot, `button ${i + 1} data-spell`);
    assert.equal(span("name"), s.name, `slot ${s.slot} .name`);
    assert.equal(span("formula"), s.hotbar_label, `slot ${s.slot} .formula`);
  });
});

test("effect ids in chem.rs equal spells.json effects, in Spell::ALL order", () => {
  const all = /pub const ALL: \[Spell; \d+\] = \[([^\]]*)\];/.exec(chem);
  assert.ok(all, "Spell::ALL in chem.rs");
  const order = all[1]!.split(",").map((v) => v.trim().replace(/^Spell::/, "")).filter(Boolean);
  const body = /pub fn effect_id\(self\) -> &'static str \{\s*match self \{([\s\S]*?)\n\s*\}\s*\n\s*\}/.exec(chem);
  assert.ok(body, "Spell::effect_id in chem.rs");
  const arms = new Map([...body[1]!.matchAll(/Spell::(\w+)\s*=>\s*"([^"]*)"/g)].map((m) => [m[1]!, m[2]!]));
  assert.equal(arms.size, order.length, "one effect_id arm per Spell");
  assert.deepEqual(
    order.map((v) => arms.get(v)),
    spells.map((s) => s.effect),
  );
});

test("LN2_C equals nitrogen_boiling rounded", () => {
  const m = /pub const LN2_C: f32 = (-?[\d.]+);/.exec(chem);
  assert.ok(m, "LN2_C in chem.rs");
  const n = reactions.phase_changes.find((p) => p.id === "nitrogen_boiling");
  assert.ok(n, "nitrogen_boiling in content/reactions.json");
  assert.equal(Number(m[1]), Math.round(n.temperature_c));
});

test("every reaction or phase_change reference exists in reactions.json", () => {
  for (const s of spells) {
    const r = s.reaction;
    if (r.kind === "reaction") assert.ok(reactions.reactions.some((x) => x.id === r.id), `${s.id}: reaction ${r.id}`);
    else if (r.kind === "phase_change") {
      assert.ok(reactions.phase_changes.some((x) => x.id === r.id), `${s.id}: phase_change ${r.id}`);
    }
  }
});

test("every register reference names a sourced row whose claim is that spell's text", () => {
  const md = read("content/science-sources.md");
  const sourcesAt = md.indexOf("\n## Sources");
  assert.ok(sourcesAt >= 0, "## Sources in the register");
  const defined = new Set([...md.slice(sourcesAt).matchAll(/^- \[(S\d+)\]/gm)].map((m) => m[1]!));
  const registerRefs = spells.filter((s) => s.reaction.kind === "register");
  assert.ok(registerRefs.length > 0, "at least one register reference (CO2)");
  for (const s of registerRefs) {
    const row = md.split("\n").find((l) => l.startsWith(`| ${s.reaction.id} |`));
    assert.ok(row, `${s.id}: register row ${s.reaction.id}`);
    const cells = row.split(/(?<!\\)\|/).map((c) => c.trim());
    // | id | claim | where | verdict | note | sources |
    const [, , claim, , verdict, , sources] = cells;
    assert.ok(verdict === "correct" || verdict === "simplified", `${s.reaction.id} verdict ${verdict}`);
    const ids = [...(sources ?? "").matchAll(/\[(S\d+)\]/g)].map((m) => m[1]!);
    assert.ok(
      ids.some((id) => defined.has(id)),
      `${s.reaction.id} cites a defined source`,
    );
    const text = normalize((claim ?? "").replace(/^`|`$/g, ""));
    assert.ok(
      PLAYER_TEXT.some((k) => normalize(s[k]) === text),
      `${s.reaction.id}'s claim is one of ${s.id}'s player-text fields`,
    );
  }
});

test("only spells.rs loads the spell and reaction books, inside its OnceLock", () => {
  const dir = join(REPO, "crates/bevy-client/src");
  const files: string[] = [];
  const walk = (d: string) => {
    for (const n of readdirSync(d).sort()) {
      const p = join(d, n);
      if (statSync(p).isDirectory()) walk(p);
      else if (n.endsWith(".rs")) files.push(p);
    }
  };
  walk(dir);
  const pattern = /SpellBook::(parse|load)|ReactionBook::load/;
  const hits: string[] = [];
  for (const f of files) {
    read(f.slice(REPO.length + 1))
      .split("\n")
      .forEach((l, i) => {
        if (pattern.test(l)) hits.push(`${f.slice(dir.length + 1)}:${i + 1}`);
      });
  }
  assert.ok(hits.length > 0, "spells.rs loads the books");
  assert.ok(
    hits.every((h) => h.startsWith("spells.rs:")),
    `only spells.rs may load the books: ${hits.join(", ")}`,
  );
  const src = read("crates/bevy-client/src/spells.rs");
  const init = src.indexOf("get_or_init(");
  assert.ok(init >= 0, "spells.rs uses OnceLock::get_or_init");
  // The closure ends at the `})` that closes get_or_init.
  const end = src.indexOf("})", init);
  src.split("\n").forEach((l, i) => {
    if (!pattern.test(l)) return;
    const at = src.split("\n").slice(0, i).join("\n").length + 1;
    assert.ok(at > init && at < end, `spells.rs:${i + 1} is outside the get_or_init closure`);
  });
});
