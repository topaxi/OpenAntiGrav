# The craft flag word `craft+0x1c0`: every writer, every bit (2026-10-20)

**Binary:** Pulse PSP USA `BOOT.BIN` (`/pulse/BOOT-psp-pulse-usa.BIN`). **Lane:**
`craft-flags-sideshift`. **Live:** PPSSPP v1.20.4, software renderer, Time Trial,
Talon's Junction, VENOM, restarted from `time-trial-talons-venom-grid.ppst`.

`craft` here is the physics craft, `*(entity+0x94)`. Fourteen bits of its flag word
are tested across the force law (`engine.md` used to say only bit 0 was established).
This page finds every store to the word, names each bit by its writer, and records
which bits nothing ever sets.

## How the writers were found

1. **Static.** `scripts/psp-relocate.py --binary BOOT.BIN field 0x1c0` lists every
   load and store at displacement `0x1c0` in the image: 109 stores. Dropping the 31
   stack stores (`(r29)`) leaves 62, and the base register of each was traced back
   to what it holds. A byte or halfword store at `0x1c1`-`0x1c3` would also reach
   the word; `field 0x1c1`, `0x1c2` and `0x1c3` return no store at all.
2. **Live.** `scripts/psp-craft-flags.py` breaks at `Ship_UpdateSideshiftInput_q`
   (`0x08846a54`) once a frame for the player, logs the word with the sideshift
   timers, the pad clock `craft+0x2cc`, the pickup object's fire word and held id,
   and puts a write-on-change memory breakpoint on `craft+0x1c0`. Every stop is
   logged with its pc. PPSSPP stops **before** the store executes, so the word read
   at a stop is the old value; each store's direction was read off its `ori`/`and`. The script drives the pad: cross held, a veteran double tap
   of `L` and then of `R` during the countdown, the same taps after GO, and
   `SQUARE` once the free Turbo is held. 1,100 frames from a restart (332 frames of
   countdown).

## The writers, by base

`craft` is `*(entity+0x94)`. **The pickup object** is `*(entity+0x4c)`: it owns the
fire-request word `+0x1b8`, the held weapon id `+0x1bc` and a copy of that id at
`+0x1c0`. That copy is a **different field that happens to share the offset**.
Every `WeaponPickup_*` store, and `FUN_0885776c`, `FUN_088612c8`, `FUN_088612d8`,
`FUN_088612e8` and `FUN_088629f8`, writes the pickup object's `+0x1bc`/`+0x1c0`
pair, never the craft. `FUN_08844ec4`'s store increments a counter on
`*(0x08b31774)`. The stores above `0x08880000` belong to unrelated structures (UI,
movie player, libraries).

