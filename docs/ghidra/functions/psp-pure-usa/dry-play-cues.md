# Pure's `SpeedupPad` and `Disengaging` cues, found through the shared dry-play chain

**Binary:** `pure-psp` `BOOT.BIN`, image base `0x08804000`.
**Status:** decompilation only, no PPSSPP leg for either title.

Answers `oag_game::audio::sfx::Cue::SpeedupPad` and `Cue::Disengaging` for
Pure. Found as a side effect of
[`shield-sound.md`](shield-sound.md)'s own investigation: once `Sound_Play`
(`0x08831ddc`) and the dry-play chain's gate helper (`func_0x001209e0`,
`0x089249e0`) were both identified, searching every `jal` call site against
that gate helper's pseudo-target (`search_instructions`, `mnemonic: "jal"`,
`operand_pattern: "1209e0"`) turned up nine callers total. Three read as
wired cues: `shieldactive` (`shield-sound.md`), and the two this page covers.
The other four (`WEAPONPICKUP`, `TURBO` twice, `FLIP`, `~AIRBRAKE_MOTOR`) are
unwired cues or non-`Cue` sounds and are out of scope, per `sfx.rs`'s own
"nothing here is fired on a guess" rule.

## `SpeedupPad`

Pulse's own trigger (`sfx.rs`'s `Cue::SpeedupPad` doc comment): `Ship_ApplySpeedupPad`
calls `Sound_Play(..., "SPEEDUPPAD", ...)` from its new-pad branch, and the
`Placement` table records that the *dry* variant specifically goes through
`ExhaustFlare_OnSpeedupPad`/`FUN_0883e9b0` depending on `racer+0x368` -
two branches, one positional (through the flare's own emitter) and one dry.

Pure's `FUN_0886b548` is that same two-branch dispatcher, in one function
rather than two: gated on `entity+0x1a4`, it reads `craft+0x488` (the same
field `func_0x001209e0`'s own gate reads, Pure's struct-drifted `0x368`) and
branches - non-zero calls `Sound_Play` (`func_0x0002dddc`) directly with a
positional emitter, zero calls the dry chain
(`func_0x001209e0(entity, bank, name, 0x400, 0)`). Both branches pass the
same string, built once and moved through registers rather than rebuilt:
`0x244fb8`-family offset - read directly, `0x002450b8 + 0x08804000 =
0x08a490b8` reads `SPEEDUPPAD\0`, matching `sfx.rs`'s literal and the disc's
own `SPEEDUPPAD` cue (`just wad sounds`, confirmed present).

**Three near-identical sibling functions sit beside it**
(`0x0886b600`/`0x0886b6b8`/`0x0886b770`), same shape, different strings
(`TURBO`, `FLIP`, `TURBO` again) - the same family of weapon/pad-effect
dispatchers `SPEEDUPPAD`'s function belongs to, not separately wired cues.

Not renamed - matching this project's precedent of leaving `sfx.rs`-cited
Pulse functions like `FUN_0883e9b0` unnamed even when their role is well
understood, and consistent with `ShipCollisionFx_Trigger`'s own callers
staying unverified for the same `jal`-wart reason.

## `Disengaging`

Pulse's own trigger (`sfx.rs`'s `Cue::Disengaging` doc comment):
`Autopilot_Update` (`0x08861404`) plays it on the tick the remaining time
crosses `1.0`, through the same `FUN_0883e9b0` -> `FUN_0893a768` ->
`Scream_PlaySoundByName` dry chain `shield-sound.md` documents.

Pure's `FUN_0884c794` calls the same chain's gate helper
(`func_0x001209e0`) with volume `0x400` and a string built directly
(`lui a2,0x24` / `addiu a2,a2,0x3dbc`, `0x00243dbc + 0x08804000 =
0x08a47dbc`, reads `disengaging\0` - undotted, lowercase, matching `sfx.rs`'s
literal and the disc's own `disengaging` cue). The function's own gate shape
- a countdown field compared against a threshold before the call, matching
`sfx.rs`'s "the tick the remaining time crosses `1.0`" description - is
structurally consistent with an autopilot countdown, though this pass did
not decompile enough of the surrounding entity fields to name the countdown
field itself or confirm the `1.0` threshold on Pure's side.

Not renamed, same reasoning as `SpeedupPad` above.

## Confidence

**78** for both: each cue's string is read directly from memory, matching
`sfx.rs`'s literal exactly, through the same dry-play chain
`shield-sound.md` already corroborated structurally against Pulse
(`Scream_PlaySoundByName`) at confidence 82 - one point below that page's own
82 because neither caller function here was itself compared field-for-field
against its Pulse counterpart (`Ship_ApplySpeedupPad`'s dispatcher,
`Autopilot_Update`) the way `Scream_PlaySoundByName` was, only the string and
the shared downstream chain.

## What is not verified

- **Either function's own caller**, blocked on the `jal` wart.
- **`Disengaging`'s countdown field and threshold on Pure** - the call site
  reads as gated on *something* time-shaped, not confirmed to be the same
  `1.0`-second edge Pulse's `Autopilot_Update` uses.
- **Whether `SpeedupPad`'s positional branch reads the same emitter offset**
  Pulse's `ExhaustFlare_OnSpeedupPad` does (`craft+0x50`) - the call passes
  `*(entity+0xd4)`, a different offset consistent with the struct-drift
  pattern throughout this comparison, not independently confirmed to be the
  same *kind* of field.
- **`Blowup` and `LockOn`**, the two remaining dry-play `Cue` variants - did
  not turn up among this chain's nine call sites, so either they reach it
  through an unfound call site or use a different path. Still open.
- **Runtime verification.** No PPSSPP leg for either binary.

## History

- **2026-09-04.** Written answering `every-sfx-trigger-is-a-pulse-reading-applied.md`'s
  `SpeedupPad` and `Disengaging` cues for Pure, found while chasing
  `shield-sound.md`'s dry-play chain.
