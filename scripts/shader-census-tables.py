#!/usr/bin/env python3
"""Render the shader census tables for docs/rendering/shader-census.md.

  OAG_CENSUS_OUT=<dir> OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-mesh \
      --run-ignored all -E 'test(/^shader_census_ground_truth::/)' --no-fail-fast
  python3 scripts/shader-census-tables.py <dir>

Reads the `*-table.tsv` files the ground-truth test writes and prints markdown.
"""

import collections
import csv
import pathlib
import sys

TITLES = {
    "HD/Fury": ["hd"],
    "Omega": ["omega-hd-circuits", "omega-2048-circuits", "omega-rest"],
    "2048": ["2048"],
}

# Inputs the generic reading covers or every program of a title declares: the
# albedo, the lightmap, the lights, fog, shadow maps and the Zone variant's own.
STANDARD = {
    "Texture1", "Texture1Sampler", "diffuseTexture", "DiffuseTexture",
    "DiffuseTextureSampler", "DiffuseMap", "DiffuseMapSampler", "Diffuse_Texture",
    "Diffuse_TextureSampler", "DiffuseSampler", "DiffuseAlphaMap",
    "DiffuseAlphaMapSampler", "lightmap", "lightmapSampler",
    "02df31e5", "2dba643d", "3dc31258", "81db67ea", "002c73e8", "8670f0be",
    "constantAmbientColour", "fogColour", "occlusionMap", "shadowMap",
    "shadowMapSampler", "shadowCoeffs", "shadowLightDirection",
    "shadowLightSpecularDiffuse", "novaScaleBiasPower", "liveLighting0direction",
    "liveLighting0diffuse", "liveLighting0specular", "probeOffsetAndDiffuse",
    "probeOffsets", "shCoeffs", "fadeResCoeffs", "reflectiveBlurCoeffs", "ibl",
}


def extras(reads):
    return [
        n for n in reads.split()
        if n not in STANDARD and not n.lower().startswith(("zonegrowing", "zoneaniso",
        "zonecolourtint", "zoneeffect", "zonetex"))
    ]


def load(directory, shards):
    rows = {}
    for shard in shards:
        path = directory / f"{shard}-table.tsv"
        with path.open(newline="") as handle:
            for r in csv.DictReader(handle, delimiter="\t", quoting=csv.QUOTE_NONE):
                key = (r["key"], r["status"])
                row = rows.setdefault(key, {
                    "chunks": 0, "materials": set(), "places": collections.Counter(),
                    "example": r["example"], "reads": r["reads"] or "",
                })
                row["chunks"] += int(r["chunks"])
                row["materials"] |= set(r["materials"].split(","))
                for item in r["places"].split(","):
                    name, n = item.rsplit(":", 1)
                    row["places"][name] += int(n)
    return rows


def top(items, n):
    shown = ", ".join(items[:n])
    return shown + (f", +{len(items) - n}" if len(items) > n else "")


def table(selected, limit, with_extras):
    head = "| program | status | chunks | material families | circuits (chunks) | file |"
    if with_extras:
        head += " reads beyond the standard inputs |"
    print(head)
    print("|" + "---|" * (head.count("|") - 1))
    for (key, status), r in selected[:limit]:
        places = [f"{p} {n}" for p, n in r["places"].most_common()]
        line = (
            f"| `{key}` | {status} | {r['chunks']} | {top(sorted(r['materials']), 3)} "
            f"| {top(places, 3)} | `{r['example']}` |"
        )
        if with_extras:
            line += f" {top(extras(r['reads']), 8)} |"
        print(line)


def main():
    directory = pathlib.Path(sys.argv[1])
    loaded = {t: load(directory, s) for t, s in TITLES.items()}
    print("## Totals\n")
    print("| title | programs | chunks | matcher | generic, reads more | generic, plain | unread |")
    print("|---|---|---|---|---|---|---|")
    for title, rows in loaded.items():
        total = sum(r["chunks"] for r in rows.values())
        share = collections.Counter()
        for (_, status), r in rows.items():
            kind = status.split(":")[0]
            if kind == "generic":
                kind = "extra" if extras(r["reads"]) else "plain"
            share[kind] += r["chunks"]
        pct = lambda k: f"{share[k]} ({100 * share[k] / total:.1f}%)"
        programs = len({k for k, _ in rows})
        print(f"| {title} | {programs} | {total} | {pct('matcher')} | {pct('extra')} "
              f"| {pct('plain')} | {pct('unread')} |")
    print("\n## Chunks by matcher\n")
    for title, rows in loaded.items():
        by = collections.Counter()
        for (_, status), r in rows.items():
            if status.startswith("matcher:"):
                by[status[8:]] += r["chunks"]
        print(f"- **{title}**: " + ", ".join(f"{k} {v}" for k, v in by.most_common(8)))
    for title, rows in loaded.items():
        ranked = sorted(rows.items(), key=lambda kv: (-kv[1]["chunks"], kv[0]))
        unread = [kv for kv in ranked if kv[0][1].startswith("unread")]
        print(f"\n## {title}: unread ({len(unread)} programs)\n")
        table(unread, 30, False)
        generic = [kv for kv in ranked if kv[0][1] == "generic" and extras(kv[1]["reads"])]
        print(f"\n## {title}: generic, reading more than the standard inputs "
              f"({len(generic)} programs, top 30)\n")
        table(generic, 30, True)


if __name__ == "__main__":
    main()
