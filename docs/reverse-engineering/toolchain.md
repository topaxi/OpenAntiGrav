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
git clone https://github.com/clienthax/Ps3GhidraScripts.git \
    data/tools/ps3-ghidra-scripts
chmod +x data/tools/ps3-ghidra-scripts/gradlew
env -u JAVA_TOOL_OPTIONS \
    JAVA_HOME=/usr/lib/jvm/java-21-openjdk \
    GHIDRA_INSTALL_DIR=/opt/ghidra \
    data/tools/ps3-ghidra-scripts/gradlew \
        --project-dir data/tools/ps3-ghidra-scripts \
        --no-daemon buildExtension
```

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

#### PS3 traps

**`import_file` over GhidraMCP silently loads a raw binary if you pass a
language.** The tool's own documentation says a language is for "raw firmware
binaries", and passing one selects the Raw Binary loader: the result is a single
flat block at address `0` with no sections and no entry point, which looks like
a successful import. The tell is `Memory Blocks: 1` in `get_metadata` where the
ELF has eight program headers. Import without a language and correct it after,
or import in the GUI, or drive `analyzeHeadless`, which takes a loader and a
processor together.

**GhidraMCP cannot run the two scripts.** `run_ghidra_script` is gated behind
`GHIDRA_MCP_ALLOW_SCRIPTS=1` in the environment of the *Ghidra process*, so
enabling it means restarting Ghidra. Without it the pre/post-analysis flow above
has to be driven from the Script Manager by hand.

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

**Some Cell vector instructions are missing from Ghidra's sleigh.** `lvlx`,
`lvrx`, `stvlx`, `stvrx` and their `l` variants are PPC970/Cell extensions that
`altivec.sinc` does not implement, and they break decompilation where they
appear. Measured on Wipeout HD / Fury: **851 `lvlx`** in 1,911,344 instructions
across the four executable sections, and none of the other seven forms. Narrow,
but concentrated in vector code.

The Ghidra project (`OpenAntiGrav.gpr` / `OpenAntiGrav.rep/`) lives at the
repository root, not under `data/`, and is gitignored by name rather than by
directory - see the comment above `*.gpr` in `.gitignore`: a project opened at
the repo root leaves its lock beside the `.gpr`, and that lock records the
developer's hostname and username. The project database is a working copy;
the record of truth is `docs/ghidra/`. See
[ADR-0005](../architecture/adr/0005-ghidra-conventions.md).

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

Provides an EE and IOP debugger, memory search, and save states.

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
