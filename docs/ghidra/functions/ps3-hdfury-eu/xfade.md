# The crossfade system: HD's engine sound, found and half-driven

2026-10-04. Lane `hd-engine-xfade`. The format side is
[`hd-xfx.md`](../../../formats/hd-xfx.md); this page is the code that reads it.
Read [memory.md](memory.md) first for the per-function TOC defect. **Every
function here is below `0x32d5e0`, where `0x008ad4d8` is the only TOC in use**
([race-manager.md](race-manager.md) has the boundary), so Ghidra's references
resolve correctly. `FUN_000d5968`'s constants were additionally read from its
own `.opd` entry (`0x008749c0`: `000d5968 008ad4d8`) rather than from the
decompile.

## The names

| Address | Name | Confidence |
| --- | --- | --- |
| `0x00312660` | `XFadeSystem_AddCrossFader` | 90 |
| `0x00313910` | `XFadeSystem_StartTrigger` | 72 |
| `0x00313d10` | `XFadeSystem_UpdateChannels` | 72 |
| `0x00314b00` | `XFadeSystem_UpdateLayers` | 72 |
| `0x00314618` | `XFadeSystem_SetInput` | 72 |
| `0x003164c8` | `XFadeSystem_Update` | 70 |
| `0x002ff250` | `XFadeShip_GetTeamTable` | 78 |
| `0x002ff628` | `XFadeShip_InitSystem` | 70 |
| `0x002fcaa0` | `XFadeShip_CreateSound` | 60 |
| `0x000d5968` | `Ship_UpdateEngineCrossfade` | 82 |

Scores stop at 90 and mostly sit near 72 because none of this has run under an
emulator: it is static reading, corroborated by the file format
([hd-xfx.md](../../../formats/hd-xfx.md)) agreeing with the structure the code
walks.

## `XFadeSystem_AddCrossFader` (90)

Seven error strings carry the function's own name
(`XFadeSystem::AddCrossFader: Not a crossfader file`, `Incorrect Crossfader file
version`, `Incorrect target platform in data`, `... higher maxElementsPerHandle
(%d)`, `... maxControllerPerHandle (%d)`, `Unknown data type found in xfader
element %d`, `Out of handles`, `0x007a4610..0x007a4808`), and the pointer table at
`0x008b5064` lists them in the order the code reaches them. The function checks
`XFDX`, then `word & 0xffffff == 0x060000`, then `word >> 24 == 2`, relocates the
channel and layer offsets to pointers the first time, finds a free handle in a
`0x10`-stride table and gives it `0x50` bytes of channel state per channel and
`0x80` bytes of slot state per layer. That is every field
[hd-xfx.md](../../../formats/hd-xfx.md) reads. The thin wrapper at `0x003143c0`
passes the global system `0x00b6e308`.

The system is initialised by `FUN_003141c8(sys, 8, 0x20, 4, 0x30)` from
`XFadeShip_InitSystem`. Its body stores the arguments to `sys+0x14` (maximum
handles, 8: the eight craft of a race), `sys+0xc` (`maxElementsPerHandle`,
`0x20`), `sys+0x10` (`maxControllerPerHandle`, 4) and `sys+0x20` (maximum
triggers, `0x30`), and sizes the handle, element and controller arrays from
them. The loader compares `sys+0xc` against the file's element count and
`sys+0x10` against its controller count, which is how the mapping reads off
the error strings.

## The per-frame path

`XFadeSystem_Update` (`0x003164c8`; an identical copy at `0x003165a8` passes the
system through a global instead) calls, for every live handle,
`XFadeSystem_UpdateChannels` then `XFadeSystem_UpdateLayers`:

