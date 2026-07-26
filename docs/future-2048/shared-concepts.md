# Shared concepts across the lineage

The project's premise is that the **second title should be cheaper than the
first**. That only holds if reusable things are recognised as reusable while
they are being built, rather than being retrofitted later.

This page tracks what appears to carry forward. It is speculative by nature;
entries get promoted to real documentation as evidence arrives.

## Titles

| Title | Platform | Year | Status |
| --- | --- | --- | --- |
| Wipeout Pure | PSP | 2005 | Format ancestor, image available |
| Wipeout Pulse | PSP | 2007 | Primary target |
| Wipeout Pulse | PS2 | 2009 | Cross-validation, image available |
| Wipeout HD / Fury | PS3 | 2008/09 | Not yet examined |
| Wipeout 2048 | Vita | 2012 | Long-term goal, package available |
| Omega Collection | PS4 | 2017 | If feasible |

## Confirmed shared

### WAD container

The same header shape appears in Pure PSP, Pulse PSP and Pulse PS2:

```
u32 version = 1
u32 entry_count
entry[] { u32 name_hash; u32 offset; u32 size; u32 size_uncompressed; }
```

This is the first concrete evidence for the whole premise. See
[formats/wad.md](../formats/wad.md).

Confidence: **85** that the format is genuinely shared. The header shape and
entry counts match across all three; the record layout has been validated
against Pulse PSP only.

**Worth checking next:** whether HD and 2048 use it too. 2048 is a Vita title
from a different studio generation, so probably not, but the check is cheap and
the answer shapes how much of the asset pipeline is reusable.

### Directory naming

Pure and Pulse PSP both use `Data.wad`, `FE.wad` and `FEData.wad` in
`PSP_GAME/USRDIR/`. The `FE` (front end) abbreviation survived at least two
years and one title.

## Suspected shared

Untested, listed so they get checked rather than rediscovered.

| Concept | Reasoning |
| --- | --- |
| Ship handling model | The series' handling is its identity. Wholesale replacement between adjacent titles is unlikely. |
| Track section / spline representation | Track authoring tools tend to outlive individual titles. |
| Weapon set and behaviour | Largely consistent across the series by observation. |
| Speed class scaling | Vector / Flash / Rapier / Phantom appear throughout. |
| HUD structure | Visually similar across titles. |
| Team stat tables | The same teams with the same relative characteristics. |

## Divergences to expect

| Area | Why |
| --- | --- |
| Rendering | PSP, PS2, PS3, Vita and PS4 have nothing in common here. |
| Audio | Platform middleware differs completely: ATRAC3, SCREAM, and whatever HD uses. |
| Networking | Ad-hoc wireless, PSN and Vita near are different systems. |
| Asset compression | Likely retuned per platform. |

## Design implications

Three rules follow, and all three cost nothing now but a lot later:

1. **Do not hard-code Pulse-specific constants** where a table would do. Team
   stats, speed classes and weapon parameters should be data, even while there
   is only one title's worth of data.
2. **Keep the format layer separate from the runtime layer.** `oag-formats`
   decodes; `oag-assets` normalises. A new title adds decoders without touching
   the runtime. See [asset pipeline](../architecture/asset-pipeline.md).
3. **Document formats generically.** Say "the WAD container as used by Pulse
   PSP", not "the Pulse WAD format". The name shapes what the next person
   assumes.

## Wipeout 2048 notes

Not yet examined. Available as a Vita package in `~/Downloads`, base plus v1.04
patch, for both EUR and USA.

Known at a distance: developed by Studio Liverpool for Vita launch; uses the
same team and track lineage; the last title before the studio closed. Whether
any code or format DNA survives from the PSP era is entirely unknown, and is the
single most valuable open question for the project's long-term premise.

Examining it is an M7 task. Doing it earlier would be a distraction, but the
answer would materially change how much abstraction is worth building now, so it
may be worth a cheap look sooner rather than later.
