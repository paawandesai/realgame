#!/usr/bin/env python3
"""Builds the codex's "by rule" tier: content/chem/rules.json (Bench plan B2,
docs/loop/items/I-056/spec.md).

Every reaction here is generated from one of six textbook families, each with
a stated rule, never written by hand:

- acid_base: an acid and a base (a hydroxide, an oxide or ammonia) give a
  salt and water (ammonia gives the ammonium salt alone);
- metal_acid: a metal above hydrogen in the reactivity series gives a salt
  and hydrogen with a non-oxidising acid;
- carbonate_acid: a carbonate or hydrogencarbonate gives a salt, water and
  carbon dioxide with an acid;
- fuel_burns: a fuel of carbon, hydrogen (and oxygen) burns in plenty of
  oxygen to carbon dioxide and water;
- precipitate: two dissolved salts swap partners and the new pair that does
  not dissolve (the solubility rules) comes out as a solid;
- displacement: a more reactive metal pushes a less reactive one out of its
  salt (the reactivity series, by standard electrode potential).

The tables the rules use (ions, salts, solubility, the reactivity series,
acids, fuels, hazards) are written into rules.json with their sources, so
the Rust tests (crates/chem/src/rules.rs) check every reaction against its
rule independently of this script. Heat classes come from the same CRC/NBS
enthalpies of formation as the show records (build_show_data.py), and a
reaction whose species lack one is left out.

Pipeline (a venv outside the repo, as build_show_data.py):
  /tmp/v/bin/python -I tools/chemdb/build_rules.py content/chem
  /tmp/v/bin/python -I tools/chemdb/build_show_data.py content/chem
"""
import json
import math
import os
import re
import sys
from fractions import Fraction

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import build_show_data as bsd  # noqa: E402

OUT = bsd.OUT
BANDS = ["sprout", "seedling", "apprentice", "adept"]

SOURCES = {
    "OPENSTAX-SOL": "OpenStax Chemistry 2e (2019), section 4.2, Table 4.1 'Solubilities of Common Ionic Compounds in Water' (https://openstax.org/books/chemistry-2e/pages/4-2-classifying-chemical-reactions); slightly soluble salts (CaSO4, Ag2SO4, PbCl2, Ca(OH)2, Li2CO3, MgCO3) are marked slight and never used as a precipitate or a dissolved reactant.",
    "CRC-ECS": "CRC Handbook of Chemistry and Physics, 'Electrochemical Series' (P. Vanysek), standard reduction potentials E (V) at 298.15 K: Li+ -3.04, K+ -2.93, Ba2+ -2.91, Ca2+ -2.87, Na+ -2.71, Mg2+ -2.37, Al3+ -1.66, Zn2+ -0.76, Fe2+ -0.45, Pb2+ -0.13, H+ 0, Cu2+ +0.34, Ag+ +0.80.",
    "OPENSTAX-ACID": "OpenStax Chemistry 2e (2019), section 4.2 'Acid-Base Reactions' and section 14 'Acid-Base Equilibria': an acid and a base give a salt and water; metal oxides and hydroxides are bases; ammonia is a base that gives the ammonium salt.",
    "OPENSTAX-COMB": "OpenStax Chemistry 2e (2019), section 4.2 'Combustion reactions': a hydrocarbon or other C, H, O fuel burning in plenty of oxygen gives carbon dioxide and water.",
    "OPENSTAX-REDOX": "OpenStax Chemistry 2e (2019), section 4.2 'Single-displacement (replacement) reactions' and section 17.3 'Standard reduction potentials'.",
    "GHS": "Hazard classes as in each substance's GHS classification (PubChem Laboratory Chemical Safety Summary): corrosive (skin corrosion category 1) or acutely toxic (category 1 to 4 oral).",
}

# ---------------------------------------------------------------------------
# Ions and salts
# ---------------------------------------------------------------------------

# CRC-ION name -> (formula part, charge, chemical name).
CATIONS = {
    "Na+": ("Na", 1, "sodium"), "K+": ("K", 1, "potassium"), "Li+": ("Li", 1, "lithium"),
    "NH4+": ("NH4", 1, "ammonium"), "Mg+2": ("Mg", 2, "magnesium"), "Ca+2": ("Ca", 2, "calcium"),
    "Ba+2": ("Ba", 2, "barium"), "Zn+2": ("Zn", 2, "zinc"), "Cu+2": ("Cu", 2, "copper(II)"),
    "Fe+2": ("Fe", 2, "iron(II)"), "Fe+3": ("Fe", 3, "iron(III)"), "Al+3": ("Al", 3, "aluminium"),
    "Ag+": ("Ag", 1, "silver"), "Pb+2": ("Pb", 2, "lead(II)"),
}
ANIONS = {
    "Cl-": ("Cl", -1, "chloride"), "Br-": ("Br", -1, "bromide"), "I-": ("I", -1, "iodide"),
    "NO3-": ("NO3", -1, "nitrate"), "SO4-2": ("SO4", -2, "sulfate"), "CO3-2": ("CO3", -2, "carbonate"),
    "HCO3-": ("HCO3", -1, "hydrogencarbonate"), "OH-": ("OH", -1, "hydroxide"),
    "PO4-3": ("PO4", -3, "phosphate"), "CH3COO-": ("CH3COO", -1, "acetate"), "O-2": ("O", -2, "oxide"),
}
GROUP1 = {"Na+", "K+", "Li+", "NH4+"}


