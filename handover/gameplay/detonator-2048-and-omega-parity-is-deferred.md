---
status: ready-to-wire
titles: [2048, omega]
updated: 2026-11-26
summary: "Detonator on 2048/Omega: EMP light and kill explosions, Omega end screen, hull scale, captures, a parity sweep of HD-only fixes"
categories: [gameplay, rendering]
---

# Detonator on 2048 and Omega: what the HD drive left for a later drive

The 2026-11-25/26 Detonator drive focused on HD. These are the lanes it drew up for 2048 and Omega and deferred
(maintainer, 2026-11-26). Each was scoped from a merged lane's report; the evidence is on the doc pages named.
The main thread is `detonator-races-its-stages-without-mines-emp-or-auto-speed.md` in this directory.

## Open

- **EMP light barely shows (opus, about half a day, Ghidra plus discs).** Both titles submit the EMP light
  `(0, 5, 10)`, range 50 (2048 `0x811e8650`, Omega `0x0133c150`). On 2048 ours changes 0 pixels; on Omega it
  changes at most 84, against about 31,000 on HD. HD's cyan wash is an HD-only Zone path, and the originals pick up
  to 6 lights per drawable, so theirs is not hull-only. Which GCN/GXP shader adds it is unread
  (`docs/ghidra/functions/ps3-hdfury-eu/detonator-emp.md`).
- **The mine explosions' shards (sonnet, 30 min, with the item below).** HD draws both mine effects with
  their disc sprites since 2026-11-26; 2048 and Omega ship the same emitter trees and the shard atlas
  (`Debris_DetonatorMines.gxt`/`.gnf`), but list no effect sprites
  (`docs/ghidra/functions/ps3-hdfury-eu/detonator-mine.md`, "The shards, against the original").
- **The EMP's mine and bomb kills show no explosion (sonnet, 1-2 h).** The wave clears them on both titles, but
  no explosion effect plays. Map the explosion triggers from each title's own effect table.
- **Omega's Detonator end screen (sonnet, half a day to a day).** Omega runs HD's path (`FUN_012943b0` with the
  same `%s_fury` key, init-0 CRC and five-entry compare; `FUN_0155bc90` is the same filler). But Omega's
  `endrace_entry` is `None`, because through HD's loader `EndRace Menu` draws no option blocks
  (`docs/formats/omega-status.md`); the cause is unread. 2048's four-row `Results` page is not drawn
  (`docs/ui/endrace-2048.md`).
- **Hull scale (sonnet, needs a frame).** 2048's camera constructor writes the same 0.7/0.9 pair statically
  (`docs/ghidra/functions/vita-2048-eu-v104/camera.md`). Wire `Detonator::hull_scale` `Some(0.63)` for 2048 and
  Omega once a Vita3K or shadPS4 frame measures the hull matrix; it is measured, not inferred.
- **Captures against the originals (sonnet capture, Vita3K/shadPS4, about a day).**
  - The cannon bolt follows HD's law on both (static, confidence 85).
  - The wall-hit spark fires on the last-wall death with HD's orientation (confidence 80); its spawn offset is HD's, unmeasured here.
  - The visualiser ring: 2048 draws a faint pale lavender two-band ring, HD a ragged violet band, Omega small saturated violet circles.

  None has an original frame yet (`docs/ghidra/functions/ps3-hdfury-eu/detonator-cannon.md`, `detonator-mine-draw.md`).
- **Parity sweep of HD-only fixes (sonnet, 2-3 h, no emulator).** Check these HD merges against 2048 and Omega,
  wire what applies, and record "ported" or "checked, differs" on each page:
  - the hull charge shells (`Charge1`/`Charge2` alpha, the ready-count law);
  - the Zone glass alpha from the grid's red;
  - the mine's inline tangent;
  - the cannon round riding its own surface normal.
- **Mine draw.** The 2048/Omega mine rim draw is not wired and their mine programs are unread. The music envelopes
  that drive the mines (`--detonator-mine-envelopes`) cannot be read from either title's memory yet, so a 2048 or
  Omega mine pair cannot carry them (see `docs/ghidra/functions/ps3-hdfury-eu/detonator-mine-material.md`).

## Next Steps

1. The EMP light read, opus: it is the most visible gap. A player fires the EMP and sees no light.
2. The kill explosions, sonnet: a short wiring lane once each title's effect table is checked.
3. The parity sweep, sonnet: cheap and catches several small gaps at once.
4. Omega's end screen, then 2048's `Results` page.
5. One Vita3K/shadPS4 capture lane for the bolt, spark, ring and hull scale.