| Bit | Set by | Cleared by | Meaning | Confidence |
| --- | --- | --- | --- | --- |
| `0x1` | `Ship_HoverTwoPoint` `0x0884a938`, `Ship_HoverFourCorner` `0x0884b3b4` (`ori 0x1`) | `Ship_UpdateHover` `0x08848724` (`and ~1`, every frame before the probes) | a hover probe touched something this frame | 95: live, two stores a frame, the only pcs besides the ones below |
| `0x2` | `Craft_EnterGridState` `0x088486e0` | `Craft_ReleaseFromGrid` `0x088486f4` | the grid state | 95: live `0x3` through the countdown, cleared at frame 332 by `0x088486f4` (`ra 0x0884860c`, inside `Craft_SetState`) |
| `0x4` | `PlayerStatus_Update` `0x0883bec8` | `PlayerStatus_Update` `0x0883bf5c` | the Autopilot's speed-up (`autopilot.md`) | 90 (earlier lane) |
| `0x8` | `Craft_SetUncappedThrust` `0x0884868c` (`a1 != 0`) | the same, `a1 == 0` | uncapped thrust (Autopilot, finished) | 90 (earlier lane) |
| `0x10` | **nothing** | - | the slowdown arm's escape in `Ship_UpdateEngine` | 85: no store sets it, none seen live |
| `0x20` | **nothing** | - | `Ship_UpdateSteering`'s bias `craft+0x2e4` | 85 |
| `0x40` | **nothing** | - | `Ship_UpdateSteering`'s reversed controls | 85 |
| `0x80` | **nothing** | - | read once in `Ship_HoverTwoPoint` (`0x0884aab8`) | 85 |
| `0x100` | **nothing** | - | `Ship_UpdateCraft` zeroes both airbrakes in the control record (`0x08849824`) | 85 |
| `0x200` | `Ship_UpdateStartBoost` `0x0883fe18` | `Ship_UpdateStartBoost` `0x0883fe30` | **the Turbo pickup is running**: a copy of the pickup object's `+0x1b8 & 0x200`, made every frame | 95: live, set on the frame after the pickup's bit, cleared 47 frames later with it |
| `0x400` | `Ship_UpdateSideshiftInput_q` `0x0884703c` | the same, `0x08846ff0` / `0x0884707c` | the barrel roll's landing payout (`input-bindings.md`) | 90 (earlier lane) |
| `0x800` | `Ship_UpdateSideshiftInput_q` `0x088475c0` | the same, `0x088475e0` | the left sideshift's timer (`entity+0x8a4`) is running | 95: live |
| `0x1000` | `Ship_UpdateSideshiftInput_q` `0x0884760c` | the same, `0x08847624` | the right sideshift's timer (`entity+0x8a8`) is running | 95: live |
| `0x2000` | `Ship_UpdateEntity` `0x08842914` | the same, `0x08842924` | **the Eliminator wrong-way kill** (below) | 82: static, not watched live |

`Ship_InitCraft` (`0x0884952c`) zeroes the whole word. Nothing else writes it.

**The five dead bits.** `0x10`, `0x20`, `0x40`, `0x80` and `0x100` each have a
reader and no writer. The static census covers every `sw`/`sh`/`sb` at the word's
four byte offsets. It cannot see a store through a computed address (`addiu rX,
base, 0x1c0` then `sw 0(rX)`), a VFPU `sv.s` or a struct copy, so the live leg is
what closes it: across a countdown, a release, two sideshifts, a Turbo and 768
frames of racing, the write watch stopped at exactly the static set
(`0x08848724` x1098, `0x0884a938` x1099, `0x088486f4`, `0x088475c0`, `0x088475e0`,
`0x0884760c`, `0x08847624`, `0x0883fe18`, `0x0883fe30`, once each). So the
branches behind them are dead in Pulse as shipped: no steering bias, no reversed
controls, no slowdown escape. Not exercised live: a barrel roll, a respawn, an
Eliminator race.

## The live trace, in numbers

| Frame | Word | What happened |
| --- | --- | --- |
| 0-331 | `0x3` | grid; taps at 60/66 (`L`) and 150/156 (`R`): no tap window opens (`entity+0x89c` stays `0.0`), no shift |
| 332 | `0x1` | `Craft_ReleaseFromGrid`; `craft+0x2a4` 0 to 1 |
| 392/398 | `0x1` | `L` double tap: window `0.25` at 395, shift armed at 401 |
| 401-412 | `0x801` | left shift, `entity+0x8a4` from `0.2`, lockout `entity+0x8ac` `1.0` |
| 491-502 | `0x1001` | right shift from the `R` taps at 482/488 |
| 573 | `0x1` | `SQUARE` with held id `4` |
| 576 | `0x1` | pickup fire word `0x200`, held id `-1` |
| 577-623 | `0x201` | Turbo running; `craft+0x2cc` held at `0.0` |
| 624 | `0x1` | `craft+0x2cc` `0.017`, then `+dt` a frame |

## `Ship_UpdateEntity` (`0x088418e0`), the per-craft entity update

