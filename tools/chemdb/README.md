# chemdb: the Reference tier reaction database

This pipeline builds the codex's "Reference tier" from published open reaction datasets.
It writes one gzip-compressed JSON-lines file with one record per unique reaction, a
formula index, and a stats report. The scripts, this README and `stats.json` are kept in
git. The data files are not, because they are larger than 20 MB (see "Where the data lives").

## Sources and licences

| Source | Licence | Status in the current build |
|---|---|---|
| Daniel Lowe, *Chemical reactions from US patents (1976-Sep2016)*, figshare article 5104873 (doi:10.6084/m9.figshare.5104873) | CC0 1.0 | **Not reached.** The build environment's egress proxy refuses `api.figshare.com`, `figshare.com` and `ndownloader.figshare.com` with a 403 policy denial. The importer (`run_uspto`) is written and runs as soon as figshare is reachable. It has not yet been run against the real files. |
| Open Reaction Database, `github.com/open-reaction-database/ord-data` (doi:10.5281/zenodo.4321713) | **CC BY-SA 4.0** | Imported in full: all 93 datasets at the ord-data commit recorded in `stats.json`. |

### ORD attribution and share-alike

ORD-derived records contain data from the Open Reaction Database:

> Kearnes, S. M. et al. *The Open Reaction Database.* J. Am. Chem. Soc. 2021, 143, 18820–18826.
> doi:10.1021/jacs.1c09820. Data: https://github.com/open-reaction-database/ord-data, CC BY-SA 4.0.

Every ORD record has `"source":"ORD CC-BY-SA-4.0"`, and its `id` is `ord:<ORD reaction_id>`,
so each reaction can be traced back to the original record. `stats.json` → `datasets`
lists every ORD dataset used, with its ID, name and path. Those entries are the
per-dataset attribution.

**Share-alike:** `reactions.jsonl.gz` and `formula_pairs.json.gz` are adaptations of ORD.
Anyone who redistributes them, or a derivative of them, must:

- release that copy under CC BY-SA 4.0 (or a compatible licence),
- keep the attribution above, and
- say that changes were made.

The changes are: formulas computed, duplicates removed, and records reshaped into this schema.
This applies to the data only. The game code stays under its own licence. If a file must
not carry share-alike terms, build it from `--sources uspto` (CC0) alone.

### Lowe's USPTO data inside ORD

ORD's `uspto-grants-YYYY` datasets are ORD's import of Lowe's USPTO grant reactions.
In the current build they make up most of the ORD records. Lowe's data is CC0, but we
obtained it through ORD, so we label these records as ORD (CC BY-SA). That is the
conservative choice. Once figshare is reachable, run the default `--sources uspto,ord`.
USPTO is then processed first, the same reactions are kept with
`"source":"USPTO-Lowe-CC0"` and `uspto:<n>` ids, and the ORD copies are dropped as
duplicates. Lowe's set also contains the patent *applications* (2001–2016), which ORD
does not include.

## Record schema (`reactions.jsonl.gz`)

```json
{"id":"ord:ord-…","reactants":{"C7H6O2":1,"C2H6O":1},"products":{"C9H10O2":1,"H2O":1},
 "smiles":"<reactants>.<…>><agents>><products>",
 "conditions":{"temp_c":80.0,"catalyst":"sulfuric acid","solvent":"toluene"},
 "source":"ORD CC-BY-SA-4.0","balanced":true}
```

- **Species.** A species is one molecule as the source records it. A salt such as
  `C(=O)([O-])O.[Na+]` is one species. For CXSMILES, the `|f:…|` fragment groups are honoured.
- **Formula.** Formulas are in Hill order: C, then H, then the other elements alphabetically.
  With no carbon, every element is alphabetical (`ClNa`, `H2O`). A non-zero net charge is
  added as a suffix (`C2H3O2-`). Hydrogens are exactly the ones the SMILES implies: explicit
  H plus RDKit's implicit valence H. Nothing is added or guessed. A reaction is dropped
  (`rejected` in stats) in three cases:
  - a species does not sanitize in RDKit,
  - a species contains a dummy atom or R-group,
  - a species has no machine-readable structure (name only).
- **Counts and `balanced`.** The builder looks for the smallest positive integer coefficients
  (each at most 20) that conserve every element and the net charge, using only the recorded
  species. If such coefficients exist, `balanced` is `true` and the counts are those
  coefficients (`2 H2 + O2 → 2 H2O`). Otherwise `balanced` is `false` and the counts are how
  many times each formula is listed. No by-products (H2O, HCl, …) and no protons are ever
  added. Most literature records list only the main product, so few of them balance.
- **`smiles`.** The canonical reaction SMILES, with atom maps removed and each side's
  species sorted. Agents (reagents, catalysts, solvents) go in the middle field.
- **Reactant vs agent roles.** For Lowe/USPTO records, roles come from the reaction SMILES:
  Lowe's atom mapping decides what is a reactant. For other ORD records, roles come from
  ORD's `reaction_role`, using REACTANT inputs and the products of the outcomes.
- **`conditions`.**
  - `temp_c` is the ORD temperature setpoint, converted to °C, or `null`. Nothing is
    inferred from "room temperature".
  - `catalyst` and `solvent` are the names of ORD components with those roles, falling back
    to SMILES and separated by `; `. They are `null` when unknown.
  - The USPTO `.rsmi` files carry no conditions, so for USPTO records all three are `null`.
