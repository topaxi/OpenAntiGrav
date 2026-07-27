# Pulse PSP functions

Functions from `PSP_GAME/SYSDIR/BOOT.BIN` (Wipeout Pulse, PSP, UCUS-98712),
image base `0x08804000`, language `Allegrex:LE:32:default`.

An unencrypted ELF, so no decryption step is needed. See
[workflow](../../workflow.md), and read
[Allegrex and the VFPU](../../../psp/allegrex-vfpu.md) first.

| Page | Covers |
| --- | --- |
| [WAD subsystem](wad-subsystem.md) | Name hash, lookup, mount, read, LZSS and zlib decoders |
| [Main loop](main-loop.md) | Frame pacing, timestep, state machine |
| [Input](input.md) | Pad polling, the abstract button layer, deadzone and gain |
| [Collision](collision.md) | Triangle soup, sweep and prune, surface types |
| [Camera views](camera.md) | The player-selectable in-race views and how SELECT cycles them |
| [Engine, brakes, steering, pitch](engine.md) | The craft update frame, every control force term, and the complete handling parameter block |
| [Video and the Movie widget](frontend-video.md) | Intro sequence, sceMpeg playback, XML-driven playback, the START skip |
| [Import stubs](imports.md) | The 335 library calls, 306 of them resolved by NID |

## Renames

**Applied.** 426 symbols: 120 from the pages above, collected in
[names.tsv](names.tsv), plus 306 import stubs derived from the binary itself.

```sh
just apply-names        # into whichever program the Ghidra bridge has open
```

The Ghidra database is not committed, so that command is how a fresh import
gets the names back.
[`scripts/apply-ghidra-names.py`](../../../../scripts/apply-ghidra-names.py)
enforces the two rules of
[ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md) rather than
trusting them: it refuses any row whose address and name are not both still on
its evidence page, and it applies the `_q` suffix below 70 confidence itself.

Nothing in this set scored below 70, so no `_q` names exist yet.

## Highest-confidence findings

| Address | Name | Conf | Why it matters |
| --- | --- | ---: | --- |
| `0x08940d0c` | `Wad_HashName` | 97 | Unblocked all asset work; entry names are recoverable |
| `0x08807244` | `Game_MainLoop` | 95 | Names itself via a profiler string |
| `0x08804978` | `Game_UpdateFrame` | 92 | Showed the timestep is variable, not fixed |
| `0x089411e8` | `Wad_Open` | 93 | Hash plus rotating linear scan |
| `0x0894f1cc` | `Input_IsPressed` | 92 | Abstract button layer, fully mapped |
| `0x088ba284` | `Movie_ParseAttributes` | 95 | The front end is XML-driven |
| `0x0883a2f0` | `Handling_ParseStats` | 88 | Placed all 32 handling parameters and found four are pre-scaled at load |
| `0x08849618` | `Ship_UpdateCraft` | 82 | The craft frame: what order the force terms run in, and which read stale groundedness |
| `0x0883c0cc` | `Camera_UpdatePlayerView` | 88 | The three player views, the SELECT cycle, and the sign convention of the on-disc camera offsets |
