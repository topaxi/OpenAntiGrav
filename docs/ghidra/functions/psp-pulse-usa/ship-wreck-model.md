# The craft's two models: when the wreck replaces the hull

| | |
| --- | --- |
| **Binary** | `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse, PSP, UCUS-98712) |
| **Subsystem** | ship models, state machine |
| **Related** | [`mesh-draw.md`](mesh-draw.md) ("The hull's extra pass": the entity's `+0x8b4` hull and `+0x8b8` wreck), [`zone-mode.md`](zone-mode.md) (`Ship_SetState` states 4 and 5), [`shield.md`](shield.md) (`Ship_GatherCollisionFxNodes`) |

Read 2026-10-01: statically (headless Ghidra on a scratch copy of the project,
decompile of `0x0883eae8`, `0x0883eb68` and their callers), then measured on a
running original (the section "Measured on a running original"). Question: the
hull model sits at `entity+0x8b4` and the wreck at `entity+0x8b8` - **when does
the wreck draw, what is drawn, and what does the craft throw as it goes out?**
Answered below; this port now draws it.

## The two functions

`entity+0x8b0` is the **live hull**: the model the craft's node tree and the
collision-FX gather use. `+0x8b4` is the ordinary hull model and `+0x8b8` the
wreck (`%s\%swreck.vex`, `shipwreck.vex` - see [`exhaust.md`](exhaust.md)),
`0` on a craft with none. Two small functions point `+0x8b0` at one or the
other:

| Address | Name | What it does | Conf. |
| --- | --- | --- | ---: |
| `0x0883eae8` | `Ship_SelectHullModel` | if a wreck exists: carries the model flag word's bit `4` (`model+0x2c`) from the wreck to the hull when the wreck was live, then clears it on the wreck. Always `entity+0x8b0 = entity+0x8b4`, then `Ship_GatherCollisionFxNodes`. | 82 |
| `0x0883eb68` | `Ship_SelectWreckModel` | the mirror: carries bit `4` from the hull to the wreck when the hull was live, clears it on the hull, `entity+0x8b0 = entity+0x8b8`, then `Ship_GatherCollisionFxNodes`. A craft with no wreck changes nothing. | 82 |

## Who calls them

`Ship_SelectHullModel`: `Ship_LoadModel` (`0x08843258`, after loading, so a
craft starts as its hull), `Ship_SetState` (`0x08844100`) cases **0, 1, 2 and
3**, and `Ship_UpdateRespawn` (`0x08847914`, at `0x08847a68`).

`Ship_SelectWreckModel`: **one caller**, `Ship_SetState` **case 5** (`0x08844578`).
Case 5 is the state `Ship_SetState(entity, 5)` enters half a second after state 4
(the explosion, `~BLOWUP`) - the chain on [`zone-mode.md`](zone-mode.md): the
shield pool reaches zero, state 4, `entity+0x874` counts `0.5 s` down, state 5.
Case 5 also sets `entity+0x860 |= 0x1000`, `+0x874 = 1.5`, and after the swap ORs
`2` into the **now live** model's flag word (`*(+0x8b0) + 0x2c |= 2`) and calls
`FUN_0883e064`. Confidence **85** on the call graph (decompiled branch by
branch; `Ship_SetState`'s case 5 arm is the jump-table entry `0x08844548`).

So the wreck is the live model from the moment the craft has finished exploding
until the next `Ship_SetState` to 0 to 3 or `Ship_UpdateRespawn` puts the hull
back.

## Measured on a running original, 2026-10-01

`scripts/psp-wreck-capture.py` calls `Ship_SetState(entity, 4)` on a craft in a
live race (PPSSPP, Talon's Junction, Venom class; a grid Piranha, so the chase
camera sees it), then photographs at 480x272 and logs the entity's state, its
three model pointers and their flag words per frame. A race reaches state 4 only
by draining a shield, so the call is the lever; everything after it is the
game's own state machine.

| Since the call | State | Live model | What the frames show |
| --- | --- | --- | --- |
| 0 to 29 frames | 4 (`entity+0x874` counts `0.5` s) | hull | the hull, unchanged |
| 30 frames | 5 (`+0x874 = 1.5`) | **wreck** | a full-screen yellow wash (kind 0), then a scorched dark model with orange fire glints where the hull was; no engine flare |
| 120 frames | 6 | wreck | the same wreck, `WO_SHIP_EXPLOSION` going off over it |

**Bit `2` hides nothing - the question the first reading left open.** The wreck
is drawn in states 5 and 6 and the hull is not. The flag words say why: the hull
(`+0x8b4`) read `0x5e026`, the wreck (`+0x8b8`) `0x52000`; at the state 5 edge the
hull reads `0x5f022` (bit `4` cleared, bit `0x1000` set) and the wreck `0x5e026`
(the hull's word, bit `4` set). So **bit `4` is the visible bit and it moves with
the live model** (`Ship_SelectWreckModel`'s carry), and case 5's `|= 2` changes
nothing here: the hull's bit `2` was already set and the wreck takes the hull's
whole word. The same bit is a widget's "on" bit elsewhere (`FUN_088bb83c` sets
`4` while a fade is rising and clears it when alpha reaches zero), and bit `2` is
the "enabled / updated" bit (`Node_UpdateTree` recurses only into children with
it; `FUN_088be070` sets it on the list it shows). State 6 adds `0x1000000` to the
wreck's word, unread. Confidence **88** (a runtime read of the flag words and the
frames agree, one title, one team and one circuit).

**The wreck authors no `0x2000` extra pass.** Its meshes carry `0x1821` (Assegai's
batches `0x1821`/`0x1021`) where the hull's carry `0x3001`; the earlier note that
"the wreck carries the same flags" read `model+0x1a8 == 1`, the fixed light basis,
which is shared (the section above). `oag_render::shine::build` finds no batch on
any of the eight teams' `shipwreck.vex`, so the wreck has no extra pass to draw.
Confidence **92** (decoded off every team's file).

**What `FUN_0883e064` throws, read live.** At the state 5 edge `Psys_Spawn_q` is
entered 14 times in a row, `WO_SHIP_FXNODE_EXPLO` then `WO_SHIP_DEATH_SPARKS`,
seven times, each pair parented to a distinct node; the Piranha's
`shipwreck.vex` authors seven `Ship Collision Fx` locators. `WO_SHIP_EXPLOSION`
follows 90 frames later, on the entry to state 6. Confidence **90**.

**Per team the wreck is a smaller, separate model**: 198 to 302 triangles against
the hull's 845 to 1,497; locators 4 (Assegai, Triakis), 7 (Piranha), 8 (EGX), 9
(AG_Systems) or 10 (Qirex, Feisar, Goteki). `zonewreck.vex` exists beside
`zone.vex` (Assegai: 2 meshes, 182 triangles).

**Camera.** For the local player state 4 also puts the camera in mode 5,
`Camera_SetMode(DAT_08b32c64, 5)`. It is not a high pull-back: it is a fixed camera at one
of the circuit's authored `Camera` nodes, zoomed to frame the wreck - read and ported
2026-10-01, see [camera.md](camera.md), "The destroy camera". The opponent wreck above is
seen from the ordinary chase camera, which is why it was the first comparison.

## What this port does

`oag_game::race::scene::wreck` draws `shipwreck.vex` (`zonewreck.vex` in Zone)
instead of the hull while the craft is `CraftState::Eliminated`, which is case 5;
the hull stays through `Destroyed`. The player's camera cuts to the circuit's own camera
for it ([camera.md](camera.md)), and `WO_SHIP_EXPLOSION` goes off 1.5 s later
([screen-flash-callers.md](screen-flash-callers.md)). The engine flare quad is not drawn on a wreck
(**seen**, mechanism unread: `Exhaust_Update` has no state test, so it is probably
the hull's node going invisible). `race::wreck_fx` throws the two particles at each
wreck locator on that edge. Pulse on a PSP disc only. Side by side with the
capture, the wreck's shape, pose, scorch and fire glints agree. **The fireballs**
read white until 2026-10-01 because the `FIRE` emitter of `WO_SHIP_FXNODE_EXPLO` (and
`SHIP_DEBRIS` of `WO_SHIP_EXPLOSION`) author a **4 bits per pixel** sprite that the `.pob`
reader refused, so they drew the procedural white disc: `FIRE`'s colour table is white all
life, so its flame colour is its texture's. A GE dump of the original (`scripts/psp-ge-dump.py`,
the `0x11e` batches) shows the additive draw binding a 128x64 `CLUT4` texture
([pob.md](../../../formats/pob.md), "Four bits per pixel"). With it the first frames match the
original's. At 160 to 180 frames the explosion's fire was brighter and longer in ours (fire-coloured pixels 2.2x
to 2.4x the original's) and its smoke thinner. **Closed 2026-10-01** (pulse-fx-recheck): the explosion's matrix
has rows of `0.75`, which scales every root emitter's spawn offset and velocity and not its sizes; ours
ignored it. With that, one tick off every emitter's run, and the `Bomb_Shockwave.vex` ring the same call
builds ([ship-shockwave.md](ship-shockwave.md), [particle-system.md](particle-system.md#the-instance-matrix-scales-a-root-emitters-spawn-and-a-run-emits-one-tick-short-2026-10-01)),
a grid opponent's wreck at the same place agrees to the colour-class counts: fire `1.04, 1.08, 1.23, 1.24, 1.57`
and smoke `1.08, 0.96, 1.0, 0.87, 0.72` of the original's at 140, 150, 160, 170, 180 frames after the call (the
earlier tree: `0.97` to `1.36` and `1.15` to `1.9`); the per-emitter pool census (counts, sizes, alpha, RMS radius, rise)
agrees to 1.0 for the smoke and within `0.8` to `1.2` for the fire and debris. The fire still holds a little
longer at the tail and ours sits about `25` px to the right of the original's (our grid slot is not the original's).

## Not read

- What `0x1000000` in the wreck's word at state 6 is.
- The wreck's two authored `Trail` nodes ([`exhaust.md`](exhaust.md)): whether a
  ribbon draws from them was not looked for.
- The craft's blob shadow under a wreck, and the other per-craft overlays (shield
  shell, absorb): ours still draw as for a hull.
