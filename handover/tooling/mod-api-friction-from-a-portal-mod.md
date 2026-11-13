---
status: ready-to-wire
titles: [all]
updated: 2026-11-13
summary: "Mod API friction left after names-by-key, held scalars, stop-mod-effect and state_bytes landed: a second fill, a picture age, a cut flag."
kind: debt
size: S (50)
difficulty: easy (60)
---

# What building a portal's tunnel cost in the mod API

Written by the session that took `oag.slipstream` from a screen wash to a tunnel flown through
(2026-11-10, branch `slipstream`; the engine side of that is in
`handover/gameplay/slipstream-engine-follow-ups.md`, "The flight through the tunnel"). Every
item is something that cost time or went wrong there, with what would have saved it. Sizes are
effort for one member with a confidence 0-100; ranked by how many mods the fix reaches.

Landed with this thread, small and not in doubt:

- `oag_ext::Craft::speed()` (track units a second), with `SPEED_TO_KMH` and `TICKS_PER_SECOND`
  in `oag-ext-abi`: a guest had its own `3.6` and `60.0`.
- `oag_ext::lap::{ahead, gap}`: sums and differences of two lap progresses, taken round the
  line. The guest had written both by hand.
- `oag_ext::test::Race::set_progress`: the guest's tests rebuilt the whole race to move a craft.
- `just check-guests` (`scripts/check-mod-guests.py`): every guest's own tests, and each
  committed `ext/*.wasm` against a fresh build. Now in the gate.
- ABI 1 revision 5 (`mod-api` lane): a guest names a shader, effect or sound by its key in the
  manifest (the reply's string pool, as an icon always was), and a key the manifest lacks faults
  the module naming it and the keys it declares; `set-shader-param` holds until sent again, until
  the fill through its shader stops or the module faults (four values do not fit a 12-byte record,
  so one command for all four is declined); `stop-mod-effect` (command 38) ends a mod's placed
  surfaces for a slot; a manifest with no `state_bytes` takes the module's `oag_state_len`;
  `view_aspect()` in mod shaders; a frozen transit craft's `progress()` is pinned by a test and
  `hooks.md`; the cut frame is written up in `mod-shaders.md`. Every guest in `assets/mods` is
  updated and rebuilt.

## Open

1. **One screen fill a slot a tick.** With the tunnel a backdrop, the same mod cannot also wash
   the view (a flash at the exit was dropped for this, and because a fill cannot be both). A
   second fill a slot, or a backdrop and an overlay as two commands. Size S (40). Declined for
   now: the next visual mod that needs it asks.
2. **A shader cannot tell a view that has no engine picture from one that just lost it.**
   `engine_texture_bound()` is false for PORTAL VIEW Low, for a split's second seat, and for
   every view the tick after the mod stops asking. So the light at the far end of a tunnel
   with no picture cannot fade across the exit: it would have to know the picture is gone for
   good, and Low cuts from light to the road. Keeping the last picture bound for a few frames
   after the request stops, with its age readable, would let a shader fade it. Size S (30).
   Declined for now, as above.
3. **A shader is not told a frame is a cut.** Written up in `mod-shaders.md` ("At a cut"); a
   `between` of 0 on a cut frame, or a flag, would remove the guest's workaround. Size XS (60).
4. **`hud-icon` and `pickup-look` naming an icon the manifest lacks draw nothing and fault
   nothing**, where `raise-warning` faults the module naming it. Their documented behaviour is
   to draw nothing, so it was left; faulting is one arm in `named()` of
   `crates/raceplay/src/extensions/on.rs`, and existing tests that send undeclared icon names
   (`tests::extensions::on::a_mods_own_string_is_shown_in_the_games_language_...`) would declare them.
   Size XS (70).
5. **`oag.slipstream` does not use the new commands yet**: its constants (`transit`, `speed`) are
   re-sent each tick, and its portal sprite outlives a refused approach by up to 10 ticks and a
   transit by 45. `set_shader_param` held and `stop_mod_effect` cover both. A picture-checked
   change to a shipped mod, so its own lane. Size S (60).

## Next Steps

1. Item 5 is the one that makes the changes visible to a player; it needs a screenshot pair of a
   portal flight (`docs/architecture/wasm-extensions/mod-shaders.md`).
2. Items 1 and 2 are what the next visual mod will ask for; take them when one does.
