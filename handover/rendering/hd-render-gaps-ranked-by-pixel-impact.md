---
status: measure
titles: [hd]
updated: 2026-10-27
summary: "HD gaps by tile: gantry card panels read R+11 to +6 (card is 5-6-5 linear, wired); booth glass wired 2026-10-27, glass_reflect next"
---

# HD/Fury render gaps ranked by pixel impact

2026-10-26, lane `hd-impact-scout`. Question: where does HD differ most from the
original by pixels a player sees, so the next HD lane aims at the biggest gap.

## Open

**Set.** 13 HD pairs under `compare-page/hd/` (Harimau classic, Time Trial start,
1920x1080, chase cam). `talons-pack` (`hd-race/`) is excluded: its frame is a
different moment with an imposed HUD and a warm-up exhaust path, so a per-pixel
diff ranks nothing there.

**Method (reproducible).** For each pair, per pixel: mean abs RGB difference
(0-255) between `theirs.png` and `ours.png`; signed luma `ours - theirs`. Tile
the frame 120 px (16x9 grid). Impact = sum of the per-pixel error in a tile.
Low frequency = mean abs RGB difference after a Gaussian blur of sigma 7 px on
both frames (what survives is colour, brightness or missing content). High
frequency = raw minus low (AA, texture detail, noise). Sharpness = mean
absolute gradient of ours over theirs, per block, on luma. Blocks below are
hand-drawn tile groups; the tile list is in the table's column "tiles".

**Global tone is not the story.** A per-channel linear fit `ours ~ a*theirs + b`
over each whole frame removes only 5% of raw error (fit/raw 0.95). There is no
frame-wide exposure lane. The luma error is spatial: the top rows run ours
brighter (+3.7 to +6.3 luma), the lower left runs darker. The vertical split is
already recorded in [hds-frame-was-too-bright-and-too-bloomy.md](hds-frame-was-too-bright-and-too-bloomy.md)
(2026-09-17, one circuit). The 13-pair mean shows the same split; it does not
hold on every pair (anulpha-pass: whole-frame bias -2.2, row-5 bias +0.1).

**Totals.** Raw error per pair: mean 6.2 to 11.0 per pixel (13-pair mean 8.8).
65% of the raw error survives the sigma-7 blur, so most of it is low frequency.
Sharpness is not the gap: ours-over-theirs gradient is 0.98 to 1.06 per block
(6 pairs: the-amphiseum, vineta-k, sol-2, talons, metropia, modesto-heights).
Hue is not measured; only luma is in this table.

**Ranked table.** Ranked by low-frequency share, the part a lane can fix.
Raw share of all 13-pair error in brackets. Low impact is low share times the
total low error (about 153 M, the sum of per-pixel error over 13 pairs at
0-255). "Low err/px" is the mean per-pixel low-frequency error in its block.
"Top 3 pairs" are the three with the most raw error in the block.

| # | Block (120 px tiles) | Low share (impact) | Raw share | Low err/px | Luma bias | Top 3 pairs | Covered by |
|---|---|---|---|---|---|---|---|
| 1 | Centre scenery and gate (tx 6-12, ty 1-4) | 21.8% (33 M) | [21.6%] | 7.5 | +2.7 | amphiseum, metropia, ubermall | position bucket: content changes per circuit; partly [hd-glow-mask.md](../../docs/rendering/hd-glow-mask.md); not a lane yet |
| 2 | Tiles containing HUD (LAP, shield, RECORD, bottom gauges) | 18.8% (29 M) | [16.7%] | 5.5 | +2.4 | amphiseum, vineta-k, anulpha | tiles include track and wall behind the HUD, so the HUD-only share is unmeasured; [hd-hud.md](../../docs/formats/hd-hud.md) |
| 3 | Left wall and stands (tx 0-5, ty 1-4) | 17.1% (26 M) | [17.5%] | 5.9 | +0.6 | sol-2, modesto-heights, sebenco-climb | position bucket; the FX-350 advert is a timed board, see [hd-glow-mask.md](../../docs/rendering/hd-glow-mask.md) ("Still open on the board") |
| 4 | Track left and right (tx 0-5 or 10-15, ty 5-6) | 11.2% (17 M) | [11.7%] | 3.8 | +2.3 | moa-therma, anulpha, sol-2 | [hds-frame-was-too-bright-and-too-bloomy.md](hds-frame-was-too-bright-and-too-bloomy.md) (blown lightmap); crops match closely |
| 5 | **Top-right band (tx 9-12, ty 0-1)** | 9.6% (15 M) | [8.3%] | **9.9** | **+6.3** | vineta-k, amphiseum, modesto-heights | **new lane candidate, cause split**: see Next Steps 1 |
| 6 | Craft hull and plume (tx 6-9, ty 5-8) | 7.8% (12 M) | [10.3%] | 4.0 | +0.3 | anulpha, ubermall, sol-2 | [hds-engine-light-washes-the-hull-on-boost.md](hds-engine-light-washes-the-hull-on-boost.md); half its error is high frequency (low/raw 0.50) |
| 7 | Right wall and sky (tx 13-15, ty 2-4) | 7.3% (11 M) | [7.3%] | 6.7 | +2.5 | vineta-k, modesto-heights, ubermall | position bucket; no crop of its own |
| 8 | Other sky and track (rest of rows 0-1 and 5-8) | 6.5% (10 M) | [6.5%] | 3.9-6.0 | +1.0 | position bucket | position bucket; no crop of its own |

