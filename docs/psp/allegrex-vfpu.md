# Allegrex and the VFPU: stock Ghidra is not enough

**Read this before analysing any PSP binary.** Stock Ghidra silently
mis-decodes every VFPU instruction in Pulse. It does not error; it produces
wrong instructions that the decompiler turns into wrong C.

## The problem

The PSP's CPU is Allegrex: MIPS II, plus a 128-bit vector FPU that Sony added
and that no standard MIPS variant has. Ghidra's ELF loader sees `e_machine =
MIPS` and picks `MIPS:LE:32:default`, which knows nothing about the VFPU.

Several VFPU opcodes collide with MIPS III 64-bit instructions that do not
exist on a 32-bit MIPS II part. Ghidra decodes them as those 64-bit
instructions rather than rejecting them.

## Evidence

Counting the top six bits of every word in `.text` of
`PSP_GAME/SYSDIR/BOOT.BIN` (Pulse PSP, UCUS-98712):

| Opcode | Allegrex meaning | Count |
| --- | --- | ---: |
| 0x12 | COP2 / VFPU | 130 |
| 0x18 | VFPU0 family | 860 |
| 0x19 | VFPU1 family (`vmul`, `vdot`, `vscl`, `vdet`) | 1,563 |
| 0x1b | VFPU3 family | 286 |
| 0x32 | `lv.s` | 822 |
| 0x36 | `lv.q` (128-bit vector load) | 10,174 |
| 0x37 | VFPU4+ family | 290 |
| 0x3a | `sv.s` | 612 |
| 0x3e | `sv.q` (128-bit vector store) | 11,295 |

**26,032 VFPU instructions, 4.1% of `.text`.**

4.1% understates the damage. VFPU code is not spread evenly: it clusters in
exactly the math the project cares most about. The densest run in this binary is
**70 consecutive VFPU instructions** at `0x0004bdc0`.

### What Ghidra produces

Asking Ghidra (`MIPS:LE:32:default`) to disassemble that dense run at
`0x0004bdc0`:

```
0004bdc0  9007b0db  ldc2 s0, 0x790(sp)
0004bdc4  8007b1db  ldc2 s1, 0x780(sp)
0004bdc8  4000b0fb  sdc2 s0, 0x40(sp)
0004bdcc  7007b0db  ldc2 s0, 0x770(sp)
```

Those are really `lv.q` and `sv.q`: **128-bit** vector loads and stores to VFPU
registers. Ghidra reports 64-bit generic coprocessor transfers. Wrong width,
wrong register file.

The arithmetic is worse. At `0x00010e7c`:

```
00010e7c  0a888964  daddiu t1, a0, -0x77f6
```

`daddiu` is a MIPS III 64-bit doubleword add. **The PSP cannot execute it.** The
real instruction is from the VFPU1 family. Ghidra emits a confident, completely
fictional instruction, and the decompiler builds C on top of it.

Reproduce:

```sh
curl -s -X POST http://127.0.0.1:8089/disassemble_bytes \
  -d '{"start_address":"0x0004bdc0","length":40}'
```

## Why this matters more than it looks

A missing instruction is obvious. A *wrong* instruction is not: the decompiler
produces readable C, the control flow looks sane, and nothing flags an error.
Anyone reading a physics function would be reading fiction.

This is the [methodology's](../reverse-engineering/methodology.md#anti-patterns)
"trusting a tool's output as fact" anti-pattern, encountered on day one.

**Any analysis of math-heavy code done on a stock `MIPS:LE:32` program should be
treated as confidence 0 and redone.**

## The fix

[kotcrab/ghidra-allegrex](https://github.com/kotcrab/ghidra-allegrex) adds the
Allegrex processor definition (VFPU included), a PSP ELF/PRX loader, and
relocation handling.

```sh
just build-allegrex
```

That clones, builds and verifies it, leaving an installable zip in
`data/tools/`.

### Why build rather than download

Ghidra matches the `version` in an extension's `extension.properties` against
its own `application.version` **exactly**, and upstream does not publish a build
for every point release. As of v21.3 the newest asset is
`ghidra_12.1_PUBLIC_20260520_ghidra-allegrex.zip`, which Ghidra 12.1.2 refuses.

Building from source sidesteps this entirely: Ghidra's own
`support/buildExtension.gradle` stamps the version from the installation being
built against, so the result always declares the right one. The script asserts
this rather than assuming it, because a mismatch makes Ghidra ignore the
extension without any obvious complaint.

### Build requirements

| Requirement | Why |
| --- | --- |
| **JDK 21** | The upstream Gradle wrapper is 8.10.2, which does not support JDK 23+, and the build declares `jvmToolchain(21)`. A newer default JDK fails with an unhelpful Gradle error. |
| Ghidra install | Read via `GHIDRA_INSTALL_DIR`, default `/opt/ghidra`. Ghidra's `application.gradle.min` is 8.5 with no maximum, so the bundled wrapper is in range. |
| Network | Gradle wrapper and Maven dependencies are downloaded on first build. |

The script finds JDK 21 itself on common paths; override with `JAVA_21_HOME`.
On Arch: `sudo pacman -S jdk21-openjdk`, which coexists with a newer default.

```sh
just build-allegrex --ref v21.3     # build a specific tag instead of master
just build-allegrex --clean         # discard the checkout and start over
```

### Verified build

| | |
| --- | --- |
| Output | `data/tools/ghidra_12.1.2_DEV_20260726_ghidra-allegrex.zip`, 1.9 MiB |
| Declares | `version=12.1.2` |
| Registers | language ID `Allegrex:LE:32:default` |
| Source | `aec4265`, master |

The build emits `WARN 16 NOP constructors found` while compiling
`allegrex.slaspec`. That is normal SLEIGH output, not a problem.

### After installing

1. Ghidra: `File > Install Extensions > +`, select the zip, restart.
2. **Re-import** `BOOT.BIN` rather than re-analysing the existing program. The
   processor language is fixed at import time, so changing it means starting the
   analysis over regardless.
3. Choose language **`Allegrex:LE:32:default`**, and prefer the extension's PSP
   loader over the generic ELF loader if it offers one: it understands PSP
   relocations and module metadata the generic loader ignores.

Confirm it took effect by disassembling `0x0004bdc0` again (adjusted for the new
image base). It should read `lv.q` and `sv.q`, not `ldc2` and `sdc2`.

## Also worth fixing: the image base

The program is currently loaded at image base `0x00000000`, because the ELF is a
relocatable PSP module whose `LOAD` segment has `VirtAddr 0x0`.

PSP user modules run at `0x08800000` and above, and that is what PPSSPP's
debugger will show. With base 0, **no address in our documentation matches
anything seen at runtime**, and the
[verification loop](../reverse-engineering/verification-protocol.md) depends on
being able to move between the two.

Rebase in Ghidra (`Window > Memory Map`, then the "Set Image Base" button), or
set the base at import. Confirm the correct value by loading the game in PPSSPP
and reading the module's load address rather than assuming it: `0x08804000` is
the usual value for a game's main module, but it is not guaranteed.

Do this at the same time as the re-import, so addresses are only invalidated
once.

## Status

| Item | State |
| --- | --- |
| Allegrex extension | Built and verified, in `data/tools/` |
| `BOOT.BIN` imported | Yes, 10,646 functions, but with the **wrong** processor |
| Processor language | `MIPS:LE:32:default` - needs re-import as Allegrex |
| Image base | `0x00000000` - needs rebasing |
| GhidraMCP bridge | Working, listening on 127.0.0.1:8089 |

No function has been documented yet, so nothing needs retracting. Fixing both
issues before the first analysis is the reason to do it now.
