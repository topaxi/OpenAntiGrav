#!/usr/bin/env python3
"""Generate the schema index served at /mods/schemas/ from schemas/v*/*.tosd.

Never hand-maintained: the kind and version directory come from the path, the
"what it describes" line from the first sentence of the schema's own leading
comment, so a new schema appears in the index the moment it is added.

    python3 scripts/gen-schema-index.py <dist>          # write index.html + index.json
    python3 scripts/gen-schema-index.py --verify <dist> # exit 1 if <dist>'s index and the
                                                        # schemas/ tree disagree

<dist> is the Pages root (scripts/stage-schemas.sh); the files land in
<dist>/mods/schemas/.
"""
import html
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BASE = "https://oag.topaxi.com/mods/schemas"
GUIDE = "https://github.com/topaxi/OpenAntiGrav/blob/main/docs/tools/modding.md"
CONVENTION = "https://github.com/topaxi/OpenAntiGrav/blob/main/docs/formats/toml-schema.md"


def description(path: Path) -> str:
    """The first sentence of the leading comment block."""
    lines = []
    for line in path.read_text().splitlines():
        if not line.startswith("#"):
            break
        lines.append(line.lstrip("#").strip())
    text = " ".join(lines)
    sentence = re.split(r"(?<=[a-z0-9`)])\.(?:\s|$)", text, maxsplit=1)[0]
    sentence = re.split(r"[,:]? see ", sentence, maxsplit=1)[0]
    return sentence.strip().rstrip(".") + "."


def scan() -> list[dict]:
    entries = []
    for path in sorted((ROOT / "schemas").glob("v*/*.tosd")):
        major = path.parent.name
        kind = path.stem
        entries.append(
            {
                "kind": kind,
                "dir": major,
                "path": f"{major}/{path.name}",
                "url": f"{BASE}/{major}/{path.name}",
                "describes": description(path),
            }
        )
    return entries


def page(entries: list[dict]) -> str:
    rows = "\n".join(
        "<tr><td><code>{kind}</code></td><td>{dir}</td>"
        '<td><a href="{path}"><code>{path}</code></a></td><td>{desc}</td></tr>'.format(
            kind=html.escape(e["kind"]),
            dir=html.escape(e["dir"]),
            path=html.escape(e["path"]),
            desc=re.sub(r"`([^`]+)`", r"<code>\1</code>", html.escape(e["describes"])),
        )
        for e in entries
    )
    return f"""<!doctype html>
<html lang="en">
<meta charset="utf-8">
<title>OpenAntiGrav TOML schemas</title>
<meta name="viewport" content="width=device-width, initial-scale=1">
<style>
body {{ font: 16px/1.5 system-ui, sans-serif; max-width: 60rem; margin: 2rem auto; padding: 0 1rem; }}
table {{ border-collapse: collapse; width: 100%; }}
th, td {{ border-bottom: 1px solid #8884; padding: .4rem .6rem; text-align: left; vertical-align: top; }}
</style>
<h1>OpenAntiGrav TOML schemas</h1>
<p>Every TOML file this project reads names its schema in a <code>toml-schema</code>
header. These are those schemas, in <a href="{CONVENTION}">TOML Schema</a> form
(<code>.tosd</code>). A machine-readable list is <a href="index.json"><code>index.json</code></a>.
How to write the files: the <a href="{GUIDE}">modding guide</a>.</p>
<table>
<thead><tr><th>Kind</th><th>Version</th><th>Schema</th><th>Describes</th></tr></thead>
<tbody>
{rows}
</tbody>
</table>
</html>
"""


def write(dist: Path) -> None:
    entries = scan()
    out = dist / "mods" / "schemas"
    out.mkdir(parents=True, exist_ok=True)
    (out / "index.html").write_text(page(entries))
    doc = {"base": BASE, "guide": GUIDE, "schemas": entries}
    (out / "index.json").write_text(json.dumps(doc, indent=2) + "\n")


def verify(dist: Path) -> list[str]:
    out = dist / "mods" / "schemas"
    problems = []
    on_disk = {f"{p.parent.name}/{p.name}" for p in (ROOT / "schemas").glob("v*/*.tosd")}
    for name in ("index.html", "index.json"):
        if not (out / name).is_file():
            problems.append(f"{name} was not generated")
    if problems:
        return problems
    listed = json.loads((out / "index.json").read_text())["schemas"]
    in_json = {e["path"] for e in listed}
    in_html = set(re.findall(r'<a href="([^"]+\.tosd)">', (out / "index.html").read_text()))
    for label, names in (("index.json", in_json), ("index.html", in_html)):
        for name in sorted(on_disk - names):
            problems.append(f"schemas/{name} is missing from {label}")
        for name in sorted(names - on_disk):
            problems.append(f"{label} names schemas/{name}, which does not exist")
    for name in sorted(in_json | in_html):
        if not (out / name).is_file():
            problems.append(f"{name} is in the index but not staged under {out}")
    for e in listed:
        if not e["describes"].strip(". "):
            problems.append(f"{e['path']} has no description (a leading comment)")
    return problems


def main() -> int:
    args = sys.argv[1:]
    if len(args) == 2 and args[0] == "--verify":
        problems = verify(Path(args[1]))
        for p in problems:
            print(f"schema index: {p}", file=sys.stderr)
        return 1 if problems else 0
    if len(args) == 1 and not args[0].startswith("-"):
        write(Path(args[0]))
        return 0
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main())