def poly(part):
    return sum(bsd.atoms(part).values()) > 1


def salt_formula(cat, an):
    cs, cq, _ = CATIONS[cat]
    as_, aq, _ = ANIONS[an]
    g = math.gcd(cq, -aq)
    n, m = -aq // g, cq // g
    cpart = cs if n == 1 else (f"({cs}){n}" if poly(cs) else f"{cs}{n}")
    if an == "CH3COO-":
        return "CH3COO" + cpart if m == 1 else f"{cpart}(CH3COO){m}"
    apart = as_ if m == 1 else (f"({as_}){m}" if poly(as_) else f"{as_}{m}")
    return cpart + apart, n, m


def salt(cat, an):
    f = salt_formula(cat, an)
    if isinstance(f, tuple):
        return f
    cq, aq = CATIONS[cat][1], ANIONS[an][1]
    g = math.gcd(cq, -aq)
    return f, -aq // g, cq // g


def salt_name(cat, an):
    return f"{CATIONS[cat][2]} {ANIONS[an][2]}"


def solubility(cat, an):
    """The solubility rules (OPENSTAX-SOL): soluble, insoluble or slight."""
    if an == "O-2":
        return "insoluble"
    if cat in GROUP1:
        if (cat, an) == ("Li+", "CO3-2"):
            return "slight"
        if (cat, an) == ("Li+", "PO4-3"):
            return "insoluble"
        return "soluble"
    if an in ("NO3-",):
        return "soluble"
    if an == "CH3COO-":
        return "slight" if cat == "Ag+" else "soluble"
    if an in ("Cl-", "Br-", "I-"):
        if cat == "Ag+":
            return "insoluble"
        if cat == "Pb+2":
            return "insoluble" if an == "I-" else "slight"
        return "soluble"
    if an == "SO4-2":
        if cat in ("Ba+2", "Pb+2"):
            return "insoluble"
        if cat in ("Ca+2", "Ag+"):
            return "slight"
        return "soluble"
    if an == "OH-":
        if cat == "Ba+2":
            return "soluble"
        if cat == "Ca+2":
            return "slight"
        return "insoluble"
    if an == "CO3-2":
        return "slight" if cat == "Mg+2" else "insoluble"
    if an == "HCO3-":
        return "soluble"
    if an == "PO4-3":
        return "insoluble"
    raise ValueError((cat, an))


# Standard reduction potentials (CRC-ECS), the reactivity series. A metal
# displaces another only when its E is at least MARGIN_V lower, so the
# reaction goes at a rate a Bench shows (Sn and Pb, 0.01 V apart, do not).
E0 = {"Li": -3.04, "K": -2.93, "Ba": -2.91, "Ca": -2.87, "Na": -2.71, "Mg": -2.37, "Al": -1.66,
      "Zn": -0.76, "Fe": -0.45, "Pb": -0.13, "H": 0.0, "Cu": 0.34, "Ag": 0.80}
METAL_ION = {"Mg": "Mg+2", "Al": "Al+3", "Zn": "Zn+2", "Fe": "Fe+2", "Pb": "Pb+2", "Cu": "Cu+2", "Ag": "Ag+",
             "Ca": "Ca+2", "Na": "Na+", "K": "K+", "Li": "Li+", "Ba": "Ba+2"}
MARGIN_V = 0.25
# Metals that react with the water of a solution first (water_metal family).
WATER_FIRST = ["Li", "K", "Ba", "Ca", "Na"]

ACIDS = {
    "HCl": {"anion": "Cl-", "protons": 1, "oxidising": False, "strong": True},
    "HBr": {"anion": "Br-", "protons": 1, "oxidising": False, "strong": True},
    "HNO3": {"anion": "NO3-", "protons": 1, "oxidising": True, "strong": True},
    "H2SO4": {"anion": "SO4-2", "protons": 2, "oxidising": False, "strong": True},
    "CH3COOH": {"anion": "CH3COO-", "protons": 1, "oxidising": False, "strong": False},
    "H3PO4": {"anion": "PO4-3", "protons": 3, "oxidising": False, "strong": False},
}

