---
status: measure
titles: [pulse, pure, hd]
updated: 2026-10-26
summary: "Pulse PSP bloom 1.2-1.6x on a racing straight unexplained; matched boost-pad pair; HD strength; Zone grid strip"
categories: [rendering]
---

# Bloom has no setting; Pulse PSP's racing strength is read, and the 1.2-1.6x is not reproduced

2026-10-02, `bloom-setting` lane and the maintainer's ruling the same day. The
maintainer, playing Pulse: bloom "was VERY strong". Ours was 4.4-5.7x the
original over a racing frame, fixed by per-tap byte truncation in `bloom.wgsl`
(`docs/ghidra/functions/psp-pulse-usa/bloom.md`, "Is ours stronger than the
original?").

**Decided 2026-10-02: no bloom setting at all, only the original's.** A level
(`Off/Low/Original/High`) was proposed and declined: the original offers no
bloom option in any title, so the title's own strength is the only faithful
one. `Graphics::bloom` is removed; a settings file that still says
`bloom = false` is ignored on read (serde skips the unknown key) and the key
disappears on the next save.

## 2026-10-08, `bloom-racing`: the 1.2-1.6x is not reproduced

Read off the original's own scratch buffers (`scripts/psp-trace.py
--edram-every`, composite poked off, `scripts/psp-bloom-chain.py`), full account in
`docs/ghidra/functions/psp-pulse-usa/bloom.md`, "Racing strength against the
original's own scratch buffers". **Done**: the chain is exact (the model on the
original's framebuffer lands on its A; ours is the model to 1 % on 13 frames);
the old figure's cause is not identified (its capture was at intensity `1`,
so `--pose-boost 10` is not it; its poke-and-control subtraction carried
`+-0.4` luma on a number near 1). Landed:
the blur tap is `(v * (w + 1)) >> 8` and the composite truncates (both
measured), the ribbon stamps one byte per segment (`trail_stencil`, `114/102`
measured). Talon's Junction ours/original `0.96-1.08` over seven poses, Moa
Therma `1.03-1.37`, the latter following the scene's `rgb * mask` energy.
**No colour grade in Pulse PSP** (the composite is a recorded frame's last
prim). HD: Pulse's finding does not generalise; ours is weaker than the stored
matched captures.

**Open.** (1) The bright pass sampling phase: the model lands within 0.2-5 % of
the original's A and the leftover is at lit edges and the craft. (2) The
original's bright pass reads the HUD's pixels; ours does not. (3) Moa Therma: closed 2026-10-19 (see below), posing plus the shine pass's order;
what remains is the craft flare's shape and a `4-5 %` strip colour excess.
(4) The ribbon shows segment 2 in ours at the chase framing and the original
only segments 0 and 1: the screen length of the ribbon. (5) Whether a racing
frame on a second circuit has any full-screen pass after the composite (one
rest frame was read). (6) The same readout on HD (RPCS3) at a racing intensity.

## 2026-10-11, `pulse-bloom`: not a chain term; Moa's leftover is open

Second boot, three circuits, full numbers in `bloom.md`, "Racing strength,
second boot, and what the Moa Therma excess is". Whole-frame ours/original:
Talon's `1.01` (`0.91-1.06`, racing-speed poses `1.01-1.06`, shadows `original`),
Metropia `1.11` (shadows `original`), Moa Therma `1.18` as posed by default.
Two things move Moa: the track's animated glow textures run on a 60-tick clock a
`--pose-from` frame starts at tick 1 (matching it fixes the strip's mask
placement, mean difference `20 -> 0.3`; the colour under the mask was not
measured, and the matched renders drift the craft), and the original draws the
craft's shadow wedge while the default `Shadows::Off` does not (`1.13` alone;
craft-box `1.13 1.27 0.98 1.12 1.21`). **The shadow part is real in play**: the
default's doc comment says it holds "until `original` exists", and it exists on
Pulse now - the maintainer's call. The scenery outside the craft still reads
`1.12-1.17` at three of five matched poses. The chain, the tap, the composite and
the mask are unchanged and nothing in `oag-render` or `oag-post` moved. The
2026-10-08 table was already in the post-2026-10-04 composite order.

**Open.** Moa's leftover (scenery `1.12-1.17` with the clock matched; the colour
under the mask at matched phase, one more capture). The ribbon's screen extent
grows with speed in ours more than in the original (first segment `1,463` pixels
to row `265` against `1,051` to `258` at tick 140, equal at tick 100; a real run
of ours gives `1,425` at the same speed). The standing `+1.4-2.9 %` scene luma.
The HUD's read by the original's bright pass is still not isolated. The Zone grid
strip was not reached (see `bloom.md`).

## 2026-10-19, `pulse-bloom-moa`: Moa mostly posing, a strip remainder is real

Numbers in `bloom.md`, "Moa Therma's excess at a matched glow clock". `--anim-seconds`
sets the glow clock render-only; the phase is `(tick + 4 +- 1) mod 60` ticks. Whole-frame
ratio with default shadows (already `original` on Pulse): `1.103` at the default clock,
`1.045` matched; no code changed. **Closed 2026-10-19 (`moa-strip-band`):** the pale band over the trees was the track
shine pass drawn after the blended list; it is now drawn before it, and list-B meshes
go far to near by view depth (`mesh-draw.md`). Ticks 120-160 whole frame `1.09-1.14`
to `1.06-1.11`, the strip's colour under the mask `1.08-1.11` to `1.03-1.05`. Left:
the craft flare's shape (about `0.1` luma at tick 160) and a `4-5 %` colour excess on
the strip. PS2 is untouched (no shine pass, VIF batches).

## Measured first

Ours was 4.4-5.7 times the original over a matched racing frame and 1.4 times
at rest, and that is fixed on this branch (per-tap byte truncation in
`bloom.wgsl`; `docs/ghidra/functions/psp-pulse-usa/bloom.md`, "Is ours stronger
than the original?"). Whole-frame mean luma the bloom adds, racing straight:
original 1.0-1.3, ours before 5.6-5.7, ours after 1.55-1.63; at rest on the grid
original 8.35, ours before 11.8, ours after 8.3. It was a wash over the whole
picture, not the exhaust, and it is resolution independent already. Ours is
still about 1.2-1.6x the original on a racing straight (1.3x around the craft),
and that residual is a fidelity gap, not a setting. The boost pad's glow is the
original's own look (same size in both). The glow-mask value of the pad is a texture byte, not the stamp.

## What runs today, per title

| Title | Bloom path | Gated by the removed `graphics.bloom` | Original reference |
| --- | --- | --- | --- |
| Pulse PSP | `post::bloom`, four passes, constants from `BOOT.BIN` | yes | always runs (121 of 121 frames) |
| Pulse PS2 | `post::ps2_bloom`, its own three passes (7 integer taps, 320x224, `127/128 * 64/128` add) over its own mask (`GlowMask::StampedByTexel`), both read off replayed GS dumps, `docs/rendering/ps2-bloom.md` | n/a | always runs (every race field) |
| Pure PSP | `post::bloom` over the same stencil stamps (`Looks::psp_glow_mask`), composited before the HUD (`Looks::bloom_under_hud`), 2026-10-26 | no setting | read: Modesto GE list, [`psp-pure-usa/bloom.md`](../../docs/ghidra/functions/psp-pure-usa/bloom.md) |
| HD / Fury | `post::hd_bloom`, five stages, per-circuit `HDR and Bloom` values | yes, as `Glow::Drawn` or `Suppressed` (the chain still runs; only the glow term goes) | read, not measured as a player option |
| Omega | none: the patch carries no `HDR and Bloom` block | n/a | n/a |
| 2048 | none | n/a | unread |

The original exposes no bloom option in any title. A level is therefore ours by
definition, and the only faithful value is the title's own.

## Zone (2026-10-02, `zone-bloom` lane)

The maintainer found the bloom "quite obvious" in Zone. Measured on a native
Zone race (`bloom.md`, "Is ours stronger than the original in Zone?"): **ours
is 0.88-0.90x the original racing** (zones 2, 3 and 5, two boots), 1.2x on the start grid
(boot 1 and 2) from a bottom-of-frame strip that ours stamps `214` and the original leaves at
`4` (cause not identified). Zone's bloom is the original's own look: the same
pose adds 1.5 luma on the ordinary circuit and 13 on Zone's, and the original's
mask is 13-18 % at >= 200. No code change.

## The bloom draws over the HUD (2026-10-04, `pulse-bloom-roll`)

Closed. The original's queue puts `Bloom_Draw` (`0x70`) after every HUD widget
(`0x52`..`0x6d`). With the composite poked off, the HUD's glyphs over a glowing
panel lose about 50 luma per channel. Ours now prepares the bloom after the
scene and composites it after the HUD (`Scene::composite_bloom`). Evidence:
`docs/ghidra/functions/psp-pulse-usa/bloom.md`, "The bloom draws over the HUD".
Not reproduced: over a glow surface, the original blooms the HUD widget's own
colour (its bright pass reads the HUD). **Pulse PS2 inherits the order
(2026-10-04, maintainer's call, under the rule that a source with no measured
rule takes Pulse PSP's)**: `Ps2Bloom` splits into prepare and composite the same
way, unmeasured on the PS2 disc (`ps2-bloom.md`). **HD keeps its old order,
pending the maintainer**: its read resolve (`downsamplescaleaddfeedback_fp`)
adds the bloom to the exposure-scaled scene *before* the encode, so a HUD under
the bloom is not a reorder but a choice - the HUD exposure-scaled in the linear
target, or the bloom added after a nonlinear encode - and neither is the
executable's.

## Open

- Why ours is still 1.2-1.6x on a racing straight: partly answered 2026-10-11 (animation
  clock, shadow tier; see the section above).
- A matched boost-pad on/off pair in the original (1 of 4 placed approaches
  triggered the pad in this lane).
- HD's own strength at matched poses: this lane measured Pulse PSP only.

- ~~Does Pulse PS2's original bloom at all?~~ **Yes**, and **ported** (2026-10-02):
  the mask (a glow batch's own alpha, zero elsewhere) was read back off replayed GS
  dumps and the chain (320x224, 7 integer taps, half-strength add) is
  `post::ps2_bloom`. Matched on Moa Therma's grid at slot 8, ours adds 1.06x the
  original's mean luma (1.62 against 1.53). Racing craft box (pose not matched): ours 0.84 against 0.40 luma, 2.1x, mask matching. Open: **one circuit and a grid frame**
  - Talon's Junction was not reached and no racing frame has a recoverable pose; the
  ghost craft stamps nothing on PS2; `ATST NOTEQUAL 6` and whether the HUD groups
  before the downsample take the composite are read from registers only. See
  `docs/rendering/ps2-bloom.md`.

## Next Steps

0. Zone grid strip: still open. Ours reaches a Zone start and stamps `214` on
   the bottom rows (`bloom.md` has the numbers); it needs the original's Zone
   grid at the same pose, so first an unlocked Zone on a PPSSPP profile.

1. Partly explained 2026-10-11 (the section above): not a chain term. Left: Moa's
   scenery `1.12-1.17`, the colour under the mask at matched phase, the
   ribbon's extent at speed (a real run's ribbon against the original's at
   several speeds, `exhaust.md`), the `+1.4-2.9 %` luma.
2. PS2 bloom: a second circuit (Talon's Junction) and a racing frame at a pose read out of the emulator, for the racing-straight strength the PSP chain still carries 1.2-1.6x on.
3. Measure HD's bloom strength at matched poses on RPCS3.

## Pulse PSP boosting plume at speed, `pulse-boost-plume` (2026-10-21)

On `talons-alongside` the original is 0.50 s into a pad boost (`boost_timer` 0.299, `plume_timer` 0.484, `half_size` 4.4-5.1) and ours renders with `boost_timer` 0 because `Race::warm_up_field` never calls `Exhaust::boost`. The "original is bigger" claim is false: ours is already brighter than the original at the nozzle (mean grey 219.6 against 193.0) and in the ribbon tail (69.1 against 37.1), and `--pose-boost 0.501` makes both worse (246.6 and 115.2; whole frame 54.05 to 61.49 against the original's 46.68). Evidence and the table: `docs/ghidra/functions/psp-pulse-usa/exhaust.md`, "A boosting plume at speed".

## Next Steps (boost plume)

`pulse-plume-ribbon` (2026-10-21) corrected this thread: the race pair is not boosting (flare `boost_timer` 0 at the pair's
own clock; the 0.2991 came from a later freeze), and the original's ribbon fragment colour is about a twentieth of what
ours draws while its bloom stamp matches (details and table in `docs/ghidra/functions/psp-pulse-usa/exhaust.md`, "The
race pair is not boosting"). So:

1. **Done 2026-10-21 (`pulse-ribbon-colour`): the colour is not low, it is overpainted.** The original queues every
   `Trail` at `0x4d000000`, ahead of every list-B circuit mesh, and a translucent floor patch drawn after it paints over the
   ribbon (the ribbon writes no depth); ours now draws the ribbons after the hulls and before the circuit's blended list
   (tail box 69.1 to 41.6 against the original's 37.1; `exhaust.md`, "The ribbon is queued first"). Open from it:
   (a) the flare is keyed by its own depth (`0x4d | depth`) and ours still draws it after the whole blended list, which is
   right for the player's (112th of 113) and not for an opponent's, so interleave the flares by depth with the blended
   draws (needs per-craft ranges in the exhaust upload); (b) HD's tube, Omega's and 2048's keep the old order, their queue
   keys unread (RPCS3: read `display`'s queue the way `scripts/psp-queue-read.py` does); (c) the rest of the tail-box gap
   (41.6 against 37.1) and the nozzle disc (210.4 against 193.0) is the flare quad, the hull and the bloom of those.
2. **Done 2026-10-21 (`pose-lean-boost`):** `psp-pulse-pair.py` reads the flare at its own stop and passes `--pose-boost
   (0.8 - boost_timer)` with the warmed trail kept (`Race::warm_up_field` arms the boost), the load stops at the state's own
   first frame and `--target-clock` lands re-makes on one instant (docs/tools/compare-page.md, "A deterministic stop").
   `talons-boost` whole-frame RMSE 0.140 to 0.113, craft crop 0.292 to 0.179 against the same `theirs.png`.
3. **Done:** the `--trace-out` row of a `--field-warmup` run is written with the warm-up now, so it reads ours' exhaust at
   the rendered tick.

## 2026-10-26, `talons-alongside`: "ours is too bright" on the race shot was the craft's light scale, and the geometry was the visible set

Not a bloom or plume term. On `pulse-psp-race/talons-alongside` a posed capture ran no tick, so `Race::ship_hull_scale` fell to the
default `255` (the original's hull lights at about `123/255` inside the tunnel) and `visibility_sections` read `UNPLACED` (every
section drawn, terrain over the ceiling). Both fixed render-side (`Course::locate`, `Spline::nearest`, read only); the neon strips were
the pair script's world clock. Whole frame 14.14 to 3.29 mean grey; hull-only boxes 1.4-1.95 to 1.03-1.13; nozzle 1.20 to 1.05;
scenery boxes 0.93-1.01 to 1.00-1.04. Full table and tests: `docs/tools/compare-page.md`, "A posed race shot was lit, culled and
animated as if no tick had run", and `docs/ghidra/functions/psp-pulse-usa/scene-light.md`, "A craft no tick has located".
`3ff1d331f` (`pulse-vex-normals`) did not move this pair (195 pixels, 0.018 grey). **Open (small):** the nozzle box is still `1.05`
and the hull's left fin `1.13` (the fin catches the strip light's point-light slot, not read per frame), the plume interior is the
original's `rand` size/alpha phase (above).
