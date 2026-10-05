---
categories: [rendering, tooling]
---

# A race flies a declared ship skin; what *selects* one is chosen, not measured

2026-09-09. Replaces `the-skin-dat-is-decoded-nothing-parses-it.md`, whose
two blocking gaps - a parser, and a caller - are both closed. That thread's
own predecessors were `team-model-and-skin-variants-are-declared-and.md` and
[per-team-hulls-are-drawn-which-team-flies.md](per-team-hulls-are-drawn-which-team-flies.md).

**What landed.** `oag_texture::ship_skin::parse` and
`oag_mesh::mesh::ship_skin::apply` landed 2026-09-07 with no caller;
2026-09-09 gave them one end to end:

- `catalogue::Team::skins` collects every `PI_ModelSkin` under a team's
  `PI_TeamModel name="Normal"` - name and the **full path** its own `location`
  spells, in file order. The disc's definition and every mounted pack manifest
  are read the same way, so a DLC team's skin resolves like a disc team's.
- `livery::ship_skin::resolve` looks an asked-for name up against that,
  case-insensitively, and `livery::one` repaints grid slot 0's hull from the
  `.dat` it names.
- `--skin Alternative` / `--skin Eliminator` on `oag-game`.

Verified against `pulse-psp-usa.chd`
(`livery_ground_truth::a_skin_repaints_the_players_own_hull_and_nobody_elses`):
all four of Assegai's `Ship.vex` texture slots take a block, at exactly the
size the block declares, **100% of the player's own texels change** and every
opponent's stay byte-identical. Eyeballed too, off
`crates/game/examples/pulse_skin_probe.rs`' PNG dump: the alternative atlas is
the same panel layout in a different colour scheme, which is what a repaint
should look like and not what garbage looks like.

## Open