- **`UpdateChannels`** moves each channel's state toward the target
  `XFadeSystem_SetInput` stored, at a rate picked by the band the value is in
  (`u16` edges at channel `+0`, `i32` rates at `+8` rising and `+0x18` falling,
  multiplied by the frame's millisecond delta, capped at 5000). A rate of zero
  snaps. It also runs a random jitter (`+0x30..+0x48`) that every shipped file
  leaves at zero.
- **`UpdateLayers`** takes `(state + jitter) >> 16`, clamps to `0..=511`, and
  reads the layer's two curves at that index: gain from `+0x1c`, pitch from
  `+0x20`. Volume is `gain * slot1 * slot2` over `2^20`; pitch is
  `((pitch - 0x200) * 0x7fff >> 9) + slot offsets`, clamped to `+-0x8000`. It
  then starts or retunes a voice (`FUN_0031c948` / `FUN_0031c338`,
  `FUN_0031c8b0`) whose first argument is the cue the layer names.
- **`SetInput`** (`0x00314618`): `inputs[channel] = value * channel[+0x50] +
  (channel[+0x54] << 16)`.
- **`StartTrigger`** (`0x00313910`) reads a trigger by `(channel << 16) | index`
  (error `Trigger ID %d does not exist`), and returns without a voice if the
  trigger has no name (`+0x2b`) and no cue index (`+0x3c`): every shipped
  channel-0 trigger is that.

## The ship side

- `XFadeShip_GetTeamTable` (`0x002ff250`) is a 12-slot cache keyed on a team
  hash; it builds `xfship_%s.xfx` (the strings sit at `0x007a3ab0`, with the
  `det` and `feisar` special cases for the `DATA00` file and the layer count)
  and returns the handle. Callers: `XFadeShip_InitSystem` (all teams up front),
  `FUN_000ddd58` and `FUN_000dfd90`, the two ship constructors.
- `FUN_000dfd90` (`0x000dfd90`, a ship constructor) resolves the team table at
  `0x000e14ec` with `XFadeShip_GetTeamTable(sys, *(ship[0x18a5] + 0x78))`
  (`ship+0x6294` is the team record), calls
  `XFadeShip_CreateSound(…, &ship[0x17c6])`, then `FUN_002fd808(h, 2, 2)` and,
  for `ship+0x628c == 0` only, `FUN_002fd7f0(1.0, h, 1)`. `FUN_000ddd58` does the
  same at `0x000df4b8`. **Nothing else in the binary calls `FUN_002fd7f0` or
  `FUN_002fd808`** (xrefs: those two constructors and an `.opd` slot), so after
  construction the instance is driven only through `SetInput`.
- `XFadeShip_CreateSound` (`0x002fcaa0`) allocates a `0x70` object
  (`FUN_002febb0`), links it at the head of a list on its owner
  (`+0x7c` head, `+0x78` count) and stores it through the out-pointer. The
  handle `SetInput` addresses is in that object at `+0x54`.

## `Ship_UpdateEngineCrossfade` (65): the per-tick law

`0x000d5968`, called from `FUN_000eadb8` at `0x000ebee4` and `0x000ec750`. For a
ship whose `+0x5f18` (ship `+0x17c6`) is set:

```text
speed_field = 3.6 * (*(ship+0x6944))->f[0x4c4]
X           = (*(ship+0x5fac))->f[0x260]
ctrl        = *(*(ship+0x5fac) + 0x84)           ; may be null

ship+0x5f20 = clamp(trunc(0.5 * speed_field + 5.0 * X), 0, 511)   -> SetInput(h, 0)
ship+0x5f2c = clamp(trunc(speed_field * 0.01 * ctrl.f[2]), 0, 511) -> SetInput(h, 1)   ; ctrl non-null
ship+0x5f30 = clamp(trunc(speed_field * 0.01 * ctrl.f[3]), 0, 511) -> SetInput(h, 2)   ; ctrl non-null
ship+0x5f24 = clamp(trunc(5.12 * ctrl.f[1]), 0, 511)               -> SetInput(h, 3)   ; ship+0x628c == 0 only
```

`ship+0x628c` is the craft's role (0 the local player; the update treats 1 and
2 as other roles). Channel 3 is computed only for role 0 and not in the modes
whose bit is set in `0x206040` (bits 6, 13, 14, 21 of the mode id at
`+0xe0` of the race record). Roles 1 and 2 never write channel 3. (Role 1 loads
`100.0` into the same register and then skips the write, so that constant is not
used.) Channels 0, 1 and 2 are written for every role that has a `+0x5f18`.

