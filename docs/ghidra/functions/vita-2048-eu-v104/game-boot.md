# Game boot: entry point and the GameRoot singleton

Functions in `eboot.elf` (WipEout 2048, Vita, `PCSF00007` patch v1.04), image
base `0x81000000`. **The names here are applied**, from [names.tsv](names.tsv).
This is the first RE pass on this binary - see
[toolchain.md#vita](../../../reverse-engineering/toolchain.md#vita) for how it
got imported at all.

## `Game_Main` - `0x81003dd2`

**Confidence: 86**

The game's top-level entry point, called from `FUN_812ed860` via a computed
call (the SDK's module-start-to-`main` trampoline) at the same address in
both the EU and USA patch-v104 builds - address-identical, not just
structurally similar, which is stronger than a second read of the same
binary.

Evidence, in call order as decompiled:

- Opens with `SceLibc_9A004680("3...2...1...Welcome to WipEout\n")` -
  `SceLibc_9A004680` is the NID-resolved `puts`/log import, and this is an
  unmistakable startup banner, not a coincidental string.
- Sets the app's working path to `"app0:PSP2/"` - the Vita convention for "this
  app's own install directory", consistent with a boot/setup routine.
- Allocates a `0x100`-byte block tagged `"GameRoot LinkObj"` through the
  engine's tracked allocator (`FUN_812e5590(size, tag, line, flags)`, called
  the same way throughout this function and its callees - see
  `GameRoot_Construct` below) and passes it straight to `GameRoot_Construct`.
- Goes on to build a speech manager, a sound manager, a music manager and a
  "3D audio" block (each the same allocate-construct-link pattern, logged with
  `"Create speech manager..."` / `"Create sound manager..."` /
  `"Create music manager..."` / `"done\n"`), sets the window/session title to
  `"WIPEOUT_HD"`, and picks a boot-mode string (`"Main Menu"`, `"RaceBox"`,
  `"MPStress"`, `"Launch 2048"`, `"Boot Connect"`) from flags read off
  `DAT_818bbf88`.
- Logs `"Entering main game loop\n"`, tags a profiler region
  `"Main Game Loop"`, then runs the `do { ... } while (DAT_815412e8 == '\0')`
  loop that polls pad state, steps simulation/render (`FUN_8100014a`,
  `FUN_81000240`) and presents (`FUN_8129417c`/`FUN_8129423e`) every
  iteration - the shape of a game's per-frame loop, not a one-shot routine.
- On loop exit, tears the same objects back down
  (`FUN_812aae4c`/`FUN_812232c4`/`FUN_8101fd90`/`FUN_81036154`) before
  returning `0`.

Not runtime-verified (no Vita debugger harness in this project yet), so this
stays below the 95+ band regardless of how unambiguous the decompilation is.

**Worth following up**: the window/session title this sets is `"WIPEOUT_HD"`,
not `"WIPEOUT_2048"` or similar - on a Vita build. 2048 ships HD/Fury content
as DLC (the store listings for both are real), so this may be the same
`ps3-hdfury-eu` codebase retargeted for Vita rather than a fresh build off the
PSP/PS2 Pulse lineage. Unconfirmed hypothesis, not yet checked against
`docs/ghidra/functions/ps3-hdfury-eu/`.

## `GameRoot_Construct` - `0x81000032`

**Confidence: 84**

Constructs the engine's `GameRoot` singleton object, called exactly once, as
the very first thing `Game_Main` does after allocating its backing storage -
same call site (one instruction of drift between builds) in both EU and USA
patch-v104.

Evidence:

- The object passed in (`param_1`) gets `param_1[0xb] = "Game/GameRoot.cpp"` -
  the engine's debug/link-tracking tag literally naming the source file this
  constructor lives in, the same pattern `Game_Main` uses for
  `"GameRoot LinkObj"`, `"Unknown LinkObj"`, etc.
- Sets a vtable pointer (`*param_1 = &PTR_LAB_81221a9e_1_...`) and a second
  function pointer at `param_1[1]` - the shape of a C++ constructor
  initialising its vtable and a callback/destructor slot.
- Stores the constructed object in the global `DAT_8153fb10`, which
  `Game_Main` (and the manager-construction blocks after it) reads back
  immediately afterward as "the GameRoot instance" to link every subsequent
  manager against.
- Allocates a second, nested `0x2040`-byte object tagged
  `"../../Wipeout/Code/System/Render/BlockList.h"` and links it to the
  GameRoot object via `FUN_81222014` - consistent with GameRoot owning the
  renderer's block-list allocator as a member.

Scored one band below `Game_Main`: the identification runs through the
embedded debug-tag strings rather than the function's own log output, one
inferential step further from direct evidence.

## History

- 2026-08-26: 86 / 84, first RE pass on this binary. Decompilation plus
  address-identical cross-check against `/vita-2048-usa-v104/eboot.elf`; no
  runtime trace yet.
