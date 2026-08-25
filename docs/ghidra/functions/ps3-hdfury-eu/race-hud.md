# The race HUD, and what Zone does without a mode manager

This page starts from the asymmetry [mode-manager.md](mode-manager.md) turned
up - four single-player modes have a race manager and **no** mode manager, and
they are exactly the ones that are not conventional races: Detonator, Free Play,
Nitro and Zone. Reading `SPZone_RaceManager`'s constructor to find out what it
does instead lands one name and a structural answer, and **does not answer the
question it was opened for**. That is recorded below rather than papered over.

Read [memory.md](memory.md) first for the per-function TOC defect. Everything
here is below `0x32d5e0`, where Ghidra's TOC is the right one.

## The name

| Address | Name | Confidence |
| --- | --- | --- |
| `0x00097be8` | `Hud_LoadDefinition` | 80 |

`Hud_LoadDefinition(owner, xmlPath, flags)`. Every race manager constructor
calls it, once per HUD file that mode needs, and **the second argument is always
a `*_HUD.xml` path**. Two call sites were resolved by hand through the TOC rather
than trusting a label:

| Call site | In | Slot | Path |
| --- | --- | --- | --- |
| `0x00076ad4` | `SPZone_RaceManager` ctor | `0x008a7240` | `Data\XML\wo3_HUD\Zone_HUD.xml` |
| `0x000618bc` | `SPArcade_RaceManager` ctor | `0x008a6b64` | `Data\XML\wo3_HUD\SplitScreen_hud\Vert_Sp…` |

It has **more than 40 call sites**, and every one is inside a race-manager
constructor or the helper at `0x00048ac8` that `MPArcade`'s constructor calls.
`SPArcade` alone calls it nine times per constructor, which is what a mode with
split-screen and three HUD skins would need.

80 rather than higher: two arguments were resolved, not forty, so "always a HUD
path" is an inference from the string set plus two checks. A generic XML loader
that happens only to be used for HUDs from race managers is not excluded, and
one call site passing a non-HUD path would rename this.

## The HUD is skinned three ways

The disc carries **31 `*_HUD.xml` paths**, and they fall into three sets:

```
Data\XML\Arcade_HUD.xml
Data\XML\wo3_HUD\Arcade_HUD.xml
Data\XML\2097_HUD\Arcade_HUD.xml
```

The same three-way split appears for `SpeedLap_HUD.xml` and
`TimeTrial_HUD.xml`. `wo3` and `2097` are Wipeout 3 and Wipeout 2097 - the
retro HUD skins, with the bare `Data\XML\` path as the default. Others have no
skin variants at all: `Elimination_HUD.xml`, `MPTag_HUD.xml`,
`Duel_HUD\Duel_HUD.xml`, and `wo3_HUD\Zone_HUD.xml`, which exists only in the
`wo3` directory.

Nothing here reads how a skin is chosen. The paths are literals in the
constructors, so the choice is made by *which call runs*, not by composing a
directory name at runtime - which means the selection logic is in the
constructor's control flow and is readable, just unread.

**The user recalls HD offering a menu option to pick a retro HUD skin**
(2026-08-25, from memory of playing the original rather than a fresh capture -
so this is testimony to record and corroborate later, not a measurement). That
fits the three-way split above: a **settings choice** rather than a
per-mode/per-circuit rule would explain why every mode that has a skin variant
at all has exactly the same three (`Data\XML\`, `wo3_HUD\`, `2097_HUD\`), and
why `Elimination_HUD.xml`, `MPTag_HUD.xml` and `Duel_HUD\Duel_HUD.xml` have
none - a global setting applies uniformly, a per-mode rule would need a reason
those three are exempt. Worth checking against a settings/options XML's own
key names before chasing the constructor control flow by hand.

## What `SPZone_RaceManager` actually adds

`0x00076940` chains to `RaceManager_Construct` at `0x00076984`, installs its own
vtable from TOC slot `0x008a6b8c`, and then zeroes a block of fields:

```
000769b8: sth r0,0x2dd8(r30)      six u16 counters at
000769bc: sth r0,0x0(r10)         +0x2dd8, +0x2dda, +0x2ddc,
000769c0: sth r0,0x0(r8)          +0x2dde, +0x2de0, +0x2de2
000769c4: sth r0,0x0(r7)
000769c8: sth r0,0x0(r6)
000769cc: sth r0,0x0(r5)
000769d0: stw r4,0x2de4(r30)      one u32
000769d4..000769fc                ten bytes at +0x2de8
```

**The block starts exactly where the base class stops.**
[race-manager.md](race-manager.md) records `RaceManager`'s own highest field
write as `0x2dd4`; Zone's first is `0x2dd8`. The two agree to the field, which is
the check worth having on both readings at once.

Six `u16` counters is suggestive next to the profile field names the disc
carries - `uNumZones`, `uNumZoneLaps`, `uNumPerfectZones`, `uNumImperfectZones`,
`uTimeInZoneMode` - but nothing observed connects the offsets to those names, and
five names against six fields does not line up on its own. **Not recorded as a
mapping.**

## The question this page did not answer

**What a mode manager does that Zone gets by without is still open.** The
constructor difference is a field block and a HUD load, and both are things every
race manager has. Nothing read here explains why Arcade, Elimination, TimeTrial
and Tournament each need a second class and Zone does not.

The next place to look is not the constructor but the **vtable**. Zone installs
its own at TOC slot `0x008a6b8c`; `RaceManager`'s is at `0x008628e0`. The slots
that differ between them are precisely the virtuals Zone overrides, and that is
the shortest path from here to behaviour - it needs no strings and no TOC
arithmetic, just a diff of two tables.

## Not recorded

- **The six `u16` fields' meanings.** See above; the profile names are a lead,
  not a mapping.
- **How a HUD skin is selected.** The three paths are literals in different
  branches; which branch runs was not read.
- **`0x003238f8`**, called immediately before the HUD load in both `SPZone`'s
  constructor and the `ModeManager` new-site. Clearly a shared init, unread.
- **`0x00048ac8`**, the helper `MPArcade`'s constructor calls that itself calls
  `Hud_LoadDefinition` four times.
