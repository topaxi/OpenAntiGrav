#!/usr/bin/env python3
"""Enforce the dependency-boundary rules CLAUDE.md calls "enforceable".

Finding 2 of the 2026-07-30 code review: both rules held on inspection, but
nothing ran that check - no CI job, no test, no `deny.toml`. A `cargo add`
that broke one would pass every other gate, and the property it protects (a
simulation testable without a GPU and comparable against a trace) is the one
thing the architecture exists for. This runs `cargo metadata`, walks the
resolved normal (non-dev, non-build) dependency graph, and fails the same way
clippy or fmt would.
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Rule 1: no gameplay crate may reach a rendering/audio/input/windowing
# dependency. The simulation consumes an input *snapshot type* owned by
# oag-gameplay, never the input system itself. oag-audio was listed here before
# it existed, so that the rule would hold the moment it did; it exists now
# (`crates/audio`, a mixer and a cpal device) and the entry means what it says.
# oag-core is included too: oag-render also depends on it, so it
# is not exclusive to the gameplay branch, but a forbidden dependency landing
# there would reach oag-gameplay just as surely as one added directly.
# oag-title and oag-pulse are here for the reason ADR-0022 gives: a title
# package is tables, readable from both sides of the simulation boundary, so
# rule 1 has to cover them or a Pulse constant becomes the back door a texture
# handle walks through. **Every** title package joins in the same change that
# creates it; this script validates that every name here is a real workspace
# member, so it cannot be listed ahead of time the way oag-audio once was.
# oag-pure did join that way and oag-hd did not, which is how rule 1 stopped
# covering a third of the title packages for as long as it took the 2026-08-18
# review (finding S1) to notice. The list is the invariant, so a new title
# crate with no line here is a hole with no symptom.
GAMEPLAY_CRATES = {
    "oag-2048",
    "oag-ai",
    "oag-core",
    "oag-gameplay",
    "oag-hd",
    "oag-omega",
    "oag-physics",
    "oag-pulse",
    "oag-pure",
    "oag-race",
    # A replay is what the simulation consumed and what it arrived at, so it
    # sits on the simulation's side of the line; the ghost it carries is poses,
    # drawn by the composition root. ADR-0055.
    "oag-replay",
    "oag-title",
    "oag-weapons",
}
# oag-gpu, oag-mesh, oag-post and oag-fx are the crates split out of oag-render;
# each links wgpu so the entry is redundant today, and is listed for the reason
# oag-render is: the rule should name the renderer's crates, not lean on one
# transitive dependency staying put.
FORBIDDEN_FOR_GAMEPLAY = {
    "oag-render",
    "oag-fx",
    "oag-gpu",
    "oag-mesh",
    "oag-post",
    "oag-audio",
    "oag-input",
    "winit",
    "wgpu",
}

# The crates that link `oag-title` without being title packages: the reader of
# any title's archives, the composition root that picks one, and the trace
# harness, which is a CLI over a race and reads the vocabulary to spell-check
# `--class` against every measured speed-class ladder. Every *other*
# workspace member that links `oag-title` is a title package by construction -
# a package exists to fill that vocabulary in - so it belongs in
# GAMEPLAY_CRATES above.
#
# This is what stops S1 recurring. Listing `oag-hd` fixes the instance; the
# reason it went missing for as long as it did is that a title package with no
# line in GAMEPLAY_CRATES has no symptom - the gate passes, quieter than
# before. So a new crate reaching for `oag-title` now has to be classified
# here or there, and cannot be neither.
TITLE_VOCABULARY = "oag-title"
# `oag-ui` and `oag-ui-screens` joined deliberately, not by omission: menus draw, so this crate may
# one day link a renderer directly, which is exactly what `GAMEPLAY_CRATES`
# exists to keep away from everything gameplay depends on. Classifying it here
# rather than there is what makes that legal for `oag-ui` alone - nothing
# gameplay-side may ever depend on `oag-ui` (`oag-game`'s own composition-root
# rule already forbids the only crate that could try).
NOT_TITLE_PACKAGES = {
    "oag-assets",
    "oag-game",
    "oag-hud",
    "oag-livery",
    "oag-music",
    "oag-present",
    "oag-raceplay",
    "oag-sound",
    "oag-source",
    "oag-trace",
    "oag-ui",
    "oag-ui-screens",
}

# Rule 3: the generic render-side crates carry no title's data. `oag-mesh` was
# the one that did (a re-exported Pulse animation table and the PS2 texture-name
# rule); both moved, and this stops it recurring. `oag-render`, `oag-present` and
# `oag-ui` are deliberately not listed: `oag-render` still reads `oag-pulse`'s
# presentation tables (`loading`, HUD) and `oag-present` reaches `oag-ui`, whose
# front end names every title. `oag-title` (the vocabulary) is not forbidden
# here: it is types only, and `oag-assets` carries it.
TITLE_DATA_CRATES = {
    "oag-2048",
    "oag-hd",
    "oag-omega",
    "oag-pulse",
    "oag-pure",
}
TITLE_FREE_RENDER_CRATES = {
    "oag-fx",
    "oag-gpu",
    "oag-mesh",
    "oag-post",
}

# Rule 2: no crate may depend on the composition root.
COMPOSITION_ROOT = "oag-game"


def load_metadata() -> dict:
    result = subprocess.run(
        ["cargo", "metadata", "--format-version=1"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=True,
    )
    return json.loads(result.stdout)


def normal_dep_graph(meta: dict) -> tuple[dict[str, str], dict[str, list[str]]]:
    """Returns (package id -> name, package id -> normal dependency ids)."""
    names = {pkg["id"]: pkg["name"] for pkg in meta["packages"]}
    graph = {
        node["id"]: [
            dep["pkg"]
            for dep in node["deps"]
            if any(kind["kind"] is None for kind in dep["dep_kinds"])
        ]
        for node in meta["resolve"]["nodes"]
    }
    return names, graph


def reachable_names(
    start: str, graph: dict[str, list[str]], names: dict[str, str]
) -> set[str]:
    seen: set[str] = set()
    stack = [start]
    while stack:
        current = stack.pop()
        if current in seen:
            continue
        seen.add(current)
        stack.extend(graph.get(current, []))
    seen.discard(start)
    return {names[pkg_id] for pkg_id in seen}


def main() -> int:
    meta = load_metadata()
    names, graph = normal_dep_graph(meta)
    id_by_name = {name: pkg_id for pkg_id, name in names.items()}
    problems: list[str] = []

    for crate in sorted(GAMEPLAY_CRATES):
        pkg_id = id_by_name.get(crate)
        if pkg_id is None:
            problems.append(f"workspace member {crate!r} not found in cargo metadata")
            continue
        hit = reachable_names(pkg_id, graph, names) & FORBIDDEN_FOR_GAMEPLAY
        if hit:
            problems.append(
                f"{crate} transitively depends on {sorted(hit)}, "
                "which rule 1 (no gameplay crate depends on a renderer/audio/"
                "input/window backend) forbids"
            )

    for crate in sorted(TITLE_FREE_RENDER_CRATES):
        pkg_id = id_by_name.get(crate)
        if pkg_id is None:
            problems.append(f"workspace member {crate!r} not found in cargo metadata")
            continue
        hit = reachable_names(pkg_id, graph, names) & TITLE_DATA_CRATES
        if hit:
            problems.append(
                f"{crate} transitively depends on {sorted(hit)}, which rule 3 "
                "(a generic render-side crate carries no title's data) forbids; "
                "inject the table from the caller instead"
            )

    vocabulary_id = id_by_name.get(TITLE_VOCABULARY)
    if vocabulary_id is None:
        problems.append(f"workspace member {TITLE_VOCABULARY!r} not found in cargo metadata")
    else:
        unclassified = sorted(
            names[member_id]
            for member_id in meta["workspace_members"]
            if vocabulary_id in graph.get(member_id, [])
            and names[member_id] not in GAMEPLAY_CRATES
            and names[member_id] not in NOT_TITLE_PACKAGES
        )
        if unclassified:
            problems.append(
                f"{unclassified} depend on {TITLE_VOCABULARY} but are in neither "
                "GAMEPLAY_CRATES nor NOT_TITLE_PACKAGES in this script. A crate "
                "that fills the title vocabulary is a title package and rule 1 "
                "has to cover it (add it to GAMEPLAY_CRATES); if it reads titles "
                "rather than being one, say so in NOT_TITLE_PACKAGES."
            )

    root_id = id_by_name.get(COMPOSITION_ROOT)
    if root_id is None:
        problems.append(f"composition root {COMPOSITION_ROOT!r} not found in cargo metadata")
    else:
        dependents = sorted(
            names[member_id]
            for member_id in meta["workspace_members"]
            if member_id != root_id and root_id in graph.get(member_id, [])
        )
        if dependents:
            problems.append(
                f"{dependents} depend on {COMPOSITION_ROOT}, "
                "which rule 2 (nothing depends on the composition root) forbids"
            )

    if problems:
        print("\n\n".join(problems), file=sys.stderr)
        return 1

    print("OK: all three dependency-boundary rules hold")
    return 0


if __name__ == "__main__":
    sys.exit(main())