Constants read from TOC `0x008ad4d8`: `3.6` (`-0x4770`), `0.5` (`-0x49d4`), `5.0`
(`-0x4890`), `0.01` (`-0x49dc`), `5.12` (`-0x4760`), and the clamp is the `0x1ff`
compare at `0x000d5abc`.

Beside the four writes the function does two other things, noted so nobody
re-derives them:

- It arms `ship+0x5f1c` when `4.0 < X < 4.5` and, once armed and `X` leaves the
  band, fires one channel-0 trigger by band (`X <= 2.5`: 2; `<= 3.0`: 1;
  `<= 3.5`: 0; `>= 4.8`: 3; `>= 5.5`: 4, `>= 7.0` and `< 10.0`: 5). The
  `>= 5.5` and `>= 7.0` arms are unreachable after the `>= 4.8` test. Every
  channel-0 trigger is unnamed on disc, so this plays nothing.
- It steps through `speed_field` thresholds `200..1000` calling `FUN_006772f8`
  with ids `0x14..0x16`, with a strength `trunc(speed_field * 0.127065)`. That
  is not audio; the shape (a pad id, a strength) reads as controller rumble.
  Not investigated.

### Live check (2026-10-04) and why 82, not higher

`scripts/rpcs3-hd-engine-xfade-probe.py` paused RPCS3 twelve times in a driven
Campaign race and read the stored channels beside their inputs. Channel 0
equals `trunc(0.5 * speed_field + 5.0 * X)` on 11 of 12 samples (the twelfth,
mid-acceleration, is 2.1 under),
channel 3 is `511` at throttle `100` and `0` at throttle `0`, and
`ctrl[+4]` is that throttle (`100.0` / `0.0`). The table is in
[hd-xfx.md](../../../formats/hd-xfx.md#the-per-tick-law-confidence-85-for-channels-0-and-3).
What keeps it from 90: channels 1 and 2 never left `0` in the capture, so their
two terms are read but unobserved; and `X` is a measured number with no
identified source.

`*(ship + 0x5fac)` is the body entry itself (first word `g_CraftVtable`
`0x008636e0`, vtable at `+8` is `0x008745b8`), so `X` is `entry+0x260`.

## What closes the lane

**`X` (`+0x260` of the body entry at ship `+0x5fac`).** `search_instructions` for
`stfs`/`lfs` at `0x260(` finds one writer on a non-stack base
(`FUN_000be530`, an XML tuning loader for a different, `0x1c8..0x26c`-field
class) and readers at `FUN_000d5968`, `FUN_000e7760`, `FUN_00109350`,
`FUN_0002fab0`. The entry is `0x3a0` bytes, built in `FUN_000dfd90` by
`FUN_006762b8(0x3a0)` and `FUN_000f00e8`. Live, `X` tracks `entry[+0x354]`,
`[+0x364]`, `[+0x368]`, `[+0x36c]`, `[+0x370]` plus a constant 1.12 (r above
0.994, twelve samples). The writer is not an `stfs` at `0x260(rN)`, so it is a
vector store or a copy; the RPCS3 write watchpoint patch
(`just build-rpcs3-watchpoints`) on `entry+0x260` would name the writing
instruction in one session. The vtable's slots (`0x00682b68`...) are the other
route and were not read.

**The pitch unit** needs the voice function chain below
`FUN_0031c948` (`FUN_006796b8` takes the pitch) or one pair of measured
pitch-versus-input samples.
