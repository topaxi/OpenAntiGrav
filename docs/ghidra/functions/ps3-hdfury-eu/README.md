# Wipeout HD / Fury PS3 functions

Functions from `PS3_GAME/USRDIR/EBOOT.elf` (Wipeout HD Fury, PS3, BCES-00664),
language `PowerPC:BE:64:A2ALT-32addr`, big endian.

**The imported file does not exist on the disc.** A PS3 `EBOOT.BIN` is an
encrypted SELF; what gets analysed is the ELF `rpcs3 --decrypt` writes beside
it. That makes this the only binary in this tree whose program name differs
from the file the disc ships, and the reason
`scripts/apply-ghidra-names.py`'s `BINARY_PROGRAMS` carries an `.elf` entry for
it. Reproduce the whole import with:

```sh
scripts/import-ps3-eboot.sh     # close Ghidra first; it needs the project lock
```

Setup, the compiler-spec fix this needs, and the traps are in
[toolchain.md](../../../reverse-engineering/toolchain.md#ps3).

## What this binary is, and is not

**Not a reverse-engineering target.** Wipeout HD / Fury is a
[later title](../../../overview/goals.md#scope) and no milestone is open on it.
What this directory exists for is the lineage question: HD is the first title
after Pulse, on completely different hardware, and whether its subsystems are
recognisably the same shapes is worth knowing cheaply before anyone plans work
on it. Names here are opportunistic, and any claim about *Pulse* still has to
be proved against Pulse's own binaries.

**The asset half of that question is answered, and this directory is no longer
where it is answered.** As of 2026-08-17 the disc's archives read and the survey
is [hd-status](../../../formats/hd-status.md): HD ships Pulse's `.vex` at
version 6 with Pulse's own class IDs, byte-swapped, and ten of its sixteen
environments are a Pulse or Pure circuit's spline. Where the executable is
still the only source is everything *behavioural* - what HD does with any of
it - and three things learned here so far carry: gameplay lives in `EBOOT.elf`
(`Collision.cpp`, `RaceManager.cpp` and `ModeManager.cpp` are all named from it)
rather than in the closed `DFEngine.sprx`; the C++ base class stores its own
`__FILE__` at object offset `0x30`, which attributes whole classes cheaply; and
**the renderer is in `EBOOT.elf` too** - it drives libgcm itself and imports one
single symbol from `DFEngine.sprx`. See [renderer.md](renderer.md).

**A second lineage question, this binary as the *known* side of the
comparison rather than the unknown one, is also answered.** WipEout 2048
(Vita) turns out to be this codebase retargeted, not a fresh Pulse-lineage
build - confirmed by literal `.cpp`/asset-path string matches against
`vita-2048-eu-v104`. See
[`vita-2048-eu-v104/README.md`](../vita-2048-eu-v104/README.md#the-lineage-question-is-answered-confirmed).

Two structural facts to expect, both different from every other binary here:

- **PPC64 with an OPD.** A function pointer is a descriptor pair
  `{code address, TOC value}` in the data segment, not a code address. The
  entry point `0x870530` is an OPD entry resolving to `func=0x00010230`,
  `toc=0x008ad4d8`. `AssignPs3R2FromOpd.java` in the script pack is what makes
  the decompiler read TOC-relative data correctly.
- **Stripped of symbols, but not of library names.** There is no symbol table
  and the section-name table is gone, so `readelf` shows 156 unnamed sections.
  The 122 `cell*`/`sce*` identifiers that survive are the module and export
  names, which `AnalyzePs3Binary.java` resolves against its NID database - so
  imports get real names and everything else starts as `FUN_`.

## Pages

- [memory.md](memory.md) - the allocator, its heap, the mutex around both, and
  **the per-function TOC defect this database has**. Read it before trusting
  any data or string reference in this program.
- [game-boot.md](game-boot.md) - `Game_Main` and the `GameRoot`/`SpeechManager`/
  `SoundManager`/`FrontendRoot`/`MusicManager` construction chain, cross-verified
  literally (matching `.cpp` tags, not just role) against `vita-2048-eu-v104`'s
  own boot chain.
- [weapons.md](weapons.md) - `Repulser`/`Rocket`/`RocketManager`/
  `WeaponExplosions` construct, cross-referenced against `psp-pulse-usa`'s own
  weapon docs, plus two owning classes (`WeaponManager`, `Plasma`) found but
  not fully read, and the one trap in `batch_string_anchor_report`'s
  per-binary behaviour.
- [collision.md](collision.md) - `Collision.cpp`: the arena, the class, and the
  MeshAABB narrowphase.
- [race-hud.md](race-hud.md) - the per-mode HUD definitions, their three retro
  skins, and what `SPZone` adds on top of the base race manager.
- [mode-manager.md](mode-manager.md) - `ModeManager.cpp`: the sibling mode
  hierarchy, and the one place a C++ constructor pair could be told apart.
- [race-manager.md](race-manager.md) - `RaceManager.cpp`: the singleton holder
  and the base of the race-mode hierarchy.
- [renderer.md](renderer.md) - the rendering layer: that it is in this binary
  rather than `DFEngine.sprx`, the GCM device bring-up, `RenderManager`, and
  the finding that HD keeps Pulse's one-translation-unit-per-`.vex`-class
  importer layout. Also the **out-of-Ghidra** way around the TOC defect,
  [`scripts/ps3-toc.py`](../../../../scripts/ps3-toc.py).
- [billboards.md](billboards.md) - `TrackStartup_Load`: how a `<Billboard>`
  becomes a 9-entry slot array indexed by its own `Num`, that it instantiates
  rather than textures existing geometry, and the one slot the engine
  overrides regardless of what its manifest authored.
- [sound.md](sound.md) - the SCREAM grain-opcode dispatch table
  (`0x00927614`), the located `0x05`/`0x06`/`0x08` "play/stop a child cue"
  handlers, guard `0x22`'s three-way variable-versus-immediate skip (the
  mechanism `.COLLISIONS`' severity and ship/wall split runs on, still
  undecoded past the opcode itself), and `0x19`'s alternate-selection
  handler - a random pick that never repeats the previous one.

Add a row to [`names.tsv`](names.tsv) and the page it cites in the same change:
`scripts/apply-ghidra-names.py` refuses a row whose address and name do not
both still appear on the page named in its last column.
