# Mag-floor gfx/sfx: two real `.vex` effect models, trigger read from the tail of `Ship_CastHoverProbes`

| | |
| --- | --- |
| **Binary** | `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse, PSP, UCUS-98712) |
| **Subsystem** | ship scene-graph effects, resource loading |
| **Related** | [`engine.md`](engine.md#the-tail-of-ship_casthoverprobes-fires-the-mag-floor-gfxsfx-2026-09-02) (the trigger), [`vex.md`](../../../formats/vex.md) (`CLASS_MAG_FLOOR_COLLISION`, the *collision* mesh - a different thing from this page), [`track.md`](../../../formats/track.md) (the track's own static magstrip texture/material, also a different thing - closed as unanimated in the project's handover history) |

## Summary

Two ordinary `.vex` models ship on Pulse's disc and preload every race:
`Data\visual_effects\MagEffect1.vex` and `Data\visual_effects\MagEffect2.vex`,
separate from both the collision mesh (`Mag Floor Collision`, class `0x3e6`)
and the track's own glowing strip texture. Separately, the tail of
`Ship_CastHoverProbes` edge-detects the mag-floor contact bool it already
computes and calls `Ship_MagFloorEnter`/`Ship_MagFloorExit`
([engine.md](engine.md#the-tail-of-ship_casthoverprobes-fires-the-mag-floor-gfxsfx-2026-09-02)),
which show/hide two child scene nodes on rising/falling edge. **That the two
assets and the two shown/hidden nodes are the same two is inferred from
proximity and shape, not located** - see [The trigger](#the-trigger) for
exactly how far the evidence goes.

**Found starting from an `AskUserQuestion` to the maintainer** (they play these
games): both a visible spark/glow below the craft and an audible hum were
reported. This page recovers the gfx half in full down to the trigger; the sfx
half is open - see below.

## The two assets are real and on the disc

Both filenames appear twice in `BOOT.BIN`'s data segment: once mixed-case
(`MagEffect1.vex`/`MagEffect2.vex`, in a `"file"`-tagged resource block shared
with every weapon's `.vex`/sound-cue declarations, `0x08a7bfa8`/`0x08a7bfcc`)
and once lowercase, immediately beside the per-race `Data\Psys\WO_*.POB`
preload list (`0x08a7c39c`/`0x08a7c3c0`) - so they preload every race, the same
as the weapon effects, regardless of whether the track has a magstrip.

`oag-wad hash` on both path spellings gives `ba9996ee` (`MagEffect1.vex`) and
`fd39ec3e` (`MagEffect2.vex`); both hashes are real entries in
`pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad` (index 1079, 15312 bytes and index
1080, 4976 bytes - adjacent, consistent with the pair being authored and
packed together). Extracted and parsed with the existing `.vex` reader:

```
Data\visual_effects\MagEffect1.vex: 1 mesh, 114 vertices, 112 triangles, radius 10.95
Data\visual_effects\MagEffect2.vex: 1 mesh, 103 vertices, 101 triangles, radius 10.53
```

Both files are ordinary version-6 VEXX, authored from
`Z:/WipeoutPSP/X2/Data/visual_effects/...` (the same dev-path convention every
other authored `.vex` on this disc carries), not generated or stray data.
`just view ... --mesh 'Data\visual_effects\MagEffect1.vex' --screenshot` and
the same for `MagEffect2.vex` both render: thin, additive-looking
purple/blue streak geometry, splayed outward from a point - `MagEffect1` a
narrower cluster of three or four spikes, `MagEffect2` a wider symmetric fan.
Matches the maintainer's "spark/glow below the craft" description well enough
to be the effect, though the identification is still via the trigger's
structural position (see below), not a side-by-side capture against the
running game.

**Confidence 90** that these are the mag-floor effect's geometry: the asset
names are unambiguous (`visual_effects\MagEffect*`, no other candidate string
anywhere in the binary), they parse as real authored content rather than
placeholder data, and they preload every race the way every other
per-craft/per-weapon effect model does. Not yet runtime-verified against a
`just play` capture on a magstrip track.

## The trigger

Read in full on [engine.md](engine.md#the-tail-of-ship_casthoverprobes-fires-the-mag-floor-gfxsfx-2026-09-02).
Short version: `Ship_CastHoverProbes` writes `craft+0x240` (the same byte
`Ship_UpdateMagLock` gates on) from a probe restricted to surface type 3 - the
mag floor - and edge-detects the write, calling `Ship_MagFloorEnter`
(`0x0883e5fc`) or `Ship_MagFloorExit` (`0x0883e624`) with the craft's entity
pointer. Those two gate on an unidentified `entity+0x8bc` condition and then
call a shared, unnamed enable/disable pair (`0x088598ac`/`0x088598d8`, not a
recognised Ghidra function boundary) that flips a byte at `entity+0xb4` and
ORs/ANDs bit `0x4` on a field at `+0x2c` of two objects reached through
`entity+0xac`/`entity+0xb0`.

**Not proven**: that `entity+0xac`/`+0xb0` are specifically the two MagEffect
nodes. No constructor writing those two fields was traced - the identification
rests on the two assets existing, being effect-shaped, and having no other
found consumer, which is circumstantial rather than a located write. This is
why `0x088598ac`/`0x088598d8` are cited by address rather than named:
confidence on the node-identity claim is **55**, below the 70 line for a
plain name and arguably below the 50 line for any name at all on the *specific*
mag-floor reading, even though the mechanical behaviour of the pair (flip a
flag, OR/AND a bit on two children) is read at high confidence.

## The sfx half is open

No call to any sound-play function was found in `Ship_MagFloorEnter`,
`Ship_MagFloorExit`, or the enable/disable pair - the whole visible chain is
scene-graph flag flips, nothing that looks like `Sound_Play`.

**`oag-wad sounds` was run against the whole of `Data.wad`** (36 banks, 582
cues, all named), closing off the more obvious version of "the cue was never
searched for". The `SHIP` bank (`#860`) - the one most likely to carry a
per-craft magfloor cue, since it already carries `~ENGINE`, `MALFUNCTION`,
`FLIP`, `.COLLISIONS` - has no candidate: its nine cues are `.COLLISIONS`,
`EXPLBIG`/`EXPLBIG_PC`, `EXPLSMALL`/`EXPLSMALL_PC`, `FLIP`, `MALFUNCTION`,
`RESET`, `~ENGINE`. `SHIP_ZM` (Zone mode's ship bank) has none either. A
broader grep for `mag|strip|hum|grind|lock` across every bank does turn up
`~HUM` (in the per-track ambience banks `basilic`, `dekonst`), `~bighum`/
`~hum` (in `gentrak`, alongside `~radardish`/`~billboard`/`~neon`/`~crowd`/
`~startline`) and `~SINGLE_GRINDER` (in `fortcle`) - but every one of these
banks is named after a *track*, not a craft, and sits beside plainly
environmental cues (crowd noise, a radar dish, a starting-line neon buzz).
Read as generic track-scenery ambience, not a per-craft magfloor cue - though
whether any of them happens to be *placed* along that track's magstrip
sections specifically was not checked, and would need the pad/emitter
placement data, not just the bank's cue list.

So: **no per-craft magfloor sfx cue exists in `Data.wad`.** What remains
unconfirmed is only the first reading - that the hum is baked into one or
both `.vex` nodes' own emitter, the way `Engine Flare` owns `~ENGINE`
([exhaust.md](exhaust.md#exhaust_updateenginesound)) - which would make it
invisible to a bank-name search rooted at the trigger, and needs the node
class's own update function, not yet read.

**Do not invent a cue for this.** Per `CLAUDE.md`'s do-not-invent rule, no sfx
is wired until the emitter reading above is confirmed or refuted.

## Cross-platform

| Title | Result |
| --- | --- |
| Pulse PSP (USA) | Both assets present, hashes `ba9996ee`/`fd39ec3e`, entries 1079/1080 of `Data.wad`. The trigger above. |
| Pure PSP (USA/EU) | **These two exact hashed names are absent.** `oag-wad hash` on the same two path spellings (`Data\visual_effects\MagEffect1.vex`/`MagEffect2.vex`) gives the same two hashes (the hash function is path-text-only, not per-title), and neither hash is an entry in `pure-psp-usa.chd`'s or `pure-psp-eu.chd`'s `Data.wad` (832 entries, censused in full). That is narrower than "Pure has no such effect": a different path, a different casing that hashes differently, or a different archive (`FE.wad` was not censused) would all read the same as absent here. Pure's own Ghidra binary was not reachable in this session (bridge only exposed one program at a time) so the *code* side - whether Pure even has an equivalent trigger, over a different or absent asset - is unchecked, not ruled out. |
| HD/Fury PS3 | **No `MagEffect` string anywhere in `EBOOT.elf`.** Whatever HD/Fury does for a magstrip section (if anything - HD's track set may not reuse Pulse's magstrip sections at all) is a different mechanism or absent; not investigated further this session. |

## Open questions

- Which node class `MagEffect1`/`MagEffect2` register as, and whether that
  class's own update function is where a sound emitter would live - the same
  question `exhaust.md` answered for `Engine Flare`/`Trail`, unasked here yet.
- Whether `entity+0xac`/`+0xb0` really are the two MagEffect nodes: needs the
  constructor that writes those two fields, not yet located.
- What `entity+0x8bc` gates (possibly per-player scoping, unconfirmed - see
  `entity+0x368` on [shield.md](shield.md#entity--0x368-is-craft_construct_qs-own-second-argument)
  for a similarly-shaped but distinct field this codebase has already chased).
- The sfx mechanism - whether the hum is baked into one of the two `.vex`
  nodes' own emitter; no bank cue names it, but the node's own update
  function was not read.
- Pure's code side (does it have an equivalent trigger over a different or
  absent asset, under a path this session did not hash?) and HD/Fury's
  mechanism, if any, are both unchecked.

## History

- 2026-09-02: Assets found and parsed (90), trigger function tail read (85),
  the enable/disable pair read mechanically (confidence on node identity 55,
  left unnamed). All 582 cues across Pulse's 36 sound banks enumerated: no
  per-craft magfloor cue in `SHIP`/`SHIP_ZM`, and the `MAG*`/`HUM`/`GRIND`
  hits elsewhere are track-ambience banks, not craft ones - narrows the sfx
  question to "is it baked into the vex node's own emitter", not answers it.
  Pure ruled out for these two exact asset hashes in `Data.wad` only; Pure's
  code and HD/Fury both unchecked.
