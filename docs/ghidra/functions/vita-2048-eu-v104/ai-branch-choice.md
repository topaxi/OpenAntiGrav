# AI branch choice (Wipeout 2048)

**Binary:** `vita-2048-eu-v104` `eboot.elf`, image base `0x81000000`.

**Status:** read 2026-10-05 (ai-forks lane), decompile only. **Not
runtime-verified**; no Vita capture path was used. The Pulse page,
[ai-branch-choice.md](../psp-pulse-usa/ai-branch-choice.md), holds the full
reading of the same law; this page records where 2048 matches it and where it
adds to it.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x81198708` | `Ai_Construct` | 80 |
| `0x8119ae90` | `Ai_ChooseBranch` | 80 |

## `Ai_Construct` (`0x81198708`)

Identified by its strings and their order, the same as Pulse's `Ai_Construct`
(`0x088536bc`): `"autopilot input %d"` or `"AI input %d"` formatted into the
record's registry name depending on `craft+0x58dc == 0` (Pulse: `craft+0x368`),
then `"AI track data"` looked up and its handle kept at `ai+0x110` (Pulse:
`ai+0x34`). It stores `-1` at **`ai+0x13c`** (`0x8119899e str.w r2,[r4,#0x13c]`)
- the excluded path, Pulse's `ai+0x60`.

Two things Pulse's constructor does not have:

- `ai+0xc8 = rand() % 2`, a per-craft coin drawn once at construction. Its
  reader was not found; it is **not** read by `Ai_ChooseBranch`.
- Two flags from string compares of the circuit record's path (`+0x74`):
  `ai+0x5c0` set on `05_Track` when `DAT_8153fd18 == 3`, `ai+0x5c1` on
  `20_Track`. Their readers were not followed.

## `Ai_ChooseBranch` (`0x8119ae90`)

Same three states on `ai+0x134` (Pulse `+0x58`), the same path index in
`ai+0x138` (`+0x5c`), the same excluded path in `ai+0x13c` (`+0x60`). The cursor
is `craft+0x898` (Pulse `craft+0x248`).

**State 0, the coin: identical.** The current path's exit junction (`+0x10`),
its alternate (`+0x0c`) tested non-zero, `SceLibc_C0883865()` (`rand`), and
`(r & 0x100) == 0` picks `junction+0x08` (primary) else `+0x0c` (alternate) as
the path to exclude.

**An authored per-circuit override follows the coin** - the one input Pulse
does not have. When `FUN_81001982(&DAT_8153fc40)` is non-zero and the circuit
record (`DAT_8153fe00`, the record whose `+0x74` is the circuit's path string,
[zone-environment-fallback.md](zone-environment-fallback.md)) has a non-zero
`+0x150`:

```text
if per_craft_value >= (float) circuit.+0x154      // a table at +0x2b2c4, 0xc stride, by craft+0x58d4
   && rand() * 2^-31 <= circuit.+0x158:
    circuit.+0x150 == 1 -> exclude next_primary   (take the alternate)
    circuit.+0x150 == 2 -> exclude next_alternate (take the primary)
```

What fills `+0x150`/`+0x154`/`+0x158`, and what the per-craft table holds, was
**not found**: no string in the executable names either. Until it is, the
override is recorded and not ported.

**State 1: identical.** **State 2, the reset: identical** (`ai+0x134 = 0`,
`ai+0x13c = -1` once the cursor is past the merge).

**State 2, the re-commit, differs.** The Pulse rule (relocate on the sibling,
compare `|across| * 10 + along`, adopt the sibling if it scores better) is
still there, but only when the craft is **off the track and moving**. When it
is on the track or slow, 2048 runs a second rule Pulse does not have: with
a timer at `ai+0x160` at or below zero and `ai+0xcc` clear, it walks the other
craft within `150` units (`22500.0` squared), keeps the ones that are the
player's (`craft+0x58dc == 0`) and not exploding (`+0x60a4 & 0x1000`), and for
one whose heading differs (`dot < 0.91`) and that sits ahead within a cone
(`0.707 + 0.0018666667 * distance`), switches to the sibling path if the
relocated record there is on the track and nearer. With the timer running it
instead switches when the sibling is within `15` units of a stored point
(`ai+0x148`). Read from the decompile only; the NEON comparisons were not
checked instruction by instruction. **Not ported.**

## Status against the port

The override, the per-craft construction coin and the 2048 re-commit are not
ported; what is, is recorded on
[`docs/gameplay/ai.md`](../../../gameplay/ai.md#branch-choice-at-a-fork).