# Fuels: formula -> (chemical name, everyday name, SMILES or None if in
# build_show_data, flame emitter in show.json, the need that lights it).
FUELS = {
    "C10H22": ("decane", None, None, "soot", "spark"),
    "C7H16": ("heptane", "heptane", "CCCCCCC", "soot", "spark"),
    "C9H20": ("nonane", "nonane", "CCCCCCCCC", "soot", "spark"),
    "C12H26": ("dodecane", "dodecane (kerosene)", "CCCCCCCCCCCC", "soot", "spark"),
    "C16H34": ("hexadecane", "hexadecane (diesel)", "CCCCCCCCCCCCCCCC", "soot", "spark"),
    "C6H12": ("cyclohexane", "cyclohexane", "C1CCCCC1", "soot", "spark"),
    "C3H6": ("propene", "propene", "C=CC", "soot", "spark"),
    "C4H8": ("1-butene", "butene", "C=CCC", "soot", "spark"),
    "C5H10": ("1-pentene", "pentene", "C=CCCC", "soot", "spark"),
    "C3H4": ("propyne", "propyne", "C#CC", "soot", "spark"),
    "C11H24": ("undecane", "undecane", "CCCCCCCCCCC", "soot", "spark"),
    "C4H9OH": ("1-butanol", "butanol", "CCCCO", "alcohol", "spark"),
    "C5H11OH": ("1-pentanol", "pentanol", "CCCCCO", "alcohol", "spark"),
    "C12H22O11": ("sucrose", None, None, "soot", "heat"),
}
FUEL_NEW_GAS = {"C3H6", "C4H8", "C3H4"}

# Corrosive or toxic species (GHS): a reaction with any of them gets the lab
# badge.
HAZARDS = {
    "HCl": "corrosive", "HBr": "corrosive", "HNO3": "corrosive", "H2SO4": "corrosive", "H3PO4": "corrosive",
    "NaOH": "corrosive", "KOH": "corrosive", "LiOH": "corrosive", "Ba(OH)2": "corrosive and toxic",
    "CaO": "corrosive", "NH3": "corrosive and toxic", "AgNO3": "corrosive", "ZnCl2": "corrosive",
    "FeCl3": "corrosive", "ZnBr2": "corrosive", "AlBr3": "corrosive", "ZnI2": "corrosive", "AlI3": "corrosive",
    "AlCl3": "corrosive", "K3PO4": "corrosive", "Na3PO4": "corrosive",
    "BaCl2": "toxic", "Ba(NO3)2": "toxic", "BaCO3": "toxic", "Pb(NO3)2": "toxic", "PbI2": "toxic",
    "PbCl2": "toxic", "Pb": "toxic", "CuSO4": "toxic", "CuCl2": "toxic", "Cu(NO3)2": "toxic", "CuBr2": "toxic",
    "Cu(CH3COO)2": "toxic", "Li2CO3": "toxic",
}
# Never generated: hydrofluoric acid and fluorides, cyanides, perchlorates,
# chromates, mercury, and any explosive product (silver acetylide, ammonium
# nitrate made in bulk from a melt). The ion tables above hold none of them.
EXCLUDED = "F-, HF, CN-, ClO4-, CrO4-2, Cr2O7-2, Hg2+ and Hg+2 salts, explosive products"

# Familiarity: new species a Bench offers at Apprentice (everything else
# new is Adept). Core species keep the band their bottle first appears at.
APPRENTICE_NEW = {"K2CO3", "KHCO3", "Ca(NO3)2", "Mg(NO3)2", "NaI", "Ca(CH3COO)2", "Mg(CH3COO)2",
                  "Zn(CH3COO)2", "CH3COOK", "KNO3", "ZnCO3", "BaCO3", "Ag2CO3", "C7H16", "C6H12", "C3H6",
                  "C4H8", "C4H9OH", "C12H26", "C16H34", "C10H22", "Al(OH)3"}
# Seedling: simple acid + base and metal + acid with familiar names.
SEEDLING_OK = {"HCl", "CH3COOH", "NaOH", "Ca(OH)2", "Mg(OH)2", "CaO", "Mg", "Zn", "H2O", "H2",
               "NaCl", "CaCl2", "MgCl2", "ZnCl2", "CH3COONa", "Ca(CH3COO)2", "Mg(CH3COO)2"}

RULES = [
    {"id": "acid_base", "family": "neutralisation", "name": "acid + base",
     "text": "An acid and a base cancel each other out, making a salt and water (ammonia makes the salt alone).",
     "source": "OPENSTAX-ACID"},
    {"id": "metal_acid", "family": "metal_acid", "name": "metal + acid",
     "text": "A metal above hydrogen in the reactivity series pushes hydrogen out of an acid, making a salt and hydrogen gas.",
     "source": "CRC-ECS"},
    {"id": "carbonate_acid", "family": "carbonate_acid", "name": "carbonate + acid",
     "text": "A carbonate or hydrogencarbonate fizzes in an acid, making a salt, water and carbon dioxide.",
     "source": "OPENSTAX-ACID"},
    {"id": "fuel_burns", "family": "combustion", "name": "fuel burns",
     "text": "A fuel made of carbon and hydrogen (and sometimes oxygen) burns in plenty of oxygen to make carbon dioxide and water.",
     "source": "OPENSTAX-COMB"},
    {"id": "precipitate", "family": "precipitation", "name": "two salts swap",
     "text": "Two dissolved salts swap partners; the new pair that cannot dissolve (the solubility rules) comes out as a solid.",
     "source": "OPENSTAX-SOL"},
    {"id": "displacement", "family": "displacement", "name": "metal swap",
     "text": "A more reactive metal pushes a less reactive metal out of its salt (the reactivity series).",
     "source": "OPENSTAX-REDOX"},
]
RULE = {r["id"]: r for r in RULES}

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

