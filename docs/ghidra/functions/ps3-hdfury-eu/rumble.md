# Rumble: the pattern library and every call site

Binary: `/hdfury/EBOOT-ps3-hdfury-eu.elf`. Read 2026-11-04, decompiler and
disassembly, no live run. The whole picture is
[gameplay/rumble.md](../../../gameplay/rumble.md); Omega's copy is
[its page](../ps4-omega-eu/rumble.md).

| Address | Name | Conf |
| --- | --- | ---: |
| `0x00342598` | `Rumble_LoadLibrary` | 84 |
| `0x00341de0` | `Rumble_ParsePatternXml` | 84 |
| `0x00342418` | `Rumble_QueueOneShot` | 84 |
| `0x00342290` | `Rumble_QueueHeld` | 80 |
| `0x0033fa28` | `Rumble_QueueOnPad` | 84 |
| `0x0033fee8` | `Rumble_PlayOnLocalPads` | 80 |
| `0x00340a88` | `Rumble_StepVoice` | 84 |
| `0x003417e8` | `Rumble_ChooseVoices` | 84 |
| `0x00340080` | `Rumble_MixSmallMotor` | 84 |
| `0x00340158` | `Rumble_MixLargeMotor` | 84 |
| `0x00340588` | `Rumble_TypeName` | 80 |
| `0x0033e460` | `Pad_Update` | 78 |
| `0x000d10c8` | `Ship_UpdateContactRumble` | 82 |

## Two TOCs and thunks

The library's code runs with `r2 = 0x008bd3c4`, not the usual `0x008ad4d8`, so its
TOC reads resolve only with the right base, and game code reaches it through
stubs at `0x00677xxx` (`std r2`, `addis r2,r2,1`, `subi r2,r2,0x114`, `b target`):
`0x00677518` to `Rumble_QueueOnPad`, `0x00677048` to `Rumble_PlayOnLocalPads`,
`0x006778d8` to `Rumble_QueueOneShot`, `0x00677908` to `Rumble_QueueHeld`.

`Rumble_PlayOnLocalPads` with `g_PhysicsHalfStep` clear queues on one pad with no
position, as on Omega; the voice takes the default vector at `*0x008b5914`, in
uninitialised memory in the image, so zero at start.

Four entry points, 42 call sites, listed with what each fires on
[gameplay/rumble.md](../../../gameplay/rumble.md#triggers). A stub has one target,
so that enumeration is complete unless a site loads the target by address; that
is why "never fired" below is scored 75, not higher.

## Library

Same library as Omega, with 32-bit layout: manager `+0x40`, stride `0x890`, seven
pads, library at pad `+0x40`, voices `0x30` bytes (`+0x00` active, `+0x04` pattern,
`+0x08` release flag, `+0x10` position, `+0x20`, `+0x21` output, `+0x24` frame),
ring of 20 at library `+0x120`, slots at library `+0x18 + 4*id`.

`Rumble_LoadLibrary` stores each pattern at library offset `0x18 + 4*id`. All 64
ids match Omega's, with 11 empty (`landing_very_light` at `+0x40`, `pad_speed` at
`+0x48`). **The table at `0x00923ba0` is not this**: seven 64-entry lists of the
same paths in forward-slash form, read by a debug file loader (`FUN_0031f9f8`).
Its indices run one lower than the ids from 12 up; do not use them.

`Rumble_ParsePatternXml` opens with `RumbleApp`, `RumbleData` and six sub-parsers,
one per field, as on Omega. `Rumble_TypeName` is a 65-way table with
`UNKNOWN RUMBLE TYPE` above 64.

## The step

`Rumble_StepVoice`: the same frame, loop and release law as Omega, and the same
gate: outputs clear when `(class & *0x008b5924) == 0`, or when `g_GamePaused` is
set and `class != 1`. **Positional gain is unread**: the decompiler truncates that
branch at bad instruction data, so a positional voice's law here is Omega's by
lineage.

`Rumble_ChooseVoices`: the same five-of-twenty rule as Omega, ties included.

`Rumble_MixLargeMotor` sums the five large bytes and clamps at 255.
**`Rumble_MixSmallMotor` is an OR: any non-zero small byte gives 1.** `Pad_Update`
stores `large * 1/255` at `pad+0x58c` and the small value **unscaled** at
`pad+0x588`, so 1.0 means fully on. This fits `cellPadSetActDirect`, whose small
motor is on or off. Both writes are skipped by the `ActuatorSmall`,
`ActuatorLarge` values in the device table at `0x00924780`, both 1. `pad+0x87c`
zeroes both; its writer is unread.

`cellPadSetActDirect` is called from `FUN_005b9f60`, the seven-pad driver.

## Not this library

- **`FUN_000a3528(0.07, 1.0, ...)` and `(0.06, 0.3, ...)`**, called "Rumble" on
  [plasma.md](plasma.md): writes float slots in a per-player block and takes a
  random number, and never reaches a pad or the ring. Its other callers are
  `Ship_ApplyDamage_q` and the like. It has the shape of a **camera shake**.
  Confidence 70 on "not controller rumble"; 60 on what it is.
- **`FUN_006772f8`**, on [xfade.md](xfade.md): a stub to `0x00623408`, which writes
  a byte into a 32-entry table at `*0x008c0038`. No pad is involved.
