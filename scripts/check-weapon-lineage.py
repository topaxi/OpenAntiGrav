#!/usr/bin/env python3
"""`docs/gameplay/weapon-lineage.md` must agree with the title crates' weapon data.

The page's "Checked axes" table has one row per axis and one column per title.
Each cell is `` `value` - provenance ``. This script reads the Rust source of
the five title crates (text only, no compiler) and fails when

- a public field of `Weapons` or `WeaponModels` (crates/title/src/weapons.rs)
  has no row, or a row names a field that no longer exists;
- a cell's value differs from the source: a table path, `PerTick`/`PerTrip`,
  and `set`/`none` for a model field or an effect trigger;
- the provenance word differs from the source, **where the source carries an
  `Origin`**: `ghost_static` and the `effects.*` rows.

What it does not check, and the page says so: the provenance of a `Weapons`
axis or of any other `WeaponModels` field (comments only), every cell of the
aspect tables, and the confidence scores on the pages it links.

Text parsing is deliberate. The check runs in the gate on a machine with no
build, and a regexp over five small blocks is faster and no less honest than a
second parser that has to track the type definitions.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DOC = ROOT / "docs/gameplay/weapon-lineage.md"
TITLE_SRC = ROOT / "crates/title/src/weapons.rs"

TITLES = {
    "Pulse (PSP, PS2)": "pulse",
    "Pure": "pure",
    "HD / Fury": "hd",
    "2048": "2048",
    "Omega": "omega",
}

EFFECT_ROWS = {
    "effects.hit_spark": "HitSpark",
    "effects.leach_hit_spark": "LeachHitSpark",
    "effects.weapon_spark": "WeaponSpark",
    "effects.shield_absorb": "ShieldAbsorb",
    "effects.bomb_rays": "BombRays",
}

# Where each title's `Effects` are built. 2048 and Omega build them in lib.rs.
EFFECT_FILES = {
    "pulse": "crates/pulse/src/effects.rs",
    "pure": "crates/pure/src/effects.rs",
    "hd": "crates/hd/src/effects.rs",
    "2048": "crates/2048/src/lib.rs",
    "omega": "crates/omega/src/lib.rs",
}

# Where a title's `WeaponModels` literal may live, besides its lib.rs.
MODEL_FILES = {
    "pulse": ["crates/pulse/src/race.rs", "crates/pulse/src/lib.rs"],
    "pure": ["crates/pure/src/lib.rs"],
    "hd": ["crates/hd/src/race.rs", "crates/hd/src/lib.rs"],
    "2048": ["crates/2048/src/lib.rs"],
    "omega": ["crates/omega/src/weapons.rs", "crates/omega/src/lib.rs"],
}


def strip_comments(text: str) -> str:
    """Drop `//` comments. Raw strings in these blocks never contain `//`."""
    return re.sub(r"//[^\n]*", "", text)


def read(rel: str) -> str:
    return strip_comments((ROOT / rel).read_text(encoding="utf-8"))


def block_after(text: str, opener: str) -> str | None:
    """The brace-balanced body following the first `opener` that ends in `{`."""
    start = text.find(opener)
    while start != -1:
        brace = start + len(opener) - 1
        if text[brace] == "{":
            depth = 0
            for i in range(brace, len(text)):
                if text[i] == "{":
                    depth += 1
                elif text[i] == "}":
                    depth -= 1
                    if depth == 0:
                        return text[brace + 1 : i]
            return None
        start = text.find(opener, start + 1)
    return None


def split_fields(body: str) -> dict[str, str]:
    """`name: value` pairs at depth 0 of a struct literal body."""
    pieces: list[str] = []
    depth = 0
    cur: list[str] = []
    in_str = False
    for ch in body:
        if ch == '"':
            in_str = not in_str
        if not in_str:
            if ch in "([{":
                depth += 1
            elif ch in ")]}":
                depth -= 1
            elif ch == "," and depth == 0:
                pieces.append("".join(cur))
                cur = []
                continue
        cur.append(ch)
    pieces.append("".join(cur))
    fields: dict[str, str] = {}
    for piece in pieces:
        piece = piece.strip()
        m = re.match(r"(\w+)\s*:\s*(.*)$", piece, re.S)
        if m:
            fields[m.group(1)] = m.group(2).strip()
    return fields


def struct_fields(text: str, name: str) -> list[str]:
    body = block_after(text, f"pub struct {name} {{")
    if body is None:
        raise SystemExit(f"check-weapon-lineage: no `pub struct {name}` in {TITLE_SRC}")
    return re.findall(r"^\s*pub (\w+)\s*:", strip_comments(body), re.M)


def origin_word(value: str) -> str | None:
    m = re.search(r"Origin::(Measured|Chosen|InheritedFrom)", value)
    if not m:
        return None
    return {"Measured": "measured", "Chosen": "chosen", "InheritedFrom": "inherited"}[
        m.group(1)
    ]


def weapons_block(title: str) -> dict[str, str]:
    text = read(f"crates/{title}/src/lib.rs")
    body = block_after(text, "weapons: &oag_title::weapons::Weapons {")
    if body is None:
        raise SystemExit(f"check-weapon-lineage: no Weapons literal for {title}")
    return split_fields(body)


def models_block(title: str) -> dict[str, str]:
    for rel in MODEL_FILES[title]:
        text = read(rel)
        for opener in (
            "= &oag_title::weapons::WeaponModels {",
            "weapon_models: &oag_title::weapons::WeaponModels {",
        ):
            body = block_after(text, opener)
            if body is not None:
                return split_fields(body)
    raise SystemExit(f"check-weapon-lineage: no WeaponModels literal for {title}")


def path_of(value: str) -> str:
    """`Some(r"x")` -> `x`, `None` -> `none`."""
    if value == "None":
        return "none"
    m = re.search(r'r?"([^"]*)"', value)
    return m.group(1) if m else value


def model_value(value: str | None) -> str:
    if value is None or value in ("None", "false"):
        return "none"
    return "set"


def code_cells() -> dict[str, dict[str, tuple[str, str | None]]]:
    """axis -> title -> (value, origin word or None when the code carries none)."""
    cells: dict[str, dict[str, tuple[str, str | None]]] = {}
    model_fields = struct_fields(strip_comments(TITLE_SRC.read_text(encoding="utf-8")), "WeaponModels")
    for title in TITLES.values():
        weapons = weapons_block(title)
        for field in ("race", "elimination", "ai"):
            cells.setdefault(f"weapons.{field}", {})[title] = (path_of(weapons[field]), None)
        drain = weapons["leach_drain"].rsplit("::", 1)[-1]
        cells.setdefault("weapons.leach_drain", {})[title] = (drain, None)
        # `QuakeHit::WaveHeight { origin: Origin::X }`: the variant is the value,
        # and the source carries an `Origin`, so the provenance is checked.
        quake = weapons["quake_hit"]
        variant = re.search(r"QuakeHit::(\w+)", quake).group(1)
        cells.setdefault("weapons.quake_hit", {})[title] = (variant, origin_word(quake))

        models = models_block(title)
        for field, value in models.items():
            if field.startswith("."):
                continue
            origin = origin_word(value) if field == "ghost_static" else None
            cells.setdefault(f"weapon_models.{field}", {})[title] = (model_value(value), origin)
        # `..EMPTY` fills every field the literal does not spell out.
        for field in model_fields:
            cells.setdefault(f"weapon_models.{field}", {}).setdefault(title, ("none", None))

        text = read(EFFECT_FILES[title])
        for axis, trigger in EFFECT_ROWS.items():
            m = re.search(
                rf"Trigger::{trigger}\s*,\s*EffectSpec::new\(\s*[\w:]+\s*,\s*"
                rf"(?:oag_title::)?Origin::(\w+)",
                text,
            )
            if m:
                word = {"Measured": "measured", "Chosen": "chosen", "InheritedFrom": "inherited"}[
                    m.group(1)
                ]
                cells.setdefault(axis, {})[title] = ("set", word)
            else:
                cells.setdefault(axis, {})[title] = ("none", None)
    return cells


def doc_cells(text: str) -> dict[str, dict[str, tuple[str, str]]]:
    section = text.split("## Checked axes", 1)
    if len(section) != 2:
        raise SystemExit("check-weapon-lineage: no `## Checked axes` heading")
    rows: dict[str, dict[str, tuple[str, str]]] = {}
    header: list[str] | None = None
    for line in section[1].splitlines():
        if line.startswith("## ") and header is not None:
            break
        if not line.startswith("|"):
            continue
        cols = [c.strip() for c in line.strip().strip("|").split("|")]
        if cols[0] == "Axis":
            header = cols
            continue
        if header is None or set(cols[0]) <= set("- "):
            continue
        axis = cols[0].strip("`")
        if len(cols) != len(header):
            raise SystemExit(f"check-weapon-lineage: row `{axis}` has {len(cols)} cells, want {len(header)}")
        for name, cell in zip(header[1:], cols[1:]):
            title = TITLES.get(name)
            if title is None:
                raise SystemExit(f"check-weapon-lineage: unknown column `{name}`")
            m = re.match(r"`([^`]*)`\s*-\s*(\w+)\s*$", cell)
            if not m:
                raise SystemExit(f"check-weapon-lineage: `{axis}` / {name}: cell `{cell}` is not `value` - provenance")
            rows.setdefault(axis, {})[title] = (m.group(1), m.group(2))
    return rows


PROVENANCE = {"measured", "inherited", "chosen", "absent", "unread"}


def main() -> int:
    errors: list[str] = []
    src = strip_comments(TITLE_SRC.read_text(encoding="utf-8"))
    want_axes = {f"weapons.{f}" for f in struct_fields(src, "Weapons")}
    want_axes |= {f"weapon_models.{f}" for f in struct_fields(src, "WeaponModels")}
    want_axes |= set(EFFECT_ROWS)

    doc = doc_cells(DOC.read_text(encoding="utf-8"))
    code = code_cells()

    for axis in sorted(want_axes - set(doc)):
        errors.append(f"{axis}: a field of the code with no row in {DOC.name}")
    for axis in sorted(set(doc) - want_axes):
        errors.append(f"{axis}: a row in {DOC.name} with no field in the code")

    for axis in sorted(want_axes & set(doc)):
        for title in TITLES.values():
            want_value, want_origin = code[axis][title]
            got = doc[axis].get(title)
            if got is None:
                errors.append(f"{axis} / {title}: no cell")
                continue
            value, provenance = got
            if provenance not in PROVENANCE:
                errors.append(f"{axis} / {title}: provenance `{provenance}` is none of {sorted(PROVENANCE)}")
            if value != want_value:
                errors.append(f"{axis} / {title}: page says `{value}`, source says `{want_value}`")
            if want_origin is not None and provenance != want_origin:
                errors.append(f"{axis} / {title}: page says {provenance}, the source's Origin is {want_origin}")
            if want_origin is None and axis.startswith("effects.") and want_value == "none" and provenance in (
                "measured",
                "inherited",
                "chosen",
            ):
                errors.append(f"{axis} / {title}: a `none` cell cannot be {provenance}; use absent or unread")

    if errors:
        print(f"check-weapon-lineage: {len(errors)} disagreement(s) with {DOC.relative_to(ROOT)}:")
        for line in errors:
            print(f"  {line}")
        return 1
    print(f"check-weapon-lineage: {len(want_axes)} axes x {len(TITLES)} titles agree with {DOC.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
