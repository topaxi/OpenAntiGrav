# 2048's seven boot-manager names could be cross-verified against `ps3-hdfury-eu`, once that side has the same singletons named

2026-09-01. Follow-on from
[the lineage question, now confirmed](../docs/ghidra/functions/vita-2048-eu-v104/README.md#the-lineage-question-is-answered-confirmed):
`vita-2048-eu-v104` is HD/Fury's own codebase retargeted for Vita, not a
fresh Pulse-lineage build. That makes `ps3-hdfury-eu` a legitimate
corroboration source for any 2048 name going forward - but only for names
`ps3-hdfury-eu` already has.

`Game_Main` ([game-boot.md](../docs/ghidra/functions/vita-2048-eu-v104/game-boot.md))
constructs seven singletons in a row, all named on single-binary evidence
only, at confidence 80-90: `Memory_Alloc`, `Heap_Alloc`,
`SystemRoot_Construct`, `SpeechManager_Construct`, `SoundManager_Construct`,
`FrontendRoot_Construct`, `MusicManager_Construct`. None of the seven was
guessed off an assumed HD/Fury shape, so a same-role match on the PS3 side
would raise confidence, not change any of the names.

Checked directly: `ps3-hdfury-eu/names.tsv` has `FwMemAllocator_*` and
`FwMemHeap_*` (a plausible match for `Memory_Alloc`/`Heap_Alloc`, prefix
convention aside) but **no `SystemRoot`, `SpeechManager`, `SoundManager`,
`FrontendRoot` or `MusicManager` equivalent named yet**. The cross-check
needs fresh RE work on the PS3 side before it can happen at all, not just a
lookup.

## Open

- Whether `ps3-hdfury-eu`'s boot sequence constructs the same five
  higher-level managers (system/speech/sound/frontend/music roots) in a
  recognisable, same-role way has not been checked - `EBOOT.elf`'s own
  `Game_Main`-equivalent boot function has not been located.
- Whether `Memory_Alloc`/`Heap_Alloc`'s call shape actually matches
  `FwMemAllocator_Allocate`/`FwMemHeap_Create` beyond "both are an allocator
  near the front of boot" is unverified - no side-by-side signature or
  call-site comparison has been done yet.

## Next Steps

- In `ps3-hdfury-eu`, find the boot entry point (the PS3 equivalent of
  `vita-2048-eu-v104`'s `Game_Main`) - likely near the ELF entry OPD or
  reachable from `RenderManager_CreateInstance`/`Gcm_Init`'s callers, both
  already named. `Collision_Construct`/`RaceManager_Construct` give a rough
  idea of construction-order idiom to look for.
- Once found, check whether it constructs a system/speech/sound/frontend/music
  manager sequence in the same relative order `Game_Main` does, and whether
  any object carries a `__FILE__`-style debug tag naming a matching class
  (the same `param_1[0xb] = "<Path>.cpp"` idiom `GameRoot_Construct` and this
  binary's own `Collision_Construct` both use).
- If a same-role match is found, raise the corresponding 2048 name's
  confidence and cite the PS3 evidence on its page; if a PS3-side name gets
  applied as a result, it needs its own `names.tsv` row per the usual rule.
- If no matching sequence turns up after a real search (not just an absence
  in the current 107-row `names.tsv`), say so and close this thread rather
  than leaving it open indefinitely on a name that may never get PS3-side
  RE attention.
