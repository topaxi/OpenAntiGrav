#!/usr/bin/env python3
"""The picture's lag behind RAM, per grab, from a tabulated camera path: the check `hd_picture_lag.fit` is held against.

    python3 scripts/hd-picture-lag-table.py RECORD.json [RECORD.json ...]

Every record (`rpcs3-hd-detonator-shots.py`'s `NN.json` with `NN.png` beside it) holds one push-buffer camera at tick
`T - skew` (`T` the RAM lap tick, `skew` the hull's whole ticks behind RAM). A Detonator craft flies the same path on every
boot, so the cameras of all the records tabulate the camera's pose per tick (eye and the rotation-and-projection part of
the view-projection, linear between neighbours). Each record's near mines are then reprojected through the tabulated camera
at `T - L` for `L` from 0 to 12 ticks; the printed `L` is the lag behind RAM that puts them on their blobs (`L - skew` is how
far behind the push buffer), with the rms and each mine's own `L`. No camera velocity is assumed and the rotation is the
path's, which is what the single-grab fit cannot see; the table needs records on both sides of every tick it is asked about
(a gap over 40 ticks is refused). Prints one line per record that has a camera, a hull and at least one mine blob.
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import hd_picture_lag as hpl  # noqa: E402

GAP_MAX = 40
L_STEP = 0.05
L_MAX = 12.0


def translation(eye):
    m = np.eye(4)
    m[3, :3] = eye
    return m


def load(path):
    record = json.loads(Path(path).read_text())
    ship, camera = record["ship"], record.get("camera")
    hull = ship.get("hull_pb")
    if not camera or not hull:
        return None
    tick = np.array(ship["vel"]) / 60.0
    skew = float(np.dot(np.array(ship["pos"]) - np.array(hull["translation"]), tick) / np.dot(tick, tick))
    lap_tick = int(round(ship["race_clock_s"] * 60))
    eye = np.array(camera["eye"])
    view_proj = np.array(camera["view_proj"]).reshape(4, 4)
    return {"path": Path(path), "record": record, "lap_tick": lap_tick, "skew": skew, "pb_tick": lap_tick - int(round(skew)),
            "eye": eye, "rest": translation(eye) @ view_proj}


class Path4:
    """The camera's eye and view-projection-less-eye per tick."""

    def __init__(self, loaded):
        by_tick = {}
        for r in loaded:
            by_tick.setdefault(r["pb_tick"], []).append(r)
        self.ticks = np.array(sorted(by_tick), float)
        self.eyes = np.array([np.mean([r["eye"] for r in by_tick[int(t)]], axis=0) for t in self.ticks])
        self.rests = np.array([np.mean([r["rest"] for r in by_tick[int(t)]], axis=0) for t in self.ticks])

    def view_proj(self, t):
        i = int(np.searchsorted(self.ticks, t))
        if i == 0 or i >= len(self.ticks) or self.ticks[i] - self.ticks[i - 1] > GAP_MAX:
            return None
        w = (t - self.ticks[i - 1]) / (self.ticks[i] - self.ticks[i - 1])
        eye = self.eyes[i - 1] * (1 - w) + self.eyes[i] * w
        rest = self.rests[i - 1] * (1 - w) + self.rests[i] * w
        return translation(-eye) @ rest


def fit(path, r):
    ship, camera = r["record"]["ship"], r["record"]["camera"]
    mines = [m for m in ship.get("mines") or [] if m.get("live")]
    mask = hpl.blob_mask(r["path"].with_suffix(".png"))
    height, width = mask.shape
    eye, forward = r["eye"], np.array(camera["forward"])
    reference = path.view_proj(r["lap_tick"] - 6)
    if reference is None:
        return None

    def project(point, view_proj):
        p = np.append(point, 1.0) @ view_proj
        return np.array([(p[0] / p[3] * 0.5 + 0.5) * width, (0.5 - p[1] / p[3] * 0.5) * height])

    observed = []
    for mine in mines:
        point = np.array(mine["position"])
        depth = float(np.dot(point - eye, forward))
        if not hpl.Z_MIN < depth < 60.0:
            continue
        guess = project(point, reference)
        if not (0 <= guess[0] < width and 0 <= guess[1] < height):
            continue
        blob = hpl.blob_centroid(mask, guess, int(min(hpl.WINDOW_CAP, hpl.WINDOW_PX / depth)))
        if blob is not None:
            observed.append((point, blob, depth))
    if not observed:
        return None
    lags = np.arange(0.0, L_MAX + 1e-9, L_STEP)
    views = [path.view_proj(r["lap_tick"] - lag) for lag in lags]
    errors = np.array([[np.linalg.norm(project(p, v) - c) if v is not None else np.inf for lag, v in zip(lags, views)]
                       for p, c, _ in observed])
    rms = np.sqrt(np.mean(errors ** 2, axis=0))
    best = int(np.argmin(rms))
    per = [(d, float(lags[int(np.argmin(errors[i]))]), float(errors[i].min())) for i, (_, _, d) in enumerate(observed)]
    return float(lags[best]), float(rms[best]), per


def main():
    loaded = [r for r in (load(p) for p in sys.argv[1:]) if r]
    if not loaded:
        sys.exit("no record with a camera and a push-buffer hull")
    path = Path4(loaded)
    print("camera path ticks:", " ".join("%d" % t for t in path.ticks))
    print("%-30s %4s %5s %3s %6s %7s %7s  per mine (depth: L, residual px)" % ("record", "T", "skew", "n", "L", "rms_px", "L-skew"))
    for r in loaded:
        got = fit(path, r)
        if got is None:
            continue
        lag, rms, per = got
        print("%-30s %4d %5.2f %3d %6.2f %7.1f %7.2f  %s" % ("/".join(r["path"].parts[-2:]), r["lap_tick"], r["skew"], len(per), lag, rms,
                                                          lag - r["skew"], " ".join("z%.0f:%.2f(%.0f)" % p for p in per)))


if __name__ == "__main__":
    main()
