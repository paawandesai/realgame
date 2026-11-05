#!/usr/bin/env python3
"""Builds the Bench "reaction show" data: content/chem/substances.json and
content/chem/geometry.json (I-054 B1, docs/loop/items/I-054/spec.md).

Inputs, all machine-readable copies of published tables, read from the
`chemicals` package (Caleb Bell, MIT licence), plus RDKit and GFN2-xTB (tblite) for geometry:

- CRC Handbook of Chemistry and Physics, "Standard Thermodynamic Properties
  of Chemical Substances" (Hf of the solid, liquid and gas, kJ/mol);
- CRC Handbook, "Thermodynamic Properties of Aqueous Ions" (Hf(aq) of ions;
  a dissolved strong electrolyte is the sum of its ions);
- CRC Handbook, "Physical Constants of Inorganic/Organic Compounds"
  (melting point, boiling point, density) and the heats of fusion and
  vaporization tables;
- NIST Chemistry WebBook Shomate heat capacity coefficients (per phase);
- API TDB (Albahri) Hf of gases, for pentacosane (candle wax);
- RDKit ETKDGv3 conformers, optimised with GFN2-xTB, for every molecular
  species; ionic and lattice species get a
  representative ion cluster (`lattice: true`).

Values that are not in those files are entered by hand below, each with its
source and `verified: false` (entered from the printed table, not checked
against a machine-readable copy), or `estimated: true` with how. Colours are
hand-entered descriptions with a source; every hex is a rendering of the
described colour (`hex_estimated: true`), never a measurement.

Usage (a venv outside the repo):
  python -m venv /tmp/v && /tmp/v/bin/pip install chemicals rdkit tblite ase
  /tmp/v/bin/python -I tools/chemdb/build_show_data.py content/chem
"""
import csv
import json
import math
import os
import re
import sys

import chemicals

OUT = next((a for a in sys.argv[1:] if not a.startswith("--")), "content/chem")
BASE = os.path.dirname(chemicals.__file__)
TOKEN = re.compile(r"([A-Z][a-z]?)(\d*)")

SOURCES = {
    "CRC-T": "CRC Handbook of Chemistry and Physics, 'Standard Thermodynamic Properties of Chemical Substances' (section 5), as tabulated in the chemicals package (https://github.com/CalebBell/chemicals, file 'CRC Standard Thermodynamic Properties of Chemical Substances.tsv').",
    "CRC-ION": "CRC Handbook of Chemistry and Physics, 'Thermodynamic Properties of Aqueous Ions' (section 5), as tabulated in the chemicals package (file 'CRC Thermodynamic Properties of Aqueous Ions.tsv'). A dissolved strong electrolyte is the sum of its ions at infinite dilution.",
    "CRC-PHYS": "CRC Handbook of Chemistry and Physics, 'Physical Constants of Inorganic Compounds' and 'Physical Constants of Organic Compounds' (melting point, boiling point, density), as tabulated in the chemicals package.",
    "CRC-PRINTED": "CRC Handbook of Chemistry and Physics, 'Physical Constants of Inorganic Compounds', the printed boiling or sublimation point (KCl boils at 1420 C; CaO at 2850 C; ZnS sublimes at 1185 C). Entered by hand (verified: false).",
    "CRC-PC": "CRC Handbook of Chemistry and Physics, heat of fusion and heat of vaporization tables (section 6), as tabulated in the chemicals package.",
    "NIST-SHOMATE": "NIST Chemistry WebBook (SRD 69), Shomate heat capacity coefficients per phase, as tabulated in the chemicals package (file 'webbook_shomate_coefficients.json'). Cp = A + B T + C T^2 + D T^3 + E / T^2 in J/(mol K), T in K.",
    "API-TDB": "API Technical Data Book, gas-phase enthalpies of formation (Albahri), as tabulated in the chemicals package (file 'API TDB Albahri Hf (g).tsv').",
    "NBS82": "Wagman et al., 'The NBS tables of chemical thermodynamic properties', J. Phys. Chem. Ref. Data 11, Suppl. 2 (1982). Entered from the printed table (verified: false): not checked against a machine-readable copy; check at the science review.",
    "GHAZ": "Enthalpies of vaporization at 298 K, appendix of Ghazerati et al., as tabulated in the chemicals package (file 'Ghazerati Appendix Vaporization Enthalpy.tsv').",
    "RDKIT": "Computed: an RDKit 2026.09 ETKDGv3 starting conformer (random seed 61453), optimised in the gas phase with the GFN2-xTB semi-empirical method (tblite 0.4, ASE BFGS to 0.01 eV/A). A computed conformer, not a measured structure; small molecules come out within about 0.02 A of experiment (CO2 C=O 1.14 A against 1.16 A measured).",
    "ION-CLUSTER": "Representative ion cluster, not a crystal structure: ions on a simple cubic grid, cations and anions alternating, spacing from the CRC density (the cube root of the volume per ion). Polyatomic ions are RDKit conformers (see RDKIT).",
    "ATOM": "A single atom standing for one formula unit of a metal, covalent network solid or elemental lattice; spacing from the CRC density.",
    "COLOUR-GE": "Greenwood and Earnshaw, 'Chemistry of the Elements', 2nd ed. (1997), the description of each substance; cross-checked against the Wikipedia article for that substance (appearance in its infobox).",
    "COLOUR-FLAME": "Flame colours: Wikipedia, 'Flame test' (https://en.wikipedia.org/wiki/Flame_test): lithium crimson, sodium yellow-orange, potassium lilac, calcium orange-red, strontium red, barium pale green, copper blue-green; magnesium burns with a brilliant white light (https://en.wikipedia.org/wiki/Magnesium). Emission lines: NIST Atomic Spectra Database (Na 589.0 and 589.6 nm; Li 670.8 nm; K 766.5 and 769.9 nm, with 404.4 nm violet).",
    "COLOUR-HC": "Hydrocarbon flames: Gaydon, 'The Spectroscopy of Flames', 2nd ed. (1974): the blue of a premixed flame is CH* (431 nm) and C2* Swan band (516 nm) emission; the yellow of a diffusion (candle) flame is glowing soot, close to a blackbody. Hydrogen and methanol burn with an almost invisible pale flame (OH* emits at 306 nm, in the ultraviolet).",
}

