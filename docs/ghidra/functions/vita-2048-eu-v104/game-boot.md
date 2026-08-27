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
- Allocates a `0x100`-byte block tagged `"GameRoot LinkObj"` through
  `Memory_Alloc(size, tag, line, flags)` - called the same way throughout this
  function and its callees, see `Memory_Alloc` below for why "tag"/"line" is
  the wrong way to read those two arguments - and passes it straight to
  `GameRoot_Construct`.
- Immediately builds a second, `0x3c`-byte object off the just-constructed
  `GameRoot` and passes it to `SystemRoot_Construct` - the one manager with no
  log line of its own, see below.
- Goes on to build a speech manager, a sound manager, a "frontend root" and a
  music manager (each the same allocate-construct-link pattern; three of the
  four are logged with `"Create speech manager..."` / `"Create sound
  manager..."` / `"Create music manager..."` / `"done\n"`, the frontend root
  falls between sound's `"done\n"` and music's own banner with no log line),
  sets the window/session title to `"WIPEOUT_HD"`, and picks a boot-mode
  string (`"Main Menu"`, `"RaceBox"`, `"MPStress"`, `"Launch 2048"`, `"Boot
  Connect"`) from flags read off `DAT_818bbf88`.
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

## `Memory_Alloc` - `0x812e5590` and `Heap_Alloc` - `0x812e4f68`

**Confidence: 82 / 80**

Every allocation in `Game_Main` and its callees goes through
`FUN_812e5590(size, "<tag string>", 0x1ef, flags)`. The call shape looks like
a tagged, line-tracked allocator, and the existing prose on this page
(pre-2026-08-27) called it that. It isn't: `Memory_Alloc`'s entire body is two
Thumb instructions - `uxtb r1, r3` then `bl Heap_Alloc` - so under AAPCS it
forwards only `r0` (size, unchanged) and `r3` zero-extended into `r1`
(flags), and never reads `r1` (tag pointer) or `r2` (the `0x1ef` line
constant) at all. This is proven in the disassembly, not inferred from the
decompiler dropping unused parameters:

```
812e5590: push {r4,lr}
812e5592: uxtb r1,r3
812e5594: bl 0x812e4f68
812e5598: pop {r4,pc}
```

Two corroborating observations rule out this being a per-call-site
peculiarity: every one of the six `Game_Main` call sites passes the literal
`0x1ef` for the line argument regardless of where in the function the call
actually is (a real `__LINE__` would vary per site - this is a macro-baked
constant that never reached its expansion site, or reached one that discards
it), and five of the six pass the generic literal `"Unknown LinkObj"` rather
than a distinct tag (only the very first, `GameRoot`'s own allocation, carries
a real name, `"GameRoot LinkObj"`). Whatever originally consumed the tag and
line - almost certainly a debug/tracking build of this allocator - is compiled
out in this release binary; the two arguments are dead weight computed at
every call site for no observable effect. Named for what it actually does
(forward a size and a size-class flag), not for what its signature suggests.

`Heap_Alloc` is the real allocator underneath. It brackets its body with
`SceDebugLed_78E702D3(1)`/`(0)` (a recovered NID import, used here as an
in-progress marker rather than for its literal LED purpose), lazily
constructs a `0x1e8`-byte global heap-manager structure on first call and
initialises three named sub-heaps inside it - `"Large"`, `"Small"`, `"String"`
- each with a lock (`"GlobalHeaps Main Lock"` / `"GlobalHeaps String Lock"`)
via `FUN_812e57f4`. It routes small requests (`< 0x400` bytes) to the
`"Small"` sub-heap first, falls through to the size-appropriate sub-heap
(`Heap_Alloc`'s own `param_2` selects `Small`/`Large` via the flag
`Memory_Alloc` forwarded), and on exhaustion retries after growing the
backing pool via `FUN_812e39ca`, looping until an allocation succeeds or the
grow call itself fails. A registered callback at `DAT_81525218` fires when an
allocation still fails after that retry loop - the shape of an
out-of-memory/GC hook, not read further this pass.

Both are single-binary evidence (EU patch-v104 only): `/vita-2048-usa-v104/eboot.elf`
has not been auto-analyzed in this Ghidra project (1,197 functions found vs.
12,420 for EU), so the address-identical cross-check `Game_Main`/
`GameRoot_Construct` got was not repeated here. That is why both sit at 80-82
rather than the 84-86 band, despite the allocator's own disassembly being
about as direct as evidence gets.

## `SystemRoot_Construct` - `0x8122ebfc`

**Confidence: 84**

