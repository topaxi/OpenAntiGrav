# Pure's shield-activate function fires the same two cues Pulse's does

**Binary:** `pure-psp` `BOOT.BIN`, image base `0x08804000`.
**Status:** decompilation only, no PPSSPP leg for either title.

Scoped narrowly to the sound question `crates/game/src/audio/sfx.rs`'s module
doc asks about Pure and HD dispatch - not a re-derivation of the shield
subsystem itself, which [`psp-pulse-usa/shield.md`](../psp-pulse-usa/shield.md)
and [`shield-pickup.md`](../psp-pulse-usa/shield-pickup.md) already cover for
Pulse. Answers `oag_game::audio::sfx::Cue::Shield` and `Cue::ShieldActive`.

## Pulse's `Shield_Activate` (`0x0883e544`) fires two cues in a fixed order

```c
void Shield_Activate(int param_1)
{
  if (_DAT_002bddfc != 0) {
    func_0x0003a9b0(param_1, _DAT_002bddfc, _DAT_002777b4 /* "shieldactive" */, 0x400, 0);
  }
  func_0x00059e4c(*(undefined4 *)(param_1 + 0x8c8));
  func_0x001352b0(0x3f800000, *(undefined4 *)(param_1 + 0x50), _DAT_002bddf8,
                  0, _DAT_00277778 /* "~SHIELD" */, param_1 + 0x54);
  return;
}
```

Both string operands are indirect loads (`lui`/`lw`, a pointer-table slot -
the same shape the corrected `.COLLISIONS`/`ABSORB` reads used on Pulse),
read directly as memory rather than inferred: `_DAT_002777b4` resolves to
`0x08a7b7b4`, whose pointer (`0x002777a4` -> `0x08a7b7a4`) reads
`shieldactive\0`; `_DAT_00277778` resolves to `0x08a7b778`, whose pointer
(`0x00277770` -> `0x08a7b770`) reads `~SHIELD\0`. Both match `sfx.rs`'s
existing `Cue::name()` literals exactly - this page found no correction here,
unlike the `.COLLISIONS` one.

`func_0x0003a9b0` (`0x0883e9b0`) is a gated dry-play helper - Blowup's own
doc comment in `sfx.rs` already cites the same address for `~BLOWUP`, so it
is shared between the two cues on Pulse. `func_0x001352b0` is `Sound_Play`
(`0x089392b0`) called with a sixth argument (`param_1 + 0x54`, a handle
out-slot) - the same "looping" usage shape `Sound_PlayLooping` names
throughout `sfx.rs`, not a separately-named function.

## Pure's `FUN_0892425c` fires the same two cues in the same order

Found by searching Pure's disc data for the literal `shieldactive` string
first (`search_strings`, not arithmetic) - it resolves to `0x08a7a418`, and
`get_xrefs_to` on it (as expected, per the string wart) returns nothing.
Working backward from `off = 0x08a7a418 - 0x08804000 = 0x276418` and
searching for a direct `lui`/`addiu` pair building it
(`search_instructions`, `operand_pattern "0x6418"`) finds
`FUN_0892425c` (`089242d0: _addiu a2,a2,0x6418`), which also appears in
`Sound_Play`'s own 36-site caller list - the same cross-reference method
that found `FUN_08925e20` for `Absorb`.

**Decompiled in full:**

```c
void FUN_0892425c(int param_1)
{
  // ... previous-instance cleanup on entry, unread past its shape ...
  func_0x001209e0(param_1, _DAT_00288f2c, 0x276418 /* "shieldactive" */, 0x400, 0);
  // ... entity allocate-or-reuse for the shield's own audio-linked object ...
  func_0x0002dddc(0x3f800000, *(undefined4 *)(param_1 + 0xd4), _DAT_00288f14,
                  0, 0x276428 /* "~SHIELD" */, param_1 + 0xd8);
  return;
}
```

Both string operands here are **direct** `lui`/`addiu` builds, not indirect
table loads (matching Pure's `.COLLISIONS`/`ABSORB` codegen, not Pulse's for
this function) - `0x276418 + 0x08804000 = 0x08a7a418` reads `shieldactive\0`,
`0x276428 + 0x08804000 = 0x08a7a428` reads `~SHIELD\0`. Both addresses are
independently corroborated by Ghidra's own `search_strings` index landing on
the identical addresses, not solely by the addition. `func_0x0002dddc` is
`Sound_Play` (`0x08831ddc`, renamed in `rocket-and-collision-fx.md`), called
with the same sixth-argument handle-out-slot shape (`param_1 + 0xd8`) Pulse's
looping call uses. Volume constant `0x400` matches Pulse's exactly. **Order
matches**: the dry `shieldactive` call precedes the looping `~SHIELD` call in
both binaries, unconditionally in Pure and gated on `_DAT_002bddfc != 0` in
Pulse - the gate's own field is not read on Pure's side, so whether an
equivalent gate exists here is open.

