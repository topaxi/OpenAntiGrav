# Confidence rubric

Every reverse-engineering claim carries a score from 0 to 100. The score says
how much weight the claim can bear, and it is the difference between a knowledge
base and a pile of guesses.

## Scale

| Score | Meaning | What it took |
| --- | --- | --- |
| **95-100** | Established | Verified against a runtime trace **and** corroborated in a second binary (PSP and PS2, or Pulse and Pure). |
| **85-94** | Confident | Decompilation is unambiguous and every call site is consistent with the reading. Not runtime-verified. |
| **70-84** | Probable | Strong structural evidence: signature, control flow and data access all fit. Not verified in either sense. |
| **50-69** | Plausible | A reasonable inference from surrounding code. Could be wrong. |
| **0-49** | Guess | A hypothesis. **Do not rename the function.** Write the hypothesis down instead. |

## Rules

**Below 50, do not rename.** A guess dressed as a name stops other people from
looking. Leave `FUN_08831af0` and record what you suspect.

**Below 70, suffix the name with `_q`.** The uncertainty must be visible at every
call site, not only on the documentation page. See
[ADR-0005](../architecture/adr/0005-ghidra-conventions.md).

**Runtime verification is worth more than any amount of reading.** A single
breakpoint that shows the function doing what you claim moves a 70 to a 90 in
minutes. Reading the decompilation harder does not.

**A second binary is worth more than a second reading of the first.** If PSP and
PS2 both do the thing, the reading is probably right. If they disagree, that is
a finding.

**Score the claim, not your feelings.** "I am sure of this" is not evidence.
The score answers: what would someone else need in order to check this?

## Recording evidence

State what you actually observed, not what you concluded:

> **Confidence: 88**
>
> - Reads the analog stick state at `0x0891_2340`, which
>   [`Input_PollPad`](#) also writes.
> - Writes a float to offset `0x1c` of the structure passed in `a0`; that offset
>   is read by `Ship_ApplyRotation` and used as an angle.
> - Called once per iteration from `Game_SimulationStep`, before physics
>   integration.
> - Not runtime-verified, so capped below 95.

The last line matters as much as the others. Saying explicitly why a claim is
*not* higher tells the next person what work would raise it.

## Revising a score

Scores move as evidence arrives, in both directions. When a score changes,
record why:

```markdown
## History
- 2026-07-26: 65, inferred from call site position.
- 2026-08-02: 88, confirmed by breakpoint trace in PPSSPP.
- 2026-08-14: 45, contradicted by the PS2 build, which calls it from the
  audio update. Renamed back to FUN_08831af0 pending re-analysis.
```

A score that only goes up is a score nobody is checking.
