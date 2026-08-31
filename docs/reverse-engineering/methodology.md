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

## Rules learned the expensive way

Each of these cost at least a session before it was written down. They lived on
`HANDOVER.md` until 2026-08-09; they are here because they are properties of the
method rather than of any pass.

**The cheap instrument keeps outvoting the expensive evidence, and the direction
it points is what disguises it.** Two failures on the same thread, two days
apart, opposite in direction and identical in shape. Six days went into a
lamp-centroid metric that contradicted a background nobody had registered - and
it *felt like diligence*, because the number kept moving and each pass refined
it. Then a 3-of-16-tick fit rejected a driver that a store reads unambiguously
at instruction level - and it *felt like rigour*, because it arrived with a
t-statistic and a confidence interval. Both are a cheap, re-runnable measurement
overruling an expensive one, and **neither announces itself as that**; a
pixel metric and a regression are equally easy to keep re-running, which is
exactly why they accumulate apparent weight.

The habit that catches it is not general skepticism. It is two questions, and
the second is the one this project kept arriving at late:

1. **Name the control that could refute you.** If you cannot name one, that is
   the finding. (Worked example, four instances deep, on
   [the rendering page](../rendering/README.md#measuring-a-renderer-change).)
2. **Name the reading under which the harder evidence survives your result
   anyway.** Six days needed "the background registers at 1.000 - has anyone
   checked?". The three ticks needed "what would make a dot-product store look
   off-axis?" - and that one has an answer: a *smoothed* velocity, which lands
   between the two candidates precisely where the divergence lives. Asking it is
   what stopped the second failure from becoming another six-day one.

Question 2 is the cheaper of the two and the one more often skipped, because a
result that survives question 1 feels finished.

**Decode, then ask "can this change a pixel?", then report - not the other way
round.** Three times in one session a recovered GE mechanism was reported as a
finding and turned out to be **inert**: the boost plume's alpha test
(`GU_GREATER, 0` discards exactly the fragments a `SrcAlpha` blend already
weights to zero), its colour test (an RGB-zero fragment adds nothing under an
additive blend - redundant *algebraically*, for any content), and the spark
streak's `v` layout (a real difference, but `oag_render::sparks` binds no
texture at all). Each check was cheap - a vertex-alpha histogram, one line of
algebra, a grep for a texture bind - and each would have taken minutes *before*
the write-up rather than after. The general form: **recovering a GE call is not
by itself an argument for implementing it.** A fixed-function console programs
per-fragment tests to avoid paying for writes the blend would have made
invisible; fill rate is the scarcest resource on the part. On a reimplementation
with a different fill budget those are recovered-but-inert state - correct to
document, wrong to port, and actively harmful to port as a "fix".

**When something animates or glows, look at the texture-coordinate path
first - but confirm before concluding, because this engine has already faked
it once.** Three separate subsystems on this project turned out to move a
texture rather than move geometry, and each was found late:

| Mechanism | Where | How it animates |
| --- | --- | --- |
| a scroll advanced inside a draw loop | `Trail_DrawRibbon`, per layer, into `Gu_TexOffset`/`Gu_TexScale` | on a 60 Hz clock |
| a per-material `TEXSCALE`/`TEXOFFSET` display list | built by `FUN_08927358` from a **curve evaluator** (`FUN_08927204`), gated on the material's `& 0x10` bit, replayed by `FUN_089271cc` on one draw path and `FUN_0892733c` on the other | on a time at `mesh+0x40` |
| environment-mapped UV generation (uvgen 2) | `Mesh_BeginTransparentPass`, from the vertex normal and two fixed light vectors | as the **model** turns, not on a clock |

The third is the trap inside the trap: it animates without any clock, so a
search for "what advances the offset" finds nothing and the effect still moves.

**And the counterexample is why this is a hint rather than a rule.** The ship's
shoulder lights measurably pulse - 120 frame-accurate screenshots, pixels
sampled at a real light - and [`vex.md`](../formats/vex.md) read that as the
engine scrolling the shared texture's V globally. A breakpoint on
`Gu_TexOffset` through a live race, 60 hits, found the exhaust ribbon to be the
**only** non-zero offset the engine ever submits; no track surface and no ship
receives one. The pulse is real and the mechanism was *not* a coordinate
scroll. That cost a long pass, and the standing candidate is an animated
*colour* instead - `Trail_DrawRibbon` already sets a per-draw colour through
`Gu_Ambient`, and a per-object colour on a clock pulses a light without moving
a UV or rewriting a palette.

So: check the UV path early because it is cheap and it is often right, then
**put a breakpoint on the two primitives before writing it down**. "It looks
like a scroll" has already been wrong once, at length, and the write-up that
believed it had to be retracted from a format page rather than a scratch note.

**An xref count on a class descriptor bounds *authored* instances only.** One
pass concluded that racing craft have no exhaust trail, because `Trail`'s class
descriptor is referenced by nothing but its own registration function and no
`Ship.vex` authors a `Trail` node. Both facts are true; the conclusion is false.
The descriptor is used **only** by the class-table lookup the `.vex` loader
takes to instantiate a node by ID - code that builds an object directly calls
its constructor and assigns the vtable itself, which is exactly what
`ExhaustFlare_Init` does. What refuted it was **looking at the emulator**, which
shows a trail plainly. Take a side-by-side before concluding anything is absent
from a *visible* subsystem. The same asymmetry runs the other way: "has
instances" and "has a handler" are independent properties of a class -
`exitglow` is authored 13 times on `16_Track` and has no registration site at
all - so no census reads as a to-do list.

**A claim that the crate is missing a term is a claim about the crate.** It has
been wrong three times - the airbrake drag term, the hover downforce magnitude
and the roll-oscillator note were each recorded as unimplemented in prose nobody
had checked against the code, and twice the correct value was sitting in the
same file a hundred lines above the claim.

**A page can go stale while a sibling page in the same tree already carries the
correction**, and so can the code. It has happened four times (the front-end
font claim; the airbrake drag term in four places; M4's blocker for 31 commits;
and `rigid-body.md` recording that `<Misc>`'s dimensions are scaled by `0.75`
before reaching the box collider while `crates/physics/src/wall.rs` built that
box unscaled for two days). `git log -S <symbol>` against the doc's own
last-touched commit settles it in one command, and `git grep` for a constant
across `docs/` costs less than rediscovering the instruction sequence behind it.

**Never write a fitted constant in as a recovered value.** Every one that has
been chased turned out to be something real - an inverse-inertia entry, a box
tensor, a friction coefficient - and writing the fit in would have closed the
question at exactly the wrong moment.

**A number in authored data that looks like a height usually is not one.**
`Start Position`'s `y` reads exactly like a ride height on `01_Track` (2.42,
between the surface and the lifted spline) and is not one: measured against each
track's own collision mesh it ranges **1.03 to 7.36** across the 40 files.
Checking it on one track would have shipped a spawn that only works there. Ask
what an authored constant is a height *above*, then measure that on every file
rather than the convenient one.

