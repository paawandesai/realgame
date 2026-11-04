#!/usr/bin/env python3
"""Build the Formula "Reference tier" reaction database from open reaction datasets.

Sources (see README.md for licences):
  uspto  Daniel Lowe, "Chemical reactions from US patents (1976-Sep2016)", figshare
         article 5104873, CC0. Reaction SMILES (.rsmi) files inside the *_smiles.7z archives.
  ord    Open Reaction Database, github.com/open-reaction-database/ord-data, CC-BY-SA 4.0.
         Parquet datasets stored in Git LFS.

Output (in --out-dir):
  reactions.jsonl.gz       one JSON record per unique reaction (schema in README.md)
  formula_pairs.json.gz    {"<reactant formulas joined by +>": n_reactions, ...}
  stats.json               counts, % balanced, top reactant formulas, file sizes
Raw downloads live in --data-dir and are never written into the git tree.

Usage:
  python -I tools/chemdb/build_chemdb.py --data-dir /home/claude/chemdb-data \
      --out-dir /home/claude/chemdb-data/out --stats-copy tools/chemdb/stats.json
"""

from __future__ import annotations

import argparse
import concurrent.futures as cf
import datetime as dt
import gzip
import hashlib
import json
import multiprocessing as mp
import os
import pathlib
import re
import subprocess
import sys
import time
import urllib.error
import urllib.request
from collections import Counter, defaultdict

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import chemlib  # noqa: E402

ORD_REPO = "https://github.com/open-reaction-database/ord-data"
# GitHub serves LFS objects of public repos at this host. The repo's own .lfsconfig points at
# a Hugging Face mirror; pass --ord-lfs-base to use that (or any other mirror) instead.
ORD_LFS_BASE = "https://media.githubusercontent.com/media/open-reaction-database/ord-data/{ref}/{path}"
FIGSHARE_ARTICLE = "https://api.figshare.com/v2/articles/5104873"
SRC_USPTO = "USPTO-Lowe-CC0"
SRC_ORD = "ORD CC-BY-SA-4.0"
BATCH = 400


def log(*a):
    print(time.strftime("%H:%M:%S"), *a, file=sys.stderr, flush=True)


def download(url: str, dest: pathlib.Path, sha256: str | None = None, md5: str | None = None,
             size: int | None = None, retries: int = 3) -> None:
    tmp = dest.with_suffix(dest.suffix + ".part")
    for attempt in range(1, retries + 1):
        try:
            h256, h5, n = hashlib.sha256(), hashlib.md5(), 0
            with urllib.request.urlopen(url, timeout=120) as r, open(tmp, "wb") as f:
                while chunk := r.read(1 << 20):
                    f.write(chunk)
                    h256.update(chunk)
                    h5.update(chunk)
                    n += len(chunk)
            if size is not None and n != size:
                raise IOError(f"size {n} != {size}")
            if sha256 and h256.hexdigest() != sha256:
                raise IOError("sha256 mismatch")
            if md5 and h5.hexdigest() != md5:
                raise IOError("md5 mismatch")
            tmp.replace(dest)
            return
        except (urllib.error.URLError, IOError, TimeoutError) as e:
            log(f"download failed ({attempt}/{retries}) {url}: {e}")
            if isinstance(e, urllib.error.HTTPError) and e.code in (403, 404, 407):
                break
            time.sleep(3 * attempt)
    tmp.unlink(missing_ok=True)
    raise IOError(f"could not download {url}")


# ---------------------------------------------------------------- ORD conversion (workers)

_pb = None


def _ord_init():
    global _pb
    from ord_schema.proto import reaction_pb2
    from ord_schema import message_helpers
    _pb = (reaction_pb2, message_helpers)


def _structure(comp) -> str | None:
    try:
        s = _pb[1].smiles_from_compound(comp)
    except Exception:  # an unreadable identifier is the same as no structure
        return None
    return s.split(" ", 1)[0] if s else None


def _label(comp) -> str | None:
    names = [i.value for i in comp.identifiers if i.type == _pb[0].CompoundIdentifier.NAME]
    return names[0] if names else _structure(comp)


def _join(xs):
    xs = list(dict.fromkeys(x for x in xs if x))
    return "; ".join(xs) if xs else None


