#!/usr/bin/env python3
"""Cheat a weapon into the player's hands in a live PPSSPP race, and photograph it.

Reading a weapon's effect out of the executable gets you the call graph; it does
not get you the picture. This puts the weapon in the air on the real thing, which
is the only way to check a static reading against what the game actually draws -
and it is how `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md` was
verified.

**The mechanism is one word.** `Weapons_DispatchFire` (`0x08861814`) walks every
craft once a frame and reads the fire-request word at `craft+0x1b8`, dispatching
one handler per set bit. Setting a bit is therefore exactly equivalent to the
player having the pickup and pressing fire, without needing the grant path (which
is still unread - see `docs/gameplay/pickups.md`). Most handlers clear their own
bit, so one write is one shot.

    uv run --with websocket-client scripts/psp-fire-weapon.py --port 47810 rocket

**Two traps, both of which cost a session:**

- **Writes only take while the CPU is stepping.** Writing `craft+0x1b8` against a
  free-running emulator appears to succeed and then does nothing: the word reads
  back with the bit still set long after a handler should have consumed it,
  because the write never reached the emulated RAM the dispatcher reads. So this
  breaks in `Ship_UpdateCraft` and writes inside the breakpoint, which is also
  the only place `ppsspp_debugger.write` claims writes are meaningful.
- **The rocket is not the missile.** They are different bits, different
  constructors and different effects, and a reading of one has been mistaken for
  the other in this repository before (`weapon-fire.md`'s History). `--list`
  prints what each bit here is believed to be, and at what confidence; anything
  below 70 is a hypothesis you are testing, not a label you can trust.

**And the craft the weapons code walks is not the craft `Ship_UpdateCraft`
takes.** `psp-trace.py` learns a craft from `Ship_UpdateCraft`'s `a0`, and
writing `+0x1b8` on *that* object does nothing at all - the write takes, the word
reads back set, and no handler ever consumes it, because it is a different
structure. `Weapons_DispatchFire` walks an **inline array in the world**:
`world + 0x70 + index * 0x1f0`, `_DAT_000577f8` entries. So this learns the world
from the dispatcher's own argument and indexes that array, which is why the
breakpoint here is `Weapons_DispatchFire` and not the ship update.

**The absorb is not a bit here: use `psp-absorb-frames.py`** (a held-pickup grant plus a held
circle, with EDRAM and GE-list readout).

**Never fire the LeachBeam (bit `0x8000`) through this script.** It halts
PPSSPP within a frame, deterministically: the instance lands in the weapon
pool `FUN_08866b08` walks with its matrix pointer `+0xa0` never filled in,
and the next update tick loads a quad from `null + 0x30`
(`E[MEMMAP] Bad memory access detected! 00000030 ... Stopping emulation`).
That is why `leachbeam` is deliberately absent from `WEAPONS` below. Granting the
held id (`10`) and pressing fire does not halt (2026-10-01, `ppsspp-debugger.md`) but
needs a lock to produce a beam, which a held-slot write did not cause. The
whole diagnosis, including the register read one instruction before the
fault, is in
`docs/ghidra/functions/psp-pulse-usa/bad-memory-access-halt.md`.

**The Missile is fine, and used to be blamed for that halt alongside the
LeachBeam.** It is not implicated: bit `0x40` was written, consumed and
produced no halt, in the same session as a rocket control and the LeachBeam
run that did halt (2026-09-05, two independent boots). The older claim here
that neither bit was "ever observed consumed" did not reproduce for either
weapon. What has *not* been confirmed for the Missile is the picture - that
a missile visibly flies - so treat `missile` below as a bit that dispatches,
not as a verified visual.

Subcommands are the weapon names in `WEAPONS`. `--probe-dispatch` skips firing
and just reports the arguments `Weapons_DispatchFire` is called with.
"""

import argparse
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger

# `Weapons_DispatchFire`, and the world pointer it is handed. The prototype is
# `(float dt, World *world)`; the float rides in `f12`, so the world arrives in
# `a0` rather than `a1` - confirmed live, `a1` is zero at the breakpoint.
WEAPONS_DISPATCH_FIRE = 0x08861814
WORLD_REGISTER = "a0"

