"""Live check that a barrel-roll payout and a fired Turbo both arm the exhaust flare and play TURBO.

    uv run --with websocket-client python3 scripts/psp-roll-turbo-flare.py --port P roll|turbo [log]

Load a Time Trial grid state stopped at Ship_UpdateCraft first (`psp-state.py load --stop-at`).

Only the most recently added execution breakpoint fires on v1.20.4, so each mode drives the
craft from one breakpoint and, on the tick the call is due, swaps it for the single one at
`ExhaustFlare_OnTurbo`'s player-branch sound call (0x089051d4: a0 = the craft, a1 = bank
handle, a2 = cue name, ra saved at 0(sp)). The flare is craft+0x78; its boost timer +0xb8.
`roll` scripts the flight as psp-roll-gate.py does (contact bit forced, real d-pad taps); this is
a trigger check, not a physics measurement. `turbo` sets bit 0x400 (the Turbo fire request, which Weapons_DispatchFire turns into Turbo_Fire 0x088614c4) in craft 0's fire word.
"""
import subprocess
import sys
import time
sys.path.insert(0, "scripts")
from ppsspp_debugger import Debugger

SIDESHIFT = 0x08846A54
DISPATCH = 0x08861814
SOUND_CALL = 0x089051D4
SHOTS = None  # set by --shots DIR: photograph the free-running window after the event
TAPS = {5: "right", 20: "left", 35: "right"}
GROUNDED = set(range(120, 126))


def regs(dbg):
    r = dbg.call("cpu.getAllRegs")
    gpr = next(c for c in r["categories"] if c["name"] == "GPR")
    return dict(zip(gpr["registerNames"], gpr["uintValues"]))


def report(dbg, out, why):
    r = regs(dbg)
    craft = r["a0"]
    flare = dbg.read_u32(craft + 0x78)
    out.write("%s: pc %#x craft %#x caller(ra@sp) %#x\n" % (why, SOUND_CALL, craft, dbg.read_u32(r["sp"])))
    out.write("  flare %#x boost_timer +0xb8 = %.4f (len +0xb4 = %.4f)\n" % (
        flare, dbg.read_f32(flare + 0xB8), dbg.read_f32(flare + 0xB4)))
    out.write("  bank handle a1 = %#x, cue name a2 = %#x -> %r\n" % (r["a1"], r["a2"], dbg.read_cstring(r["a2"])))
    out.write("  craft+0x898 = %.4f, +0x860 = %#x, +0x368 (player kind) = %d\n" % (
        dbg.read_f32(craft + 0x898), dbg.read_u32(craft + 0x860), dbg.read_u32(craft + 0x368)))
    out.flush()


def shots(dbg, directory, prefix):
    """Free-run the emulator and photograph its window a few times while the flare burns (0.8 s)."""
    for name in (SIDESHIFT, DISPATCH, SOUND_CALL):
        try:
            dbg.remove_breakpoint(name)
        except Exception:
            pass
    dbg.call("cpu.resume")
    for index in range(5):
        time.sleep(0.12)
        subprocess.run(["magick", "import", "-window", "root", "%s/%s-%d.png" % (directory, prefix, index)],
                       env={"DISPLAY": "127.0.0.1:91", "PATH": "/usr/bin:/bin"}, check=False)


def swap_and_wait(dbg, old, why, out):
    dbg.remove_breakpoint(old)
    dbg.add_breakpoint(SOUND_CALL)
    dbg.call("cpu.status")
    dbg.pending = []
    dbg.call("cpu.resume")
    dbg.wait_for_break(SOUND_CALL, timeout=20)
    report(dbg, out, why)


def roll(dbg, out):
    dbg.brk()
    dbg.add_breakpoint(SIDESHIFT)
    tick, player = 0, None
    while tick < 300:
        dbg.call("cpu.status")
        dbg.pending = []
        dbg.call("cpu.resume")
        dbg.wait_for_break(SIDESHIFT, timeout=30)
        ent = regs(dbg)["a0"]
        if dbg.read_u32(ent + 0x78) == 0:
            continue
        player = player or ent
        if ent != player:
            continue
        craft = dbg.read_u32(ent + 0x94)
        flags = dbg.read_u32(craft + 0x1C0)
        flags = (flags | 1) if tick in GROUNDED else (flags & ~1)
        dbg.write_u32(craft + 0x1C0, flags)
        if tick in TAPS:
            dbg.send("input.buttons.press", button=TAPS[tick], duration=2)
        arm, phase = dbg.read_u32(ent + 0x860), dbg.read_f32(ent + 0x87C)
        flare = dbg.read_u32(ent + 0x78)
        out.write("tick %d contact %d arm860 %#x phase %.3f 898 %.3f flare+b8 %.3f\n" % (
            tick, flags & 1, arm & 0x380, phase, dbg.read_f32(ent + 0x898), dbg.read_f32(flare + 0xB8)))
        due = (flags & 1) and not arm & 0x200 and arm & 0x180 and abs(phase) > 0.5
        if due:
            out.write("payout due this tick\n")
            swap_and_wait(dbg, SIDESHIFT, "roll payout", out)
            after = dbg.read_f32(flare + 0xB8)
            out.write("  flare+0xb8 two instructions later: %.4f\n" % after)
            if SHOTS:
                shots(dbg, SHOTS, "roll")
            return
        tick += 1
    out.write("no payout within 300 ticks\n")


def turbo(dbg, out):
    dbg.brk()
    dbg.add_breakpoint(DISPATCH)
    dbg.call("cpu.status")
    dbg.pending = []
    dbg.call("cpu.resume")
    dbg.wait_for_break(DISPATCH, timeout=30)
    world = regs(dbg)["a0"]
    record = world + 0x70
    dbg.write_u32(record + 0x1BC, 0xFFFFFFFF)
    dbg.write_u32(record + 0x1B8, dbg.read_u32(record + 0x1B8) | 0x400)
    out.write("fire word %#x\n" % dbg.read_u32(record + 0x1B8))
    swap_and_wait(dbg, DISPATCH, "fired Turbo", out)
    if SHOTS:
        shots(dbg, SHOTS, "turbo")


if __name__ == "__main__":
    if "--shots" in sys.argv:
        SHOTS = sys.argv[sys.argv.index("--shots") + 1]
    port = int(sys.argv[sys.argv.index("--port") + 1])
    mode = sys.argv[sys.argv.index("--port") + 2]
    out = open(sys.argv[-1], "w") if sys.argv[-1].endswith(".log") else sys.stdout
    with Debugger(port) as dbg:
        {"roll": roll, "turbo": turbo}[mode](dbg, out)
        for a in (SIDESHIFT, DISPATCH, SOUND_CALL):
            try:
                dbg.remove_breakpoint(a)
            except Exception:
                pass
        dbg.resume()
