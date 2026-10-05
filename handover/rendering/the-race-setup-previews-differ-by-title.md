---
categories: [rendering, tooling]
---

# The race-setup previews are meshes on Pulse and HD, sprites on Pure

2026-09-05, corrected and closed out 2026-09-10.

The permanent write-up is
[`docs/formats/race-setup.md`](../../docs/formats/race-setup.md); this thread is
only the work still to do.

## The finding in one line

**Pulse PSP, Pulse PS2 and Wipeout HD draw a rendered 3D mesh for every track
and craft preview. Pure draws a pre-rendered 256x128 still for both**, one of
three the entry's own `screen.xml` authors, cycling `Info -> Side -> Top` at
two seconds a step. Pure's *speed-class* preview is likewise a layered 2D stat
graph.

That the two answers differ is the point. The naive reading - a track
thumbnail - is wrong on Pulse and HD, where authoring a placeholder thumbnail
would have been an invention. It is right on Pure, where wiring a mesh was
one, and was wired for nine hours on 2026-09-10 before the maintainer spotted
it.

## What is verified

| Title | Track preview | Craft preview |
| --- | --- | --- |
| Pulse PSP | `<location>\FE\forward.vex` / `reverse.vex` | `<team>\<variant>_FE.vex` |
| Pulse PS2 | same names, own geometry | same names, own geometry |
| HD / Fury | `<Model name="TrackModel">` | `<Model name="ShipModel">` |
| Pure | `<location>\screen.xml`'s three stills | `<team>\screen.xml`'s three stills |

Pure's rows are resolved end to end and **need no inference at all**: a PPSSPP
texture dump matched the drawn pixels to `FEData.wad` entries at RMSE 0, and
each entry is named in full by its own `screen.xml`
(`hash_name(r"Data\Environments\01_Vineta_K\Images\Track_1.mip")` is
`0xd6404ac7`, `FEData.wad` entry 194). Both screens are wired and draw, on
`pure-psp-eu.chd`, through the same `oag_ui_screens::picker`/`oag_game::picker_stage`
machinery Pulse's use.

## Open

- **`screen_m.xml` and `screen_tt.xml` are unread.** Pure authors them beside
  `screen.xml` for split screen and time trial. Choosing between them needs a
  race-mode parameter `oag_game::preview::slideshow` does not take; on the
  circuit checked, all three name the same three stills, so nothing is
  currently visibly wrong. `screen_z.xml` (Zone) *is* wired.
- **Pure's craft preview call site (`0x08951bd0`,
  `TeamSelection_ApplySelection`) is fully decompiled** - see
  [`race-box-screens.md`](../../docs/ghidra/functions/psp-pure-eu/race-box-screens.md).
  It composes `%s\VR\Phantom.vex` or `%s\Phantom.vex` from
  `*(matched_record + 0x9c)`, gated on two flags on the *screen* object
  (`+0xe1`, `+0xe2`) that trace to a `"Championship"` record's tier byte
  against `3` plus multiplayer/tournament-permission lookups - **not the
  `Class` global**. Confidence 60, not wired. **This is the one preview mesh
  Pure does load**, a special case layered over the still, not a default path.
  Different function from the in-race model loader
  (`FUN_08927dac`/`FUN_08927694`).
- **Pure's `Layout` still borrows Pulse's `panel` and `preview` rects**
  (`[290, 25, 170, 200]` / `[290, 49, 170, 96]`). Its own XML states a
  `<Viewport>` of `x=12 y=45 width=233 height=162` and no `<LeftLayer>`, so
  `Layout::read` falls through to constants that are Pulse-shaped. Nothing
  Pure draws today reads those rects - its stills carry their own x/y and its
  rows come off the `<Menu>` widget - so this is latent rather than visible,
  and it will bite the first widget that does.