def _ord_one(rid: str, blob: bytes):
    pb = _pb[0]
    R = pb.ReactionRole
    rxn = pb.Reaction.FromString(blob)
    comps = [c for inp in rxn.inputs.values() for c in inp.components]
    temp = None
    sp = rxn.conditions.temperature.setpoint
    if rxn.conditions.HasField("temperature") and rxn.conditions.temperature.HasField("setpoint"):
        u = sp.units
        if u == pb.Temperature.CELSIUS:
            temp = sp.value
        elif u == pb.Temperature.KELVIN:
            temp = sp.value - 273.15
        elif u == pb.Temperature.FAHRENHEIT:
            temp = (sp.value - 32) * 5 / 9
        if temp is not None:
            temp = round(temp, 1)
    cond = {
        "temp_c": temp,
        "catalyst": _join(_label(c) for c in comps if c.reaction_role == R.CATALYST),
        "solvent": _join(_label(c) for c in comps if c.reaction_role == R.SOLVENT),
    }
    rxn_smiles = [i.value for i in rxn.identifiers
                  if i.type in (pb.ReactionIdentifier.REACTION_CXSMILES, pb.ReactionIdentifier.REACTION_SMILES)]
    reactants = agents = products = None
    if rxn_smiles:
        sp3 = chemlib.split_reaction_smiles(rxn_smiles[0])
        if sp3 and sp3[0] and sp3[2]:
            reactants, agents, products = sp3
            path = "reaction_smiles"
    if reactants is None:
        path = "components"
        rc = [c for c in comps if c.reaction_role == R.REACTANT]
        if not rc:
            return ("no_reactants", None, None, path)
        reactants = [_structure(c) for c in rc]
        prods = [p for o in rxn.outcomes for p in o.products
                 if p.reaction_role in (R.UNSPECIFIED, R.PRODUCT, R.BYPRODUCT, R.SIDE_PRODUCT)]
        if not prods:
            return ("no_products", None, None, path)
        products = list(dict.fromkeys(_structure(p) for p in prods))
        if any(s is None for s in reactants) or any(s is None for s in products):
            return ("structure_missing", None, None, path)
        agents = [s for c in comps if c.reaction_role in (R.REAGENT, R.CATALYST, R.SOLVENT)
                  if (s := _structure(c))]
    key, rec = chemlib.build_record("ord:" + rid, SRC_ORD, reactants, agents, products, cond)
    return ("ok", key, rec, path)


def ord_batch(batch):
    out = []
    for rid, blob in batch:
        try:
            out.append(_ord_one(rid, blob))
        except chemlib.BadSpecies:
            out.append(("bad_structure", None, None, None))
        except Exception as e:  # keep going; report the class of failure
            out.append(("error:" + type(e).__name__, None, None, None))
    return out


def uspto_batch(batch):
    out = []
    null_cond = {"temp_c": None, "catalyst": None, "solvent": None}
    for n, smi in batch:
        try:
            sp3 = chemlib.split_reaction_smiles(smi)
            if not sp3:
                out.append(("bad_reaction_smiles", None, None, None))
                continue
            key, rec = chemlib.build_record(f"uspto:{n}", SRC_USPTO, sp3[0], sp3[1], sp3[2], dict(null_cond))
            out.append(("ok", key, rec, "reaction_smiles"))
        except chemlib.BadSpecies:
            out.append(("bad_structure", None, None, None))
    return out


# ---------------------------------------------------------------- driver


class Builder:
    def __init__(self, out_dir: pathlib.Path, workers: int):
        self.out_dir = out_dir
        self.seen: set[bytes] = set()
        self.tmp = out_dir / "reactions.jsonl.gz.part"
        self.fh = gzip.open(self.tmp, "wt", compresslevel=9, encoding="utf-8")
        self.pairs: Counter = Counter()
        self.reactant_formulas: Counter = Counter()
        self.per_source = defaultdict(lambda: Counter())
        self.datasets: list[dict] = []
        self.workers = workers

    def consume(self, results, source: str, ds: Counter):
        for status, key, rec, _path in results:
            ds["rows"] += 1
            if status != "ok":
                ds["rejected:" + status] += 1
                continue
            h = hashlib.blake2b(key.encode(), digest_size=12).digest()
            if h in self.seen:
                ds["duplicate"] += 1
                continue
            self.seen.add(h)
            ds["kept"] += 1
            ds["balanced"] += rec["balanced"]
            self.fh.write(json.dumps(rec, separators=(",", ":"), ensure_ascii=False) + "\n")
            rset = sorted(rec["reactants"])
            self.pairs["+".join(rset)] += 1
            self.reactant_formulas.update(rset)

    def run_batches(self, pool, fn, batches, source, ds):
        for res in pool.imap(fn, batches, chunksize=1):
            self.consume(res, source, ds)
        self.per_source[source].update(ds)


def iter_batches(it, n=BATCH):
    b = []
    for x in it:
        b.append(x)
        if len(b) == n:
            yield b
            b = []
    if b:
        yield b


