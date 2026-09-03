# ADR-0041: One row for what resolves the frame, and MSAA on an axis of its own

## Status

Accepted. Supersedes the **row shape** of
[ADR-0013](0013-anti-aliasing-architecture.md) - its analysis of how the three
classes cost, look and compose is unchanged and still the reference. ADR-0013
asked for this document by name: *"records the shape so the eventual change is
an addition to this document's successor rather than a rediscovery."*

## Context

ADR-0013 was written before there was any temporal reconstruction to hang a row
off. It grouped the anti-aliasing this project had into one `AntiAliasing`
enum - `off`, `fxaa`, `smaa`, `msaa4x` - and put the upscaler on a separate
`Upscaler` row, `off`, `fsr1`. Both readings were right at the time.

FSR 3.1 landed since, and it is both of those things at once: an upscaler *and*
an anti-aliaser. `assets/ui/menu.toml` already says so where it argues that the
UPSCALER row deliberately carries no `fsr3` warning - *"FSR 3.1 is temporal: it
reconstructs from several previous frames, so even at 100 % it has more samples
per pixel than one frame carries and is doing real work - what upstream calls
its native-AA mode."*

That leaves the axis cut in the wrong place twice.

**Rasterization and resolve are in one enum.** `AntiAliasing` makes `msaa4x`
mutually exclusive with `fxaa`, though MSAA is a rasterizer sample count baked
into every scene pipeline at `race::Scene::new` and FXAA is a fullscreen pass
over the resolved result. Nothing about the frame makes those alternatives. The
enum's own doc comment says as much - *"**Not one dial.** MSAA, a spatial
post-process pass and a temporal one act at three different points in the
frame"* - and then holds all three in one dial anyway.

**Resolve-time AA is split across two rows.** `fxaa`, `smaa` and `fsr3` all
answer "what carries this frame onto the surface", and they sit on two
different rows that then need warnings to describe their interaction. There are
**seven** such warnings in `menu.toml` today, five of them describing
combinations of the anti-aliasing row against the upscaler row.

The measurement that made this urgent, from a maintainer's own HD/Fury race at
2560x1440, MSAA 4x, FSR 3.1, `target_fps = 120`: `GPU SCENE` **2.4 ms** and the
FSR 3.1 chain **2.4 ms**, at 80-90 FPS. MSAA 4x sits inside a ~7 ms block of
untimed cost while FSR 3.1 anti-aliases the same frame independently. Both are
selected; only one of them is needed.

## Decision

**Two rows, split on what part of the frame each acts on.**

| Row | Values | Applies |
| --- | --- | --- |
| **RECONSTRUCTION** | `off`, `fxaa`, `smaa`, `fsr1`, `fsr3` | live |
| **MSAA** | `off`, `4x` | `restart_required` |

Row one is *what resolves the frame onto the surface*. Every value answers that
single question, so exclusivity is structural rather than something a warning
has to describe. Row two is a rasterizer sample count, read once at
`race::Scene::new`.

**`fsr3` greys the MSAA row**, through the existing `disabled_by` mechanism:

```toml
disabled_by = { setting = "graphics.msaa", value = "…" }
```

keyed on RECONSTRUCTION holding `fsr3`. Per `docs/architecture/menus.md`, a
greyed row still takes the cursor and still shows its stored value, so a player
who leaves `fsr3` gets their `4x` back untouched.

**FXAA does not become a toggle**, and this is the part most likely to be
revisited by somebody who has not read the reasoning, so it is recorded
plainly. FXAA is beneficial only when nothing else is reconstructing:

- **with `fsr3`** it is wrong in both orders. Run before, it blurs exactly the
  sub-pixel detail accumulation exists to gather, which is why
  `upscale::Framebuffer::resolve_scene` skips it outright today and why AMD's
  own integration guidance requires an unfiltered input. Run after, it would
  soften a frame `rcas` has just sharpened;