Row 5 has the highest error per pixel of any block and the largest luma bias.
It is the first lane candidate, not the largest block. Rows 1 to 3 are large
because they cover the most tiles, and their content differs on each circuit.
Row 5 is named for its place: on Vineta K and Talon's it is open sky.

**Luma bias by tile.** Top row, columns 6-15: ours brighter by +6 to +12. Row 1,
columns 6-12: +3 to +13. Rows 2-4: +2 to +5. Left columns of rows 0 to 3 and
the lower rows: near zero, some darker. Per pair, row 5's bias runs from +16.1
(the-amphiseum) and +14.9 (vineta-k) down to +0.1 (anulpha-pass). The top-right
bias is strongest on the Amphiseum and Vineta K pairs.

**Row 5 cause is split (checked on crops, 2026-10-26).** Not one rim-highlight
cause. What the crops show:

- `the-amphiseum`: ours draws brighter additive light beams from the ceiling
  lights (diagonal white shafts through the trusses); the original's beams are
  dimmer. The light-cone material was fixed on this circuit earlier
  ([hd-accuracy-pass-2026-10-05.md](hd-accuracy-pass-2026-10-05.md), `dc_lightcone`);
  what remains is unmeasured.
- `vineta-k`: white highlight lines on the ramp edges against the ceiling and
  building ledges, present in ours and not in the original. This is the one
  crop that fits the stand-in specular in
  [hd-lit-billboard-specular-and-gantry-leftovers.md](hd-lit-billboard-specular-and-gantry-leftovers.md),
  and it is not yet checked.
- `talons`: scattered small particle sprites in the sky, in different places
  in ours. Position, not brightness: a particle timing or capture moment
  difference, not a material.
- `anulpha-pass`: a bright glow column on the right pillar in ours, dim in the
  original. Unexplained.

**Capture mismatch, do not lane these.**

- `talons` (heading drift 1.72 deg, camera 0.29 units, race clock -7.2 s): kept
  in the totals, but dropping it moves every block share by under one point.
  Its scenery error is partly pose, and its sky particles are not in the same
  places.
- `talons-assegai-c1` (0.54 deg, clock -2.3 s): same; marginal.
- `talons-pack` (`hd-race/`): excluded, see Set.
- Race clock drift on the 11 other pairs runs from +0.13 to -1.42 s (three
  positive). The largest offsets, `moa-therma` and `modesto-heights` at about
  -1.4 s, are also the pairs most likely to catch a timed board in a different
  state. That is why the advert on the left wall (row 3) is not read as a
  missing decal: on `vineta-k` the FX-350 text is absent in ours.
- HUD digits: on `vineta-k` the lap clock reads 0.13.7 in both frames; on
  `the-amphiseum` it reads 0.13.6 in both. The HUD row still needs a HUD-only
  mask before it is a lane.
- Ship and camera heading are within 0.2 deg on the 11 other pairs; pose is
  close enough for a per-block read, not for per-object claims.
- No other craft appear in these Time Trial pairs. The plume cannot be split
  from the hull at 120 px, so it sits inside row 6.

## Next Steps