# Hand-entered values not in the machine-readable tables (kJ/mol).
HAND_HF = {
    ("Al2(SO4)3", "s"): (-3440.84, "NBS82"),
    ("CaSiO3", "s"): (-1634.94, "NBS82"),
    ("Fe(OH)2", "s"): (-569.0, "NBS82"),
    ("Fe(OH)3", "s"): (-823.0, "NBS82"),
    # Rule tier (I-056): freshly precipitated aluminium hydroxide is amorphous.
    ("Al(OH)3", "s"): (-1276.0, "NBS82"),
    ("Mg3N2", "s"): (-461.08, "NBS82"),
    ("Li3N", "s"): (-164.56, "NBS82"),
    ("P4O10", "s"): (-2984.0, "NBS82"),
    ("H2CO3", "aq"): (-699.65, "NBS82"),
    ("H2SO3", "aq"): (-608.81, "NBS82"),
    ("NH4OH", "aq"): (-366.12, "NBS82"),
    ("NH3", "aq"): (-80.29, "NBS82"),
    ("CO2", "aq"): (-413.80, "NBS82"),
    ("SO2", "aq"): (-322.98, "NBS82"),
    ("CH3COOH", "aq"): (-485.76, "NBS82"),
    ("H2O2", "aq"): (-191.17, "NBS82"),
    ("Cl2", "aq"): (-23.4, "NBS82"),
    ("Br2", "aq"): (-2.59, "NBS82"),
    ("I2", "aq"): (22.6, "NBS82"),
    ("H3PO4", "aq"): (-1288.34, "NBS82"),
    ("C2H5OH", "aq"): (-288.3, "NBS82"),
    ("C6H12O6", "aq"): (-1262.19, "NBS82"),
}
ESTIMATED_HF = {
    ("NI3", "s"): (146.0, "No standard value: pure NI3 is too unstable to measure (the real solid is the ammonia adduct NI3.NH3). Estimated from the decomposition enthalpy 2NI3 -> N2 + 3I2, about -290 kJ/mol, quoted by Wikipedia 'Nitrogen triiodide'."),
    ("CuCO3", "s"): (-595.0, "Pure copper(II) carbonate is rarely made; the 'copper carbonate' sold is basic copper carbonate. Estimated: a value given for CuCO3 in older compilations, not in the CRC or NBS tables used here."),
}

# Ions of each dissolved strong electrolyte (formula -> {ion: count}).
IONS = {
    "NaOH": {"Na+": 1, "OH-": 1}, "KOH": {"K+": 1, "OH-": 1}, "LiOH": {"Li+": 1, "OH-": 1},
    "Ca(OH)2": {"Ca+2": 1, "OH-": 2}, "Ba(OH)2": {"Ba+2": 1, "OH-": 2}, "Sr(OH)2": {"Sr+2": 1, "OH-": 2},
    "HCl": {"H+": 1, "Cl-": 1}, "HBr": {"H+": 1, "Br-": 1}, "HNO3": {"H+": 1, "NO3-": 1},
    "H2SO4": {"H+": 2, "SO4-2": 1},
    "NaCl": {"Na+": 1, "Cl-": 1}, "KCl": {"K+": 1, "Cl-": 1}, "LiCl": {"Li+": 1, "Cl-": 1},
    "MgCl2": {"Mg+2": 1, "Cl-": 2}, "CaCl2": {"Ca+2": 1, "Cl-": 2}, "BaCl2": {"Ba+2": 1, "Cl-": 2},
    "AlCl3": {"Al+3": 1, "Cl-": 3}, "FeCl2": {"Fe+2": 1, "Cl-": 2}, "FeCl3": {"Fe+3": 1, "Cl-": 3},
    "CuCl2": {"Cu+2": 1, "Cl-": 2}, "ZnCl2": {"Zn+2": 1, "Cl-": 2},
    "NaBr": {"Na+": 1, "Br-": 1}, "KBr": {"K+": 1, "Br-": 1}, "KI": {"K+": 1, "I-": 1},
    "NaClO": {"Na+": 1, "ClO-": 1}, "Na2CO3": {"Na+": 2, "CO3-2": 1}, "NaHCO3": {"Na+": 1, "HCO3-": 1},
    "CH3COONa": {"Na+": 1, "CH3COO-": 1},
    "CuSO4": {"Cu+2": 1, "SO4-2": 1}, "FeSO4": {"Fe+2": 1, "SO4-2": 1}, "ZnSO4": {"Zn+2": 1, "SO4-2": 1},
    "MgSO4": {"Mg+2": 1, "SO4-2": 1}, "Na2SO4": {"Na+": 2, "SO4-2": 1}, "K2SO4": {"K+": 2, "SO4-2": 1},
    "Al2(SO4)3": {"Al+3": 2, "SO4-2": 3}, "(NH4)2SO4": {"NH4+": 2, "SO4-2": 1},
    "AgNO3": {"Ag+": 1, "NO3-": 1}, "NaNO3": {"Na+": 1, "NO3-": 1}, "KNO3": {"K+": 1, "NO3-": 1},
    "Cu(NO3)2": {"Cu+2": 1, "NO3-": 2}, "Zn(NO3)2": {"Zn+2": 1, "NO3-": 2},
    "Pb(NO3)2": {"Pb+2": 1, "NO3-": 2}, "NH4NO3": {"NH4+": 1, "NO3-": 1}, "NH4Cl": {"NH4+": 1, "Cl-": 1},
    # Science review (I-054): very soluble salts (CRC aqueous solubility).
    "ZnI2": {"Zn+2": 1, "I-": 2}, "AlI3": {"Al+3": 1, "I-": 3}, "KClO3": {"K+": 1, "ClO3-": 1},
}
# Other species that dissolve (as molecules) in the Bench's water.
DISSOLVES = {"NH3", "CO2", "SO2", "CH3COOH", "H2O2", "Cl2", "Br2", "I2", "H3PO4", "C2H5OH",
             "C6H12O6", "H2CO3", "H2SO3", "NH4OH",
             # Science review (I-054): sucrose (about 2000 g/L), HF, methanol and
             # propan-1-ol (miscible), urea (about 1000 g/L), water glass.
             "C12H22O11", "HF", "CH3OH", "C3H7OH", "CH4N2O", "Na2SiO3"}

