# Reverse engineering toolchain

## Ghidra

Ghidra 11+ with a JDK 21+.

```sh
sudo pacman -S ghidra
```

**Stock Ghidra is not sufficient for PSP binaries.** Its MIPS support has no
Allegrex VFPU, and it mis-decodes vector instructions rather than rejecting
them. Build and install the Allegrex processor module before importing
anything:

```sh
sudo pacman -S jdk21-openjdk    # the build needs JDK 21 specifically
just build-allegrex             # leaves an installable zip in data/tools/
```

Then `File > Install Extensions > +` in Ghidra, restart, and re-import. Details
in [Allegrex and the VFPU](../psp/allegrex-vfpu.md).

**Stock Ghidra is not sufficient for PS2 binaries either, and fails more
dangerously than the PSP case.** Its generic MIPS support auto-detects the
Emotion Engine's R5900 core as `MIPS:LE:64:64-32R6addr` - MIPS Release 6, a
2014 ISA revision that reassigns much of the opcode space MIPS III (what the
R5900 actually implements) used. That is not a near-miss: R5900 code decoded
as R6 is mostly wrong, and analysis finds almost nothing (1 function on
`SCES_547.48`, versus 5,234 once the language is right).

Worse, some of the R5900's MMI (multimedia instruction) encodings alias onto
opcodes stock Ghidra recognises as MIPS DSP ASE - a real but different vendor
extension. Rather than rejecting the byte pattern, it silently decodes a
plausible-looking but wrong instruction (`ADDU.QB`, `DPA.W.PH` seen in
practice), which reads as legitimate disassembly rather than an obvious
failure. `halt_baddata` a few instructions later, if present, is the tell.

