# Reverse engineering methodology

## The loop

```
observe  ->  hypothesise  ->  verify  ->  document  ->  implement
   ^                                                        |
   +--------------------------------------------------------+
```

The order matters. Implementing before verifying produces code that looks right
and behaves subtly wrong, and the debugging happens later with less context.

**Never skip "document".** A discovery that exists only in someone's head, or
only in the Ghidra database, is not a project asset. See
[ADR-0005](../architecture/adr/0005-ghidra-conventions.md).

## Principles

**Understand behaviour, do not translate assembly.** The goal is a clean
implementation that behaves the same, not a Rust transliteration of MIPS. When a
recovered algorithm turns out to be a well-known one, say so and implement the
well-known one.

**Two binaries are better than one.** Pulse exists on PSP and PS2, and Pure is a
close relative. When two agree, confidence rises sharply. When they disagree,
that is a finding: work out whether it is an implementation difference, a
platform limitation or an intentional gameplay change, and record which.

**Prefer runtime evidence.** A breakpoint that shows a function doing what you
claim is worth more than an hour of reading. Both emulators have good debuggers;
use them early rather than as a last resort.

**Score every claim.** See the [confidence rubric](confidence-rubric.md).

**Record contradictions.** When evidence conflicts with an existing note, write
that down rather than quietly picking a side. Contradictions are where the
interesting findings are.

## Where to start

The PSP `BOOT.BIN` is an **unencrypted ELF**. This is the single most useful
fact established so far: it can be loaded straight into Ghidra with no
decryption step.

```sh
just unpack extract data/images/pulse-psp-usa.chd \
    -o data/extracted/psp 'PSP_GAME/SYSDIR/BOOT.BIN'
```

The PS2 main executable, `54748/../SCES_547.48`, is likewise a plain ELF.

See [toolchain](toolchain.md) for setting up Ghidra and the emulators.

## Suggested order

Roughly the [roadmap's](../overview/roadmap.md) M2, and roughly dependency
order. Each subsystem should end up independently understandable.

1. **Main loop.** Establishes the frame structure and, critically, the
   simulation tick rate, which everything downstream is scaled by.
2. **Memory management.** The heap layout tells you where everything else lives.
3. **Resource loading.** How a WAD entry becomes a live object. This connects
   the binary work to the [format work](../formats/README.md).
4. **Input.** Small, self-contained, and easy to verify by pressing a button.
5. **Camera.** Observable, and a gentle introduction to the math conventions:
   handedness, units, angle representation.
6. **Ship update and physics.** The core of the game, and the hardest part.
7. **Collision.**
8. **Weapons.**
9. **AI.**
10. **HUD, menus, replay, audio, multiplayer.**

Input and camera before physics is deliberate. They are the cheapest way to
establish the coordinate conventions and the structure layouts that physics then
depends on, and getting those wrong makes physics unreadable.

## Validating a decoder

Every format decoded here had a self-check available, and using it has caught
real errors nine times and confirmed a format outright three more. **Look for
the arithmetic that must hold before writing the parser, then put it in a test
rather than in prose.**

| Format | The invariant that closes it |
| --- | --- |
| [WAD](../formats/wad.md) | The offset chain. 193/193 against 2/193 resolved a transposition. |
| [`.vex` geometry](../formats/vex.md) | Each batch's declared bounding box, over 342,115 vertices. |
| `.vex` node tree | Child counts sum to one less than the node count. Exact as a `u16`; 111 million as a `u32`, which settled the width. |
| Embedded textures | Sizes sum to the declared block length. |
| [Track](../formats/track.md) | Exactly 64 `section` nodes, matching the 64-bit PVS mask. |
| `WO Track` | The payload closes on 40 of 40 files. |
| [LZSS](../formats/lzss.md) | Where the *reader* stops - every one of 6,053 streams consumed to within a byte or two of its end. |
| [PMF](../formats/pmf.md) | Header arithmetic, zero stray bytes, access-unit counts against duration. |
| [Collision](../formats/collision.md) | The walk closes on 319 of 319 nodes once 16-byte node padding is accounted for - which also pinned the chunk count at 32 bits and turned an unknown `u16` into the stride. |
| [Handling stats](../formats/handling-stats.md) | Completeness: 8 teams x 4 classes x every attribute, with a test that removes each of 174 attributes in turn and requires every removal to error. |
| The handling parameter block | 32 floats = `0x80` = the observed per-class stride, closing with no gap or overlap, which is what placed 13 previously unknown offsets. |

**An invariant beats a careful re-read, and it beats a plausible hypothesis.**
The collision walk came up 10 bytes short; the answer was 16-byte node padding,
not the `u16`-chunk-count theory the error message itself suggested and the code
was already nudging toward.

## Verifying a change with no "before" to compare against

A refactor that must not change behaviour - a crate extraction, a renderer
split - has no recorded baseline, but one can be manufactured:

```sh
git worktree add /tmp/before HEAD
# The worktree already contains a tracked data/README.md, so link data/images,
# not data.
ln -s "$PWD/data/images" /tmp/before/data/images
```

Build the tool in the worktree, capture screenshots there, capture the same ones
from the working tree, and `cmp` them. The [`oag-render`](../../crates/render)
extraction was verified this way - `--mesh` on a ship, `--track`, and a
front-end texture all came out byte-identical - which makes it verified rather
than only "behaviour-preserving by construction".

## Open questions

Live questions, with what would resolve each.

| Question | Why it matters | How to resolve |
| --- | --- | --- |
| **Does the physics integrator sub-step at 1/60?** | Decides how far our fixed timestep diverges. See [frame pacing](../psp/frame-pacing.md). | Start from `"antigrav_height_adjust"` at `0x08a7b1ac`, referenced from `0x08839880`. |
| **Is the physics fixed-point or float?** | Decides whether we can be bit-exact for free. See [ADR-0002](../architecture/adr/0002-determinism-model.md). | Look at the ship update's arithmetic instructions: VFPU ops mean float, shifts and integer multiplies mean fixed-point. |
| **What is the PRNG?** | AI and pickups will not match without it. | Find the seed's storage, trace back to the update function. Compare against known small PRNGs. |
| **Are PSP textures swizzled?** | Decides whether decoded pixels display correctly. | Needs a renderer; see [PSP texture](../formats/psp-texture.md). |
| **What LZSS variant do the PS2 archives use?** | Blocks PS2 asset work. See [WAD format](../formats/wad.md). | `Lzss_Decode` at `0x089419d8`; the bit layout is read but never round-tripped. |
| **Why does the US PSP disc carry a `UCES00465` directory?** | Unknown. Possibly nothing, possibly a build-provenance clue. | Compare the two `BOOT.BIN` files. See [PSP disc layout](../psp/pulse-disc-layout.md). |
| **Do PSP and PS2 share gameplay constants?** | Decides whether "gameplay is identical across asset sets" holds. See [ADR-0004](../architecture/adr/0004-asset-pipeline.md). | Locate the handling table in both, compare values. |

## Anti-patterns

**Renaming on vibes.** If you cannot write down the evidence, the confidence is
below 50 and the function keeps its address name.

**Implementing before verifying.** The code will look right.

**Trusting a tool's output as fact.** Ghidra's decompiler is very good and
sometimes wrong, particularly around MIPS delay branches and register-passed
structures. Check the disassembly when a reading matters.

**Trusting an AI's conclusion as fact.** Same rule, more so. An agent driving
Ghidra can generate a plausible reading of anything. Every claim needs evidence
recorded against it, and the evidence must be checkable by someone else.

**Leaving a contradiction unrecorded** because resolving it would take time. Write
it down; that is the cheap part.