def ord_checkout(data_dir: pathlib.Path, ref: str | None) -> pathlib.Path:
    repo = data_dir / "ord-data"
    env = dict(os.environ, GIT_LFS_SKIP_SMUDGE="1")
    if not (repo / ".git").exists():
        log("cloning ord-data (LFS pointers only)")
        subprocess.run(["git", "clone", "-q", "--depth", "1", ORD_REPO, str(repo)], check=True, env=env)
    else:
        subprocess.run(["git", "-C", str(repo), "fetch", "-q", "--depth", "1", "origin", ref or "HEAD"],
                       check=True, env=env)
        subprocess.run(["git", "-C", str(repo), "reset", "-q", "--hard", "FETCH_HEAD"], check=True, env=env)
    return repo


def ord_pointers(repo: pathlib.Path):
    out = []
    for p in sorted((repo / "data").glob("*/*.parquet")):
        txt = p.read_text(errors="replace")
        m1, m2 = re.search(r"oid sha256:([0-9a-f]{64})", txt), re.search(r"size (\d+)", txt)
        if not (m1 and m2):
            raise SystemExit(f"{p} is not an LFS pointer")
        out.append((str(p.relative_to(repo)), m1.group(1), int(m2.group(1))))
    return out


def run_ord(b: Builder, pool, args, stats_src: dict):
    from ord_schema.proto import reaction_pb2  # noqa: F401  (fail fast if ord-schema missing)
    import pyarrow.parquet as pq
    repo = ord_checkout(args.data_dir, args.ord_ref)
    commit = subprocess.run(["git", "-C", str(repo), "rev-parse", "HEAD"], capture_output=True,
                            text=True, check=True).stdout.strip()
    ptrs = ord_pointers(repo)
    if args.ord_max_files:
        ptrs = ptrs[: args.ord_max_files]
    raw = args.data_dir / "raw" / "ord"
    raw.mkdir(parents=True, exist_ok=True)
    stats_src.update(commit=commit, files=len(ptrs), lfs_bytes=sum(p[2] for p in ptrs))
    log(f"ORD @ {commit[:10]}: {len(ptrs)} files, {stats_src['lfs_bytes']/1e6:.0f} MB")

    def fetch(ptr):
        rel, sha, size = ptr
        dest = raw / pathlib.Path(rel).name
        if dest.exists() and dest.stat().st_size == size:
            return dest
        download(args.ord_lfs_base.format(ref=commit, path=rel), dest, sha256=sha, size=size)
        return dest

    with cf.ThreadPoolExecutor(1) as ex:  # prefetch the next file while this one is parsed
        fut = ex.submit(fetch, ptrs[0]) if ptrs else None
        for i, ptr in enumerate(ptrs):
            dest = fut.result()
            fut = ex.submit(fetch, ptrs[i + 1]) if i + 1 < len(ptrs) else None
            pf = pq.ParquetFile(dest)
            md = {k.decode(): v.decode() for k, v in (pf.schema_arrow.metadata or {}).items()}
            ds = Counter()

            def rows():
                for rg in range(pf.num_row_groups):
                    t = pf.read_row_group(rg, columns=["reaction_id", "reaction"])
                    yield from zip(t.column("reaction_id").to_pylist(), t.column("reaction").to_pylist())

            b.run_batches(pool, ord_batch, iter_batches(rows()), "ord", ds)
            entry = {"dataset_id": md.get("ord.dataset_id"), "name": md.get("ord.name"), "path": ptr[0],
                     **dict(sorted(ds.items()))}
            b.datasets.append(entry)
            log(f"[{i+1}/{len(ptrs)}] {entry['name']}: rows={ds['rows']} kept={ds['kept']} "
                f"dup={ds['duplicate']} total_kept={len(b.seen)}")
            if not args.keep_raw:
                dest.unlink()


