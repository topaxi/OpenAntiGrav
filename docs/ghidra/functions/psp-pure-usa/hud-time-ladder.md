# Pure's Time Trial clock: the medal ladder and its four captions

**Binary:** `psp-pure-usa` `BOOT.BIN`, image base `0x08804000`. Read 2026-10-22 (`pure-fidelity`
lane), disassembly by `llvm-objdump --triple=mipsel`. No `names.tsv` row: the function stays
`FUN_0892195c` here (the label `PlayerStatus_Update` in code comments is this page's, not a recovered name).

## What the original does

`FUN_0892195c` (`0x0892195c`, a per-tick player status update) runs `0x08921f60`-`0x08921fe0` in Time
Trial. Per race it builds three limits in centiseconds from the circuit's `stats.xml`:

- gold = `trunc(<RaceTimes class> * 100)`, the class's own attribute (Pure has five classes, Vector included);
- silver = gold + `trunc(Silver * 100)`, bronze = gold + `trunc(Bronze * 100)`.

The stored best (`0` and `0xffff` both mean none) picks the **next** medal to chase, written to
`+0x7c` of the status object, and the target the clock counts down to:

| stored best | caption (`+0x7c`) | target |
| --- | --- | --- |
| none, or slower than bronze | `HUD_Bronze time` | bronze |
| slower than silver | `HUD_Silver time` | silver |
| slower than gold | `HUD_Gold time` | gold |
| at or under gold | `HUD_Record time` | the best itself |

The countdown is `target - trunc(clock * 100)` in centiseconds, `0` and the clock red once past it; the HUD
writes it with the tenths format Pulse's clock uses. The four caption keys sit at `0x08a46428`-`0x08a46458`
and resolve in Pure's language table. Pulse's law (a campaign cell's own target) is not used: Pure
races this ladder in Time Trial alone. Confidence 90: the branch table read in full, the
fresh-profile caption and time read off four live frames.

## Pure's `stats.xml`

`<RaceTimes>` carries five class attributes and `Silver`/`Bronze` offsets (Vineta K Vector 124 s, +3, +6;
Chenghou 154; Blue Ridge 152). It has **no `<LapTimes>`**: `oag_tables::track_stats::parse` refused
the whole file for it, so Pure never had a record target. `TrackStats::lap_times` is now an `Option`
and `pure_medals` carries Pure's figures.

## Measured against the four pairs (fresh profile, Vector, Feisar)

| circuit | target | original | ours |
| --- | --- | --- | --- |
| Vineta K | 130.0 s | `BRONZE TIME 1.53.3` | `BRONZE TIME 1.53.3` |
| Modesto Heights | bronze | `BRONZE TIME 2.00.2` | `BRONZE TIME 2.00.2` |
| Chenghou Project | 160.0 s | `BRONZE TIME 2.33.5` | `BRONZE TIME 2.33.4` |
| Blue Ridge | 158.0 s | `BRONZE TIME 2.31.3` | `BRONZE TIME 2.31.2` |

The label matches on all four, the time exactly on two. The other two are one tenth under the original's
and the cause is the frame, not the law: the countdown is `target - clock`, so the original's countdown
of `2.33.5` needs a clock of at most 6.50 s and Blue Ridge's `2.31.3` one of at most 6.70 s, while our
pose-held frame sits at 6.55 s and 6.75 s (both read `0.06.5` and `0.06.7` on the `CURRENT` clock, which is
all the pair is aligned on; the emulator's captured frame sits a few ticks behind its RAM read). Vineta's
and Modesto's frames fall inside the same tenth for both clocks. Not measurable closer without a tick-exact
frame capture.

Omega/2048/HD: this is Pure's own HUD law, not a 2048/Omega finding; HD counts `RECORD` from its own
`stats.xml` (`docs/ui/hud.md`) and is unchanged (checked, differs).