- **Done, 2026-09-27: Pure's selection screens now draw their stat panel.**
  `Team Selection`'s `SPEED`/`HANDLING`/`SHIELD`/`THRUST` bars and `Track
  Selection`'s `RACE RECORD`/`LAP RECORD`/`LENGTH`/`HEIGHT` block are authored
  in the *entry's* `screen.xml` (the same file as the stills), not in
  `Selection_Definition.xml` where `oag_ui_screens::picker::body` looks for Pulse's
  named `<Text>` widgets. `oag_ui_screens::picker::slideshow::Slideshow` now reads
  the colour-only bars and the text above any named `Screen` alongside the
  stills, resolving `idstring` through the title's own string table.
  Confidence 90; see `docs/formats/race-setup.md`'s "The stat panel is read
  and drawn" section for the full evidence and the one open item it left -
  a German label (`RUNDEN-REKORD`) running into its neighbour's column,
  unmeasured against real hardware.
- **The entry list does not scroll, and the packed roster is uncounted.**
  Pure's `<Menu allocate="16">` is a capacity; `oag_ui_screens::picker`'s
  `entry_rows` draws one row per entry and a source offering more than fit
  runs off the bottom rather than paging. The base disc offers 8 circuits and
  10 teams, comfortably inside 16. **Its seven DLC packs add more and were not
  counted**: this checkout has no `data/keys/pure-dlc-keys.txt`, so the packs
  do not decrypt here and the roster stays at the base numbers. Count it on a
  checkout that has the keys before assuming 16 is enough.
- **Whether Pulse's track mesh is the circuit ribbon or something else** has
  not been looked at - the files are 10-23 KB against a real `track.vex`'s
  4.25 MB, so they are purpose-built, but nobody has decoded one and looked.
- **Camera framing for the *mesh* previews is still unmeasured** on Pulse PSP
  and PS2. HD states its own (`OriginX="1220" OriginY="412" nearZ="1.0"
  z="-24.0" RotX="0.4" RotY="-0.5"` on `ShipModel`); the PSP titles author no
  `<Model>` element, so their framing lives in the screen class, and
  `oag_game::preview::orbit_for` is this project's own pick. Pure no longer
  needs an answer here.
- **`Top->Ship` is resolved, on Pulse PSP.** Not a static screen widget:
  `TrackSelection_ApplySelection` resolves it as a node inside *each track's
  own* dynamically-created preview scene, replaced every time the selection
  changes, fed the `%s\FE\%s.vex` mesh directly. Confidence 78, static
  decompilation only.

## Next Steps

1. **Done, 2026-09-27**: the stat bars now read out of the same per-entity
   `screen.xml` the stills come from - see the Open item above and
   `docs/formats/race-setup.md`.
2. Give Pure's `Layout` its own `panel`/`preview` rects off its `<Viewport>`,
   before something starts drawing from the borrowed ones.
3. Thread a race mode into `oag_game::preview::slideshow` so `screen_m.xml`
   and `screen_tt.xml` are reachable.
4. Settle `TeamSelection_ApplySelection`'s Phantom-model trigger past
   confidence 60: break at `0x08951bd0`, read `+0xe1`/`+0xe2` at the moment
   the model would visibly change, on a save further into the campaign than a
   fresh profile.
5. Do the Pulse-side `Skin.vex` skin-swap check: whether `Skin_ApplyToModel`'s
   four-slot texture swap
   (`docs/ghidra/functions/psp-pulse-usa/ship-skin.md`) applies to
   `ship_FE.vex` the way it does to the in-race hull.
6. Check whether Pure's own real hardware/PPSSPP also overlaps
   `RUNDEN-REKORD`/`HÖHE` in German, or wraps/shrinks somehow this build does
   not reproduce - a capture with the language set to German, which none of
   this thread's own captures were.

## The two checks that would have caught this

**Sprite or mesh?** Raise PPSSPP's internal resolution and look again. It
upscales geometry *and* render targets, but not a fixed-size source image - so
a preview that stays blurry at 5x is a sprite, whatever it looks like at 1x.
Per-entity distinctness proves nothing: a per-team *sprite* is distinct per
team too, and that was the whole basis of the wrong reading. The art here is
also anti-aliased with soft gradients, which PSP hardware cannot produce.

**Undeclared, or declared somewhere else?** Pure's colour globals were
recorded as "seventeen usages, zero declarations - confirmed by dumping the
whole expanded file" and pixel-sampled off captures at confidence 60-65. The
file dumped was the front-end root. Pure's front end is *skinnable*, and its
plugin definition activates a second `PI_Skin` at `Data\Skins\Default` whose
own `Skin.xml` declares all 41 of them. Three of the four sampled values were
wrong. **Before recording a name as undeclared on a title, check whether that
title activates a style skin.**

## From the HANDOVER.md index (moved 2026-09-25)

the track and craft previews are **rendered 3D meshes** on Pulse PSP, Pulse PS2 and HD, with verified paths rather than inferred ones: `<location>\FE\{forward,reverse}.vex` for a circuit and `<team>\<variant>_FE.vex` for a craft, with HD authoring both as explicit `<Model>` widgets. **Pure is the exception and previews with a pre-rendered 256x128 still**, one of three its own `<location>\screen.xml` names, cycling `Info -> Side -> Top` at two seconds a step - a PPSSPP texture dump matches the drawn pixels to `FEData.wad` entries at RMSE 0 (2026-09-10), correcting nine hours of `Ship.vex`/`track.vex` mesh wiring that was a plausible-looking stand-in. Both of Pure's screens now draw their entry list and their preview off the disc. Open: its stat bars (authored in the same per-entity `screen.xml`, unread), its `Layout` still borrowing Pulse's `panel`/`preview` rects, `screen_m.xml`/`screen_tt.xml` needing a race-mode parameter, and the *camera* the PSP titles frame the mesh previews with. See [race-setup.md](../../docs/formats/race-setup.md)