# Lattice decomposition for geometry: formula -> list of (ion SMILES, count).
LATTICE = {
    "NaCl": [("[Na+]", 1), ("[Cl-]", 1)], "KCl": [("[K+]", 1), ("[Cl-]", 1)], "LiCl": [("[Li+]", 1), ("[Cl-]", 1)],
    "MgCl2": [("[Mg+2]", 1), ("[Cl-]", 2)], "CaCl2": [("[Ca+2]", 1), ("[Cl-]", 2)], "BaCl2": [("[Ba+2]", 1), ("[Cl-]", 2)],
    "AlCl3": [("[Al+3]", 1), ("[Cl-]", 3)], "FeCl2": [("[Fe+2]", 1), ("[Cl-]", 2)], "FeCl3": [("[Fe+3]", 1), ("[Cl-]", 3)],
    "CuCl2": [("[Cu+2]", 1), ("[Cl-]", 2)], "ZnCl2": [("[Zn+2]", 1), ("[Cl-]", 2)], "PbCl2": [("[Pb+2]", 1), ("[Cl-]", 2)],
    "AgCl": [("[Ag+]", 1), ("[Cl-]", 1)], "NaBr": [("[Na+]", 1), ("[Br-]", 1)], "KBr": [("[K+]", 1), ("[Br-]", 1)],
    "AgBr": [("[Ag+]", 1), ("[Br-]", 1)], "KI": [("[K+]", 1), ("[I-]", 1)], "AgI": [("[Ag+]", 1), ("[I-]", 1)],
    "PbI2": [("[Pb+2]", 1), ("[I-]", 2)], "AlI3": [("[Al+3]", 1), ("[I-]", 3)], "ZnI2": [("[Zn+2]", 1), ("[I-]", 2)],
    "NaH": [("[Na+]", 1), ("[H-]", 1)],
    "NaOH": [("[Na+]", 1), ("[OH-]", 1)], "KOH": [("[K+]", 1), ("[OH-]", 1)], "LiOH": [("[Li+]", 1), ("[OH-]", 1)],
    "Ca(OH)2": [("[Ca+2]", 1), ("[OH-]", 2)], "Mg(OH)2": [("[Mg+2]", 1), ("[OH-]", 2)], "Ba(OH)2": [("[Ba+2]", 1), ("[OH-]", 2)],
    "Sr(OH)2": [("[Sr+2]", 1), ("[OH-]", 2)], "Cu(OH)2": [("[Cu+2]", 1), ("[OH-]", 2)], "Fe(OH)2": [("[Fe+2]", 1), ("[OH-]", 2)],
    "Fe(OH)3": [("[Fe+3]", 1), ("[OH-]", 3)], "Zn(OH)2": [("[Zn+2]", 1), ("[OH-]", 2)],
    "CaO": [("[Ca+2]", 1), ("[O-2]", 1)], "MgO": [("[Mg+2]", 1), ("[O-2]", 1)], "ZnO": [("[Zn+2]", 1), ("[O-2]", 1)],
    "CuO": [("[Cu+2]", 1), ("[O-2]", 1)], "HgO": [("[Hg+2]", 1), ("[O-2]", 1)], "Li2O": [("[Li+]", 2), ("[O-2]", 1)],
    "Na2O": [("[Na+]", 2), ("[O-2]", 1)], "Ag2O": [("[Ag+]", 2), ("[O-2]", 1)], "Al2O3": [("[Al+3]", 2), ("[O-2]", 3)],
    "Fe2O3": [("[Fe+3]", 2), ("[O-2]", 3)],
    "FeS": [("[Fe+2]", 1), ("[S-2]", 1)], "MgS": [("[Mg+2]", 1), ("[S-2]", 1)], "ZnS": [("[Zn+2]", 1), ("[S-2]", 1)],
    "Cu2S": [("[Cu+]", 2), ("[S-2]", 1)], "Ag2S": [("[Ag+]", 2), ("[S-2]", 1)],
    "Mg3N2": [("[Mg+2]", 3), ("[N-3]", 2)], "Li3N": [("[Li+]", 3), ("[N-3]", 1)],
    "CaCO3": [("[Ca+2]", 1), ("[O-]C([O-])=O", 1)], "MgCO3": [("[Mg+2]", 1), ("[O-]C([O-])=O", 1)],
    "CuCO3": [("[Cu+2]", 1), ("[O-]C([O-])=O", 1)], "Na2CO3": [("[Na+]", 2), ("[O-]C([O-])=O", 1)],
    "NaHCO3": [("[Na+]", 1), ("OC([O-])=O", 1)], "CH3COONa": [("[Na+]", 1), ("CC([O-])=O", 1)],
    "CuSO4": [("[Cu+2]", 1), ("[O-]S([O-])(=O)=O", 1)], "FeSO4": [("[Fe+2]", 1), ("[O-]S([O-])(=O)=O", 1)],
    "ZnSO4": [("[Zn+2]", 1), ("[O-]S([O-])(=O)=O", 1)], "MgSO4": [("[Mg+2]", 1), ("[O-]S([O-])(=O)=O", 1)],
    "Na2SO4": [("[Na+]", 2), ("[O-]S([O-])(=O)=O", 1)], "K2SO4": [("[K+]", 2), ("[O-]S([O-])(=O)=O", 1)],
    "CaSO4": [("[Ca+2]", 1), ("[O-]S([O-])(=O)=O", 1)], "BaSO4": [("[Ba+2]", 1), ("[O-]S([O-])(=O)=O", 1)],
    "Al2(SO4)3": [("[Al+3]", 2), ("[O-]S([O-])(=O)=O", 3)], "(NH4)2SO4": [("[NH4+]", 2), ("[O-]S([O-])(=O)=O", 1)],
    "AgNO3": [("[Ag+]", 1), ("[O-][N+]([O-])=O", 1)], "NaNO3": [("[Na+]", 1), ("[O-][N+]([O-])=O", 1)],
    "KNO3": [("[K+]", 1), ("[O-][N+]([O-])=O", 1)], "Cu(NO3)2": [("[Cu+2]", 1), ("[O-][N+]([O-])=O", 2)],
    "Zn(NO3)2": [("[Zn+2]", 1), ("[O-][N+]([O-])=O", 2)], "Pb(NO3)2": [("[Pb+2]", 1), ("[O-][N+]([O-])=O", 2)],
    "NH4NO3": [("[NH4+]", 1), ("[O-][N+]([O-])=O", 1)], "NH4Cl": [("[NH4+]", 1), ("[Cl-]", 1)],
    "KClO3": [("[K+]", 1), ("[O-]Cl(=O)=O", 1)], "NaClO": [("[Na+]", 1), ("[O-]Cl", 1)],
    "Na2SiO3": [("[Na+]", 2), ("[O-][Si]([O-])=O", 1)], "CaSiO3": [("[Ca+2]", 1), ("[O-][Si]([O-])=O", 1)],
    "NH4OH": [("[NH4+]", 1), ("[OH-]", 1)],
}
# Single atoms standing for a lattice (metals, network solids, elemental solids).
ATOMS = {"Ag", "Al", "Ba", "C", "Ca", "Cu", "Fe", "K", "Li", "Mg", "Na", "Pb", "Si", "Sr", "Zn", "Hg", "S"}
ATOM_NOTE = {"S": "Rhombic sulfur is S8 rings; one atom stands for one formula unit S.",
             "C": "Charcoal is mostly disordered graphite-like carbon; one atom stands for one C.",
             "Hg": "Liquid mercury is monatomic.", "Si": "Silicon is a diamond-lattice network."}
SILICA = "SiO2"  # a network solid: one SiO2 unit cut from the network

