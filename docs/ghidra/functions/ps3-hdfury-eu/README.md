# Wipeout HD / Fury PS3 functions

Functions from `PS3_GAME/USRDIR/EBOOT.elf` (Wipeout HD Fury, PS3, BCES-00664),
language `PowerPC:BE:64:A2ALT-32addr`, big endian.

**The imported file does not exist on the disc.** A PS3 `EBOOT.BIN` is an
encrypted SELF; what gets analysed is the ELF `rpcs3 --decrypt` writes beside
it. That makes this the only binary in this tree whose program name differs
from the file the disc ships, and the reason
`scripts/apply-ghidra-names.py`'s `BINARY_PROGRAMS` carries an `.elf` entry for
it. Reproduce the whole import with:

```sh
scripts/import-ps3-eboot.sh     # close Ghidra first; it needs the project lock
```

Setup, the compiler-spec fix this needs, and the traps are in
[toolchain.md](../../../reverse-engineering/toolchain.md#ps3).

## What this binary is, and is not

**Not a reverse-engineering target.** Wipeout HD / Fury is a
[later title](../../../overview/goals.md#scope) and no milestone is open on it.
What this directory exists for is the lineage question: HD is the first title
after Pulse, on completely different hardware, and whether its subsystems are
recognisably the same shapes is worth knowing cheaply before anyone plans work
on it. Names here are opportunistic, and any claim about *Pulse* still has to
be proved against Pulse's own binaries.

**The asset half of that question is answered, and this directory is no longer
where it is answered.** As of 2026-08-17 the disc's archives read and the survey
is [hd-status](../../../formats/hd-status.md): HD ships Pulse's `.vex` at
version 6 with Pulse's own class IDs, byte-swapped, and ten of its sixteen
environments are a Pulse or Pure circuit's spline. Where the executable is
still the only source is everything *behavioural* - what HD does with any of
it - and three things learned here so far carry: gameplay lives in `EBOOT.elf`
(`Collision.cpp`, `RaceManager.cpp` and `ModeManager.cpp` are all named from it)
rather than in the closed `DFEngine.sprx`; the C++ base class stores its own
`__FILE__` at object offset `0x30`, which attributes whole classes cheaply; and
**the renderer is in `EBOOT.elf` too** - it drives libgcm itself and imports one
single symbol from `DFEngine.sprx`. See [renderer.md](renderer.md).

**A second lineage question, this binary as the *known* side of the
comparison rather than the unknown one, is also answered.** WipEout 2048
(Vita) turns out to be this codebase retargeted, not a fresh Pulse-lineage
build - confirmed by literal `.cpp`/asset-path string matches against
`vita-2048-eu-v104`. See
[`vita-2048-eu-v104/README.md`](../vita-2048-eu-v104/README.md#the-lineage-question-is-answered-confirmed).

Two structural facts to expect, both different from every other binary here:

- **PPC64 with an OPD.** A function pointer is a descriptor pair
  `{code address, TOC value}` in the data segment, not a code address. The
  entry point `0x870530` is an OPD entry resolving to `func=0x00010230`,
  `toc=0x008ad4d8`. `AssignPs3R2FromOpd.java` in the script pack is what makes
  the decompiler read TOC-relative data correctly.
- **Stripped of symbols, but not of library names.** There is no symbol table
  and the section-name table is gone, so `readelf` shows 156 unnamed sections.
  The 122 `cell*`/`sce*` identifiers that survive are the module and export
  names, which `AnalyzePs3Binary.java` resolves against its NID database - so
  imports get real names and everything else starts as `FUN_`.

## Pages

- [memory.md](memory.md) - the allocator, its heap, the mutex around both, and
  **the per-function TOC defect this database has**. Read it before trusting
  any data or string reference in this program.
- [game-boot.md](game-boot.md) - `Game_Main` and the `GameRoot`/`SpeechManager`/
  `SoundManager`/`FrontendRoot`/`MusicManager` construction chain, cross-verified
  literally (matching `.cpp` tags, not just role) against `vita-2048-eu-v104`'s
  own boot chain.
- [weapons.md](weapons.md) - `Repulser`/`Rocket`/`RocketManager`/
  `WeaponExplosions` construct, cross-referenced against `psp-pulse-usa`'s own
  weapon docs, plus two owning classes (`WeaponManager`, `Plasma`) found but
  not fully read, and the one trap in `batch_string_anchor_report`'s
  per-binary behaviour.
- [rocket-trail.md](rocket-trail.md) - the Rocket's smoke ribbon: the PPU-side
  `RibbonEffects` pool manager, the ribbon node law (life 1.85 s, half-width
  0.4 to 2.0, the opacity ramp TGA read two bytes late), the three-fin
  geometry and the material, all matched against a live RPCS3 dump.
- [plasma.md](plasma.md) - `PlasmaManager_Update`, `Plasma_Update`,
  `Plasma_CheckShipHit` and the `WeaponExplosions` update/draw pair: the
  1.0 s hardcoded wind-up, the 10.0 s reap, the 6.0 probe, the +-6.0 hit
  corridor, `WeaponStats_ParsePlasma`'s eleven attributes with `charge_time`
  unread, and the explosion's 1.7/1.3/1.3 s ramps - the PS3 half of the
  Pulse/HD/Omega comparison.
- [cannon.md](cannon.md) - `CannonBullet`: `hd_muzzleflash` flashed at the
  craft's `cannon_flash` locators for the round's first `0.1` s rather than
  riding the round, the 0.25-wide full-segment bolt, and the negative search
  that leaves `WO_CANNON_MUZZLEFLASH`/`WO_CANNON_HOTSPOT` with no trigger.
- [collision.md](collision.md) - `Collision.cpp`: the arena, the class, and the
  MeshAABB narrowphase.
- [race-hud.md](race-hud.md) - the per-mode HUD definitions, their three retro
  skins, and what `SPZone` adds on top of the base race manager.
- [hud-sight.md](hud-sight.md) - the lock-on reticle's own widget bind and its
  two separate per-tick updates (Missile, LeachBeam), the shared `0.5` s hold
  time that disagrees with the PSP's `0.8`, and the LeachBeam's four-widget
  reveal that is not a simple all-or-nothing.
- [hud-readouts.md](hud-readouts.md) - `Hud_Update` and the three per-tick
  updates behind the shield hexagon (its `0x1664FF` fill cropped from the top,
  the 20 % and post-hit red flash), the lap arc and the position arc, whose
  yellow is the atlas's own.
- [mode-manager.md](mode-manager.md) - `ModeManager.cpp`: the sibling mode
  hierarchy, and the one place a C++ constructor pair could be told apart.
- [race-manager.md](race-manager.md) - `RaceManager.cpp`: the singleton holder
  and the base of the race-mode hierarchy.
- [renderer.md](renderer.md) - the rendering layer: that it is in this binary
  rather than `DFEngine.sprx`, the GCM device bring-up, `RenderManager`, and
  the finding that HD keeps Pulse's one-translation-unit-per-`.vex`-class
  importer layout. Also the **out-of-Ghidra** way around the TOC defect,
  [`scripts/ps3-toc.py`](../../../../scripts/ps3-toc.py).
- [billboards.md](billboards.md) - `TrackStartup_Load`: how a `<Billboard>`
  becomes a 9-entry slot array indexed by its own `Num`, that it instantiates
  rather than textures existing geometry, and the one slot the engine
  overrides regardless of what its manifest authored.
- [xfade.md](xfade.md) - the crossfade system HD drives a craft's engine
  sound with: `XFadeSystem_AddCrossFader` and its update path, the ship-side
  cache and instance, and `Ship_UpdateEngineCrossfade`, the per-tick writer of
  four input channels (three of four terms recovered, one unidentified).
- [sound.md](sound.md) - the SCREAM grain-opcode dispatch table
  (`0x00927614`), the located `0x05`/`0x06`/`0x08` "play/stop a child cue"
  handlers, guard `0x22`'s three-way variable-versus-immediate skip (the
  mechanism `.COLLISIONS`' severity and ship/wall split runs on, still
  undecoded past the opcode itself), and `0x19`'s alternate-selection
  handler - a random pick that never repeats the previous one.
- [shadow-model-maps.md](shadow-model-maps.md) - `Job RenderModelShadowMaps`'s
  run function and the per-ship matrix builder behind it: one map per ship,
  looking from 70 units up `Lighting.Sun direction` at the ship, an
  orthographic box fitted to the ship's own bbox with near 1 / far 140, and
  the bias form that becomes `shadowMatrix`. Plus the lazily-initialised
  env-settings block every `Lighting.*` key registers into.
- [ship-sun-occlusion.md](ship-sun-occlusion.md) - `Job RenderShips` is two
  passes, and the first renders the track within ten units of each craft from
  the sun through `SunOcclusionLightmap`/`SunOcclusionVertex` into a second
  per-ship map; the hull's `ShadowMap` variant gates its sun by that map times
  its own depth map. Also: the ship map pool and its size table, and that the
  model shadow map is depth rendered colour-masked-off, not coverage.
- [shadow-stencilvolume.md](shadow-stencilvolume.md) - the `LiveStencilShadow`
  shader technique's registration and its three named constants, the
  per-model flag bit that builds a fixed-named `shadow.stencilvolume` sibling
  path and loads it through a hashed resource cache, the draw call itself (a
  textbook two-sided depth-fail stencil shadow volume, its RSX register
  identities cross-checked against a vendored `rpcs3`'s own source), and the
  record shape that loader parses - which rhymes with, but does not
  byte-match, Pulse's `DynamicShadowOccluder` `.vex` payload.
- [zone-advance.md](zone-advance.md) - `Zone_UpdateCraftClass`, the per-craft,
  per-tick Zone-mode ladder-rung advance both its callers were read for; also
  the negative that rules it out as the still-missing writer of
  `craftArray[n]->+0x640` (see `zone-effectsettings-loader.md`).
- [endrace-results-grid.md](endrace-results-grid.md) - `EndRace Results`'
  standings grid laid out in code per mode: the race family's 443-tall frame,
  rows at `96 + 45 r`, columns at 40/200/545, the footer block hidden.
- [menu-sounds.md](menu-sounds.md) - which front-end cue HD plays for which event, and the timeline each one is.
- [menu-blocks.md](menu-blocks.md) - `Block_Item.cpp`, the box behind every
  `<HorizMenu>` tab, `<List>` row and `<aVertMenu>` entry: a nine-patch frame
  off `file2.gtf`, a fill whose alpha is a swatch texel (drawn twice, the second
  pass opaque on the HD style only, which is the whole HD-versus-Fury
  transparency difference), the strip's `ItemWidth` default of 298 and its
  selected-entry bonus of 70, and the one-sixth-per-frame easing all three
  menu classes grow with.
- [shield.md](shield.md) - `Shield.cpp`: the fired pickup's shell, read and
  cross-checked field for field against `psp-pulse-usa/shield-pickup.md` - the
  same law (colour lerp, swell lerp, 60 Hz substep, cockpit-vs-shell camera
  branch), two colours that moved (the hit flash is amber where Pulse's is
  cyan) and one that is parameterized rather than hardcoded (the steady-state
  target, unresolved - ported as Pulse's white, chosen not measured).
- [menu-backdrop-scene.md](menu-backdrop-scene.md) - `BackgroundAnim_Item.cpp`, the HD
  style's backdrop and the one Omega's menus draw: the scene `FrontEndScene_HD_ATG.vex` flown
  through its own animated camera into a white target, filtered by `FEBackgroundAnim_fp`'s
  Roberts cross (edge and fill levels) and blurred, the `ScreenSetting` row layout and its
  0.97/0.03 easing, the 60 s clock, the 57.3 degree field of view.
- [menu-backdrop.md](menu-backdrop.md) - `BackgroundAnimFury_Item.cpp`, the
  Fury menu's backdrop: one of nineteen `.points2` ship point clouds (the record
  format, measured on all nineteen), the camera paths and colours in
  `fury.envsettings`, the path picker, the `RadioHead2` vertex program read as
  arithmetic (depth-of-field, fog, the travelling colour ramp), the procedural
  six-mip sprite, the feedback-trail post passes, and the per-screen tint. Also
  where `ShaderRegistry_RegisterPair` closes renderer.md's unpaired-shader
  question.
- [race-campaign.md](race-campaign.md) - `PI_Cell`'s own attribute table
  (`0x008ae898`), which puts `NitroElimNovice`/`Skilled`/`Elite` in the class's
  own field list rather than an unrelated subsystem; the `HARD(ELITE)` save
  migration that measures the rung-name equivalence; and why the actual medal
  comparison consumer was not found (reflection-driven binder, no string
  cross-reference to chase).

Add a row to [`names.tsv`](names.tsv) and the page it cites in the same change:
`scripts/apply-ghidra-names.py` refuses a row whose address and name do not
both still appear on the page named in its last column.
