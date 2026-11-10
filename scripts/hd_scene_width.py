#!/usr/bin/env python3
"""The width a Wipeout HD 1080p frame's scene was drawn at, read off the frame or off the running game.

    python3 scripts/hd_scene_width.py <frame.png>...          # one line per frame
    python3 scripts/hd_scene_width.py --record <pair-dir>...  # write original.scene_width into each pair.json

HD's 1080p path draws the scene into one of 32 targets, `1920 - 20 * level` pixels wide (level 0 to 31,
1920 down to 1300, all 1080 lines), and stretches it to 1920 in the final stage. The level is
`g_RenderGlobals + 0x88`; the end of `Render_FinishFrame` moves it one step a frame
(docs/ghidra/functions/ps3-hdfury-eu/scene-width-throttle.md).

Off the frame: a linear stretch from `w` to 1920 leaves the horizontal second difference periodic at
`(1920 - w) / 1920` cycles a pixel, which is bin `20 * level` of a 1920-point spectrum of its column
profile. `measure` scores each level's bin against the bins around it and takes the best; under
`PEAK` nothing was stretched and the frame is 1920. The page's "Reading the width off a frame" has
the frames of known width this was checked on, and its limit: race frames only.

Off the game: `live(read)` reads the level and the enable byte in a pause. The presented frame is up to
two frames older than the paused game, so while the controller moves the level read can be two steps
from the frame's; a pair records both and the frame's value is the pair's width.
"""
import json
import struct
import sys
from pathlib import Path

G_RENDER_GLOBALS = 0x00C81A80
LEVEL = G_RENDER_GLOBALS + 0x88
ENABLE = G_RENDER_GLOBALS + 0x324
FULL = 1920
STEP = 20
LEVELS = 32
#: A stretched frame's weakest score on the frames of known width was 6.7 (1840); an unstretched
#: frame's strongest was 3.8, and 4.3 on the stored front-end originals.
PEAK = 5.5
METHOD = "frame: periodicity of the horizontal second difference (scripts/hd_scene_width.py)"


def width_of(level):
    return FULL - STEP * level


def scores(path):
    """`(score, level)` for levels 1 to 31, best first: the level's bin over the median of 16 bins around it."""
    import numpy as np
    from PIL import Image

    rgb = np.asarray(Image.open(path).convert("RGB"), dtype=np.float64)
    if rgb.shape[1] != FULL:
        raise ValueError("%s is %d wide, not %d" % (path, rgb.shape[1], FULL))
    luma = 0.299 * rgb[..., 0] + 0.587 * rgb[..., 1] + 0.114 * rgb[..., 2]
    profile = np.abs(luma[:, 2:] - 2 * luma[:, 1:-1] + luma[:, :-2]).mean(axis=0)
    profile = np.concatenate([[profile[0]], profile, [profile[-1]]])
    spectrum = np.abs(np.fft.rfft(profile - profile.mean()))
    out = []
    for level in range(1, LEVELS):
        b = STEP * level
        around = np.concatenate([spectrum[b - 9:b - 1], spectrum[b + 2:b + 10]])
        out.append((float(spectrum[b] / np.median(around)), level))
    return sorted(out, reverse=True)


def measure(path):
    """The record a pair keeps: the frame's scene width, its level, the score and the runner-up."""
    ranked = scores(path)
    (best, level), (second, second_level) = ranked[0], ranked[1]
    stretched = best >= PEAK
    return {
        "px": width_of(level) if stretched else FULL,
        "level": level if stretched else 0,
        "method": METHOD,
        "score": round(best, 1),
        "peak": PEAK,
        "best_candidate_px": width_of(level),
        "runner_up": {"px": width_of(second_level), "score": round(second, 1)},
    }


def front_end(found):
    """A front-end frame's record: 1920 by how it is drawn, the frame's score kept beside it unused.

    A menu or loading screen is 2D on 1920-wide surfaces and its regular patterns (the loading screen's
    dot grid) score like a stretch, so the frame is no evidence there.
    """
    return {
        "px": FULL,
        "level": 0,
        "method": "front end: drawn on 1920-wide surfaces, the throttle is off in the menus "
                  "(docs/ghidra/functions/ps3-hdfury-eu/scene-width-throttle.md); the frame's score is not used",
        "frame_score_unused": {"px": found["best_candidate_px"], "score": found["score"]},
    }


def live(read):
    """The level and the throttle's enable byte, `read(address, size)` being guest memory in a pause."""
    level = struct.unpack(">I", read(LEVEL, 4))[0]
    if level >= LEVELS:
        raise ValueError("level %d at %#x: not this executable" % (level, LEVEL))
    return {"level": level, "px": width_of(level), "throttle_enabled": read(ENABLE, 1) != b"\x00"}


def record(pair_dir):
    """Add `original.scene_width` to a pair's `pair.json`, touching nothing else in it."""
    pair_dir = Path(pair_dir)
    path = pair_dir / "pair.json"
    raw = path.read_text()
    pair = json.loads(raw)
    tail = raw[len(raw.rstrip("\n")):]
    if json.dumps(pair, indent=2) + tail != raw:
        raise ValueError("%s is not `json.dumps(indent=2)`: rewriting it would move more than one field" % path)
    found = measure(pair_dir / "theirs.png")
    if "screen" in pair["original"]:
        found = front_end(found)
    kept = pair["original"].get("scene_width", {})
    pair["original"]["scene_width"] = {**found, **{k: kept[k] for k in ("at_pause",) if k in kept}}
    found = pair["original"]["scene_width"]
    path.write_text(json.dumps(pair, indent=2) + tail)
    return found


def main():
    args = sys.argv[1:]
    if not args:
        sys.exit(__doc__)
    if args[0] == "--record":
        for pair_dir in args[1:]:
            found = record(pair_dir)
            if "score" in found:
                print("%-24s %4d  score %5.1f  runner-up %d at %.1f" % (
                    Path(pair_dir).name, found["px"], found["score"], found["runner_up"]["px"], found["runner_up"]["score"]))
            else:
                print("%-24s %4d  front end" % (Path(pair_dir).name, found["px"]))
        return
    for path in args:
        found = measure(path)
        print("%s: %d (level %d, score %.1f, best candidate %d, runner-up %d at %.1f)" % (
            path, found["px"], found["level"], found["score"], found["best_candidate_px"],
            found["runner_up"]["px"], found["runner_up"]["score"]))


if __name__ == "__main__":
    main()
