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