**When position and heading disagree about whether a reading is right, the
heading is the one that can carry the argument.** The authored grid slot is
139.7 units from where the original's craft actually starts, which is equally
consistent with a misread matrix and with a grid behind the line. Its forward
agreeing to 1.12 degrees is consistent with only one of those. Rotations have no
free parameters; positions have three.

**"Read far enough to rule X out" is how 80 instructions of a different
mechanism stay unread for three passes.** `Ship_UpdateMagLock` had been opened
twice, each time only to the depth that day's question needed; its tail turned
out to rewrite the basis directly and was the whole of the remaining attitude
gap.

**Time-boxing pays anyway.** Two unread leads (the PS2 PAL/NTSC selector's
trigger, the `.IPF` container) were left alone deliberately and the video got
playing regardless, because the measured facts did not need them.

**Diff the output, not the compile.** A shader uniform field existing is not the
same as a shader reading it: `Draw::Video`'s `rect` was computed and discarded
on the way to the GPU, and the first "fix" produced a pixel-identical screenshot
while silently doing nothing.

**A fidelity fix that changes nothing measurable is still a finding.**
`raycast_all` moved the whole-lap scenario by not one digit and single-sided
rejection moved it by 1.1 units - both real divergences from
`Collision_BoxAgainstMesh` that `16_Track`'s geometry never exercises. Written
down, that is knowledge; unwritten, it gets re-litigated. (Both figures were
measured from the old spline-sample-0 spawn and do not reproduce since `4b2236a`
moved the start onto the authored grid slot; the finding stands, the numbers do
not.)

**A sweep's negative is only as good as its addressing model, and a sweep that
cannot find a known positive is not evidence about anything.** Five instances
in one day on `ps3-hdfury-eu`, each a *clean* negative that was wrong or nearly
wrong, and none of which announced itself:

- Four independent sweeps failed to find what writes eight per-frame vec4s, and
  the page concluded an emulator watchpoint was the only route left. All four
  keyed on `displacement(base)`; the stores are indexed `stvx rV, rA, rB`, which
  carries no displacement. A fifth sweep found the writer statically in one
  pass. **"No displacement to grep" is a reason a displacement sweep fails, not
  a reason static analysis fails** - and the two get conflated precisely when
  the first sweep was expensive enough to feel exhaustive.
- Two searches for what consumes a fixed address came back empty - no xrefs, no
  `addi` forming it - and the address turned out to have two more consumers. The
  pointer is computed in a two-instruction getter, returned in `r3`, spilled to
  the caller's stack frame and selected by a runtime flag: **no literal for an
  xref, no `addi` in the consuming function, no constant offset near the
  store.** Both searches were source-side and shared one blind spot.
- The destination-side sweep that should have caught it returned its own clean
  negative, from a register-model defect: `llvm-objdump` prints `lfs 31,` and
  `lwz 31,` identically, so a float load silently clobbered the tracked base.
- A disassembler dropped the per-instruction condition code, so both arms of a
  predicated selection printed as straight-line code. The natural reading of
  that output is that one arm is dead, which is how a whole mechanism stayed
  hidden behind a tool that never errored.
- The same tool described a parameter patch chain in its own docstring and did
  not implement it, printing `{0, 0, 0, 0}` where a named parameter belonged.

The habit that catches all five is one check, and it is cheap:

**Before believing a sweep's negative, confirm the sweep recovers a positive you
already know.** If it cannot re-find the answer you have, it is not evidence
about the answers you do not. Write the control into the sweep itself rather
than running it once by hand - the defect above survived three re-runs because
each re-run asked the same question of the same broken model.

And when a search does come back empty, **name the addressing form it was blind
to** before concluding anything: displacement versus indexed, source-side versus
destination-side, literal versus computed, direct versus spilled-and-reselected.
A negative with that sentence attached is publishable. Without it, it is a
report about the tool.
