# docs/architecture/memory.md, "A finished race held under its replay (ADR-0099 step 5)".
"""The measured points of a walk, shared by the desktop and phone samplers."""

import re
import subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent
IMAGES = {"pulse": "pulse-psp-eu.chd", "hd": "hdfury-ps3-eu-dec.iso"}


def walk_text(title, override=None):
    """The walk to run: `walk.sh`'s, or the file `override` names."""
    if override:
        return Path(override).read_text()
    return subprocess.run([str(HERE / "walk.sh"), title], check=True, capture_output=True,
                          text=True).stdout


def end_tick(walk):
    """The sim tick the walk holds the end menu until; (b) is the 700 ticks before it.
    None for a walk cut short of the end menu."""
    found = re.search(r"screenshot SHOTS/b\.png; until sim_tick=(\d+)", walk)
    return int(found.group(1)) if found else None


def window(state, seen, end):
    """Which point a state line belongs to, or None. `seen` holds the points already met."""
    stage, tick = state.get("stage"), state.get("sim_tick") or 0
    if stage == "menu" and state.get("page") == "race" and "a" not in seen:
        return "pre"
    if stage == "menu" and state.get("page") == "main" and "c" in seen:
        return "d"
    if stage != "race":
        return None
    if state.get("replay") == "playing" and 3000 <= tick <= 3600:
        return "c"
    if state.get("replay") is None:
        # The end menu again, under a replay viewer that popped back to it.
        if state.get("endrace") == "other" and "c" in seen:
            return "e"
        if state.get("endrace") is None and 3000 <= tick <= 3600:
            return "a"
        if end and state.get("endrace") == "other" and end - 700 <= tick <= end:
            return "b"
    return None


def new_directory(path):
    """`path` made empty: a run never mixes with, or starts from, an earlier one."""
    out = Path(path).resolve()
    if out.exists() and any(out.iterdir()):
        raise SystemExit(f"{out} is not empty; give each run a directory of its own")
    out.mkdir(parents=True, exist_ok=True)
    return out
