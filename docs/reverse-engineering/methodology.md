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

## Open questions

Live questions, with what would resolve each.

| Question | Why it matters | How to resolve |
| --- | --- | --- |
| **Confirm the simulation tick rate.** Defaulted to 60 Hz; unverified. | Every physics constant is scaled by it. | Find the main loop's frame pacing, or count ticks between two known events in a trace. |
| **Is the physics fixed-point or float?** | Decides whether we can be bit-exact for free. See [ADR-0002](../architecture/adr/0002-determinism-model.md). | Look at the ship update's arithmetic instructions: VFPU ops mean float, shifts and integer multiplies mean fixed-point. |
| **What is the PRNG?** | AI and pickups will not match without it. | Find the seed's storage, trace back to the update function. Compare against known small PRNGs. |
| **How are WAD entries named?** | Blocks all asset work. See [WAD format](../formats/wad.md). | Find the lookup function in `BOOT.BIN`; the hash algorithm will be right there. |
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
