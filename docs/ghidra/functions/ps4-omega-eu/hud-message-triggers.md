# What raises Omega's HUD message lines, and its time gap

**Binary:** `ps4-omega-eu` `eboot.bin`, image base `0x01000000`, Ghidra
`/omega/eboot-ps4-omega-eu.bin`. Read 2026-10-12 (`hud-messages-port` lane) from
the disassembly (`disassemble_bytes`); Omega's executable **is** statically
checkable (x86-64, no custom processor module), which the earlier threads had
recorded as "not checkable". Companion to HD's
[hud-message-triggers.md](../ps3-hdfury-eu/hud-message-triggers.md) and 2048's
[hud-message-triggers.md](../vita-2048-eu-v104/hud-message-triggers.md).

**A 2048 finding is checked against Omega and the reverse.** Result: **ported**
for both routines, **checked, applies, not wired** for the split table.

## Where it is

Omega builds its strings from immediates, so the ids have no data xrefs
(`IG_HUD_PLAP` is `mov rcx,0x505f4455485f4749` followed by `"LAP"`); the way in is
a neighbouring literal with a real reference. `RACE_Perfect_lap` (`0x018118bf`)
has them, at `0x0126a10a` and `0x0126a7af`, inside the function at `0x0125e9b0`:
a 55 KB `Hud_Update` with the messages and the gap inlined (`0x0125e9b0` to
`0x0126c018`). Unnamed (`FUN_0125e9b0`); no `names.tsv` row.

## The drain (`0x01269f64`-`0x0126a1af` and on)

`mov ebx,[rax+0x138]; mov [rax+0x138],0` with `rax = [r13+0x78]`: the flag word at
**status `+0x138`**. Slots are `0x18` bytes from `hud+0x198` (a heap string
pointer, the timer at `+0x1a0` set to `0x42c80000` = `100.0`, the busy byte at
`+0x1a4`, the good byte at `+0x1a5`); four of them, found by the busy bytes
`+0x1a4`/`+0x1bc`/`+0x1d4`/`+0x1ec`. HD's drain with the offsets moved:

- `test [r13+0x80],0x20` is HD's `hud+0x44 & 0x20` voice-only branch (to
  `0x0126a792`);
- bit `0x1` pushes `IG_HUD_PLAP` (`0xc` bytes) and plays the cue `RACE_Perfect_lap`
  (`0x018118bf`) when `byte [0x019405a0] < 2` or `byte [0x02037d00] == 0`;
- bit `0x2` pushes `IG_HUD_NEWLAP_REC` (`0x12` bytes) **unless
  `dword [0x01f999e4] == 5`** (Time Trial), with no profile-byte test;
- the later bits follow in the same function (`IG_HUD_CONT_ELIM` is referenced at
  `0x0126a27d`, `0x0126a302` and `0x0126a3ad`).

The remaining bits, the slot timers and the colours sit further down the same
function and were not read line by line: nothing here contradicts HD's. The ids
are the title's own and the layouts' `Info1`-`Info4` exist (`message_slots` was
already on).

## The gap (`0x0126812a`-`0x01268398`)

`TimeDiffText` is `hud+0x2f8` and `TimeDiffIcon` `hud+0x300` (the immediate
`"TimeDiff"` + `"Text"` at `0x0126fbc9`, stored at `0x0126fbf2`; `"Icon"` stored
at `0x0126fc83`). The update is HD's `Hud_UpdateTimeDiff`:

- `[0x01f999e4] == 10` or `== 5` (Speed Lap, Time Trial): the count-up runs on the
  leading flag `+0x11c` alone, **no live-craft count** (HD tests the same two
  modes behind a profile byte; Omega has no such test); any other mode needs
  `FUN_01297ea0(...) > 1` craft live;
- the count-up is `"%.1f"` under `[0x01800be4]` = `0x41f00000` = 30.0, colour
  `0xff60ff60`;
- the held gap uses `+0x11d` (holding), `+0x11e` (ahead), the timer `+0x124`,
  `(int)(timer * [0x01800c0c])` with `0x40a00000` = **5.0** Hz for the flash, the
  four colours `0xff10c010`/`0xff30ff30` (ahead) and `0xffc01010`/`0xffff3030`
  (behind), a hold limit `[0x01800cfc]` = `0x40600000` = **3.5** s, the draw range
  `+/-30.0` (`0xc1f00000` at `0x01800c9c`);
- **no place read**: the green hold ends at 3.5 s only.

Confidence 85 for each constant, 80 for the whole as "HD's routine".

## Not read

The split table and its writers (HD's `Craft_RecordSplit`, `Craft_UpdateLapProgress`
and their thirds). `0x3eaaaaab` is used in dozens of places, so a constant search
does not find them; the table is wired at **one split a lap** and a 100-crossing
size, **chosen, not measured, from HD**.

## Wired

`oag_omega::hud::ART.lap_hud` (`oag_title::lap_hud`): the three ids,
`record_line_in_time_trial: false`, `MessageTint::GoodBad`, the gap rule as above
with `splits_per_lap: 1`. Racing is still out of scope for Omega, so nothing
disc-backed drives it; HD's tests carry the machine and the data is the only
Omega-specific part.
