#!/usr/bin/env python3
"""`just compare-page`: original and ours side by side, on one static page.

    scripts/compare-page.py MANIFEST_OR_DIR... [--out DIR] [--serve [--port N]]
    scripts/compare-page.py --check-publish-lock      (used by `just publish-compare`)

MANIFEST is TOML (docs/tools/compare-page.md): a `[page]` table and one `[[entry]]`
per review pair. Every image path is relative to the manifest. The page lands in
DIR (default `data/compare/`, gitignored: nothing captured or rendered is ever
committed) as `index.html` plus `img/`, with no external dependency, so the same
directory opens from disk, serves over the LAN, or uploads as a Pages project.

Each pair is normalised at build time with ImageMagick: letterbox bars trimmed
from the original, both sides brought to one size without stretching the
original's aspect, and a difference image precomputed (a canvas read would taint
under file://).

A manifest this tool reads is a developer file under data/, never loaded by the
game, so it carries no `[toml-schema]` header (ADR-0064 is for files the game or a
modder reads); a wrong shape is refused by name here instead.
"""
import argparse
import hashlib
import http.server
import json
import re
import shutil
import socket
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_OUT = ROOT / "data" / "compare"

# The screenshot-policy ADR that allows publishing curated screenshots, and the
# Pages project it may go to. The number is filled in once the ADR exists; until
# the lock refuses by name if the file is missing or not Accepted.
POLICY_ADR = "0074"
PUBLISH_PROJECT = "openantigrav-compare"
MAIN_SITE_PROJECT = "openantigrav"

SLOTS = ("original", "before", "ours")
LABELS = {"original": "Original", "before": "Ours (before)", "ours": "Ours"}
CAPTION_KEYS = ("title", "track", "craft", "state", "setting")


class ManifestError(Exception):
    pass


def refuse(msg):
    raise ManifestError(msg)


def manifest_files(args: list[str]) -> list[Path]:
    """Each argument is a manifest or a directory of them (every *.toml in it, sorted)."""
    files = []
    for a in args:
        p = Path(a).resolve()
        files += sorted(p.glob("*.toml")) if p.is_dir() else [p]
    if not files:
        refuse(f"no manifest found in {args}")
    return files


def load_all(paths: list[Path]) -> dict:
    """The first manifest's [page]; every file's entries, each remembering its own directory."""
    merged = None
    ids = set()
    for path in paths:
        doc = load_manifest(path)
        for e in doc["entry"]:
            if e["id"] in ids:
                refuse(f"{path}: entry id {e['id']!r} already used by an earlier manifest")
            ids.add(e["id"])
            e["_base"] = path.parent
        if merged is None:
            merged = doc
        else:
            merged["entry"] += doc["entry"]
    return merged


def load_manifest(path: Path) -> dict:
    try:
        doc = tomllib.loads(path.read_text())
    except (OSError, tomllib.TOMLDecodeError) as e:
        refuse(f"{path}: {e}")
    page = doc.get("page")
    if not isinstance(page, dict) or "title" not in page:
        refuse(f'{path}: no [page] table with a title; add `[page]` and `title = "..."`')
    entries = doc.get("entry")
    if not isinstance(entries, list) or not entries:
        refuse(f"{path}: no [[entry]] tables")
    seen = set()
    for i, e in enumerate(entries):
        where = f"{path}: entry {i + 1} ({e.get('id', '?')})"
        for k in ("id", "title", "game"):
            if k not in e:
                refuse(f"{where}: missing `{k}`")
        if e["id"] in seen:
            refuse(f"{where}: duplicate id")
        seen.add(e["id"])
        have = [s for s in SLOTS if e.get(s)]
        if len(have) < 2:
            refuse(f"{where}: needs at least two of original/before/ours; has {have or 'none'}")
        for s in have:
            p = path.parent / e[s]
            if not p.is_file():
                refuse(f"{where}: `{s}` image {p} does not exist")
        unknown = set(e) - {"_base", "id", "game", "note", "not_captured", *SLOTS, *CAPTION_KEYS}
        if unknown:
            refuse(f"{where}: unknown key(s) {sorted(unknown)}")
    return doc


