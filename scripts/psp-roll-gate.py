"""Live check of the barrel-roll arm gate in Ship_UpdateSideshiftInput_q (0x08846a54).

Break at the function's entry each tick for the player's entity (entity+0x78 != 0), force
craft+0x1c0 bit 0 (contact) to script a flight, press real d-pad taps, and log
+0x860 (arm bits), +0x87c (phase), +0x88c/890/894 (tap history), +0x898 (payout timer) and
settings+0x23c (arm counter) / +0x238 (payout counter).
"""
import sys
sys.path.insert(0, "scripts")  # run from the repo root
from ppsspp_debugger import Debugger

ENTRY = 0x08846A54
PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 47899
out = open(sys.argv[2], "w")

# tick -> d-pad button
TAPS = {}
for first_tick, spacing, seq in ((5, 15, "rlr"), (50, 10, "lrl"), (100, 10, "rlr"), (155, 15, "lrl")):
    for i, c in enumerate(seq):
        TAPS[first_tick + spacing * i] = {"r": "right", "l": "left"}[c]
GROUNDED = {140, 141}
END = 260

def regs(dbg):
    r = dbg.call("cpu.getAllRegs")
    gpr = next(c for c in r["categories"] if c["name"] == "GPR")
    return dict(zip(gpr["registerNames"], gpr["uintValues"]))

with Debugger(PORT) as dbg:
    dbg.brk()
    tick = 0
    player = None
    out.write("tick pad contact arm860 phase87c taps(88c,890,894) timer898 armcount23c payoutcount238\n")
    for _, msg in dbg.each_hit(ENTRY, 4000, timeout=30):
        ent = regs(dbg)["a0"]
        if dbg.read_u32(ent + 0x78) == 0:
            continue
        if player is None:
            player = ent
        if ent != player:
            continue
        craft = dbg.read_u32(ent + 0x94)
        flags = dbg.read_u32(craft + 0x1C0)
        flags = (flags | 1) if tick in GROUNDED else (flags & ~1)
        dbg.write_u32(craft + 0x1C0, flags)
        pad = TAPS.get(tick)
        if pad:
            dbg.send("input.buttons.press", button=pad, duration=2)
        settings = dbg.read_u32(0x08B31774)
        row = (
            tick, pad or "-", flags & 1,
            hex(dbg.read_u32(ent + 0x860) & 0x380),
            round(dbg.read_f32(ent + 0x87C), 3),
            (dbg.read_u32(ent + 0x88C), dbg.read_u32(ent + 0x890), dbg.read_u32(ent + 0x894)),
            round(dbg.read_f32(ent + 0x898), 3),
            dbg.read_u32(settings + 0x23C), dbg.read_u32(settings + 0x238),
        )
        out.write(" ".join(map(str, row)) + "\n"); out.flush()
        tick += 1
        if tick >= END:
            break
    dbg.resume()
