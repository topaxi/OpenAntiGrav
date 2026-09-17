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

2026-09-13: **the matched-camera comparison landed, and it reverses the
direction of every reading above.** Full account in
[renderer.md](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md), "The
matched-camera comparison: the frame reads darker than the original, and the
post chain is cleared of it". Every prior figure on this page was aggregates
over two close-but-not-identical framings; `data/reference/hd-capture/
talons-matched/{00,01,03}` gives an exact camera match, and at an exact
match **our frame is darker than the original everywhere measured** -
whole-frame mean luminance -0.13 to -0.24, road and distant-geometry regions
the same direction - not the "too bright" this page's own title names. Two
negatives narrow where that gap can be, so they should not be re-run: forcing
the exposure resolve's `scale` to `1.0` changes nothing (it is already
pinned there for this scene, `.envsettings`-read `Tone` family and all), and
`[graphics] bloom` on vs. off moves the reading by only 0.02-0.05 against a
0.13-0.24 gap. **The remaining gap is not this chain's** - it localises to
`mesh.wgsl`'s lit material path and possibly `sky_cube.rs`'s texture decode,
both `lane-ship-hull` territory this session did not touch. The harness is
`scripts/hd-frame-compare.py`, committed this session.

2026-09-13, later the same day (`lane-hd-dark-frame`): **the gap's shape is
narrowed, two comparison-harness defects are found and fixed, and the lit
path itself is still open.** Full account in
[renderer.md](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md), "The
darkness gap does not reduce to an offset, a scale, or a single curve". What
lives only here:

**The per-pixel regression the previous entry read as "not gamma" could not
have said either way** - its own R^2 (0.01-0.26) says it lacks the pixel
correspondence to discriminate anything, which is why `hd-frame-compare.py`
gained a rank-matched quantile-quantile output that needs no correspondence
at all. Read against that, poses `00`/`01` show a gap that grows through the
low-to-upper range and collapses to zero at both true black and true white -
not a constant offset, not a proportional scale, and not a single power law
either (the pointwise exponent runs 0.15 to 0.78 across the same data,
where a real curve would hold one value). **A single global transfer-
function mismatch does not fit the whole-frame aggregate, confidence 80** -
narrowed from the 70-confidence stalemate, not settled to zero and not a
refutation of candidate (a) as such: a per-material mix of a correct and an
incorrect encode path produces the same non-constant exponent, and 275 of
Talon's Junction's 301 drawn materials take a lit formula this reading has
not checked per-material. ADR-0026's own question about what the 8-bit
surfaces hold stays open; settling a per-material transfer-function defect
needs a per-material probe, not this aggregate. The residual at true black
(pose `00`, post-aspect-fix: `ours 0.050` vs `ref 0.078`, `+0.028`) is also
worth carrying forward - a pure power law through the origin would put that
gap at zero.

Candidate (b) is refuted for the core terms with an exact match: Talon's
Junction's raw `.envsettings` (`scripts/psarc.py cat`) and
`hd_light_probe`'s echo of the same values agree to the printed digit for
ambient, sun colour, prelit scale/power and specular scale. Nothing is read
short for this circuit's light rig.

**Two things a session chasing this gap should not re-derive.** (1) Poses
`01`/`03`'s reference frames are moving (529/431 km/h) and ours are posed
statically - the original's own speed streak is baked into those captures by
construction and no render setting reproduces it, so pose `00` (74 km/h) is
the only one of the three that isolates the lit-material question; `01`/`03`
corroborate direction only. (2) The comparison itself was rendering at the
wrong aspect (`psp`, 30:17, the project's own default, against a 16:9
canvas) - fixed in `hd-frame-compare.py` by pinning `aspect = "wide"` - which
also surfaces a real `hd_bloom` defect (a dropped viewport offset leaves a
10-column strip unwritten at the canvas edge on every HD/Fury frame this
project renders at a non-native aspect, absent on Pulse) filed rather than
fixed here, since `Chain::run`'s signature is `lane-ship-hull`'s file. Its
effect on the darkness reading is under 0.005 luma either way - real bugs,
not the cause.

