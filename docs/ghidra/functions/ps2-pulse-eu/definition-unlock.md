# `Definition_IsUnlocked` and the dev-unlock word on PS2 Pulse (2026-11-11, `gallery-pulse-ps2`)

**Binary:** `SCES_547.48` (`/pulse/SCES_547.48-ps2-pulse-eu`), image base `0x00100000`. The reading is the PSP's,
[`../psp-pulse-usa/race-box-screens.md`](../psp-pulse-usa/race-box-screens.md), "`Definition_IsUnlocked` and the
two-axis unlock read"; this page is the PS2 counterpart and the live check that a profile offers every circuit.

## What the function is

`FUN_0017fa68` (`0x0017fa68`, `Definition_IsUnlocked`, confidence 85) is the PSP function with the same shape, line for line:

- `*(int *)(def + 0xa0) != 0` (PSP `*(char *)(def + 0x99)`) returns false: a definition switched off for good;
- with `param_2 != 0` the definition's own virtual at vtable `+0xd4` must also return non-zero (the same gate on both);
- **the dev-unlock word**: `*(int *)(g_settings + 0x46c) == 0` is the only way into the `<Unlock>` row walk. A non-zero
  word returns true before any row is read. The PSP reads `sceKernelGetGPI() & 1` or the byte at `*(0x08b31774) + 0x45f`;
  the PS2 has no GPI call, only the word;
- otherwise it walks the definition's `<Unlock>` rows (`*(def + 0xac)`, PSP `+0x9c`, built by the parser `FUN_0017fbf8`,
  which reads attribute names including `MedalCount` (`0x002aa918`) and `Loyalty` (`0x002aa938`), as the PSP's does): a row whose first word is
  zero ANDs its met-tests together, a row with a non-zero first word returns true on the first test that passes. The
  helpers (matched to the PSP's by their place in the walk, not by their bodies) are `FUN_0017f258` (the row applies), `FUN_0017f568`/`0017f660` (medal value and met), `FUN_0017f600`/`0017f8c8`
  (medal count), `FUN_0017f630`/`0017f918` (loyalty), `FUN_0017f4b8`/`0017f780` (grid points). Their names are not
  recorded: only the shape was read, not each body.

`g_settings` (`0x002db350`, [`camera.md`](camera.md)) holds a pointer: `0x002db350` read `0x00d941f0` on the boot below, so the
word is at EE `0x00d9465c`. It lives in the loaded profile object, so a reboot or a profile reload clears it, and a
savestate carries it.

## Live check

PCSX2 v2.7 (own data path, Xvfb), `pulse-ps2-eu.chd`, a memory card already holding a profile, Racebox, Single Race:

| Step | Track Select reads |
| --- | --- |
| word left at 0 | `1 / 3` (Talon's Junction White first) |
| `python3 scripts/pcsx2_pine.py write32 0xd9465c 1`, back out (triangle) and into Track Select again | `1 / 32` |

The list is built when Track Select is entered, so the write goes in **before** it, as on the PSP. All 32 entries then
step through (Talon's Junction, Moa Therma, Metropia, Arc Prime, Orcus, de Konstruct, Tech de Ra, Vostok Reef, The Amphiseum,
Fort Gale, Basilico, Edgewinter, Platinum Rush, Vertica, Gemini Dam, Outpost 7, White then Black each: sixteen circuits,
four of them (Orcus, Vostok Reef, Edgewinter, Gemini Dam) not on the PSP). `scripts/pcsx2-pulse-walk.py unlock` writes the
word; it reads `g_settings` first, so the address follows the boot. Confidence 90 for the word and the function's role: one
boot, the effect read off the screen, the shape equal to the PSP's, which was confirmed live on its own.

## Track Select's cursor

The cursor is a 0-based index in EE RAM (the screen's `n / 32` less one), found as the only two words that read 27, 28, 29 at
three savestates whose Track Select counter read 28, 29, 30: `0x0048dfa0` and `0x0048dfa4`. These are heap words of the
menu state `pcsx2-pulse-walk.py` loads (it never reaches the address in another boot), used there to press `down` until the
word reads the wanted entry: a `down` is lost about one time in four on a loaded machine, so counting presses does not land.

## Port check

PSP: the same function, the same dev word, confirmed live (ported, with its own evidence). 2048 and Omega: not applicable, a
Pulse-only function; their unlock machinery is a different build.
