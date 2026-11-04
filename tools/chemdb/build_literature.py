#!/usr/bin/env python3
"""Builds content/chem/literature.json, the Bench's "lab records" index.

Reads reactions.jsonl.gz from build_chemdb.py and keeps the balanced, neutral
reactions whose species use only the Bench's elements. Each record becomes a
net equation: a species listed on both sides (a reagent that was not used up)
is cancelled, coefficients are divided by their common factor, and species are
sorted. The key format is the one `realgame_chem::literature_key` makes from a
player's equation, so the browser can look a reaction up directly.

Formulas only: two different molecules with the same formula (isomers) share
a key, so a hit means "a reaction with these formulas is recorded", which is
how the Bench words it.

Usage: build_literature.py reactions.jsonl.gz content/chem/names.json out.json
"""
import gzip
import json
import math
import re
import sys

TOKEN = re.compile(r"([A-Z][a-z]?)(\d*)")
MAX_HEAVY = 20  # heavy (non-hydrogen) atoms per species


def atoms(formula):
    out = {}
    for el, n in TOKEN.findall(formula):
        out[el] = out.get(el, 0) + int(n or 1)
    return out


def side_text(side):
    return " + ".join(
        (str(n) if n > 1 else "") + f for f, n in sorted(side.items())
    )


def net(reactants, products):
    """Cancels shared species and reduces; None if a side empties."""
    left, right = dict(reactants), dict(products)
    for f in set(left) & set(right):
        k = min(left[f], right[f])
        left[f] -= k
        right[f] -= k
    left = {f: n for f, n in left.items() if n}
    right = {f: n for f, n in right.items() if n}
    if not left or not right:
        return None
    g = 0
    for n in list(left.values()) + list(right.values()):
        g = math.gcd(g, n)
    return {f: n // g for f, n in left.items()}, {f: n // g for f, n in right.items()}


def main(src, names_path, out_path):
    elements = set(json.load(open(names_path))["elements"])
    index, kept = {}, 0
    with gzip.open(src, "rt") as fh:
        for line in fh:
            r = json.loads(line)
            if not r["balanced"]:
                continue
            species = list(r["reactants"]) + list(r["products"])
            if any(f.endswith(("+", "-")) for f in species):
                continue
            ok = True
            for f in species:
                a = atoms(f)
                if not set(a) <= elements or sum(n for e, n in a.items() if e != "H") > MAX_HEAVY:
                    ok = False
                    break
            if not ok:
                continue
            sides = net(r["reactants"], r["products"])
            if sides is None:
                continue
            key = side_text(sides[0]) + " -> " + side_text(sides[1])
            hit = index.get(key)
            if hit:
                hit[0] += 1
            else:
                index[key] = [1, r["id"].split(":", 1)[1]]
            kept += 1
    stats = json.load(open(src.replace("reactions.jsonl.gz", "stats.json")))
    doc = {
        "licence": "CC BY-SA 4.0",
        "attribution": "Data from the Open Reaction Database (Kearnes et al., J. Am. Chem. Soc. 2021, 143, 18820; "
        "https://github.com/open-reaction-database/ord-data, CC BY-SA 4.0). Changed: formulas computed, "
        "reactions balanced, reagents on both sides cancelled, duplicates merged.",
        "ord_commit": stats["sources"]["ord"]["commit"],
        "records": kept,
        "reactions": dict(sorted(index.items())),
    }
    with open(out_path, "w") as fh:
        json.dump(doc, fh, separators=(",", ":"))
        fh.write("\n")
    print(f"{kept} records, {len(index)} distinct net equations -> {out_path}")


if __name__ == "__main__":
    main(*sys.argv[1:4])
