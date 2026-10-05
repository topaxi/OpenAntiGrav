# AI branch choice: which side of a fork an opponent takes

**Binary:** `pulse-psp` `BOOT.BIN`, image base `0x08804000`.

**Status:** read 2026-10-05 (ai-forks lane), decompile plus an
instruction-level read of every load-bearing branch. **Not runtime-verified.**
The same law was found, at the same structure, in three more binaries (Wipeout
2048, Wipeout HD/Fury, Omega Collection) - see [other titles](#other-titles).

| Address | Name | Confidence |
| --- | --- | --- |
| `0x08854920` | `Ai_ChooseBranch` | 84 |
| `0x0887d39c` | `AiTrack_SampleAhead` | 80 |
| `0x08852318` | `Ai_Rand` | 90 |
| `0x08852334` | `Ai_RandUnit` | 90 |

## The law, in one paragraph

**Every opponent flips a fair coin at every fork, independently, every time it
reaches one.** When the craft's own track cursor enters a path whose exit
junction has an alternate successor, `Ai_ChooseBranch` draws `rand()` and keeps
**bit 8**: set, it excludes `next_alternate` (so the craft takes the primary);
clear, it excludes `next_primary` (so it takes the alternate). Nothing else
enters the choice in Pulse: not position, speed class, difficulty, the pilot,
the corridor, or anything authored per branch. The excluded path is then the
one argument every later walk of that craft's track honours, its lookahead
and its lap progress included, until the craft is past the merge.

## `Ai_ChooseBranch` (`0x08854920`)

Called from `Ai_Update` (`0x08853a80`) once per tick, right after the lateral
target (`FUN_08854e7c`). Returns at once unless `craft+0x360` is set (the same
gate `FUN_08854e7c` uses). Its state lives on the AI record:

| Field | Meaning | Written by |
| --- | --- | --- |
| `ai+0x58` | state: `0` undecided, `1` decided, `2` on the far side of the fork junction | this function |
| `ai+0x5c` | the path index the craft was on when the state last moved | this function |
| `ai+0x60` | the **excluded path**, `-1` for none | this function; `Ai_Construct` (`0x088536bc`) initialises it to `-1` (`param_1[0x18] = 0xffffffff`) |

The cursor is the craft's own, at `craft+0x248` (`{track, path_index, point,
point_ptr}`); the path record is `track->paths + path_index * 0x20`, its exit
junction at `path+0x10`, and a junction's successors at `+0x08` (`next[0]`,
primary) and `+0x0c` (`next[1]`, alternate) - the layout of
[track.md](../../../formats/track.md#junctions).

**State 0** (`0x088549f4`-`0x08854a60`):

```text
088549f4: lw   s2,0x10(s1)        ; exit junction of the current path
08854a00: lw   a0,0xc(s2)         ; its alternate successor
08854a04: bne  a0,zero,...        ; no alternate -> nothing to choose
08854a1c: jal  0x08852318         ; Ai_Rand -> rand()
08854a24: andi a0,v0,0x100        ; bit 8
08854a2c: beql a0,zero,...
08854a30: _lw  s2,0xc(s2)         ;   bit set:   s2 = next_alternate
08854a38: _lw  s2,0x8(s2)         ;   bit clear: s2 = next_primary
08854a40: jal  0x0887d37c         ; AiTrack_PathIndex(s2)
08854a4c: sw   v0,0x60(s0)        ; excluded = that path
08854a58: sw   v0,0x5c(s0)        ; ai+0x5c = current path
08854a60: sw   a0,0x58(s0)        ; state = 1
```

The value stored is the path **not** to take: `AiTrack_SampleAhead` below takes
`next_alternate` exactly when `next_primary` is the excluded one. So bit 8 set
is "primary", clear is "alternate", each with probability one half - `rand` is
the newlib LCG ([prng.md](prng.md), confidence 95), whose bit 8 has period 512
and is balanced over it.

**The decision is made on entry to the path before the fork**, not at the
junction: state 0 fires on the first tick the craft's cursor sits on a path
whose exit is a fork. The whole of that path is driven already knowing which
side it will take.

**State 1** (`0x08854a6c`): waits for the cursor's path to differ from `+0x5c`
(the craft has crossed the fork junction), then records the new path in `+0x5c`
and moves to state 2.

**State 2** (`0x08854aa4`): if the cursor's path is neither `+0x5c` nor the
excluded path, the craft is past the merge: `+0x60 = -1`, state 0
(`0x08854ad4`-`0x08854ae0`). The next fork rolls afresh. Otherwise it is still
on the branch it chose (or, the `+0x60` comparison says, on the one it did not),
and the **re-commit** below runs.

### The re-commit: a craft knocked onto the other side keeps it

Still in state 2, from `0x08854ae4`. With the located record at `craft+0x260`
(position `+0x00`, tangent `+0x20`, lateral `+0x30`, half-widths `+0x44`/`+0x48`):

- `along = |dot(pos - rec.pos, rec.tangent)|` (`0x08854b34`);
- `across = (dot(pos - rec.pos, rec.lateral) + hw_left) / (hw_left + hw_right)`,
  folded to the excess outside `[0, 1]` - `0` inside the track, `x - 1` past the
  right edge, negative past the left (`0x08854b8c`-`0x08854be8`).

If the craft is **on the track** (`across == 0` and `along < 5.0`,
`0x08854c34`-`0x08854c60`) or **barely moving** (`craft->+0x94->+0x2b0 < 0.1`,
`0x08854c68`), nothing happens. Otherwise `AiTrack_LocateOnSiblingPath`
(`0x0887d4d4`) relocates a copy of the cursor on the sibling path, the same two
numbers are computed there, and if `|across'| * 10 + along' < |across| * 10 +
along` the craft is better placed on the sibling: the excluded path becomes the
current one and `+0x5c` the sibling (`0x08854e10`-`0x08854e58`). So a craft
shoved across the divider follows the side it is on rather than fighting its
way back.

A third branch (`0x08854bec`-`0x08854c30`) applies only to the player's own
record (`ai+0xa8`, set by `Ai_Update` for `DAT_08b34418`'s craft) with `ai+0x21`
clear: if the player is on the excluded path, `+0x5c` and `+0x60` swap. That is
the autopilot following the player's own choice; it never runs for an opponent.

## Who honours the excluded path

`Ai_Update` copies `ai+0x60` into **`craft+0x8f0`** every tick (`0x08853c08`
region: `*(param_2[1] + 0x8f0) = param_2[0x18]`; the player gets `-1` unless
`ai+0x21`). Three callers then pass `craft+0x8f0` as `AiTrack_LocatePosition`'s
`excluded_path` argument (`t0`, see [track.md](../../../formats/track.md#locating-a-ship-on-the-track)):

| Caller | Instruction | What it locates |
| --- | --- | --- |
| `Craft_UpdateLapProgress` (`0x08842a18`) | `0x08842af4 lw t0,0x8f0(s0)` then `jal 0x0887ce78` | the craft's lap progress cursor `craft+0xad8` |
| `Ship_UpdateRespawn` (`0x08847914`) | `0x08847aac lw t0,0x8f0(s0)` then `jal 0x0887ce78` | the respawn point |
| `FUN_0883ff6c` | `0x0883fff8 lw t0,0x8f0(s0)` then `jal 0x0887ce78` | the same shape as the respawn call, radius `500.0` |

**So lap progress is read off whichever branch the craft is on.** The progress
value is the point's own `+0x40` field (normalised arc position round the
circuit, [track.md](../../../formats/track.md#control-points)), which the
exporter authors on a branch's points as well as the primary's: a craft on
either side reads a progress on the same scale, and no lap is gained or lost.

## `AiTrack_SampleAhead` (`0x0887d39c`)

`(track, out[], cursor, distances[], excluded_path, count)`. Walks forward from
the cursor point by point, summing each point's `+0x67` byte (the
`dist_to_prev` the loader writes), and records the point where the running sum
first exceeds `distances[i] * 20.0`, for `count` distances. At a path end it
takes the exit junction's `+0x08` successor, **or `+0x0c` if `+0x08` is the
excluded path** - the traversal recorded in
[track.md](../../../formats/track.md#topology-a-graph-not-a-ring), here with its
caller. `Ai_Update` calls it with `count = 3` from `craft+0x248` and
`ai+0x60`: the three lookahead points the steering reads follow the chosen
branch before the craft reaches it. Confidence 80: read whole, one caller.

## `Ai_Rand` and `Ai_RandUnit`

`0x08852318` is a bare `jal 0x089731c4` (`rand`) and return. `0x08852334` calls
it and multiplies by `0x30000000` (`2^-31`): a unit draw in `[0, 1)`.
`Ai_Construct` uses `Ai_RandUnit` for four per-craft constants
(`ai+0x1a0`/`+0x1a8` times `100.0`, `ai+0x1a4`/`+0x1ac` as
`(u * 0.5 + 1.0) * 0.2`). Confidence 90: three instructions each.

## Other titles

Recorded per the repository rule that a 2048 finding is checked against Omega
and the reverse; HD is checked because Omega's lineage runs through it.

| Title | Binary, address | Status |
| --- | --- | --- |
| **Wipeout 2048** (Vita, v1.04) | `Ai_ChooseBranch` `0x8119ae90`, `Ai_Construct` `0x81198708` - see [the 2048 page](../vita-2048-eu-v104/ai-branch-choice.md) | **checked, applies, with additions.** Same three-state machine (`ai+0x134` state, `+0x138` path, `+0x13c` excluded, initialised `-1`), same `rand() & 0x100` coin on junction `+0x08`/`+0x0c`. Two additions: an authored per-circuit override after the coin, and a different re-commit. |
| **Wipeout HD/Fury** (PS3) | `FUN_000fe818`, `0x000fe8a8`-`0x000fe8d8` | **checked, applies.** `lwz r0,0x10(r27)` (exit junction), `lwz r0,0xc(r30)` (alternate, zero test), `bl 0x006762f8` (`rand`), `rlwinm r3,r3,0,23,23` (bit 8), then `lwz r29,0x8(r30)` (primary). It is followed by a mode check (`lwz r0,0xe0(r26)`, `cmpwi r0,0xd`) not read further. |
| **Omega Collection** (PS4) | `FUN_012c0480`, `0x012c345a`-`0x012c3494` | **checked, applies.** `movsxd rax,[r13+0xc]` (alternate) against `0x7fffffff`, `call 0x017a59f0` (`rand`), `test ah,1` (bit 8), else `movsxd rax,[r13+0x8]` (primary). Not read beyond the coin. |

## What would raise these

- A PPSSPP watchpoint on `ai+0x60` for a craft approaching `05_Track`'s
  junction 1, over a few dozen races: the split should sit near half and the
  write should land on entry to the pre-fork path. That moves
  `Ai_ChooseBranch` past 90.
- `FUN_08854e7c` (the lateral target that runs before this) is read but not
  named: it chooses one of nine lateral lanes from a smoothed score and is a
  separate question from which branch.
