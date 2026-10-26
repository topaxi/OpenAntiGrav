#!/usr/bin/env python3
"""Summarise a capture of `psp-autopilot-expiry.py`, `psp-pure-autopilot-expiry.py` or
`rpcs3-hd-autopilot-expiry.py`: one line per expiry and the totals a "does anything hold
the Autopilot" claim is judged on.

    python3 -I scripts/autopilot-expiry-summary.py {pulse,pure,hd} capture.json

Per expiry (the first frame or row the running flag reads clear):

- the timer at the last running frame and at the first clear one (a release on the timer
  reads `<= 0` there, and the one before it reads above 0);
- the heading rate in degrees a frame (Pulse, Pure) or a second (HD), as the mean and the
  largest over the six frames before, and the largest bank (the roll of the craft out of the
  horizontal, so a craft on a steep wall reads high) over the same span;
- the speed in units a frame (Pulse, Pure), from the position;
- where the controls went: on Pulse the frame the pad record was selected again, on HD the
  number of rows and milliseconds until `craft+0x84` pointed at the pad record.

The last lines count the failures a hold would show: running frames with the timer at or
below zero, and a timer that rose while running (a re-arm is written between two frames of a
stint and is not counted; rows that straddle a write are not counted).
"""

import json
import math
import sys

RUN = 0x800


def pulse(data):
    frames = data["frames"]
    out = []
    for i in range(1, len(frames)):
        a, b = frames[i - 1], frames[i]
        if a["flags"] & RUN and not b["flags"] & RUN:
            before = frames[max(0, i - 6):i]
            rates = [abs(r.get("yaw_rate", 0.0)) for r in before]
            speed = math.dist(frames[i - 1]["pos"], frames[i - 2]["pos"]) if i > 1 else 0.0
            nxt = frames[i + 1] if i + 1 < len(frames) else None
            out.append({
                "frame": b["frame"], "timer_last": a["timer"], "timer_off": b["timer"],
                "yaw_mean": sum(rates) / len(rates), "yaw_max": max(rates),
                "bank_max": max(abs(r["bank"]) for r in frames[max(0, i - 6):i + 1]),
                "speed": speed, "engine_off": b["engine"], "blend_off": b["blend"],
                "pad_next": bool(nxt and nxt["controls_is_pad"]),
            })
    bad = [r["frame"] for r in frames if r["flags"] & RUN and r["timer"] <= 0]
    rises = sum(1 for i in range(1, len(frames))
                if frames[i]["flags"] & RUN and frames[i - 1]["flags"] & RUN
                and frames[i]["timer"] > frames[i - 1]["timer"] + 1e-6)
    return out, bad, rises


def pure(data):
    frames = data["frames"]
    out = []
    for i in range(1, len(frames)):
        a, b = frames[i - 1], frames[i]
        if a["running"] and not b["running"]:
            before = frames[max(0, i - 6):i]
            rates = [abs(r.get("yaw_rate", 0.0)) for r in before]
            speed = math.dist(frames[i - 1]["pos"], frames[i - 2]["pos"]) if i > 1 else 0.0
            out.append({
                "frame": b["frame"], "timer_last": a["timer"], "timer_off": b["timer"],
                "yaw_mean": sum(rates) / len(rates), "yaw_max": max(rates),
                "bank_max": max(abs(r["bank"]) for r in frames[max(0, i - 6):i + 1]),
                "speed": speed,
            })
    bad = [r["frame"] for r in frames if r["running"] and r["timer"] <= 0]
    rises = sum(1 for i in range(1, len(frames))
                if frames[i]["running"] and frames[i - 1]["running"]
                and frames[i]["timer"] > frames[i - 1]["timer"] + 1e-6)
    return out, bad, rises


def hd(data):
    rows = data["rows"]
    out = []
    for i in range(1, len(rows)):
        if rows[i - 1]["running"] and not rows[i]["running"]:
            j = i
            while j < len(rows) and not rows[j]["running"]:
                j += 1
            window = rows[i:j]
            pad = next((k for k, r in enumerate(window) if not r["ctl_is_driver"]), None)
            before = rows[max(0, i - 8):i]
            out.append({
                "row": i, "timer_last": rows[i - 1]["timer"], "timer_off": rows[i]["timer"],
                "dps_max": max(abs(r.get("dps", 0.0)) for r in before),
                "bank_max": max(abs(r["bank"]) for r in before),
                "speed": rows[i]["speed"],
                "rows_to_pad": pad,
                "ms_to_pad": (window[pad]["t_wall"] - window[0]["t_wall"]) * 1000 if pad is not None else None,
                "engine_seq": [hex(e) for k, e in enumerate(r["engine"] for r in window)
                               if k == 0 or window[k - 1]["engine"] != e][:4],
            })
    bad = [k for k, r in enumerate(rows) if r["running"] and r["timer"] <= 0]
    rises = sum(1 for k in range(1, len(rows))
                if rows[k]["running"] and rows[k - 1]["running"]
                and rows[k]["timer"] > rows[k - 1]["timer"] + 1e-6)
    return out, bad, rises


def main():
    kind, path = sys.argv[1], sys.argv[2]
    data = json.load(open(path))
    out, bad, rises = {"pulse": pulse, "pure": pure, "hd": hd}[kind](data)
    for row in out:
        print(" ".join("%s=%s" % (k, ("%.4g" % v) if isinstance(v, float) else v) for k, v in row.items()))
    forced = [e for e in data["events"] if "forced_at_frame" in e or "forced_row" in e]
    print("expiries: %d, forced into a corner: %d" % (len(out), len(forced)))
    print("released with the timer <= 0 on the first clear frame: %d" % sum(1 for r in out if r["timer_off"] <= 0))
    print("released with the previous frame's timer above 0: %d" % sum(1 for r in out if r["timer_last"] > 0))
    print("running frames with the timer <= 0: %d %s" % (len(bad), bad[:6]))
    print("timer rises while running: %d" % rises)
    if kind == "hd":
        pads = [r["rows_to_pad"] for r in out if r["rows_to_pad"] is not None]
        print("rows until the pad record is selected: min %s max %s of %d" % (min(pads), max(pads), len(pads)))


if __name__ == "__main__":
    main()
