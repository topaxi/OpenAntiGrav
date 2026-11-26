#!/usr/bin/env python3
"""How many ticks older than the push buffer's camera is the picture an RPCS3 grab shows? Fitted per grab from the mines.

    python3 scripts/hd_picture_lag.py            # the self test
    python3 scripts/hd_picture_lag.py RECORD.json  # one grab: `rpcs3-hd-detonator-shots.py`'s NN.json with NN.png beside it

A frame whose mines fit no value is measured off the static scene instead (`static_fit`, the pair script's fallback).

A Detonator grab holds three instants (`docs/tools/compare-page.md`, "The presented lag of a Detonator grab"): RAM (the
pause), the push buffer (the frame the game last handed the RSX: its hull and camera sit `skew` whole ticks behind RAM, 2 or
4 on a Detonator grab) and the picture (older still). Each laid mine is a static object at a position read from RAM, so the
bright blob it draws in the picture is a cross-track and depth measurement of where the picture's camera was: the camera
is moved back `k` ticks along its own velocity and `k` is the value that puts each near mine's projected centre on its blob (a blob
that fits no `k` is dropped, the rest are averaged).
The camera's velocity is read, not assumed: the camera rig's eye in RAM less the push buffer's eye, over the skew (it is
the craft's speed but not its direction, the camera's height follows the track). A grab that fits no `k` (fewer than two mines
between `Z_MIN` and `Z_MAX` units, fewer than two whose own best fit lands within `RES_MAX` pixels of the blob, mines that disagree by over `SPREAD_MAX` ticks) returns
`ticks: None` and the reason; nothing is guessed.
"""
import json
import sys
from pathlib import Path

Z_MIN, Z_MAX = 8.0, 50.0
#: A mine's blob is searched inside a window of `WINDOW_PX * focal / depth`, capped at `WINDOW_CAP`.
WINDOW_PX, WINDOW_CAP = 1600.0, 150
MIN_BLOB_PX = 30
#: A mine's own best fit must land within `RES_MAX` pixels of its blob, and the mines kept must agree to `SPREAD_MAX` ticks.
RES_MAX = 8.0
SPREAD_MAX = 1.0
K_RANGE = (-1.0, 9.0)
K_STEP = 0.025
SIZE = (1920, 1080)


def blob_mask(path):
    import numpy as np
    from PIL import Image
    img = np.asarray(Image.open(path).convert("RGB")).astype(int)
    return (img[..., 2] > 180) & (img[..., 0] > 150) & (img[..., 0] - img[..., 1] > 12)


def blob_centroid(mask, centre, window):
    import numpy as np
    height, width = mask.shape
    x0, x1 = int(max(0, centre[0] - window)), int(min(width, centre[0] + window))
    y0, y1 = int(max(0, centre[1] - window)), int(min(height, centre[1] + window))
    if x1 <= x0 or y1 <= y0:
        return None
    ys, xs = np.nonzero(mask[y0:y1, x0:x1])
    if len(xs) < MIN_BLOB_PX:
        return None
    return np.array([xs.mean() + x0, ys.mean() + y0])


