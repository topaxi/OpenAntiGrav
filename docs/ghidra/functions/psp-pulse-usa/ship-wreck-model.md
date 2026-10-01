# The craft's two models: when the wreck replaces the hull

| | |
| --- | --- |
| **Binary** | `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse, PSP, UCUS-98712) |
| **Subsystem** | ship models, state machine |
| **Related** | [`mesh-draw.md`](mesh-draw.md) ("The hull's extra pass": the entity's `+0x8b4` hull and `+0x8b8` wreck), [`zone-mode.md`](zone-mode.md) (`Ship_SetState` states 4 and 5), [`shield.md`](shield.md) (`Ship_GatherCollisionFxNodes`) |

Read 2026-10-01, static only (headless Ghidra on a scratch copy of the
project, decompile of `0x0883eae8`, `0x0883eb68` and their callers). Question:
the `0x2000` extra pass is drawn for the hull model at `entity+0x8b4`, and the
wreck model at `entity+0x8b8` carries the same flags - **when does the wreck
draw?** Nothing in this port loads `shipwreck.vex` for a race, so there is no
wreck to give the pass; this page is the trigger, recorded so the wiring can be
done from it.

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

## What is not read

- **What bit `2` of the model's flag word does.** `zone-mode.md` calls it "a
  render flag that hides the model"; this pass did not read a consumer of
  `model+0x2c`. If it hides the wreck the swap is for the collision-FX node list
  and the craft's tree only and nothing of the wreck is ever drawn in a race. A
  GE list of a craft in state 5 would answer it (does a mode-2 PRIM with the
  wreck's world matrix appear?). That recording was not made.
- What bit `4` is (it follows the live model across the swap).
- Whether the wreck's `0x2000` batches use the same fixed basis as the hull's
  (`model+0x1a8 == 1` was read on the wreck live: [`mesh-draw.md`](mesh-draw.md))
  - yes, recorded there - so the pass for it would be the hull's, over the wreck's
  own batches.

Nothing is wired from this: `shipwreck.vex` is not loaded, no state 5 is
modelled with a model swap, and no wreck is drawn. Giving the wreck the extra
pass is a consequence of drawing the wreck, which is its own piece of work.
