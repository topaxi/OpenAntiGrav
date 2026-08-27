# Check whether 2048 is the HD/Fury codebase retargeted, not a Pulse-lineage build

2026-08-26. User hypothesis, raised while naming `Game_Main`: WipEout 2048
(Vita) ships HD/Fury content as DLC, so it may share a codebase with
`ps3-hdfury-eu` rather than being a fresh build off the PSP/PS2 Pulse
lineage this project's other RE work is anchored to.

**One piece of independent corroboration already found, not sought
deliberately**: `Game_Main` (`0x81003dd2` in `vita-2048-eu-v104/eboot.elf`)
sets the window/session title to the literal string `"WIPEOUT_HD"` - not
`"WIPEOUT_2048"` - via `FUN_8106878e(iVar2, pcVar4, iVar2 + 0x1f8)` where
`pcVar4` is a boot-mode string ("Main Menu", "RaceBox", etc.) but the first
argument's string is fixed. See
[game-boot.md](../docs/ghidra/functions/vita-2048-eu-v104/game-boot.md) for
the full call. That is a real, if narrow, data point: whatever generated
this window title did not get a fresh string for the 2048 SKU.

Nothing else has been checked yet. This is a hypothesis with one supporting
observation, not a conclusion.

## Open

- No function in `vita-2048-eu-v104` has been compared address-by-address or
  shape-by-shape against `docs/ghidra/functions/ps3-hdfury-eu/` yet - PS3 is
  PowerPC64 and Vita is ARM Thumb-2, so any match would have to be at the
  level of call structure, string content and constant values, never
  identical machine code or address.
- `ps3-hdfury-eu`'s own `names.tsv` has 107 rows (`docs/ghidra/functions/ps3-hdfury-eu/`)
  - a real body of named functions to check against, not a cold start.
- If the hypothesis holds, it changes where to look first for *every* future
  2048 name, not just this one. **2026-08-27: the boot-manager naming thread
  went ahead without waiting on this one** - `Memory_Alloc`, `Heap_Alloc`,
  `SystemRoot_Construct`, `SpeechManager_Construct`, `SoundManager_Construct`,
  `FrontendRoot_Construct` and `MusicManager_Construct` are named in
  [game-boot.md](../docs/ghidra/functions/vita-2048-eu-v104/game-boot.md) on
  single-binary evidence only, at confidence 80-90. If this hypothesis is
  later confirmed, revisit those seven against `ps3-hdfury-eu`'s `names.tsv`
  for cross-verification - a same-role match there would raise confidence,
  not change any of the names, since none of the seven were guessed off an
  assumed HD/Fury shape.

## Next Steps

- Pick 2-3 of `ps3-hdfury-eu`'s already-named functions with a distinctive,
  language-independent signature (a fixed set of string constants, a
  distinctive constant table, an unusual argument count) and search
  `vita-2048-eu-v104` for the same strings/constants via `search_strings`/
  `search_byte_patterns` rather than trying to match code shape directly.
- `GameRoot` itself is a good first target either way: check whether
  `ps3-hdfury-eu` has a same-named or same-role singleton (a
  `"GameRoot.cpp"`-tagged constructor, or an equivalent global) already
  documented.
- If the lineage holds up, record it as a real project fact (worth its own
  line in `docs/ghidra/functions/README.md` or a short ADR) rather than
  leaving it scattered across handover threads - this project's threads are
  deliberately deleted once their work lands, so a conclusion worth keeping
  needs a permanent home.
- If it doesn't hold up (2048 turns out to share nothing structural with
  HD/Fury beyond the DLC business relationship), say that explicitly and
  close this thread rather than leaving it to be re-asked later.
