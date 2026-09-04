# Pure's rocket and ship-collision effects: which trigger fires what

Reads Pure's own trigger code for the three effects
[`pure-status.md`](../../../formats/pure-status.md) already showed decode and
play off Pulse's recovered triggers, and answers the open question of what
Pure's rocket does on a track hit, since `WO_ROCKET_EXPLO_TRACK` does not
exist on this disc under any name (see that page).

| | |
| --- | --- |
| **Binary** | `PSP_GAME/SYSDIR/BOOT.BIN` (Pure, PSP), image base `0x08804000` |
| **Related** | [`psp-pulse-usa/rocket-visuals.md`](../psp-pulse-usa/rocket-visuals.md) (the Pulse counterpart every reading here is checked against), [`psp-pulse-usa/contact-response.md`](../psp-pulse-usa/contact-response.md) (`ShipCollisionFx_Trigger`'s Pulse original), [`psp-pure-eu/string-anchors.md`](../psp-pure-eu/string-anchors.md) (the import trap worked below) |

| Address | Name | Confidence |
| --- | --- | --- |
| `0x0885ee0c` | `Rocket_Init` | 76 |
| `0x088561f0` | `Rocket_SpawnCraftExplosion` | 78 |
| `0x0885f28c` | `Rocket_Update` | 80 |
| `0x0888e340` | `ShipCollisionFx_Trigger` | 82 |
| `0x08831ddc` | `Sound_Play` | 82 |

## The import trap applies here too, and the workaround is the same

`get_xrefs_to` on every `WO_ROCKET_*`/`WO_SHIP_COLL_*` string in this binary
reports "No references found", exactly the symptom
[`psp-pure-eu/string-anchors.md`](../psp-pure-eu/string-anchors.md) documents:
Pure was imported at base `0`, so the loader's PSP relocations were a no-op
against `.rel.text`, and rebasing the database afterwards moved the data
listing without ever revisiting the `lui`/`addiu` pairs that build a string's
address in code. That page marked `psp-pure-usa` as "untested, almost
certainly the same relocation shape" - confirmed here: `get_xrefs_to` on this
binary's own `Data\XML\HandlingStats.xml` also returns nothing, and the
`off = A - 0x08804000` / split-into-`lui`+`addiu` technique that page
describes finds the one real reference
(`FUN_088990f4`, `_addiu a1,a1,-0x3bb0`) exactly where it predicts.

**One addition to that page's method, learned here**: the `addiu` half of a
string-loading pair is very often in a `jal`'s delay slot, and
`search_instructions`' `mnemonic` field for a delay-slot instruction is
`_addiu`, not `addiu` (the same underscore `disassemble_function` prints).
Filtering by `mnemonic: "addiu"` alone silently misses these; search by
`operand_pattern` only, mnemonic empty, to catch both.

**The `jal` call-target wart layers on top of the string wart, and it is the
same defect on a different operand class.** `get_function_callers` returns
nothing for any of the four functions below, for the same reason
`psp-pulse-usa/billboards.md` recorded for `jal`: targets print as
unrelocated pseudo-addresses (`jal 0x0006de1c`, not `jal 0x08871e1c`), so
Ghidra's own call-graph is built against the wrong address and finds nothing
at the real one. This is not a display quirk with its own separate cause -
the `.rel.text` relocations were never applied (the string wart's own
finding), so a `jal`'s 26-bit immediate field still encodes a target computed
for load base `0`, and `imm26 << 2` is a **file offset**, not a runtime
address, exactly the way an un-rebuilt `lui`/`addiu` string address is. `off
+ 0x08804000` recovers the real target for the identical reason it recovers a
real string address: `0x08804000` is not a fitted constant, it is this
program's own image base, the thing the never-applied relocation was
supposed to add. Finding a caller means that arithmetic against a `jal`'s
printed operand, searched with `mnemonic: "jal"` and the pseudo-target's hex
tail as `operand_pattern`. This is how the 26 call sites of the shared spawn
helper below were found; it did **not** turn up a caller for any of the four
renamed functions themselves, so none of the four has a verified call site -
the reason each stays inside the 70-84 decompilation-only band regardless of
how clean its own body reads.

## One generic spawn helper, 26 call sites, shared with everything else

**Correction, 2026-09-04: this section's own address had the exact arithmetic
error the paragraph above warns about** - `0x0006de1c + 0x08804000` was
computed as `0x0886de1c` (using `0x08800000`), which is not a function entry
at all - `get_function_by_address` resolves it to the *middle* of an
unrelated function (`FUN_0886db80`, body `0x0886db80`-`0x0886de2f`). Caught
while chasing the `Absorb` cue's own trigger for
`every-sfx-trigger-is-a-pulse-reading-applied.md` and cross-checking this
page's `Sound_Play`-call formula against a second, independently-derived
case (`0x0002dddc` -> `0x08831ddc`, the `Sound_Play` rename above).

**The correct sum is `0x08871e1c`, and this is not arithmetic alone: it is
the same structural-match evidence class `Sound_Play` earned, run against the
function this section actually claims a counterpart of.** Both `0x08871e1c`
and `0x08831ddc` were derived by the identical `+0x08804000` delta, so a
single wrong constant could in principle have produced two wrong-but-plausible
answers - what rules that out is decompiling `0x08871e1c` and comparing it
directly against `Psys_Spawn_q` (`0x08915484`) itself, not just checking that
it's *a* function entry:

```c
// Pure, 0x08871e1c                      // Pulse, Psys_Spawn_q (0x08915484)
int FUN_08871e1c(int param_1, ...)       int Psys_Spawn_q(int param_1, ...)
{                                        {
  func_0x0007d824();                       func_0x001273bc();
  *(undefined4*)(param_1+0x3c) = ...;      *(undefined4*)(param_1+0x38) = ...;
  uVar1 = func_0x001376f8(                 uVar1 = func_0x000efcf4(
            _DAT_002b0304, param_2);                 _DAT_002ae248, param_2);
  func_0x0006e894(param_1, uVar1, ...);    func_0x00112200(param_1, uVar1, ...);
  return param_1;                          return param_1;
}                                        }
```

Four statements, identical shape and order, the same six-parameter signature,
and the offset shift (`0x3c` vs `0x38`) is the same struct-layout-drift
pattern `Sound_Play`'s own comparison already showed. This is decisive in a
way "it's a function entry with a plausible body" is not - the binary holds
thousands of small wrapper functions, and a coincidental hit on one of them
was the real risk an arithmetic-only argument couldn't rule out.

`0x08871e1c` (printed as `func_0x0006de1c` throughout, per the `jal` wart
above) is Pure's counterpart of Pulse's `Psys_Spawn_q` (`0x08915484`): second
argument a name pointer, third a four-character tag, little-endian, called
from a small allocate-tag-init-call sequence (`func_0x00090f34` /
`func_0x00090cf4` / `func_0x0008facc`) identical at every site. **26 call
sites total** (`search_instructions jal, operand_pattern "6de1c"`), so it is
not rocket- or collision-specific; the four sites below are the ones this page
identifies by name.

## `Rocket_Init` (`0x0885ee0c`) fires `WO_ROCKET_FLARE`

One call: `func_0x0006de1c(node, 0x244a2c, 0x4c464f52, &carried_matrix, 0, 0)`
at `0x0885f138`/`0x0885f13c`. `0x244a2c + 0x08804000 = 0x08a48a2c`, the bare
name `WO_ROCKET_FLARE`; `0x4c464f52` as bytes is `52 4f 46 4c` = **`ROFL`**,
Pulse's own tag for the same effect
([`rocket-visuals.md`](../psp-pulse-usa/rocket-visuals.md)). The surrounding
body builds a `0x1d0`-ish node from four `vmov.q`-copied matrix rows and a
rolloff-shaped scale (`vrcp.s` against `0x40666666` = `3.6f`), matching the
shape of Pulse's `Rocket_Init` closely enough to carry its name, but the body
is heavy PSP-vector code this pass did not fully unpick term-by-term - hence
76 rather than higher.

## `Rocket_SpawnCraftExplosion` (`0x088561f0`) fires `WO_ROCKET_EXPLO`

One call: `func_0x0006de1c(node, 0x244310, 0x58454f52, ...)` at
`0x088562e8`-`0x088562f4`. `0x244310 + 0x08804000 = 0x08a48310`, the bare name
`WO_ROCKET_EXPLO`; `0x58454f52` as bytes is `52 4f 45 58` = **`ROEX`**, again
Pulse's own tag for the craft-hit explosion. Scored 78, the same confidence
Pulse's own `Rocket_SpawnCraftExplosion_q` carries for an identical
name+tag-only reading with no verified caller.

## `Rocket_Update` (`0x0885f28c`) does **not** fire `WO_ROCKET_EXPLO_TRACK` - it fires `WO_TRACK_ROCK_DEBRIS`

This is the answer to the open question
[`pure-status.md`](../../../formats/pure-status.md#pures-particle-systems-decode-unchanged)
records - whether Pure spells the track-hit effect differently or ships none
at all. The function:

- Integrates `param_2+0x50` (age) by `dt` (`param_1`), first instruction.
- Runs a swept query twice through `func_0x00020a9c`, matching the shape of
  Pulse's two `func_0x0002d98c` calls (surface probe, flight sweep).
- On no hit, subtracts `param_1 * 50.0` from the velocity's Y - **the exact
  same fall constant** Pulse's `Rocket_Update` uses.
- On a hit, one branch (`local_4b8 == 1`, gated on
  `*(int *)(*(int *)(param_3 + local_4c0*0xc + 0x2454) + 0x6c) == 0`,
  structurally the track-vs-craft test) spawns an effect **twice**, once per
  detonating code path, each with its own tag:

  ```c
  func_0x0006de1c(iVar25, 0x244a48, 0x42444f52, &local_490, 0, 0);  // tag "RODB"
  func_0x0006de1c(iVar25, 0x244a48, 0x32444f52, &local_400, 0, 0);  // tag "ROD2"
  ```

  **Both branches spawn the same name**, `0x244a48 + 0x08804000 = 0x08a48a48`.
  Reading memory there directly (not inferring from the constant):

  ```
  0x08a48a2c  "WO_ROCKET_FLARE\0"
  0x08a48a3c  "~ROCKETTVL\0\0"
  0x08a48a48  "WO_TRACK_ROCK_DEBRIS\0\0\0\0"
  0x08a48a60  "WeaponCommon..."
  ```

  So the name is **`WO_TRACK_ROCK_DEBRIS`**, not `WO_ROCKET_EXPLO_TRACK`.
  Verified byte-identical on both pressings: the same four strings in the
  same order sit at the same offsets on `psp-pure-eu` (`0x08a4732c` there).
- Also carries the speed conversion `fVar27 / 3.6` immediately before scaling
  velocity by it - the exact divisor
  [`rocket-visuals.md`](../psp-pulse-usa/rocket-visuals.md) established for
  Pulse's authored-speeds-are-km/h finding.

**`ROD2` is Pulse's own tag for `WO_ROCKET_EXPLO_TRACK`.** Reused here for a
different resource name entirely - the tag is a short internal label for
"the rocket's second detonation call site", not a name for the asset it
happens to carry. `RODB` is presumably "rocket detonate, branch B" by the
same pattern; no direct evidence names it further.

**`WO_TRACK_ROCK_DEBRIS.POB` exists on Pure's disc, and it decodes.** It
resolves in `PSP_GAME/USRDIR/Data.wad` at the WAD name hash `0x7fe66b3f`
(entry 595 of 841, computed per
[`wad.md`](../../../formats/wad.md#the-name-hash) and checked directly against
the archive listing - not inferred from the string being present), and the
unmodified `.pob` parser reads it as 3 emitters: a one-tick root named
`WO_TRACK_ROCK_DEBRIS` itself (8 additive streaks), a peer root `ROCK_DEBRIS`
(16 alpha-over billboard chunks whose particles each spawn a `trail` child),
and that `trail` child (a fading billboard, longer and more spread-out
lifetime than its parent). The root record naming itself
`WO_TRACK_ROCK_DEBRIS` corroborates the hash hit independently of the WAD
lookup - per [`pob.md`](../../../formats/pob.md), the root's own name is the
resource's own, so the payload agrees with the directory rather than some
unrelated blob happening to hash to `0x7fe66b3f`. Reads as a sensible
debris-impact shape next to this effect's siblings. Until now this name was
catalogued in
[`pob.md`](../../../formats/pob.md#flag-0x1-is-this-effect-loops---2026-08-12)
as **PS2-only within Pulse** (absent from Pulse's own PSP `Data.wad`, present
in `WADS2.WAD`); that finding is about Pulse's two platforms and stands
unchanged, but it did not anticipate the name shipping on a different title's
PSP disc as its rocket's own track-hit effect - confirmed absent from Pulse's
own PSP `Data.wad` by the same hash, so this is a second, independent
appearance of the name, not the same file already covered by the earlier
count.

Scored 80: four independent structural invariants match Pulse's
`Rocket_Update` exactly (dt integration first, two-sweep collision shape, the
`50.0` fall constant, the `/3.6` speed conversion), and the two spawn calls
themselves are unambiguous disassembly, not decompiler guesswork - but no
caller is verified (the `jal` wart above) and the surrounding vector code is
only partially read, so it stays short of the runtime-verified 88 Pulse's own
`Rocket_Update` earned.

## `ShipCollisionFx_Trigger` (`0x0888e340`) matches Pulse's function of the same name almost exactly

Pulse's own `ShipCollisionFx_Trigger` (`0x089246b4`,
[`contact-response.md`](../psp-pulse-usa/contact-response.md)) takes
`(intensity, instance, kind, damaged)` and branches `kind`/`damaged` across
four spark variants. Pure's function takes the same four-argument shape
(`float, int, int, char`) and, reading its three `func_0x0006de1c` calls
directly:

| Pure branch | Name (verified by reading memory) | Tag |
| --- | --- | --- |
| `param_3 == 1`, returns immediately | `WO_WEAPON_ABSORB` (`0x08a4e76c`) | `0x4f534241` = **`ABSO`** |
| gate passes, `param_4 == 0` | `WO_SHIP_COLL_SPARK_NODAMAGE` (`0x08a4e7a8`) | `0x444e5053` = **`SPND`** |
| gate passes, `param_4 != 0` | `WO_SHIP_COLL_SPARK_DAMAGE` (`0x08a4e78c`) | `0x44415053` = **`SPAD`** |

This is Pulse's structure exactly: `kind==2`/`param_3==1` is the shield-absorb
case and bypasses the cooldown-gated path entirely (an early `return` here,
matching Pulse's documented cooldown-bypass for the absorb kind), and the
damaged/not-damaged split under the gate reproduces Pulse's
`WO_SHIP_COLL_SPARK_DAMAGE`/`_NODAMAGE` choice verbatim, tag shorthand and
all (`SPAD`/`SPND` reading as "SPArk Damage"/"SPark NoDamage"). Pure's version
has no third `kind` value visible here, so whether Pure ships an equivalent to
Pulse's `WO_SHIP_SPARK_DAMAGE_LEACHBEAM` branch (`kind==1`) is not settled by
this function alone - it may live in a fourth branch this reading did not
need, or Pure's Leach Beam may not share this trigger.

Scored 82: three literal strings pin the variant selection unambiguously (not
inferred, read directly from memory), and the structural agreement with an
independently-recovered Pulse function of the same shape is exactly the kind
of second-binary corroboration the confidence rubric weighs above a second
reading of the same file - short of a runtime trace, which neither binary has
for this function, so it does not clear the rubric's 85+ band.

## `ShipCollisionFx_Trigger` also fires the `.COLLISIONS` sound cue, same as Pulse

Answers, for `oag_game::audio::sfx::Cue::Collision`'s own doc comment (see
`crates/game/src/audio/sfx.rs`), whether Pure calls the same `Sound_Play` site
at the same edge Pulse does, and whether the gating matches. Not yet extended
to the other eight `Cue` variants.

**The cue exists on Pure's disc**, confirmed independently of anything in
Ghidra: `just wad sounds "data/images/pure-psp-usa.chd:PSP_GAME/USRDIR/Data.wad"`
lists a `.COLLISIONS` cue (10 waveforms) in both `SHIP_CL` (bank `#471`) and
`SHIP_ZM` (bank `#472`) - the same dotted name Pulse's own executable passes
(confirmed while chasing this: `psp-pulse-usa/contact-response.md`'s call site
turned out to be an indirect pointer-table load the earlier pass had misread
as a bare `"COLLISIONS"` literal - corrected there the same day). Pure's own
call, by contrast, builds `0x24a780` directly the way its three spark names
do, not through a table - read as memory rather than inferred either way, and
landing on the identical dotted string both times.

**`ShipCollisionFx_Trigger`'s decompile (above) calls it at the same point
Pulse's does.** Right after the cooldown gate (`func_0x0013492c(...) == 0`)
passes - the same gate that guards the spark spawn - and before the
damaged/no-damage branch, so it fires once for either branch, matching
Pulse's "fired once per surviving kind-0/1 call" (`contact-response.md`):

```c
if (*(int *)(param_2 + 0x14c) != 0) {
    func_0x0002dddc(0x3f800000, *(undefined4 *)(*(int *)(param_2 + 0x14c) + 0xd4),
                    _DAT_00288f1c, 0, 0x24a780, 0);
}
```

`0x24a780 + 0x08804000 = 0x08a4e780` (the import-relocation wart above applies
to this literal too), read directly as memory rather than inferred from the
constant: `.COLLISIONS\0`. `0x3f800000` is `1.0`, the same leading volume
argument Pulse's own call carries.

**`func_0x0002dddc` (`0x08831ddc`) renamed `Sound_Play`.** Its body is Pulse's
`Sound_Play` (`0x089392b0`,
[`sound.md`](../psp-pulse-usa/sound.md)) near-verbatim: the same six-parameter
shape, the same linked-list insertion at the end (`entity+0x38`/`+0x34` here
vs `entity+0x58`/`+0x54` there), the same `bit 0 of entity+0x3c` voice-steal
gate (`+0x5c` there) and the same conditional bank-bake lookup
(`_DAT+idx*4+0x110` here, `_DAT+idx*4+0x124` there) - offsets shifted by a
constant throughout, consistent with a different struct layout in the older
build rather than a logic change, the same shape `corroboration.md`'s other
five matches show.

**The cooldown constant is `0.8` too.** `ShipCollisionFx_Trigger` sets
`*(param_2 + 0x144) = now + 0.8` on a successful gate pass, the same field the
gate check reads - bit-for-bit the same literal Pulse's own function uses for
the spark/sound shared cooldown (`contact-response.md`). Answers this
thread's third `## Open` question for the `Collision` cue specifically: the
gating matches, not just the call site's existence.

Scored **82** for both the call site and the `Sound_Play` rename: decompilation
only, but corroborated by an independently-recovered Pulse function of near
identical shape and by the disc's own bank data agreeing on the cue name - the
same evidence class `ShipCollisionFx_Trigger` itself was scored on, short of
the rubric's 85+ band absent a runtime trace.

## `FUN_08925e20` fires `ABSORB` and is `ShipCollisionFx_Trigger`'s own first verified caller

Answers `every-sfx-trigger-is-a-pulse-reading-applied.md`'s next pick,
`oag_game::audio::sfx::Cue::Absorb`, and corrects the "no call site for any of
the four functions above" claim below: with the spawn-helper correction above
in hand, the same `off + 0x08804000` search
(`search_instructions jal, mnemonic jal, operand_pattern "8a340"`, the
pseudo-target for `ShipCollisionFx_Trigger`'s real `0x0888e340`) finds two
hits, one of which - `FUN_08925e20` - also appears in `Sound_Play`'s own
36-site caller list. **Decompiled in full:**

```c
void FUN_08925e20(int param_1)
{
  int iVar1;
  int iVar2;

  iVar1 = func_0x0012010c();
  if ((iVar1 != 0) && (*(int *)(param_1 + 0xc10) != 0)) {
    func_0x0002dddc(0x3f800000, *(undefined4 *)(param_1 + 0xd4), _DAT_00288f14, 0, 0x2764bc, 0);
    iVar1 = *(int *)(param_1 + 0xbf0);
    for (iVar2 = 0; (iVar1 != 0 && (iVar2 < 8)); iVar2 = iVar2 + 1) {
      _DAT_0028b5c0 = (float)iVar2 * 0.1;
      func_0x0008a340(0x3f800000, *(undefined4 *)(param_1 + 0xbf0), 1, 0);
      iVar1 = *(int *)(param_1 + 0xbf4);
      param_1 = param_1 + 4;
    }
  }
  return;
}
```

This is Pulse's `FUN_08840640` (`contact-response.md`) almost exactly: a gate
check, a second `!= 0` field check, one `Sound_Play` call, then a stagger loop
that calls `ShipCollisionFx_Trigger` with the absorb `kind` (`1` here, `2` on
Pulse - the numbering difference the `ShipCollisionFx_Trigger` section above
already established) and a `0.1`-per-iteration float stagger into a global.
`0x2764bc + 0x08804000 = 0x08a7a4bc` reads `ABSORB\0` directly - built with a
direct `lui a3,0x27` / `addiu a3,a3,0x64bc` pair (`0x08925e6c`/`0x08925e80`),
not an indirect table load - undotted, matching Pulse's own literal
(`contact-response.md`) and the cue already confirmed present in Pure's
`WEP_CLA` bank (`ABSORB`, undotted, `just wad sounds`).

**One real difference, not an error: the loop runs 8 times on Pure, 10 on
Pulse.** `iVar2 < 8` here against `iVar2 < 10` in `FUN_08840640` - read
directly off each disassembly, not inferred. Consistent with a smaller
attached-instance cap on Pure's `ShipCollisionFx` list (`Ship_DispatchCollisionFx`'s
own "up to 10" search on Pulse, `contact-response.md`) rather than a
reading error; not chased further to confirm the cap's own source field.

**Not renamed**, matching this project's own precedent on this exact
function's Pulse twin: `contact-response.md` left `FUN_08840640` unrenamed at
a comparable confidence "for now rather than renamed", pending a dedicated
rename-and-document pass rather than the pass that answered a different
question first. Scored **80**: the same evidence class as the section above
(structural match to an independently-recovered Pulse function, corroborated
disc data) minus one leg - this reading, unlike `Sound_Play`'s, has not had
its callee bodies (`func_0x0012010c`, the gate) read at all.

## What is not verified

- **No call site for `Rocket_Init`, `Rocket_SpawnCraftExplosion` or
  `Rocket_Update`**, still blocked on the `jal` relocation wart - see the trap
  section. `ShipCollisionFx_Trigger` now has one verified caller
  (`FUN_08925e20`, above); its other call sites and `Sound_Play`'s own beyond
  `ShipCollisionFx_Trigger` are equally unfound for the same reason.
- **Pure's Leach Beam trigger**, if it exists and if it shares
  `ShipCollisionFx_Trigger` at all.
- **The other seven `Cue` variants** (`SpeedupPad`, `Engine`, `Shield`,
  `ShieldActive`, `Disengaging`, `Blowup`, `LockOn`) - only `Collision` and
  `Absorb` have had their Pure trigger read so far.
- **`func_0x0012010c`, `FUN_08925e20`'s own gate check** - read only by its
  call shape (parameterless, boolean-ish return), not decompiled.
- **Wipeout HD's PPC64/TOC binary** - untouched by this finding, per the
  handover thread above.
- **Runtime verification.** Nothing here has a PPSSPP leg; every score is
  capped by the decompilation-only ceiling for exactly that reason.
- **The `func_0x00020a9c` / `func_0x0013492c` / `func_0x00090f34` family**
  read only by their call shape, not decompiled for their own sake.

## History

- **2026-09-04, second pass.** Fixed the generic spawn helper's own address
  (`0x0886de1c` -> `0x08871e1c`, an arithmetic slip caught while cross-checking
  the `jal`-wart formula) and added the `Absorb` section: `FUN_08925e20`
  fires `ABSORB` then loops `ShipCollisionFx_Trigger` eight times, Pulse's
  `FUN_08840640` shape with a real (not erroneous) 8-vs-10 loop-count
  difference. This is also `ShipCollisionFx_Trigger`'s first verified caller.
- **2026-09-04.** Added the `Sound_Play` section, answering
  `every-sfx-trigger-is-a-pulse-reading-applied.md`'s first pick (`Collision`)
  for Pure: the same call site, the same `.COLLISIONS` cue name (confirmed
  against the disc's own bank data), and the same `0.8`-second cooldown as
  Pulse. `FUN_08831ddc` renamed `Sound_Play`, confidence 82.
- **2026-09-01.** Written answering
  `pures-particle-effects-decode-and-play-and-two.md`'s "no Pure trigger has
  been read" and "does `WO_ROCKET_EXPLO_TRACK` spell differently" questions.
  Both rocket explosions and the collision spark's trigger turned out to be
  readable directly, once the string- and `jal`-relocation traps documented
  on `psp-pure-eu` were worked the same way here; the track-hit effect turned
  out to be a different resource entirely, not a spelling variant.
