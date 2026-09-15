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
| Wipeout 2048 | Vita | 2012 | Package examined; PSARC, HD's asset-extension family and its whole team roster carry over. Craft/track spawn not yet wired. |
| Omega Collection | PS4 | 2017 | RE target as of 2026-09-15 (x86-64, no custom Ghidra processor module needed); no gameplay work planned |

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

**HD does, and so does 2048.** 2048's `data.psarc` (1.6 GiB, EU base package)
opens with `oag_assets::psarc::Archive::open` unmodified: version 1.4, flags 1,
against HD's version 1.3, flags 3 - neither value is asserted by the reader, so
both open the same way. Confidence **90**: measured directly against a real
package, on the same terms as the WAD finding above, not yet corroborated by a
second 2048 archive (the patch's own `data1.psarc`/`data2.psarc` are unread).

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

Examined 2026-08-26, against the EU base package's `data.psarc` (1.6 GiB,
18,430 entries). Developed by Studio Liverpool for Vita launch; the last title
before the studio closed.

**The lineage question this section used to call "entirely unknown" has an
answer, and it is HD/Fury, not the PSP/PS2 pair.** The asset tree is HD's, not
Pulse's: `.rcsmodel`/`.rcsmaterial`/`.rcsskeleton`/`.rcsanimclip`, `.pob`,
`.pvs`/`.probes`, `.envsettings`, `.bnk` are all present, and 2048 ships all
fourteen HD teams verbatim under `data/art/published/hdships/<Team>/`
(`Ship.vex`, `handlingstats.xml`, the `_c1`/`_n1` Fury variants and all)
alongside its own five-team native roster (`AG Systems`, `Auricom`, `Feisar`,
`Piranha`, `Qirex`) and ten native circuits. Confidence **85**: strong,
convergent evidence from the shipped file tree and naming, not yet corroborated
by reading any code.

**Not everything carries over unchanged, though - two of 2048's own binary
formats have diverged from HD's**, each confirmed against the real package:
the track spline's per-point record shrank from 112 to 96 bytes (confirmed
exactly against all fourteen shipped tracks), and the external track-geometry
container is not HD's `.rcsmodel` version at all (a different, unrecovered
header). Neither has its own format page yet - one is owed once a decoder
exists for either.

Handling-stats XML, the engine-wide `<Global>` block and the PSARC container
itself are all unchanged from HD, so the "second title cheaper" premise holds
for those three outright; the two divergent formats are exactly the kind of
finding this page exists to track, not a reason to doubt it.