- **Deduplication.** Duplicates are matched on canonical `reactants>>products` (agents are
  ignored), so the same transformation with a different solvent or base is kept once. The
  first occurrence wins. Sources are processed in `--sources` order, and ORD datasets in
  path order.

## `formula_pairs.json.gz`

This is a JSON object that maps a reactant formula set to the number of unique reactions
that use it. The key is the reactant formulas, sorted and joined with `+`, for example
`"C2H6O+C7H6O2": 12`. Entries are sorted by count, highest first. Use it to answer "how
many literature reactions start from exactly these formulas?". The data includes reagent
formulas only where the source counts them as reactants (Lowe's atom mapping, or ORD's
REACTANT role).

## Rebuilding

```bash
python3 -m venv /home/claude/chemdb-data/venv
/home/claude/chemdb-data/venv/bin/pip install -r tools/chemdb/requirements.txt
/home/claude/chemdb-data/venv/bin/python -I tools/chemdb/build_chemdb.py \
    --data-dir /home/claude/chemdb-data --out-dir /home/claude/chemdb-data/out \
    --stats-copy tools/chemdb/stats.json
```

- To reproduce this exact build, add `--ord-ref <commit from stats.json>`.
- ORD LFS objects are downloaded from `media.githubusercontent.com`, one file ahead of
  processing, and each one is checked against its LFS sha256. To use the Hugging Face
  mirror named in ord-data's `.lfsconfig` instead, pass
  `--ord-lfs-base 'https://huggingface.co/datasets/open-reaction-database/ord-data/resolve/main/{path}'`.
- Raw files are deleted after they are parsed unless you pass `--keep-raw`. ORD is about
  1.3 GB.
- To cap a run, use `--ord-max-files N` or `--uspto-max-rows N`. Any cap is recorded in
  `stats.json` → `caps`.
- Outputs go only to `--data-dir` and `--out-dir`. Nothing is written into the repo except
  the `--stats-copy` file.
- Disk and time: a full ORD build takes about 15 minutes on 4 cores and needs about 0.3 GB of
  free disk, because raw files are streamed and deleted. The exact time is in `stats.json` →
  `build_seconds`.
- Lowe's figshare set is about 1.1 GB of 7z archives. Unpacked, its `.rsmi` files are
  roughly 3–4 GB, so pass `--keep-raw` only with enough free disk.
- `python tools/chemdb/chemlib.py` runs the formula and balancing self-tests. The build
  also runs them at start-up.

## Where the data lives

- `/home/claude/chemdb-data/out/` holds `reactions.jsonl.gz`, `formula_pairs.json.gz` and
  `stats.json`.
- `/mnt/project-files/realgame/chemdb/` is a shared copy of the same files.

SHA-256 checksums for the current build:

- `reactions.jsonl.gz` `aec2e605a9fe5c73ca6e65a932043a335938968d077264fb5c143c52f5868fca`
- `formula_pairs.json.gz` `c425da8587e1ef6f79a38aa36bc5b921cd485aa0d3cb0eed23f333d5722fe5ef`

## Current build (2026-11-04, ord-data commit `3de6417`)

| | |
|---|---|
| ORD rows read (93 datasets) | 2,428,291 |
| Unique reactions kept | **1,512,217** |
| Duplicates removed | 914,764 |
| Rejected: structure does not sanitize or has an R-group | 756 |
| Rejected: name-only structure | 385 |
| Rejected: no reactants or no products | 169 |
| Balanced | 28,864 (**1.91 %**) |
| Distinct reactant formula sets | 1,361,907 |
| USPTO-Lowe-CC0 (figshare) | 0: unreachable (403 from the egress proxy) |
| `reactions.jsonl.gz` | 102,620,661 bytes (98 MB) |
| `formula_pairs.json.gz` | 7,939,638 bytes (7.6 MB) |
| Build time | 16.8 min on 4 cores |

Of the 1.51 M reactions, 1.04 M come from ORD's 41 `uspto-grants-*` datasets. Another
417 k come from the train, validation and test sets of doi:10.1039/C8SC04228D (Schwaller et al.,
also patent-derived). The remaining 50 datasets are mostly high-throughput experimentation
(HTE) screens. Those collapse heavily under deduplication because only their conditions vary.

### Caveats for game use

- **Few reactions balance.** Patent and HTE records list the main product and leave out
  by-products, so only about 2 % balance. The other 98 % keep `balanced:false`. They are
  real reactions, but not full equations.
- **Common reactants include solvents and reagents.** In Lowe's mapping and in ORD's REACTANT
  role, reagents and solvents often sit on the reactant side (H2O, HCl, DMF `C3H7NO`,
  Et3N `C6H15N`, THF `C4H8O`, …). Some ions are also recorded as separate species (`Na+`).
  See `top50_reactant_formulas` in `stats.json`. To match the game's formula keys, consider
  filtering out common solvents and reagents first.

## The Bench's lab records (`content/chem/literature.json`)

`build_literature.py reactions.jsonl.gz content/chem/names.json content/chem/literature.json`
turns the balanced, neutral records whose species use only the Bench's elements (at most
20 heavy atoms each) into net equations: reagents listed on both sides cancel, coefficients
are reduced, and identical equations merge with a count. The keys match
`realgame_chem::literature::key`, so the Bench page looks a player's equation up directly.
The file is ORD-derived and therefore CC BY-SA 4.0; the page shows the attribution.
Current build: 8,338 records, 7,403 net equations (ord-data 3de6417).
