# HD's frame was too bright and too bloomy; the bloom chain was not what was wrong

2026-08-20, **resolved**. The RE is in [renderer.md](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md) - "runs on 8-bit surfaces", "The 14 surface binds", "The lit track material", "The vertex-light constants", "Ships have no Lambert diffuse". This row keeps only what is not there. **Method**: `--race --ticks 0 --screenshot` at 1440x816 against two rpcs3 grabs at 1280x720 - **close framings, not identical, so aggregates only, never a pixel diff**. **The metric that discriminates is clipped-white share** (all channels >= 250), *not* mean luminance, which matched the reference throughout and is why the defect survived review. **Ablation table**, clipped-white %: baseline **16.5**; scene clamped to `[0,1]` **14.25**; chain in gamma space **17.4**; specular removed **14.8**; bloom removed **4.9**; scene / 4 **0.85**; `scaleAdd` 0.25 **10.7**; no-lightmap placeholder alpha zeroed **9.08**. Reference **5.8 / 7.3**. **Shipped**: the 8-bit surface clamp, the invented `sun * (ndl * baked.a)` diffuse summand removed, and the per-vertex light decoded and added. 16.5 -> **10.20 %**, mean 0.583 -> **0.497** against 0.585. **The mean is the part still wrong**, and it is the honest kind: what is missing is a material, not a light. **Do not tune the chain** - `alpha_contribution`, the exposure constants and `scaleAdd` are each read off the disc and each was verified. **Findings that live only here.** (a) The **sampled lightmap is bimodal on screen**: 67 % pure black (only 85 materials name an `lmaps/*-lmap.gtf`) and 10 % blown white in hard-edged rectangles, so an exponent changes nothing, and removing its sRGB decode moves the frame **0.00** points. (b) **`-lmap.gtf` files are not shadow maps** but 1024x1024 full-colour baked atlases, ~28 per circuit. (c) **The attribute census**: of 983 chunks, **327 declare `lightmapUV` and no colour set, 351 the reverse, 0 both**, 305 neither. **Three negatives, so nobody re-runs them.** (1) Forcing the glow mask's alpha to 0.0 gives a **byte-identical PNG** - `GlowMask::Protected` already masks it off. (2) The colour set as a *multiplied* tint lands good aggregates and **blacks out the banners and the hull**: it is added, not multiplied. (3) The disc's RGBE decode on that colour set yields ~1e-39 and blacks the frame - it is real but reads `SpuVertexColours`, which **no `.rcsmodel` declares** (0 of 123). **Verification**: `just test-data` - all **149 render-touching ground truths pass**; the single failure, `stall_rescue_ground_truth::a_craft_that_stops_on_the_disc_is_put_back`, **fails identically on a clean tree** and is physics. **Still open**: whether those 8 bits hold linear or gamma light (nothing in the executable gamma-encodes; confidence 70 against ADR-0026, not acted on, and the obvious tiebreaker does not discriminate); whether `0x1aaf7631` and `colorSet1` are one attribute; what the colour set's fourth byte gates; and `shadowMapTex`, which the disc projects and we have none of. **On the report of missing geometry at the start line**: not missing - blown-white track surface. The 56 unaddressed nodes are four 2-unit cambots plus sky traffic whose chunks live in **other circuits'** `.rcsmodel` files.

2026-09-06: two of the four open items below are settled, both negative
results, in [renderer.md](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md)
- "Per-texture sRGB/`GAMMA` decode: settled negative" and the paragraph
starting "One of these is now settled, one is not." **`0x1aaf7631` and
`colorSet1` are refuted as one attribute** (confidence 78: their fourth-byte
distributions differ, 255-on-every-vertex versus a 23%/66% split, over a
combined ~5 M vertices - the 297+54=351 chunk count only ever showed they are
*complementary*, never identical). **No per-texture gamma/sRGB control exists
anywhere in the engine's per-texture register set** (confidence 90, all ten
disc format bytes checked against `Texture_BuildGcmRegisters`'s `FILTER`-nibble
write) - and this does **not** settle the linear-vs-gamma question below,
contrary to what this file previously implied: input decode and output
encoding are different ends of the pipe.

