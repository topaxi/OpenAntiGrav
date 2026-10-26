# Pure's Autopilot pickup (2026-10-26, `autopilot-re`)

Pure's pair to Pulse's `Autopilot_Fire` and `Autopilot_Update`
([psp-pulse-usa/autopilot.md](../psp-pulse-usa/autopilot.md)), read with
capstone off `pure-usa`'s `BOOT.BIN` (no Ghidra database is open for it). This
import has the same unrelocated `jal` and `lui` constants as Pulse's: add
`0x08804000`. Found from the `disengaging` string at `0x08a47dbc`
(`psp-relocate.py xrefs`, the `lui`/`addiu` pair at `0x0884c7dc`/`0x0884c7f0`),
which [dry-play-cues.md](dry-play-cues.md) had already traced to this function
without naming it.

**Static only. Nothing here is runtime-verified.**

| Address | Name | Confidence |
| --- | --- | --- |
| `0x0884c770` | `Autopilot_Fire` | 85 |
| `0x0884c794` | `Autopilot_Update` | 85 |

## The two functions

```c
// 0x0884c770, one call site (0x0884cf8c, the fire dispatch)
void Autopilot_Fire(World *w, Pickup *p)
{
    p->running   = 1;                 // sb 1, 0x154(a1)
    p->0x1d8     = 0;                 // sw zero, 0x1d8(a1)
    p->timer     = weapon_stats->0xa4;  // the table at 0x000538f0 + 0x08804000, swc1 0x15c
    p->requested = 0;                 // sb zero, 0x155(a1), in the return's delay slot
}

// 0x0884c794, one call site (0x0884d090), behind lbu 0x154 at 0x0884d080
void Autopilot_Update(float dt, World *w, Pickup *p)
{
    p->timer -= dt;                                        // 0x0884c79c, stored 0x0884c7bc
    if (p->timer < 1.0f && p->prev > 1.0f)                 // +0x158
        play_dry(p->entity, bank, "disengaging", 0x400);   // 0x0884c7ec
    if (p->timer <= 0.0f) {
        p->running = 0;                                    // 0x0884c810
        if (p->entity->0xdc) { stop(...); p->entity->0xdc = 0; }
    }
    p->prev = p->timer;                                    // 0x0884c834
}
```

Pulse's shape exactly, with Pure's layout: the running flag and the request
are bytes (`+0x154`, `+0x155`) where Pulse has bits `0x800` and `0x1000` of
one word, and the timer and its previous value are `+0x15c`/`+0x158` where
Pulse has `+0x148`/`+0x144`. Confidence 85 for both: every store is read, and
the threshold, the edge and the cue match Pulse's instruction for instruction.

## No cancel and no hold

- **Nothing outside these two writes the timer as a cancel.** `psp-relocate.py
  field 0x15c` finds 15 non-stack `sw`/`swc1` stores. Two are these
  (`0x0884c788`, `0x0884c7bc`). Every other was read with the seven
  instructions before it, and none stores zero to a pickup record: a call's
  return (`0x0882198c`, `0x089781d0`), two accumulators with their own limits
  (`0x0889e198..0x0889e1b4` against `5.0`, `0x08946f88`), two stores in the
  AI driver's code (`0x0891a350`, `0x0891a3a0`, after `Ai_Construct` at
  `0x08919024`), constructor stores in the driver and in other objects
  (`0x08919098`, `0x089466f8`, and `0x08978c80`, whose struct gets `8` at
  `+0x154` as a word), a VFPU block (`0x08888858`) and a field-by-field struct
  copy (`0x089793cc`).
- **The running byte is cleared only by the update and by a whole-record
  reset** (`0x0884e194`, among a run of clears from `0x0884e174`). Its readers
  are the dispatch (`0x0884d068`, `0x0884d080`) and two outside the weapon code
  (`0x08922344`, which is the `SubWeapon` timer's HUD read, and `0x08929b78`,
  next to the `~AUTOPILOT` open at `0x08929c40`).

