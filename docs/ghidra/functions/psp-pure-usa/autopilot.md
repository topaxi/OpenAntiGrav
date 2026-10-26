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

- **Nothing outside these two writes the timer** in the weapon code
  (`psp-relocate.py field 0x15c`: `0x0884c788` and `0x0884c7bc` between
  `0x08830000` and `0x08860000`; the other `swc1` hits are stack slots and
  other structures elsewhere in the image).
- **The running byte is cleared only by the update and by a whole-record
  reset** (`0x0884e194`, among a run of clears from `0x0884e174`). Its readers
  are the dispatch (`0x0884d068`, `0x0884d080`) and two outside the weapon code
  (`0x08922344`, which is the `SubWeapon` timer's HUD read, and `0x08929b78`,
  next to the `~AUTOPILOT` open at `0x08929c40`).

So Pure has no absorb cancel of the kind Pulse and HD have, and nothing holds
the Autopilot through a corner. **Confidence 70** for the absence of a cancel:
a cancel through another base register or a whole-record copy is not excluded
by a field scan, and nothing here was watched running.