# The inline per-craft weapon records the dispatcher walks, read straight off its
# own loop induction: `iVar9 = world + 0x70`, `iVar9 += 0x1f0` per craft.
CRAFT_ARRAY = 0x70
CRAFT_STRIDE = 0x1F0

# The fire-request word and the target slot beside it, from weapon-fire.md.
FIRE_WORD = 0x1B8
TARGET_WORD = 0x1BC

# One entry per bit that has a *read* handler. The confidence is the handler
# identification's, not the bit number's - the bit numbers are all read directly
# off `Weapons_DispatchFire`'s dispatch chain and are not in doubt.
WEAPONS = {
    "rocket": (0x0080, "Weapon_FireRocket 0x0886e104 - three at once, fanned", 88),
    "missile": (0x0040, "Weapon_FireMissile 0x088685cc - homing; bit dispatches, visual unconfirmed", 90),
    "burst": (0x0002, "Weapon_DropMines 0x088675cc - mines, per mine.md", 90),
    "backward": (0x0100, "FUN_08863a20 - fires backwards, reads as a Bomb or Mine", 60),
    "turbo": (0x0200, "timed pickup, the engine gate's half", 65),
    # plasma.md's full sixteen-bit dispatch table; 88 is `Weapon_FireQuake`'s own
    # confidence in names.tsv. One wave at a time: the handler returns early
    # while the previous Quake's road spans are still rippling.
    "quake": (0x0008, "Weapon_FireQuake 0x0886c600 - the travelling road ripple", 88),
}

# `leachbeam` (bit 0x8000, Weapon_FireLeachBeam 0x08866658) is deliberately not
# here: setting it halts PPSSPP within a frame. See the module docstring.


def gpr(dbg):
    """The general-purpose registers at a breakpoint, as a name -> value dict."""
    registers = dbg.call("cpu.getAllRegs")
    category = next(c for c in registers["categories"] if c["name"] == "GPR")
    return dict(zip(category["registerNames"], category["uintValues"]))


def screenshot(display, directory, name):
    """Grab the whole virtual screen with ImageMagick.

    `import -window root` rather than a compositor call, so this works under the
    Xvfb that `docs/reverse-engineering/ppsspp-debugger.md` prefers as well as on
    a real display.

    ImageMagick 7 dropped the standalone `import` binary in favour of the
    `magick import` subcommand, and the failure is a bare exit 1 with no
    message - so try the subcommand first and keep the old spelling as the
    fallback for a version 6 install.
    """
    path = Path(directory) / name
    path.parent.mkdir(parents=True, exist_ok=True)
    env = {"DISPLAY": display, "PATH": "/usr/bin:/bin"}
    for argv in (["magick", "import"], ["import"]):
        done = subprocess.run(
            [*argv, "-window", "root", str(path)], env=env, check=False
        )
        if done.returncode == 0:
            return path
    raise RuntimeError("no working ImageMagick import; tried `magick import` and `import`")


def probe_dispatch(dbg):
    """Report the arguments `Weapons_DispatchFire` is actually called with."""
    for _, _ in dbg.each_hit(WEAPONS_DISPATCH_FIRE, 1, timeout=40.0):
        registers = gpr(dbg)
        print(
            "Weapons_DispatchFire a0=0x%08x a1=0x%08x a2=0x%08x"
            % (registers["a0"], registers["a1"], registers["a2"])
        )
    dbg.resume()