def run_uspto(b: Builder, pool, args, stats_src: dict):
    raw = args.data_dir / "raw" / "uspto"
    raw.mkdir(parents=True, exist_ok=True)
    try:
        with urllib.request.urlopen(FIGSHARE_ARTICLE, timeout=60) as r:
            art = json.load(r)
    except Exception as e:
        stats_src["status"] = f"unreachable: {FIGSHARE_ARTICLE}: {e}"
        log("USPTO (figshare) unreachable:", e)
        return
    files = [f for f in art["files"] if re.search(r"smiles\.7z$", f["name"], re.I)]
    stats_src.update(title=art.get("title"), license=(art.get("license") or {}).get("name"),
                     files=[f["name"] for f in files])
    try:
        import py7zr
    except ImportError:
        raise SystemExit("pip install py7zr (needed to unpack the USPTO .7z archives)")
    n = 0
    for f in files:
        arc = raw / f["name"]
        if not (arc.exists() and arc.stat().st_size == f["size"]):
            download(f["download_url"], arc, md5=f.get("computed_md5"), size=f["size"])
        out = raw / arc.stem
        if not out.exists():
            with py7zr.SevenZipFile(arc) as z:
                z.extractall(out)
        for rsmi in sorted(out.rglob("*.rsmi")):
            ds = Counter()

            def rows():
                nonlocal n
                with open(rsmi, encoding="utf-8", errors="replace") as fh:
                    header = fh.readline()
                    if not header.startswith("ReactionSmiles"):
                        fh.seek(0)
                    for line in fh:
                        n += 1
                        if args.uspto_max_rows and n > args.uspto_max_rows:
                            return
                        yield n, line.split("\t", 1)[0]

            b.run_batches(pool, uspto_batch, iter_batches(rows()), "uspto", ds)
            b.datasets.append({"dataset_id": None, "name": rsmi.name, "path": f["name"], **dict(sorted(ds.items()))})
            log(f"{rsmi.name}: rows={ds['rows']} kept={ds['kept']} dup={ds['duplicate']}")
        if not args.keep_raw:
            arc.unlink(missing_ok=True)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--data-dir", type=pathlib.Path, default=pathlib.Path("/home/claude/chemdb-data"))
    ap.add_argument("--out-dir", type=pathlib.Path, default=None, help="default: <data-dir>/out")
    ap.add_argument("--sources", default="uspto,ord", help="comma list, processed in this order "
                    "(first source wins on duplicates)")
    ap.add_argument("--workers", type=int, default=os.cpu_count() or 2)
    ap.add_argument("--keep-raw", action="store_true", help="keep downloaded files (ORD is ~1.3 GB)")
    ap.add_argument("--ord-ref", default=None, help="ord-data commit/branch (default: remote HEAD)")
    ap.add_argument("--ord-lfs-base", default=ORD_LFS_BASE)
    ap.add_argument("--ord-max-files", type=int, default=0, help="cap ORD datasets (0 = all)")
    ap.add_argument("--uspto-max-rows", type=int, default=0, help="cap USPTO rows (0 = all)")
    ap.add_argument("--stats-copy", type=pathlib.Path, default=None, help="also write stats.json here")
    args = ap.parse_args()
    args.out_dir = args.out_dir or args.data_dir / "out"
    args.out_dir.mkdir(parents=True, exist_ok=True)
    chemlib._selftest()

    t0 = time.time()
    b = Builder(args.out_dir, args.workers)
    src_stats: dict[str, dict] = {}
    with mp.get_context("fork").Pool(args.workers, initializer=_ord_init) as pool:
        for s in args.sources.split(","):
            src_stats[s] = {}
            {"uspto": run_uspto, "ord": run_ord}[s](b, pool, args, src_stats[s])
    b.fh.close()
    final = args.out_dir / "reactions.jsonl.gz"
    b.tmp.replace(final)
    pairs_path = args.out_dir / "formula_pairs.json.gz"
    with gzip.open(pairs_path, "wt", compresslevel=9) as f:
        json.dump(dict(sorted(b.pairs.items(), key=lambda kv: (-kv[1], kv[0]))), f, separators=(",", ":"))
    elapsed = time.time() - t0

    kept = sum(c["kept"] for c in b.per_source.values())
    bal = sum(c["balanced"] for c in b.per_source.values())
    per = {}
    for s, c in b.per_source.items():
        per[s] = {**src_stats.get(s, {}), "rows_read": c["rows"], "kept_unique": c["kept"],
                  "duplicates": c["duplicate"], "balanced": c["balanced"],
                  "pct_balanced": round(100 * c["balanced"] / c["kept"], 3) if c["kept"] else None,
                  "rejected": {k.split(":", 1)[1]: v for k, v in sorted(c.items()) if k.startswith("rejected:")}}
    for s in src_stats:
        per.setdefault(s, {**src_stats[s], "rows_read": 0, "kept_unique": 0})
    stats = {
        "built_utc": dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "build_seconds": round(elapsed),
        "workers": args.workers,
        "rdkit": __import__("rdkit").__version__,
        "caps": {"ord_max_files": args.ord_max_files, "uspto_max_rows": args.uspto_max_rows},
        "total_reactions": kept,
        "balanced": bal,
        "pct_balanced": round(100 * bal / kept, 3) if kept else None,
        "distinct_reactant_sets": len(b.pairs),
        "sources": per,
        "top50_reactant_formulas": [[f, n] for f, n in b.reactant_formulas.most_common(50)],
        "files": {p.name: p.stat().st_size for p in (final, pairs_path)},
        "datasets": b.datasets,
    }
    for p in [args.out_dir / "stats.json"] + ([args.stats_copy] if args.stats_copy else []):
        p.write_text(json.dumps(stats, indent=1) + "\n")
    log(f"done: {kept} reactions, {stats['pct_balanced']}% balanced, {elapsed/60:.1f} min")


if __name__ == "__main__":
    main()