SMILES = {
    "H2": "[H][H]", "O2": "O=O", "N2": "N#N", "O3": "[O-][O+]=O", "F2": "FF", "Cl2": "ClCl", "Br2": "BrBr",
    "I2": "II", "P4": "P12P3P1P23", "H2O": "O", "H2O2": "OO", "CO2": "O=C=O", "CO": "[C-]#[O+]",
    "CH4": "C", "C2H6": "CC", "C3H8": "CCC", "C4H10": "CCCC", "C5H12": "CCCCC", "C6H14": "CCCCCC",
    "C8H18": "CCCCCCCC", "C10H22": "CCCCCCCCCC", "C25H52": "C" * 25, "C2H4": "C=C", "C2H2": "C#C",
    "C6H6": "c1ccccc1", "CH3OH": "CO", "C2H5OH": "CCO", "C3H7OH": "CCCO",
    "C6H12O6": "OC[C@H]1O[C@H](O)[C@H](O)[C@@H](O)[C@@H]1O",
    "C12H22O11": "OC[C@H]1O[C@@](CO)(O[C@H]2O[C@H](CO)[C@@H](O)[C@H](O)[C@H]2O)[C@@H](O)[C@@H]1O",
    "CH3COOH": "CC(=O)O", "CH3COOC2H5": "CCOC(C)=O", "CH3Cl": "CCl", "C2H4Br2": "BrCCBr",
    "CH4N2O": "NC(N)=O", "CS2": "S=C=S", "SiH4": "[SiH4]", "NH3": "N", "NO": "[N]=O", "NO2": "[O][N]=O",
    "N2O4": "O=[N+]([O-])[N+]([O-])=O", "NI3": "IN(I)I", "HCl": "Cl", "HBr": "Br", "HF": "F",
    "HNO3": "O[N+](=O)[O-]", "H2SO4": "OS(=O)(=O)O", "H2SO3": "OS(=O)O", "H2CO3": "OC(=O)O",
    "H3PO4": "OP(=O)(O)O", "H2S": "S", "SO2": "O=S=O", "SO3": "O=S(=O)=O",
    "P4O10": "O=P12OP3(=O)OP(=O)(O1)OP(=O)(O2)O3",
    # Rule tier (I-056) fuels.
    "C7H16": "CCCCCCC", "C9H20": "CCCCCCCCC", "C11H24": "CCCCCCCCCCC", "C12H26": "CCCCCCCCCCCC",
    "C16H34": "CCCCCCCCCCCCCCCC", "C6H12": "C1CCCCC1", "C3H6": "C=CC", "C4H8": "C=CCC", "C5H10": "C=CCCC",
    "C3H4": "C#CC", "C4H9OH": "CCCCO", "C5H11OH": "CCCCCO",
}

# Colours: (state or "*") -> (described, hex or None, source). None hex: the
# colour is known but not rendered (colourless, or unsure of a hex).
C = "COLOUR-GE"
COLOURS = {
    "AgCl": {"s": ("white", "#f4f4f4", C)},
    "AgBr": {"s": ("cream", "#f1e6b0", C)},
    "AgI": {"s": ("pale yellow", "#f2df73", C)},
    "PbI2": {"s": ("bright yellow", "#ffd21f", C)},
    "PbCl2": {"s": ("white", "#f4f4f4", C)},
    "BaSO4": {"s": ("white", "#f4f4f4", C)},
    "CaCO3": {"s": ("white", "#f4f4f4", C)},
    "CaSO4": {"s": ("white", "#f4f4f4", C)},
    "Mg(OH)2": {"s": ("white", "#f4f4f4", C)},
    "Ca(OH)2": {"s": ("white", "#f4f4f4", C), "aq": ("colourless", None, C)},
    "Zn(OH)2": {"s": ("white", "#f4f4f4", C)},
    "Cu(OH)2": {"s": ("pale blue", "#8cc4ea", C)},
    "Fe(OH)2": {"s": ("dirty green", "#5b7f4f", C)},
    "Fe(OH)3": {"s": ("red-brown", "#a0522d", C)},
    "Cl2": {"g": ("pale yellow-green", "#c8d84a", C), "aq": ("very pale yellow-green", None, C)},
    "NO2": {"g": ("red-brown", "#a2401c", C)},
    "N2O4": {"g": ("colourless", None, C), "l": ("colourless to pale yellow", None, C)},
    "Br2": {"l": ("deep red-brown", "#6b1d0e", C), "g": ("red-brown", "#a3411f", C), "aq": ("yellow-orange", "#e8a33a", C)},
    "I2": {"s": ("shiny grey-black", "#2b2530", C), "g": ("violet", "#8a3fa0", C), "aq": ("brown", "#8b4513", C)},
    "F2": {"g": ("very pale yellow", None, C)},
    "O3": {"g": ("colourless at bench strength (pale blue when concentrated)", None, C)},
    "CuSO4": {"aq": ("blue", "#3a9ee0", C), "s": ("white", "#f4f4f4", C)},
    "Cu(NO3)2": {"aq": ("blue", "#3a9ee0", C), "s": ("blue", "#2f6fc0", C)},
    "CuCl2": {"aq": ("blue-green", "#2aa198", C), "s": ("yellow-brown", "#b5852a", C)},
    "FeSO4": {"aq": ("pale green", "#b8d8a0", C)},
    "FeCl2": {"aq": ("pale green", "#b8d8a0", C)},
    "FeCl3": {"aq": ("yellow-orange", "#d98c1e", C), "s": ("dark brown-black", "#3a2a1a", C)},
    "Cu": {"s": ("pink-brown", "#b87333", C)},
    "Ag": {"s": ("shiny silver", "#c0c0c0", C)},
    "Pb": {"s": ("dull grey", "#5a5f66", C)},
    "Zn": {"s": ("bluish grey", "#8e9aa6", C)},
    "Fe": {"s": ("grey", "#6e6e6e", C), "l": ("glowing orange", None, C)},
    "Mg": {"s": ("silvery", "#c8c8c8", C)},
    "Al": {"s": ("silvery", "#c8c8c8", C)},
    "Na": {"s": ("silvery", "#c8c8c8", C)},
    "K": {"s": ("silvery", "#c8c8c8", C)},
    "Li": {"s": ("silvery", "#c8c8c8", C)},
    "Ca": {"s": ("silvery grey", "#b0b0b0", C)},
    "Hg": {"l": ("silvery", "#c0c0c8", C)},
    "S": {"s": ("bright yellow", "#f2d72e", C)},
    "C": {"s": ("black", "#1c1c1c", C)},
    "Si": {"s": ("shiny grey", "#6a6f78", C)},
    "P4": {"s": ("waxy pale yellow", "#f2ecc8", C)},
    "CuO": {"s": ("black", "#1c1c1c", C)},
    "Cu2S": {"s": ("black", "#1c1c1c", C)},
    "CuCO3": {"s": ("green", "#3f8f4f", C)},
    "Fe2O3": {"s": ("red-brown", "#8b3a1e", C)},
    "FeS": {"s": ("black", "#1c1c1c", C)},
    "Ag2S": {"s": ("black", "#1c1c1c", C)},
    "Ag2O": {"s": ("brown-black", "#3b2a20", C)},
    "HgO": {"s": ("red", "#c0392b", C)},
    "ZnO": {"s": ("white", "#f4f4f4", C)},
    "MgO": {"s": ("white", "#f4f4f4", C)},
    "CaO": {"s": ("white", "#f4f4f4", C)},
    "Al2O3": {"s": ("white", "#f4f4f4", C)},
    "SiO2": {"s": ("white", "#f4f4f4", C)},
    "P4O10": {"s": ("white", "#f4f4f4", C)},
    "Li2O": {"s": ("white", "#f4f4f4", C)},
    "Na2O": {"s": ("white", "#f4f4f4", C)},
    "NH4Cl": {"s": ("white", "#f4f4f4", C)},
    "Mg3N2": {"s": ("greenish-yellow", "#c9c46a", C)},
    "Li3N": {"s": ("red-brown", "#8a3b2a", C)},
    "NI3": {"s": ("black-purple", "#2a1830", C)},
    "MgCl2": {"s": ("white", "#f4f4f4", C)}, "NaCl": {"s": ("white", "#f4f4f4", C)},
    "KCl": {"s": ("white", "#f4f4f4", C)}, "CaCl2": {"s": ("white", "#f4f4f4", C)},
    "AlCl3": {"s": ("white to pale yellow", "#f4f4f4", C)}, "NaBr": {"s": ("white", "#f4f4f4", C)},
    "ZnI2": {"s": ("white", "#f4f4f4", C)}, "AlI3": {"s": ("white", "#f4f4f4", C)},
    "MgS": {"s": ("white", "#f4f4f4", C)}, "ZnS": {"s": ("white", "#f4f4f4", C)},
    "C25H52": {"s": ("white", "#f4f4f0", C)},
    # Science review (I-054), nit 19.
    "Ba": {"s": ("silvery", "#c8c8c8", C)}, "Sr": {"s": ("silvery", "#c8c8c8", C)},
    "MgCO3": {"s": ("white", "#f4f4f4", C)}, "KClO3": {"s": ("white", "#f4f4f4", C)},
    "C12H22O11": {"s": ("white", "#f4f4f4", C)}, "C6H12O6": {"s": ("white", "#f4f4f4", C)},
    "CH4N2O": {"s": ("white", "#f4f4f4", C)}, "CaSiO3": {"s": ("white", "#f4f4f4", C)},
    "Na2SiO3": {"s": ("white", "#f4f4f4", C)}, "NaH": {"s": ("grey-white", "#d8d8d4", C)},
    # Rule tier (I-056): precipitates and coloured solutions.
    "Ag2CO3": {"s": ("pale yellow", "#f2e6a6", C)},
    "BaCO3": {"s": ("white", "#f4f4f4", C)}, "ZnCO3": {"s": ("white", "#f4f4f4", C)},
    "FeCO3": {"s": ("white, turning green-brown in air", "#e6e4da", C)},
    "Al(OH)3": {"s": ("white", "#f4f4f4", C)}, "AlPO4": {"s": ("white", "#f4f4f4", C)},
    "Ca3(PO4)2": {"s": ("white", "#f4f4f4", C)},
    "CuBr2": {"aq": ("blue", "#3a9ee0", C), "s": ("black", "#1c1c1c", C)},
    "Cu(CH3COO)2": {"aq": ("blue", "#3a9ee0", C), "s": ("blue-green", "#2e8b7a", C)},
    "FeBr2": {"aq": ("pale green", "#b8d8a0", C)}, "Fe(NO3)2": {"aq": ("pale green", "#b8d8a0", C)},
    "Fe(NO3)3": {"aq": ("yellow-brown", "#c8892a", C)}, "Fe2(SO4)3": {"aq": ("yellow-brown", "#c8892a", C)},
    "FeBr3": {"aq": ("red-brown", "#9c4a1e", C)},
}
COLOURLESS_GASES = ["H2", "O2", "N2", "CO2", "CO", "CH4", "C2H6", "C3H8", "C4H10", "C2H4", "C2H2", "NH3",
                    "HCl", "HF", "HBr", "SO2", "H2S", "NO", "SiH4", "CH3Cl",
                    # Rule tier (I-056) fuels.
                    "C3H6", "C4H8", "C3H4"]