def fit(camera, ship, theirs_png, skew):
    """`{"ticks", "reason", "rms_px", "mines", "per_mine", "camera_velocity"}` for one grab; `ticks` is `None` with a `reason` when no fit stands.

    `camera` is the grab record's `camera` (`view_proj` row-vector 4x4 flat, `eye`, `forward`), `ship` its `ship` (`mines`, `rig_eye`),
    `skew` the whole ticks the push buffer's hull sits behind RAM (`rpcs3-hd-pair.py`'s `frame_pose`)."""
    import numpy as np
    out = {"ticks": None, "reason": None, "rms_px": None, "mines": 0, "per_mine": [], "camera_velocity": None}
    if not ship.get("rig_eye"):
        out["reason"] = "the record has no camera rig eye (`ship.rig_eye`), so the camera's own velocity is unknown (re-grab)"
        return out
    mines = [m for m in ship.get("mines") or [] if m.get("live")]
    if not mines:
        out["reason"] = "the record holds no live mine"
        return out
    eye, forward = np.array(camera["eye"]), np.array(camera["forward"])
    view_proj = np.array(camera["view_proj"]).reshape(4, 4)
    velocity = (np.array(ship["rig_eye"]) - eye) / skew
    out["camera_velocity"] = [float(v) for v in velocity]
    mask = blob_mask(theirs_png)
    height, width = mask.shape

    def project(point, k):
        p = np.append(point + k * velocity, 1.0) @ view_proj
        return np.array([(p[0] / p[3] * 0.5 + 0.5) * width, (0.5 - p[1] / p[3] * 0.5) * height])

    observed = []
    for mine in mines:
        point = np.array(mine["position"])
        depth = float(np.dot(point - eye, forward))
        if not Z_MIN < depth < Z_MAX:
            continue
        guess = project(point, 4.0)
        if not (0 <= guess[0] < width and 0 <= guess[1] < height):
            continue
        blob = blob_centroid(mask, guess, int(min(WINDOW_CAP, WINDOW_PX / depth)))
        if blob is not None:
            observed.append((point, blob, depth))
    out["mines"] = len(observed)
    if len(observed) < 2:
        out["reason"] = "%d mine blob(s) between %g and %g units (2 needed)" % (len(observed), Z_MIN, Z_MAX)
        return out
    ks = np.arange(K_RANGE[0], K_RANGE[1] + 1e-9, K_STEP)
    per = []
    for point, blob, depth in observed:
        errors = [np.linalg.norm(project(point, k) - blob) for k in ks]
        best = int(np.argmin(errors))
        per.append({"depth": depth, "ticks": float(ks[best]), "residual_px": float(errors[best]), "edge": best in (0, len(ks) - 1)})
    out["per_mine"] = per
    # A blob that merges with its neighbour, or a mine half off the frame, sits far from any camera on the path: it is dropped,
    # not averaged in. What is left must agree.
    kept = [p for p in per if p["residual_px"] <= RES_MAX and not p["edge"]]
    if len(kept) < 2:
        out["reason"] = "%d of %d mines sit on a camera within %g px (2 needed)" % (len(kept), len(per), RES_MAX)
        return out
    values = sorted(p["ticks"] for p in kept)
    if values[-1] - values[0] > SPREAD_MAX:
        out["reason"] = "the mines disagree by %.2f ticks (limit %g)" % (values[-1] - values[0], SPREAD_MAX)
        return out
    out["ticks"] = sum(values) / len(values)
    out["mines"] = len(kept)
    out["rms_px"] = (sum(p["residual_px"] ** 2 for p in kept) / len(kept)) ** 0.5
    return out


#: The static scene's fit: ours is rendered at these camera lags (ticks behind the push buffer), first at the whole
#: values, then in `STATIC_FINE` steps around the best.
STATIC_COARSE = range(0, 7)
STATIC_FINE = 0.25
#: The best score must beat every score `STATIC_AWAY` ticks or more from it by `STATIC_SHARP`, else the peak says nothing.
STATIC_AWAY, STATIC_SHARP = 1.5, 0.05
#: What the score leaves out of a 1920x1080 frame: the HUD, the hull and its plumes, and everything above `STATIC_TOP`
#: (scenery ours and the original draw differently whatever the camera does). Mines and rounds go by their colour.
STATIC_BOXES = [(680, 20, 1240, 240), (85, 60, 320, 225), (430, 75, 560, 115), (150, 850, 670, 980), (1250, 850, 1760, 980),
                (800, 600, 1120, 1000)]
STATIC_TOP, STATIC_BLOB_REACH = 380, 30


def pick_static(curve):
    """`(ticks, reason)` of a `{lag: score}` curve: the best lag, or `None` with why its peak does not stand."""
    best = max(curve, key=curve.get)
    if best <= min(curve) or best >= max(curve):
        return None, "the static scene's best lag (%.2f ticks) is at the end of the range swept" % best
    away = [score for k, score in curve.items() if abs(k - best) >= STATIC_AWAY]
    if not away or curve[best] - max(away) < STATIC_SHARP:
        return None, "the static scene's peak is flat: %.3f at %.2f ticks, %.3f elsewhere" % (
            curve[best], best, max(away) if away else float("nan"))
    return best, None


def static_mask(theirs_png):
    """The pixels the static-scene score reads: the frame less `STATIC_BOXES`, the top band and the bright blobs' surroundings."""
    import numpy as np
    blobs = blob_mask(theirs_png)
    height, width = blobs.shape
    r = STATIC_BLOB_REACH
    total = np.zeros((height + 1, width + 1), np.int64)
    total[1:, 1:] = blobs.cumsum(0).cumsum(1)
    ys, xs = np.arange(height), np.arange(width)
    y0, y1 = np.clip(ys - r, 0, height), np.clip(ys + r + 1, 0, height)
    x0, x1 = np.clip(xs - r, 0, width), np.clip(xs + r + 1, 0, width)
    near = (total[y1][:, x1] - total[y0][:, x1] - total[y1][:, x0] + total[y0][:, x0]) > 0
    mask = ~near
    mask[:STATIC_TOP] = False
    for bx0, by0, bx1, by1 in STATIC_BOXES:
        mask[by0:by1, bx0:bx1] = False
    return mask