2026-09-09: a from-play report that HD/Fury's bloom "seemed to have gone much
stronger recently" was hunted as a regression. **There is no regression, and
the report has a cause anyway.** The RE and the tables are in
[renderer.md](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md) - "The
chain was unswitchable, and how bright it actually is". What lives only here:

**The metric is committed now** - `scripts/clipped-white.py`, because this
row's own 2026-08-20 figures are not reproducible today, the harness having
lived in scratch. Its docstring carries the reference readings.
`data/reference/hd-capture/talons/00.png` measures **3.85 %** and
`talons-fifo/00.png` **6.99 %**; neither is the 5.8/7.3 pair this row cites,
and **which two grabs those numbers came from is now unrecoverable**. Treat
3.85-7.0 % as the reference band and re-derive rather than trusting the pair.

**The switch, which is the finding.** `race::Scene` gated the bloom on
`bloom_enabled && hd.is_none()`, so `[graphics] bloom` reached only the PSP
chain. Flipping it moved a Pulse frame 2.069 % -> 0.408 % and left an HD frame
**byte-identical**. **The key defaults to `false`**
(`settings::default_bloom`), so this was never one player's configuration: HD/
Fury was the only title that bloomed out of the box. Nothing about anyone's
settings changed - what changed is that HD races became reachable through the
menus, its front end having become walkable on 2026-09-05. That last step is a
**hypothesis, not a measurement**; what is measured is that the switch was
inert.

Two traps for the next bisect across this window: a build at `5189f942^` **dies
with SIGKILL on the real audio path** and needs `--dump-audio` (the null
backend) to run headless at all - peak RSS 57 MB, so not OOM; and
`--dump-audio` is **pixel-neutral**, byte-identical at tick 0, so a run that
needs it is still comparable with one that does not. Fixed in `9f6a1be3` via `hd_bloom::Glow`, which suppresses the summand
and leaves the exposure resolve running - no read constant moves.

**Three negatives, so nobody re-runs them.** (1) The glow-mask alpha ablation
still gives a byte-identical PNG - the term is as inert as it was on
2026-08-20. (2) Nothing accumulates: stationary ticks 0..1200 read
9.86-10.58 %, and under autopilot the frame sits at 5.8-6.0 %. (3) A build at
`5189f942^` and one at `2871f895` measured at six matched `--pose` framings
give a *lower* number today at all six, so the 2026-08-31/09-02 glow and pad
commits did not brighten the frame.

**Two defects filed and deliberately not fixed**, both in renderer.md's own
list: `fs_blur`'s tap offset escapes `drawn()`'s clamp (inert at native, wrong
under DRS/FSR), and `mesh.wgsl` sums the HD emissive `glow` into `plain` only,
so on HD every `in.lit == 1` chunk drops it - the layer `5189f942` landed
reaches at most 3.60 % of the frame, the far background and the hull.

**Not measured, and it should be said plainly**: Fury's own circuits and any
mounted DLC pack. The sweep was HD's sixteen base environments and the numbers
above are Talon's Junction.

2026-09-09, later the same day: **the defect filed above is fixed, and the
reference band it was going to be judged against was wrong.** The RE, the
tables and the sweep are in
[renderer.md](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md) - "The
emissive glow belongs on the lit path, and the reference band was padding".
What lives only here:

**`data/reference/hd-capture/talons/00.png` is 52.2 % black padding** - a
1278x718 frame on a 1600x1200 canvas. Its 3.850 % is a share over the canvas;
over its own picture it is **8.056 %**. So the "3.85 %" this row published
above as the band's *lower* bound is really its upper one, and the pair's
1.8x disagreement had a mechanical cause the whole time. Trimmed, the
seventeen rpcs3 race grabs span **2.848-20.549 %, median 7.491 %**, and they
split by speed rather than by grab: 6.40-8.06 % on the grid, 14.41-20.55 % in
motion. **`scripts/clipped-white.py`'s docstring still carries the old pair**
and was left alone deliberately - it belongs to another lane - so read the
band off `scripts/hd-glow-sweep.py reference`, which trims before it measures.

