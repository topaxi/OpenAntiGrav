# Camera views (PS2)

Functions in `SCES_547.48` (Wipeout Pulse, PS2, SCES-54748), image base
`0x00100000`.

The PSP counterpart is [psp-pulse-usa/camera.md](../psp-pulse-usa/camera.md), which
established the three player views, the SELECT cycle and the sign convention of
the on-disc offsets, most of it against the running game. This page records the
PS2 equivalent and confirms the cycle in a second binary.

**The names below are applied**, from [names.tsv](names.tsv).

| Address | Name | Conf |
| --- | --- | ---: |
| `0x0014f208` | `Camera_UpdatePlayerView` | 88 |
| `0x002db350` | `g_settings` | 78 |

The parameter blocks themselves are parsed by the five
`HandlingXml_Parse*Camera` functions, documented in
[handling-xml.md](handling-xml.md); all 27 floats sit at the same offsets as on
PSP.

## The SELECT cycle is the same, in the same order

`Camera_UpdatePlayerView` (`0x0014f208`) opens with the same three steps as the
PSP function:

```c
setting = Camera_GetViewSetting(playerIndex);       /* 0x0014f108 */
if (Input_IsPressed(g_input, playerIndex, 0xf, 0)) {  /* 0xf == SELECT */
    Input_ConsumePress(g_input, playerIndex, 0xf);
    *(u32 *)(g_settings + 0x45c) = 1;                 /* settings dirty */
    if      (streq(setting, "OPT_INT"))   next = "OPT_CLOSE";
    else if (streq(setting, "OPT_CLOSE")) next = "OPT_FAR";
    else if (streq(setting, "OPT_FAR"))   next = "OPT_INT";
    else                                  next = setting;
    Settings_SetString(g_settings, hash("Camera"), next, strlen(next) + 1);
}
```

**`OPT_INT` -> `OPT_CLOSE` -> `OPT_FAR` -> `OPT_INT`**, wrapping, on button
index `0xf`, with the press consumed and a dirty flag set on the settings
object. The string literals are at `0x002a5c98`, `0x002a5c80` and
`0x002a5c90`; the `"Camera"` key is at `0x002a5cb0`. The dirty flag is at
`g_settings + 0x45c` here against `+0x45b` on PSP, and is a word rather than a
byte.

The three views then select a rig by name and set the hide-own-ship flag:

| Setting | Rig | Cached index at `controller+0x3c` | Craft flag |
| --- | --- | ---: | ---: |
| `OPT_INT` | (internal; no `printf`-built name) | 0 | `+0xf4 = 1` |
| `OPT_CLOSE` | `"player%d external close tripod"` | 1 | `+0xf4 = 0` |
| `OPT_FAR` | `"player%d external far tripod"` | 2 | `+0xf4 = 0` |

and `+0xf4` is copied to `+0xfc` immediately afterwards - the same
write-then-mirror pair the PSP page found at `craft+0x6d` / `craft+0x6f`, with
the same "1 only for the internal view" correlation. Two builds agreeing on
that pattern raises the PSP page's confidence-70 reading of it as "hide the
player's own ship".

The rig names are `printf`-formatted with the player index, which the PSP's
fixed `player_internal_tripod` is not. That is the split-screen difference
showing through, not a behavioural one.

Confidence **88** for the cycle: the three literals and their rotation are
unambiguous in the decompilation and identical to the PSP reading. Not higher
because nothing on the PS2 side has been observed running.

## The chase distance is computed here, and it is not a constant 3/4

The PSP page's largest open question is that both external offsets reach the
eye at exactly **0.75** of their authored value, from a source it could not
find. The PS2 function does its distance work inline and visibly, which is a
lead rather than an answer:

- it takes the ship-to-eye vector, normalises it (`vrsqrt`), and keeps the
  length `L`;
- it forms `d = min(L - 1.0, 3.0)` and probes along the ray with a trace
  (`0x00132ed0`), shortening `d` when geometry is hit;
- it low-passes the result into a per-player global at `0x0027e8b0` with
  `x += (target - x) * 0.5` each frame;
- and it applies a vertical lift of `(7.5 - d) * 0.5` when `d < 7.5`.

None of `0.75`, nor a multiply by three quarters, appears anywhere in it. So
**the PS2 does not reach its eye position the way the PSP page assumed the PSP
does**, and the 3/4 factor is not a shared constant sitting in the camera code
of both. Whoever picks that question up should treat the PS2 path as a
different algorithm rather than as a second copy of the same one. Confidence
**70** on that negative: the function was read, not run, and the trace at
`0x00132ed0` was not decoded.

## Not determined

- **`0x0014f108`**, which returns the current view setting, reading either from
  `g_settings` or from a per-player array at `0x0027e8a8` depending on a global
  at `0x002dab3c` (which reads like an attract/demo switch). Not renamed: the
  branch is clear but what selects it is not.
- **The rig lookup** at `0x00149450` and the trace at `0x00132ed0`.
- **The second, spectator/photo camera enum** that the PSP page warns not to
  confuse with this one. Not searched for here.
- **Nothing here was verified at runtime.**

## Cross-platform

| Function | PS2 (`SCES_547.48`) | PSP (`BOOT.BIN`) |
| --- | --- | --- |
| `Camera_UpdatePlayerView` | `0x0014f208` | `0x0883c0cc` |
| `g_settings` | `0x002db350` | `0x08b31774` |
| `Camera_SetMode` | not located | `0x08880724` |

## History

- 2026-07-27: first pass. Cycle 88 from an exact match with the PSP reading;
  the 3/4 chase factor recorded as absent from the PS2 path.
