# HD/Fury's engine sound is a per-team crossfade table, and nothing reads it yet

2026-09-23. An HD race plays music and SFX with no engine note under it:
`shiphd.bnk` has no `~ENGINE` cue, so the held voice
`crates/game/src/audio/sfx/engine.rs` opens on Pulse never opens on HD. A lane
started on this and was stopped after about five minutes when the team was
reprioritised to weapon visuals. What follows is everything it established.
No Ghidra renames were made (read-only calls only), and no repository file
changed.

## What is known

**`.xfx` is a crossfade table (string evidence, not a guess).** HD ships
`data\sound\xfship_<team>.xfx`, one per team: 12 files, all in
`PS3_GAME/USRDIR/DATA01.PSARC` except `xfship_det.xfx` in `DATA00.PSARC`.
Every file is 20,976 bytes except `xfship_feisar.xfx` at 18,876 (2,100 bytes
smaller, unexplained). The literal `"Crossfade system failed to initialize"`
sits in rodata beside the three `xfship_*.xfx` path templates (`0x007a3ab0`,
referenced from `FUN_002ff628`). That reads as layered engine samples
crossfaded against some drive value, the HD counterpart of Pulse's single
pitched `~ENGINE` voice. `docs/formats/hd-status.md` still lists `.xfx` as
unknown.

**Byte layout, read by eye off the hex only (not parsed, no coverage
measured):** magic `XFDX` (`58 46 44 58`), then `02 06 00 00`, then an
undecoded header, then what look like repeating ~48-byte records carrying
`0x01ff` (511, possibly Q.9 unity) and `0x3333`/`0x6666` (fixed-point
~0.2/~0.4). Consistent with per-layer curve control points; not confirmed.

**Functions reached on `ps3-hdfury-eu` (`/hdfury/EBOOT-ps3-hdfury-eu.elf`),
none renamed, all below the rename bar:**

| Address | What it appears to do |
| --- | --- |
| `0x002ff628` (`FUN_002ff628`) | Crossfade-system init: allocates a pool (`FUN_003141c8(uVar2, 8, 0x20, 4, 0x30)`), logs the failure string and returns -1 on failure; otherwise, for each item of a list (`_opd_FUN_0015dc28`), calls `FUN_002ff250(param_1, item+0x78)`. Hypothesis: `+0x78` is a team hash or id. |
| `0x002ff250` (`FUN_002ff250`) | 12-slot hash-keyed cache: builds `xfship_%s.xfx` (special-casing `det` and `feisar`), loads it through `FUN_0031f438` then `FUN_00312398`, caches and returns the handle. Three callers: `FUN_002ff628`, `FUN_000ddd58`, `FUN_000dfd90`. |
| `0x000ddd58` (`FUN_000ddd58`) | Ship construction (large, mostly zero-init). Near its end it resolves the team's `.xfx` handle and passes it to `_opd_FUN_002fcaa0(..., uVar17, param_1 + 0x17c6)`, storing it on the ship at `+0x17c6`. This attaches the table; it is not the per-tick law. |

## Open

- The per-tick pitch/volume law, if HD has one: whatever reads the ship's
  `+0x17c6` each tick. Not found.
- The `XFDX` byte layout: header fields, record meaning, the `feisar` size
  difference.
- Whether the table's layers name `c_*` cues in `shiphd.bnk` or raw sample
  ids. This is the link that confirms or kills the "layered samples
  crossfaded by speed" reading. The `c_*` cues were never listed; check
  `oag_formats::sblk` reads the PS3 byte order before trusting
  `oag-wad sounds` on it.

## Next Steps

1. Decompile `FUN_000dfd90` (`0x000dfd90`), the resolver's third caller and
   never opened. It is the most likely per-race or per-tick user of the
   handle.
2. Decompile `FUN_002fcaa0` (`0x002fcaa0`) and find the readers of ship
   `+0x17c6`. HD is PPC64 with a per-function TOC: read
   `docs/ghidra/functions/ps3-hdfury-eu/memory.md` and
   `docs/reverse-engineering/toolchain.md#ps3` before trusting a
   cross-function reference.
3. Byte-decode `XFDX` with coverage, into a new `docs/formats/` page and a
   row in `docs/formats/README.md`. Extract the twelve files again with
   `just unpack extract` off `hdfury-ps3-eu-dec.iso`
   (`DATA00.PSARC`/`DATA01.PSARC`, `data/sound/xfship_*.xfx`).
4. Wire it only once 1-3 give a recovered law. A Pulse-shaped `~ENGINE`
   stand-in on HD is the invention CLAUDE.md forbids.