def magick(*args):
    r = subprocess.run(["magick", *args], capture_output=True, text=True)
    if r.returncode != 0:
        refuse(f"magick {' '.join(args[:3])}...: {r.stderr.strip()}")
    return r.stdout


def dims(p: Path) -> tuple[int, int]:
    w, h = magick("identify", "-format", "%w %h", str(p) + "[0]").split()
    return int(w), int(h)


def trimmed_geometry(p: Path) -> str | None:
    """The crop that removes a uniformly black letterbox, or None when there is none."""
    out = magick(str(p), "-fuzz", "3%", "-format", "%@", "info:").strip()
    m = re.fullmatch(r"(\d+)x(\d+)\+(\d+)\+(\d+)", out)
    if not m:
        return None
    w, h = dims(p)
    tw, th, x, y = map(int, m.groups())
    # Only a real bar counts: a dark scene edge trims a few pixels at most.
    if (w - tw) * h + (h - th) * w < 0.02 * w * h:
        return None
    return out


def normalise(entry: dict, base: Path, img: Path) -> dict:
    """Write the entry's slots to img/ at one common size and return the page record."""
    srcs = {s: base / entry[s] for s in SLOTS if entry.get(s)}
    key = hashlib.sha1("".join(str(p.resolve()) + str(p.stat().st_mtime_ns) for p in srcs.values()).encode()).hexdigest()[:8]
    # The reference size is the first present slot in this order; trimmed if it has bars.
    ref_slot = next(s for s in ("ours", "original", "before") if s in srcs)
    rw, rh = dims(srcs[ref_slot])
    if ref_slot == "ours" and "original" in srcs:
        # Our frame is the reference size; the original is brought to it.
        pass
    sizes = {}
    files = {}
    for slot, src in srcs.items():
        out = img / f"{entry['id']}-{slot}-{key}.png"
        files[slot] = out
        crop = trimmed_geometry(src) if slot == "original" else None
        if out.exists():
            continue
        args = [str(src) + "[0]"]
        if crop:
            args += ["-crop", crop, "+repage"]
        magick(*args, "-alpha", "off", str(out))
    # One size for every slot: the aspect of the first slot, never stretched; a slot
    # of another aspect is fitted inside it and padded, so a mismatch stays visible.
    w, h = dims(files[ref_slot])
    for slot, out in files.items():
        cw, ch = dims(out)
        if (cw, ch) != (w, h):
            if abs(cw / ch - w / h) < 0.01:
                magick(str(out), "-filter", "Lanczos", "-resize", f"{w}x{h}!", str(out))
            else:
                magick(str(out), "-filter", "Lanczos", "-resize", f"{w}x{h}", "-background", "black",
                       "-gravity", "center", "-extent", f"{w}x{h}", str(out))
        sizes[slot] = [w, h]
    rec = {s: f"img/{files[s].name}" for s in files}
    diffs = {}
    pairs = [("original", "ours"), ("before", "ours"), ("original", "before")]
    for a, b in pairs:
        if a in files and b in files:
            d = img / f"{entry['id']}-{a}-{b}-diff-{key}.png"
            if not d.exists():
                magick(str(files[a]), str(files[b]), "-compose", "difference", "-composite",
                       "-evaluate", "multiply", "2", str(d))
            diffs[f"{a}|{b}"] = f"img/{d.name}"
    caption = {k: entry[k] for k in CAPTION_KEYS if k in entry}
    return {
        "id": entry["id"],
        "game": entry["game"],
        "title": entry["title"],
        "caption": caption,
        "note": entry.get("note", ""),
        "not_captured": entry.get("not_captured", ""),
        "size": [w, h],
        "images": rec,
        "diffs": diffs,
    }


def build(manifests: list[Path], out: Path) -> Path:
    doc = load_all(manifests)
    img = out / "img"
    img.mkdir(parents=True, exist_ok=True)
    records = [normalise(e, e["_base"], img) for e in doc["entry"]]
    # Drop images no entry references any more, so a rebuilt page carries nothing stale.
    used = {Path(p).name for r in records for p in [*r["images"].values(), *r["diffs"].values()]}
    for f in img.glob("*.png"):
        if f.name not in used:
            f.unlink()
    page = {"title": doc["page"]["title"], "intro": doc["page"].get("intro", ""), "entries": records}
    (out / "index.html").write_text(TEMPLATE.replace("/*DATA*/", json.dumps(page).replace("</", "<\\/")))
    return out / "index.html"


