#!/usr/bin/env python3
"""Which tick is the frame an RPCS3 pause shows? RAM against the push buffer, in one pause.

    OAG_RPCS3_ATTACH=1 uv run --with evdev --with numpy python3 scripts/hd-frame-skew.py probe OUT_DIR STEM [--resume-ms N]
    uv run --with numpy python3 scripts/hd-frame-skew.py read OUT_DIR STEM

`probe` pauses the attached HD race (optionally after running it N ms), and reads in that one pause: every
craft's body position and velocity, the player's ship and class entry, and the three push-buffer regions
`rpcs3-drive.py capture` reads. It leaves the target paused. `read` then reports, in ticks along each craft's
own velocity, how far the push buffer's hull world matrices (scale 0.75, a plain row-layout world matrix) sit
from the RAM bodies, and how far the push-buffer camera's eye sits from the craft's rig eye (`ship+0x69a0`).
Both read 1 or 2 whole ticks on every pause measured: RAM is ahead of the frame, for the craft and the camera
alike (`docs/tools/compare-page.md`, "A moving HD craft").
"""
import importlib.util, json, struct, sys, time
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import ps3_pose  # noqa: E402


def probe(out, stem, resume_ms):
    spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
    drive = importlib.util.module_from_spec(spec); spec.loader.exec_module(drive)
    import rpcs3_place as place
    from rpcs3_debugger import Debugger
    out.mkdir(parents=True, exist_ok=True)
    GUEST_BASE = 0x300000000


    def triples_near(blob, base, target, radius):
        arr = np.frombuffer(blob[: len(blob) // 4 * 4], dtype=">f4").astype(np.float64)
        hits = []
        if len(arr) < 3:
            return hits
        ok = (np.abs(arr[:-2] - target[0]) < radius) & (np.abs(arr[1:-1] - target[1]) < radius) & (np.abs(arr[2:] - target[2]) < radius)
        for i in np.nonzero(ok)[0]:
            hits.append((base + int(i) * 4, [float(arr[i]), float(arr[i + 1]), float(arr[i + 2])]))
        return hits


    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    rec = {}
    with Debugger(port=port, timeout=90.0) as gdb:
        if resume_ms:
            gdb.resume(); time.sleep(resume_ms / 1000.0)
        gdb.pause()
        ship, body = place.find_player(gdb)
        pose = place.read_pose(gdb, body)
        rec["ship"], rec["body"] = ship, body
        rec["pos"], rec["vel"], rec["rows"] = list(pose["pos"]), list(pose["vel"]), [list(r) for r in pose["rows"]]
        field = []
        for other in struct.unpack(">8I", gdb.read(place.CRAFT_ARRAY, 32)):
            if not other:
                continue
            other_body = place.u32(gdb.read(other + place.OFF_BODY, 4))
            if not other_body:
                continue
            other_pose = place.read_pose(gdb, other_body)
            field.append({"ship": other, "player": other == ship, "pos": list(other_pose["pos"]), "vel": list(other_pose["vel"]),
                          "node": list(struct.unpack(">3f", gdb.read(other + 0x70, 12))),
                          "rig": [list(struct.unpack(">3f", gdb.read(other + off, 12))) for off in (0x69a0, 0x69b0, 0x69c0)]})
        rec["field"] = field
        entry = place.u32(gdb.read(ship + place.OFF_ENTRY, 4))
        rec["entry"] = entry
        ship_blob = b"".join(gdb.read(ship + at, 0x1000) for at in range(0, 0x8000, 0x1000))
        (out / (stem + "-ship.bin")).write_bytes(ship_blob)
        entry_blob = b"".join(gdb.read(entry + at, 0x1000) for at in range(0, 0x2000, 0x1000))
        (out / (stem + "-entry.bin")).write_bytes(entry_blob)
        body_blob = gdb.read(body, 0x400)
        (out / (stem + "-body.bin")).write_bytes(body_blob)
        pb = []
        for chain, size in drive.PUSHBUFFER_REGIONS:
            at = int(chain, 16)
            blob = b"".join(gdb.read(at + o, min(0x1000, size - o)) for o in range(0, size, 0x1000))
            (out / ("%s-%08x.bin" % (stem, at))).write_bytes(blob)
            pb.append((at, blob))
        try:
            mem = drive.RemoteMem(drive.read_session()["socket"])
            ram = []
            pos = np.array(rec["pos"])
            mem.seek(GUEST_BASE + body + 0x200)
            rec["pos_after_reads"] = list(struct.unpack(">3f", mem.read(12)))
            # the camera eye is unknown until the packets are read; keep the heap around the craft
            for lo, hi in ((0x00860000, 0x00a00000), ((ship & ~0xfffff) - 0x400000, (ship & ~0xfffff) + 0x800000)):
                at = lo
                while at < hi:
                    mem.seek(GUEST_BASE + at)
                    try:
                        chunk = mem.read(0x100000)
                    except Exception as error:  # noqa: BLE001
                        chunk = b""
                    if chunk:
                        ram.append((at, chunk))
                    at += 0x100000
            rec["ram_bytes"] = sum(len(c) for _, c in ram)
        except Exception as error:  # noqa: BLE001
            rec["ram_error"] = repr(error); ram = []
        try:
            drive.screenshot(str(out / (stem + ".png")), trim=True)
        except Exception as error:  # noqa: BLE001
            rec["shot_error"] = repr(error)
        # stays paused

    pos = np.array(rec["pos"]); vel = np.array(rec["vel"]); tick = vel / 60.0
    cands = []
    for at, blob in pb:
        cands.extend(ps3_pose.packet_candidates(blob, base=at))
    rec["packet_candidates"] = len(cands)
    picks = ps3_pose.pick_camera([cands])
    camera, reason, count = picks[0]
    rec["camera_reason"] = reason
    if camera:
        rec["camera"] = {k: camera[k] for k in ("address", "order", "eye", "view_proj", "registers") if k in camera}
        eye = np.array(camera["eye"])
        rec["eye_minus_pos"] = list(eye - pos); rec["eye_body_distance"] = float(np.linalg.norm(eye - pos))
        # RAM copies of the eye, exact and within 8 units
        near = []
        for at, chunk in ram:
            for addr, v in triples_near(chunk, at, eye, 8.0):
                d = np.array(v) - eye
                near.append({"addr": "%#x" % addr, "value": v, "delta": list(d), "ticks_along_v": float(d @ tick / (tick @ tick)), "off_axis": float(np.linalg.norm(d - (d @ tick / (tick @ tick)) * tick))})
        rec["ram_eye_copies"] = near
    # position copies on the ship, entry and body
    copies = []
    for name, base, blob in (("ship", ship, ship_blob), ("entry", entry, entry_blob), ("body", body, body_blob)):
        for addr, v in triples_near(blob, base, pos, 12.0):
            d = np.array(v) - pos
            copies.append({"in": name, "off": "%#x" % (addr - base), "value": v, "ticks_along_v": float(d @ tick / (tick @ tick)), "off_axis": float(np.linalg.norm(d - (d @ tick / (tick @ tick)) * tick))})
    rec["pos_copies"] = copies
    (out / (stem + ".json")).write_text(json.dumps(rec, indent=1))
    print("pos", rec["pos"], "vel", rec["vel"], "speed", float(np.linalg.norm(vel)))
    print("camera", rec.get("camera", {}).get("eye"), reason, "eye-body", rec.get("eye_body_distance"))
    print("ram bytes", rec.get("ram_bytes"), rec.get("ram_error"), rec.get("shot_error"))


def read(d, stem):
    rec = json.loads((d / (stem + ".json")).read_text())
    pos = np.array(rec["pos"]); vel = np.array(rec["vel"]); tick = vel / 60.0
    moved = np.linalg.norm(np.array(rec.get("pos_after_reads", rec["pos"])) - pos)
    def along(v):
        t = float(v @ tick / (tick @ tick)); return t, float(np.linalg.norm(v - t * tick))
    print("== %s pos %s speed %.1f u/s, tick step %.3f units, body moved during reads %.4f" % (stem, np.round(pos, 3), np.linalg.norm(vel), np.linalg.norm(tick), moved))
    regions = [(int(f.name.split("-")[-1][:8], 16), f.read_bytes()) for f in sorted(d.glob(stem + "-400*.bin"))]
    cams = {}
    for base, blob in regions:
        for addr, order, err, eye, m, reg in ps3_pose.packet_candidates(blob, base=base):
            if reg == 256 and order == "row":
                cams.setdefault(tuple(np.round(m, 4)), []).append(addr)
    print("distinct reg-256 cameras:", len(cams))
    cam_list = []
    for m, addrs in sorted(cams.items(), key=lambda kv: -len(kv[1])):
        eye = np.array(ps3_pose.eye_of(list(m)))
        t, off = along(eye - pos)
        cam_list.append((np.array(m).reshape(4, 4), eye, len(addrs)))
        print("   x%-3d first %#x eye %s |eye-pos| %.3f" % (len(addrs), addrs[0], np.round(eye, 3), np.linalg.norm(eye - pos)))
    VP, eye, _ = cam_list[0]
    inv = np.linalg.inv(VP)
    found = {}
    for base, blob in regions:
        arr = np.frombuffer(blob[: len(blob) // 4 * 4], dtype=">f4").astype(np.float64)
        win = np.lib.stride_tricks.sliding_window_view(arr, 16).reshape(-1, 4, 4)
        ok = np.all(np.isfinite(win), axis=(1, 2)) & (np.abs(win).max(axis=(1, 2)) < 1e5) & (np.abs(win).max(axis=(1, 2)) > 1e-3)
        idx = np.nonzero(ok)[0]
        if not len(idx):
            continue
        for name, W in (("wvp", win[idx] @ inv), ("world", win[idx]), ("worldT", np.transpose(win[idx], (0, 2, 1)))):
            aff = (np.abs(W[:, 3, 3] - 1) < 1e-2) & (np.abs(W[:, :3, 3]).max(axis=1) < 1e-2)
            for i, w in zip(idx[aff], W[aff]):
                T = w[3, :3]; sc = np.linalg.norm(w[0, :3])
                if 0.3 < sc < 2.5 and np.linalg.norm(T - pos) < 30:
                    found.setdefault((name, tuple(np.round(T, 2))), []).append((base + int(i) * 4, sc))
        # a bare position: any three floats within 6 units of the body, with what follows
        near = (np.abs(arr[:-2] - pos[0]) < 6) & (np.abs(arr[1:-1] - pos[1]) < 6) & (np.abs(arr[2:] - pos[2]) < 6)
        for i in np.nonzero(near)[0]:
            found.setdefault(("xyz", tuple(np.round(arr[i:i + 3], 2))), []).append((base + int(i) * 4, 0.0))
    print("world translations within 30 units of the RAM body (row layout), as WVP.inv(VP) or as a plain world matrix:")
    for (name, key), lst in sorted(found.items(), key=lambda kv: np.linalg.norm(np.array(kv[0][1]) - pos)):
        t, off = along(np.array(key) - pos)
        print("   %-5s T %s x%-3d first %#x scale %.3f  along v %+.3f ticks, off-axis %.3f" % (name, key, len(lst), lst[0][0], lst[0][1], t, off))
    if rec.get("field"):
        print("every craft against the push buffer's 0.75-scale world matrices (nearest translation):")
        worlds = [(np.array(key), len(lst)) for (name, key), lst in found.items() if name == "world"]
        allw = {}
        for base, blob in regions:
            arr = np.frombuffer(blob[: len(blob) // 4 * 4], dtype=">f4").astype(np.float64)
            win = np.lib.stride_tricks.sliding_window_view(arr, 16).reshape(-1, 4, 4)
            ok = np.all(np.isfinite(win), axis=(1, 2)) & (np.abs(win).max(axis=(1, 2)) < 1e5)
            aff = ok & (np.abs(win[:, 3, 3] - 1) < 1e-2) & (np.abs(win[:, :3, 3]).max(axis=1) < 1e-2)
            for w in win[aff]:
                sc = np.linalg.norm(w[0, :3])
                if 0.74 < sc < 0.76:
                    allw[tuple(np.round(w[3, :3], 3))] = allw.get(tuple(np.round(w[3, :3], 3)), 0) + 1
        for craft in rec["field"]:
            p = np.array(craft["pos"]); v = np.array(craft["vel"]) / 60.0
            best = None
            for key, n in allw.items():
                if n < 6:
                    continue
                dd = np.array(key) - p
                if best is None or np.linalg.norm(dd) < np.linalg.norm(best[0]):
                    best = (dd, n, key)
            if best is None:
                print("   %#x: no hull matrix" % craft["ship"]); continue
            dd, n, key = best
            t = float(dd @ v / (v @ v)) if v @ v > 1e-6 else 0.0
            print("   %s%#x speed %.1f: PB hull x%d minus RAM body: %+.3f ticks along its velocity, off-axis %.3f (|d| %.3f); node == body: %s"
                  % ("player " if craft["player"] else "", craft["ship"], np.linalg.norm(v) * 60, n, t, np.linalg.norm(dd - t * v), np.linalg.norm(dd), np.allclose(craft["node"], craft["pos"], atol=1e-3)))
    ship = (d / (stem + "-ship.bin")).read_bytes()
    def v3(off): return np.array(struct.unpack_from(">3f", ship, off))
    print("RAM rig points on the ship against the push-buffer eye:")
    for off in (0x69a0, 0x69b0, 0x69c0, 0x6980, 0x6990):
        p = v3(off); t, o = along(p - eye)
        print("   ship+%#x %s  minus PB eye: along v %+.3f ticks, off-axis %.3f; |p - RAM pos| %.3f" % (off, np.round(p, 3), t, o, np.linalg.norm(p - pos)))
    print("node matrix ship+0x40 translation candidates:", np.round(v3(0x70), 3), np.round(v3(0xb0), 3))


def main():
    command, out, stem = sys.argv[1], Path(sys.argv[2]), sys.argv[3]
    if command == "probe":
        resume_ms = int(sys.argv[sys.argv.index("--resume-ms") + 1]) if "--resume-ms" in sys.argv else 0
        probe(out, stem, resume_ms)
    elif command == "read":
        read(out, stem)
    else:
        raise SystemExit(__doc__)


if __name__ == "__main__":
    main()
