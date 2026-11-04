"""Chemistry helpers for the chemdb import pipeline.

Pure functions only (RDKit + stdlib), so they can run inside worker processes.

Rules this module enforces, matching tools/chemdb/README.md:
  * Formulas come straight from the structure RDKit parses from SMILES. Hydrogens
    are the SMILES-implied ones (explicit + implicit valence H); nothing is added.
    A structure that RDKit cannot sanitize is rejected, not repaired.
  * Charges come only from formal charges written in the SMILES.
  * Dummy atoms / R-groups (atomic number 0) make a species unusable.
"""

from __future__ import annotations

import re
from collections import Counter
from fractions import Fraction
from functools import reduce
from itertools import product as iproduct
from math import gcd

from rdkit import Chem, RDLogger

RDLogger.DisableLog("rdApp.*")

MAX_COEFF = 20  # largest stoichiometric coefficient we accept from the solver
BRUTE_MAX = 4  # coefficient range for the (rare) multi-solution search
BRUTE_MAX_SPECIES = 6


class BadSpecies(ValueError):
    """A species whose structure cannot be turned into an exact formula."""


def hill_formula(counts: dict[str, int], charge: int = 0) -> str:
    """Hill order: C, then H, then the rest alphabetically; with no C, all alphabetical.

    A non-zero net charge is appended as e.g. "+", "2+", "-", "3-".
    """
    def part(el: str) -> str:
        n = counts[el]
        return el if n == 1 else f"{el}{n}"

    els = [e for e, n in counts.items() if n]
    if "C" in els:
        order = ["C"] + (["H"] if "H" in els else []) + sorted(e for e in els if e not in ("C", "H"))
    else:
        order = sorted(els)
    s = "".join(part(e) for e in order)
    if charge:
        mag = abs(charge)
        s += ("" if mag == 1 else str(mag)) + ("+" if charge > 0 else "-")
    return s


def species_from_smiles(smi: str) -> tuple[str, str, Counter, int]:
    """Parse one species (may be multi-fragment, e.g. a salt).

    Returns (canonical_smiles_without_atom_maps, hill_formula, element_counts, charge).
    Raises BadSpecies if the SMILES does not sanitize or contains dummy atoms.
    """
    mol = Chem.MolFromSmiles(smi)
    if mol is None:
        raise BadSpecies(smi)
    counts: Counter = Counter()
    charge = 0
    for a in mol.GetAtoms():
        if a.GetAtomicNum() == 0:
            raise BadSpecies(smi)
        a.SetAtomMapNum(0)
        counts[a.GetSymbol()] += 1
        h = a.GetTotalNumHs()
        if h:
            counts["H"] += h
        charge += a.GetFormalCharge()
    canon = Chem.MolToSmiles(mol)
    return canon, hill_formula(counts, charge), counts, charge


_FRAG_GROUPS = re.compile(r"f:([0-9.,]+)")


def split_reaction_smiles(rxn: str) -> tuple[list[str], list[str], list[str]] | None:
    """Split a (CX)SMILES reaction into species lists for reactants, agents, products.

    Honors the CXSMILES "f:" fragment groups (e.g. "|f:1.2|" says fragments 1 and 2,
    counted across the whole reaction, are one species such as Na+ HCO3-).
    """
    rxn = rxn.strip()
    ext = ""
    if " " in rxn:
        rxn, ext = rxn.split(" ", 1)
    parts = rxn.split(">")
    if len(parts) != 3:
        return None
    frags: list[tuple[int, str]] = []
    for side, p in enumerate(parts):
        if p:
            frags.extend((side, f) for f in p.split("."))
    if any(not f for _, f in frags):
        return None
    grouped: dict[int, list[int]] = {}
    m = _FRAG_GROUPS.search(ext)
    if m:
        for g in m.group(1).strip(",.").split(","):
            idx = [int(x) for x in g.split(".") if x.isdigit()]
            if len(idx) < 2 or max(idx) >= len(frags):
                return None
            if len({frags[i][0] for i in idx}) != 1:
                return None
            for i in idx:
                grouped[i] = idx
    out: tuple[list[str], list[str], list[str]] = ([], [], [])
    seen = set()
    for i, (side, f) in enumerate(frags):
        if i in seen:
            continue
        if i in grouped:
            idx = grouped[i]
            seen.update(idx)
            out[side].append(".".join(frags[j][1] for j in idx))
        else:
            out[side].append(f)
    return out


def _rref_nullspace(rows: list[list[Fraction]], ncols: int) -> list[list[Fraction]]:
    m = [r[:] for r in rows]
    pivots = []
    r = 0
    for c in range(ncols):
        piv = next((i for i in range(r, len(m)) if m[i][c] != 0), None)
        if piv is None:
            continue
        m[r], m[piv] = m[piv], m[r]
        pv = m[r][c]
        m[r] = [x / pv for x in m[r]]
        for i in range(len(m)):
            if i != r and m[i][c] != 0:
                f = m[i][c]
                m[i] = [a - f * b for a, b in zip(m[i], m[r])]
        pivots.append(c)
        r += 1
        if r == len(m):
            break
    free = [c for c in range(ncols) if c not in pivots]
    basis = []
    for fc in free:
        v = [Fraction(0)] * ncols
        v[fc] = Fraction(1)
        for i, pc in enumerate(pivots):
            v[pc] = -m[i][fc]
        basis.append(v)
    return basis


