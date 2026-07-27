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

The PS2's Emotion Engine is a plain MIPS variant and works out of the box.

The Ghidra project lives in `data/ghidra/`, which is gitignored. The project
database is a working copy; the record of truth is `docs/ghidra/`. See
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