2026-09-13 (`lane-hd-captures`): **the darkness gap does not reproduce with
one sign or magnitude across circuits, which is new evidence the gap is
per-material rather than global.** Two matched-camera pairs added, `Sol 2`
(fog-heaviest circuit by `.envsettings` data) and `Amphiseum` (busiest
emissive layer, previously ungrabbed), both via Racebox
(`--team feisar_c1 --hull-variant concept1`, confirmed live rather than
assumed - see `rpcs3-capture.md`'s new "Racebox races the same hull the Team
Selection screen does not show"). Sol 2 corroborates Talon's Junction: ours
darker by 0.15-0.33 whole-frame luma across its three poses, same sign and
similar order of magnitude to the established 0.13-0.24. **Amphiseum
reverses it**: ours reads *brighter* by 0.04-0.19 luma whole-frame - a sign
flip a single global offset/scale/exposure-curve cannot produce. Part of
Amphiseum's gap has an identified, distinct cause rather than being an
unexplained tone shift: at the grid-start tunnel, `oag-game` draws the two
angled trackside wall panels as flat unlit black where the reference frame
shows them brightly lit cyan/white (`data/reference/hd-capture/
amphiseum-matched/00.png` vs the paired `...-compare/00-ours-bloom-on.png`,
gitignored) - visible without reading a number, and consistent with the
already-open emissive/second-texture material gap below, now corroborated
on a second circuit. Poses further into the Amphiseum lap do not show the
black-panel defect and read close to the reference in lighting character,
which is what pins it to that specific geometry rather than the whole
circuit. Sol 2 also weakly corroborates "fog is off/imperceptible here" from
the sibling thread: no distance-graded haze visible at its grid pose either,
on the circuit with the heaviest authored `Fog.Fog Density` on the disc.
Full per-region numbers, the region-box caveats measured this session (Sol
2's `road surface` boxes land off-track at its grid pose; Amphiseum's `sky`
box samples an indoor ceiling, not atmospheric sky) and the fog-density
table are in `/tmp/oag-drive/lane-hd-captures-report.md` (scratch, not
committed). `scripts/hd-frame-compare.py` gained `--pair-dir` for this,
deriving `--track` from each pair's own JSON rather than a hardcoded
Talon's-Junction constant.

2026-09-13, later still (`lane-hd-material-curve`): **the letterbox-offset
defect filed twice above is fixed, generally, and a per-material probe of
the darkness gap ran - it does not locate a fixable term, and it finds the
one lead it did check (the blanket `sun_diffuse`) pointing the wrong way.**
Full account in [renderer.md](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md),
"The letterbox strip is fixed at the engine level, and a per-material probe
of the darkness gap is inconclusive". What lives only here:

**The fix**: `oag_render::post::hd_bloom::Chain::run` and
`oag_render::post::bloom::Bloom::render` both take the scene's own drawn
`origin` now, threaded from `crates/game/src/race/scene/frame.rs`, and each
chain's write-back-to-the-shared-canvas pass anchors there instead of at
`(0, 0)`. Verified on a non-wide aspect at `--size 1280x720`: the right-edge
strip goes from a 10px-wide, asymmetric black column to a symmetric ~5px
margin either side, matching the expected letterbox math. A GPU unit test
pins it. `hd-frame-compare.py`'s own pose-00 numbers do not move - it pins
`aspect = "wide"` specifically so this was already zero there.

**The probe found a confound before it found anything about the gap**: fog
reaches `OAG_TINT_MATERIALS`'s unlit stand-in path, because `mesh.wgsl`
calls `fogged()` unconditionally regardless of `lit`. A plain exact-match
tint classifier therefore only ever attributed the nearest ~30-44% of the
frame to a known material. `scripts/hd-material-probe.py`'s classifier now
matches a pixel against the segment from its candidate's tint colour to the
circuit's own `Fog.Fog Color` (read off `.envsettings`, not fitted),
recovering 86-89% coverage - the remainder reads as fog-dominated far
geometry a straight-line approximation cannot arbitrate, not a broken
instrument.

**On the gap itself**: 14 materials met a 500-pixel floor at pose `00`,
every one `ref - ours` positive (+0.05 to +0.23), not clustering tightly by
the `lightmap`/`ambient`/`sun` role bits `skin::roles` already carries (two
materials sharing identical bits span 0.05 to 0.23). The one candidate this
session could check against a known-different population -
`track_surface`'s own microcode lacks the `N.L`/sun term every other
material's blanket formula adds, so it should read *relatively* brighter
here, over-lit by sun it should not receive - reads the other way instead:
`track_surface` averages `+0.21` against `+0.12` for the rest, stably across
tolerance settings. The probe is sensitive enough to separate the two
populations; the separation argues against the one term it set out to
check. **No fix applied** - this session does not have a second,
disc-sourced candidate to check, and `CLAUDE.md`'s rule against a fitted
constant rules out reaching for one from the table alone.

2026-09-13, later still (`lane-hd-resolve-fill`): **the "why do eleven of
sixteen circuits read identically with `[graphics] bloom` on and off" open
item below is answered, mechanically.** Full account in
[renderer.md](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md), "The
resolve's Fury circuits carry the front end's own Tone triple, and the plain
variant is confirmed live". What lives only here:

**The eleven are exactly the circuits whose `envsettings_bloom` returns
`None`.** `zone_2`/`zone_3`/`zone_4` ship no `track.envsettings` at all; the
eight Fury/DLC circuits (`01_vineta_k`, `02_track`, `03_track`,
`04_chenghou_project`, `05_ubermall`, `10_sebenco_climb`, `12_sol_2`,
`15_anulpha_pass`) each author `Tone adaption boost` but never `Tone
darkening clamp`/`Tone maximum brightness` - confirmed by a full dump of
every `.envsettings` on the disc, not a grep miss. `envsettings_bloom`
requires all ten `HDR and Bloom` keys present in the **circuit's own** file
and returns `None` the moment one is missing, so `oag_render::post::hd_bloom::Chain`
is never constructed on any of those eleven - not a bloom that runs
unbloomed, no gate/blur/exposure-resolve/HDR-encode of any kind. The five
that *do* respond (`amphiseum`, `modesto_heights`, `talons_junction`,
`tech_de_ra`, `zone_1`) are exactly the five whose file is complete.
Confirmed by running the built game and reading its own load report:
`oag-game --race --track 'Data\Environments\12_Sol_2\track.vex' ...` logs
"no complete HDR and Bloom block; the race draws without the read bloom
chain" where Talon's Junction and Amphiseum log the full exposure formula.

**The disc's real engine does not skip the chain there.** A live read during
a Racebox race on Sol 2 found the settings singleton's `Tone` triple holding
`(20.0, 3.0, 4.0)` - the front end's own `fe.track.envsettings`/
`fe.fury.track.envsettings` value, carried forward because the registrar is
a persistent, cumulative object (renderer.md's own prior reading) that a
circuit's file only overrides the keys it declares. So this project's
"require the whole block or skip the chain" policy is a genuine gap on all
eleven circuits, not a faithful reading of a circuit that authors no
exposure - the wiring the fix needs is in renderer.md's new section, and it
is a data-plumbing change (seed from the front-end file, let the circuit
override), not a shader or material one.

**Applying the read exposure `scale` alone to Sol 2's existing render is
still a clean negative**, extending "the exposure stage is already
saturated at `scale = 1.0`" from Talon's Junction (established above) to all
three matched-camera circuits - `scripts/hd-resolve-probe.py`, new this
session. What is not settled: whether the *additive* bloom summand, also
never computed on Sol 2, would move the reading - Sol 2 authors a much
stronger `Bloom from frame contribution` (1.0 against Talon's 0.03) but also
a higher `Bloom adaption boost` (15 against 5) that the existing gate
formula may fold back toward zero on a scene this bright. Not measured -
needs the real chain constructed, `crates/render`/`crates/game` work outside
this lane.

2026-09-13, later still (`lane-amphiseum-walls`): **the grid-start wall-panel
defect is fixed, and it was not the emissive/second-texture *shading* gap
this row's own "New 2026-09-13" bullet guessed - it was the renderer reading
the wrong sampler entry as the panel's picture.** Full account in
[rcsmaterial.md](../../docs/formats/rcsmaterial.md), "A trackside wall panel
drew solid black because its picture was its glow decal". What lives only
here: Amphiseum's `track_wall.rcsmaterial` names four sampler entries -
`EmissiveTexture` (a 95%+ solid-black glow decal) at entry 0, the real
diffuse (`Texture1`) at entry 1, a normal map at entry 2, the circuit's own
lightmap at entry 3 - and `mesh::rcs::skin::picks`'s lightmap branch chose
albedo by raw position rather than by sampler role, landing on the glow
decal every time. Fixed by adding `EmissiveTexture`'s hash to
`skin.rs`'s `NOT_A_PICTURE` list, the same idiom already used for the glass
family's ramps and normal maps; a disc-wide binding census
(`hd_emissive_texture_bind_census.rs`) agrees the hash is never a plain
diffuse anywhere it is bound (71 distinct paths, all glow/advert/gradient by
name). The right-hand panel goes from 0.062 to 0.458 mean luma against the
reference's 0.470 at the matched pose; the left barely moves and stays
brighter than the reference, closer to the crowd stands (a different
material, still over-bright both before and after) than to this panel. **Not
the same mechanism as `etched_glass_tech`'s open routing gap** - that
material's picture is traced correctly and simply has no bound texture at
the unit its combine wants; this one had a bound, correct texture the whole
time, at an entry the picker never looked at.

2026-09-17 (`lane-hd-track-lighting`): **the Amphiseum sign flip is re-
derived at a true 0 km/h grid pose, and it does not survive a bloom-on/off
check - but a large hue gap does.** Full account in
[renderer.md](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md), "The
Amphiseum sign flip is re-derived at a clean pose - the luma reading turns
out to be inside the known bloom sensitivity, but a large hue gap survives
with bloom on or off". What lives only here:

**The captures this row's own "Open" list rested on are gone** -
`amphiseum-matched`, `talons-matched`, `talons-ships` all confirmed absent
from `data/reference/hd-capture/` - and a real bug (`scripts/rpcs3-
drive.py`'s `with session(args) as session:` self-shadow, `UnboundLocalError`
on every subcommand, landed 2026-09-15 in `0a181a67`) meant no lane had been
able to capture anything since. Fixed this session (`open_session` rename);
every subcommand this project's other lanes use for RPCS3 (`boot`, `shot`,
`race`, `capture`, `browse`, `bootchain`, `record`) was silently broken
until now, not only this lane's own captures.

**Fresh 0 km/h grid captures, same protocol both circuits**
(`--load 20 --interval 2`, verified against each pair's own `track` field),
checked with `--bloom on` and `--bloom off` both: Talon's Junction
reproduces the established darker-frame gap and it is robust to the
bloom setting (luma ref-ours +0.121 on, +0.144 off; hue gap 9-13 deg,
brightness-only both ways). **Amphiseum's own luma gap is not robust** -
+0.045 (ours brighter) with bloom on collapses to +0.008 (essentially
matched) with bloom off, a swing the same size as the whole on-bloom
reading, and Amphiseum is one of only five circuits whose bloom chain
even builds (`amphiseum`, `modesto_heights`, `talons_junction`,
`tech_de_ra`, `zone_1`). **So the prior "sign flip" claim rests on a
reading inside the chain's own known 0.02-0.05 sensitivity, not outside
it - it is not falsified, but it is not safe to call confirmed either.**
What *does* survive the bloom check is a 154-155 degree hue gap (on and
off alike, 1 degree apart) - the reference's dome interior reads warm
gold, this project's render reads cool blue-violet, confirmed by eye on
both renders and corroborated on a second pose (22 km/h, shot 06, bloom
on). Two confounds worth checking before trusting a whole-frame hue
number were checked and did not explain it: clipping (ours 6.7% vs
reference 2.8-3.0% with bloom on, nearly matched with bloom off - the hue
gap does not move even though clipping does) and the reference's opponent
craft this project does not render (`coloured_pct`, the share of each
region actually weighing into the hue mean, is 75-92% on both sides, not
a small livery-dominated slice). Read together: Talon's Junction's gap is
brightness-only and robust; Amphiseum's sturdier finding is the hue shift,
not the luma sign, and whether bloom itself over-contributes on Amphiseum
(a third hypothesis, distinct from a per-material lighting gap) is now
open rather than distinguished. `scripts/hd-frame-compare.py` gained mean
HSV saturation, a saturation-weighted circular mean hue and a printed
`coloured_pct` per region this session, plus a center-crop fix for a
1px-per-side reference/render size mismatch that made every comparison
raise before any number came out.

**Not chased this session**: the `track_surface` per-material lead below,
anything in `crates/render/src/mesh/`, `mesh.wgsl`, `emissive.rs` or
`sky_cube.rs`, and whether Amphiseum's bloom chain over-contributes
relative to the original's - the brief scoped this session to
re-deriving the sign first, and the result argues for chasing Amphiseum's
hue gap (and, separately, its bloom-chain magnitude) rather than assuming
Talon's Junction's eventual fix reaches either.

## Open

- **The frame is measurably darker than the original at a matched camera, by 0.13-0.24 mean luminance on Talon's Junction and Sol 2, and the exposure/bloom post chain is ruled out as the cause** - open at `lane-ship-hull`'s files (`mesh.wgsl`, `crates/render/src/mesh/`, `sky_cube.rs`), not this lane's; **narrowed 2026-09-13 (twice)**: first, not a single offset/scale/curve (quantile-quantile evidence, confidence 80), and `.envsettings` plumbing is confirmed exact for Talon's Junction's core terms; second, **the gap is not a single global magnitude either** - Amphiseum measures the opposite sign (ours brighter by 0.04-0.19 luma), so what remains is a per-material or per-circuit-lighting-data cause, not a shared constant. Amphiseum's own black-wall-panel defect is **fixed** (below, `lane-amphiseum-walls`) and was a distinct bug (a wrong sampler-entry pick, not a shading gap); the panel's residual colour still reads flatter/greyer than the reference's cyan-white, in the same direction as this broader per-material gap, and is left to whoever picks that up next. **Re-derived at a clean 0 km/h grid pose 2026-09-17 (`lane-hd-track-lighting`, dated entry above): the "opposite sign" reading does not survive a bloom-on/off check** - it is +0.045 with bloom on and +0.008 (essentially matched) with bloom off, inside the chain's own known 0.02-0.05 sensitivity on the one circuit where that chain builds at all, so it is not safe to call a confirmed sign flip. What does survive both bloom settings is a 154-155 degree hue shift Talon's Junction's gap (9-13 degrees, hue intact both ways) does not have - the sturdier finding is the hue gap, not the luma sign, and the two circuits still read as different defects rather than one gap with a per-circuit sign, just on different evidence than originally stated.
- Whether the 8-bit surfaces hold linear or gamma light is undetermined (confidence 70 against ADR-0026, not acted on) - still open, unchanged by the above
- Texture-decode brightness (candidate (c)) is not pixel-verified against a reference decoder - only that `oag-texture`'s `.gtf` path carries no gamma/sRGB logic of its own and its ground-truth test passes
- What the colour set's fourth byte gates is unknown - `0x1aaf7631` is declared by no `SHO` shader block, so the only place left to check is per-material microcode
- `shadowMapTex` is projected by the disc and unimplemented here
- `scripts/clipped-white.py`'s docstring still publishes the 3.85/6.99 pair, and 3.85 % is a padded canvas rather than a frame - another lane owns that file, so the correction lives in `scripts/hd-glow-sweep.py` and renderer.md instead
- ~~Why **eleven** of the sixteen circuits measure identically with `[graphics] bloom` on and off~~ **answered 2026-09-13 (`lane-hd-resolve-fill`)**: those eleven are exactly the circuits whose `.envsettings` does not author the whole `Tone` family (`zone_2`/`zone_3`/`zone_4` ship no file at all; the eight Fury/DLC circuits omit `Tone darkening clamp`/`Tone maximum brightness`), so `envsettings_bloom` returns `None` and `oag_render::post::hd_bloom::Chain` is never built - the switch has nothing to toggle. See the dated entry above and renderer.md's new section. "Six of those eleven do take the authored [lit-material] path" still stands and is a separate finding - the material lighting and the missing exposure chain are two independent absences that happen to overlap on the same circuit set
- Fury's own circuits and the DLC packs still have no rpcs3 grab to be read against at all (Amphiseum now does - see 2026-09-13 above)
- ~~Amphiseum's grid-start trackside wall panels draw unlit/flat-black~~ **fixed 2026-09-13 (`lane-amphiseum-walls`)**: a wrong sampler-entry pick (`EmissiveTexture` bound as the picture instead of the material's own `Texture1`), not the `etched_glass_tech` shading gap this bullet guessed - see the dated entry above and `rcsmaterial.md`. The panel's residual colour (flatter/greyer than the reference's cyan-white) is left open, folded into the per-material darkness-gap bullet above rather than tracked separately.
- ~~what makes Amphiseum's opening seconds already 406-429 km/h at the same capture settings that gave Talon's Junction and Sol 2 a slow (70-74 km/h) grid pose is unread~~ **answered 2026-09-17 (`lane-hd-track-lighting`)**: neither a downhill start nor a shorter countdown - `cmd_capture` holds thrust for `--interval` seconds before every shot including the first, so shot 00's speed is an artefact of where the fixed `--load`+`--interval` window happens to land relative to `GO`, not a circuit property. A shorter `--load`/`--interval` isolates a true 0 km/h grid pose on both circuits; see the dated entry above for the resulting sign-flip re-check
- `scripts/hd-material-probe.py`'s fog-segment classifier assumes the frame's exposure scalar is `1.0`, calibrated by eye against one unclipped near-camera channel rather than solved - a session that wants coverage past 89% or tighter per-slot deltas needs to actually solve for it (or read `k` off `.envsettings`' `Tone` family and the adapted-luminance state directly) rather than assume it
- ~~`envsettings_bloom` (and possibly `envsettings_fog`/`envsettings_light`) should seed from the title's own front-end `.envsettings` for any `HDR and Bloom` key a circuit's file omits~~ **done 2026-09-13 (`lane-envsettings-carry`)**: `crates/game/src/race/load/environment.rs::staged_envsettings` carries `/data/fe/fe.track.envsettings` forward for exactly the keys a circuit's own file does not declare, and `hd_envsettings_carry_ground_truth.rs` now measures 16 of 16 environments building the chain (was 5 of 16). `envsettings_fog`/`envsettings_light` were checked for the same partial-key gap and do not have it - every circuit that ships a `.envsettings` authors a complete `Fog`/`Lighting` block on its own, so they were deliberately left un-carried; see renderer.md's new section for the full account, including a correction to this thread's own prior "both Fury front-end files carry `20/3/4`" reading (DATA02's own does not). **Whether wiring it up closes Sol 2's own darkness/under-clipping gap is now measured**: `hd-frame-compare.py`'s pose-00 numbers move toward the reference on every region but road surface (13-24% of the per-region clip-share gap), which corroborates the gate-cancellation hypothesis below rather than confirming it outright - the chain is real and doing something small, not nothing, and what is left open is everything this thread's first bullet already names

## Next Steps

- **New 2026-09-17**: Amphiseum's 154-degree hue gap (dated entry above) is the sharpest lead for that circuit specifically - reference reads warm gold, this project's render reads cool blue-violet, over a region needing no per-circuit box work (`"whole (excl HUD, craft)"`). Not yet localised to a material, a lightmap, an ambient/sun colour term, or the dome's own emissive surfaces; `crates/render/src/mesh/`, `mesh.wgsl`, `emissive.rs`, `sky_cube.rs` unchanged this session. Worth checking before assuming it is the same `track_surface`/`prelitScale` lead below: that lead was built entirely on Talon's Junction data, which this session's own re-check shows has *no* comparable hue gap (9 degrees, noise-level) - so whatever explains Amphiseum's colour shift is not yet shown to be the same mechanism at all.
- The per-material probe's own contradiction is the sharpest lead now: find why `track_surface` (no authored sun term) reads *more* deficient than the population that does get the blanket sun addition, not less - candidates worth checking against the disc before anything else: whether `track_surface`'s own `prelitScale`/`prelitPower` pair actually differs from the shared assumption, whether its lightmap texture decodes correctly, and whether its distinct `specular_exponent` (70, against the 32 the rest of the sampled population carries) points at a genuinely different `.rcsmaterial` row being read
- Extend the per-material probe past pose `00`'s 14-material sample - run it against Anulpha Pass or another circuit with more materials in frame, and reproduce the same role-bucket/track_surface checks, to see whether the "no clean clustering by role bits" reading holds generally or is a small-sample artefact of the fourteen materials pose `00` happens to show
- Pixel-verify `oag-texture`'s `.gtf` decode against a reference decoder on a flat, evenly-lit Talon's Junction albedo (candidate (c), not reached this session - only that the decoder carries no gamma logic of its own and its ground-truth test passes, neither of which rules out a bit-level DXT/BC defect)
- Sweep per-material microcode (`scripts/ps3-microcode.py`) for a read of `0x1aaf7631`'s fourth byte, to settle what it gates
- `shadowMapTex` remains unread; locate what projects it and whether the disc's shadow map is reachable from data already on disc
- ~~Grab Amphiseum on rpcs3~~ **done 2026-09-13**: `data/reference/hd-capture/amphiseum-matched/` (3 matched-camera poses) - see the dated entry above for what it found (a black-wall-panel defect at the grid, and a sign-flipped whole-frame gap versus Talon's Junction/Sol 2)
- ~~Chase the Amphiseum grid-start wall-panel defect~~ **done 2026-09-13**: `EmissiveTexture` was being bound as the panel's own picture - see the dated entry above
- Extend `hd-frame-compare.py`'s region boxes past the shared Talon's-Junction shape, or accept per-circuit `--dump-regions` verification as the standing process - Sol 2's grid pose lands `road surface` off-track, Amphiseum's `sky` box samples an indoor ceiling. **Narrowed 2026-09-17**: this only matters for `sky`/`road surface`/`distant geometry` - `"whole (excl HUD, craft)"` only subtracts the fixed HUD boxes and the centred craft box, both of which generalise across circuits without per-circuit tuning, and is what the sign-flip re-check above used for exactly that reason. Amphiseum's own `sky`/`road surface` boxes were `--dump-regions`-checked this session (dome ceiling, side floor panels respectively) but not re-shaped.
- ~~Find why `[graphics] bloom` is inert on eleven of the sixteen circuits~~ **answered, see the 2026-09-13 (`lane-hd-resolve-fill`) entry above and renderer.md**. ~~The next step this opens: seed `envsettings_bloom` (and check `envsettings_fog`/`envsettings_light` for the same gap) from the title's own front-end `.envsettings` for any key a circuit's own file omits~~ **done 2026-09-13 (`lane-envsettings-carry`)** - 16 of 16 environments now build the chain; see the Open entry above and renderer.md's new section for the measured, narrow-not-close effect on Sol 2 and the fog/light no-op finding