COLOURLESS_LIQUIDS = ["H2O", "H2O2", "C2H5OH", "CH3OH", "C3H7OH", "CH3COOH", "CH3COOC2H5", "C5H12",
                      "C6H14", "C8H18", "C10H22", "C6H6", "CS2", "H2SO4", "HNO3", "C2H4Br2",
                      # Science review (I-054): SO3 melts at 290 K and boils at 318 K.
                      "SO3",
                      # Rule tier (I-056) fuels.
                      "C7H16", "C9H20", "C11H24", "C12H26", "C16H34", "C6H12", "C5H10", "C4H9OH", "C5H11OH"]
# Precipitate grain (how a solid that forms in a solution settles): cloudy =
# a fine or gelatinous suspension that settles slowly; grainy = crystals or a
# metal deposit that settles fast. Source: the same descriptions (COLOUR-GE).
# PbI2 mixed from cold solutions is a fine suspension; its glittering plates
# come only from a hot solution cooling (science review, I-054).
GRAIN = {"Ca(OH)2": "cloudy", "AgCl": "cloudy", "AgBr": "cloudy", "AgI": "cloudy", "BaSO4": "cloudy", "CaCO3": "cloudy",
         "Mg(OH)2": "cloudy", "Zn(OH)2": "cloudy", "Cu(OH)2": "cloudy", "Fe(OH)2": "cloudy",
         "Fe(OH)3": "cloudy", "CaSO4": "cloudy", "PbI2": "cloudy", "PbCl2": "grainy", "Cu": "grainy",
         "Ag": "grainy", "Pb": "grainy", "Zn": "grainy", "I2": "grainy",
         # Rule tier (I-056): fresh precipitates are fine or gelatinous.
         "Ag2CO3": "cloudy", "BaCO3": "cloudy", "ZnCO3": "cloudy", "FeCO3": "cloudy", "Al(OH)3": "cloudy",
         "AlPO4": "cloudy", "Ca3(PO4)2": "cloudy"}

# Boiling or sublimation points missing from the CRC physical constants file
# (K, at 1 atm), from the CRC Handbook's printed table (verified: false).
# They cap the adiabatic temperature of a product.
HAND_BP = {"KCl": 1693.0, "CaO": 3123.0, "ZnS": 1458.0}

EXTRA_CAS = {"AgI": "7783-96-2", "Al2O3": "1344-28-1", "Ba(OH)2": "17194-00-2", "C": "7782-42-5",
             "Li2O": "12057-24-8", "Na2O": "1313-59-3", "SiO2": "14808-60-7", "Cu(OH)2": "20427-59-2",
             "C6H12O6": "26655-34-5", "P4O10": "16752-60-6", "Mg3N2": "12057-71-5", "CaSiO3": "10101-39-0",
             "Al2(SO4)3": "10043-01-3", "C25H52": "629-99-2", "Li3N": "26134-62-3", "NaClO": "7681-52-9",
             "Fe(OH)3": "1309-33-7", "Fe(OH)2": "18624-44-7", "CuCO3": "1184-64-1", "NI3": "13444-85-4"}

