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

**The fix**: `oag_post::hd_bloom::Chain::run` and
`oag_post::bloom::Bloom::render` both take the scene's own drawn
`origin` now, threaded from `crates/raceplay/src/scene/frame.rs`, and each
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
and returns `None` the moment one is missing, so `oag_post::hd_bloom::Chain`
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
anything in `crates/mesh/src/mesh/`, `mesh.wgsl`, `emissive.rs` or
`sky_cube.rs`, and whether Amphiseum's bloom chain over-contributes
relative to the original's - the brief scoped this session to
re-deriving the sign first, and the result argues for chasing Amphiseum's
hue gap (and, separately, its bloom-chain magnitude) rather than assuming
Talon's Junction's eventual fix reaches either.

2026-09-17, later the same day (`lane-hd-amphiseum-hue`): **the maintainer
was asked directly whether Amphiseum reads brighter or darker, brightness
or colour - "Both, and it varies by area."** That answer is why the
whole-frame numbers above kept disagreeing with themselves: a box-free
tile grid (`scripts/hd-frame-compare.py --tiles 4x6`, new this session)
shows the ceiling/stands half of the frame reads **brighter** in this
project's render (px-weighted ref-ours -0.121) and the floor/barrier half
reads **darker** (+0.100) - a genuine vertical split, not one gap with an
unstable sign. That 0.22-luma swing from framing alone is bigger than the
0.053 bloom swing the entry above measured, and holds with bloom off too.
Hue tells a sharper story than magnitude: the reference is warm
(39-86 deg) at the top and cool (155-198 deg) at the bottom - two real
material colours - while this project's render sits in one blue-violet
family (217-305 deg) everywhere, so the defect reads as "our render
doesn't reproduce two materials' colour difference" rather than a uniform
colour cast. Talon's Junction's own tile grid, run for contrast, is close
to uniform (20/22 cells one sign, hue gaps under 20 deg almost everywhere)
- the concrete form of "Talon's Junction is brightness-only and global,
Amphiseum is not." Full account in
[renderer.md](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md),
"Amphiseum's gap is vertical, not a single sign". Not yet done: mapping
rows to actual `.rcsmaterial`s (`hd-material-probe.py`); the row labels in
the table are read by eye, not measured.

2026-09-17, later still (`lane-hd-amphiseum-hue`, continued): **the ceiling
rows map to real materials, their albedo textures pixel-verify clean, and
the ceiling's own colour traces to the wired ambient constant, which is
itself blue-violet on disc.** Full account in
[renderer.md](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md), "The
rows map to real materials, the albedo textures pixel-verify clean, and the
ambient constant is itself the colour the ceiling reads". `hd-material-probe.py`
(extended with `--pair-dir` and the same center-crop fix `hd-frame-compare.py`
already carries) confirms the tile grid's row split by material identity:
`animhexlights`/`cf_diff_spec`/`lambert`/`base_diffusespecular` (non-lightmap
slots, delta -0.53 to -0.15, "ours brighter") draw exactly the ceiling rows
(y 0-298), against `track_wall`/`track_surface_no_emissive` (+0.06 to +0.28,
"ours darker") on the floor (y 296-717) - coverage 79.1%, below the script's
own 90% floor, so read as indicative. **The maintainer's own lead 1 (channel
order/`.gtf` decode) is a clean negative for this circuit**: the three
ceiling materials' own DXT1 albedo textures (`dc_hexgrid.gtf`,
`and_metaldark.gtf`, `dc_cement_base_edges.gtf`) decode to plain neutral
grey with `crates/texture/examples/gtf_to_png.rs`, no blue/violet/gold cast
at all - so the colour is not in the texture sample. Three of the four
ceiling materials carry no lightmap and draw chunks with no colour set, and
their normals face down (toward camera) against Amphiseum's own strongly
upward sun direction, clamping `sun_diffuse` toward zero - which leaves
`scene.light.ambient` as close to the only non-zero term feeding a
near-grey albedo. Amphiseum's own `Lighting.Constant ambient color`
(`0.745098 0.611765 0.925490`, hue 265.5 deg) is a hue-family match to the
285-degree hue this session measured on the rendered ceiling. Talon's
Junction's ambient constant is *also* blue-leaning on disc (hue 246 deg)
but its HDR-magnitude warm sun (`2.0 1.827 0.886`) dominates its own
sun-facing pose 00 geometry, which is offered as why the same wired-blue
ambient never surfaces there. **Not confirmed as the root cause** - this is
a mechanism built from disc values already wired into the shader, not a
read of what the original does with them, and the per-pixel `ndl` on the
ceiling's own chunks was not directly measured this session. New RE:
`Environment_RegisterLightingSchema` (`0x003a83d8`, confidence 80, `ps3-
hdfury-eu`) identified - the full `.envsettings` key registrar, confirming
`Lighting.Sky colour` (neutral grey on every circuit, `128-140`) is read
into the *same* live struct `Constant ambient color` lands in, at offset
`+0x440` versus `+0x430` - sharpening but not closing `envsettings.md`'s
existing "Read, unused" line. **Whether anything downstream ever reads
struct offset `+0x440` back out is the open RE question**, not traced this
session. No shader or Rust code changed.

2026-09-17, later still (`lane-hd-ambient-light`): **the "does the original
consume `Lighting.Sky` where we only consume `Lighting.Constant`" lead from
the last two entries is answered - yes it consumes it, but for an unrelated
backdrop pass, and the answer refutes the lead rather than confirming it.**
Full account in
[renderer.md](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md),
"`Lighting.Sky colour`'s consumer is found and is a backdrop clear, not a
material term". What lives only here:

**A struct-offset correction, found while tracing the consumer**: `Constant
ambient color` lands at `+0x420`, not `+0x430` as the previous entry states
(`+0x430` is `Sun color`) - cross-checked against seven other offsets this
page already had independently and all seven agree, so this decompile's TOC
reads sound and the prior `+0x430` line was the error. `Lighting.Sky
colour`'s own consumer (`Scene_PrepareFrame`, `0x003aa888`) is pinned by
tracing the local variable it reads through, not by offset coincidence -
the exact trap this page's own TOC-mismatch section warns about - and it
turns out to feed a screen backdrop/clear-fill colour, gated by a
`g_ZoneEffectsActive` branch and the `Debug_Draw_sky` toggle, entirely
outside material shading. It cannot explain the ceiling's colour: the
ceiling is drawn opaque geometry, which would occlude any backdrop fill
underneath regardless of its colour. **Separately, `Constant ambient color`
(`+0x420`) is confirmed bound into the same per-frame shader-parameter
table `fogColour` uses** - so the original's material pipeline does feed
itself the same ambient constant `mesh.wgsl` already applies; there is no
missing-key or wrong-key defect on the ambient front.

**Two more candidates checked and closed the same session.**
`Lighting.Enable ambient false lighting` defaults off and is declared by
neither circuit's own `.envsettings` nor by the front-end file
`staged_envsettings` would otherwise carry it forward from - off by every
route, so whatever this bit gates (its key names suggest a fake specular
highlight for lightmapped surfaces, not a diffuse ambient blend) is inactive
here regardless. And a new one-off diagnostic
(`crates/render/examples/light_census.rs`, on `oag_vex::vex::nodes`) swept
every `.vex` file on both circuits (9 each) and found **zero**
`AmbientLight`/`DirectionalLight`/`PointLight` nodes on either - ruling out
vex-authored dynamic lights as the ceiling's colour source too, though not
ruling out the SPU dynamic-light subsystem (`Enable dynamic lights=1` on
both circuits, unimplemented in this project) being fed from some other,
unenumerated data source.

**Net: three candidate mechanisms are now refuted, and the root cause is
still not established.** No shading code changed - `mesh/`, `mesh.wgsl`,
`emissive.rs`, `sky_cube.rs` are unchanged, per this project's own rule
against tuning to a reference in the absence of a recovered term. Full
findings, including the gate run, are in `scratch/lane-light-report.md`.

2026-09-17, later still (`lane-hd-ceiling`): **the per-material microcode
sweep two sessions asked for is done, and it names the wrong operation.**
Full account in
[renderer.md](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md), "The
per-material microcode sweep: none of the four ceiling programs ever
references the ambient constant, and the disc's own replacement term
produces warm gold from two of them." Summary: none of the four ceiling
materials' resolved fragment programs (`animhexlights`, `cf_diff_spec`,
`lambert`, `base_diffusespecular`'s non-lightmap slot) ever patches
`constantAmbientColour` - `mesh.wgsl` adds `scene.light.ambient` to all four
anyway, unconditionally. The original ships a genuine **three-way**
permutation (`IleLightmap`/`IleVertex`/`Ambient`, confirmed by dumping all
three feature hashes on two of the four materials): a lightmapped chunk
replaces ambient with `pow(lightmap, prelit_power) * prelit_scale` (already
wired in `mesh.wgsl`, for the lightmap path only); an `IleVertex` chunk (has
a baked colour set, no lightmap - which is what all four ceiling materials'
*drawn* ceiling chunks resolve to) replaces it with `pow(colour_set,
prelitBias) * prelitScaleSpecular`, using the **same** two `.envsettings`
keys (`Prelit ambient colour scale/power`) `mesh.wgsl` already reads for the
lightmap curve, just never applies to the vertex-colour path; only a plain
`Ambient` chunk (neither) actually gets the flat `constantAmbientColour`
`mesh.wgsl` currently gives every chunk. **Reconstructed by hand from disc
values alone (no fitting): one of the four materials clearly lands in the
reference's own 39-86° warm-band hue once the disc's own curve is applied
to the disc's own baked colour** - `animhexlights` at 45.5° (currently
renders at ~200-305° via the wrong flat blue-violet ambient), stable across
raw/top-decile/curved statistics (70/56/45°). `lambert`'s own number
(42.8°) is not load-bearing - only 72 vertices, and its own raw-mean (2.9°)
and top-decile (78.4°) hues disagree by 75° on the same population, which is
noise, not a second confirmation. `animhexlights`' own colour is also close
to grey (~11% saturation), which HSV hue reads noisily on - the reference's
own "39-86° warm" figure used a saturation-weighted mean for exactly this
reason, and this session's number was not checked against it. The other
two, `base_diffusespecular` and `cf_diff_spec`, are a clean negative -
their own baked colour-set data is cool-blue (190-202°) at every stage,
curve included, so applying the correct operation to them specifically
would not turn them warm; whatever supplies their share of the reference's
warm reading is not in this term. **A magnitude check does not corroborate
the hue result**: the curved term times the material's own measured DXT1
albedo overshoots `hd-material-probe.py`'s own measured reference luma by
roughly 4-5x, which says the top-decile-by-luma statistic is the wrong
stand-in for a screen pixel's actual (triangle-interpolated) brightness, not
that the ambient finding is wrong - the magnitude question is open, only
hue was checked. **A self-caught data-handling error, recorded so it is not
repeated**: a per-region follow-up first read three tile-grid cells
(`r0c4`/`r0c5`/`r1c5`) as genuinely warm (52°/334°/4°, the grid's largest
gap) and built a "fifth material corroborates the bug at a verified pixel"
claim on it. Those readings are real but came from pose `01`, not pose `00`
- `hd-frame-compare.py --tiles 4x6` prints one grid per pose back to back,
and grepping the three cell labels without separating by pose merged them.
Rerun with `--pose 00` alone, those same cells read **200-205°** - no warm
signal in pose `00`'s own capture, matching the rest of its top row. The
per-region material classification this session ran (`cf_diff_spec`/
`base_diffusespecular` dominant, `uvanim_diffuse_emissive` a real minority)
was built from pose `00`'s own camera, so it cannot be paired with pose
`01`'s warm reading regardless - poses `01`/`03` are moving (431-529 km/h,
already flagged two sessions ago as carrying the original's own baked-in
speed streak that a static render cannot reproduce or segment by material).
**So the "warm pixels traced to the cool-negative pair plus an emissive
fifth material" claim is retracted as stated.** What survives on its own
footing: `uvanim_diffuse_emissive`'s own resolved program (dumped
independently of any pixel pairing) shares the exact `constantAmbientColour`-
free, `f[TC0]`/`f[TC1]`/`f[TC2]`-additive shape the four ceiling materials
do, plus a genuine `EmissiveTexture` glow layer, and its own role bits
(`no_ambient`, not `no_sun`) put it on the exact `mesh.wgsl` code path
(`(in.slots & 192u) == 192u` false, so it takes the full ambient-corrupted
`lit_sum`) this section's fix targets - a fifth material sharing the defect,
established by microcode alone, not confirmed against any specific warm
pixel. A short RPCS3 recapture was separately attempted, to get a static
pose showing more of the dome directly, and landed on Talon's Junction
instead - `rpcs3-capture.md`'s own carousel-count nav plan does not carry
over unchanged (this build's menu route inserts an unnamed "Single Player"
screen) - not chased further, RPCS3 and its Xvfb stopped cleanly. RPCS3 was
not otherwise needed for the core finding: the brief's gate for a live read
("none of the four programs can produce warm gold from static disc inputs")
is false - `animhexlights` does, from disc values alone. The fix is
sequenced in renderer.md's new
section (drop the ambient add and add the colour-set curve together, not
the drop alone, which would regress the surface to near-black) and left for
the coordinator - `mesh/`, `mesh.wgsl`, `emissive.rs`, `sky_cube.rs` are
unchanged, out of this lane's files.