Install [ghidra-emotionengine-reloaded](https://github.com/chaoticgd/ghidra-emotionengine-reloaded)
before importing anything. Build from source against the installed Ghidra,
same reasoning as Allegrex above - the declared extension version must match
Ghidra's exactly and upstream does not publish a build for every point
release:

```sh
git clone https://github.com/chaoticgd/ghidra-emotionengine-reloaded.git \
    data/tools/ghidra-emotionengine-reloaded
# Optional, and irrelevant for SCES_547.48, which is stripped: fetches the
# stdump helpers the extension uses to import MIPS .mdebug symbols.
bash data/tools/ghidra-emotionengine-reloaded/os/download.sh

# No system gradle needed - Allegrex's wrapper drives any project via
# --project-dir, and its distribution is already cached from that build.
env -u JAVA_TOOL_OPTIONS \
    JAVA_HOME=/usr/lib/jvm/java-21-openjdk \
    GHIDRA_INSTALL_DIR=/opt/ghidra \
    data/tools/ghidra-allegrex/gradlew \
        --project-dir data/tools/ghidra-emotionengine-reloaded \
        --no-daemon buildExtension
# zip lands in data/tools/ghidra-emotionengine-reloaded/dist/
```

This project has no Gradle wrapper of its own and no Kotlin toolchain
declaration, so it builds under any JDK Gradle itself supports. The pin above
is the *wrapper's* constraint, not the project's: Allegrex ships Gradle 8.10.2,
which refuses JDK 23+, and this machine's system JDK is 26. Ghidra wants Gradle
8.5 or newer (`application.gradle.min`), so 8.10.2 satisfies both ends.

Verify the zip before installing it - a version mismatch is not reported as an
error, Ghidra just never loads the extension:

```sh
unzip -p data/tools/ghidra-emotionengine-reloaded/dist/*.zip '*/extension.properties' \
    | grep '^version='
grep '^application.version=' /opt/ghidra/Ghidra/application.properties
```

Then `File > Install Extensions > +`, restart Ghidra, and re-import. Unlike
Allegrex, no manual language selection is needed afterward: the extension
ships its own `.opinion` file matching the EE's ELF `e_flags`, so a plain
import auto-detects language `r5900:LE:32:default` directly. It also declares
a single `default` compiler spec calibrated for the EE - there is no
o32/n32/o64 choice to make, unlike picking a bare `MIPS:LE:64:*` language by
hand.

PS2 does not need the image-base fix PSP does: `SCES_547.48`'s `LOAD` segment
already declares `VirtAddr 0x00100000`, the PS2 user-module convention, so the
ELF loader places it correctly with no manual rebase step.

### PS3

**Unlike PSP and PS2, no processor module is needed.** Stock Ghidra decodes the
Cell PPU: the language is `PowerPC:BE:64:A2ALT-32addr` (shown as variant
`PowerISA-Altivec-64-32addr`). What PS3 needs instead is a decryption step
first, a compiler-spec fix, and a script pack.

**1. Decrypt the executable.** A disc `EBOOT.BIN` is a SELF - an encrypted
container - so there is nothing to import until it is decrypted. RPCS3 does it
with keys it carries itself:

```sh
rpcs3 --decrypt data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR/EBOOT.BIN
# writes EBOOT.elf beside it, in about two seconds
```

**No firmware install is needed**, which is worth stating because the opposite
is the natural assumption: RPCS3 prints `Missing Firmware` and decrypts anyway.
A disc SELF uses compiled-in keys, and the `PS3UPDAT.PUP` on the disc plays no
part. The decrypted ELF is derived data - gitignored, and regenerable in
seconds, so it is never committed.

Sanity-check the result rather than trusting the magic: `readelf -h` should
report `ELF64`, big endian, `OS/ABI <unknown: 66>` (Cell LV2) and machine
`PowerPC64`. On Wipeout HD / Fury the entry point `0x870530` sits in the *data*
segment, which is correct and not a decryption failure - PPC64 ELF entry points
are OPD descriptors, and this one resolves to `{func=0x00010230,
toc=0x008ad4d8}`, one address in the executable segment and one in the data
segment.

**2. Fix the compiler spec.** On the PPU, `r2` holds the TOC pointer and a call
does not clobber it; Ghidra's `ppc_64_32.cspec` does not say so, so the
decompiler invents TOC reloads. The output looks fine and is wrong, which is the
dangerous kind of failure:

```sh
just patch-ppc-cspec           # needs sudo; idempotent
just patch-ppc-cspec --check   # exit 0 patched, 1 not
```

**This edits the Ghidra installation, so a Ghidra upgrade silently reverts it.**
Run `--check` after every upgrade. `--revert` restores the backup it takes.

**3. Install the script pack.** [Ps3GhidraScripts](https://github.com/clienthax/Ps3GhidraScripts)
defines the imports, exports and TOC, and names what it can from a bundled NID
database. Build from source against the installed Ghidra, same reasoning as
Allegrex and the Emotion Engine above:

```sh
just build-ps3-scripts
```

This also adds this project's PS3 compiler-spec fix as a second language - see
below - since upstream ships no `data/languages/` of its own to conflict with.
Then `File > Install Extensions > +` and restart.

**4. Import.** Use the script, which does all of the below in one command and
checks the prerequisites first. Close Ghidra before running it - headless cannot
open a project the GUI holds:

```sh
scripts/import-ps3-eboot.sh
```

By hand, the order matters and is easy to get wrong:

1. Import `EBOOT.elf`, language `PowerPC:BE:64:A2ALT-32addr`, big endian, with
   auto-analysis **off**. There is no `.opinion` file, so the language has to be
   chosen by hand - untick "recommended" in the import dialog to see it.
2. Run `AnalyzePs3Binary.java` from the Script Manager.
3. Run auto-analysis.
4. Run `AssignPs3R2FromOpd.java`. **Not optional, and not in upstream's README** -
   see the TOC trap below.
5. Run `DefinePS3Syscalls.java`.

A correct import of Wipeout HD / Fury reports 26,100 functions, 159 memory
blocks, and imports named from the NID database.

#### An extension-shipped alternative to step 2

Step 2 edits the Ghidra installation itself, which is root-owned and does not
survive a Ghidra upgrade. The fix can instead ship as part of the
Ps3GhidraScripts extension: `scripts/ghidra-ps3-language/ppc_64_32_ps3.cspec`
(the same `ppc_64_32.cspec` with `r2` added to `<unaffected>`) plus
`scripts/ghidra-ps3-language/ppc_ps3.ldefs`, defining a second language,
`PowerPC:BE:64:A2ALT-32addr-PS3`, that reuses stock Ghidra's own
`ppc_64_isa_altivec_be.sla` and `ppc_64.pspec` by filename rather than
vendoring them - Ghidra's `SleighLanguageProvider` falls back to an
application-wide search by filename when a referenced file is not found
beside the `.ldefs`, so this works as long as the filename is unique across
the install (verified true for both on this machine). A duplicate id is not
allowed, which is why it needs its own rather than reusing step 1's.

`just build-ps3-scripts` (`scripts/build-ghidra-ps3-scripts.sh`) copies these
two tracked files into the Ps3GhidraScripts checkout before building, so step
3 always produces an extension carrying both the scripts and this language -
there is no separate build to run. Install it the same way as step 3 and
restart Ghidra. `scripts/import-ps3-eboot.sh --ps3-cspec` then imports under
it instead of step 2's language, and skips the `just patch-ppc-cspec` check
entirely since there is nothing on the Ghidra install left to check. Verified
2026-08-26: `analyzeHeadless` against a scratch project reports `Using
Language/Compiler: PowerPC:BE:64:A2ALT-32addr-PS3:default` and imports
successfully.

**The default since 2026-08-28.** The live `OpenAntiGrav.gpr` project was
originally imported under step 1/2's stock id; Ghidra does not migrate a
program between language ids, so switching meant a fresh reimport under the
new one. That was safe by this project's own rule (`just apply-names`
reproduces the recovered names onto a fresh import, per
[ADR-0005](../architecture/adr/0005-ghidra-conventions.md)), and the
maintainer made the deliberate choice to do it: `ps3-hdfury-eu`'s `EBOOT.elf`
now reads `PowerPC:BE:64:A2ALT-32addr-PS3`, still 26,100 functions, all 114
`names.tsv` rows re-applied clean. There is no `.opinion` file for it, and
there cannot be one: traced through Ghidra's own `ElfLoader.java`
(`ghidra/app/util/opinion/ElfLoader.java`, from `Base-src.zip`), the ELF
loader's opinion matching only ever keys on `e_machine` and `e_flags`, never
`EI_OSABI` - and this binary's `e_flags` is `0x0`, generic rather than
PS3-specific, so there is nothing left to key a constraint on even though
`EI_OSABI` itself (`0x66`, `ELFOSABI_CELLOSLV2`) is genuinely PS3-specific.
The language still has to be chosen by hand at import time, permanently.

The two added files are this project's own, tracked in `scripts/`, not
upstream's - Ps3GhidraScripts itself ships no `data/languages/` to compare
against. Whether to open a PR to
[clienthax/Ps3GhidraScripts](https://github.com/clienthax/Ps3GhidraScripts) is
undecided - ask before publishing anything there, same rule as the `lvlx`
vendoring question below.

#### PS3 traps

**`import_file` over GhidraMCP silently loads a raw binary if you pass a
language.** The tool's own documentation says a language is for "raw firmware
binaries", and passing one selects the Raw Binary loader: the result is a single
flat block at address `0` with no sections and no entry point, which looks like
a successful import. The tell is `Memory Blocks: 1` in `get_metadata` where the
ELF has eight program headers. Import without a language and correct it after,
or import in the GUI, or drive `analyzeHeadless`, which takes a loader and a
processor together.

**GhidraMCP runs scripts only when `GHIDRA_MCP_ALLOW_SCRIPTS=1` reaches the
bridge's environment.** This repository's `.mcp.json` sets it, and
`run_script_inline` ran live against the PS3 import on 2026-09-15 - so the
pre/post-analysis flow above is scriptable from here, and "scripting is off"
is a state to probe, not a constraint to route around; the probe, and what to
do when it reports disabled, are at the end of
[the `search_instructions` section](#search_instructionss-mnemonic-filter-is-exact-match-not-substring).

**Every function has its own TOC, and Ghidra uses one for all of them.** The OPD
declares a TOC per function, and this executable has two - `0x008ad4d8` over
`0x010200`-`0x758110` and `0x008bd3c4` over `0x32d5e0`-`0x7579c0`, overlapping,
so an address does not determine which. Ghidra uses the entry point's for
everything, which means **59% of functions have every TOC-relative load resolved
against the wrong base**, and a string cross-reference comes out as a real string
at a real address that is not the one the code loads. `AssignPs3R2FromOpd.java`
is the fix. Worked example and the manual three-read check:
[memory.md](../ghidra/functions/ps3-hdfury-eu/memory.md).

**`DFEngine.sprx` is not importable.** Its ELF type is `0xffa4`
(`ET_SCE_PPURELEXEC`, a relocatable PRX) and Ps3GhidraScripts states outright
that relocations are unsupported. `EBOOT.elf` is the only usable PS3 target.

**Some Cell vector instructions are missing from Ghidra's sleigh, and each one
truncates the function it sits in.** `lvlx`, `lvrx`, `stvlx`, `stvrx` and their
`l` variants are PPC970/Cell extensions that `altivec.sinc` does not implement.
Measured on Wipeout HD / Fury: **851 `lvlx`** in 1,911,344 instruction words
across the four executable sections, and none of the other seven forms -
re-derivable offline with `scripts/scan-ps3-cell-vector-ops.py`, which also
lists every site. What that does to the live import, measured 2026-09-15 by an
inline script over the same 851 words (confidence 95 - mechanical, and the
count reproduces from the raw file):

- **All 851 are undefined bytes belonging to no function body** - `lvlx` is not
  a "bad instruction" inside a function, it is where the function *stops*.
  Ghidra's disassembler halts at the undecodable word and never resumes at its
  fall-through, so everything after each site up to the next branch target is
  missing too. 341 carry an `Error` bookmark; the disassembler never reached
  the other 510 at all - typically because they sit in the fall-through shadow
  of an earlier site. Because every PS3 function starts from an OPD entry, the
  truncated bodies still exist - just short.
- **The sites fall in 294 functions** (nearest OPD entry before each site). Of
  the 418,640 bytes between those entries and the next function, **213,044 are
  not disassembled** - 39% of the 547,320 undisassembled bytes in the whole
  7.6 MB of executable text is this one missing constructor.
- **Eight of them are already named** in `names.tsv`, and the damage is not
  cosmetic: `Collision_MarchSegment` (`0x000364c0`) keeps 36 of its 2,272 bytes
  and decompiles to a lone `halt_baddata()`; `Collision_TestMeshObb`
  (`0x00037438`) keeps 1,148 of 2,288. `RaceManager_Construct`,
  `Craft_IntegrateHull`, `EngineFlare_Update`, `EngineFlare_RenderTick`,
  `ShipCollisionFx_Trigger_q` and `Trail_HitShipEffect` each lose 36-284
  bytes. [physics.md](../ghidra/functions/ps3-hdfury-eu/physics.md) read
  `Collision_MarchSegment` by hand-disassembling past the truncation - that is
  the workaround, and it does not survive into the decompiler or the call
  graph.

To map a site to its function in the live project, `getFunctionBefore(site)`
from an inline script is the owner (`getFunctionContaining` is `null` for every
site, by the first bullet), `function.getBody().getNumAddresses()` against the
distance to `getFunctionAfter` is how much survived, and
`listing.getCodeUnitAt(addr) instanceof Instruction` walked over that range
counts the hole. `search_byte_patterns` cannot even find the sites: its `mask`
is ignored
([the `search_instructions` section](#search_instructionss-mnemonic-filter-is-exact-match-not-substring)
has the original finding), so `7c00040e`/`fc0007fe` returns "No matches"
against 851 real sites, and a masked query for one site's exact bytes returns
the same five hits as the unmasked one.

**Teaching sleigh `lvlx` does not require vendoring Ghidra's sources.**
Compile-tested 2026-09-15: copying the stock
`Ghidra/Processors/PowerPC/data/languages/*.sinc` and
`ppc_64_isa_altivec_be.slaspec` from the local install into a scratch
directory, adding one `@include "cell_lvlx.sinc"` after `altivec.sinc`, and
running `support/sleigh` on it produced a `.sla` in 5.4 s with warnings
byte-identical to the stock spec's. The added constructor, modelled on `lvx`
(Power ISA 2.06 Book I, 6.7.2 - `EA = (RA|0)+RB`, the bytes from `EA` to the
end of its 16-byte block land at the top of `vD`, the rest zero; on a
big-endian vector that is the aligned block shifted left by `EA[0,4]` bytes):

```
:lvlx vrD,RA_OR_ZERO,B    is OP=31 & vrD & RA_OR_ZERO & B & XOP_1_10=519 & Rc=0
{
    build RA_OR_ZERO;
    ea:$(REGISTER_SIZE) = RA_OR_ZERO + B;
    eb:1 = ea[0,4];
    aligned:$(REGISTER_SIZE) = ea & 0xfffffffffffffff0;
    block:16 = *[ram]:16 aligned;
    vrD = block << (eb * 8);
}
```

That is the same shape as `scripts/build-ghidra-allegrex.sh`'s tracked patch:
a copy-at-build-time from the user's own Ghidra plus a small file of ours, with
nothing of Ghidra's committed here. It is **not installed** - shipping it means
a new `.sla` in the Ps3GhidraScripts extension, a language id or version bump,
and a reimport of the live project, which is the maintainer's call for the same
reason the cspec switch above was.

#### Two PS3 read errors that produce a plausible wrong answer with no visible error

Both of these were hit on Wipeout HD / Fury, both cost real time, and both share
a shape that makes them worth one combined warning: **the wrong reading resolves
to a real address holding real, sensible-looking data.** Nothing throws, nothing
looks malformed, and the mistake survives review because the output reads like a
finding. Neither is caught by any check in this repository; the only defence is
knowing to look.

**1. `get_xrefs_to` on a fixed address is unreliable, and so is any sweep that
only knows one addressing form.** Ghidra resolves every function's `d(r2)`
against a single TOC, and 59% of this binary's functions use the other one (see
[the two-TOC defect](../ghidra/functions/ps3-hdfury-eu/memory.md)). A consumer
of a global therefore may not appear in Ghidra's xrefs *at all*. It took
nineteen passes to find the reader of `0x00c81a5c` on the Zone thread partly for
this reason, and partly for a second one: every sweep looked for the field the
way the **writer** addresses it - `0x3aac(rX)` off a base pointer - while the
reader reached it through **its own dedicated TOC pointer slot**, making the
load `lfs f0, 0x0(r9)` at displacement zero. A displacement sweep, a branch scan
and an OPD-reference scan are all blind to that.

> **The rule: before concluding that a fixed address has no consumer, search the
> whole image for that address as a 4-byte literal.** Every hit in the data
> segment is a candidate TOC slot; for each, compute its displacement from both
> TOCs and scan the text for `d(r2)` instructions using it. That single check
> would have found the `0x00c81a5c` reader on pass two. A worked example is in
> [zone-effectsettings-loader.md](../ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md),
> "a nineteenth pass".

**2. A raw 16-bit displacement above `0x7fff` is negative.** PPC `lwz`/`lfs`/
`stw`/`addi` displacements are **signed**. Reading the raw field as unsigned
gives a slot roughly `0x10000` too high - and in a TOC region that is still a
live, readable address holding a plausible value. On the same thread,
`lwz r31, <raw 0x9d78>(r2)` was read as `TOC + 0x9d78 = 0x008c713c`, which holds
`0` in the image and reads perfectly as "an uninitialised runtime pointer, so
the buffer is heap-allocated". The correct reading is `TOC - 0x6288 =
0x008b713c`, which holds the static address `0x00c49000`. The wrong version was
committed and had to be corrected.

> **The rule: sign-extend before you resolve.** `d = raw - 0x10000 if raw >=
> 0x8000 else raw`. Ghidra's own listing prints the signed form (`-0x6288`), so
> the error only bites when reading raw instruction words in a script - which is
> exactly what the whole-image scans above require you to do. A slot that reads
> as `0` is the classic tell, because a genuinely-used TOC slot rarely is.

**The whole-image scan above is mechanical enough to script, and doing so
turns a nineteen-pass manual search into seconds.** Confirmed 2026-08-31
(`docs/ghidra/functions/ps3-hdfury-eu/ship-collision-fx.md`): via
`run_script_inline` (gated `GHIDRA_MCP_ALLOW_SCRIPTS=1`, same as
`run_ghidra_script`), read every function's `r2` value from the program's
own context register (`ProgramContext.getRegisterValue`) - reliable because
`AssignPs3R2FromOpd.java` already populated it at import, not something the
script has to recompute - then for a target address, keep only functions
whose own displacement (`target - r2`) fits a signed 16-bit range *and*
whose disassembly actually contains a `d(r2)` instruction at that exact
displacement (the first filter alone over-matches: many functions share a
TOC, only one of them typically executes the load). Validated before being
trusted on an unknown target by reproducing
`zone-effectsettings-loader.md`'s already-known answer
(`Scene_PrepareFrame @ 0x003aaf8c: lwz r9,-0x61f8(r2)`) from a blind script
run with zero other hits. Reach for this before assuming a string or global
with no `get_xrefs_to` hits has no consumer - it very likely still does, one
TOC hop away.

The Ghidra project (`OpenAntiGrav.gpr` / `OpenAntiGrav.rep/`) lives at the
repository root, not under `data/`, and is gitignored by name rather than by
directory - see the comment above `*.gpr` in `.gitignore`: a project opened at
the repo root leaves its lock beside the `.gpr`, and that lock records the
developer's hostname and username. The project database is a working copy;
the record of truth is `docs/ghidra/`. See
[ADR-0005](../architecture/adr/0005-ghidra-conventions.md).

### Vita

All four of WipEout 2048's `eboot.elf` (base + patch v1.04, both regions) import
cleanly into [VitaLoaderRedux](https://github.com/CreepNT/VitaLoaderRedux) as of
2026-08-26: language `ARM:LE:32:v7`, base address `0x81000000`, named
`.text`/`.data`/`VarImport` blocks, 1197 functions seeded by the loader itself
before auto-analysis even runs. `docs/ghidra/functions/vita-2048-*/` and
`BINARY_PROGRAMS` entries still wait on a first recovered name, per
[ADR-0005](../architecture/adr/0005-ghidra-conventions.md)'s own workflow.

**Default RE target: `PCSF00007` (EU), `patch-v104`.** Same role split as
`psp-pulse-usa` vs `psp-pulse-eu`: the patch is what a real device actually
runs (`base` is corroboration only), and EU is preferred over USA by policy
here the way `pulse-psp-eu` is preferred over the USA disc elsewhere in this
project. Program paths: `/vita-2048-eu-v104/eboot.elf` (target of record),
`/vita-2048-usa-v104/eboot.elf`, `/vita-2048-eu-base/eboot.elf`,
`/vita-2048-usa-base/eboot.elf` (corroboration-only).

**`FixupVLRImportThunks.java` (shipped with VitaLoaderRedux) is not needed
here, checked directly rather than left unevaluated.** Its own README frames
it as a fallback for when the NID Analyzer's import-thunk naming doesn't
resolve cleanly - renaming a thunk's target to `Library_NID` form. On
`eu-v104` (fully auto-analyzed), that naming is already clean without it:
`list_imports` returns systematic `Library_NID` names directly (e.g.
`SceCtrl_67E7AB83`, `SceAppMgrUser_10B5765F`), and a decompile of `Game_Main`
calls through those same systematic names at each call site
(`SceLibc_9A004680(...)`, `SceAppUtil_5DFB9CA0(...)`) rather than through
`FUN_`-prefixed thunk placeholders. VitaLoaderRedux's loader and its own NID
Analyzer are doing this at import time, before the optional script would run.
Re-check if a future program (or a `FixupVLRImportThunks`-adjacent script
update) starts showing `FUN_`-named call targets for known imports.

#### `eboot.bin` is still NpDrm-encrypted under the SELF wrapper

**An earlier `strip-vita-self.py` only ever cut the SCE header off - it never
decrypted anything.** A plaintext ELF header and phdrs made `readelf` look
satisfied (the earlier "extraction pipeline complete" claim), but that says
nothing about the segments: measured directly against
`data/extracted/vita/PCSF00007/patch-v104/eboot.bin`, the bytes at each
segment's declared offset are not zlib (no `0x78` header, `zlib.decompress`
fails immediately), and the first 64 KiB measures 7.9975 bits/byte of entropy
- ciphertext, not compressed-but-parseable data. `scripts/vita-self-decrypt.py`
replaces it and does the whole job; the old script is removed rather than kept
around half-working.

Two more things were wrong before any of that mattered:

1. **The program header table is not where the ELF's own `e_phoff` says.**
   `e_phoff` reads `0x34` (the standard 52-byte `Elf32_Ehdr` size), but the
   real table is 12 bytes later, at absolute `phdr_offset` in the outer
   `SCE_header` (`0xe0` here = `elf_offset (0xa0) + 0x40`, not `elf_offset +
   e_phoff`) - confirmed directly from the header's own `phdr_offset` field,
   not inferred from byte-shifting. This is
   [CelesteBlue-dev/PSVita-RE-tools](https://github.com/CelesteBlue-dev/PSVita-RE-tools)'s
   own `vita-unmake-fself/src/self.h` `ELF_header` struct: a trailing
   `uint32_t pad[3]` after the standard 14 fields, which is exactly the gap.
2. **The `SCE_header` fields past `elf_offset` matter and neither script read
   them.** `phdr_offset`, `section_info_offset` and friends are 6 more `Q`
   fields [KorewaWatchful](https://github.com/KorewaWatchful)/
   [Vita3K](https://github.com/Vita3K/Vita3K) both read; a script stopping at
   `elf_offset` (as `strip-vita-self.py` did) has no way to find the real
   phdrs or the per-segment metadata that names each segment's encryption and
   compression.

#### `vita-self-decrypt.py`: klicensee, no hardware, no F00D

**A full PC-only decrypt path exists and needs nothing but the title's
klicensee** - confirmed by reading
[Vita3K/Vita3K](https://github.com/Vita3K/Vita3K)'s `vita3k/packages/src/
sce_utils.cpp` (`decrypt_fself`/`get_segments`) directly, the same code path
Vita3K itself uses to run retail titles from a PKG + zRIF with no console.
`scripts/vita-self-decrypt.py` reimplements it in Python
(`cryptography` for AES, `zlib` for inflate): klicensee unwraps an
NpDrm-wrapped intermediate key, which with a compiled-in "metadata key"
(selected by system-version range and key revision - the table in
`sce_utils.cpp`'s `register_keys()`, credited there to TeamMolecule's
`sceutils`) decrypts the SELF's metadata block into per-segment AES-128-CTR
keys; each segment decrypts, then inflates if compressed, then lands at its
own phdr's `p_offset`. Verified three ways on the EU patch-v104 output, not
assumed: `readelf` reports clean `PT_LOAD`/`PT_SCE_VERSION` phdrs, disassembly
at `0x81000000` is real ARM Thumb-2 (`movw`/`movt` pairs, sane `push`/`pop`
prologues, not noise), and `SceModuleInfo.name` at the address `e_entry`
encodes (top 2 bits segment, bottom 30 bits offset - the convention
`ArmElfPrxLoader` itself uses) reads `WO_Game\0`.

**The klicensee itself needs zero hardware and zero network crypto calls -
it is recoverable purely from the zRIF string**, which is a much lighter
claim than it sounds: a zRIF is `base64(zlib(license_bytes, preset_dictionary))`,
confirmed by reading [KorewaWatchful/libzrif](https://github.com/KorewaWatchful/libzrif)'s
`keyflate.c` directly - no AES anywhere in it. `scripts/zrif-to-klicensee.py`
does this decode; `data/keys/README.md` has the details. This is a *different,
lighter* layer than `psvpfsparser`'s own `-f00d_url`/`-f00d_cache`, which
derives the *PFS filesystem* key and does need the F00D service - confirmed
by reading `motoharu-gosuto/psvpfstools`'s own README, which never mentions
SELF or NpDrm at all. Do not conflate the two: PFS decrypt (`psvpfsparser`)
gets you a *readable* `eboot.bin`; klicensee decrypt
(`vita-self-decrypt.py`) gets you a *plain* one.

zRIFs for both regions' base app (patch reuses the base license - same
content ID family, checked directly) came from nopaystation.com's own TSV
export (`https://nopaystation.com/tsv/PSV_GAMES.tsv`, fetched and grepped by
title ID directly, not scraped through a rendered page - the earlier attempt
at that was unreliable for picking one row out of a large file).

#### GhidraMCP import traps, still true regardless of input

**`analyzeHeadless -loader <name>` matches `Loader.getClass().getSimpleName()`,
not the display name.** `ArmElfPrxLoader.getName()` returns `"ARM ELF-PRX for
PlayStation®Vita"` (with the `®`), and passing that string to `-loader` fails
with `Invalid loader name specified` - confirmed by decompiling
`LoaderService.getLoaderClassByName`, which filters on `getClass().equals`
compared against `getSimpleName()`. Pass `ArmElfPrxLoader`. Also,
`analyzeHeadless` rejects a project path of `.` ("Path element starting with
'.' is not permitted") - pass an absolute path.

**`GhidraMCP`'s `import_file` picks the loader Ghidra's own auto-detection
would - which is only `ArmElfPrxLoader` once the file is genuinely decryptable.**
Importing the *encrypted* `strip-vita-self.py` output with no `language`
silently fell through to the stock ELF loader instead of erroring: the tell
was language `ARM:LE:32:v8` (`ArmElfPrxLoader` hardcodes `v7`), base address
`0`, and stock `_elfHeader`/`_elfProgramHeaders` blocks
`ArmElfPrxLoader.load()` never creates - exactly the PS3 raw-binary trap
above, but reached without passing a language at all. The cause is
`findSupportedLoadSpecs` swallowing its real rejection reason
(`UnsupportedElfException`/`MalformedElfException`, or a failed
`SceModuleInfo` name check) into a silently empty list, so Ghidra falls back
to whatever else claims the file. Importing the *correctly decrypted*
`vita-self-decrypt.py` output the same way (no `language`) picks
`ArmElfPrxLoader` correctly, because this time it actually has a load spec to
offer. The lesson: a clean `import_file` result on this format is not proof
the right loader ran - check `Language` and the block names in
`get_metadata`/`list_segments` regardless.

### GhidraMCP

[GhidraMCP](https://github.com/LaurieWired/GhidraMCP) exposes Ghidra over an
HTTP API so an agent can drive it directly.

1. Ghidra: `File > Install Extensions`, add the `GhidraMCP` zip, restart.
2. In CodeBrowser, accept the new plugin when prompted, or enable it via
   `File > Configure > Miscellaneous > GhidraMCP`.
3. Check it is listening:
   ```sh
   ss -tlnp | grep 8089
   ```
4. The repository's `.mcp.json` starts the bridge with **no arguments**. Run
   `/mcp` in Claude Code and approve the `ghidra-mcp` server.

The server only responds while a program is open in CodeBrowser.

### Do not pass `--ghidra-server`

Older GhidraMCP bridges took a `--ghidra-server http://127.0.0.1:8089/`
argument. Bridge 1.28.1 does not, and argparse rejects it and exits, which
Claude Code reports only as:

```
Connection failed (-32000): MCP error -32000: Connection closed
```

That message says nothing about the cause. If you see it, run the bridge by
hand to get the real error:

```sh
/usr/bin/python /opt/ghidra-mcp/bridge_mcp_ghidra.py --help
```

The current bridge discovers running instances itself, preferring the Unix
socket in `/run/user/$UID/ghidra-mcp/` over TCP, and auto-connects to the open
project:

```
INFO - Auto-connecting via UDS to OpenAntiGrav
INFO - Auto-registered 184 tools from OpenAntiGrav
```

**The rules in [ADR-0005](../architecture/adr/0005-ghidra-conventions.md) apply
to agent-driven analysis exactly as they do to manual analysis, and matter more.
An agent can generate a plausible reading of anything.** Every rename needs a
documentation page with checkable evidence, and every claim needs a confidence
score.

#### `search_instructions`'s `mnemonic` filter is exact-match, not substring

Its own tool description says "case-insensitive substring match on both
fields" - for `mnemonic` that is not what it does. Confirmed live
2026-08-27 against `psp-pulse-usa`: `mnemonic="lv"` returns 0 matches
(524,719 instructions scanned) where `mnemonic="lv.q"` returns 1,000+
(truncated at the request limit). `"lv"` is a literal substring of
`"lv.q"`, so a true substring match would have returned the same set for
both; it does not, so the field is exact-match in practice regardless of
what the description claims.

This matters because **MIPS delay-slot instructions render with their own,
distinct mnemonic**: a leading underscore - `_lw`, `_lwc1`, `_addiu`,
`_move`, `_sw`, and so on for every mnemonic that can occupy a delay slot.
A sweep that filters on `mnemonic="lw"` silently misses every `_lw` in a
branch delay slot, which on this compiler's output is a large fraction of
all loads. **A sweep run against one mnemonic form has checked at most
half of what it looks like it has.**

Sweep with `operand_pattern` only and no `mnemonic` filter, or run each
delay-slot form explicitly alongside its plain one.

**Confirmed again 2026-08-30 on `ps3-hdfury-eu` (PowerPC, no delay slots at
all), so this is the filter's own behaviour, not a MIPS-specific artefact.**
`mnemonic="st"` and `mnemonic="stf"` both returned 0 matches against a
function (`FUN_003df360`) whose already-fully-read disassembly plainly
contains `stw`, `std`, `stfs`, `stfd` and `stvx`. A prefix-style "sweep for any
store" is a vacuous zero, indistinguishable from a real negative unless every
exact mnemonic is tried individually.

**A third architecture, same day: `vita-2048-eu-v104` (ARM:LE:32:v7)
reproduces it too**, with a real known-positive control this time rather than
just an unrelated disassembly read - `mnemonic: ldr, operand_pattern: 0x634`
returned zero matches even though `Zone_UpdateStage` itself contains
`ldr.w r6,[r5,#0x634]`. Three unrelated instruction sets, three confirmations:
this is the filter's own implementation, not an artefact of any one
architecture's mnemonic table. The same investigation also found
`search_byte_patterns`' `mask` parameter does not filter at all on this
bridge - a masked query returns either the exact-pattern match or nothing,
never a genuinely wildcarded set, confirmed by comparing masked queries
against exact-match controls. **Workaround**: drop the mask and enumerate the
one truly-unknown nibble (e.g. a register number) as a small set of exact
patterns run in parallel, rather than relying on wildcarding to do it in one
call.

`run_script_inline`/`run_ghidra_script`, which would otherwise let a single
script enumerate every form in one pass, are gated off by config rather than
broken: they return `"Script execution disabled. Set
GHIDRA_MCP_ALLOW_SCRIPTS=1 ..."` (checked live 2026-08-27, `dry_run:
true`). An earlier session (2026-08-17) saw a different failure from the
same call - `GhidraPlaceholderBundle cannot be cast to
GhidraSourceBundle` - which does not reproduce now; whether that was fixed
or is just superseded by the env-var gate is unknown.

**This repository's own `.mcp.json` sets `GHIDRA_MCP_ALLOW_SCRIPTS=1` in
`ghidra-mcp`'s `env`, so scripting should come up enabled whenever Claude
Code itself starts the bridge from this repo.** Do not treat "scripting is
off" as a fixed constraint to route around by habit. Before any analysis
pass that scripting would do better than a manual sweep - anything the
per-mnemonic-sweep workaround above exists for - check the actual state
first with a one-call `run_script_inline`/`run_ghidra_script` probe
(`dry_run: true` if available). If it still reports scripting disabled
(a bridge started outside `.mcp.json`, an older bridge build, or a `.mcp.json`
edit that was not picked up), **stop and ask the user to enable it** (confirm
`GHIDRA_MCP_ALLOW_SCRIPTS=1` reaches the bridge's environment and restart the
bridge/MCP connection) rather than silently falling back to the slower,
easier-to-miss-a-form workaround. Only fall back to the manual sweep if the
user declines or scripting is confirmed unavailable. Once enabled, treat that
as the normal state and prefer scripting for anything the explicit-sweep
pattern above was compensating for.

## Emulators

### PPSSPP (PSP)

The primary verification target.

```sh
sudo pacman -S ppsspp     # installs the binary as PPSSPPSDL, not ppsspp
```

What it provides: a disassembler, memory viewer and watchpoints; breakpoints
with conditions; save states, which are what make scenarios reproducible; and a
frame-step mode.

Enable the debugger under `Tools > Developer tools`.

It also has a **websocket debugger**, which is the one that matters here: it is
scriptable, it works headless, and it is how the verification harness reads state
out of the original. See
[Driving PPSSPP from its websocket debugger](ppsspp-debugger.md) for how to start
it, what it can do, and the four traps in it.

### PCSX2 (PS2)

For cross-validation.

```sh
sudo pacman -S pcsx2
```

Provides an EE and IOP debugger, memory search, and save states - and, more
usefully, **PINE**, a memory-read/write socket API the stock build already has
compiled in. It boots Wipeout Pulse with no window and no human
(`just pcsx2-boot`) and, uniquely among the three emulators here, replays a
savestate frame for frame: the same scripted thirty frames produce a
pixel-identical capture twice.

See [Driving PCSX2 from PINE](pcsx2-debugger.md) for the protocol, the exact
commands, and the traps - the ini path `-datapath` invents, the Vulkan renderer
that cannot present to a virtual display, and the two silent input gates.

### RPCS3 (PS3)

Two separate jobs: it is what decrypts a disc `EBOOT.BIN` (above), and it is the
only way to watch a PS3 title behave. It boots WipEout HD / Fury from the
**decrypted** disc image with no window and no human -
`just launch-hdfury-ps3-headless` - and a stock `config.yml` already exposes a
GDB stub on `127.0.0.1:2345` that reads and writes guest memory at the same
addresses the Ghidra corpus uses.

What it has no equivalent of is PPSSPP's input API, which is what stops a
scripted run reaching a race. See
[Driving RPCS3 from its GDB stub](rpcs3-debugger.md) for what the stub answers,
the four traps in it, and the one permission that unblocks synthetic input.

## Disc tooling

`oag-unpack` handles CHD and raw ISO natively, so `chdman` is not needed for
normal work.

```sh
just unpack info      data/images/pulse-psp-usa.chd
just unpack container data/images/pulse-ps2-eu.chd   # when info fails
just unpack list      data/images/pulse-psp-usa.chd
just unpack sniff     data/images/pulse-psp-usa.chd
just unpack hexdump   data/images/pulse-psp-usa.chd PSP_GAME/USRDIR/FE.wad
```

### chdman

Install `mame-tools` if you want the reference implementation to check our
reader against:

```sh
sudo pacman -S mame-tools
chdman extractdvd -i data/images/pulse-psp-usa.chd -o /tmp/pulse.iso
```

Our listing and per-file sizes must match `7z l` on the extracted ISO exactly.
Any discrepancy is a bug in `oag-disc`.

## Getting the executables out

```sh
# PSP: BOOT.BIN is an unencrypted ELF, load it straight into Ghidra
#
# Still the USA disc, even though EU is now the Ghidra target of record
# (ADR-0048) - the untagged data/extracted/psp path is wired to it throughout
# resolve-psp-imports.py/mine-names.py; see methodology.md#where-to-start.
just unpack extract data/images/pulse-psp-usa.chd \
    -o data/extracted/psp 'PSP_GAME/SYSDIR/BOOT.BIN'

# PS2: likewise a plain ELF
just unpack extract data/images/pulse-ps2-eu.chd \
    -o data/extracted/ps2 'SCES_547.48'
```

`EBOOT.BIN` is the encrypted, signed variant the console actually boots. Ignore
it; `BOOT.BIN` is the same program without the wrapper.

## Optional

- **`binwalk`** for entropy and signature scanning across a directory. Overlaps
  with `oag-unpack sniff`, but is better at finding embedded structures partway
  into a file.
- **`imhex`** or another hex editor with a template language, for interactive
  structure exploration.
- **`ffmpeg`**, which already handles the PS2's `.PSS` files (MPEG-2 program
  streams) and the PSP's ATRAC3 audio.
