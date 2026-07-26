#!/usr/bin/env python3
"""Rebuilds the WAD entry-name candidate list from a disc image.

WAD archives store only a CRC-32 of each entry name, so a listing with names
requires hashing candidate names and matching. Most names are never stored
whole: they are assembled at runtime from templates like `%s\\%strack%s.vex`.
Recovering them is therefore a mining exercise, and this script is that
exercise made repeatable.

Four sources, in increasing order of how much they yield:

1. **Strings in the executable.** Whole paths that happen to be stored
   literally. Cheap, and covers most of `BEData.wad`.
2. **Guessed combinations** of known team names and known filenames, built from
   the templates recovered from the binary.
3. **Plugin definitions.** `Data\\Plugins\\PI001\\Definition.xml` lists every
   track with a `location` attribute; combining those with the track templates
   is what makes track files findable at all.
4. **Front-end XML.** Any attribute value that looks like a path.

Output goes to `data/extracted/<platform>/names.txt`, which
`oag-wad list --names` and `oag-view --names` both accept.

Usage:
    scripts/mine-names.py data/images/pulse-psp-usa.chd
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Teams whose directories exist under Data\Ships. Taken from the string table
# at 0x08a78200 in the PSP executable.
TEAMS = [
    "AG_Systems", "Assegai", "Auricom", "EGX", "Feisar",
    "Goteki", "Mirage", "Piranha", "Qirex", "Triakis",
]

# Per-ship files, from the path templates recovered from the binary.
SHIP_FILES = [
    "Ship.vex", "ship_FE.vex", "Zone.vex", "zonewreck.vex",
    "handlingstats.xml", "stats.xml", "stats_reversed.xml",
    "Definition.xml", "Skin.xml", "screen.xml", "stringtable.xml",
    "ship_eliminator.dat",
]

# Per-track files. The two `%s` in `%s\%strack%s.vex` are a `zone_` prefix and
# a `_reversed` suffix, so all four combinations exist.
TRACK_FILES = [
    f"{prefix}{stem}{suffix}.vex"
    for stem in ("track", "start_grid")
    for prefix in ("", "zone_")
    for suffix in ("", "_reversed")
] + ["TrackStartup.xml", "Definition.xml", "screen.xml", "screen_zone.xml"]

# Plugins loaded at boot, in load order, from the manifest at 0x08ab110c.
PLUGINS = [
    "PI012", "PI010", "PI008", "PI009", "PI011", "PI001",
    "PI004", "grids", "music", "loading", "news",
]


def run(*args: str) -> bytes:
    """Runs a workspace binary and returns its stdout."""
    result = subprocess.run(
        ["cargo", "run", "-q", "--release", "-p", *args],
        cwd=ROOT,
        capture_output=True,
    )
    return result.stdout


def wad(image: str, archive: str, *args: str) -> bytes:
    """Runs `oag-wad <subcommand> <archive> [args]`."""
    return run("oag-tools", "--bin", "oag-wad", "--", args[0], f"{image}:{archive}", *args[1:])


def from_executable(boot: Path) -> set[str]:
    """Whole paths stored literally in the executable."""
    if not boot.exists():
        print(f"  (skipping {boot.name}: not extracted)", file=sys.stderr)
        return set()

    data = boot.read_bytes()
    out: set[str] = set()
    for run_ in re.findall(rb"[\x20-\x7e]{5,180}", data):
        text = run_.decode("ascii")
        for match in re.finditer(r"[A-Za-z0-9_\-./\\]+\.[A-Za-z0-9]{2,4}", text):
            path = match.group(0)
            if ("\\" in path or "/" in path) and not path.startswith(("umd:", "ms0:")):
                out.add(path)
    return out


def guessed() -> set[str]:
    """Combinations built from known names and known templates."""
    out = set()
    for team in TEAMS:
        for name in SHIP_FILES:
            out.add(f"Data\\Ships\\{team}\\{name}")
        for suffix in ("", "wreck", "shield", "boost"):
            out.add(f"Data\\Ships\\{team}\\{team}{suffix}.vex")
    for plugin in PLUGINS:
        for name in ("Definition.xml", "stringtable.xml", "screen.xml", "Skin.xml"):
            out.add(f"Data\\Plugins\\{plugin}\\{name}")
    out.update(f"Data\\Plugins\\grids\\grid_{i:02d}.xml" for i in range(16))
    return out


def paths_in(text: str) -> set[str]:
    """Attribute values that look like asset paths."""
    out = set()
    for value in re.findall(r'="([^"]+)"', text):
        leaf = value.replace("\\", "/").rsplit("/", 1)[-1]
        if ("\\" in value or "/" in value) and "." in leaf:
            out.add(value)
    return out


def from_track_plugin(image: str) -> set[str]:
    """Track directories, from the plugin that lists every track.

    This is the step that makes tracks findable: their directory names are
    numbered (`01_Track`), not derived from the track's display name, so no
    amount of guessing finds them.
    """
    text = wad(
        image, "PSP_GAME/USRDIR/Data.wad",
        "cat", "Data\\Plugins\\PI001\\Definition.xml", "--expand",
    ).decode("utf-8", "replace")

    out = set()
    locations = set(re.findall(r'location="([^"]+)"', text))
    for location in locations:
        out.update(f"{location}\\{name}" for name in TRACK_FILES)
    out |= paths_in(text)

    print(f"  {len(locations)} track locations", file=sys.stderr)
    return out


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2

    image = sys.argv[1]
    if not Path(image).exists():
        print(f"error: {image} does not exist", file=sys.stderr)
        return 1

    out_dir = ROOT / "data/extracted/psp"
    out_dir.mkdir(parents=True, exist_ok=True)

    names: set[str] = set()

    print("mining names", file=sys.stderr)
    print("  executable strings", file=sys.stderr)
    names |= from_executable(out_dir / "PSP_GAME/SYSDIR/BOOT.BIN")

    print("  guessed combinations", file=sys.stderr)
    names |= guessed()

    print("  track plugin", file=sys.stderr)
    try:
        names |= from_track_plugin(image)
    except Exception as e:  # noqa: BLE001 - a missing archive is not fatal
        print(f"  (skipped: {e})", file=sys.stderr)

    out = out_dir / "names.txt"
    out.write_text("\n".join(sorted(names)) + "\n")
    print(f"\n{len(names)} candidates -> {out}", file=sys.stderr)

    print(
        "\ncheck coverage with:\n"
        f"  just wad list {image}:PSP_GAME/USRDIR/Data.wad --names {out} | grep '^names'",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