1. ~~**Measure first, then lane: ceiling light beams and glow (row 5).**~~ Measured and wired 2026-10-26, see
   "Top-right band, measured and wired" below. The beams are `dc_lightcone` read raw (its taps are gamma-7 units);
   Vineta K's rim lines were a stand-in specular on a program with no half-vector power. Rows left: the open list
   below.
2. **Measure, not a lane: HUD-only share.** Claim: the HUD's own pixels carry
   under 5% of the 13-pair error, so the HUD row is an estimate artifact.
   Method: mask the HUD layout rects from
   [hd-hud.md](../../docs/formats/hd-hud.md) and take the low-frequency error
   inside the mask over the 13 pairs. Acceptance: one number in this thread.
   If it is above 10%, open a HUD text-alpha lane.
3. ~~**Measure, not a lane: left wall and centre gate.**~~ Measured 2026-10-26, see
   "Blocks 1 and 3 split by cause" below. The falsifier was met for the gantry boards (block 1) and the
   `ds_booth_glass` / `glass_reflect` families (block 3); the lit opaque remainder is many draws and stays
   unlaned.

## Blocks 1 and 3 split by cause (2026-10-26, lane `hd-diagnose-blocks`)

**Falsifier, written before the sweep.** One or two named toggles must explain at least half of a block's
low-frequency error on two circuits; if none does, the honest result is "many draws, no lane" and no lane is
proposed for that block. Result: **met for block 1** (the gantry cards, at least half of the block on
Anulpha Pass 69%, Chenghou 54% and Ubermall 52%; Talon's Assegai 54% is a capture mismatch) and **met for
block 3 only on two circuits** (glass, 64% on Sol 2 and 59% on Modesto Heights; 30% over all 13). Opaque
scenery fails it in both blocks.

**Method (reproducible, no code changed).** Ours re-rendered per pair from its own recorded flags with the
pair's own `ours-profile` `settings.toml` and the `oag-game` build that made `ours.png`
(`oag-game <hd image> <pair.json ours.flags> --screenshot`; the baseline re-render differs from `ours.png` by
0.000 on all 13 pairs). Nine ablations per pair, each a whole-frame render:
`OAG_OPAQUE_ONLY=1` (every blended chunk removed), `OAG_SKIP_MATERIAL` with `321go`, `billboards`, `glow`,
`light`, `sky,cloud`, `glass`, `palm` (a needle matches the material path or either texture path), and
`OAG_ALBEDO_ONLY=1` (light rig off). A pixel belongs to a toggle when `|blur7(base) - blur7(toggle)|`
(mean over RGB) exceeds 3 of 255; the pixel goes to the specific toggle with the largest delta (ties go to
the earlier of `321go`, `billboards`, `glow`, `light`, `glass`, `sky`, `palm`), then to "other blended"
(opaque-only delta), then to "opaque scenery, light-sensitive" (albedo-only delta above 12), else "rest".
Masks are exclusive, so shares sum to 100%. The error summed in a mask is the same low-frequency map the
table above uses (PIL Gaussian blur 7, mean abs RGB), so the denominators match: block 1 is 33.6 M (21.8%)
and block 3 26.3 M (17.1%) of 154.2 M, as before; dropping both `talons` pairs moves no cause by more than
3 points; the threshold at 2 and 5 moves every share by at most 3 points. A mask is where ours draws a
material, **not** where the original differs from it: a skipped material shows what lies behind, so no
"error removed by the toggle" number is used as a share. What each needle matches (from the draw dumps):
`321go` is the textures under `data/billboards/hd_adverts/321go/` (`321_go_64` the GO glyph, `fx350_nomip`
the side cards' advert, `honeyblue_noalpha`); `glow` matches **any material or texture path containing
"glow"**, which on Metropia and Sebenco removes the whole card (the cards' own material) but on Tech de Ra
removes the `banner_*_glow` emissive banners and elsewhere trackside and landscape glow strips;
`cf_vex_billboard` as a needle matches only Metropia and Vineta K and is not the card's name on the others.
`billboards` removes real draws (203k pixels on Vineta K), but every advert except the 321go cards totals
only 0.2 M in block 1.

| Block 1 cause (33.6 M) | Impact | Share of block | Share of frame | Bias (ours - theirs, luma) | Pairs it leads on | Covered by |
|---|---:|---:|---:|---:|---|---|
| **Gantry cards** (`321go` mask 10.2 M plus the `glow`-material pixels within 40 px of it) | 14.0 M | **41.7%** | 9.1% | +1.7, sign varies by pair | anulpha-pass, chenghou, ubermall (52 to 69% each) | [hd-glow-mask.md](../../docs/rendering/hd-glow-mask.md) ("The GO board", "Still open on the board"); [start-gantry.md](../../docs/rendering/start-gantry.md) |
| Opaque scenery, light-sensitive (albedo-only differs by more than 12; **not a lighting attribution**, see below) | 9.4 M | 28.0% | 6.1% | +2.4 | the-amphiseum (61% of its block), vineta-k, sebenco-climb | partly [hd-lit-billboard-specular-and-gantry-leftovers.md](hd-lit-billboard-specular-and-gantry-leftovers.md); else none |
| `glow`-material surfaces away from the cards (emissive banners, glow strips, halo spill) | 3.5 M | 10.4% | 2.3% | +34 on Tech de Ra (**+1.1 since `hd-tech-de-ra`**), +10 on Metropia and Sebenco | tech-de-ra, metropia, sebenco-climb | [hd-glow-mask.md](../../docs/rendering/hd-glow-mask.md) (the glow mask); Tech de Ra's is `nr_twinblend`, wired: rcsmaterial.md, "`nr_twinblend`: two pictures, a scrolling wipe and bars" |
| Glass | 2.0 M | 5.9% | 1.3% | -0.6 | tech-de-ra, sebenco-climb, metropia | [hd-fidelity-lanes-queued-after-the-cold-originals.md](hd-fidelity-lanes-queued-after-the-cold-originals.md) |
| Other blended (Talon's pose, particles) | 2.0 M | 5.9% | 1.3% | +7.1 | talons, talons-assegai-c1 (capture mismatch) | none |
| Rest (opaque, not light-sensitive) | 1.7 M | 5.1% | 1.1% | +0.6 | modesto-heights | none |
| Ceiling light cones (`light`) | 1.6 M | 4.6% | 1.0% | +6.1 | the-amphiseum, vineta-k, ubermall | the top-right band lane (`hd-top-right-band`, same cones) |
| Sky, non-gantry adverts, palms | 0.5 M | 1.5% | 0.3% | mixed | none | none |

(The cards and `glow` lines split the 17.5 M the `321go` and `glow` masks cover inside block 1's box, x 720-1560,
y 120-600 without the top-right band, by distance to the `321go` mask; the other lines are the exclusive
masks of the first pass, whose `321go` plus `glow` was 16.4 M, so the column sums to 103%.)

| Block 3 cause (26.3 M) | Impact | Share of block | Share of frame | Bias | Pairs it leads on | Covered by |
|---|---:|---:|---:|---:|---|---|
| Opaque scenery, light-sensitive | 9.4 M | 35.9% | 6.1% | +0.9 | chenghou-project, sebenco-climb, anulpha-pass (spread, 0.3 to 1.2 M per pair) | none |
| **Glass** (the left arch's panes) | 8.1 M | **30.7%** | 5.2% | -2.1, per circuit -26 to +42 | sol-2 64%, modesto-heights 59%, sebenco-climb 46% | [hd-fidelity-lanes-queued-after-the-cold-originals.md](hd-fidelity-lanes-queued-after-the-cold-originals.md) ("The other families that declare the sky map"); [hd-track-tone.md](../../docs/rendering/hd-track-tone.md) |
| `321go` and `glow` surfaces (the left FX-350 side card, which runs on its own advert timeline, and `glow`-material surfaces) | 4.3 M | 16.2% | 2.8% | +1.1 to +3.4 | vineta-k, ubermall 40% | [hd-glow-mask.md](../../docs/rendering/hd-glow-mask.md) ("Still open on the board") |
| Other blended | 1.8 M | 7.0% | 1.2% | +1.0 | moa-therma | none |
| Rest | 1.8 M | 6.7% | 1.1% | -1.2 | talons | none |
| Cones, sky, palms, other adverts | 0.9 M | 3.5% | 0.6% | mixed | none | none |

**Gantry cards, split further.** Using the `321go` mask only (the cards, 10.2 M in block 1; 11.4 M inside
the box):

- **Phase is about 11% of the board.** Ours re-rendered at ten `--gantry-seconds` values over one 0.6 s pulse
  (period confirmed: 3.90, 4.50 and 5.10 are identical) against each pair's `theirs.png`, in the union box
  above. Nine pairs carry a pin: on six (Vineta K, Chenghou, Amphiseum, Sol 2, Moa, Tech de Ra) the pinned
  phase is within 0.1 M of the best; Metropia and Ubermall are 0.31 M and 0.25 M off; **Anulpha Pass is 1.13 M
  off** (2.81 M against 1.68 M at its best phase; the glyph reads 164 against the original's 229). Summed
  over the 11 pairs swept: pinned 16.4 M, best phase 14.5 M. **Modesto Heights and Sebenco Climb carry no
  `--gantry-seconds` at all** (the RAM search found no windowed node), so their board runs on the race clock;
  both land on a bright phase, which is luck, not a match.
- **Glyph and halo (theirs or ours luma above 150): 3.4 M.** Ours brighter on Metropia (+26) and Ubermall
  (+27), dimmer on Anulpha (-54, the phase) and Talon's Assegai (-33, pose).
- **Panel and side cards (not bright): 8.0 M, the one consistent signal.** On 9 of 13 pairs ours reads
  **R +6 to +17** (Chenghou +10, Metropia +16, Moa +7, Modesto +6, Sebenco +17, Sol 2 +8, Talon's +14,
  Amphiseum +11, Vineta K +9; mean about +11) against G -0 to +10 and B -1 to +5 inside the cards. On Anulpha, Ubermall,
  Tech de Ra and Talon's Assegai it is R +0 to +3 and G, B -1 to -4. **It is not the frame's tone:** the same
  box's opaque scenery outside every mask reads R -1 to +4 on 11 pairs (Amphiseum and Tech de Ra, +6 to +8 on
  all channels, are the exceptions), so the cards carry about +6 to +9 R over the scenery. The cause is
  **unread**. The documented leftover that ours adds the second texture to the albedo where
  `cf_vex_billboard`'s program multiplies (`hd-glow-mask.md`) is a candidate, but the same page says the GO
  card "reads right through the halo" with the current path, which cuts against it, and there is no switch
  to test it.
- **The original moves too.** Inside the `321go` mask the original's two captures per pair
  (`capture/00.png`, `01.png`, 0.1 to 1.4 s apart) differ by 9.6 M, 94% of ours-against-theirs there. That
  is the pulse and the capture offset, **not** an irreducible floor (the pinned phase is right on most pairs);
  it only says a board comparison needs both frames at one pinned phase. Elsewhere the same capture-to-capture
  difference is 10 to 18% of ours-against-theirs (opaque scenery 13%, glow 14%, glass 5 to 10%).

**Glass in block 3: two unwired families, not one pane.** Skipping by material name inside the block 3 box
(x 0-720, y 120-600), 13 pairs: **`ds_booth_glass` 4.97 M** on 12 pairs (biggest: Sebenco 1.04 M, bias -23;
Modesto 0.85 M, -27; Sol 2 0.79 M, +4; Tech de Ra 0.69 M, +13; Talon's 0.35 M, -13; Vineta K 0.26 M, -15) and
**`glass_reflect` 1.99 M** in block 3 plus 0.98 M in block 1 (Sol 2 1.18 M, +42 brighter; Sebenco 0.29 M and
0.62 M; Chenghou, Ubermall, Metropia 0.1 to 0.2 M). Together about 7 M, 4.5% of the frame. Signs differ by
circuit (ours darker where the pane sits over the arch on Modesto, Sebenco, Talon's and Vineta K; brighter on
Sol 2 and Tech de Ra), consistent with ours drawing a stand-in over a program that reads the circuit's sky
map (a reading). (Sol 2 and Sebenco both show 26,102 pixels in their `glass_reflect` block 3 masks; the
shapes differ, 41% overlap, so it is a coincidence of the threshold.) Both families are listed as open,
unread in `hd-fidelity-lanes-queued-after-the-cold-originals.md` and `hd-track-tone.md`; this measurement says
where they cost most. `glass_texture_customr` (Modesto, wired 2026-10-22) still reads +23 bright over 1.14 M
there.

**Opaque scenery: no lane, and no lighting attribution.** 18.8 M (12.2% of the frame) across both blocks,
about 0.2 to 1.2 M per pair per block, bias +0.9 to +2.8: many draws, fails the falsifier. The
"light-sensitive" label is a threshold artefact: about 82% of the opaque pixels clear the albedo-only delta
of 12 (1.86 M against 0.40 M "rest" in block 1), as expected when the whole light rig and the exposure
adaptation change, so it does not separate lighting from texture. Two named exceptions:
**the-amphiseum** block 1 (2.42 M, bias +7.0: ceiling and cone surfaces, overlaps `hd-top-right-band`) and
**vineta-k**'s right building (box x 1300-1920, y 0-540: 8.6 M, bias +21), where skipping `and_concretestrip`
(0.99 M inside, bias +32) and `and_solidwhite` (0.54 M, +14.5) removes the bright ledge strips. Those strips
are present with `OAG_OPAQUE_ONLY` and vanish with `OAG_ALBEDO_ONLY`, so they are a lighting term on opaque
geometry. **Guess, not measured:** the no-LG2 stand-in specular in
`hd-lit-billboard-specular-and-gantry-leftovers.md`; no switch exists to turn it off in a scratch render.

**Not measured, named so nobody reads them as zero.**

- Missing geometry in ours: an ablation can only remove what ours draws. The "rest" and "sky" masks carry
  1.5 to 2 M each and the crops read (all 13 pairs, blocks 1 and 3) show no missing structure, but absence
  is not tested.
- Fog and haze: no toggle exists (`rg` finds fog code in `crates/raceplay` and `crates/render`, no environment
  variable), so it is **unmeasured**, not zero.
- Hue beyond the card panels (RGB bias was taken in the card and scenery masks only), normal maps (the glass
  and wall programs omit them), and high-frequency error (this section is all low-frequency).
- The cones and top-right band belong to `hd-top-right-band`; not repeated here.

### Lane claims, ranked by impact

1. **Gantry card panels: DONE 2026-10-26 (`hd-card-panels`), partly.** The cause was the card's format, not the
   board's program: the original's card is `R5G6B5` holding linear light, so the dark red rounds to 0
   (`oag_title::adverts::Adverts::card_565`, `SceneStore::Packed565`; `docs/rendering/start-gantry.md`, "The card is 5-6-5
   in linear light"). Panel (teal pixels of the card mask) red bias +11.1 to +6.3 and panel error 7.34 to 6.14 on the 13
   pairs, no pair worse; card-mask error 9.77 to 9.06 (the glyph's phase is the rest). Acceptance **not met**: the panel red
   is not within +2 of the scenery's on any of the four named pairs (Chenghou +7.7, Sebenco +9.4, Vineta K +7.6, Metropia
   +14.4) and the card-mask error fell 8 to 13 %, not a third. Open: (a) ours' card red level mean 0.294 against the original's 0.267 (cause unread; on Chenghou's strip red is +12.6 before, +6.1 now, +3.0 with the original's own card); (b) **Metropia green +14 and red +14 whatever the card holds**, its `cf_vex_billboard` lit
   combine; (c) the gantry phase pinning on `modesto-heights`, `sebenco-climb` and `anulpha-pass`, untouched.
2. **~~`ds_booth_glass`~~ (wired 2026-10-27, `hd-tech-de-ra-2`: modesto-heights booth 34.5 to 4.2, sebenco-climb 31.3 to 6.2, no
   pair worse by more than 0.01 whole; hd-track-tone.md, "`ds_booth_glass` reflects the sky through its normal map's alpha") and
   `glass_reflect`: measure then wire, one family per commit.** Claim: these two
   families, each on 11 to 12 circuits, cost about 7 M (4.5% of the frame, 19% of block 3 for booth alone)
   because ours draws them without their own programs. Acceptance: block 3 box on **modesto-heights** and
   **sebenco-climb** (booth 0.85 M and 1.04 M, bias -27 and -23) and **sol-2** (`glass_reflect` 1.18 M, +42):
   each mask's error halved, no worse on `ubermall`, `anulpha-pass`, `chenghou-project`. Unread: the two
   fragment blocks (`ds_booth_glass` unit table is in `hd-track-tone.md`, rows `#102` and `#76`; its albedo
   carries gamma nibble 0). Covered by the existing queue thread; this gives it an order.
3. **~~Vineta K ledge strips and the no-LG2 specular~~ (measured closed 2026-10-27, `hd-tech-de-ra-2`: the box below reads luma -1.4, the specular went with `hd-top-right-band`): wire lane, small.** Claim: `and_concretestrip` and
   `and_solidwhite` read +32 and +14 luma on Vineta K's right building because they draw a stand-in specular
   (a guess until the family's `LG2` census, recipe in the specular thread, is run). Acceptance: Vineta K box
   x 1300-1920, y 0-540, bias from +21 to within +8, craft and track blocks unchanged. Only 1.5 M sits inside
   the strips, so it is third by size.

Not lanes: opaque scenery elsewhere (diffuse, 12% of the frame, fails the falsifier), the `talons` pair
differences (capture mismatch), the `glow`-material banners on Tech de Ra (**closed 2026-10-26, `hd-tech-de-ra`**:
the surface was `nr_twinblend` slot 133, not the `banner_*_glow` banners; mask bias +30.35 to +1.13, error 0.809 to
0.087 M; Metropia +11.4 and Sebenco +4.7 unchanged, the other 12 pairs byte-identical), the
GO board's pulse phase on the pairs that already pin it.

## Top-right band, measured and wired (2026-10-26, `hd-top-right-band`)

Claim (row 5): ours runs too bright in tx 9-12, ty 0-1; acceptance, mean luma bias within +2 on
`the-amphiseum` and `vineta-k` with the craft and track blocks unchanged. **Half true, half open.**
Evidence and method are in `docs/formats/rcsmaterial.md`, "A program with no `LG2` reading a dot product
draws no specular; the cone's taps are sRGB-decoded". Per-slot ablation on both pairs, then a live RPCS3 frame
dump for the cone's sampler words.

- **`vineta-k` (ramp-edge lines): a specular the program does not compute.** Slot 257 (`lambertzeroalpha`
  `martin_alum_grill`) has `LG2`s on its lightmap curve only; `Program::has_dot_power` is `false`, so it
  and every program like it draws no specular. Band +14.9 to -3.1 (error 25.1 to 9.1), whole frame +3.1
  to -0.2. Overshoots the acceptance the other way: -3.1.
- **`the-amphiseum` (light shafts): the cone's taps are gamma-7 units** (live draws 724 and 730, `@0x1520`
  matched, address words `0x60710101` and `0x60750102`). Band +16.1 to +5.7 (error 23.5 to 14.4). Not within
  +2.
- **`talons` sky particles and `anulpha-pass` pillar glow: not touched, still open** (not measured; the first
  is capture-mismatched).
- 13-pair band mean +6.3 to +3.0 (error 13.1 to 10.4). Per pair, before to after (bias, error): moa-therma
  +7.1/11.6 to -1.3/5.1, sebenco-climb +8.4/15.1 to +5.5/13.0, chenghou-project +2.7/8.3 to +1.2/7.2,
  tech-de-ra +2.9/6.1 to +2.1/5.5; the other seven moved under 0.5. Worse in absolute error: Talon's and
  Talon's Assegai left wall +0.5 (the cone streaks the original does not show), Vineta K track block +0.1.
- Omega ships both materials as PS4 containers (unread); 2048 neither: not checkable, reasons on the page.

### Open from this lane

1. **Amphiseum band +5.7, 3.4 over the no-cone floor.** Shaft width or the `s` scale (0.49596); alpha
   composition is the same as the authored blend `SRC_ALPHA, ONE`. Needs the cone's geometry against
   the original's at the same pose, or the live `s` read off the draw's fragment constants.
2. **Vineta K band -3.1.** Slot 96's strip is grey where the original's is blue (hue); the tower's
   yellow-lit glass is dimmer than the original's.
3. **Modesto Heights, Metropia and the Talon's pairs band** are unmoved by either rule (+5.6, +0.3 with
   15.6 error, +9.2): another cause, never ablated.
4. **Talon's cone:** same decode applied, not independently verified (no matched frame that frames a
   cone).

**Note (lead, at merge):** the band lane above landed `Program::has_dot_power` (no specular where a
program has no dot-product power). That is lane claim 3's guessed cause for Vineta K's ledge strips, so
re-measure claim 3 on the merged main before drawing it.

## Later

- The left-wall advert timeline (table row 3): capture a same-moment frame
  before calling it a missing decal.
- Craft hull high-frequency error (table row 6, low/raw 0.50): AA or texture
  detail; not a lane until the hull's own draws are checked.
- The `anulpha-pass` pillar glow (row 5) and the `talons` sky particles: one
  circuit each, not yet explained.
- Refresh this table after any HD lane merges (same method, same 13 pairs).