# Atomic masses (IUPAC 2021 standard atomic weights, abridged).
MASS = {"H": 1.008, "Li": 6.94, "C": 12.011, "N": 14.007, "O": 15.999, "F": 18.998, "Na": 22.990,
        "Mg": 24.305, "Al": 26.982, "Si": 28.085, "P": 30.974, "S": 32.06, "Cl": 35.45, "K": 39.098,
        "Ca": 40.078, "Fe": 55.845, "Cu": 63.546, "Zn": 65.38, "Br": 79.904, "Sr": 87.62, "Ag": 107.87,
        "I": 126.90, "Ba": 137.33, "Hg": 200.59, "Pb": 207.2}

# Ion SMILES for the lattice clusters of the rule tier's salts (CRC-ION ion
# names, as content/chem/rules.json writes them).
ION_SMILES = {
    "Na+": "[Na+]", "K+": "[K+]", "Li+": "[Li+]", "NH4+": "[NH4+]", "Ag+": "[Ag+]", "Mg+2": "[Mg+2]",
    "Ca+2": "[Ca+2]", "Ba+2": "[Ba+2]", "Zn+2": "[Zn+2]", "Cu+2": "[Cu+2]", "Fe+2": "[Fe+2]", "Fe+3": "[Fe+3]",
    "Al+3": "[Al+3]", "Pb+2": "[Pb+2]", "Cl-": "[Cl-]", "Br-": "[Br-]", "I-": "[I-]", "OH-": "[OH-]", "O-2": "[O-2]",
    "NO3-": "[O-][N+]([O-])=O", "SO4-2": "[O-]S([O-])(=O)=O", "CO3-2": "[O-]C([O-])=O", "HCO3-": "OC([O-])=O",
    "PO4-3": "[O-]P([O-])([O-])=O", "CH3COO-": "CC([O-])=O",
}

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from show_names import NAMES  # noqa: E402  formula -> chemical name for the CAS lookup


def load_rules(path):
    """Folds the rule tier (content/chem/rules.json, written by
    build_rules.py) into the tables: its new species' names and CAS numbers,
    the ions of every soluble salt (Hf(aq) is the sum of its ions) and the
    lattice of every salt."""
    if not os.path.exists(path):
        return
    r = json.load(open(path))
    for f, sp in r.get("species", {}).items():
        NAMES.setdefault(f, sp["name"])
        if sp.get("cas"):
            EXTRA_CAS.setdefault(f, sp["cas"])
    for f, salt in r["tables"]["salts"].items():
        ions = {salt["cation"]: salt["n"], salt["anion"]: salt["m"]}
        if f in r.get("species", {}) and r["tables"]["solubility"].get(f) == "soluble" and "O-2" not in ions:
            IONS.setdefault(f, ions)
        LATTICE.setdefault(f, [(ION_SMILES[salt["cation"]], salt["n"]), (ION_SMILES[salt["anion"]], salt["m"])])


load_rules(os.path.join(OUT, "rules.json"))


def atoms(formula):
    """Atom counts of a formula with one level of brackets: (NH4)2SO4."""
    out = {}
    def add(s, k):
        for el, n in TOKEN.findall(s):
            out[el] = out.get(el, 0) + k * int(n or 1)
    rest = formula
    for m in re.finditer(r"\(([^)]+)\)(\d*)", formula):
        add(m.group(1), int(m.group(2) or 1))
    rest = re.sub(r"\(([^)]+)\)(\d*)", "", formula)
    add(rest, 1)
    return out


def tsv(path, key="CAS"):
    with open(path, newline="") as f:
        return {r[key]: r for r in csv.DictReader(f, delimiter="\t")}


def num(s):
    try:
        return float(s)
    except (TypeError, ValueError):
        return None


def species():
    """Every species of the codex: the core tier and the rule tier."""
    core = json.load(open(os.path.join(OUT, "core.json")))
    reactions = list(core["reactions"])
    rpath = os.path.join(OUT, "rules.json")
    if os.path.exists(rpath):
        reactions += json.load(open(rpath))["reactions"]
    out = set()
    for r in reactions:
        for side in r["equation"].split("->"):
            for t in side.split("+"):
                out.add(re.match(r"\s*\d*(.*?)\s*$", t).group(1))
    return sorted(out)


def embed(smiles, seed=61453):
    """An RDKit ETKDGv3 conformer, optimised with GFN2-xTB (gas phase)."""
    # Imported here so the data tables can be rebuilt with only `chemicals`
    # installed when every conformer is already in geometry.json.
    from rdkit import Chem
    from rdkit.Chem import AllChem
    from ase import Atoms
    from ase.optimize import BFGS
    from tblite.ase import TBLite
    m = Chem.AddHs(Chem.MolFromSmiles(smiles))
    if m.GetNumAtoms() == 1:
        a = m.GetAtomWithIdx(0)
        return [a.GetSymbol()], [[0.0, 0.0, 0.0]], [], "single atom"
    p = AllChem.ETKDGv3()
    p.randomSeed = seed
    if AllChem.EmbedMolecule(m, p) != 0:
        p.useRandomCoords = True
        AllChem.EmbedMolecule(m, p)
    conf = m.GetConformer()
    els = [a.GetSymbol() for a in m.GetAtoms()]
    pos = [list(conf.GetAtomPosition(i)) for i in range(m.GetNumAtoms())]
    charge = Chem.GetFormalCharge(m)
    electrons = sum(a.GetAtomicNum() for a in m.GetAtoms()) - charge
    mult = 3 if smiles == "O=O" else (2 if electrons % 2 else 1)
    atoms_ = Atoms(els, positions=pos)
    atoms_.calc = TBLite(method="GFN2-xTB", charge=charge, multiplicity=mult, verbosity=0)
    BFGS(atoms_, logfile=None).run(fmax=0.01, steps=1000)
    pos = [[round(float(c), 3) for c in p_] for p_ in atoms_.get_positions()]
    bonds = []
    for b in m.GetBonds():
        bonds.append([b.GetBeginAtomIdx(), b.GetEndAtomIdx(), b.GetBondTypeAsDouble()])
    return els, pos, no_false_aromatic(els, bonds), "GFN2-xTB"


def no_false_aromatic(els, bonds):
    """RDKit perceives the P4 tetrahedron as aromatic (order 1.5); white
    phosphorus has six P-P single bonds. Aromatic orders are kept only in
    molecules with carbon (benzene)."""
    if "C" in els:
        return bonds
    return [[a, b, 1.0 if o == 1.5 else o] for a, b, o in bonds]



def centre(pos):
    n = len(pos)
    c = [sum(p[i] for p in pos) / n for i in range(3)]
    return [[round(p[i] - c[i], 3) for i in range(3)] for p in pos]