So Pure has no absorb cancel of the kind Pulse and HD have: no function on
the absorb side reads the running byte (its only byte reads are the four
above, and the other `+0x154` hits are word accesses to other structures), and
nothing writes the timer to zero. Nothing holds the Autopilot through a corner
either. **Confidence 70** for the absence of a cancel: a cancel through another
base register is not excluded by a field scan, and nothing here was watched
running.

**Ported 2026-10-26 (`autopilot-wire`).** Pure's `Weapons::autopilot_absorb_cancels` is
`false`, `Origin::Measured` (static, 70): an absorb under its running Autopilot does
nothing, where Pulse's and HD's cancel it. The disc-backed trace is
`crates/raceplay/tests/autopilot_cancel_ground_truth.rs` (`pure_has_no_cancel`: the
timer runs on through the press). Pure's weapon pad grant, its fire path under a
running stint and any `disengaging` call outside the countdown's own are unread, so
ours runs Pulse's pad gate and fire suppression there (inherited). A PPSSPP run of
`scripts/psp-autopilot-cancel.py` adapted to this page's offsets (running byte `+0x154`,
timer `+0x15c`) would settle the absence.

## Expiry mid-corner, live (2026-10-26, `autopilot-expiry`)

The pasted claim (the timer is extended in a turn zone, a bank or at high g, and it
disengages only on a straight; its example was Chenghou Project) was run on Pure's own
Chenghou Project, which Pure does ship. **No hold, measured live, confidence 88** (one
boot, one circuit; the timer only, see the limits below). Pulse's, HD's and 2048's:
[Pulse](../psp-pulse-usa/autopilot.md#expiry-mid-corner-and-the-pasted-extended-in-a-turn-zone-claim-2026-10-26-autopilot-expiry),
[HD](../ps3-hdfury-eu/autopilot.md#expiry-mid-corner-2026-10-26-autopilot-expiry),
[2048](../vita-2048-eu-v104/autopilot.md).

`scripts/psp-pure-autopilot-expiry.py`, PPSSPP v1.20.4 on `pure-psp-usa.chd` (disc `UCUS98612`,
2.00), the Time Trial state `time-trial-chenghou-vector-feisar-restart`, Vector, Feisar. The
pickup record is found at the per-craft weapons loop's tail (`0x0884d0d4`, `a1`); the craft
is read once a frame at `Ship_UpdateCraft` (`0x0892d214`). The craft is flown on the throttle
for 40 frames, then armed as `Autopilot_Fire` does (`+0x154 = 1`, timer `+0x15c`). Pure runs
its simulation at 30 Hz: the timer fell `0.0333` a frame. Pure's Autopilot flies the lap
(the craft covered 1 to 4 units a frame for 600 frames). Policy `corner` with the trigger at
1.2 degrees of heading a frame (36 degrees a second), the stint 5 s, the forced timer 0.05 s.

| | |
| --- | ---: |
| Expiries, all forced into the hairpin | 20 |
| Heading rate at the trigger | 1.20 to 2.71 degrees a frame (36 to 81 a second) |
| Mean over the six frames before the expiry | 0.8 to 2.2 degrees a frame |
| Released on the first frame with the timer `<= 0` (the frame before it above 0) | 20 of 20 |
| Frames with the byte set and the timer `<= 0` | **0** |
| Timer rises while the byte is set | **0** |

So the static reading above holds: `Autopilot_Update` clears the byte the first time the timer
reads `<= 0`, in the middle of Pure's tightest corner as everywhere. The "no hold" lines of
this page's "No cancel and no hold" section are now measured; their confidence 70 stays only
for the **absence of a cancel**.

**Limits.** Only the timer and the byte were read: Pure's controls' blend and engine bits are
not located in this tree's offsets, so the hand-back's tick count and the `disengaging`
timing are Pulse's and HD's, not watched on Pure. A face-button probe (CIRCLE, TRIANGLE, then
SQUARE held 10 frames against a 30 s stint) left the byte set and the timer falling, but
the race's weapons switch in Pure is not found, so the buttons may never have reached a
pickup path and the probe has **no positive control**: the cancel's absence stays static, 70.
