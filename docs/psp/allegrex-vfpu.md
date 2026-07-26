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

## Verified before and after

Same bytes, same file, both readings from Ghidra. Addresses differ only by the
image base (`+0x08804000`).

The dense vector run:

| Bytes | Stock `MIPS:LE:32` | With Allegrex |
| --- | --- | --- |
| `9007b0db` | `ldc2 s0, 0x790(sp)` | `lv.q C400, 0x790(sp)` |
| `8007b1db` | `ldc2 s1, 0x780(sp)` | `lv.q C410, 0x780(sp)` |
| `4000b0fb` | `sdc2 s0, 0x40(sp)` | `sv.q C400, 0x40(sp)` |

64-bit generic coprocessor transfers, versus 128-bit vector loads and stores
naming actual VFPU registers.

The arithmetic case is starker. At `0x08814e7c`, bytes `0a888964`:

```
stock:     daddiu t1, a0, -0x77f6      # a 64-bit integer add the PSP cannot execute
allegrex:  vdot.t S220, C200, C210     # a 3-component dot product
```

A 3D dot product was being read as a nonexistent integer instruction. Nothing
about the stock output looked wrong, which is the entire problem.

Reproduce:

```
disassemble_bytes start_address=0x0884fdc0 length=48 dry_run=true
disassemble_bytes start_address=0x08814e7c length=16 dry_run=true
```

## Setting the image base

The PSP loader leaves the program at image base `0x00000000`, because the ELF is
a relocatable module whose `LOAD` segment has `VirtAddr 0x0`. It hardcodes no
default.

PSP user modules run at `0x08800000` and above, and that is what PPSSPP's
debugger shows. Left at 0, **no address in our documentation matches anything
seen at runtime**, and the
[verification loop](../reverse-engineering/verification-protocol.md) depends on
moving between the two.

### Rebasing is safe here, which is not generally true

Ghidra's "Set Image Base" normally just slides the memory blocks. Any pointer
already written into `.data` by a relocation keeps its old value, so afterwards
the data points at addresses where nothing lives. On most targets, rebasing a
relocated binary after load quietly corrupts it.

The Allegrex extension ships an `AllegrexRelocationFixupHandler`, a Ghidra
`RelocationFixupHandler` that Ghidra invokes on an image base change and which
**re-applies every Allegrex relocation against the new base**. Rebasing is a
supported operation, not a workaround.

It declines to handle `ET_REL` object files, but `BOOT.BIN` is type `0xffa0`
(PSP PRX), so it is covered. The handler stashes and restores instructions, so
it also works after analysis, though rebasing first is cheaper.

### How

1. `Window > Memory Map`
2. Click **Set Image Base** in that window's toolbar (the small house icon)
3. Enter `08804000`
4. Then `Analysis > Auto Analyze`

**Rebase before analysing.** Both orders work, but analysing first means paying
for it twice.

### On the value

`0x08804000` is the conventional load address for a game's main module and is
the right default. It is a convention, not a guarantee: the allocator decides,
and a module that reserves memory differently can land elsewhere.

Confirm it against PPSSPP when convenient. Load the game, open
`Tools > Developer tools`, and read the module's load address. If it differs,
rebase again; the fixup handler makes that repeatable rather than destructive.

Verify what Ghidra currently thinks:

```sh
curl -s http://127.0.0.1:8089/get_current_program_info | jq -r '.image_base, .language'
```

## Status

**Resolved.** The environment is ready for M2.

| Item | State |
| --- | --- |
| Allegrex extension | Built, installed, verified |
| `BOOT.BIN` imported | Via the `PSP Executable (ELF)` loader |
| Processor language | `Allegrex:LE:32:default` |
| Image base | `0x08804000` (conventional; confirm against PPSSPP) |
| Auto-analysis | Complete. **10,683 functions**, 51,549 symbols |
| VFPU decoding | Verified: `lv.q`, `sv.q`, `vscl.q`, `vdot.t` |
| GhidraMCP bridge | Connected. Auto-discovers over UDS; pass it no arguments. |

No function had been documented before the fix, so nothing needed retracting.
That is the only reason this was cheap, and the reason to check a processor
module is correct before rather than after doing the analysis.