def balance(left: dict[str, tuple[Counter, int]], right: dict[str, tuple[Counter, int]]):
    """Find smallest positive integer coefficients that conserve every element and charge.

    left/right map formula -> (element counts, charge). Returns (lcoef, rcoef) dicts or None.
    Nothing is added to either side; if the recorded species cannot balance, it is None.
    """
    lel = set().union(*(c for c, _ in left.values()))
    rel = set().union(*(c for c, _ in right.values()))
    if lel != rel:
        return None
    lk, rk = sorted(left), sorted(right)
    cols = [(left[k], 1) for k in lk] + [(right[k], -1) for k in rk]
    keys = sorted(lel)
    rows = [[Fraction(sgn * cnt.get(e, 0)) for (cnt, _q), sgn in cols] for e in keys]
    if any(q for _, q in left.values()) or any(q for _, q in right.values()):
        rows.append([Fraction(sgn * q) for (_cnt, q), sgn in cols])
    n = len(cols)
    basis = _rref_nullspace(rows, n)
    coeffs = None
    if len(basis) == 1:
        v = basis[0]
        if all(x < 0 for x in v):
            v = [-x for x in v]
        if all(x > 0 for x in v):
            den = reduce(lambda a, b: a * b // gcd(a, b), (x.denominator for x in v), 1)
            ints = [int(x * den) for x in v]
            g = reduce(gcd, ints)
            ints = [x // g for x in ints]
            if max(ints) <= MAX_COEFF:
                coeffs = ints
    elif len(basis) > 1 and n <= BRUTE_MAX_SPECIES:
        best = None
        for cand in iproduct(range(1, BRUTE_MAX + 1), repeat=n):
            if reduce(gcd, cand) != 1:
                continue
            if all(sum(r[i] * cand[i] for i in range(n)) == 0 for r in rows):
                if best is None or sum(cand) < sum(best):
                    best = cand
        if best:
            coeffs = list(best)
    if coeffs is None:
        return None
    return dict(zip(lk, coeffs[: len(lk)])), dict(zip(rk, coeffs[len(lk):]))


def build_record(rid: str, source: str, reactants: list[str], agents: list[str],
                 products: list[str], conditions: dict) -> tuple[str, dict]:
    """Turn species SMILES lists into (dedup_key, record). Raises BadSpecies on bad input."""
    if not reactants or not products:
        raise BadSpecies("empty side")

    def side(smis):
        canon, formulas, info = [], Counter(), {}
        for s in smis:
            c, f, cnt, q = species_from_smiles(s)
            canon.append(c)
            formulas[f] += 1
            info[f] = (cnt, q)
        return sorted(canon), formulas, info

    rc, rf, rinfo = side(reactants)
    pc, pf, pinfo = side(products)
    ac = []
    for a in agents:  # agents are informational only; skip ones that do not parse
        try:
            ac.append(species_from_smiles(a)[0])
        except BadSpecies:
            pass
    ac = sorted(set(ac))
    key = ".".join(rc) + ">>" + ".".join(pc)
    bal = balance(rinfo, pinfo)
    if bal:
        rf, pf = Counter(bal[0]), Counter(bal[1])
    rec = {
        "id": rid,
        "reactants": dict(sorted(rf.items())),
        "products": dict(sorted(pf.items())),
        "smiles": ".".join(rc) + ">" + ".".join(ac) + ">" + ".".join(pc),
        "conditions": conditions,
        "source": source,
        "balanced": bool(bal),
    }
    return key, rec


def _selftest() -> None:
    assert hill_formula(Counter({"O": 1, "H": 2})) == "H2O"
    assert hill_formula(Counter({"C": 2, "H": 6, "O": 1})) == "C2H6O"
    assert hill_formula(Counter({"Na": 1, "Cl": 1})) == "ClNa"
    assert species_from_smiles("CC(=O)[O-]")[1] == "C2H3O2-"
    assert species_from_smiles("[Na+].[Cl-]")[1] == "ClNa"
    assert split_reaction_smiles("[CH3:1]O.C(=O)([O-])O.[Na+]>CO>CC |f:1.2|") == (
        ["[CH3:1]O", "C(=O)([O-])O.[Na+]"], ["CO"], ["CC"])
    k, r = build_record("t:1", "t", ["[H][H]", "[H][H]", "O=O"], [], ["O", "O"], {})
    assert r["balanced"] and r["reactants"] == {"H2": 2, "O2": 1} and r["products"] == {"H2O": 2}, r
    k, r = build_record("t:2", "t", ["CCO", "CC(=O)O"], [], ["CCOC(C)=O"], {})
    assert not r["balanced"] and r["reactants"] == {"C2H4O2": 1, "C2H6O": 1}, r
    k, r = build_record("t:3", "t", ["CCO", "CC(=O)O"], [], ["CCOC(C)=O", "O"], {})
    assert r["balanced"], r
    k, r = build_record("t:4", "t", ["[Fe]", "[Cl][Cl]"], [], ["Cl[Fe](Cl)Cl"], {})
    assert r["balanced"] and r["reactants"] == {"Cl2": 3, "Fe": 2}, r
    try:
        species_from_smiles("*CC")
        raise AssertionError("dummy atom accepted")
    except BadSpecies:
        pass
    print("chemlib selftest ok")


if __name__ == "__main__":
    _selftest()
