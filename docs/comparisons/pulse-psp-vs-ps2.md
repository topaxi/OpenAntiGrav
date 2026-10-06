# Pulse: PSP vs PS2

> **The region confound applies to everything on this page.** Our PSP copy is US
> and our PS2 copy is EU. No difference below has been shown to be caused by the
> platform rather than the region.

Sources: [PSP disc layout](../psp/pulse-disc-layout.md),
[PS2 disc layout](../ps2/pulse-disc-layout.md).

## At a glance

| | PSP | PS2 |
| --- | --- | --- |
| Serial | UCUS-98712 (US) | SCES-54748 (EU) |
| Volume date | 2008-01-04 | 2009-05-15 |
| Publisher field | SCEE | SCEE |
| Executable | `BOOT.BIN`, 3.85 MiB ELF | `SCES_547.48`, 1.99 MiB ELF |
| Content size | 354 MiB | ~900 MiB |
| Media capacity | 436 MiB UMD | 3.6 GiB DVD, 76% padding |

The PS2 release came 16 months later. That gap is large enough that gameplay
changes between them are plausible and should be checked rather than assumed
absent.

## Differences observed

| Area | PSP | PS2 | Class |
| --- | --- | --- | --- |
| Archive names | `Data`, `FE`, `FEData`, `BEData` | `WADS2`, `WADSP`, `PS2MUSIC`, `PRERACE` | Unclassified |
| Archive format | WAD container | `WADS2`/`WADSP` same container; `PS2MUSIC`/`PRERACE` different | Unclassified |
| Audio | ATRAC3, `libmp3.prx` | SCREAM engine, `LIBSD.IRX` | Platform limitation |
| Video | 1 PMF (icon animation) | 2 PSS + 2 IPF, at 512 and 640 wide (PAL/NTSC cuts, both decoded now; `.IPF` backdrop is not) | Platform limitation |
| Executable size | 3.85 MiB | 1.99 MiB | Unclassified |
| Network modules | 3 PRX, no network stack | none | Unclassified |
| Content volume | 354 MiB | ~900 MiB | Platform limitation (likely) |

### Executable size

The PS2 executable is roughly half the PSP's, which is the opposite of what the
platform's greater capability would suggest.

Plausible explanations, none tested: more code lives in the IOP modules; the PS2
build links less statically; or the PSP build embeds data the PS2 keeps in its
archives.

Not classified because none of it has been checked.

### Archive organisation

`WADS2.WAD` and `WADSP.WAD` share the PSP's [WAD container](../formats/wad.md)
header shape, so the container survived the port.

`PS2MUSIC.WAD` and `PRERACE.WAD` do not. Different headers, and in
`PRERACE.WAD`'s case an entropy of 0.084, meaning 85 MiB of near-uniform bytes.

**The archive layout is the only difference `oag-game` had to be taught.** It now
finds a source's bulk archive by name - `Data.wad` or `WADS2.WAD` - and runs the
same race off either disc with the same `--track` and `--team`, because the entry
names, the handling schema and the mesh and collision decoders are already shared.
`oag_assets::Layout` is where that lives, and the PS2 archives are found by
name rather than by path because they sit in a directory named after the disc's
serial. See [oag-game](../tools/oag-game.md#either-disc) for what is still worse on
the PS2 source - untextured track art (ships now draw correctly), and the
looping menu backdrop having no picture (the intro itself now does).

Whether these are new formats or the same data reorganised is unknown.

### Does the PS2 port show anything of Wipeout HD? (2026-10-06)

PS2 Pulse (volume date 2009-05-15) shipped after Wipeout HD (2008), so the
question is whether HD-era code or data reached it. A string census of four
executables says **no HD format reached the PS2 disc, and the code the two
share is not yet attributable**. Raw tables, commands and string lists:
`data/scratch/ps2-lineage/` (a scripted `strings`/`comm` sweep, not a code
read).

- **No HD asset format on the PS2.** No PSARC, `.gtf` or `.rcsmodel` on its
  disc or in its executable's strings; it ships WADs, `.vex` and `PI_` plugin
  XML like the PSP (both executables carry `PI_` plugin strings: PSP 35, PS2
  21, Pure 20, HD 52).
- **Unique strings (length 8 or more) shared:** PS2 and PSP 2,438; PS2 and HD
  2,152. **95 are in PS2 and HD but not in the PSP executable**, 22 of them
  also in Pure. They are almost all engine log lines (`Entering main game
  loop`, `Create plugins`, the particle affectors' `PATurbulence ...`, the
  AI's `Network has %d neurons and %d connections`), mode names
  (`Multiplayer_Head2Head`, `Time_Trial`) and string ids (`IG_HUD_3RD`).
- **HD itself carries the PSP project's path**:
  `Z:\WipeoutPSP\Art_Resources\Psys\Tex\psysed_default_glow.tga` is in both
  the PS2 and the HD executable (not in the PSP one's strings). HD was built
  from the PSP Pulse source tree, so a string shared by PS2 and HD alone does
  not show the PS2 took it from HD.
- **The two readings the census cannot separate:** (a) the PS2 port was cut
  from the shared tree after HD's changes landed in it, or (b) the PSP
  release build compiled its logging out and the PS2 and HD builds did not,
  so the 95 lines were in the PSP's source all along. The log-line character
  of most of them leans towards (b). **Confidence 40** that the PS2 port
  carries no HD-era code beyond what the shared tree already had; settling it
  needs a function-level comparison in Ghidra (one shared subsystem, such as
  the particle affectors, read on all three executables).

## Questions to resolve

| Question | Why it matters |
| --- | --- |
| Are the handling constants identical? | Decides whether [ADR-0004](../architecture/adr/0004-asset-pipeline.md)'s "gameplay is identical across asset sets" holds. |
| Are the track splines identical? | Same. A changed spline is a changed track. |
| Did anything change in 16 months? | Weapon balance, AI, speed classes. The gap is long enough for real changes. |
| Is `WADS2.WAD` a merge of the PSP's four archives? | 360 MiB versus 324 MiB combined, and 7,200 entries versus 1,465. The entry count difference is far larger than the size difference, which suggests reorganisation rather than a straight merge. |

The last one is the most tractable and the most immediately useful: it would
confirm whether one WAD parser really does serve both platforms.

## Method

When comparing, prefer evidence in this order:

1. **Runtime traces of the same scenario on both.** Strongest, and settles
   gameplay questions directly.
2. **Corresponding functions in both binaries.** Strong for logic.
3. **Corresponding data tables in both binaries.** Strong for constants.
4. **File listings and headers.** Weakest, and all we currently have.

Everything on this page is currently level 4.