core = json.load(open(os.path.join(OUT, "core.json")))
names_json = json.load(open(os.path.join(OUT, "names.json")))
show_json = json.load(open(os.path.join(OUT, "show.json")))


def terms(side):
    out = []
    for t in side.split(" + "):
        m = re.match(r"\s*(\d*)(.*?)\s*$", t)
        out.append((int(m.group(1) or 1), m.group(2)))
    return out


def shelf():
    """The Bench's bottles per band (the reactants of the core reactions a
    band sees), as the page builds its shelf."""
    out = {}
    for r in core["reactions"]:
        b = BANDS.index(r["band"])
        for _, f in terms(r["equation"].split(" -> ")[0]):
            out[f] = min(out.get(f, 9), b)
    return out


SHELF = shelf()
CORE_SPECIES = set()
for _r in core["reactions"]:
    for _side in _r["equation"].split(" -> "):
        for _, _f in terms(_side):
            CORE_SPECIES.add(_f)


def nullspace_coeffs(reactants, products):
    """Smallest positive integer coefficients that balance every element,
    or None."""
    species = reactants + products
    els = sorted({e for f in species for e in bsd.atoms(f)})
    rows = []
    for e in els:
        rows.append([Fraction(bsd.atoms(f).get(e, 0) * (1 if i < len(reactants) else -1))
                     for i, f in enumerate(species)])
    n = len(species)
    # Fix the last coefficient to 1 and solve the rest (Gaussian elimination).
    a = [r[:-1] + [-r[-1]] for r in rows]
    m = n - 1
    piv = []
    row = 0
    for col in range(m):
        p = next((i for i in range(row, len(a)) if a[i][col] != 0), None)
        if p is None:
            continue
        a[row], a[p] = a[p], a[row]
        pv = a[row][col]
        a[row] = [x / pv for x in a[row]]
        for i in range(len(a)):
            if i != row and a[i][col] != 0:
                k = a[i][col]
                a[i] = [x - k * y for x, y in zip(a[i], a[row])]
        piv.append(col)
        row += 1
    if len(piv) != m:
        return None
    for i in range(row, len(a)):
        if a[i][-1] != 0:
            return None
    x = [Fraction(0)] * m
    for i, col in enumerate(piv):
        x[col] = a[i][-1]
    x.append(Fraction(1))
    if any(v <= 0 for v in x):
        return None
    lcm = 1
    for v in x:
        lcm = lcm * v.denominator // math.gcd(lcm, v.denominator)
    ints = [int(v * lcm) for v in x]
    g = 0
    for v in ints:
        g = math.gcd(g, v)
    return [v // g for v in ints]


def equation(reactants, products):
    c = nullspace_coeffs(reactants, products)
    if c is None:
        raise ValueError((reactants, products))
    side = lambda fs, cs: " + ".join((str(k) if k > 1 else "") + f for f, k in zip(fs, cs))
    n = len(reactants)
    return side(reactants, c[:n]) + " -> " + side(products, c[n:])


def everyday(f):
    """The name a player reads: names.json's everyday name, else ours."""
    if f in names_json["substances"]:
        return names_json["substances"][f]
    return SPECIES[f]["everyday"]


# The name of a species as the dissolved reagent a Bench bottle holds.
PLAIN = {"HBr": "hydrobromic acid"}


def plain(f):
    """A name without its bracketed everyday hint ("acetic acid (vinegar)"
    reads "vinegar" where the hint is the familiar word)."""
    if f in PLAIN:
        return PLAIN[f]
    n = everyday(f)
    m = re.match(r"(.*?) \((.*)\)$", n)
    if m:
        hint = m.group(2)
        return hint if hint in ("vinegar", "limewater", "petrol", "kerosene", "diesel") else m.group(1)
    return n


def slug(f):
    return re.sub(r"[^a-z0-9]", "", f.lower())


# Species metadata: formula -> {name (for the CAS lookup), everyday}.
SPECIES = {}
# Every salt the tier uses: formula -> {cation, n, anion, m}.
SALTS = {}


def add_salt(cat, an):
    f, n, m = salt(cat, an)
    SALTS[f] = {"cation": cat, "n": n, "anion": an, "m": m}
    if f not in CORE_SPECIES and f not in bsd.NAMES:
        SPECIES[f] = {"name": salt_name(cat, an), "everyday": salt_name(cat, an)}
    elif f not in CORE_SPECIES:
        SPECIES[f] = {"name": bsd.NAMES[f], "everyday": bsd.NAMES[f]}
    return f


for _cat in CATIONS:
    for _an in ANIONS:
        add_salt(_cat, _an)
# Al(OH)3's CAS (the identifier search does not find "aluminium hydroxide").
EXTRA_CAS = {"Al(OH)3": "21645-51-2"}

# ---------------------------------------------------------------------------
# Candidates
# ---------------------------------------------------------------------------

cands = []  # (rule, reactants, products, extra)


def cand(rule, reactants, products, **extra):
    cands.append((rule, reactants, products, extra))


SOL = {f: solubility(s["cation"], s["anion"]) for f, s in SALTS.items()}

# acid + base
BASES = ["NaOH", "KOH", "LiOH", "Ca(OH)2", "Mg(OH)2", "Ba(OH)2", "Cu(OH)2", "Zn(OH)2", "Fe(OH)3", "Al(OH)3",
         "MgO", "CaO", "ZnO", "CuO", "Fe2O3", "Ag2O", "NH3"]
for acid, a in ACIDS.items():
    for base in BASES:
        if base == "NH3":
            cat = "NH4+"
        else:
            cat = SALTS[base]["cation"]
        if acid == "CH3COOH" and cat in ("Fe+3", "Al+3"):
            continue  # basic acetates, not the simple salt
        if acid == "H3PO4" and base not in ("NaOH", "KOH", "Ca(OH)2", "NH3"):
            continue
        if acid == "HNO3" and base == "NH3":
            continue  # ammonium nitrate (an explosive in bulk); core has it
        prod = salt(cat, a["anion"])[0]
        base_soluble = base in ("NaOH", "KOH", "LiOH", "Ba(OH)2", "NH3")
        if SOL[prod] == "slight" or (SOL[prod] == "insoluble" and not base_soluble):
            continue
        cand("acid_base", [acid, base], [prod] if base == "NH3" else [prod, "H2O"])

# metal + acid
for metal in ["Mg", "Al", "Zn", "Fe"]:
    for acid in ["HCl", "HBr", "H2SO4", "CH3COOH"]:
        if acid == "CH3COOH" and metal != "Mg":
            continue  # the others are too slow to see fizz
        prod = salt(METAL_ION[metal], ACIDS[acid]["anion"])[0]
        if SOL[prod] != "soluble":
            continue
        cand("metal_acid", [metal, acid], [prod, "H2"])

# carbonate + acid
for carb in ["Na2CO3", "NaHCO3", "K2CO3", "KHCO3", "CaCO3", "MgCO3", "CuCO3", "ZnCO3", "Li2CO3"]:
    for acid in ["HCl", "HBr", "HNO3", "H2SO4", "CH3COOH", "H3PO4"]:
        cat = SALTS[carb]["cation"]
        prod = salt(cat, ACIDS[acid]["anion"])[0]
        if SOL[prod] != "soluble":
            continue
        cand("carbonate_acid", [carb, acid], [prod, "H2O", "CO2"])

# fuels
for fuel in FUELS:
    cand("fuel_burns", [fuel, "O2"], ["CO2", "H2O"])

# precipitation: two soluble salts swap partners
POOL = {
    "Ag+": ["AgNO3"], "Pb+2": ["Pb(NO3)2"], "Ba+2": ["BaCl2", "Ba(NO3)2", "Ba(OH)2"],
    "Ca+2": ["CaCl2", "Ca(NO3)2"], "Mg+2": ["MgCl2", "MgSO4", "Mg(NO3)2", "MgBr2"],
    "Zn+2": ["ZnCl2", "ZnSO4", "Zn(NO3)2", "ZnBr2", "ZnI2"], "Cu+2": ["CuCl2", "CuSO4", "Cu(NO3)2", "CuBr2"],
    "Fe+2": ["FeCl2", "FeSO4", "FeBr2"], "Fe+3": ["FeCl3", "Fe(NO3)3", "Fe2(SO4)3"],
    "Al+3": ["AlCl3", "Al2(SO4)3", "Al(NO3)3", "AlBr3", "AlI3"],
}
PARTNERS = {
    "Cl-": ["NaCl", "KCl", "LiCl", "NH4Cl"], "Br-": ["NaBr", "KBr", "LiBr", "NH4Br"],
    "I-": ["NaI", "KI", "LiI", "NH4I"], "SO4-2": ["Na2SO4", "K2SO4", "Li2SO4", "(NH4)2SO4"],
    "CO3-2": ["Na2CO3", "K2CO3", "(NH4)2CO3"], "OH-": ["NaOH", "KOH", "LiOH"], "PO4-3": ["Na3PO4", "K3PO4"],
}
pairs = set()
for cat, srcs in POOL.items():
    for s1 in srcs:
        # A group-1 or ammonium partner for every anion.
        for an, ps in PARTNERS.items():
            for s2 in ps:
                pairs.add((s1, s2))
        # Classic ion tests between two metal salts: silver for halides,
        # barium for sulfate and carbonate, barium hydroxide for hydroxides.
        if cat in ("Ag+",):
            for c2, s2s in POOL.items():
                pairs.update((s1, s2) for s2 in s2s if c2 != cat)
        if s1 in ("BaCl2", "Ba(NO3)2", "Ba(OH)2"):
            for c2, s2s in POOL.items():
                pairs.update((s1, s2) for s2 in s2s if c2 != cat)
for s1, s2 in sorted(pairs):
    c1, a1 = SALTS[s1]["cation"], SALTS[s1]["anion"]
    c2, a2 = SALTS[s2]["cation"], SALTS[s2]["anion"]
    if c1 == c2 or a1 == a2 or SOL[s1] != "soluble" or SOL[s2] != "soluble":
        continue
    p1, p2 = salt(c1, a2)[0], salt(c2, a1)[0]
    sols = (SOL[p1], SOL[p2])
    if sorted(sols) != ["insoluble", "soluble"]:
        continue
    ppt, spect = (p1, p2) if SOL[p1] == "insoluble" else (p2, p1)
    pc, pa = SALTS[ppt]["cation"], SALTS[ppt]["anion"]
    if "NH4+" in (c1, c2) and "OH-" in (a1, a2):
        continue  # gives ammonia, not a salt swap
    if pc == "Ag+" and pa == "OH-":
        continue  # silver hydroxide falls apart to Ag2O and water
    if pc in ("Fe+3", "Al+3") and pa == "CO3-2":
        continue  # the hydroxide and CO2 come out instead
    if pc == "Pb+2" and (ppt not in CORE_SPECIES or not ({c1, c2} - {"Pb+2"}) <= GROUP1):
        continue  # lead: only the core's lead salts, with a group-1 or ammonium partner
    cand("precipitate", [s1, s2], [ppt, spect], precipitate=ppt)

# displacement
for m1 in ["Mg", "Al", "Zn", "Fe", "Cu"]:
    for cat, srcs in [("Zn+2", ["ZnCl2", "ZnSO4", "Zn(NO3)2", "ZnBr2"]), ("Fe+2", ["FeCl2", "FeSO4", "FeBr2"]),
                      ("Cu+2", ["CuCl2", "CuSO4", "Cu(NO3)2", "CuBr2"]), ("Ag+", ["AgNO3"]),
                      ("Pb+2", ["Pb(NO3)2"])]:
        m2 = CATIONS[cat][0]
        if E0[m1] > E0[m2] - MARGIN_V:
            continue
        for s in srcs:
            prod = salt(METAL_ION[m1], SALTS[s]["anion"])[0]
            if SOL[prod] != "soluble":
                continue
            cand("displacement", [m1, s], [prod, m2])

# ---------------------------------------------------------------------------
# Species data, heat and filtering
# ---------------------------------------------------------------------------

for f, (cname, every, smi, _, _) in FUELS.items():
    if f not in CORE_SPECIES:
        SPECIES[f] = {"name": cname, "everyday": every}
        assert not smi or bsd.SMILES.get(f) == smi, f
for f in ("H3PO4",):
    if f not in CORE_SPECIES:
        SPECIES[f] = {"name": "phosphoric acid", "everyday": "phosphoric acid"}

used = sorted({f for c in cands for f in c[1] + c[2]})
# Fold the tier's salts into the show-data tables, as build_show_data does
# from rules.json.
for f, sp in SPECIES.items():
    bsd.NAMES.setdefault(f, sp["name"])
for f, c in EXTRA_CAS.items():
    bsd.EXTRA_CAS.setdefault(f, c)
for f in used:
    if f in SALTS and f in SPECIES:
        s = SALTS[f]
        ions = {s["cation"]: s["n"], s["anion"]: s["m"]}
        if SOL[f] == "soluble" and "O-2" not in ions:
            bsd.IONS.setdefault(f, ions)
T = bsd.tables()
subs, _ = bsd.substance_records(used, T)

AQUEOUS = set(show_json["aqueous_families"])


def state_of(f, side, medium, hot):
    """content/chem crates/chem/src/show.rs `state_of`, for the heat class."""
    sub = subs[f]
    s = sub["state"]
    if medium == "water" and f != "H2O":
        has_aq = "aq" in sub["dhf"]
        if s == "g" and side == "p":
            s = "g"
        elif sub["soluble"] and has_aq:
            s = "aq"
    if hot and medium != "water" and side == "p" and s == "l" and sub.get("bp_k") and sub["bp_k"] <= 400:
        s = "g"
    return s


def heat_of(rule, eq):
    family = RULE[rule]["family"]
    medium = "water" if family in AQUEOUS else "air"
    hot = family == "combustion"
    total = 0.0
    for side, sign in ((eq.split(" -> ")[0], -1), (eq.split(" -> ")[1], 1)):
        for k, f in terms(side):
            st = state_of(f, "p" if sign > 0 else "r", medium, hot)
            d = subs[f]["dhf"].get(st)
            if d is None:
                return None, f"{f}({st}) has no enthalpy of formation"
            total += sign * k * d["v"]
    return round(total, 1), None


dropped = []
seen = {}
core_keys = set()
core_left = set()


def key(eq):
    """Reduced equation key: both sides as sorted (formula, coef / gcd)."""
    l, r = eq.split(" -> ")
    tl, tr = terms(l), terms(r)
    g = 0
    for k, _ in tl + tr:
        g = math.gcd(g, k)
    full = (tuple(sorted((f, k // g) for k, f in tl)), tuple(sorted((f, k // g) for k, f in tr)))
    gl = 0
    for k, _ in tl:
        gl = math.gcd(gl, k)
    left = tuple(sorted((f, k // gl) for k, f in tl))
    return full, left


for r in core["reactions"]:
    full, left = key(r["equation"])
    core_keys.add(full)
    core_left.add(left)


def band_of(f):
    if f in SHELF:
        return SHELF[f]
    if f in CORE_SPECIES:
        return BANDS.index("apprentice")
    return BANDS.index("apprentice") if f in APPRENTICE_NEW else BANDS.index("adept")


reactions = []
added = {}
for rule, rs, ps, extra in cands:
    eq = equation(rs, ps)
    full, left = key(eq)
    if full in core_keys or left in core_left:
        dropped.append((eq, "a checked core reaction has these reactants"))
        continue
    if full in seen:
        continue
    kj, why = heat_of(rule, eq)
    if kj is None:
        dropped.append((eq, why))
        continue
    seen[full] = True
    family = RULE[rule]["family"]
    species_all = rs + ps
    seedling = rule in ("acid_base", "metal_acid") and all(f in SEEDLING_OK for f in species_all)
    if seedling:
        b = max([BANDS.index("seedling")] + [band_of(f) for f in rs])
    else:
        b = max([BANDS.index("apprentice")] + [band_of(f) for f in species_all])
    band = BANDS[b]
    heat = "little" if abs(kj) <= 25 else ("releases" if kj < 0 else "absorbs")
    hz = sorted({f for f in species_all if f in HAZARDS})
    hazard = "fire" if family == "combustion" else ("lab" if hz else None)
    # Names and words.
    if rule == "acid_base":
        acid, base = rs
        prod = ps[0]
        name = f"{plain(base)[0].upper() + plain(base)[1:]} and {plain(acid)}"
        say = f"{plain(acid)[0].upper() + plain(acid)[1:]} and {plain(base)} cancel out, giving {plain(prod)}" + ("." if base == "NH3" else " and water.")
        short = "Acid and base cancel out: salt and water." if base != "NH3" else "Acid and ammonia make a salt."
    elif rule == "metal_acid":
        metal, acid = rs
        name = f"{plain(metal).capitalize()} in {plain(acid)}"
        say = f"{plain(metal).capitalize()} fizzes in {plain(acid)}, giving {plain(ps[0])} and hydrogen gas."
        short = "Metal fizzes in acid, giving hydrogen gas."
    elif rule == "carbonate_acid":
        carb, acid = rs
        name = f"{plain(carb)[0].upper() + plain(carb)[1:]} in {plain(acid)}"
        say = f"{plain(carb)[0].upper() + plain(carb)[1:]} fizzes in {plain(acid)}, giving {plain(ps[0])}, water and carbon dioxide."
        short = "It fizzes, giving off carbon dioxide."
    elif rule == "fuel_burns":
        fuel = rs[0]
        name = f"{plain(fuel)[0].upper() + plain(fuel)[1:]} burns"
        say = f"{plain(fuel)[0].upper() + plain(fuel)[1:]} burns in oxygen, giving carbon dioxide, water and heat."
        short = "Fuel burns, giving carbon dioxide and water."
    elif rule == "precipitate":
        ppt = extra["precipitate"]
        col = subs[ppt]["colour"]["s"]["described"]
        name = f"{plain(ppt)[0].upper() + plain(ppt)[1:]} from {plain(rs[0])} and {plain(rs[1])}"
        # A colour with a clause ("white, turning green-brown in air")
        # follows the noun: "a solid (white, turning ...)".
        a_solid = f"a solid ({col})" if "," in col else f"a {col} solid"
        say = f"Mixing {plain(rs[0])} and {plain(rs[1])} solutions makes {plain(ppt)}, {a_solid} that does not dissolve."
        if len(say) > 140:
            say = f"The two solutions swap partners and {plain(ppt)}, {a_solid}, comes out."
        short = "Two clear liquids make a solid."
    else:
        m1, s = rs
        m2 = ps[1]
        name = f"{plain(m1).capitalize()} and {plain(s)}"
        say = f"{plain(m1).capitalize()} is more reactive than {plain(m2)}, so it pushes {plain(m2)} out of {plain(s)}."
        short = "A stronger metal pushes out a weaker one."
    if len(name) > 64:
        name = name.split(" from ")[0] if " from " in name else name[:64]
    assert len(say) <= 140, say
    needs = [FUELS[rs[0]][4]] if rule == "fuel_burns" else []
    e = {
        "id": f"rule_{rule}_{'_'.join(slug(f) for f in rs)}",
        "name": name,
        "family": family,
        "band": band,
        "equation": eq,
        "needs": needs,
        "heat": heat,
        "say": say,
        "source": f"By rule ({RULE[rule]['name']}): {SOURCES[RULE[rule]['source']].split(' (')[0]}. Heat direction from CRC/NBS standard enthalpies of formation (substances.json).",
        "check": "draft",
        "tier": "rule",
        "rule": rule,
        "dh_kj": kj,
    }
    if band in ("sprout", "seedling"):
        e["short"] = short
    if hazard:
        e["hazard"] = hazard
    if hz:
        e["hazard_why"] = ", ".join(f"{f} ({HAZARDS[f]})" for f in hz)
    reactions.append(e)
    for f in rs:
        if f not in SHELF:
            added[f] = min(added.get(f, 9), b)

reactions.sort(key=lambda e: ([r["id"] for r in RULES].index(e["rule"]), BANDS.index(e["band"]), e["id"]))

# show.json fixes for the rule tier: each burning's flame emitter.
show = {}
for e in reactions:
    if e["rule"] == "fuel_burns":
        fuel = terms(e["equation"].split(" -> ")[0])[0][1]
        show[e["id"]] = {"flame": FUELS[fuel][3], "why": "by rule: the flame of a fuel of its kind (COLOUR-HC)"}
    elif e["rule"] == "acid_base":
        # A neutralisation shows little: say what there is to see, as
        # show.json does for the core ones.
        (_, acid), (_, base) = terms(e["equation"].split(" -> ")[0])
        prod = terms(e["equation"].split(" -> ")[1])[0][1]
        warm = {"releases": "the mixture warms a little", "little": "the temperature hardly changes",
                "absorbs": "the mixture cools a little"}[e["heat"]]
        pcol = subs[prod].get("colour", {})
        if base in ("NaOH", "KOH", "LiOH", "Ba(OH)2", "NH3", "Ca(OH)2"):
            if SOL.get(prod) == "insoluble":
                note = f"A {pcol['s']['described']} solid forms as the two solutions mix; {warm}."
            else:
                note = f"Two colourless solutions mix and stay colourless; {warm}."
            show[e["id"]] = {"note": note, "why": "by rule: two solutions"}
        else:
            bcol = subs[base]["colour"]["s"]["described"]
            aq = pcol.get("aq", {}).get("described", "colourless")
            turns = "" if aq.startswith("colourless") else f" and the solution turns {aq}"
            note = f"The {bcol} powder dissolves{turns}; {warm}."
            show[e["id"]] = {"note": note, "speed": ["fast", "a minute or so"], "why": "by rule: a solid base dissolves"}

used_final = sorted({f for e in reactions for side in e["equation"].split(" -> ") for _, f in terms(side)})
out = {
    "version": 1,
    "about": ("The codex's 'by rule' tier (Bench plan B2, docs/loop/items/I-056/spec.md). Generated by "
              "tools/chemdb/build_rules.py; edit that script, not this file. Every reaction comes from one of the "
              "rules below, applied to the tables below (ions, salts, solubility, reactivity series, acids, fuels), "
              "and never shadows a checked core reaction (no rule reaction has the reactants of a core one). "
              "tier 'rule' and check 'draft' until the science review of a sample. dh_kj is the reaction's "
              "standard enthalpy change from the substances' enthalpies of formation, which sets heat. bottles "
              "lists the bottles the tier adds to the Bench shelf and the band each first appears at."),
    "check": "draft",
    "sources": SOURCES,
    "rules": RULES,
    "excluded": EXCLUDED,
    "tables": {
        "ions": {k: {"formula": v[0], "charge": v[1], "name": v[2]} for k, v in {**CATIONS, **ANIONS}.items()},
        "salts": {f: SALTS[f] for f in sorted(SALTS) if f in used_final or f in CORE_SPECIES},
        "solubility": {f: SOL[f] for f in sorted(SALTS) if f in used_final or f in CORE_SPECIES},
        "reduction_potentials_v": E0,
        "metal_ions": METAL_ION,
        "margin_v": MARGIN_V,
        "water_first": WATER_FIRST,
        "acids": ACIDS,
        "hazards": HAZARDS,
    },
    "bottles": [{"species": f, "band": BANDS[b]} for f, b in sorted(added.items())],
    "species": {f: {"name": SPECIES[f]["name"], "everyday": SPECIES[f]["everyday"],
                    **({"cas": EXTRA_CAS[f]} if f in EXTRA_CAS else {})}
                for f in used_final if f in SPECIES},
    "show": show,
    "reactions": reactions,
}
with open(os.path.join(OUT, "rules.json"), "w") as fh:
    json.dump(out, fh, indent=1, ensure_ascii=False)
    fh.write("\n")

# names.json: everyday names for the new species, and the new gases.
ns = names_json
for f in used_final:
    if f not in ns["substances"]:
        ns["substances"][f] = SPECIES[f]["everyday"]
for f in sorted(FUEL_NEW_GAS):
    if f in used_final and f not in ns["gases"]:
        ns["gases"].append(f)
with open(os.path.join(OUT, "names.json"), "w") as fh:
    json.dump(ns, fh, indent=1, ensure_ascii=False)
    fh.write("\n")

from collections import Counter  # noqa: E402
print(len(reactions), "rule reactions;", len(dropped), "dropped")
print("by rule:", dict(Counter(e["rule"] for e in reactions)))
print("by band:", dict(Counter(e["band"] for e in reactions)))
if "--verbose" in sys.argv:
    for eq, why in dropped:
        print("dropped:", eq, "|", why)