Named here from what it calls and stores, confidence 75. Live, it is the return
address of `Ship_UpdateSideshiftInput_q` (`ra 0x08841b34`) and of
`Ship_UpdateStartBoost` (`ra 0x08841dc0`). It also drains the pending slowdown
into `Ship_AddSlowdown` (`0x08842104`, `crates/weapons/src/slowdown.rs`) and holds
the airborne and distance reset triggers (`leaving-the-track.md`). Its last block
is the wrong-way kill:

```c
mode = (g_debug_mode_override == 0) ? game->mode : 0;     // 0x08842864..0x08842878
if (mode == 8 || mode == 18) {                            // Elimination, MP Elimination
    d = dot(node(entity+0x794)->row2, entity+0xb00);      // forward . located tangent
    if (d < -0.4f) craft->flags |=  0x2000;                // 0x088428dc: 0xbecccccd
    else           craft->flags &= ~0x2000;
}
```

`entity+0xb00` is the tangent of the spline record `Craft_UpdateLapProgress`
locates (`race-progress.md`). Two readers act on the bit:

- `Ship_UpdateCraft` (`0x088497e8`) writes `100.0` into both airbrake inputs of the
  control record, the same override the grid state makes, without zeroing thrust
  or steering.
- `Ship_UpdateEngine` (`0x0884c954`), as its last step, writes `0.0` to the
  throttle state, the thrust and the lift.

A craft turned more than about 114 degrees off the track's direction in an
Eliminator race coasts with its airbrakes out until it turns back.

**Census of ours (2026-10-20, a scratch test not kept):** an Eliminator field with
the player parked, seed 1, 90 s on each of the twelve Pulse circuits forward and
reversed, set the bit on **no tick at all**; a craft turned round by hand at tick
900 on `16_Track` and `03_Track` had it set on the next tick. Ours reads the
nearest sample's tangent rather than the lap-progress record's (chosen, see
`oag_raceplay`'s `wrong_way_kill`); the census found no tick where the two could
disagree, and the scan runs only in Eliminator. Pure USA's
image has no masked read of the word against `0x2000` at all.

## `0x200`, the Turbo, and what reads it

`Ship_UpdateStartBoost` (`0x0883fdec`) opens with

```c
if (entity->pickup != NULL && (entity->pickup->fire & 0x200)) craft->flags |=  0x200;
else                                                          craft->flags &= ~0x200;
```

and `0x200` is `Turbo_Fire`'s running bit (`autopilot.md`). `Ship_UpdateEngine`
reads the copy four times:

1. **Prologue** (`0x0884c5cc`): `craft+0x2cc = (flags & 0x200) ? 0.0 : craft+0x2cc + dt`.
   `Ship_ApplySpeedupPad` scales the pad force by `craft+0x2cc` while it is below
   `1.0`. A pad gives nothing while a Turbo runs and fades back in over the second
   after it. `Ship_InitCraft` writes `0.0`; its one caller is `Craft_Construct_q`
   (`0x088412d4`), so a respawn does not reset the clock.
2. **Early return** (`0x0884c678`, `0x0884c690`): the stun/slowdown return is skipped
   while `0x200` is set. A running Turbo keeps the engine through a stun.
3. **Turbo add** (`0x0884c89c`): `(flags & 0x200 || flags & 0x400) && craft+0x2a4 == 1`
   adds `<Engine turbo>`. If d-pad Up is held (`*(craft+0x78)+0x24 & 1`) the
   **boost lift** is then `g_turbo_jump * T`.

`g_turbo_jump` is `0x08b36be8`. `Xml_ReadGlobalSettings` writes it at `0x0883ac54`
from the attribute whose name pointer sits at `sp+0x578`, set to `0x08a7b3cc` at
`0x0883aa2c`. That string reads `turbo_jump` live, and the value reads `0.1`.
So it is `<Special turbo_jump>`. The lift goes into the local accumulator's `.y`
(`0x0884c990`) and is taken before the launch multiplier and the doubling.

**The AI gets the bit too.** `Ship_UpdateEntity` calls `Ship_UpdateStartBoost` in
state 1 when `entity+0x368` is `0` (the human) or `2` (every AI craft,
`0x08841d7c`-`0x08841db8`), so an opponent's Turbo escapes a stun and fades its
pads as the player's does. The lift's button is a held-mask bit the AI never
writes (it fills the analog record only), so no opponent lifts: ported as
`ShipControls::dpad_up`, which only the pad fills.

## The sideshift gate: `& 0x2`

`Ship_UpdateSideshiftInput_q` (`0x088472bc`) wraps the whole gesture block in
`if (!(flags & 2) && entity+0x8ac <= 0)`. That covers the novice flick's arm and
fire and the veteran tap windows and fire. The countdowns of `+0x89c..+0x8ac` and
the `timer > 0 -> flag, lockout = 1.0` tail run outside it. Live: four taps
during the countdown armed nothing, and the same taps after GO shifted.
Confidence 92.

Two more callers set or clear the grid bit besides `Craft_SetState`, both through
`Entity_SetGridState` (`0x088272f4`: `entity = *(a0+0x2c0)`, then
`a1 ? Craft_EnterGridState : Craft_ReleaseFromGrid` on its craft, confidence 80 for
what it does). One is a mode object's per-frame update (`FUN_08826044`, through two
one-line wrappers `FUN_0882652c`/`FUN_08826548`). The other is `FUN_0889683c`,
in game modes 14 and above (multiplayer). What they hold the grid state for is not
read.

