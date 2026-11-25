#!/usr/bin/env python3
"""HD's Zone end camera object read live: its orbit fields per frame from the blast through the slow phase.

    scripts/emu-restore-state.sh <lane> data/saves/hd-fury/hd-zone-circuit0-feisar-grid-orbit.SAVESTAT.zst   # GDB stub on
    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/rpcs3-hd-zone-orbit-camera.py --image ISO --out DIR

Starts the race, writes the player's shield to 0, pauses at the blast, resumes a few frames until the game has
entered mode 2, pauses again and scans guest memory for the camera object by its orbit rates (`+0x234` = 0.01,
`+0x240` = 0.02, a mode word `+0x40` = 2). Then it runs free and samples the object's block `+0x200..+0x3c0`
as fast as the socket allows, keeping one row per change of the clock `+0x2c4`. Each row also carries `rcam`, the render camera's 4x4 at `*0x987884 + 0x40` (row 3 is the eye the orbit view writes, `0x000ae6c0`). Writes `block0.hex` (the first
whole block) and `rows.json`. `docs/ghidra/functions/ps3-hdfury-eu/destroy-camera.md`, "Measured on RPCS3".
"""
import argparse, importlib.util, json, re, struct, sys, time
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(spec); sys.modules["rpcs3_drive"] = drive; spec.loader.exec_module(drive)
from rpcs3_debugger import Debugger  # noqa: E402
G = 0x300000000
FIELDS = {"fov": 0x22C, "fov_target": 0x230, "fov_rate": 0x234, "dist": 0x238, "dist_target": 0x23C, "dist_rate": 0x240,
          "pitch": 0x2A0, "yaw": 0x2A4, "field": 0x288, "clock": 0x2C4, "r3a0": 0x3A0, "r3a4": 0x3A4, "r3a8": 0x3A8}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--image", required=True); ap.add_argument("--out", required=True)
    ap.add_argument("--eye-probe", type=float, default=0.0, help="pause past this orbit clock and find the pushbuffer eye in guest memory")
    ap.add_argument("--secs", type=float, default=7.0, help="game seconds of orbit to log")
    args = ap.parse_args()
    out = Path(args.out); out.mkdir(parents=True, exist_ok=True)
    with drive.Session(args.image, str(out / "logs")) as session:
        mem = session.open_mem()
        def rd(a, n): mem.seek(G + a); return mem.read(n)
        u32 = lambda a: struct.unpack(">I", rd(a, 4))[0]
        f32 = lambda a: struct.unpack(">f", rd(a, 4))[0]
        player = next(s for s in (u32(0x98D7C0 + 4 * k) for k in range(8)) if s and u32(s + 0x7A60) == 0)
        body = u32(player + 0x6944)
        spd = lambda: sum(v * v for v in struct.unpack(">3f", rd(body + 0x190, 12))) ** 0.5
        t0 = time.time()
        while time.time() - t0 < 120 and spd() < 90:
            if spd() < 10: session.tap("cross", settle=0.0)
            time.sleep(1)
        print("speed %.1f" % spd(), flush=True)
        with Debugger(port=int(drive.GDB_SERVER.rsplit(":", 1)[1])) as gdb:
            gdb.resume()
            while True:
                mem.seek(G + player + 0x5FA0); mem.write(struct.pack(">f", 0.0))
                if struct.unpack(">i", rd(player + 0x5FA4, 4))[0] >= 4: break
                time.sleep(0.002)
            gdb.pause()
            cam = None
            for attempt in range(6):
                gdb.resume(); time.sleep(0.15 + 0.1 * attempt); gdb.pause()
                pat = struct.pack(">f", 0.02)
                found = []
                for base in range(0x30000000, 0x40000000, 0x1000000):
                    try: ch = rd(base, 0x1000000)
                    except Exception: continue
                    for m in re.finditer(re.escape(pat), ch):
                        a = base + m.start()
                        c = a - 0x240
                        if a % 4 == 0 and f32(c + 0x234) == struct.unpack(">f", struct.pack(">f", 0.01))[0] and u32(c + 0x40) == 2:
                            found.append(c)
                print("attempt %d: camera candidates %s" % (attempt, [hex(c) for c in found]), flush=True)
                if found: cam = found[0]; break
            if cam is None: sys.exit("no camera in mode 2 found")
            (out / "block0.hex").write_text("%#x\n" % cam + rd(cam + 0x200, 0x1C0).hex())
            gdb.resume()
            rows, last = [], None
            t1 = time.time()
            while time.time() - t1 < 40:
                full = rd(cam, 0x800); blk = full[0x200:0x3C0]
                row = {k: struct.unpack_from(">f", blk, o - 0x200)[0] for k, o in FIELDS.items()}
                row["mode"] = u32(cam + 0x40); row["state"] = struct.unpack(">i", rd(player + 0x5FA4, 4))[0]
                row["focus"] = struct.unpack_from(">3f", blk, 0x390 - 0x200); row["b3ac"] = blk[0x3AC - 0x200]
                row["craft"] = struct.unpack(">3f", rd(body + 0x200, 12))
                render = u32(0x987884)
                row["render"] = render; row["rcam"] = struct.unpack(">16f", rd(render + 0x40, 64)) if render else None; row["wall"] = time.time(); row["blk"] = blk.hex(); row["full"] = full.hex()
                if row["clock"] != last:
                    rows.append(row); last = row["clock"]
                if row["clock"] > args.secs or row["mode"] != 2: break
                if args.eye_probe and row["clock"] > args.eye_probe:
                    gdb.pause()
                    import ps3_pose
                    cands = []
                    for chain, size in drive.PUSHBUFFER_REGIONS:
                        at = drive.resolve_chain(gdb, chain)
                        cands.extend(ps3_pose.packet_candidates(gdb.read(at, size), base=at))
                    craft = struct.unpack(">3f", rd(body + 0x200, 12))
                    eyes = {c[3] for c in cands if sum((a - b) ** 2 for a, b in zip(c[3], craft)) ** 0.5 < 30 and c[2] < 1e-3}
                    print("clock %.3f craft %s eyes %s cam %#x" % (row["clock"], craft, eyes, cam), flush=True)
                    rm = struct.unpack(">16f", rd(row["render"] + 0x40, 64)); reye = tuple(-v for v in rm[12:15])
                    print("  render camera eye %s: %.2f from the craft, %.2f from the focus" % ([round(v, 2) for v in reye], sum((a - b) ** 2 for a, b in zip(reye, craft)) ** 0.5, sum((a - b) ** 2 for a, b in zip(reye, row["focus"])) ** 0.5), flush=True)
                    print("  body rows %s" % [struct.unpack(">3f", rd(body + o, 12)) for o in (0x1D0, 0x1E0, 0x1F0)], flush=True)
                    print("  cam %s" % {k: row[k] for k in ("yaw", "pitch", "dist", "fov", "clock")}, "focus", row["focus"], flush=True)
                    for eye in []:
                        pat = struct.pack(">3f", *eye)
                        for base in range(0, 0x40000000, 0x1000000):
                            try: ch = rd(base, 0x1000000)
                            except Exception: continue
                            for m in re.finditer(re.escape(pat), ch):
                                print("  eye %s at %#x (cam%+#x)" % ([round(v, 2) for v in eye], base + m.start(), base + m.start() - cam), flush=True)
                    gdb.resume()
                    break
        (out / "rows.json").write_text(json.dumps(rows))
    print("rows", len(rows))
main()