2026-09-17, later still (`lane-hd-ilevertex-ambient`): **the fix specced above
is landed in `mesh.wgsl`.** First, the reuse question the spec left open was
checked against the disc rather than assumed: `prelitBias`/`prelitScaleSpecular`
(engine parameter slots 13/12) are the *same* two `.envsettings` keys already
bound as `scene.light.prelit_scale`/`prelit_power` for the lightmap curve,
confirmed by an independent offset-arithmetic chain rather than the
value/shape match alone renderer.md's confidence-80 finding rested on -
`Scene_PrepareFrame` binds `Prelit ambient colour scale`/`power` into the
shader-parameter table at `+0x198`/`+0x1b8` (renderer.md, "`Constant ambient
color` (`+0x420`) is confirmed wired..."), and the engine's own 81-entry
table's own offset formula (`0x18 + slot * 0x20`, cross-checked against eight
known slots) solves those two offsets to slots 12/13 - the same slots the
table independently names `prelitScaleSpecular`/`prelitBias`. Two traced
chains landing on the same slot indices, not a name-string guess. No new
uniform was needed.

The shader change: on any chunk whose role bits are `NO_AMBIENT` and not
`EMISSIVE`, `scene.light.ambient` is dropped and the raw `vertex_light` term
(`in.colour.rgb`) is replaced by `scene.light.prelit_scale *
pow(in.colour.rgb, scene.light.prelit_power)` - no sRGB predecode, matching
the vertex program's own raw `LG2/EX2` read. `IleLightmap` chunks lose the
flat ambient too (their own curve, `prelit`, already stood unchanged); the
existing `EMISSIVE` branch is untouched, additive only. A new ground-truth
test, `crates/render/tests/hd_ambient_role_census_ground_truth.rs`, pins the
split on Amphiseum's own resolved materials: 641 slots total, 553
`NO_AMBIENT`, 68 of those also `EMISSIVE` (excluded from the curve).

**Measured as a player would, before/after, both circuits**
(`scripts/hd-frame-compare.py --tiles 4x6`, `data/reference/hd-capture/ilevertex-lane/{before,after}/`,
screenshots looked at directly, not just the numbers):

- **Amphiseum, pose `00`, whole frame (excl HUD, craft)**: luma ref-ours
  moved from -0.246 (ours far too bright) to +0.025 (essentially matched);
  hue moved from 241° (39° from the reference's 202°) to 205° (3° from it).
  Per-tile, the ceiling/stands rows (`r0`/`r1`) show the same shift - hue gap
  collapsed from 26-59° to 0-5° across all twelve top-row tiles, luma gap
  from 0.15-0.48-too-bright to 0.01-0.08-too-bright. The screenshot bears
  this out directly: the ceiling goes from blown-out white/washed to a
  cyan/blue tone close to the reference's. **This is a materially bigger win
  than the spec's own "expect a partial fix" framing anticipated** - most of
  the top-row tiles are `base_diffusespecular`/`cf_diff_spec`-dominated
  (renderer.md's own per-vertex reconstruction found those two stay
  cool-blue through the curve), yet the *rendered, triangle-interpolated*
  picture still closed almost all the way to the reference at this pose, not
  just `animhexlights`'s own corner. Reconciling that against the per-vertex
  finding is unchecked - candidates include the per-pixel blend diluting the
  two stubborn materials' share, or this pose's own framing not showing much
  of them at all.
  **The floor/barrier rows (`r2`/`r3`) got darker and overshot past the
  reference**, from 0.06-0.30-too-bright before to 0.06-0.31-too-dark after
  (e.g. `r2c4`: ref-ours -0.149 before, +0.219 after) - these are
  `NO_AMBIENT` lightmapped chunks losing the flat ambient with no
  compensating brightness gain, the same mechanism the spec named for
  Talon's Junction, showing up on Amphiseum's own floor too. Not chased
  further this session; flagged here rather than silently left in the
  "partial fix" framing since it is a new, measured direction change, not
  just an unclosed gap.
- **Talon's Junction, pose `03`, whole frame (excl HUD, craft)**: luma
  ref-ours widened from +0.233 to +0.308 (got darker, as specced); hue
  essentially unchanged (215° before and after, 7° from the reference's
  208°). Per-region: `road surface` ref-ours widened from +0.277 to +0.386,
  `distant geometry` from +0.273 to +0.323. Screenshots show a subtle,
  uniform darkening of the lightmapped wall/ceiling panels, not a new visual
  defect - nothing went black and no chunk class vanished. **Landed anyway**,
  per the brief: this is the disc's own arithmetic and the existing darkness
  gap is already tracked as a separate, open defect (see the "measurably
  darker" bullet below).

Nothing went black and no chunk class vanished on either circuit at either
pose. Gate: `just` and `just test-data` both green on this lane's tree (see
commit for log paths); the twelve-circuit race regression test's own
respawn-count lines matched the 6912dbc6 baseline exactly, digit for digit.

2026-09-18: the "vex-authored dynamic lights (none present)" refutation two
sessions above was narrow on purpose - it checked two circuits' `.vex` for
`AmbientLight`/`DirectionalLight`/`PointLight` nodes, not whether HD's
shaders would do anything with a `PointLight` if one were in frame. Extended
disc-wide, on the other end of the pipe: a name-hash sweep
([`scripts/hd-pointlight-sweep.py`](../../scripts/hd-pointlight-sweep.py),
committed) of every `SHO` block on `hdfury-ps3-eu-dec.iso` - every compiled
variant in all 1,632 `.rcsmaterial` entries plus every block resident in
`EBOOT.elf`, 97,861 blocks total (97,735 from materials, 126 from the
executable), all parsed, none failed - for the three named `pointLight0*`
engine-parameter-table slots found zero references, register-bound or
patched, anywhere. **A register-arithmetic first attempt at this got the
wrong answer and was thrown out** - it inferred a parameter from a vertex
instruction's raw constant-register number rather than its name hash, and
two checks (a hand-disassembled false positive, and a known-positive
parameter that came back zero the same way) showed register numbers
collide across materials and prove nothing; the tool's own docstring and
renderer.md both carry the failure as a named trap. The corrected method -
the same per-block parameter table `fp_patch_map` already reads by name
hash, extended to the register-bound case - needed both its branches
proven live before the zero meant anything: `constantAmbientColour`
(patched branch) finds 19,464 hits across 1,515 materials, and
`positionScale` (register-bound branch, the one a vertex-side point-light
parameter would actually use) finds 60,267 hits across 1,583 materials -
both the shape expected of genuinely widely-used constants.
`pointLight0*`'s zero is disc-wide across every compiled
permutation, not the single resolved variant the ceiling investigation
checked by hand, so it does not conflict with that narrower finding: a
material can ship an ambient-enabled variant nobody currently selects. Full
account and confidence (85):
[`renderer.md`](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md), "A
disc-wide name-hash sweep for `pointLight0*` finds no consumer anywhere";
cross-referenced in
[`lighting.md`](../../docs/formats/lighting.md)'s Open section. **This
closes the point-light sub-question of the "vex-authored dynamic lights"
candidate as a converged negative on this disc** - HD authors 1,160
correctly-classed `PointLight` nodes elsewhere on the disc (confirmed
against HD's own class table, not a Pulse-borrowed id), ships no
`PointLight_Importer.cpp`, and no shipped material's shader declares the
parameters a point light would need. Not the ceiling's colour source, and
not a render
feature to implement absent a runtime trace that overturns this. Does not
touch the SPU per-vertex dynamic-light subsystem
(`Enable_spu_vertex_light`) named in the same `renderer.md` section as the
"plausible, unchecked" mechanism - that is a different system, still open,
and this sweep says nothing about it.

2026-09-18, separate lane, `EBOOT-ps3-hdfury-eu.elf` open in Ghidra:
`Enable_spu_vertex_light` is traced as far as static PPU reading goes.
Confirmed at `+0x5a3`, defaulting to **on** (`Environment_
RegisterLightingSchema`, `0x003a83d8`, decompiled directly) - a fourth,
previously unlisted key in the same cluster, `Debug_Draw_spu_light_volume`
at `+0x5a4` (default off), was found alongside it. Read at 14 sites
disc-wide; two are in already-named functions,
`Shadow_CompileAmbientShadowTrackRedraw` and
`Shadow_CompileShadowedTrackRedraw` (both cached shadow-redraw
command-list compilers), where it gates - jointly with `Debug.Enable
EdgeGeom`, Sony's real EDGE middleware confirmed by string (not this
project's own naming) - selection of a slot in a `0x1000`-byte-stride
multi-buffer at `iRam008b83b0`, the shape this project's own
`rpcs3-trail-dump.py`/`hd-flare-sprite-dump.py` already read live for other
double-buffered SPU-output regions elsewhere. **What the buffer holds is
not read** - only its address arithmetic is - and the other twelve call
sites are unexamined; this does not establish what the subsystem does,
only that it is real, on by default, and reachable. Confidence 75. Full
account: [`renderer.md`](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md),
"`Enable_spu_vertex_light` is read at 14 sites, two named and reachable,
and gates a double-buffered slot - the light computation itself is not
traced". **Next steps in cost order**: the twelve unexamined call sites
(addresses listed on that page); a live RPCS3 read of the buffer's runtime
address, which this project already has the tooling shape for; SPU ELF
extraction/decompilation last, and only with a new SPU Ghidra processor
module this project has never set up for any title.

2026-09-18, later still, continued in the same lane: **the live read named
above as the next step is done, on both `evdev`/display environment
blockers fixed live in-session** (`uv run --with evdev`, not a system
package; `scripts/rpcs3-drive.py display` for Xvfb :77). New tool,
`scripts/rpcs3-spu-light-dump.py` (committed). The pointer at `0x008b83b0`
is never relocated - its live value matches the static ELF read exactly -
and the buffer holds up to 8 records of 8 floats, `(position.xyz, w=1.0,
A, A·0.25, A·0.1, D)`, the ratio exact on every record checked across two
circuits (confidence 80 for this structural claim, a repeated direct
measurement). Some records move at racing-speed magnitudes; at least one
stayed byte-identical across every snapshot on both circuits tested -
reproduced, not one track's artefact. **Checked directly against "these
are authored `.vex` markers" and it fails**: a new tool,
`crates/render/examples/hd_amphiseum_spu_light_marker_check.rs`, swept
Amphiseum's stationary position against all 2,790 node-transform
translations across every `.vex` file and every class - closest is 63.71
units away, no match. Whether the records are lights, computed markers, or
something else remains open (confidence 75 or below for any claim about
what they *are*, as opposed to their structure); slot 1 has never been
read, only slot 0; no shader consumer is found, and the standard
engine-parameter table has no slot shaped for this, so a consumer - if one
exists - is likely a vertex texture fetch, a mechanism no tool in this
project currently checks for. Full account:
[`renderer.md`](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md),
same section, "2026-09-18, later still: the live capture happened". No
code changed - this is the strongest evidence this thread has reached and
still short of what "never invent" requires before wiring anything.

2026-09-18, later still: the vertex-texture-fetch mechanism named above as
unchecked is now checked, disc-wide, and comes back clean too - zero `TXL`
instructions in any of 60,324 vertex blocks across every `.rcsmaterial` on
the disc. One ambiguous, architecturally implausible hit in `EBOOT.elf`
itself does not match the shape a consumer of this buffer would need
regardless. Both consumer mechanisms this thread could think to check are
now disc-wide negatives; full account in renderer.md, same section.

2026-09-18, same session: `uvanim_diffuse_emissive`'s own colour, the Next
Steps item below, is read. `hd_amphiseum_ceiling_variants.rs`'s material
filter now covers it (committed), which located its five slots at the
already-identified `0x56c94426`/`0x6d60` variant (`178, 354, 357, 382,
432`); `hd_amphiseum_ceiling_vertex_light.rs` (unchanged - already takes
slot numbers) reconstructed each the same way the four ceiling materials
were. **Mixed, and the dominant instances do not confirm a warm reading**:
slots 354 and 382 (2,960 and 1,664 vertices - both larger than
`animhexlights`' own load-bearing 1,712) sit at 190-198° curved, the same
unexplained cool-blue family `base_diffusespecular`/`cf_diff_spec` occupy,
not the reference's 39-86° warm band; slot 178 agrees (199.4°). Slot 432
does not - 87.5°/102.1°/102.5° across raw/top-decile/curved, stable rather
than noisy, on 168 vertices - so it fails neither of the two reasons this
thread already discarded `lambert`'s reading for (too few vertices, a 75°
raw-vs-top-decile disagreement). It is a real warm-ish outlier, just
outnumbered 30 to 1 by the vertices that read cool. Full table and account:
[`renderer.md`](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md),
"`uvanim_diffuse_emissive`'s own colour is read, and its largest
populations do not confirm the warm reading". **This closes the "if its
colour reads warm" conditional in the negative for the dominant instances,
and leaves it unresolved for slot 432's own geometry** - on most of what
this material draws, it shares the ceiling materials' code-path defect
(established two sessions ago) but not, from vertex-colour-set data alone,
their warm hue; `EmissiveTexture`'s own sampled colour (a separate glow
layer this material also carries) is untouched and is the only place a
warm contribution from this material could still come from.

2026-09-18, later still: that last sentence is now checked, with a new
tool (`crates/render/examples/hd_amphiseum_emissive_texture.rs`,
committed). The glow is real - wired, decoded, and present in
`Model::emissive`'s built table on all five slots - and its colour is
`TINT`, not the (mostly achromatic) `EmissiveTexture` sample itself.
`TINT` is warm-orange on exactly the two slots (354, 432) whose
vertex-colour-set data already read cool, and cool-blue on 357/382, whose
vertex-colour-set data already read cool too - agreeing with, not
contradicting, the earlier reading there. **Estimated magnitude (`tint *
texture_mean * albedo_alpha`, not a pixel measurement) does not overturn
the dominant-cool conclusion**: 354/432's warm contribution is small
(~0.03 on the red channel, the lowest of the five because their own
albedo alpha is the lowest) against 357/382's cool contribution (~0.05-0.16
on green/blue, 3-6x larger) - so on the instances that carry the most
vertices, the glow nudges warm by less than it reinforces cool elsewhere.
Full account: [`renderer.md`](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md),
same section, "2026-09-18, later still". This is a magnitude estimate, not
a capture-verified pixel, so it narrows rather than closes the question -
but it is evidence against, not for, the glow layer being what would flip
this material's dominant instances warm.

2026-09-18, separate lane: the channel-order/byte-order candidate this
thread's own Next Steps named for `base_diffusespecular`/`cf_diff_spec` is
tested, with a real result. `Mesh::vertex_light`'s colour-set decode reads
`[r, g, b, mask]` in file order - an assumption this project's own code
makes, since `Attribute` carries no remap field the way `.gtf`'s texture
fetch does, not something read off the disc. Swapping R and B and
reconstructing the same way (`hd_amphiseum_ceiling_vertex_light.rs`'s own
top-decile-then-curve method) lands both materials inside the reference's
39-86° warm band at saturation 0.95 (`base_diffusespecular` 200.0° -> 39.7°
on 15,256 vertices; `cf_diff_spec` 201.6° -> 38.4° on 1,688) - and breaks
`animhexlights`' own already-correct 45.5° reading (saturation only 0.233)
down to 128.5° under the identical transformation, which is the negative
control the hypothesis has to survive. Two alternative explanations
(shader swizzle, wrong-attribute selection) were checked and ruled out -
see renderer.md for both. **Confidence 70**: strong comparative evidence
with a working control, but not a runtime or register-level read, which is
what would move it - a search for the RSX vertex-array-format register
builder (`Texture_BuildGcmRegisters`'s sibling for vertex fetch) came up
empty this session, so that read is still to do, not done. `Mesh::
vertex_light` is **not** changed - a global swap would break
`animhexlights`, and nothing found explains which chunks need which order,
so this is left as a disc-value finding, not a fix. Full account:
[`renderer.md`](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md),
"`base_diffusespecular`/`cf_diff_spec` stay cool because of an unverified
byte-order assumption in this project's own code, not because the disc's
own data is cool".

2026-09-18, later still, continued: **the `Debug.Enable EdgeGeom` half of the gate is read, and it names the third SPU binary - the one the per-vertex light math actually runs in.** Full account: [`renderer.md`](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md), "The compiled-ops vocabulary is mapped, opcode `0x30` builds an `EdgeGeom` SPURS job per chunk...". What lives only here:

The `Render_RunCompiledOps_q` jump table (`0x31` opcodes) is walked end to end and the state-setting half decompiled. `Scene_PrepareFrame` copies the **`EdgeGeom`** job binary (`0x007f6d80`, `0x107e0` bytes - Sony's `libedgegeom` with game code linked in, registered by `EdgeGeom_RegisterJob`) into the frame context at `+0x144`/`+0x148` and sets context flag `0x1000` from `Debug.Enable EdgeGeom`. Opcode `0x30` - the draw the compilers emit for kind-`5` (SPU-processed) geometry, the only kind that can carry `SpuVertexColours` - runs `RenderOps_BuildEdgeGeomJob`, which with that flag builds a `0x100`-byte `EdgeGeom` job record per chunk with **the light array EA and count at `+0xb8`/`+0xbc`** (straight from context `+0x14c`/`+0x150`, i.e. opcode `0x2d`'s operands) beside the sixteen attribute-slot bindings, and without it issues a plain RSX draw with no light data at all. `_opd_FUN_005d8700`'s `0x800` path turned out to be fragment-program constant patching (CPU-side into a scratch copy, then re-upload), not lighting - a dead end for this thread, closed.

Net: the light records reach the `EdgeGeom` SPU job as `(EA, count)` in its descriptor; the per-vertex computation and the `SpuVertexColours` write are inside `data/extracted/ps3/spu-jobs/edgegeom-0x007f6d80.dis` (16,862 lines, extracted, not read). Six more names landed. No code changed.

2026-09-18, later still, continued: **`EdgeGeom`'s light path is read end to end - the `Enable_spu_vertex_light` thread now has its formula.** Full account: [`renderer.md`](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md), "`EdgeGeom`'s light path is read: per vertex, per light within range of the chunk...". What lives only here:

The job DMAs `count * 32` bytes from opcode `0x2d`'s address (record `+0xb8`/`+0xbc` = user-data `+0x38`/`+0x3c`, the hedge from the previous entry closed), culls lights against the chunk's bounds (rewriting `D` to `1/D`), then per vertex and per light accumulates `max(0, 1 - |d|/D)^w * max(0, N.L) * (r, g, b)` and RGBE-packs the sum (exponent `k + 129` for `m = 1.f * 2^k`, i.e. Radiance's frexp bias - the shader's `255`/`128` decode inverts it exactly, see the next entry) into the `SpuVertexColours` stream, found by searching the attribute bindings for `0x868f8229`. Record = `(x, y, z, w = falloff exponent, r, g, b, D = range)`; every live record had `w = 1` and `D = 0.6`-`2.0`, so these are one-to-two-unit glows under objects, not track lights. Confidence 80 for the layout and the `w = 1` formula, 70 for `w` as an authored exponent (that path never observed live); hand disassembly, three stages cross-checked. Nothing wired: ~~the `SVC1` program's combine of the decoded value is a microcode read still to do~~ (read in the next entry), and whether a glow that small is worth per-chunk plumbing in `mesh.wgsl` is a call for whoever picks this up.

2026-09-18, later still, continued: **the `SVC1` combine is read, and the exponent-bias flag is withdrawn - the RGBE round trip is exact to `256/255`.** Full account: [`renderer.md`](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md), "The `SVC1` combine is read: the decoded `SpuVertexColours` term is added to the diffuse irradiance before the albedo multiply...". What lives only here:

The packer's `k + 129` is Radiance's own bias (frexp exponent + 128); the previous entry compared it to an IEEE exponent. Multiplied through the shader's `rgb * 2^(a*255 - 128)` on hardware-normalised bytes, the decode returns `rgb * 256/255` - the packer's `1/255` and the shader's `255` are a matched pair. `SVC0`/`SVC1` twins share the fragment program on 30 of 30 pairs in `track_surface`, so the variant is a vertex-program substitution, and the shared fragment program adds `f[TC0]` (or `f[TC1]`, family-dependent) into `ambient + sun * N.L (+ lightmap)` **before** the albedo multiply - never into the specular, the fog or the post-fog emissive add. Uniform disc-wide on the vertex side (27,312 of 27,312 `SVC1` blocks decode identically); the combine read on five fragment programs across four families and both archive compiles, plus one emissive material where the term is decoded and discarded. Two new `scripts/ps3-sho.py` commands (`variants`, `svc-twins`) re-derive the numbers. For `mesh.wgsl` that is one additive term into the existing pre-albedo light sum, no new interpolant or pass. No code changed.

## Open

- **Where to start on Amphiseum's 154-degree hue gap: it is near-complementary, which is a structural signature, not a lighting one.** Recorded 2026-09-17 by the coordinating session as a *lead only*. Candidate (1) (channel/byte order) and candidate (2) (per-region attribution) were superseded by the per-material read below rather than run directly. **Candidate (3), per-material attribution, is now done and named the wrong operation** - see the `lane-hd-ceiling` dated entry above and renderer.md's new section: `mesh.wgsl` adds `constantAmbientColour` to four ceiling materials whose own resolved programs never reference it, and the disc's actual replacement term (a curve over the baked vertex colour set) reconstructs to the reference's own warm-band *hue* on one of the four (`animhexlights`) from disc values alone, at low saturation and unchecked against the reference's own saturation-weighted reading. Candidates (1)/(2) are not thereby refuted, only unnecessary to explain that one material - `base_diffusespecular`/`cf_diff_spec` staying cool-blue at every stage of that reconstruction is still unexplained, and a channel-order or byte-order defect specific to those two remains a live, unchecked candidate for their share of the gap. The capture this session identified the four materials against (`amphiseum-matched` pose `00`) reads 180-220°/200-205° uniformly across its top row - no warm cells at all in pose `00` specifically (a first pass misread pose `01`'s tile grid as pose `00`'s and drew a since-retracted conclusion from it, see the dated entry above). A fifth material, `uvanim_diffuse_emissive`, is independently confirmed by its own microcode to share the same no-`constantAmbientColour` defect, but not tied to any verified warm pixel this session. A fresh dome-facing capture (blocked on the RPCS3 menu-nav fix in Next Steps) is still what would settle coverage and whether any of these five materials' own colour is what the reference's warm reading traces to.
- **A corrective hue rotation is not an acceptable fix for this and should be refused if proposed.** Fitting a rotation to make the frame match the reference is tuning to a capture, which is what ["never invent what the assets already author"](../../CLAUDE.md) exists to prevent, and a gap this large is exactly the kind that tempts it. A fix counts only if it names the wrong operation and corrects that.
- **Retired 2026-10-09 (`hd-dark-ref`): there is no frame-wide darkness.** Every reading below was taken against captures with `Write Color Buffers` off; with it on, ours is within -5 % to +7 % of the original at the grid on Metropia, Talon's, Sol 2 and Amphiseum (dated entry at the end). Kept for its history: **The frame is measurably darker than the original at a matched camera, by 0.13-0.24 mean luminance on Talon's Junction and Sol 2, and the exposure/bloom post chain is ruled out as the cause** - open at `lane-ship-hull`'s files (`mesh.wgsl`, `crates/mesh/src/mesh/`, `sky_cube.rs`), not this lane's; **narrowed 2026-09-13 (twice)**: first, not a single offset/scale/curve (quantile-quantile evidence, confidence 80), and `.envsettings` plumbing is confirmed exact for Talon's Junction's core terms; second, **the gap is not a single global magnitude either** - Amphiseum measures the opposite sign (ours brighter by 0.04-0.19 luma), so what remains is a per-material or per-circuit-lighting-data cause, not a shared constant. Amphiseum's own black-wall-panel defect is **fixed** (below, `lane-amphiseum-walls`) and was a distinct bug (a wrong sampler-entry pick, not a shading gap); the panel's residual colour still reads flatter/greyer than the reference's cyan-white, in the same direction as this broader per-material gap, and is left to whoever picks that up next. **Re-derived at a clean 0 km/h grid pose 2026-09-17 (`lane-hd-track-lighting`, dated entry above): the whole-frame "opposite sign" reading does not survive a bloom-on/off check** - it is +0.045 with bloom on and +0.008 (essentially matched) with bloom off, inside the chain's own known 0.02-0.05 sensitivity on the one circuit where that chain builds at all. **Same day, later (`lane-hd-amphiseum-hue`): the reason the whole-frame number was never going to be stable is that it isn't one gap - a `--tiles 4x6` box-free breakdown shows the ceiling/stands half of the frame reading brighter (-0.121) and the floor/barrier half reading darker (+0.100), a 0.22-luma swing from vertical framing alone, present with bloom on or off.** The maintainer's own answer, asked directly, confirms this shape: "Both, and it varies by area." What survives every check so far is a 154-155 degree hue shift Talon's Junction's gap (9-13 degrees, hue intact, close to uniform across a tile grid) does not have, and a genuine top-warm/bottom-cool colour split in the reference that this project's render does not reproduce (stays in one blue-violet hue family everywhere). The two circuits read as different defects, on tile-grid evidence now, not just an aggregate difference.
- Whether the 8-bit surfaces hold linear or gamma light is undetermined (confidence 70 against ADR-0026, not acted on) - still open, unchanged by the above
- Texture-decode brightness (candidate (c)) is not pixel-verified against a reference decoder - only that `oag-texture`'s `.gtf` path carries no gamma/sRGB logic of its own and its ground-truth test passes
- What the colour set's fourth byte gates is unknown - `0x1aaf7631` is declared by no `SHO` shader block, so the only place left to check is per-material microcode
- `shadowMapTex` is projected by the disc and unimplemented here
- `scripts/clipped-white.py`'s docstring still publishes the 3.85/6.99 pair, and 3.85 % is a padded canvas rather than a frame - another lane owns that file, so the correction lives in `scripts/hd-glow-sweep.py` and renderer.md instead
- ~~Why **eleven** of the sixteen circuits measure identically with `[graphics] bloom` on and off~~ **answered 2026-09-13 (`lane-hd-resolve-fill`)**: those eleven are exactly the circuits whose `.envsettings` does not author the whole `Tone` family (`zone_2`/`zone_3`/`zone_4` ship no file at all; the eight Fury/DLC circuits omit `Tone darkening clamp`/`Tone maximum brightness`), so `envsettings_bloom` returns `None` and `oag_post::hd_bloom::Chain` is never built - the switch has nothing to toggle. See the dated entry above and renderer.md's new section. "Six of those eleven do take the authored [lit-material] path" still stands and is a separate finding - the material lighting and the missing exposure chain are two independent absences that happen to overlap on the same circuit set
- Fury's own circuits and the DLC packs still have no rpcs3 grab to be read against at all (Amphiseum now does - see 2026-09-13 above)
- ~~Amphiseum's grid-start trackside wall panels draw unlit/flat-black~~ **fixed 2026-09-13 (`lane-amphiseum-walls`)**: a wrong sampler-entry pick (`EmissiveTexture` bound as the picture instead of the material's own `Texture1`), not the `etched_glass_tech` shading gap this bullet guessed - see the dated entry above and `rcsmaterial.md`. The panel's residual colour (flatter/greyer than the reference's cyan-white) is left open, folded into the per-material darkness-gap bullet above rather than tracked separately.
- ~~what makes Amphiseum's opening seconds already 406-429 km/h at the same capture settings that gave Talon's Junction and Sol 2 a slow (70-74 km/h) grid pose is unread~~ **answered 2026-09-17 (`lane-hd-track-lighting`)**: neither a downhill start nor a shorter countdown - `cmd_capture` holds thrust for `--interval` seconds before every shot including the first, so shot 00's speed is an artefact of where the fixed `--load`+`--interval` window happens to land relative to `GO`, not a circuit property. A shorter `--load`/`--interval` isolates a true 0 km/h grid pose on both circuits; see the dated entry above for the resulting sign-flip re-check
- `scripts/hd-material-probe.py`'s fog-segment classifier assumes the frame's exposure scalar is `1.0`, calibrated by eye against one unclipped near-camera channel rather than solved - a session that wants coverage past 89% or tighter per-slot deltas needs to actually solve for it (or read `k` off `.envsettings`' `Tone` family and the adapted-luminance state directly) rather than assume it
- ~~`envsettings_bloom` (and possibly `envsettings_fog`/`envsettings_light`) should seed from the title's own front-end `.envsettings` for any `HDR and Bloom` key a circuit's file omits~~ **done 2026-09-13 (`lane-envsettings-carry`)**: `crates/raceplay/src/load/environment.rs::staged_envsettings` carries `/data/fe/fe.track.envsettings` forward for exactly the keys a circuit's own file does not declare, and `hd_envsettings_carry_ground_truth.rs` now measures 16 of 16 environments building the chain (was 5 of 16). `envsettings_fog`/`envsettings_light` were checked for the same partial-key gap and do not have it - every circuit that ships a `.envsettings` authors a complete `Fog`/`Lighting` block on its own, so they were deliberately left un-carried; see renderer.md's new section for the full account, including a correction to this thread's own prior "both Fury front-end files carry `20/3/4`" reading (DATA02's own does not). **Whether wiring it up closes Sol 2's own darkness/under-clipping gap is now measured**: `hd-frame-compare.py`'s pose-00 numbers move toward the reference on every region but road surface (13-24% of the per-region clip-share gap), which corroborates the gate-cancellation hypothesis below rather than confirming it outright - the chain is real and doing something small, not nothing, and what is left open is everything this thread's first bullet already names
- **`Enable_spu_vertex_light`, current state (2026-09-18, second close-out) - read this bullet, not the dated log, for where the thread actually stands.** The PPU side is closed end to end, all decompiled (confidence 85-88): fifteen producers push `(position, w, colour.rgb, radius)` candidates into a 128-slot list; `SpuLight_CompactVisibleCandidates` (`0x0040d728`, every frame from `Scene_PrepareFrame`) frustum-culls them into the double buffer at `iRam008b83b0+0x80+index*0x1000` and writes the survivor count to `+0x2084[index]` - the companion word, closed, no longer a hypothesis; the track-visibility pass hands the survivors plus the track's per-chunk bounding spheres to the `LightCulling` SPU job (`0x007f6880`, `0x490` bytes, read in full at confidence 75), which marks every chunk any light's sphere touches in the `0x1000`-byte bit table at `0x00f4a280`; `FUN_004074e0`/`FUN_00408fa8` emit opcode `0x2d` `(slot address, count)` and select the `| 0x800` = `SVC1` = `SpuVertexColours` variant for exactly those chunks. **What is retracted this time**: `0x00848780`/`0x2d00` is the SPURS job-queue policy module shared by nine queues, not the render-ops job - every SPU-side finding about it (the `GETLLAR`/`PUTLLC` decrement, the `$80`/`$81` provenance, the job-type switch) is scheduler-runtime behaviour, correct as read but off this thread's path; the previous close-out's "genuinely blocked" verdict was blocked on the wrong binary. The real render-ops job is `Live_RcsBlackbox`, `0x0084b700`, `0xd690` bytes, which receives the compiled-ops stream via the ring copy of the `0x160`-byte context and must, by the `SVC1` variant's declared input, write the `SpuVertexColours` stream. **The computation is read** (later the same day): `EdgeGeom` (`0x007f6d80`) receives the light array `(EA, count)` at descriptor `+0xb8`/`+0xbc` from `RenderOps_BuildEdgeGeomJob` (opcode `0x30`), culls lights against the chunk's bounds, and per vertex accumulates `max(0, 1 - |d|/D)^w * max(0, N.L) * colour` over the lights, RGBE-packed into `SpuVertexColours` (confidence 80, hand disassembly). Record = `(x, y, z, w = exponent, r, g, b, D = range)`; live `D` is `0.6`-`2.0` units. **The `SVC1` combine is read too** (later the same day, confidence 88): `SVC0`/`SVC1` twins share the fragment program, the `SVC1` vertex program decodes `rgb * 2^(a*255 - 128)` into an interpolant the shared fragment program adds to `ambient + sun * N.L (+ lightmap)` *before* the albedo multiply, and the packer's `k + 129` round-trips through that decode to `256/255` exactly - the exponent-bias flag is withdrawn. **One of the fifteen producers is read too** (2026-09-20, confidence 88 on the arithmetic, unconfirmed on identity): `FUN_000cfb80` submits `D = clamp(obj+0x6a7c, floor 0) * 133.332`, `w = 1.0` exact (matching every live record), position from a transform lookup, colour a shared orange-ish literal `(100, 50, 50)`. Separately, `Ship_LoadEngineLightData` (`0x000d25e0`) is found and reads `EngineLightData.xml`'s `Distance`/`Radius` into `ship+0xfc`/`+0x100` - 36 ships/skins, `Radius` `0.7`-`2.0`, matching the live `D` span almost exactly. **The two are not the same mechanism - closed, not just flagged, this same day.** `obj+0x6adc` is `Ship_SyncActiveModelPointer` (`0x000db090`) synced from `obj+0x6ae0`, the ship's own currently-loaded `ship.vex` (`Ship_ReloadModelForSkin`, `0x000dbdc8`, which also loads `shipwreck.vex`/`ship_lod.vex`/`ship_lod1.vex` into the three sibling fields) - so the light's position is the ship's own model transform, not a distinct engine-flare node; the `WARNING: SHIP MISSING ENGINE FLARE IMPORT NODE` string gates a different system entirely. `obj+0x6a7c` is a real depletable resource: exactly one writer (`FUN_000e3768`, a per-frame vtable slot alongside `FUN_000e41b0`) decrements it and a twin field by an externally-supplied amount and accumulates the total spent - not a static per-ship value, so `EngineLightData.xml`'s `Radius` correlation has no code path behind it after three sessions of tracing. See renderer.md's newest section for the full read. **What is left before anything is wired**: name the other fourteen producers (now that this one is fully traced as an independent glow, not an `EngineLightData.xml` consumer), **advanced to a dead end worth recording, 2026-09-20, later still**: the worth question is moot - the user's own call is that a faithful reproduction wires this regardless. All thirteen producers reachable through `SpuLight_AddCandidate` were catalogued and their `D`/colour values read (renderer.md's newest two sections). **None matches the live-captured buffer**: every fixed-`D` producer is `10`-`100`, the captured range is `0.6`-`2.0` (5-166x off), and every fixed-colour literal has the wrong ratio against the captured `1:0.25:0.1` (varying `40`-`440` in magnitude, so computed, not literal). Thirteen is now confirmed exhaustive (`get_xrefs_to` on `SpuLight_AddCandidate` itself, not just the thunk - only one call path exists). **This is not unfinished search, it is a negative result**: the 128-slot PPU candidate list these thirteen build most likely is not the same data as the captured 8-slot SPU buffer. Wiring any of the thirteen would mean inventing the unread fields behind them or wiring a value already shown not to match reality - so nothing is wired. What would unblock it: find where the 8-slot buffer is actually written, if not from this candidate list. `Live_RcsBlackbox` is only the Zone-Stage family's SPU-side ops interpreter, presumed to build the same job record; not read. Full history, including all four retractions, is in `renderer.md`'s dated sections from "The twelve unexamined call sites are read..." onward. **Resolved, 2026-09-20, static lane (confidence 85)**: the negative result above is retracted - its "thirteen is exhaustive" premise was Ghidra's view, not the binary's. A byte scan for every `bl` to the thunk finds **33 call sites in 24 functions**; the 18 Ghidra missed all sit past an `lvlx` (the documented Cell-instruction hole, `docs/reverse-engineering/toolchain.md`). One of them is the producer: `EngineFlare_SubmitSpuLight` (`0x0029ff28`, called once per ship per frame from `EngineFlare_Enqueue`) submits position `flare_node.pos - flare_node.z * EngineLightData.Distance + U(-0.1,0.1)`, `D = EngineLightData.Radius + U(-0.1,0.1)`, `w = 1.0`, colour `(40,10,4)` (Fury skin byte `ship+0x7d2c`) or `(4,10,40)` (classic) times `1 + 10 * boost_blend` (`+0x144 + +0x150`). Every captured record matches it: `w = 1.0`, `1:0.25:0.1` is the literal's own ratio, `40`-`440` is `blend` `0`-`1`, `D` `0.62`-`2.05` is `Radius` `0.7`-`2.0` `+-0.1`, the `(80,10,0)` outliers are its `t >= 1` transition branch. `SpuLight_AddCandidate` and `SpuLight_CompactVisibleCandidates` were both re-read and store/copy verbatim, so the buffer is the candidate list exactly as the confidence-88 trace said. `EngineLightData.xml`'s `Radius` correlation had a code path all along - on the flare object, via its ship back-pointer. Full read in renderer.md's "The captured buffer's producer is found" section. **Live lane, 2026-09-20, later still: the "negative result" above is overturned for at least one producer.** A same-instant paused read of both the candidate list and the compacted visible buffer (`scripts/rpcs3-spu-light-candidates.py`, new; `data/traces/hd-spu-light-candidates/`) found byte-for-byte verbatim copies between them: a `D=10.0`, colour `(2.0,2.0,0.4)` record - matching this page's own `FUN_001310e8` row exactly on `D`, and giving that producer's previously-unread fixed colour global its first live value - appears identically in both structures, confirmed at the raw `.bin` byte level, across two of five snapshots and both buffer slots. The candidate list *is* the buffer's source, unmodified, for at least this producer; it is a second, distinct light family the earlier five-snapshot capture never happened to catch (absent when that capture's `Enable_spu_vertex_light` sample was taken, present here), not a correction to the orange family's own numbers. The dominant orange family (`1:0.25:0.1`, `D` `0.6`-`2.0`) is present in every snapshot's candidate list at closely matching ranges (`40`-`440` magnitude, `D` `0.901`-`2.003`, both consistent with the figures already in this bullet) but was not caught byte-exact this session - read as ordinary tick-boundary timing (this family's fields are recomputed every call, unlike the fixed `D=10` family), not a second negative result. Full account: renderer.md's new "2026-09-20, live lane" section. **Live lane, 2026-09-20, later still: the orange family's true call site is found, and it is not one of the thirteen.** A live breakpoint directly on `SpuLight_AddCandidate`'s own entry (`0x0040d990`, interpreter boot, `scripts/rpcs3-spu-light-candidates.py attribute`) caught 40 calls, 0 misses, every one confirmed register-exact: all 40 share the identical return address `0x2a019c` (call at `0x2a0198`), all 40 are the orange family (`f1`/`D` and `f2`/`w` match the freshly-written record exactly every time; colour ratio `1:0.25:0.1` on all 40; `D` `0.626`-`2.065`). `0x2a019c` is `1,354,804` bytes past the nearest of the thirteen catalogued sites (`FUN_00155568`) - far outside any named function's size on this page - and sits `1,804` bytes before the already-known `EngineFlare_RenderTick` (`0x002a08a8`). **This means "thirteen is exhaustive" (the previous static session's own two-hit `get_xrefs_to` result) missed a real, live-confirmed caller** - either a function Ghidra's static analysis has not bounded at `0x2a0198`, or a second copy of the tail-call thunk pattern elsewhere. Full account, including a first attempt that hung on a documented `drain()` trap and the diagnosis that fixed it: renderer.md's new "2026-09-20, live lane, continued" section. **Reconciled at merge, 2026-09-20 (coordinator):** the live lane's `0x2a0198` is the `bl` inside `EngineFlare_SubmitSpuLight` (`0x0029ff28`) that the static lane named the same afternoon from a byte scan - the two lanes converged on the same producer independently (static: hand disassembly + capture cross-check; live: 40/40 breakpoint hits with `LR = 0x2a019c`), so the orange family is attributed and named, not merely located. The `D = 10.0, (2, 2, 0.4)` family is the one remaining unnamed producer (`FUN_001310e8` site 1 on `D`/`w`/shape, confidence 75). **Wired, 2026-09-20, implementation lane (`lane/hd-engine-light`)**: `oag_tables::enginelight` reads `EngineLightData.xml` (37 files on the disc, not 36 - all twenty-four `_c1`/`_n1` variants ship one; `enginelight_ground_truth.rs` asserts the count and the `-0.4..1.5`/`0.7..2.0` spans), `oag_livery::engine_light` pairs it with the `Engine Flare` locator's Z axis, `oag_raceplay::engine_light` builds `EngineFlare_SubmitSpuLight`'s record per craft per frame (anchor `model * (nozzle - z * Distance)`, `(40,10,4)`/`(4,10,40)` by the Fury flag times `1 + 10 * boost_blend`, `w = 1`, `D = Radius + U(-0.1, 0.1)` off its own `oag_core::Rng` stream), and `mesh.wgsl` sums the `EdgeGeom` formula per vertex from a 128-record `SpuLights` block in the `Scene` uniform, added pre-albedo, times the `256/255` round trip. `"Lighting.Enable spu vertex lights"` is honoured (default on; `02_track`/`12_sol_2` author `0`). **The finding worth carrying: bound to the track chunks alone it changed zero pixels, and the numbers say it never can at ride height** - every one of the 40 captured records sits 2.98-4.41 units (mean 4.03) from the nearest collision triangle, ours 2.85-4.53 (mean 4.09) on the same circuit, against `D` 0.62-2.05 (`crates/game/examples/hd_engine_light_reach_probe.rs`). **That circuit is Talon's Junction, not Amphiseum** as this page's capture entries say: scored against all sixteen circuits' collision soups (`hd_engine_light_which_circuit.rs`), Talon's puts 40 of 40 records a ride height off a surface and Amphiseum 3 of 40 within 5 units. So the list is bound to the hulls too - 74 ship materials compile `SVC1` twins (`ps3-sho.py svc-twins <image> materials/ships`), and the Fury engine housings take the warm tint the reference shows, the whole rear washing warm on boost (`talons-t487.png`/`talons-t470.png` under a scratch directory, not kept) - **chosen, not measured**, on static evidence; the runtime `SVC1` bit on a hull chunk is unobserved and is the one RPCS3 read that would settle it. The blue-to-orange transition branch (`+0x240`/`+0x2b4`) and the other 23 producers stay unwired.

## Next Steps

- **(2026-10-09, `hd-hull-bright`) Hull lighting, mostly closed:** the ~3x was `VertexColour1` drawn as an added baked light where the hull programs `MUL` it into `ambient + sun * N.L`, plus a stand-in specular on `diffuse_vcol` (no `LG2`); fixed for hulls only by `mesh::rcs::skin::light_inputs` (docs/rendering/hd-ship-materials.md, "Finding 3, resolved"). Housing box 0.297 -> 0.205 (`--shadows off`), 0.196 -> 0.110 (`--shadows original`, HD's default) against the original's 0.087-0.091. **Open:** the 0.02 residual; the orange rim (unbound unit-2 environment lookup, and which of the `Ambient`/`IBL`/`ShadowMap` variants the original binds); the rest-pose upright pylons; whether the track programs multiply `VertexColour1` too (Vineta K, Chenghou, Amphiseum match the rule and are left additive until measured).
- **(2026-10-09, `hd-dark-ref`) Metropia lamps slightly over:** ceiling clipped share ours 0.51 % (grid) / 2.72 % (throttle) against 0.03-0.15 % / 1.02-1.03 %; small, unattributed. Amphiseum's dark interior pose reads ours 1.21x.
- **Compare frames only against the `*-wcb-on-*` references** in `data/reference/hd-capture/` (rpcs3-capture.md, "A screenshot's brightness depends on `Write Color Buffers`"). Still to re-shoot with the option on: Vineta K for `hd_behind_glass_frame`'s teal window.
- ~~Read `Live_RcsBlackbox` for what it does with opcode `0x2d`~~ **re-targeted 2026-09-18, later still**: the light math is in the `EdgeGeom` job, not `Live_RcsBlackbox` - see the dated entry above. ~~Read `EdgeGeom` for the light loop~~ **done, later the same day** - see the dated entry above and renderer.md's "`EdgeGeom`'s light path is read" section. **What replaces it**: ~~(1) read the `SVC1` vertex program's combine of the decoded `SpuVertexColours` term, checking the exponent bias first~~ **done, later the same day** - pre-albedo additive into the diffuse sum, round trip exact to `256/255`, no factor of two; see renderer.md's "The `SVC1` combine is read" section; **(2) done, 2026-09-20**: one of the fifteen callers (`FUN_000cfb80`) is read in full and both its inputs traced to their setters. `obj+0x6adc` is `Ship_SyncActiveModelPointer` (`0x000db090`), synced from `obj+0x6ae0` = the ship's own currently-loaded `ship.vex` (`Ship_ReloadModelForSkin`, `0x000dbdc8`) - so the light sits at the ship's own model transform, not a distinct engine-flare node (that hypothesis is retracted). `obj+0x6a7c` has exactly one writer, `FUN_000e3768` (a per-frame vtable slot alongside `FUN_000e41b0`), which decrements it and a twin field by an externally-supplied amount - a real depletable resource, not `Radius`; what it represents (absorb/boost/something else) is not identified, and `FUN_000e3768` is deliberately left unnamed rather than guessed. `EngineLightData.xml`'s `Radius` correlation with live `D` has no code path behind it after three sessions - treat it as coincidental scale, not a lead, unless new evidence turns up. **Done, 2026-09-20, later still**: all thirteen `SpuLight_AddCandidate` callers are catalogued (renderer.md's "All thirteen reachable `SpuLight_AddCandidate` call sites" section) and their `D`/colour cross-checked against the live capture (renderer.md's "Thirteen is exhaustive, and the numbers rule out every one of them" section) - **none matches**, and thirteen is now confirmed exhaustive, not partial. **What replaces it**: this is no longer a producer-naming task. Find where the live-captured 8-slot buffer is actually written, since it is probably not these thirteen: (a) check for an SPU-side write that bypasses the PPU candidate list this page has been tracing (the `LightCulling` job writes a bit table, not light data itself, but something upstream of it was not re-checked for a second write path); (b) reconsider whether the 8-slot buffer might be authored/static per-track rather than gameplay-computed at all - not tested this session; (c) if a genuine fourteenth producer exists outside static reach (a true indirect/virtual call `get_xrefs_to` cannot see), a live RPCS3 breakpoint on `SpuLight_AddCandidate` during a 5-snapshot-matching capture would catch it directly rather than more static search. Only once the 8-slot buffer's real source is found does wiring `mesh.wgsl` become possible without inventing a value - the shader half is still just one additive term into the existing pre-albedo light sum, unchanged from before. Sony's `libedgegeom` is open-source-documented in the PS3 SDK samples, so the library half of the job (decompress/cull/compress, with its `EDGE error:` strings) can be recognised and skipped; the game's custom vertex callback is the part to read. The original bullet's anchors still apply to `Live_RcsBlackbox` if anyone wants the Zone-Stage interpreter itself:  - the last unread step of `Enable_spu_vertex_light`, and the one that decides what `mesh.wgsl` would add. Static, no new tooling; `0xd690` bytes is a real hand-disassembly budget, comparable to `engine-trail.md`'s `Trails` effort. Two cheap anchors before reading linearly: the job's user data word `0` is the context copy, so the code that DMAs `0x160` bytes and then reads `+0xd8`/`+0xdc` from it is the ops-stream fetch; and the stream's opcode words are 4-byte big-endian ints, so the interpreter's dispatch (table or compare chain) is findable from the `0x2d` handler's two-operand read. Also unread and PPU-side, cheaper: `_opd_FUN_005d8700`'s `0x800`-gated scratch-buffer copy loop (the `RigidBody`/`Absorb` family's own path), which is either a PPU fallback for the same per-vertex computation or a light-constant upload - either answer constrains what the SPU job must do. **Superseded, 2026-09-20, static lane**: (a)/(b)/(c) are moot - the source is `EngineFlare_SubmitSpuLight` (`0x0029ff28`), found by byte-scanning the ELF for `bl` sites rather than trusting `get_xrefs_to` (which misses 18 of 33 because of the `lvlx` hole). What replaces them: wire the `mesh.wgsl` term from the now-complete specification in renderer.md's newest section - per ship one light at the flare node offset back by `Distance`, range `Radius` (both already parsed from `EngineLightData.xml` per `Ship_LoadEngineLightData`), exponent `1.0`, colour `(40,10,4)`/`(4,10,40)` by skin times `1 + 10 * boost_blend`, the `+-0.1` jitter optional. Two small unreads that do not block wiring: what arms the flare's `+0x240` transition countdown (the `(80,10,0)` tint), and which producer wrote slot 1's single `D = 10.0, (2, 2, 0.4)` record (`FUN_001310e8`'s first site has `D = 10`, `w = 1` and a same-shape static colour `(0.3, 0.3, 0.2)`). For any future "callers are exhaustive" claim on this binary: byte-scan, do not xref. **Live lane, 2026-09-20, later still**: option (c) above - "a live RPCS3 breakpoint on `SpuLight_AddCandidate` during a 5-snapshot-matching capture would catch it directly" - is done, without needing a breakpoint at all: a same-instant paused read of the candidate list alongside the compacted visible buffer (`scripts/rpcs3-spu-light-candidates.py`) found byte-exact matches between them for a `D=10.0`/`(2.0,2.0,0.4)` light family, confirming the candidate list *is* the buffer's real source for at least that producer (renderer.md's new "2026-09-20, live lane" section). **Done, 2026-09-20, later still**: the follow-up live breakpoint on `SpuLight_AddCandidate` (`0x0040d990`) itself was also run (40 hits, 0 misses) and identifies the orange family's own call site directly: `LR=0x2a019c` on all 40 hits, a location outside every one of the thirteen catalogued sites and 1,804 bytes before `EngineFlare_RenderTick` (`0x002a08a8`) - see renderer.md's new "2026-09-20, live lane, continued" section. **What replaces this bullet**: the orange family is no longer unattributed-to-a-region, but it is still unnamed - the concrete next step is now static, not live: decompile whatever function Ghidra has (or has not) bounded at `0x2a0198` (the `bl` instruction is at `0x2a0198`, four bytes before the confirmed `LR`) and read its own callers, which settles both "is this really outside the thirteen" and the orange family's full identity. This lane deliberately did not use `ghidra-mcp` to chase that further - it belongs to whoever picks this up with Ghidra access. **Reconciled at merge, 2026-09-20 (coordinator):** the static step this paragraph asks for was done in parallel - `0x2a0198` is inside `EngineFlare_SubmitSpuLight` (`0x0029ff28`, `names.tsv`), so the orange family is named; what remains is implementation (in flight on `lane/hd-engine-light`), the `+0x240` transition arming, and the `D = 10` family's identity. **Implementation landed 2026-09-20 (`lane/hd-engine-light`, see the `## Open` bullet's dated paragraph)**; what is left on this entry is one live read: does `LightCulling` set the `SVC1` bit for a *hull* chunk (the `0x1000`-byte table at `0x00f4a280` is per track chunk - a craft's `RigidBody` chunks may be marked by a different path or not at all)? If not, `race::scene::frame` should hand the craft `SpuLights::none()` and the engine light becomes invisible at ride height by the disc's own numbers, which is a fact to record rather than a bug to fix. Also worth one look while the emulator is up: whether a craft's own hull lights up on boost in the original - the `1 + 10 * blend` gain on `(40,10,4)` is a 440-unit HDR light a hand's breadth from the housing. **Reported from play, 2026-09-20 (user), after the light landed: the bloom around the craft is exaggerated** - the hull washes white at the boost snap where the original's housings stay copper. Its own thread: [hds-engine-light-washes-the-hull-on-boost.md](hds-engine-light-washes-the-hull-on-boost.md) (the hull binding, chosen not measured, is the top candidate; the bloom is measured and is not to be tuned).
- `rpcs3-spu-light-dump.py`'s `record_count` (stop at first all-zero record) is a stale high-water mark, not a count - bound any future read of the slot by `+0x2084[index]` (the script already reads it as `companion_value_u32`). Not fixed this session; a two-line change when the script is next run.
- Secondary, off this thread's path: the `0x280`-byte SPU program at `0x0084b480` (between the policy module and `Live_RcsBlackbox`) is unidentified, and the policy module's `GETLLAR`/`PUTLLC` counter has a PPU twin (`SpuJob_ReleaseNamedBinary`, `0x00469340`) worth one check if anyone returns to it.
- ~~decompile whatever interprets the `local_190`/`local_1a0` command-stream opcodes to learn what GPU state opcode `0x2d` writes~~ **advanced twice, 2026-09-18, later still** - see renderer.md's two new sections after "Opcode `0x2d`'s PPU-side handler is read directly." (1) Opcode `0x2d`'s PPU interpreter stores its `(address, value)` operand pair into context offsets `+0x14c`/`+0x150` - confirming the `RigidBody`/`Absorb` family's direct field-store and the opcode-stream families are the same mechanism. (2) Following opcode `0x1c`'s own handler chain into `_opd_FUN_005d8700` (the real GCM constant/vertex-data upload function): the same `0x800` bit `Shader_GetVariantHash` uses is read there too, gating an entirely different, much larger code path (a scratch-buffer allocation and an unrolled block-copy loop) versus a short two-call path when clear - real, decompiled, GPU-upload-level behaviour tied to the gate, not merely a shader-variant label. **`_opd_FUN_005d8700` is reached only from the `RigidBody`/`Absorb` family's PPU-interpreted opcode stream, not the Zone-Stage family's `FUN_005fc728`-submitted one - and `FUN_005fc728` is now confirmed to be a genuine SPU job dispatch, not CPU-side GCM submission**, see renderer.md's new section "`FUN_005fc728` is confirmed as a genuine SPU job dispatch...": its own call chain (`FUN_005fc728` -> `FUN_004688a0` -> `FUN_004664e8`) ends in `cellSpursReadyCountStore`, a real, named Cell SDK function that wakes an idle SPU to dequeue queued work - three hops, no inference. **Static PPU reading has been pushed as far as it goes on the Zone-Stage family's own consumer path** - what SPU program actually executes it, and what it does with opcode `0x2d`'s `(address, value)` pair, needs SPU disassembly. **The tooling half of that is now unblocked** - see renderer.md's new section "SPU disassembly is now unblocked, tooling-wise": `ps3-spu-binutils` is installed on this machine (`/opt/ps3dev/spu/bin/spu-objdump`), and `scripts/ps3-spu-disasm.py` (committed) validates cleanly against both this project's own known SPU jobs (`Trails`, `WakeTrail`), decoding real, coherent SPU code. ~~finding this thread's own target job's bytes~~ **found, 2026-09-18, later still** - see renderer.md's new section "The job's own executable binary is located, statically, and confirmed real code": `FUN_005fc6c8` (the render-ops queue's own one-time init, decompiled directly - it writes `PTR_DAT_008bf6b8 + 0x12084` and calls `FUN_00466548`, a generic `cellSpursAddWorkloadWithAttribute` wrapper) registers the SPU program at `0x00848780`, size `0x2d00` - a real, static file offset (`PTR_DAT_008bf6cc` two entries up was a false lead, corrected in the same section). Extracted and disassembled with `scripts/ps3-spu-disasm.py 0x00848780 0x2d00`: real code (stack setup, DMA-library `brasl` calls near the top of local store), a job-type dispatch (`ceqi $2,$81,1/2/3`), and - the structurally interesting part - ~~a four-slot DMA-fetch loop reading job-descriptor fields by the same `0`-`3` index scheme `FUN_005fc728`'s own PPU-side builder writes~~ **retracted, 2026-09-18, later still - there is no DMA-fetch loop there; see the dated entry below and renderer.md's two new sections.** ~~**Now the highest-value next step**: trace what the SPU program does with field `2`'s DMA'd-in data specifically~~ **done, 2026-09-18, later still**: it is an atomic reference-count decrement (`MFC_GETLLAR`/`MFC_PUTLLC`), not vertex math - see the dated entry below. **This bullet is now stale past this point - the "job-descriptor field" framing it and the entries below it were building on is itself retracted.** For the thread's actual current state (what survives, what's retracted, what's genuinely blocked vs. just unattempted), read the `## Open` section's own `Enable_spu_vertex_light` bullet rather than continuing to read this one forward. The copy loop inside `_opd_FUN_005d8700` (the `RigidBody`/`Absorb` family's own, PPU-side path) is still unread too and remains a cheaper, no-new-tooling alternative.
- ~~Fifteen call sites reach the newly-found producer, `FUN_0040d990` (renderer.md's new section), through a one-instruction thunk `FUN_006778c8`; only one (`FUN_000cfb80`) is read. Reading a few more of the other fourteen would identify the object class(es) that register themselves as SPU-light candidates (position + a floor-clamped, scaled intensity scalar) - which this session could not name from one caller alone.~~ **partly answered 2026-09-18, later still**: the record the producer (`SpuLight_AddCandidate`) writes is `(position.xyz, w, colour.rgb, radius)` - see renderer.md's "The `+0x2084` companion word is the frustum-survivor count" section. The fourteen unread call sites are still unread; identifying the object classes is now a "which lights exist" question, not a structural one.
- ~~Whether the producer's 128-slot list (`+0x2098`/`+0x20a0`) is what gets selected down into the already-live-captured 8-slot buffer (`+0x2080`/`+0x80`) is untraced - no selection/compaction function connecting the two offset pairs was found. Worth a targeted search (`get_xrefs_to` on the two offset constants, or a live RPCS3 read of the 128-slot region alongside the 8-slot one during the same race) before assuming they are the same data at two lifecycle stages.~~ **done 2026-09-18, later still**: `SpuLight_CompactVisibleCandidates` (`0x0040d728`) is the compaction, called every frame from `Scene_PrepareFrame`, and `+0x2084[index]` is its survivor count - see the dated entry above and renderer.md.
- **Updated 2026-09-17, later still (`lane-hd-amphiseum-hue`)**: the tile rows are now known to be real material boundaries (`hd-material-probe.py --pair-dir`, dated entry above) - the ceiling's own materials (`animhexlights`, `cf_diff_spec`, `lambert`, `base_diffusespecular`'s non-lightmap slot) draw exactly the "ours brighter" rows, and their DXT1 albedo textures pixel-verify neutral grey, no colour cast - refuting the channel-order/decode hypothesis for this circuit specifically. ~~The sharpest remaining lead is `Lighting.Sky colour`...~~ **checked and refuted 2026-09-17, later still (`lane-hd-ambient-light`)**: `Lighting.Sky colour`'s consumer is found (`Scene_PrepareFrame`, `0x003aa888`) and it is a screen backdrop/clear-fill pass, gated by `Debug_Draw_sky`/`g_ZoneEffectsActive`, entirely outside material shading - it cannot explain the ceiling's colour, since the ceiling is opaque drawn geometry that would occlude any backdrop fill underneath it. `Constant ambient color` (corrected to `+0x420`, not `+0x430`) is separately confirmed bound into the material shader pipeline exactly as this project's `scene.light.ambient` already assumes. Two further candidates (`Enable ambient false lighting`, vex-authored dynamic lights) were also checked and refuted the same session - see the dated entry above and renderer.md's new section. ~~The next unchecked lead is measuring the per-pixel `ndl` on the ceiling's own chunks directly ... and a per-material microcode sweep of the four ceiling materials for which named engine parameters their own fragment programs declare~~ **the microcode sweep is done, 2026-09-17, later still (`lane-hd-ceiling`)** - see the dated entry above and renderer.md's new section: zero of the four programs ever reference `constantAmbientColour`, which is the wrong-operation finding this thread was chasing, and the disc's own replacement (a curve over the baked vertex colour set) reconstructs to the reference's warm-band hue on `animhexlights` from disc values alone (unsaturated, and against a capture that does not itself show the dome - see the dated entry above for both caveats). The per-pixel `ndl` measurement is still not directly taken (still inferred from the authored sun direction and normal facing, not measured), though it is no longer load-bearing for the ambient finding, which does not depend on `sun_diffuse`'s exact value being zero versus merely small.
- The per-material probe's own contradiction is the sharpest lead now: find why `track_surface` (no authored sun term) reads *more* deficient than the population that does get the blanket sun addition, not less - candidates worth checking against the disc before anything else: whether `track_surface`'s own `prelitScale`/`prelitPower` pair actually differs from the shared assumption, whether its lightmap texture decodes correctly, and whether its distinct `specular_exponent` (70, against the 32 the rest of the sampled population carries) points at a genuinely different `.rcsmaterial` row being read
- Extend the per-material probe past pose `00`'s 14-material sample - run it against Anulpha Pass or another circuit with more materials in frame, and reproduce the same role-bucket/track_surface checks, to see whether the "no clean clustering by role bits" reading holds generally or is a small-sample artefact of the fourteen materials pose `00` happens to show
- Pixel-verify `oag-texture`'s `.gtf` decode against a reference decoder on a flat, evenly-lit Talon's Junction albedo (candidate (c), not reached this session - only that the decoder carries no gamma logic of its own and its ground-truth test passes, neither of which rules out a bit-level DXT/BC defect)
- Sweep per-material microcode (`scripts/ps3-microcode.py`) for a read of `0x1aaf7631`'s fourth byte, to settle what it gates
- `shadowMapTex` remains unread; locate what projects it and whether the disc's shadow map is reachable from data already on disc
- ~~Grab Amphiseum on rpcs3~~ **done 2026-09-13**: `data/reference/hd-capture/amphiseum-matched/` (3 matched-camera poses) - see the dated entry above for what it found (a black-wall-panel defect at the grid, and a sign-flipped whole-frame gap versus Talon's Junction/Sol 2)
- ~~Chase the Amphiseum grid-start wall-panel defect~~ **done 2026-09-13**: `EmissiveTexture` was being bound as the panel's own picture - see the dated entry above
- Extend `hd-frame-compare.py`'s region boxes past the shared Talon's-Junction shape, or accept per-circuit `--dump-regions` verification as the standing process - Sol 2's grid pose lands `road surface` off-track, Amphiseum's `sky` box samples an indoor ceiling. **Narrowed 2026-09-17**: this only matters for `sky`/`road surface`/`distant geometry` - `"whole (excl HUD, craft)"` only subtracts the fixed HUD boxes and the centred craft box, both of which generalise across circuits without per-circuit tuning, and is what the sign-flip re-check above used for exactly that reason. Amphiseum's own `sky`/`road surface` boxes were `--dump-regions`-checked this session (dome ceiling, side floor panels respectively) but not re-shaped.
- ~~Find why `[graphics] bloom` is inert on eleven of the sixteen circuits~~ **answered, see the 2026-09-13 (`lane-hd-resolve-fill`) entry above and renderer.md**. ~~The next step this opens: seed `envsettings_bloom` (and check `envsettings_fog`/`envsettings_light` for the same gap) from the title's own front-end `.envsettings` for any key a circuit's own file omits~~ **done 2026-09-13 (`lane-envsettings-carry`)** - 16 of 16 environments now build the chain; see the Open entry above and renderer.md's new section for the measured, narrow-not-close effect on Sol 2 and the fog/light no-op finding
- **Fix the RPCS3 nav plan for Amphiseum before anything else below that needs a capture.** `rpcs3-capture.md`'s own `--nav "Main Menu=right" --nav "Track Creation=right"x8` plan (documented for Racebox) landed on Talon's Junction this session, not Amphiseum - this build's menu route inserts a "Single Player" screen between `Main Menu` and `Track Creation` that the cited page does not name, so the carousel-count table may need re-deriving with `--nav-shots` from the actual screen sequence rather than assumed to still apply. Blocks the two bullets below.
- **Re-grab Amphiseum at a pose where `animhexlights` is actually on screen** (`amphiseum-grid`'s equivalent, gone from disk) and re-run `hd-material-probe.py --pair-dir` against it, to measure how much of the reference's warm ceiling area `animhexlights` actually covers versus `base_diffusespecular`/`cf_diff_spec` - this session's own capture's only static pose (`amphiseum-matched` pose `00`) shows no warm cells anywhere in its own top-row band (180-220°/200-205° uniformly), so the coverage question is fully open. `amphiseum-matched`'s poses `01`/`03` do show localised warm tile-grid cells, but per this thread's own two-sessions-ago finding they are moving (431-529 km/h) with the original's own speed streak baked into the reference frame, which this project's static render and `hd-material-probe.py`'s own tint-pass classifier cannot be paired against - a genuinely new capture, not a re-read of an existing one, is what this needs. Also re-check the reference's own saturation-weighted hue at whichever region turns out to be `animhexlights`, since this session's `45.5°` came from an unsaturated (~11%) reconstructed colour that HSV hue reads noisily.
- ~~Read `uvanim_diffuse_emissive`'s own colour~~ **done 2026-09-18**, see the dated entry above and renderer.md's new section: its two largest populations (2,960 and 1,664 vertices) do not read warm, so the fix's reach does not extend to this material's dominant instances via the vertex-colour-set term. Its `EmissiveTexture` sampler is the one part of "own baked colour/texture/glow term" this pass did not read and remains open if anyone wants to chase it further.
- ~~Land the ambient/prelit-curve fix in `mesh.wgsl`~~ **done 2026-09-17, later still (`lane-hd-ilevertex-ambient`)** - see the dated entry above. Both `.envsettings` keys were confirmed reused (not a new pair) by an independent offset-arithmetic chain, the curve landed on the `NO_AMBIENT`-and-not-`EMISSIVE` chunks exactly as specced, and the frame moved as this bullet predicted: Amphiseum's ceiling hue gap against the reference collapsed from 26-59 degrees to 0-5 degrees across the tile grid at pose `00`, and its whole-frame luma over-brightness (ref-ours -0.246) flipped to a near-match (+0.025). Talon's Junction got measurably darker as expected (whole-frame ref-ours widened from +0.233 to +0.308) and was landed anyway per this thread's own standing call that the darkness gap is a separate, already-tracked defect.
- Resolve the magnitude discrepancy the microcode sweep found but did not chase: the curved vertex-colour term times the material's own DXT1 albedo overshoots `hd-material-probe.py`'s measured reference luma by roughly 4-5x for `base_diffusespecular`, using the top-decile-by-luma vertex population - likely because a screen pixel is a triangle-interpolated blend, not the vertex population's own top decile, so this needs either a per-triangle/per-pixel reconstruction or a direct RPCS3 read of the live per-vertex value rather than another vertex-population statistic
- ~~Check `base_diffusespecular`/`cf_diff_spec` specifically for a channel-order or byte-order defect in their own baked colour-set decode~~ **tested 2026-09-18, confidence 70, not closed** - see the dated entry above and renderer.md's new section: an R/B swap lands both materials in the reference's warm band at high saturation and correctly breaks `animhexlights`' own reading under the same transformation, but `Mesh::vertex_light` was not changed, since nothing found this session explains which chunks would need which byte order. **What would close it**: a static read of the RSX vertex-array-format GCM registers (or their in-binary builder, `Texture_BuildGcmRegisters`'s sibling for vertex fetch) - searched for and not found this session (`EBOOT.elf` has no `Vertex_*`-named function yet, and `Texture_BuildGcmRegisters`'s own seven callers are all texture wrappers)
- ~~the render-ops SPU job's field-presence dispatch (SPU offset `0x2ae0`-`0x2b28`) loads a per-field continuation address... tracing `FUN_005fc728`'s nine PPU-side callers for one that actually populates a field~~ **advanced, then partly retracted, 2026-09-18, later still**: `_opd_FUN_004074e0` (decompiled in full) confirms opcode `0x2d`'s PPU-emitted operands are the already-live-captured SPU light buffer's current-slot address plus a value from a previously-unnamed companion array at `iRam008b83b0+0x2084` - solid, PPU-side, stands. Checking `$80`/`$81`'s own SPU-side provenance found they are the *containing subroutine's own spilled-and-reused locals*, not caller-supplied descriptor data - the "job-descriptor field 0/1/2/3" reading is retracted; the GETLLAR/PUTLLC mechanism itself survives at confidence 75. **Both follow-up routes (static entry-point trace, live local-store capture) were checked this session and both come up short** - see the dated entry below. **Genuinely blocked, not just unattempted**: closing this needs either new RPCS3 tooling (an SPU-thread-capable interface - RPCS3's GDB stub only lists PPU threads, confirmed against `rpcs3-debugger.md`'s own prior finding) or an execution-order (not file-order) static trace from this job's true entry point, both materially bigger than anything tried so far. Not recommended as a quick pickup - name it to whoever has either the tooling appetite or the disassembly budget for one of those two. **superseded 2026-09-18, later still**: that binary is the SPURS job-queue policy module, not the job - see the dated entry above. Nothing on this thread depends on tracing it further.

2026-09-18, separate lane, continued (`Enable_spu_vertex_light`): **all twelve previously-unexamined call sites are read, and two of them are a real shader consumer neither of this thread's own two prior sweeps could have found - plus a distinct producer function.** Full account: [`renderer.md`](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md), "The twelve unexamined call sites are read, and two of them are a real shader consumer..." and the slot-1 section right after it. What lives only here:

**Six of the ten new sites are more instances of already-known shapes** (four more discard-pattern draw dispatchers, two more `RigidBody`/`Absorb`/`Leach` track-hazard compilers where the buffer's value+address are staged into context fields before `Render_RunCompiledOps_q` runs, then reset - circumstantial, not traced into that function). **Two are a genuine per-chunk shader consumer**: `FUN_004074e0`/`FUN_00408fa8` gate a project-internal command-stream opcode (`0x2d`) per chunk on a per-chunk bitfield test, embedding `(slot_address, buffer_value)` as its operands and switching to a distinct compiled shader variant (`Shader_GetVariantHash(... | 0x800)`) exactly when the gate is open. **This is why neither the named-parameter sweep nor the `TXL` disc-wide sweep found a consumer** - opcode `0x2d` is neither a shader-declared parameter name nor a vertex-texture-fetch instruction; it is this project's own render-command vocabulary, interpreted by `Render_RunCompiledOps_q` (not traced). What that opcode actually does to RSX state - specifically whether `address` ends up bound as a vertex-fetch source register rather than an ordinary constant - is now the single highest-value next step, and would also settle the separate, stalled `base_diffusespecular`/`cf_diff_spec` byte-order lane's own search for "the RSX vertex-array-format register builder," since it is the same register family.

**A producer, not a reader, is also found**: `FUN_0040d990` writes an 8-float record into a *different* offset pair (`+0x2098` count / `+0x20a0` array, 128-slot cap) inside the same `0x008b83b0` structure, gated by the identical `Enable_spu_vertex_light` flag, reached from **fifteen** distinct call sites via a one-instruction thunk - far too many to be one hardcoded light list. One examined caller extracts a world position (via a per-object transform lookup) and a floor-clamped, scaled intensity-like scalar before calling it - consistent with a per-object glow contributing itself as a dynamic-light candidate, though the object's own class is not identified and the other fourteen callers are unread. **Whether this 128-slot list is what later becomes the 8-slot buffer already live-captured is not established** - no selection/compaction function connecting the two was found this session.

**Separately, slot 1 is now read** (this row's own "Next Steps" from two sessions ago), and it **refutes the standing "exact ratio on every record" claim as a universal property** - most of slot 1 matches the `A`/`A*0.25`/`A*0.1` ratio exactly, as slot 0 did, but at least two records (verified against the raw slot bytes directly, not just the derived JSON) do not: one very-low-`A` record with an anomalously large `D=10.0` (versus every other record's `~0.7-2.1`), and six records at `A=80.0` whose derived fields read a constant `10.0`/`0.0` rather than `20.0`/`8.0`. A plausible but unconfirmed hypothesis - the three fields may be independently-interpolating values that only coincide exactly at steady state - is recorded in renderer.md; not confirmed, and the prior confidence-80 "exact ratio" claim is narrowed to "the common case, not a universal property."

Confidence 82 for the new structural claims (real per-chunk consumer exists; distinct producer exists). Confidence stays at 75 or below for what the records represent. No code changed - `mesh.wgsl`, `crates/render/`, `crates/game/` are all untouched; this is RE-only, same as every entry in this lane since the live capture landed.

2026-09-18, later still, continued: **the "four-slot DMA-fetch loop" this row's own Next Steps named as the highest-value next step does not exist, and is retracted - what actually runs there is an atomic reference-count decrement against main memory, and the "field 2 continuation address" lead that looked like it would finally answer the question turns out to point at a second, unrelated SPU program.** Full account: [`renderer.md`](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md), two new sections, "`Enable_spu_vertex_light`: the 'four-slot DMA-fetch loop' is retracted..." and "The four per-field continuation addresses point at a second, unrelated SPU program...". What lives only here:

**The job-type dispatch (`ceqi $2,$81,1/2/3`) is a job-descriptor field-presence check, not a DMA fetch** - it tests up to four 4-byte sub-fields of a quadword register for nonzero (field 3's test bypassed when job type is exactly 3) and branches whichever field is first nonzero into a shared subroutine at SPU offset `0x1bd0`. That subroutine is a textbook `MFC_GETLLAR`(`0xd0`)/`MFC_PUTLLC`(`0xb4`) atomic compare-and-swap loop - the field's own value is treated as a main-memory effective address, masked to its 128-byte reservation line; a locally-staged counter byte is decremented, written back, and the *pre-decrement* value is tested for `== 1` - a decrement-and-test-for-zero, and when it *was* `1` a **second**, independently-coded GETLLAR/PUTLLC helper at SPU offset `0x2c70` fires before returning. This is what field 2's (and every other field's) DMA'd-in data actually drives: not vertex math, a shared reference count. **This retires the prior session's reading of this exact code as "the standard MFC channel sequence for LSA/EAH/EAL/Size/TagID/Cmd, i.e. a DMA get"** - the `Cmd` channel at every one of these sites is always `0xd0` then `0xb4`, never a plain `MFC_GET`/`MFC_PUT`.

**A blocking check was run before any of this got written up, on the advisor's own steer, and it changed the conclusion.** Each field's own `ila $0, <addr>` (loaded before the field test, restored via local-store save/restore around the GETLLAR/PUTLLC loop, then `bi $0`'d to on return) looked at first like a genuine "continue execution here for field 2" answer - field 2's own target, SPU offset `0x3518`, sits past this job's registered image end. `iRam008bb590` (confirmed again by direct memory read: `0x2cf8`, rounding to `0x2d00`) is exactly where a second, self-contained SPU program starts in the file - same four-`ila`-word-plus-setup header shape as this job's own byte-`0` header, its own internal branches all self-consistent relative to its own base, none reaching back into this job's image or vice versa. **SPURS only DMAs the registered `0x2d00` bytes into local store at workload start, so `ila $0, 0x3518` cannot be a reachable branch target for this job** - the "field-2 continuation" reading is retracted, not merely narrowed. What the four `ila`-loaded values are actually for is an open fork: dead code for this specific job configuration (plausible - most of this job's own PPU-side callers zero the descriptor before *and* after use, per this page's own "six pure-discard sites" finding), or `$0` serving a purpose this session did not consider. Settling it needs a live RPCS3 capture of this job's own local store during a call with a genuinely nonzero descriptor field, or tracing which of `FUN_005fc728`'s nine PPU-side callers ever populates one - neither attempted this session.

Confidence 75 for the GETLLAR/PUTLLC reference-count mechanism (fixed MFC command encodings, register dataflow read directly, no gaps); confidence 80 for the region-boundary/second-program finding (mechanically checked, not inferred). No code changed; neither extracted binary (`/tmp/render-ops-job.bin`, `/tmp/render-ops-job-ext.bin`) is committed.

2026-09-18, later still, continued: **the cheaper, no-new-tooling next step this row's own previous entry named - tracing `FUN_005fc728`'s PPU-side callers for one that populates a nonzero descriptor field - paid off, and answers what opcode `0x2d`'s operands are from the emitting side.** Full account: [`renderer.md`](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md), "Opcode `0x2d`'s `(address, value)` operand pair is decompiled at its own PPU-side source...". What lives only here:

`_opd_FUN_004074e0` - one of the two functions this page already named as a real per-chunk shader consumer - was decompiled in full. Once per draw call it calls `_opd_FUN_0040d390()`/`_opd_FUN_0040d370()` (the exact two functions this thread already read and live-captured the output of: the double-buffered `iRam008b83b0+0x80+index*0x1000` light-candidate array this project has 8-record structural data for). Per chunk, when the per-chunk gate bit is set, it emits opcode `0x2d` with those two return values **verbatim** as its operands: `address` = the buffer's own current-slot main-memory pointer, `value` = a read from a **previously-unnamed companion array at `iRam008b83b0 + 0x2084`**, sharing the same running index as the `+0x80` array. This is a direct decompile chain with no inference at any step - confidence 85, the highest this thread has reached on the *emitting* side of the question.

**What it does not close**: whether the compiled-ops stream carrying this pair is literally what the SPU-side field-presence dispatch (`$80`/`$81`, two entries above) reads. Tracing `$80`/`$81`'s own provenance backward on the SPU side lands on a *third* distinct GETLLAR/PUTLLC block (SPU offset roughly `0x2900`-`0x2ae0`), shaped like a hash-table insert/lookup, not read this session - the next concrete step, and cheaper than a live capture since the address is already in hand.

2026-09-18, later still, continued: **checking that third block's own entry point (rather than pushing forward again) finds a save/restore pair that retracts part of the "four-slot DMA-fetch loop is retracted..." entry above - a second time this same thread's own structural reading dissolved on closer inspection.** Full account: [`renderer.md`](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md), "Correction: `$80`'s 'job-descriptor field 0/1/2/3'... readings are retracted...". What lives only here:

SPU offset `0x2920` (the region containing the field-presence dispatch) opens by spilling its **own incoming** `$0`/`$80`/`$81` to local store (`0x1580`/`0x1590`/`0x15a0` - the same triple the "no field set" path restores before returning), then immediately reassigns `$80` and `$81` as scratch for the rest of the region, including the GETLLAR/PUTLLC hash-table block. **The field-presence dispatch runs before the restore** - so the quadword it tests is this subroutine's own internally-computed value, not caller-supplied job-descriptor data. **Retracted**: "job-descriptor field-presence check" and "the field's own value is a main-memory effective address" - neither survives once `$80`'s provenance is checked. **Survives unchanged**: the GETLLAR/PUTLLC identification itself (fixed MFC command encodings), the decrement-and-test-for-`1` dataflow, the region-boundary/second-program finding, and the entire opcode-`0x2d` PPU-side decompile (PPU-side evidence, untouched by an SPU-side correction) - only the label for *whose* counter is being decremented changes, back to "unknown, not yet a job-descriptor field." Confidence for the surviving mechanism returns to 75, the same "single-site inference, not closed" tier it started at.

**This is the second time this exact thread has had a plausible structural reading dissolve on closer reading** (first the DMA-fetch loop, now the field-descriptor framing) - both times because a register or code shape looked meaningful in isolation and turned out to belong to different code (a separate SPU program; a callee's own spilled locals) once checked. Worth naming so a third session does not repeat it: check what saves/restores a register before naming what it holds.

**Per this session's own advisor-directed stop, not chased further**: `0x2890`-`0x2920`'s own true entry point (reached from inside a scan loop that doesn't itself set `$80`/`$81`) and whether its real SPU-ABI argument registers (`$3`/`$4`, unexamined this session) carry the opcode-`0x2d` pair directly - the question that would finally close this. A live capture of this job's own local store during a race with a real `0x800`-path chunk (`scripts/rpcs3-spu-light-dump.py` already has the shape) is named as the more promising route from here, over further hand-tracing.

2026-09-18, later still, continued: **both of the above were checked this session, and both come up short - one blocked by an already-documented project limitation, the other by a genuine dead end in the static trace.** Full account: [`renderer.md`](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md), "Both named next steps are checked this session, and both come up short...". What lives only here:

**The live-capture route is not available.** `docs/reverse-engineering/rpcs3-debugger.md` already established (before this thread existed) that RPCS3's GDB stub - what `rpcs3-spu-light-dump.py` and every other RPCS3 script in this project is built on - lists only PPU threads; "the 200-330% CPU the process burns is RSX and SPU work the thread list does not cover." `rpcs3-spu-light-dump.py`'s own read works only because the `Enable_spu_vertex_light` buffer sits in ordinary PPU-addressable main memory - it never touches an SPU core's own local store, and nothing in this project's GDB-stub client can. Reading this job's local store live needs a different RPCS3 interface than any this project has scripted against - real, new tooling research, not a quick follow-up script.

**The static route runs out of writes to follow.** Every instruction in the `0x0`-`0x2920` range was searched for a write to `$80`/`$81` as its destination: exactly one exists (SPU offset `0xd0c`, inside an unrelated small function, copying from real argument registers `$3`/`$4`), and nothing writes either register anywhere before it. Either they hold SPU reset-state values (unlikely to be meaningful) or the disassembly's own file order does not track execution order closely enough for a linear scan to find the real assignment - distinguishing the two needs tracing from this job's actual entry point in execution order, a materially more expensive kind of trace than anything done on this page so far.

**Status of the thread as of this session's own close-out**: the GETLLAR/PUTLLC mechanism (confidence 75), the opcode-`0x2d` PPU-side decompile (confidence 85) and the region-boundary/second-program finding (confidence 80) all stand. What remains open - whether the compiled-ops stream's `(address, value)` pair is literally what the SPU-side dispatch consumes - is not answered, and neither of this session's own two named routes to it works without either new tooling (an SPU-capable RPCS3 interface) or a substantially larger static effort (an execution-order trace from the job's true entry point). Recommending a pause on this specific sub-thread until one of those two is available, rather than continuing to spend session time on approaches already shown not to close it cheaply.

2026-09-18, later still, continued: **the one unblocked lead - the `+0x2084` companion array itself, ordinary PPU main memory rather than SPU local store - is read live.** Full account: [`renderer.md`](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md), "The `+0x2084` companion array is read live...". What lives only here:

`scripts/rpcs3-spu-light-dump.py` gained a one-line read of `ptr + 0x2084 + index*4` alongside its existing slot dump. Five live snapshots at Amphiseum, default play, two seconds apart: `companion_value_u32` reads `7, 7, 7, 2, 2` - small integers, changing over time, and read as a subnormal near-zero every time under the `f32` interpretation (settling that this is meant to be an integer, not a float). Both values sit inside the buffer's own already-established `0`-`8` record cap, consistent with (but not proven to be) a per-frame count of how many of the slot's 8 records are valid - `slot_nonzero_bytes` stays flat regardless of the companion value, which is what you'd expect if stale, unused record slots keep old nonzero bytes and an explicit count is exactly how a consumer would tell valid from stale. **Confidence 55** - live and reproducible, stronger than hand-disassembly alone, but no consumer anywhere has been read treating this value as a count or bound, so the interpretation rests on magnitude-range correlation only.

2026-09-18, later still, a fresh session re-checking the previous close-out before extending it: **the "genuinely blocked" verdict was blocked on the wrong binary, the `+0x2084` question is closed at confidence 88, and the SPU consumer of the light buffer is identified, disassembled in full and connected to the opcode-`0x2d` gate.** Full account: [`renderer.md`](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md), four new sections from "The three surviving `Enable_spu_vertex_light` claims are re-derived independently..." onward. What lives only here:

**The spot-check first.** All three surviving claims (opcode `0x2d`'s operands at 85, the `GETLLAR`/`PUTLLC` decrement at 75, the `0x2d00` boundary at 80) re-derive cleanly from the decompiler and the SPU disassembly. One is narrowed: the program at `0x2d00` is `0x280` bytes, not open-ended, and a third blob with a different header starts right after it.

**The fourth wrong turn, and the one the other three stood on.** `0x00848780`/`0x2d00` is the SPURS job-queue **policy module** (what `cellSpursAddWorkloadWithAttribute` takes, by definition) - the scheduler shared by nine queues, which is why it has atomic refcounts, a hash table and a job-type switch and no render code. The render-ops **job** is named on the PPU side as descriptor field `0` of `FUN_005fc728`: `Live_RcsBlackbox`, `0x0084b700`, `0xd690` bytes, registered by `FUN_005fc550` through the same named-binary registry `LightCulling` uses, carrying the same `0xc0dec0de` job header `Trails`/`WakeTrail` do. Its 16-byte user data hands it the ring copy of the `0x160`-byte context whose `+0xd8`/`+0xdc` are the compiled-ops stream - so it DMAs the stream and interprets it there. The `$80`/`$81` hunt, the "whose counter" question and the two named unblocking routes were all aimed at the policy module; none of them is needed to read the job. A one-sentence lead for whoever does return to the policy module: `_opd_FUN_00469340` is the PPU `lwarx`/`stwcx.` twin of the SPU decrement, on the named-binary registry's refcount.

**`+0x2084` is the frustum-survivor count.** `FUN_0040d728` (`SpuLight_CompactVisibleCandidates`), called every frame from `Scene_PrepareFrame`, flips the double buffer, sphere-tests each of the up-to-128 candidates against the frame's clip planes, copies survivors into the new slot and writes their number to `+0x2084[new index]`. Three consumers read it as a bound: `SpuLight_DispatchLightCullingJob` DMAs `count * 32` bytes, `SpuLight_AnyVisibleLightTouchesSphere` loops `count` records for the `RigidBody`/`Absorb` family's per-object gate, and `FUN_004074e0` suppresses the address when it is zero. The producer's record is `(position.xyz, w, colour.rgb, radius)` - the "`A`, `A*0.25`, `A*0.1` exact ratio" was the dominant orange light colour, and slot 1's "anomalies" were lights of another colour, not interpolation. The slot holds 128 records; the earlier "8" was a stale high-water mark, and record 7's "stationary" position on Amphiseum was a stale record past the count. The existing five dumps, diffed record by record, change exactly `count` leading records (`s3 -> s4`: records `0`-`1` at count `2`).

**`LightCulling` (`0x007f6880`, `0x490` bytes) is the SPU consumer, read end to end.** Dispatched by the track-visibility pass (`FUN_0040aba0`) with the visible lights, the track's per-chunk bounding spheres (`track + 0x24`, 16 bytes each) and the address of the `0x1000`-byte bit table at `0x00f4a280`; it sets bit `chunk` when any light's sphere intersects the chunk's sphere and DMAs the table back; the caller waits on the job's ticket before returning. That table is exactly what `FUN_004074e0`/`FUN_00408fa8` test per chunk before emitting opcode `0x2d` (their two base symbols are `0x200000` apart and resolve to the same address). `0x800` is bit 11 of the permutation word, `SVC1` per `rcsmaterial.md`'s independent derivation - the `SpuVertexColours` variant. **The PPU side of `Enable_spu_vertex_light` is now closed end to end**: candidates -> frustum cull with count -> chunk cull on SPU -> per-chunk opcode `0x2d` + `SVC1` variant -> `Live_RcsBlackbox`, which by the variant's own declared input must write the `SpuVertexColours` stream. That last computation is the one thing still unread, and it is a static `0xd690`-byte file range, not a live-tooling problem.

Confidence 85-88 for everything decompiled (the chain, the count, the dispatch), 75 for the `LightCulling` job's arithmetic (hand disassembly, corroborated by both loop strides matching the PPU-side sizes), 78 for `0x800 = SVC1`. Fourteen names landed in `names.tsv` and the Ghidra database. No code changed - `mesh.wgsl`, `crates/render/`, `crates/game/` untouched; nothing here is wired, since what the SPU *computes* per vertex is still unread and "never invent" holds until it is. Extracted binaries and disassemblies now live under `data/extracted/ps3/spu-jobs/` and the live slot dumps under `data/traces/hd-spu-light-companion/` (both gitignored, never committed; moved out of `/tmp` at the maintainer's request so they survive a reboot).

2026-10-08 (`hd-exposure`): **the `pow(1/2.2)` is not an extra encode, the original's exposure is unity, and the
brightness gap is per surface.** Full account in
[renderer.md](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md), "The exposure is unity, and the brightness gap is
per surface". What lives only here:

**Reconciliation.** Every earlier reading on this page is right for its own frame. Ours reads 0.77-0.84x at Talon's and
Sol 2's grid and tunnel frames (the 09-13 "darker" family and `bloom-racing`'s "HD bloom reads weaker") and **1.54x** in
Talon's corridor (the `hd-gantry` 1.6x, an exact camera, original at clock 0.26.3). The gap does not track the race
clock: final/scene is 1.02-1.06 on nine scene-target dumps including clock 0.00.0 (`scripts/hd-exposure-ratio.py`), so the
original's `scale` is 1.0 and the sky-law entry's "race-start exposure transient" is not supported on the two circuits
measured at 0.00.0 (Sol 2 at 0.00.3 was not dumped, so what made those frames bright is open).

**The register read.** `NV4097_SET_SHADER_PACKER` (`0x1fec`, RPCS3's `framebuffer_srgb_enabled`) is 1 on scene and
bloom-ladder draws and 0 on swap-buffer draws; albedo, normal and ladder units carry gamma nibble 7, the lightmap unit 0.

**Where it lives.** In ours the corridor road is the lightmap `prelit` term alone (road box 0.707, 0.244 without it,
ambient and vertex light move nothing); the original's program has the same equation, constants and raw lightmap unit and
lands ~4x lower in linear light. A uniform `x0.25` matches the road's quantiles and misses the distant geometry, so it is
**not** the fix and was not shipped.

**Negatives, so nobody re-runs them.** (1) Removing the final `pow` or pinning `scale` is not the answer: `scale` is
already 1.0 at that pose (byte-identical when pinned). (2) Restoring the lightmap's sRGB decode would be wrong: its
sampler word is nibble 0. (3) The ambient, vertex-light and sun summands are zero or near it on that road.

**Open / next (done 2026-10-09, see the entry below).** Sky-law method on the corridor road draw: hook-dump unit 1 and unit 0 of a `fp 0x759ac1` draw in full
(`rpcs3-drive.py place --hook`, a copy of `hd-sky-law`'s `hook_sky.py`), predict the road's bytes from the live
constants and diff against `hd-exposure/placeA/00-40cc0000.bin`; then check our lightmap sample (atlas, UV set, mip,
colour/alpha split of the DXT5) against it. Also fix two stale comments (`hd_bloom.rs` header, `fs_encode`) that still call
the final `pow` a stand-in.


2026-10-09 (`hd-metropia-bloom`): **Metropia's white lamp blobs are attributed, not fixed.** The whole excess (ceiling band clipped
10.9 % against the original's 3.1 %) is the gate's *frame luminance* term (`Bloom from frame contribution` 1.0, 33x Talon's), which the
adaptation fade should cancel; ours reads `adapted` about 0.1 (estimate), the original's behaves as `>= ~0.25`. Table, method,
frames and the saved grid state are in [renderer.md](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md), "Metropia's white lamp
blobs". **Next**: (1) read back our own `adapted` at tick 0 on that pose (the sweep brackets it in 0.09-0.13, yet the frame's linear luma is 0.28, which would fully fade it, so our reduction or scene mean is suspect, not the original's input); (2) read the original's `adapted` at the grid (the `FunkLayer + 0x254` read at `0xc50ee0` came back zero; break on the lerp store in `FunkLayer_RunBloomChain`). The first guess, that the original averages encoded bytes, was checked and is not supported. Same check applies to every Fury/DLC circuit (`01_vineta_k`, `03_track`, `05_ubermall`, `12_sol_2`, ...) that authors a large frame term.
2026-10-09 (`hd-corridor-road`): **the corridor's excess was a dropped output scale, not the lightmap term.** Full account in
[renderer.md](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md), "Every lit program ends on a /2 output scale". The road
program evaluated on live textures lands on the live scene target at 1.03; the final instruction of every lit program drawn into
the scene (draws 87-522) carries scale `/2`, which `ps3-microcode.py` never printed. Ported: `Looks::lit_output_halved` (HD),
`Light::output_scale`, fog colour halved with it; the exposure's adaptation now averages encoded luminance (inferred), because
otherwise ours' exposure rose to 1.73 on the halved frame. Steady-state corridor 2.49x -> 1.23x, grid 1.17x; Pulse byte-identical.
The earlier "`fp 0x759ac1` road draw" was a wall program.

**Open.** (1) **The matched pairs are probably stock-config (`Write Color Buffers` off) captures and so read about 1.77x too bright** (same corridor pose: 0.587/0.589 off, 0.331/0.342 on); retire them as references and re-shoot the poses that matter with the option on; the older "ours darker" readings that used them are suspect. Verify the `hd_behind_glass_frame` teal window the same way. (2) Amphiseum steady state was not captured (Racebox presses are
unreliable: 9 landed on Talon's, 10 on Modesto); its remaining 2x deficit is in vertex-lit or emissive terms. (3) Non-road families
read 1.1-1.4 (sun, `x8` specular scale, saturation). (4) Particles and effects end on `/2` too and `oag-fx` does not apply it.
(5) Omega: a GCN `omod` census of the circuit pixel shaders.

2026-10-09 (`hd-bloom-adapt`): **The Metropia blobs are fixed in the reduction, and the fix shows the frame-wide darkness.** Our `adapted` read 0.098 (a biased
ladder) where the original's is the encoded linear mean (exact linear ladder with the sRGB write on, read as raw bytes, 1/255, every texel left when a side reaches 1);
`oag_post::hd_bloom` now does the same: Metropia ceiling clipped share 10.87 % -> 1.38 % (original 3.07 %; throttle 08 20.27 % -> 2.63 % against 11.56 %), exposure 1.0
as in the original. renderer.md, "The adaptation is the encoded linear mean". **Cost, read with "Open / next" above**: whole-frame luma Metropia 0.364 (original 0.488, was 0.518),
Amphiseum 0.191 / 0.355 (0.286 / 0.430, was 0.280 / 0.486), Talon's within 0.010 of before, Sol 2 identical. The old low `adapted` lifted dark circuits by a x1.5-2 exposure that hid a
scene 25-29 % darker than the original's at scale 1; the cause is unattributed (the same 0.77-0.84 ratio at the Talon's and Sol 2 grids; the corridor-road lane's 4x-too-bright road
points the other way). **Next**: (1) the live read of the original's `adapted` (`FunkLayer + 0x25c`; `0x003b4690` never hit under `wait_at`, use `wait_for_stop` on `0x003b5318`);
(2) find the circuit-wide darkness at scale 1 (sky and unlit surfaces match, per the exposure section, so look at lit surfaces).
2026-10-09 (`hd-dark-ref`): **There is no darkening regression; the references were the artefact.** Full account in [renderer.md](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md), "There is no darkening regression". Every original number in `hd-bloom-adapt`'s cost line was a `Write Color Buffers`-off capture (Metropia's from that lane's own config; the September pairs inferred). Re-shot with it on (four circuits, Metropia and Talon's on two boots, a scale-100 control): Metropia grid 0.285 (ours 0.273), Talon's 0.356 (0.381), Sol 2 0.416 (0.447), Amphiseum 0.291 (0.295); driving poses within 0-5 % except one dark Amphiseum pose (1.21x). The bisect puts the whole drop in the corridor-road merge (`/2`), which these numbers confirm; the bloom-adapt cost line was measured on a branch that did not yet carry the `/2`. The falsifier fired, so the live `adapted` read was not needed and not done. What is left is in Next Steps: the hull reads 3x brighter in ours (same hull, same livery), and Metropia's lamps are now slightly over.