**Both cues are confirmed present on Pure's disc** in `WEP_CLA`
(`just wad sounds "data/images/pure-psp-usa.chd:PSP_GAME/USRDIR/Data.wad"`):
`~SHIELD` (4 waveforms, 2 looping) and `shieldactive` (1 waveform, undotted,
matching the executable's own literal) both present, same bank as Pulse.

## `func_0x001209e0`'s own chain mirrors Pulse's `FUN_0883e9b0`, three hops deep on both sides

**Correction: this section originally called Pulse's dry-play helper "a flat,
one-hop forward to `Sound_Play`" and left Pure's three-hop chain as an
unresolved oddity. That was wrong - it never checked what Pulse's own helper
calls.** Decompiling it: `FUN_0883e9b0` gates on `craft+0x368 == 0` and
forwards into `FUN_0893a768` (`func_0x00136768` printed), which does
`*(param_1 + param_2*4 + 0x124)` - the identical bank-index expression
`Sound_Play` uses internally, at the identical offset the two share
(`_DAT_002bde10`) - and forwards *that* into `func_0x0018db20`, which
resolves to `0x08991b20`: **`Scream_PlaySoundByName`, already named on
Pulse's own disc** (`sound.md`, confidence 85). So Pulse's own dry-play path
is `FUN_0883e9b0` -> `FUN_0893a768` -> `Scream_PlaySoundByName`, three hops,
not one - matching `sfx.rs`'s own citation for `Disengaging`
("`FUN_0883e9b0` -> `FUN_0893a768`, the path that takes no emitter at all"),
which this page should have cross-checked before writing the wrong claim.

**Pure's chain is the same three hops, and the terminal one is now named.**
`func_0x001209e0` (`0x089249e0`) gates on `craft+0x488 == 0` (Pulse's `0x368`
- the usual struct-drift), forwards into `func_0x0002c8f0` (`0x088308f0`,
the same `_DAT + n*4 + 0x110` bank-index expression at Pure's own offset),
which forwards into `func_0x00031a90` (`0x08835a90`) - decompiled side by
side with `Scream_PlaySoundByName` above, the two are a near-exact structural
match: identical control flow, the identical `0x6b6c4253` (`"SBlk"`,
little-endian) magic check, the identical two-callee dispatch
(`func_0x00032a5c`/`func_0x00031d94` here against `func_0x0018e304`/
`func_0x0018dc70` there), the identical error-path shape
(`func_0x000db788` here against `func_0x0016e454` there). **Renamed
`Scream_PlaySoundByName`, confidence 82** - same evidence class as
`Sound_Play`'s own rename (structural match against a named Pulse
counterpart), one notch below it because the match, while exact in shape, is
still decompilation-only with no runtime leg on either side.

`func_0x001209e0` and `func_0x0002c8f0` stay unrenamed, matching Pulse's own
`FUN_0883e9b0`/`FUN_0893a768` - the project already chose not to name these
two on the binary where they were found first, so naming only Pure's copies
would be an inconsistency this page introduces rather than removes.

## Confidence

**80** for the cue-name and order finding (`FUN_0892425c` fires
`shieldactive` then `~SHIELD`, matching Pulse's call order and both
literals): decompilation only, corroborated by an independently-recovered
Pulse function of matching shape, both string addresses cross-checked
against Ghidra's own string index rather than arithmetic alone, and the
cues' presence confirmed on the disc. Not higher because `FUN_0892425c`'s own
call site is unverified (blocked on the `jal` wart, per
`rocket-and-collision-fx.md`'s trap section) and its extra entity-lifecycle
logic (absent from Pulse's simpler `Shield_Activate`) is unexplained - a real
structural difference, not assumed to be equivalent.

**82** for the `Scream_PlaySoundByName` rename, above.

## What is not verified

- **`FUN_0892425c`'s own caller**, blocked on the `jal` wart.
- **Whether Pure gates the dry `shieldactive` call the way Pulse does**
  (`_DAT_002bddfc != 0`) - Pure's call is unconditional in the read section
  above; the entity-lifecycle logic before it may or may not be an equivalent
  gate, not chased.
- **`func_0x001209e0`/`func_0x0002c8f0`'s own callers beyond the nine found
  for `SpeedupPad`/`Disengaging`/`ShieldActive`** - `sfx.rs`'s remaining dry
  cues (`Blowup`, `LockOn`) did not turn up among those nine, so either they
  reach this chain through a call site not yet found, or they use a different
  path entirely; see `dry-play-cues.md`.
- **Runtime verification.** No PPSSPP leg for either binary.

## History

- **2026-09-04, second pass.** Corrected this page's own claim that Pulse's
  dry-play helper is a flat one-hop call - it is the same three-hop chain
  Pure's is, ending in `Scream_PlaySoundByName` on both sides. Renamed
  `FUN_08835a90` to `Scream_PlaySoundByName` (confidence 82) off a
  side-by-side structural match. The same chain's other call sites turned up
  `SpeedupPad` and `Disengaging` too - written up in `dry-play-cues.md`
  rather than here, since this page stays scoped to `Shield`/`ShieldActive`.
- **2026-09-04.** Written answering `every-sfx-trigger-is-a-pulse-reading-applied.md`'s
  `Shield`/`ShieldActive` pair for Pure.