def tables():
    """The machine-readable tables every substance record is built from."""
    T = {}
    T["crc"] = tsv(os.path.join(BASE, "Heat Capacity", "CRC Standard Thermodynamic Properties of Chemical Substances.tsv"))
    T["ions"] = tsv(os.path.join(BASE, "Electrolytes", "CRC Thermodynamic Properties of Aqueous Ions.tsv"), key="Formula")
    phys = {}
    for f in ["Physical Constants of Inorganic Compounds.csv", "Physical Constants of Organic Compounds.csv"]:
        phys.update(tsv(os.path.join(BASE, "Misc", f)))
    T["phys"] = phys
    T["hfus"] = tsv(os.path.join(BASE, "Phase Change", "CRC Handbook Heat of Fusion.tsv"))
    T["hvap"] = tsv(os.path.join(BASE, "Phase Change", "CRC Handbook Heat of Vaporization.tsv"))
    T["api"] = tsv(os.path.join(BASE, "Reactions", "API TDB Albahri Hf (g).tsv"))
    T["shomate"] = json.load(open(os.path.join(BASE, "Heat Capacity", "webbook_shomate_coefficients.json")))
    T["ghaz"] = tsv(os.path.join(BASE, "Phase Change", "Ghazerati Appendix Vaporization Enthalpy.tsv"))
    return T


def ion_table(T, ions):
    """Hf(aq) of every ion in `ions` (CRC-ION)."""
    out = {}
    for ion in sorted(ions):
        row = T["ions"][ion]
        out[ion] = {"dhf": round(float(row["Hf(aq)"]) / 1000, 2), "src": "CRC-ION", "name": row["Name"]}
    return out


def cas_of(f):
    from chemicals.identifiers import CAS_from_any
    if f in EXTRA_CAS:
        return EXTRA_CAS[f]
    try:
        return CAS_from_any(NAMES[f])
    except Exception:  # noqa: BLE001 - not every salt has an identifier entry
        return None


def substance_records(fs, T):
    """The substances.json record of every formula in `fs`."""
    crc, phys, hfus, hvap, api, shomate, ghaz = (T[k] for k in ("crc", "phys", "hfus", "hvap", "api", "shomate", "ghaz"))
    ion_hf = ion_table(T, {i for d in IONS.values() for i in d})
    subs = {}
    for f in fs:
        cas = cas_of(f)
        mm = round(sum(MASS[el] * n for el, n in atoms(f).items()), 3)
        rec = {"cas": cas, "name": NAMES[f], "molar_mass": mm}
        t = crc.get(cas, {})
        dhf = {}
        for st, col in (("s", "Hfs"), ("l", "Hfl"), ("g", "Hfg")):
            v = num(t.get(col))
            if v is not None:
                dhf[st] = {"v": round(v / 1000, 2), "src": "CRC-T"}
        for (hf_f, st), (v, src) in HAND_HF.items():
            if hf_f == f and st not in dhf:
                dhf[st] = {"v": v, "src": src, "verified": False}
        for (hf_f, st), (v, how) in ESTIMATED_HF.items():
            if hf_f == f:
                dhf[st] = {"v": v, "estimated": True, "how": how}
        if f in IONS:
            v = sum(ion_hf[i]["dhf"] * n for i, n in IONS[f].items())
            dhf["aq"] = {"v": round(v, 2), "src": "CRC-ION", "ions": IONS[f]}
        if f == "C25H52":
            g = float(api[cas]["Hfg"]) / 1000
            hv = float(ghaz[cas]["Hvap298"]) / 1000
            hf_ = float(hfus[cas]["Hfus"]) / 1000
            dhf["s"] = {"v": round(g - hv - hf_, 1), "estimated": True,
                        "how": f"Hf(g) {g:.1f} (API-TDB) minus Hvap(298 K) {hv:.1f} (GHAZ) and Hfus {hf_:.1f} at the melting point (CRC-PC); the fusion term is not at 298 K and solid-solid transitions are left out, so this is an estimate."}
        rec["dhf"] = dhf
        p = phys.get(cas, {})
        tm, tb, rho = num(p.get("Tm")), num(p.get("Tb")), num(p.get("rho"))
        if tm:
            rec["mp_k"] = tm
        if tb:
            rec["bp_k"] = tb
        if rho:
            rec["density_kg_m3"] = rho
        if tm or tb or rho:
            rec["phys_src"] = "CRC-PHYS"
        if f in HAND_BP and not tb:
            rec["bp_k"] = HAND_BP[f]
            rec["bp_src"] = "CRC-PRINTED"
        if cas in hfus and num(hfus[cas].get("Hfus")):
            rec["hfus_kj"] = round(float(hfus[cas]["Hfus"]) / 1000, 2)
        if cas in hvap and num(hvap[cas].get("HvapTb")):
            rec["hvap_kj"] = round(float(hvap[cas]["HvapTb"]) / 1000, 2)
        sh = shomate.get(cas)
        if sh:
            rec["cp"] = {st: segs for st, segs in zip("slg", sh) if segs}
            rec["cp_src"] = "NIST-SHOMATE"
        # State at 298.15 K and 1 atm.
        if f in ("Br2", "Hg", "H2O"):
            state = "l"
        elif f in ("NH4OH", "H2CO3", "H2SO3", "NaClO"):
            state = "aq"
        elif tb is not None and tb < 298.15 or (f in COLOURLESS_GASES + ["Cl2", "NO2", "F2", "O3"]):
            state = "g"
        elif tm is not None and tm > 298.15:
            state = "s"
        elif tm is not None and tm <= 298.15:
            state = "l"
        else:
            state = "s"
        if f == "N2O4":
            state = "g"
        rec["state"] = state
        rec["soluble"] = f in IONS or f in DISSOLVES
        col = {}
        for st, (desc, hx, src) in COLOURS.get(f, {}).items():
            c = {"described": desc, "src": src}
            if hx:
                c["hex"] = hx
                c["hex_estimated"] = True
            col[st] = c
        if f in COLOURLESS_GASES:
            col.setdefault("g", {"described": "colourless", "src": C})
        if f in COLOURLESS_LIQUIDS:
            col.setdefault("l", {"described": "colourless", "src": C})
        if f in IONS and "aq" not in col:
            col["aq"] = {"described": "colourless", "src": C}
        if col:
            rec["colour"] = col
        if f in GRAIN:
            rec["grain"] = GRAIN[f]
        subs[f] = rec
    return subs, ion_hf


