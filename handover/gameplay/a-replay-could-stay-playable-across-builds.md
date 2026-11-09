---
status: parked
titles: [pulse, pure, hd, 2048, omega]
updated: 2026-11-07
summary: "Low priority: a replay breaks once the simulation changes; a full state stream could keep it watchable and open to a take-over"
---

# A replay could stay playable across builds

Raised by the maintainer on 2026-11-07, comparing Dota 2, whose replays stay watchable across
patches and can be taken over. Not a priority. Nothing in ADR-0087's stages depends on it.

## Open

- **ADR-0087 today.** Inputs and hashes are the truth. Past the last agreeing hash a replay falls
  back to POSE VIEW, hulls only, and a take-over is refused (decisions 4 and 9).
- **What this would add.** A state stream in a new `.oagr` chunk, wide enough to draw the HUD,
  weapons and effects, or wider still, the whole mutable race, to start a take-over. An older
  reader skips an unknown chunk, so the container does not block it.
- **Cost, estimated from ADR-0087's tables, not measured.** A keyframe is about 28 KiB, so one a
  second is 8 to 9 MiB for five minutes before delta coding, against about 0.5 MB for a replay.
- **Layout.** `World` changes with commits, which is why keyframes in the file were rejected
  (ADR-0087, "Alternatives considered"). This needs a self-describing encoding and a migration for
  each simulation field that changes.
- **Meaning.** Old state in a newer simulation runs under different laws: not the same race. That
  relaxes "only from a verified state" (decision 9), so it needs a new ADR.
- **Waiting costs little.** A file with no such chunk stays hull-only past its last agreeing hash.
  A file that still verifies could be re-simulated to gain the chunk; a broken one cannot.

## Next Steps

1. ~~Decide at the `pose-track` briefing whether this stays parked.~~ It stays parked: the `POSE` chunk is
   hull-only as ADR-0087 specifies, with no field reserved; an unknown chunk is skipped (proved), so a state
   stream can still be added as a new chunk.