The manager `Game_Main` constructs with no log line of its own - a prior pass
over this call site had guessed "input or options manager" for this address
before it was read; the actual evidence says otherwise. Same evidence class as
`GameRoot_Construct`: `param_1[0xb] = "System/System/SystemRoot.cpp"`, a
vtable store, and the same `LinkObj` allocate-and-link pattern for a nested
`0x6250`-byte child object. Also initialises a path to
`"Data\XML\HandlingStats.xml"` and stores itself in the global `DAT_818b9fd0`
- a second engine-wide singleton alongside `GameRoot`, constructed
immediately after it and before any manager that has its own log line.

## `SpeechManager_Construct` - `0x812677ea`

**Confidence: 80**

Called with the object `Game_Main` allocates directly between logging
`"Create speech manager..."` and `"done\n"` - the same log-bracket evidence
class `Game_Main`'s own identification (confidence 86) rests on. Sets a
vtable and stores itself in the global `DAT_81521c18`. One open oddity, not
acted on: `param_1[0xb]` here is not an inline debug-tag string like every
other constructor on this page, but `&DAT_814c5f74`, a block whose bytes read
`"file\0default\0wo.VBE.LocalTransform\0wo.VBE.StaticTransform\0"` -
xref-checked, and nothing else in the binary references that address. It
does not look like a `<Subsystem>.cpp` tag, and it is not obviously about
speech either; left unexplained rather than guessed at further, since the
log-line bracket alone already puts the identification past 70.

## `SoundManager_Construct` - `0x81262c5a`

**Confidence: 90**

Two independent evidence classes agree on this one: it is both the object
built directly after `Game_Main` logs `"Create sound manager..."` (the
`Game_Main`-confidence evidence) and it carries its own debug tag,
`param_1[0xb] = "System/Sound/psp2/SoundManager.cpp"` (the `GameRoot`-style
evidence) - explicitly a PS Vita (`psp2`) sound backend. Body is mostly
placement-init of a large audio-parameter block (default volumes, curve
tables built from a log-domain approximation, `.bnk` bank paths -
`frontend.bnk`, `speech_results.bnk`, `generaltrack.bnk`,
`speech_fe_NGP.bnk`) and two fixed-size free-list pools, consistent with the
by-far-largest allocation in the table (`0x10e10`, ~68 KiB) that precedes it
in `Game_Main`.

## `FrontendRoot_Construct` - `0x8106322a`

**Confidence: 90**

The address the boot-managers thread guessed as a "3D audio/positional-audio
manager" (explicitly flagged confidence 0, hypothesis only) - it is not audio
at all. `param_1[0xb] = "Frontend/General/FrontendRoot.cpp"`, and the body
registers on the order of seventy named settings through
`FUN_812f9c74`/`FUN_812f9d6e`/`FUN_812f9e66`/`FUN_812f9bfa`/etc. against a
`"FeEnvInfo"`-tagged block, all under `Lighting.*`/`Water.*`/`Debug.*` keys
(`"Lighting.Sun direction"`, `"Lighting.BloomFactor"`, `"Water.Water plane
height"`, `"Debug.Track.Render Track"`, and eleven more), then loads
`"data/fe/fe.envsettings"` into that block - the front end's own environment
settings, the same shape [`envsettings.md`](../../../formats/envsettings.md)
documents for HD/Fury's per-circuit `.envsettings` files, but read once at
boot for the menu environment rather than per-track. Also links in a
language-plugin loader (`Data/Plugins/languages/<lang>`) and a
`Data/Plugins/neartitleentries` list. This is the strongest single-binary
identification on this page: the tag string and the settings-key content
corroborate each other independently.

## `MusicManager_Construct` - `0x8125f796`

**Confidence: 90**

Same two-evidence-class agreement as `SoundManager_Construct`: built directly
after `Game_Main` logs `"Create music manager..."`, and carries its own tag,
`param_1[0xb] = "System/Sound/psp2/MusicManager.cpp"`. Body is placement-init
of playback state (fade curves, an `0xffffffff` "no track" sentinel, default
volume `1.0`) and one call into `FUN_81264990` gated on `FUN_812e5110(0x458)`
succeeding - an allocate-a-decoder-buffer-then-open-a-stream shape, not
examined further this pass.

## History

- 2026-08-26: 86 / 84, first RE pass on this binary. Decompilation plus
  address-identical cross-check against `/vita-2048-usa-v104/eboot.elf`; no
  runtime trace yet.
- 2026-08-27: 82 / 80 / 84 / 80 / 90 / 90 / 90, the allocator wrapper and the
  five manager/root constructors `Game_Main` allocates for. Single-binary
  (EU patch-v104 only) - `/vita-2048-usa-v104/eboot.elf` was not
  auto-analyzed in this session's Ghidra project, so no address-identical
  cross-check this pass. Two names correct guesses recorded (and explicitly
  scored 0) in the handover thread that started this work:
  `FrontendRoot_Construct` was guessed as a 3D-audio manager, and
  `SystemRoot_Construct` was guessed as an input/options manager. Neither
  guess was acted on before being read.