def lan_addresses() -> list[str]:
    found = []
    try:
        for info in socket.getaddrinfo(socket.gethostname(), None, socket.AF_INET):
            ip = info[4][0]
            if not ip.startswith("127.") and ip not in found:
                found.append(ip)
    except OSError:
        pass
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        s.connect(("192.0.2.1", 9))
        ip = s.getsockname()[0]
        s.close()
        if ip not in found:
            found.append(ip)
    except OSError:
        pass
    return found


def serve(out: Path, port: int):
    class Handler(http.server.SimpleHTTPRequestHandler):
        def __init__(self, *a, **k):
            super().__init__(*a, directory=str(out), **k)

        def log_message(self, *a):
            pass

    srv = http.server.ThreadingHTTPServer(("0.0.0.0", port), Handler)
    print(f"serving {out} on all interfaces, port {port} (Ctrl-C to stop):")
    print(f"  http://localhost:{port}/")
    for ip in lan_addresses():
        print(f"  http://{ip}:{port}/")
    sys.stdout.flush()
    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        pass


def publish_lock() -> str | None:
    """None when publishing is allowed, else the refusal, in the words the user sees."""
    if PUBLISH_PROJECT == MAIN_SITE_PROJECT:
        return f"the compare Pages project is named {MAIN_SITE_PROJECT!r}, the main site's: refusing"
    if POLICY_ADR is None:
        return "screenshot-policy ADR not found: POLICY_ADR in scripts/compare-page.py is unset (the ADR is not written yet)"
    matches = sorted((ROOT / "docs/architecture/adr").glob(f"{POLICY_ADR}-*.md"))
    if not matches:
        return f"screenshot-policy ADR not found: no docs/architecture/adr/{POLICY_ADR}-*.md"
    text = matches[0].read_text().splitlines()
    status = ""
    for i, line in enumerate(text):
        if line.strip() == "## Status":
            status = next((l.strip() for l in text[i + 1:] if l.strip()), "")
            break
    status = status.lstrip("*_ ")  # ADR-0074 writes **Accepted (date)**
    if not status.startswith("Accepted"):
        return f"{matches[0].name} status reads {status!r}, not Accepted: publishing stays locked"
    return None


def publish(out: Path):
    why = publish_lock()
    if why:
        print(f"publish-compare: REFUSED. {why}", file=sys.stderr)
        sys.exit(2)
    if shutil.which("wrangler") is None:
        print("publish-compare: wrangler is not installed (npm i -g wrangler)", file=sys.stderr)
        sys.exit(3)
    if not (out / "index.html").is_file():
        print(f"publish-compare: {out}/index.html missing; run `just compare-page` first", file=sys.stderr)
        sys.exit(4)
    subprocess.run(["wrangler", "pages", "deploy", str(out), "--project-name", PUBLISH_PROJECT,
                    "--branch", "main"], check=True)


TEMPLATE = (Path(__file__).with_name("compare-page.html")).read_text()


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("manifest", nargs="*", help="manifest files or directories of them")
    ap.add_argument("--out", type=Path, default=DEFAULT_OUT)
    ap.add_argument("--serve", action="store_true", help="serve the built page on every interface")
    ap.add_argument("--port", type=int, default=8097)
    ap.add_argument("--check-publish-lock", action="store_true")
    ap.add_argument("--publish", action="store_true", help="used by `just publish-compare` only")
    a = ap.parse_args()
    if a.check_publish_lock:
        why = publish_lock()
        print(why or "unlocked")
        sys.exit(2 if why else 0)
    if a.publish:
        publish(a.out)
        return
    if not a.manifest:
        ap.error("a manifest is required")
    try:
        index = build(manifest_files(a.manifest), a.out.resolve())
    except ManifestError as e:
        print(f"compare-page: {e}", file=sys.stderr)
        sys.exit(1)
    print(f"page: {index}")
    if a.serve:
        serve(a.out.resolve(), a.port)


if __name__ == "__main__":
    main()