**The harness is committed this time**: `scripts/hd-glow-sweep.py`, both a
`reference` and a `sweep` subcommand, writing its own `settings.toml` into a
scratch `XDG_CONFIG_HOME` so a run never reads or edits the machine's own
configuration. Its baseline column reproduces this row's 2026-09-09 sweep
exactly (`04_chenghou_project` 1.319 %, `12_sol_2` 18.463 %), which is what
says it is the same instrument rather than a second one that agrees.

**Two negatives and one surprise, so nobody re-runs them.** (1) The filed
"at most 3.60 % of the frame" **does not reproduce**: forcing the pre-fix
layer to flat red against a build with it zeroed changes **0 pixels** on
Talon's Junction and **1** on Amphiseum. The layer reached nothing, which is
what this row's own "zeroing the glow moves the frame 0.000" always said.
Post-fix it reaches 10.1-10.4 %. (2) The decoded and undecoded domains for the
glow sample differ by 0.02-0.27 points on four of five circuits, so the
measurement does not choose between them and the consistency argument did -
**chosen, not measured**, no confidence score. (3) **Zone 1 gets darker** when
the term is added - and the discriminator is in the same table: bloom **on**
goes 18.490 -> 18.047 %, bloom **off** goes 13.138 -> 13.146 %, *up*. The
darkening exists only where the chain runs, so it is the ladder feeding the
luminance adaptation. Adding light to this chain is not monotonic in the
output.

**Unexplained and not this change's doing**: **eleven** of the sixteen
circuits measure identically with `[graphics] bloom` on and off, on the
baseline build as well as the fixed one. The five that respond are the four
named environments plus Zone 1. **The obvious hypothesis is already dead** -
six of the eleven moved under this change (`01_vineta_k`, `03_track`,
`04_chenghou_project`, `05_ubermall`, `10_sebenco_climb`, `15_anulpha_pass`),
so they do reach `lit_linear` and take the authored path, and still do not
respond to the switch.

## Open

- Whether the 8-bit surfaces hold linear or gamma light is undetermined (confidence 70 against ADR-0026, not acted on) - still open, unchanged by the above
- What the colour set's fourth byte gates is unknown - `0x1aaf7631` is declared by no `SHO` shader block, so the only place left to check is per-material microcode
- `shadowMapTex` is projected by the disc and unimplemented here
- `hd_bloom.wgsl`'s `fs_blur` adds its tap offset after `drawn()`, so the sub-rectangle clamp does not constrain the taps - inert at native, wrong under DRS/FSR
- `scripts/clipped-white.py`'s docstring still publishes the 3.85/6.99 pair, and 3.85 % is a padded canvas rather than a frame - another lane owns that file, so the correction lives in `scripts/hd-glow-sweep.py` and renderer.md instead
- Why **eleven** of the sixteen circuits measure identically with `[graphics] bloom` on and off, on the baseline build as much as the fixed one - and six of those eleven do take the authored path, so "they never light" is refuted
- Amphiseum, Fury's circuits and the DLC packs have no rpcs3 grab to be read against at all

## Next Steps

- Sweep per-material microcode (`scripts/ps3-microcode.py`) for a read of `0x1aaf7631`'s fourth byte, to settle what it gates
- `shadowMapTex` remains unread; locate what projects it and whether the disc's shadow map is reachable from data already on disc
- Re-derive the reference band from a fresh rpcs3 grab at a framing our `--pose` can reproduce, so the next comparison is not aggregates-at-a-distance
- Grab Amphiseum on rpcs3: it is the circuit the emissive layer is busiest on (+1.70 points at the grid, 4.84 % of the frame over 8/255) and the only one whose fix has no reference at all to be read against
- Find why `[graphics] bloom` is inert on eleven of the sixteen circuits - `scripts/hd-glow-sweep.py sweep --bloom both` is the reproducer, the four named environments plus Zone 1 are the five that do respond, and six of the inert eleven measurably take the authored path so the obvious hypothesis is already refuted