- **with `fsr1`** it fights the edge-adaptive resample, which is what
  `menu.toml`'s `FIGHTS THE UPSCALER'S EDGE-ADAPTIVE RESAMPLE` already says.
  FXAA *after* an upscale is a real technique elsewhere; this engine runs AA
  before the upscaler by ADR-0013's design and that is not revisited here;
- **with the bilinear blit** it is the cheap low-end path and genuinely useful.

A setting that is only ever right when no other value on its axis is selected
belongs *on* that axis. A separate toggle would make every wrong pairing
representable again, which is the whole thing this ADR removes.

**No FSR quality tiers.** ADR-0013 sketched a `TemporalAntiAliasing` enum
carrying `FSR3 Quality` / `Balanced` / `Performance`. Those are render-scale
presets, and this project has shipped a real `render_scale` row and a dynamic
resolution controller ([ADR-0037](0037-dynamic-resolution-varies-a-viewport-not-an-allocation.md),
[ADR-0040](0040-the-dynamic-resolution-budget-is-a-share-of-a-frame.md)) since
that was written. Duplicating the axis would give two rows that disagree about
how many pixels a frame has. The sketch is declined on those grounds rather
than forgotten.

## Alternatives considered

**Leave both rows and add a sixth warning** for `msaa4x` + `fsr3`. The
mechanism fits - `REDUNDANT ON TOP OF 200% RENDER SCALE SUPERSAMPLING` is
already a redundancy claim rather than a no-effect one - and it was the
recommendation until ADR-0013's own successor clause turned up. Rejected
because it treats the symptom: the combinations stay representable, the warning
count goes to eight, and a player who selects the expensive one anyway has been
told and not helped. **The measured cost of that combination is what makes
"told and not helped" the wrong answer here** - the whole thread started with a
maintainer running it unknowingly.

**Merge MSAA into row one as well**, which is what a merged "Anti-Aliasing"
dropdown in a shipping game usually looks like. Rejected for two concrete
reasons: it would make `msaa4x` + `fsr1` unrepresentable, which `menu.toml`
explicitly records as a *working* combination - *"it resolves before any of
this runs and composes with FSR 1 rather than fighting it"* - and
`restart_required` is a per-row string, so the merged row would force a
race restart on `fxaa` and `fsr3`, which apply live.

**Grey nothing and warn about `msaa4x` + `fsr3` instead of disabling it.** The
argument for it is real: MSAA is *not* inert under FSR 3.1, because the chain
reads the MSAA-resolved colour through `Framebuffer::perceptual`. Rejected
because the benefit is a marginally cleaner colour input to a pass already
doing temporal anti-aliasing, while three of four samples on the depth and
velocity attachments are rasterized, stored and discarded - `INPUTS_MULTISAMPLED`
takes sample 0 only. The niche it serves, "I have headroom and want cleaner
geometric edges", is served better by the `render_scale` row that already goes
to 200 %.

## Consequences

**Five of the seven `menu.toml` warnings delete themselves**, because their
combinations stop being representable: `NO EFFECT WITH FSR3, WHICH READS THE
UNFILTERED SCENE`, and both `FIGHTS THE UPSCALER'S EDGE-ADAPTIVE RESAMPLE`
pairs. Surviving: `NO EFFECT AT RENDER SCALE 100 OR ABOVE` (`fsr1` against the
render scale) and both `REDUNDANT ON TOP OF 200% RENDER SCALE SUPERSAMPLING`
(`fxaa`/`smaa` against the render scale, no upscaler involved).

**`post::fsr3::INPUTS_MULTISAMPLED` and the second `prepare_inputs` pipeline
stay, and an earlier draft of this ADR was wrong to say they could go.** The
reasoning that they become unreachable is that `fsr3` greys the MSAA row - but
greying stores the value rather than changing it, and a scene's sample count is
baked at `race::Scene::new`. `RaceStage::temporal` reads
`Scene::sample_count()`, *what the running race was built with*, not the row.
So a player who starts a race at `msaa = 4x` with `reconstruction = off` and
then switches to `fsr3` mid-race hands FSR 3.1 multisampled depth and velocity
attachments, on a build that applies live. Deleting the multisampled pipeline
would make that a validation failure in a player's frame loop. It is the one
path that reaches it, and it is reachable, so it stays - and the mechanism that
already documents this class of disagreement, `Temporal::sample_count`
travelling with the attachments rather than being read from the settings, is
what makes the surviving case correct rather than merely tolerated.

**Measured, 2026-09-03**, five `--presented` captures of the same HD/Fury frame
at `--render-scale 50`, one per reconstruction value: all five hashes differ,
and `fsr3` with `msaa = 4x` differs from `fsr3` with `msaa = off`. The second
half is the evidence for the paragraph above - the multisampled build is not
merely reachable, it changes the picture.

**It costs a settings migration**, and that is the bulk of the work. One key
becomes two, `[render_profiles.<title>] anti_aliasing` splitting into a
reconstruction key and an `msaa` key, with `msaa4x` in an old file landing as
`msaa = "4x"` and reconstruction `off`. `settings::MOVED_TO_RENDER_PROFILES` is
the precedent for a migration of this shape.

**A player who had `msaa4x` and `fsr3` loses MSAA on the next launch**, with no
prompt. That is the intended outcome and it is still a picture change nobody
asked for - the release note has to say it rather than leaving it to be noticed.

**A greyed row and a running race can disagree for the length of that race.**
MSAA's sample count is baked at `race::Scene::new` while RECONSTRUCTION applies
live, so switching to `fsr3` mid-race greys the row while the current race
stays multisampled. `Menu::in_effect` exists for exactly this file-versus-
running-build disagreement and is what the greyed row must read; the existing
`TAKES EFFECT THE NEXT TIME A RACE STARTS` note carries the rest.

**Nothing here makes a frame faster on its own.** It removes a combination that
costs a lot and buys almost nothing, which is a different claim. The budget
under-measurement that let that combination go unnoticed - `drs::SCENE_SHARE`
asserting the timed pass is 45 % of a frame where it measured 20 % - is
untouched by this ADR and is its own decision.