def edges(path):
    """Gradient magnitude of a picture's grey, lightly blurred."""
    import numpy as np
    from PIL import Image, ImageFilter
    grey = Image.open(path).convert("L").filter(ImageFilter.GaussianBlur(1.5))
    gy, gx = np.gradient(np.asarray(grey, float))
    return np.hypot(gx, gy)


def static_fit(theirs_png, render):
    """The picture's lag off the static scene: `render(k)` is ours' frame (a PNG path) with camera and craft `k` ticks
    behind the push buffer's, and the lag is the `k` whose edges line up best with the original's (`pick_static`).
    Returns `{"ticks", "reason", "curve"}`; the camera moves only along its own read velocity, so this is one number."""
    import numpy as np
    mask = static_mask(theirs_png)
    theirs = edges(theirs_png)[mask]
    curve = {}

    def score(k):
        if k not in curve:
            curve[k] = float(np.corrcoef(theirs, edges(render(k))[mask])[0, 1])

    for k in STATIC_COARSE:
        score(float(k))
    best = max(curve, key=curve.get)
    for step in (-3, -2, -1, 1, 2, 3):
        if STATIC_COARSE[0] <= best + step * STATIC_FINE <= STATIC_COARSE[-1]:
            score(best + step * STATIC_FINE)
    ticks, reason = pick_static(curve)
    return {"ticks": ticks, "reason": reason, "curve": sorted(curve.items())}


def self_test():
    """Synthetic grab: a camera looking down +x with a velocity, three mines drawn as bright discs where a camera 3.5 ticks back sees them."""
    import tempfile
    import numpy as np
    from PIL import Image
    width, height = SIZE
    eye = np.array([0.0, 0.0, 0.0])
    velocity = np.array([1.7, 0.05, 0.0])
    skew = 2
    f, aspect = 1.0 / np.tan(np.radians(34.0)), width / height
    # row-vector convention: p = [x y z 1] @ VP, camera looks along +x, up +y, right +z
    view = np.array([[0, 0, 1, 0], [0, 1, 0, 0], [1, 0, 0, 0], [0, 0, 0, 1]], float)
    proj = np.array([[f / aspect, 0, 0, 0], [0, f, 0, 0], [0, 0, 1, 1], [0, 0, -0.1, 0]], float)
    view_proj = view @ proj
    k_true = 3.5
    mines = [[20.0, 2.0, 4.0], [28.0, -1.0, -5.0], [36.0, 1.5, 0.5]]
    image = np.zeros((height, width, 3), np.uint8)
    for m in mines:
        p = np.append(np.array(m) + k_true * velocity, 1.0) @ view_proj
        x, y = (p[0] / p[3] * 0.5 + 0.5) * width, (0.5 - p[1] / p[3] * 0.5) * height
        yy, xx = np.ogrid[:height, :width]
        image[(xx - x) ** 2 + (yy - y) ** 2 < 14 ** 2] = (230, 200, 255)
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "theirs.png"
        Image.fromarray(image).save(path)
        camera = {"eye": eye.tolist(), "forward": [1, 0, 0], "view_proj": view_proj.flatten().tolist()}
        ship = {"mines": [{"live": 1, "position": m} for m in mines], "rig_eye": (eye + skew * velocity).tolist()}
        got = fit(camera, ship, path, skew)
        assert got["ticks"] is not None and abs(got["ticks"] - k_true) < 0.1, got
        assert got["rms_px"] < 1.5, got
        ship["rig_eye"] = None
        assert fit(camera, ship, path, skew)["ticks"] is None
        ship = {"mines": [{"live": 1, "position": mines[0]}], "rig_eye": (eye + skew * velocity).tolist()}
        assert "2 needed" in fit(camera, ship, path, skew)["reason"]
    print("hd_picture_lag self test ok")


if __name__ == "__main__":
    if len(sys.argv) == 1:
        self_test()
    else:
        record = json.loads(Path(sys.argv[1]).read_text())
        ship = record["ship"]
        hull = ship["hull_pb"]
        skew = round(sum((a - b) * v / 60.0 for a, b, v in zip(ship["pos"], hull["translation"], ship["vel"])) /
                     sum((v / 60.0) ** 2 for v in ship["vel"]))
        print(json.dumps(fit(record["camera"], ship, Path(sys.argv[1]).with_suffix(".png"), skew), indent=1))
