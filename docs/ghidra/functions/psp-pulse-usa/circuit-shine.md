# Pulse PSP's circuit chrome-map pass: the ordinary pass is weighted by the batch texture's alpha

**Binary:** `psp-pulse-usa` `BOOT.BIN`. Measured 2026-10-26 (lane `pulse-circuit-weighting`): two recorded GE lists of
the original (PPSSPP v1.20.4, software renderer, `scripts/psp-ge-dump.py dump`), each taken on a Pulse compare pair's own
save state (`time-trial-talons-venom-grid`, `time-trial-metropia-black-venom-grid`), a replay of an edited copy of the
first, and ours rendered on five stored pairs before and after. Companion to Pure's
[hull-shine.md](../psp-pure-usa/hull-shine.md) ("The circuits") and to [mesh-draw.md](mesh-draw.md).

## Answer

**Weighted, as Pure's are. Confidence 85.** A circuit batch that carries the weight bit (`header_flags & 0x02`) draws its
ordinary pass with `BLENDMODE 0x2a` (`FIX` source, `SRC_ALPHA` destination, `FIXA` white), after an environment pass
(`TEXMAPMODE` 2) drawn as a replace (`0xaa`, `FIXB` black). The batches without the bit draw their ordinary pass `0xaa`
with `FIXB` white (`src + dst`), which is the unweighted form Pulse's decompile reads.

The 85 rests on the recorded state of two lists on two circuits and one replay; the link from "this PRIM's twin has the
bit" to `0x2a` is read off the pairing of PRIMs, not off the `.vex` batch flags PRIM by PRIM (the decompile at
`0x0891f890` reads the same branch, 85 in [hull-shine.md](../psp-pure-usa/hull-shine.md)).

## Measured: what the GE is given

PRIMs per list, by register state at the PRIM (`TEXMAPMODE` is `0xc0`, `BLENDMODE` `0xdf`, `FIXA`/`FIXB` `0xe0`/`0xe1`):

| List | `TEXMAPMODE` 2 PRIMs | ... whose ordinary twin is `0x2a` | ... twin `0xaa`, `FIXB` white | other twins | all PRIMs with `0x2a`, `TEXMAPMODE` 0 |
| --- | ---: | ---: | ---: | ---: | ---: |
| Talons (white, venom grid) | 66 | 51 | 9 | 6 (`0x32`, `0xa2`) | 224 of 541 |
| Metropia (black, venom grid) | 48 | 34 | 9 | 5 (`0x32`, `0xa2`) | 387 of 736 |

A twin is the PRIM with the same vertex count and world-space box. The environment PRIMs read `0xaa` with `FIXB` black
(Talons 60 of 66 sit in the first block of the frame; Metropia 43 of 48 are `FIXB` black), i.e. replace, as on Pure. The
weighted ordinary PRIMs run lighting off, depth function 2 (equal) with writes off when the PRIM also alpha-blends
(depth function 6 otherwise), as Pure's.

## Measured: what weighting changes

Replay of the Talons list with every `TEXMAPMODE` 0 `0x2a` rewritten to `0xaa` plus `FIXB` white (51 PRIMs reached by the
edit script; `ge_edit` style, in-place word rewrite): the frame lights up the walls and floor with a broad pale wash
and brightens the right-hand panel, against the unedited replay (whole-frame grey MAE between the two replays 11.5, of
which the HUD clock and the on-screen notice are part). Read at player size.

Ours, `oag-game` dev build, five stored pairs re-rendered from their own `pair.json` flags, `track_shine_weighted` unread
(before) against on (after). Whole frame and the changed pixels only (pixels where the two renders differ by more than 3):

| Pair | frame MAE before -> after | changed px | MAE on changed px before -> after |
| --- | ---: | ---: | ---: |
| Talons white | 4.12 -> 4.12 | 304 | 10.12 -> 6.76 |
| Metropia black | 4.26 -> 4.24 | 474 | 6.67 -> 4.13 |
| Moa Therma white | 4.26 -> 4.11 | 1,472 | 18.85 -> 6.52 |
| Basilico black | 5.30 -> 5.31 | 305 | 16.00 -> 16.90 |
| Outpost 7 white | 4.50 -> 4.09 | 2,538 | 23.59 -> 4.14 |

Four of five move toward the original; Basilico black moves 305 pixels by 0.9 grey away from it, not understood (not
chased: the edit is a few hundred pixels of a 130,000 pixel frame). The effect is local (the chrome strips on the track and
walls), which is why the whole-frame number barely moves.

## Wired

`oag_pulse::effects::LOOKS.track_shine_weighted` is `Measured` on the PSP (it was unread). The PS2 pressing: **not
checked**. `track_shine` itself is PSP-scoped (the GE register state above is a PSP readout; the PS2's GS dumps were not
taken for the circuit's env pass), so the rule stays PSP-only and the PS2 draws no chrome-map pass either way.

**Checked against Omega/2048:** not applicable, a PSP `.vex` rule; HD/2048/Omega circuits author no `TEXMAPMODE` pass.
