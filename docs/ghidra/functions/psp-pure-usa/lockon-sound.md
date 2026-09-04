# Pure's `HudSight_UpdateTone` opens the same `~ROCKLOCK` voice, the same way

**Binary:** `pure-psp` `BOOT.BIN`, image base `0x08804000`.
**Status:** decompilation only, no PPSSPP leg for either title.

Answers `oag_game::audio::sfx::Cue::LockOn`. Found the fast way: `search_strings`
directly for the cue's literal (`~ROCKLOCK`, matching `sfx.rs`'s `Cue::name()`),
then a `lui`/`addiu` search on the derived offset - no dry-play-chain caller
search needed, and for good reason: this cue calls the chain's *middle* hop
directly, not through the gate helper `dry-play-cues.md`'s nine call sites
were all found through.

## Pulse's `HudSight_UpdateTone` (`0x0881b34c`)

```c
void HudSight_UpdateTone(int param_1, int param_2)
{
  param_2 = param_2 * 4;
  piVar3 = (int *)(&DAT_002aca54 + param_2);
  iVar2 = *piVar3;                              // 0 = off, 1 = seeking, 2 = locked
  if ((iVar2 == 0) && (*(int *)(param_2 + 0x2aca5c) != 0)) {
    func_0x0018dd94(*(undefined4 *)(param_1 + param_2 + 0x90));  // stop
    *(undefined4 *)(param_1 + param_2 + 0x90) = 0;
  } else {
    if ((iVar2 != 0) && (*(int *)(param_1 + param_2 + 0x90) == 0)) {
      uVar1 = func_0x00136768(_DAT_002bde10, _DAT_002bddec,
                               _DAT_00275ba4 /* "~ROCKLOCK" */, 0x400, 0, 0, 0);
      *(undefined4 *)(param_1 + param_2 + 0x90) = uVar1;
    }
    if (iVar2 == 1) { func_0x0018e034(handle, 0, 0); }        // seeking tone
    else if (iVar2 == 2) { func_0x0018e034(handle, 0, 1); }   // locked tone
    *(int *)(param_2 + 0x2aca5c) = iVar2;
  }
}
```

`func_0x00136768` is `FUN_0893a768` (`0x0893a768`, `shield-sound.md`) -
the **middle** hop of the dry-play chain, called **directly**, not through
its own gate helper `FUN_0883e9b0`. `sfx.rs`'s own doc comment already
records "a volume of `0x400` and no emitter argument" for this cue; the
`0/1/2` state read here is the same "which waveform is which is inference,
at 55" split `sfx.rs` documents for the seeking/locked distinction.

## Pure's `HudSight_UpdateTone` (`0x0881f67c`)

```c
void HudSight_UpdateTone(int param_1)
{
  if ((_DAT_00288a0c == 0) && (_DAT_00288a10 != 0)) {
    func_0x00031ea0(*(undefined4 *)(param_1 + 0x88));
    *(undefined4 *)(param_1 + 0x88) = 0;
  }
  if ((_DAT_00288a0c != 0) && (*(int *)(param_1 + 0x88) == 0)) {
    uVar1 = func_0x0002c8f0(_DAT_00288f3c, _DAT_00288f24,
                             0x2423e0 /* "~ROCKLOCK" */, 0x400, 0);
    *(undefined4 *)(param_1 + 0x88) = uVar1;
  }
  if (_DAT_00288a0c == 1) { func_0x00032628(handle, 0, 0); }
  else if (_DAT_00288a0c == 2) { func_0x00032628(handle, 0, 1); }
  _DAT_00288a10 = _DAT_00288a0c;
}
```

Line for line the same shape, off by the same one structural difference every
Pure/Pulse pair on this thread has shown - the state variable is a plain
global here (`_DAT_00288a0c`) rather than an indexed array slot the way
Pulse's `param_2`-scaled version is, so this reads as supporting one sight
instance rather than Pulse's per-index array; not chased further since it
does not change whether the cue fires. `func_0x0002c8f0` is `FUN_088308f0`
(`0x088308f0`, `shield-sound.md`) - Pure's own middle hop, called directly,
the identical choice Pulse's function makes to skip its own gate helper.
Volume `0x400` matches exactly.

`0x2423e0 + 0x08804000 = 0x08a463e0`, independently confirmed by
`search_strings` landing on that exact address, reads `~ROCKLOCK\0` -
matching `sfx.rs`'s literal and the disc's own `~ROCKLOCK` cue (2 waveforms,
confirmed present, `just wad sounds`).

**Renamed `HudSight_UpdateTone`, matching Pulse's own name.**

## Confidence

**82**: decompilation only (no runtime leg, no verified caller - blocked on
the `jal` wart), but corroborated by a near-exact structural match against
an independently-recovered Pulse function of the same name, including the
specific choice to bypass the gate helper both binaries otherwise use for
dry-play cues - the same evidence class the `Sound_Play`/`Scream_PlaySoundByName`
renames earned.

## What is not verified

- **`HudSight_UpdateTone`'s own caller**, blocked on the `jal` wart.
- **Whether the `0`/`1`/`2` state values mean the same thing on Pure** -
  read as matching Pulse's shape (off/seeking/locked) by analogy, not traced
  back to whatever sets `_DAT_00288a0c`.
- **The single-instance-vs-array difference noted above** - not chased past
  observing it.
- **Runtime verification.** No PPSSPP leg for either binary.

## History

- **2026-09-04.** Written answering `every-sfx-trigger-is-a-pulse-reading-applied.md`'s
  `LockOn` cue for Pure.