## Not here

- **`craft+0x2e0`'s arming** is `Ship_AddSlowdown`, called only from
  `Ship_UpdateEntity` (`0x08842104`) on the pending weapon slowdown. Already ported
  (`crates/weapons/src/slowdown.rs`); the open question in `docs/physics/README.md`
  was stale.
- **`craft+0x2a4 == 1`** gating the turbo add is not reproduced (no racing-state
  enum in `ShipState`).

## Cross-title

- **Pure (PSP, USA):** the same word, the same offsets. `masked 0x1c0 0x2` reads it
  in the sideshift function (`0x0892b784`). `masked 0x1c0 0x200` gives the engine's
  four reads (`0x0892f1f8` prologue, `0x0892f298`/`0x0892f2b0` early-return escape,
  `0x0892f52c` turbo add). The pad clock is `craft+0x284` (written `0x0892f210`,
  read twice in `FUN_0892ccd0`). **Checked, applies, ported**: the sideshift gate,
  the Turbo escape, the pad fade and the lift run on Pure (confidence 75, reads
  only), as the `CraftLaws` axes `sideshift_grid_gate` and `turbo_running_laws`,
  `measured` on Pure, `inherited` from Pulse on HD and from HD on Omega and 2048. `masked 0x1c0 0x2000` returns **zero**, so
  Pure has no wrong-way kill: **checked, differs** (`CraftLaws::eliminator_wrong_way_kill`
  measured `false`).
- **HD (PS3):** the flag word is `craft+0x264` (`autopilot.md`) and numbers its bits
  differently. `Craft_UpdateThrottle` (`0x000efb50`) resets a clock at `craft+0x320`
  under `0x20`, the shape of Pulse's `0x200`/`0x2cc`, and zeroes `craft+0x30c` under
  `0x200`. **Not mapped**: HD inherits Pulse's laws, labelled `inherited` in its
  `CraftLaws`. Open.
- **Omega, 2048:** inherit HD, so Pulse.

## Names

| Address | Name | Confidence | Evidence |
| --- | --- | --- | --- |
| `0x088418e0` | `Ship_UpdateEntity` | 75 | above: live return address of two named callees, the slowdown drain, the reset triggers, the wrong-way block |
| `0x088272f4` | `Entity_SetGridState` | 80 | above: seven instructions, read whole |