def main():
    T = tables()
    subs, ion_hf = substance_records(species(), T)

    # Geometry. Conformers already in geometry.json are kept (the xTB
    # optimisation is slow); pass --regeometry to recompute them all.
    geo = {}
    old = {}
    gpath = os.path.join(OUT, "geometry.json")
    if os.path.exists(gpath) and "--regeometry" not in sys.argv:
        old = json.load(open(gpath))["species"]
    for f in species():
        if f in old and (old[f].get("method") in ("GFN2-xTB", "monatomic ions") or old[f]["src"] == "ATOM"):
            geo[f] = old[f]
            if old[f]["src"] == "RDKIT":
                geo[f]["bonds"] = no_false_aromatic(old[f]["elements"], old[f]["bonds"])
            continue
        print("geometry", f, file=sys.stderr, flush=True)
        rho = subs[f].get("density_kg_m3")
        mm = subs[f]["molar_mass"]
        if f in ATOMS:
            el = re.match(r"[A-Z][a-z]?", f).group(0)
            g = {"elements": [el], "positions": [[0.0, 0.0, 0.0]], "bonds": [], "lattice": f != "Hg",
                 "src": "ATOM"}
            if f in ATOM_NOTE:
                g["note"] = ATOM_NOTE[f]
        elif f == SILICA:
            # One Si with two of its four network oxygens: Si-O 1.61 A, O-Si-O
            # tetrahedral, from the quartz structure (Greenwood and Earnshaw).
            a = math.radians(109.47 / 2)
            g = {"elements": ["Si", "O", "O"],
                 "positions": [[0.0, 0.0, 0.0], [round(1.61 * math.sin(a), 3), round(1.61 * math.cos(a), 3), 0.0],
                               [round(-1.61 * math.sin(a), 3), round(1.61 * math.cos(a), 3), 0.0]],
                 "bonds": [[0, 1, 1.0], [0, 2, 1.0]], "lattice": True,
                 "src": "ATOM", "note": "One SiO2 unit of the quartz network: Si-O 1.61 A, O-Si-O 109.5 degrees (Greenwood and Earnshaw)."}
        elif f in LATTICE:
            from rdkit import Chem
            parts = []
            for smi, n in LATTICE[f]:
                els, pos, bonds, ff = embed(smi)
                for _ in range(n):
                    parts.append((smi, els, centre(pos), bonds, Chem.MolFromSmiles(smi).GetAtomWithIdx(0).GetFormalCharge() > 0 or smi in ("[NH4+]",)))
            nions = len(parts)
            spacing = 3.0
            if rho:
                spacing = round((mm / 1000 / rho / 6.02214076e23 / nions) ** (1 / 3) * 1e10, 3)
            pts = sorted([(i, j, k) for i in range(-2, 3) for j in range(-2, 3) for k in range(-2, 3)],
                         key=lambda p: (p[0] ** 2 + p[1] ** 2 + p[2] ** 2, p))
            cats = [p for p in parts if p[4]]
            ans = [p for p in parts if not p[4]]
            even = [p for p in pts if sum(p) % 2 == 0]
            odd = [p for p in pts if sum(p) % 2 != 0]
            els_all, pos_all, bonds_all, centres, kinds = [], [], [], [], []
            for group, slots in ((cats, even), (ans, odd)):
                for (smi, els, pos, bonds, _), slot in zip(group, slots):
                    off = len(els_all)
                    c = [s * spacing for s in slot]
                    els_all += els
                    pos_all += [[round(p[i] + c[i], 3) for i in range(3)] for p in pos]
                    bonds_all += [[a + off, b + off, o] for a, b, o in bonds]
                    centres.append((off, c, len(els)))
            # Ionic contacts (order 0): each cation to the nearest atom of each
            # neighbouring anion (centre distance within 1.01 grid steps).
            ncat = len(cats)
            for ci in range(ncat):
                coff, cc, _ = centres[ci]
                for ai in range(ncat, len(centres)):
                    aoff, ac, an = centres[ai]
                    if math.dist(cc, ac) <= spacing * 1.01:
                        near = min(range(aoff, aoff + an), key=lambda k: math.dist(pos_all[k], pos_all[coff]))
                        bonds_all.append([coff, near, 0.0])
            g = {"elements": els_all, "positions": centre(pos_all), "bonds": bonds_all, "lattice": True,
                 "src": "ION-CLUSTER", "spacing_a": spacing,
                 "method": "GFN2-xTB" if any(len(p[1]) > 1 for p in parts) else "monatomic ions"}
        else:
            els, pos, bonds, ff = embed(SMILES[f])
            g = {"elements": els, "positions": centre(pos), "bonds": bonds, "lattice": False,
                 "src": "RDKIT", "method": ff}
        if rho and g["lattice"] and "spacing_a" not in g:
            g["spacing_a"] = round((mm / 1000 / rho / 6.02214076e23) ** (1 / 3) * 1e10, 3)
        got = {}
        for e in g["elements"]:
            got[e] = got.get(e, 0) + 1
        assert got == atoms(f), (f, got, atoms(f))
        g["id"] = f"{f}#{g['src'].lower()}-1"
        geo[f] = g

    about = ("Substances of the Bench's reaction show records (I-054, docs/loop/items/I-054/spec.md). Generated by "
             "tools/chemdb/build_show_data.py; edit that script, not this file. dhf: standard enthalpy of formation at "
             "298.15 K and 1 bar in kJ/mol per phase (s, l, g, aq); src names an entry of sources, verified false means "
             "entered from a printed table and not checked against a machine-readable copy, estimated true says how. "
             "state: the phase at 298.15 K and 1 atm. cp: NIST Shomate coefficients per phase, [[Tmin K, Tmax K, A, B, C, "
             "D, E], ...] with Cp = A + B T + C T^2 + D T^3 + E/T^2 in J/(mol K). colour: per phase, a described colour "
             "with its source; hex is an sRGB rendering of the description (hex_estimated), never a measurement; a "
             "colour with no hex is known but not rendered. grain: how a solid that forms in a solution settles.")
    json.dump({"version": 1, "about": about, "check": "draft", "sources": SOURCES, "ions": ion_hf,
               "substances": subs}, open(os.path.join(OUT, "substances.json"), "w"), indent=1, ensure_ascii=False)
    gabout = ("Geometry of every codex species (I-054). positions in angstroms, centred on the centroid; bonds "
              "[i, j, order] with order 1, 1.5 (aromatic), 2, 3, or 0 for an ionic contact. lattice true: the "
              "species is an ionic, metallic or network solid and this is a representative unit, not a molecule; "
              "spacing_a is the cube root of the volume per formula unit (or per ion), from the CRC density. "
              "Generated by tools/chemdb/build_show_data.py.")
    with open(os.path.join(OUT, "geometry.json"), "w") as fh:
        fh.write('{"version":1,"about":' + json.dumps(gabout) + ',"sources":' +
                 json.dumps({k: SOURCES[k] for k in ("RDKIT", "ION-CLUSTER", "ATOM")}) + ',"species":{\n')
        fh.write(",\n".join(json.dumps(k) + ":" + json.dumps(v, separators=(",", ":")) for k, v in geo.items()))
        fh.write("\n}}\n")
    print(len(subs), "substances,", len(geo), "geometries")


if __name__ == "__main__":
    main()
