# HD/Fury's particle effects parse, and a "field that disagrees" turned out to be a wrong check

2026-08-18. `oag_pob` now takes an `oag_formats::ByteOrder`, sniffed from the blob's own magic (`SYSP` little-endian, `PSYS` big-endian) exactly as `vex::byte_order` does. **All 88 HD `.pob` parse and their 249 emitters walk, no offset changed**; `crates/pob/tests/pob_ground_truth.rs` runs the Pulse checks over them unchanged. Only one field is not a plain re-read: the 256-entry colour table is a `u32` per entry parsed as bytes, so a big-endian entry arrives reversed. **The retraction is the part to read.** `docs/formats/hd-status.md` reported the `+0x04` length disagreeing on all 88, short by 48-208 bytes, and hypothesised an uncounted appended string block. That was `scripts/hd-survey.py` comparing `+0x04` to the *file* length; the field counts a payload starting after the 32-byte name, so it is short by `base + 16` on every platform - the same check would have failed on all 76 Pulse files. Script fixed, both doc pages carry dated retractions, and the identity is now asserted per file on all three discs. **Two HD-only authoring differences, both pinned rather than tolerated**: `blend_class` **4** on the seven emitters of `WO_NITRO_SHIP_DEATH` (`oag_fx::psys` refuses it by `Error::UnknownBlendClass` rather than guessing), and **`LOOPING` is per-emitter on HD** - four systems mix set and clear inside one tree, so `pob::flags::LOOPING`'s "property of the whole effect" was a Pulse regularity, not a rule. **One thing arrived at by fall-through and is now checked**: `crates/raceplay/src/assets.rs` picks `psys::ColourScale` by platform and a PS3 source reaches `Full` through the `_ =>` arm rather than a decision. `Full` is right - HD's palettes run to 255 like the PSP's and unlike the PS2's 0-127 - and `crates/fx/tests/psys_ground_truth.rs::every_hd_effect_translates_or_is_refused_by_name` plays all 88 at that scale. Do not narrow that arm without re-reading this. **Two of them are wired and both needed a fix beyond the parser, 2026-08-18.** (1) `WO_SHIP_COLL_SPARK_DAMAGE` is `LOOPING` on all four emitters where Pulse's is on none, so the burst never ended and sparks poured off a craft that had left the wall; the trigger now has two modes keyed on the effect's own flag - a burst keeps the recovered cooldown rule, an attached effect ignites once per contact and is released when the contact ends. **Confidence 55 on the owner** - that a looping effect needs one is the flag's contract, that it is *wall contact* is inference, and HD's own dispatch is what would settle it. (2) `WO_SHIP_ENGINEFLARE` loaded and still did not draw, because **HD keeps its locator nodes in `Locators.vex` beside the hull** rather than in `Ship.vex` - see `docs/formats/hd-status.md`. Both are `#[ignore]`d disc-backed tests. **Next step: nothing else in HD is wired to fire any of them.** The parser is done; the triggers are reverse-engineering, the same position the Pulse effects are in.

## Open

- **2026-10-06 (hd-particle-triggers):** the census and every HD function found are
  in `docs/ghidra/functions/ps3-hdfury-eu/particle-triggers.md`. "Nothing else is
  wired" was stale: 33 of 82 already fire, mostly Pulse-inherited. New: the Cannon's
  craft-hit spark (`WO_SHIP_SPARK_DAMAGE_WEAPON`, HD only). The plain
  `WO_SHIP_COLL_SPARK_DAMAGE` (and `_ZONE`) is created once per ship in the ship
  constructor (`0x000ddd58`), attached - the "what names it" bullet below is closed.
  Still open: Zone variants (need the mode-id map), `WO_DAMAGE_*` damage smoke
  (`0x002a17e8`, thresholds unread), the LeachBeam launch/hit/break callers,
  `WO_SHIP_COLL_SPARK_NODAMAGE`'s kind-2 caller, `WO_DEBRIS_FIRE`.
- **2026-08-31: HD's own dispatch has now been read** (`docs/ghidra/functions/ps3-hdfury-eu/ship-collision-fx.md`) -
  `Ship_DispatchCollisionFx_q` (nearest-of-ten locator pick, confirming
  `hd-status.md`'s own "ten" prediction from the executable side) calls
  `ShipCollisionFx_Trigger_q`, which names two of its four `kind` branches
  from literal strings: `kind==1` spawns a **previously unrecorded** effect,
  `WO_SHIP_SPARK_DAMAGE_WEAPON`, and `kind==2` spawns
  `WO_SHIP_COLL_SPARK_NODAMAGE` or its `_ZONE` variant depending on a
  game-mode mask. **Neither branch is `WO_SHIP_COLL_SPARK_DAMAGE`** - the
  plain damage variant this thread's "wall contact" question was actually
  about is still unnamed anywhere in that function, so the confidence-55
  claim is not confirmed, it is complicated: what fires the plain
  `WO_SHIP_COLL_SPARK_DAMAGE` variant specifically remains open, and the one
  dispatch-table entry read (`FUN_0010f730`, entry 11 of a >=12-entry
  reaction table at `0x00875800`) reaches the *weapon*-named branch via a
  hardcoded `kind`, not confirmed to be wall contact at all. See that page's
  own Open section for the exact gaps - do not re-close this bullet at
  confidence 55's old framing.
- `WO_SHIP_SPARK_DAMAGE_WEAPON` is not yet cross-checked against
  `psys_inventory_ground_truth.rs`'s HD names.

## Next Steps

- Reverse-engineer the triggers for the remaining parsed HD particle effects (the parser itself is done)
- ~~Find what names `WO_SHIP_COLL_SPARK_DAMAGE` (plain)~~ (closed 2026-10-06, the ship constructor); still find `WO_SHIP_SPARK_DAMAGE_LEACHBEAM`'s owner (slot loaded inside `0x002a06e0`, a conflict) and - `ship-collision-fx.md`'s
  `ShipCollisionFx_Trigger_q` does not, despite being read in full across all
  four of its `kind` values.
- Identify what `0x00875800`'s reaction table is keyed by and what index 11
  represents - the table's own base is a second TOC hop away from every
  address this pass's literal-search technique could resolve in one step;
  see `ship-collision-fx.md`'s "reusable technique" section for what worked
  and where it stopped.
