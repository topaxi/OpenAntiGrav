#!/usr/bin/env python3
"""Arm a write watchpoint on every live craft's pending-impulse field.

`entity->0x4c + 0x110` is `Ship_ApplyCollisionImpulse`'s (`0x0883f274`) pending
impulse vector - the stun/knockback slot two writers post to and it consumes
and zeroes each tick. See
`docs/ghidra/functions/psp-pulse-usa/contact-response.md#two-writers-found-at-a-live-write-breakpoint-2026-08-19`
for what this rig found the first time: `Weapon_PostBlastImpulse_q`
(`0x0886794c`) and a second, unnamed writer at `0x08868ea4`.

Reads all eight craft entities live off a `Ship_ApplyCollisionImpulse`
breakpoint hit (never guessed or hardcoded, since a restart moves every
craft), computes each one's pending-impulse address, and arms an
`enabled: false, log: true` write watchpoint on it - the non-halting,
full-speed shape `docs/reverse-engineering/ppsspp-debugger.md`'s watchpoint
section documents. It also arms a low-volume positive control (the first
craft's own rigid-body position, written every physics step) so a run that
logs nothing at the target is legibly "nothing happened" rather than "the
instrument is dead" - **always check the control's hit count before trusting
a target's zero**.

Requires a PPSSPP already running with the debugger enabled and a race
already loaded (`psp-drive.py menu --single-race` gets you there). After
running this, drive the race (input, or another `psp-drive.py` session) to
cause a contact, then read hits with `memory.breakpoint.list` and PCs from the
emulator's own stdout log (`CHK Write128(CPU) at ..., PC=...`) - this script
only arms the watchpoints, it does not wait or read results back, since a
useful drive needs real time to pass.

    uv run --with websocket-client scripts/psp-watch-pending-impulse.py --port 47810
"""

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger

APPLY = 0x0883F274  # Ship_ApplyCollisionImpulse
BODY_POINTER = 0x1CC
ENTITY_OWNER = 0x94  # *(entity + 0x94) == the Ship_UpdateCraft craft that owns it

# Correction, 2026-08-27: this used to be `ENTITY_TO_CRAFT = 0xFC0`, added
# straight to `entities[0]` below to get its craft. That does not hold -
# checked live against a known player craft address: 7 of
# 8 live Ship_ApplyCollisionImpulse a0 values produced an out-of-RAM-range
# "entity" under `a0 + 0xFC0`, and the one that happened to resolve was not
# the player. `Ship_ApplyCollisionImpulse`'s own a0 **is** the entity
# directly - the same "ship entity" reached elsewhere via `craft+0x1c4`
# (shield.md/camera.md/psp_trace_fields.py's ENTITY_POINTER), confirmed both
# directions against the known player craft and against camera.md's live fov
# law (`entity+0x790 == 0.075*dot(fwd,vel) + entity+0x7c`, matched to float
# precision only when `entity` is resolved this way). So `craft` is reached
# by *dereferencing* `entity + 0x94` (the reciprocal ENTITY_OWNER pointer
# shield.md already documents), not by adding a constant to `entity`'s own
# address - the two objects are independently heap-allocated, not two
# offsets into one block.


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=47800)
    parser.add_argument(
        "--hits",
        type=int,
        default=24,
        help="breakpoint hits to sample while collecting craft entities "
        "(3+ per craft; a full grid needs at least 8 distinct addresses)",
    )
    args = parser.parse_args()

    dbg = Debugger(args.port)
    dbg.brk()
    dbg.add_breakpoint(APPLY)

    entities = []
    seen = set()
    for _ in range(args.hits):
        dbg.call("cpu.resume")
        dbg.wait_for_break(APPLY, timeout=15.0)
        registers = dbg.call("cpu.getAllRegs")
        gpr = next(c for c in registers["categories"] if c["name"] == "GPR")
        a0 = dict(zip(gpr["registerNames"], gpr["uintValues"]))["a0"]
        if a0 not in seen:
            seen.add(a0)
            entities.append(a0)

    dbg.brk()
    dbg.remove_breakpoint(APPLY)

    print("distinct entities at Ship_ApplyCollisionImpulse: %d" % len(entities))
    targets = []
    for e in entities:
        ptr = dbg.read_u32(e + 0x4C)
        print("  entity 0x%08x -> *(+0x4c)=0x%08x" % (e, ptr))
        if 0x08000000 <= ptr < 0x0A000000:
            targets.append((e, ptr))

    for e, ptr in targets:
        addr = ptr + 0x110
        dbg.call(
            "memory.breakpoint.add",
            address=addr,
            size=16,
            enabled=False,
            log=True,
            read=False,
            write=True,
            change=False,
        )
        print("armed write watch 0x%08x (entity 0x%08x)" % (addr, e))

    craft0 = dbg.read_u32(entities[0] + ENTITY_OWNER)
    control_addr = dbg.read_u32(craft0 + BODY_POINTER)
    dbg.call(
        "memory.breakpoint.add",
        address=control_addr,
        size=16,
        enabled=False,
        log=False,
        read=False,
        write=True,
        change=False,
    )
    print("armed control 0x%08x (rigid body of craft 0x%08x)" % (control_addr, craft0))

    dbg.resume()
    print("resumed; drive the race to cause a contact, then check hit counts")
    dbg.close()


if __name__ == "__main__":
    main()
