"""Log every texture-transform block's clock and output on a running Pulse PSP, per update.

    uv run --with websocket-client python3 scripts/psp-texanim-trace.py PORT OUT.csv HITS ["a0 == 0x930d480 || ..."]

Breaks `TexAnim_UpdateTransform` (`0x08927204`, `a0` = block, `f12` = seconds; USA disc) and writes one row per hit: the
cycle counter, the block address, the clock, the offset/scale key counts, the key times and values themselves (read through
the block's relocated pointers), the loop float and raw word (`+0x2c`, bit 0 is the step flag) and the scale and offset the
PREVIOUS update left at `+0x18..+0x24`. The next hit's "previous" is therefore this hit's output, which is what is checked
against `oag_vex::TexTransform::sample` (`docs/ghidra/functions/psp-pulse-usa/texture-animation.md`). The optional fourth
argument is a breakpoint condition that keeps a few blocks (a hit costs about 55 ms with the CPU stopped; 3,600 hits take
three minutes). The CPU must be running on entry and is left stopped; resume it before loading a save state.
"""
import csv, struct, sys, time
sys.path.insert(0, "/home/topaxi/projects/OpenAntiGrav/scripts")
from ppsspp_debugger import Debugger
UPDATE = 0x08927204
port, out, hits = int(sys.argv[1]), sys.argv[2], int(sys.argv[3])
cond = sys.argv[4] if len(sys.argv) > 4 else None
dbg = Debugger(port)
def regs():
    r = dbg.call("cpu.getAllRegs")
    gpr = next(c for c in r["categories"] if c["name"] == "GPR")
    fpu = next(c for c in r["categories"] if c["name"] == "FPU")
    g = dict(zip(gpr["registerNames"], gpr["uintValues"]))
    f = {n: struct.unpack("<f", struct.pack("<I", v))[0] for n, v in zip(fpu["registerNames"], fpu["uintValues"])}
    return g, f
if not dbg.is_stepping():
    dbg.brk()
w = csv.writer(open(out, "w", newline=""))
w.writerow("hit cycles block t off_n sc_n off_times off_values sc_times sc_values loop loop_word prev_scale prev_offset".split())
for i, _ in dbg.each_hit(UPDATE, hits, timeout=30.0, condition=cond):
    g, f = regs()
    blk = g["a0"]; t = f["f12"]
    hdr = dbg.read(blk, 0x30)
    off_n, sc_n = struct.unpack_from("<HH", hdr, 0)
    p_ot, p_st, seconds_per_key, p_ov, p_sv = struct.unpack_from("<IIfII", hdr, 4)
    prev = struct.unpack_from("<4f", hdr, 0x18)
    loop_word = struct.unpack_from("<I", hdr, 0x2c)[0]; loop = struct.unpack_from("<f", hdr, 0x2c)[0]
    def keys(n, pt, pv):
        if not (0 < n < 64): return "", ""
        ts = struct.unpack("<%dH" % n, dbg.read(pt, 2 * n)); vs = struct.unpack("<%dh" % (2 * n), dbg.read(pv, 4 * n))
        return ";".join(map(str, ts)), ";".join("%d:%d" % (vs[2 * k], vs[2 * k + 1]) for k in range(n))
    ot, ov = keys(off_n, p_ot, p_ov); st, sv = keys(sc_n, p_st, p_sv)
    cyc = dbg.call("cpu.status").get("ticks")
    w.writerow([i, cyc, hex(blk), "%.6f" % t, off_n, sc_n, ot, ov, st, sv, "%.6f" % loop, hex(loop_word), "%.4f,%.4f" % prev[:2], "%.4f,%.4f" % prev[2:]])
print("done", hits)