- **What selects a skin is this project's choice, and is labelled so with no
  confidence score.** The player names it and it lands on slot 0 alone - the
  same footing [`livery::teams_for_slots`](../../crates/game/src/livery.rs)
  stands on. **No unlock is checked either**, because what `loyalty`
  accumulates is untraced.

  What the original appears to do instead: `Skin_LoadForTeam` picks
  `ship_eliminator.dat` when a global compares equal to `0x12` and consults no
  unlock at all on that path, while `ship_alt.dat` is the `loyalty`-gated one.
  **Confidence 75 on the split, 55 on what `0x12` denotes** - "Eliminator race
  mode" is the natural reading, but no Pulse mode-id table has been recovered
  to check it against and the enclosing function is multiplayer-lobby code, so
  a lobby game-type id fits the same evidence. **Do not build a mode gate on
  that number without settling it first.**

  **What would settle it, both needing the Ghidra bridge:**
  1. `scripts/psp-relocate.py resolve <instruction-address>` on the `0x12`
     comparison (`0x000578b0` as the decompiler prints it, which falls inside
     `.text` under the usual base - the tell that the relocation base is wrong
     for it). The `$gp`-relative hypothesis is **retracted**: this binary never
     uses `$gp` as a load/store base, and the address needs the segment-1 base,
     which `psp-relocate.py resolve` reads straight off the relocation record.
     The per-slot record array at `0x00057c44` is the same shape.
  2. Failing that, trace the **six non-lobby callers** of `Skin_ApplyToModel`:
     `0x08825638`, `0x088256b8`, `0x08828398`, `0x0882885c`, `0x08843804`,
     `0x088eaa84`. (`0x088b3af4` is the lobby's own.) That settles the
     mode-vs-unlock question without either address.

  Same trace also answers **the applier's flag argument**, only ever seen as
  zero: a non-zero flag uploads the *stored* 64x64 block 4 instead of the
  composite.

- **A menu row is the obvious next caller and was deliberately not added.**
  The RACE page would want a SKIN row beside VARIANT, offering the names the
  player's team declares. It needs a `string_id` and its English text in
  `assets/ui/strings/english.toml` in the same change or `just check-strings`
  fails the gate, and it wants
  `crates/game/src/main/session/menus/` - none of which this pass owned.
  `--skin` is the caller today.

- **The composed fourth slot is still not reproduced**, and the stored block 4
  is uploaded instead - which is the applier's own non-zero-flag path, so it
  needs no answer to the swizzle question. New corroboration for that question
  though, measured 2026-09-09 and written onto
  [ship-skin.md](../../docs/ghidra/functions/psp-pulse-usa/ship-skin.md):
  Assegai's own `texture4.tga` **is already a quarter atlas with a near-black
  top-right** (channel mean 16.7 against ~150 for the other three quadrants),
  and the skin's stored block 4 has the same shape (42.6 against ~185). The
  authored data is laid out the way a linear composite with an unwritten
  top-right predicts. It does not settle the swizzle flag - a swizzle mapping
  the empty tile onto the same corner is indistinguishable from outside.

- **A skinned slot is uploaded with no mip chain, where the hull's own texture
  probably has one.** `mesh::ship_skin::apply` builds the replacement with
  `ModelTexture::rgba8(..., None)`; the original generates a chain instead, by
  halving in index space with a palette-aware 2x2 filter
  (`Texture_Downsample4bpp` - see
  [ship-skin.md](../../docs/ghidra/functions/psp-pulse-usa/ship-skin.md)'s
  "Mips are generated, never stored"). So a skinned craft may alias at distance
  where the baseline does not, which is a real "more than one frame, at the
  size a player sees" difference and not a formality. It landed with the
  applier on 2026-09-07 rather than with the caller, and nothing has looked at
  it on screen. Cheap to check once a GUI is available; cheap to fix by
  generating the chain in `apply`.

- **`zone01.vex` is untouched, and deliberately.** `PI_TeamModel name="Zone"`
  names it, `Data\Ships\<Team>\zone01.vex` exists for all eight base teams, and
  it is **not** what a Zone race loads - that is `Zone.vex`, through a
  recovered selector
  ([zone-mode.md](../../docs/ghidra/functions/psp-pulse-usa/zone-mode.md)),
  and the two files are not byte-identical. Diffing it against `Zone.vex` and
  `Ship.vex` - vertex/triangle counts first - is the next real step. Until
  then, offering it anywhere would be authoring a stand-in for something the
  disc specifies differently.

- **A skin is refused on a Zone race, out loud.** No `PI_ModelSkin` names a
  file for the Zone hull, so painting one there would be a surface the disc
  does not specify. Pinned by
  `livery_ground_truth::a_zone_race_refuses_a_skin_and_says_so`.

- **The generic `%s\%s.dat` fallback path is still unexercised.** When the
  specific file fails to load, the loader walks the team node's children for
  the one named `Normal` and rebuilds the path from that child's `+0x94`
  field. Nothing observed what that field holds, so **which file the fallback
  names is unknown** - do not assume it is `ship_alt.dat`.

- **Nothing paints an opponent.** Slot 0 only, for the same reason
  `hull_variant` is: nothing offers an opponent a choice of their own, and
  inventing one would be a grid rule this project made up. If the `0x12` trace
  comes back as a *mode*, every craft in that mode wears `ship_eliminator.dat`
  and this becomes a whole-grid decision rather than a per-slot one - which is
  the shape to expect, not the shape shipped.

## Next Steps

- With the Ghidra bridge: `scripts/psp-relocate.py resolve` on the `0x12`
  comparison, then the six callers above. That converts `--skin` from a chosen
  stand-in into a measured selector, and answers the flag argument in the same
  pass.
- Add the RACE page's SKIN row (needs `crates/game/src/main/session/menus/`,
  `assets/ui/menu.toml` and `assets/ui/strings/english.toml` together).
- Diff `Data\Ships\<Team>\zone01.vex` against `Zone.vex` and `Ship.vex`.
  Independent of everything above.

## From the HANDOVER.md index (moved 2026-09-25)

**Which skin a race flies is chosen, not measured, and no unlock is checked** - the original selects `ship_eliminator.dat` on a global equal to `0x12`, confidence 55 on what that number denotes, so a mode gate on it would be a guess dressed as a reproduction. Settled by `scripts/psp-relocate.py resolve` on the comparison or by tracing the six non-lobby callers of `Skin_ApplyToModel`; both need the bridge. Still open: the composed fourth slot (new asset-side corroboration for the linear layout, not proof), `zone01.vex`, the `%s\%s.dat` fallback, and a RACE-page SKIN row that needs its own `string_id`