def fire(dbg, bit, args):
    """Set `bit` in one craft's fire word, then photograph the result.

    The write happens *inside* the `Weapons_DispatchFire` breakpoint, so it lands
    in the same frame the dispatcher is about to read - no waiting a tick and
    hoping nothing else clears it.
    """
    if args.accelerate > 0.0:
        # A volley leaving a stationary craft on the start line has nowhere to go.
        dbg.hold(cross=True)
        time.sleep(args.accelerate)

    record = None
    for _, _ in dbg.each_hit(WEAPONS_DISPATCH_FIRE, 1, timeout=args.timeout):
        world = gpr(dbg)[WORLD_REGISTER]
        record = world + CRAFT_ARRAY + args.index * CRAFT_STRIDE
        before = dbg.read_u32(record + FIRE_WORD)
        # -1 is what `Weapon_FireRocket` itself writes: no lock, no target.
        dbg.write_u32(record + TARGET_WORD, 0xFFFFFFFF)
        dbg.write_u32(record + FIRE_WORD, before | bit)
        readback = dbg.read_u32(record + FIRE_WORD)
        print(
            "world 0x%08x  craft %d at 0x%08x  fire word 0x%08x -> 0x%08x"
            % (world, args.index, record, before, readback),
            file=sys.stderr,
        )
        if not readback & bit:
            print("the write did not take; nothing will fire", file=sys.stderr)
            raise SystemExit(1)
    dbg.resume()

    shots = []
    if args.freeze_at is not None:
        # A projectile crosses the screen in a handful of frames, and a
        # screenshot of a free-running emulator costs more than one - so catching
        # the thing *in flight* means stopping the CPU rather than photographing
        # faster. Breaking in the projectile's own update leaves the last
        # completed frame on screen, which is a frame that necessarily has the
        # projectile in it.
        for hit, _ in dbg.each_hit(args.freeze_at, args.freeze_hits, timeout=args.timeout):
            if hit + 1 >= args.freeze_hits:
                shots.append(
                    screenshot(args.display, args.shots, "%s-frozen.png" % args.prefix)
                )
        dbg.resume()
    else:
        for index in range(args.frames):
            shots.append(
                screenshot(args.display, args.shots, "%s-%02d.png" % (args.prefix, index))
            )
            time.sleep(args.interval)

    after = dbg.read_u32(record + FIRE_WORD)
    consumed = not after & bit
    print("fire word after: 0x%08x (%s)" % (after, "consumed" if consumed else "still set"))
    if not consumed:
        # Worth saying plainly rather than leaving in the number: a bit still set
        # means no handler ran, so the frames show nothing.
        print(
            "no handler consumed the bit - either this bit has no handler in this "
            "build, or craft %d is not racing" % args.index,
            file=sys.stderr,
        )
    print("wrote %d frames to %s" % (len(shots), args.shots))


def main():
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("weapon", nargs="?", choices=sorted(WEAPONS), help="which bit to set")
    parser.add_argument("--list", action="store_true", help="print the bit table and exit")
    parser.add_argument("--port", type=int, default=47800)
    parser.add_argument("--probe-dispatch", action="store_true")
    parser.add_argument(
        "--index",
        type=int,
        default=0,
        help="which world craft record fires. NOT the player in a single race: "
        "measured 2026-09-24, the player was record 7 of a full grid. The "
        "player's weapon record is *(*(*(0x08b317b4)+0x2c0)+0x4c)",
    )
    parser.add_argument(
        "--accelerate",
        type=float,
        default=6.0,
        help="hold accelerate this many seconds before firing (0 to fire from a stop)",
    )
    parser.add_argument("--frames", type=int, default=14)
    parser.add_argument("--interval", type=float, default=0.15)
    parser.add_argument(
        "--freeze-at",
        type=lambda s: int(s, 0),
        help="instead of a burst of frames, break here after firing and grab one "
        "frame with the CPU stopped - 0x0885d2a8 is Rocket_Update",
    )
    parser.add_argument("--freeze-hits", type=int, default=6)
    parser.add_argument("--display", default=":97", help="the X display to screenshot")
    parser.add_argument("--shots", default="/tmp", help="directory for the PNGs")
    parser.add_argument("--prefix", default="weapon")
    parser.add_argument("--timeout", type=float, default=40.0)
    args = parser.parse_args()

    if args.list:
        for name, (bit, what, confidence) in sorted(WEAPONS.items()):
            print("%-9s bit 0x%04x  %-58s confidence %d" % (name, bit, what, confidence))
        return

    dbg = Debugger(args.port)
    dbg.resume()
    try:
        if args.probe_dispatch:
            probe_dispatch(dbg)
        elif args.weapon is None:
            parser.error("a weapon is required unless --list or --probe-dispatch is given")
        else:
            fire(dbg, WEAPONS[args.weapon][0], args)
    finally:
        dbg.hold()
        dbg.close()


if __name__ == "__main__":
    main()
