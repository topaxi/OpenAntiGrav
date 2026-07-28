# Comparisons

Differences between platforms and between titles. Every difference gets
classified, because "PSP and PS2 differ here" is only useful once you know
*why*:

| Class | Meaning |
| --- | --- |
| **Implementation** | Same intent, different code. No gameplay consequence. |
| **Platform limitation** | Forced by hardware. Texture resolution, audio codec, draw distance. |
| **Intentional change** | A deliberate design decision between releases. Gameplay consequence. |
| **Region** | A difference between territories, not platforms. |
| **Unclassified** | Observed, not yet explained. |

Only *intentional changes* affect what we implement. Confusing one of the others
for an intentional change means reproducing a hardware workaround as if it were
design.

## The region confound

**Our PSP copy is US (`UCUS-98712`) and our PS2 copy is EU (`SCES-54748`).**

Every PSP-versus-PS2 comparison in this project is therefore also a
US-versus-EU comparison. Before classifying anything as a platform difference,
ask whether region explains it.

Acquiring a matching-region pair, or a second copy of either release, would
remove the confound. Worth doing before the comparison work in M2 gets serious.
See [source images](../reverse-engineering/source-images.md#region-asymmetry).

## Documents

| Document | Covers |
| --- | --- |
| [Pulse: PSP vs PS2](pulse-psp-vs-ps2.md) | The two Pulse releases |
| [Pure status](../formats/pure-status.md) | Pure vs Pulse at the asset-format layer |

## Cross-title

Whenever something is understood, ask whether it exists in Pure, HD/Fury, 2048
or Omega. Shared concepts go in
[future-2048/shared-concepts.md](../future-2048/shared-concepts.md).

Avoid Pulse-specific assumptions where a general abstraction costs nothing. The
project's whole premise is that the second title should be cheaper than the
first.
