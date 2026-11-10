---
status: ready-to-wire
titles: [all]
updated: 2026-11-10
summary: "Mod API friction met building oag.slipstream's tunnel: stale example modules, places by name order, shader scalars, effects that cannot be stopped."
kind: debt
size: M (40)
difficulty: easy (50)
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
  committed `ext/*.wasm` against a fresh build. Not in the gate (item 1).

## Open

1. **Six of the seven committed modules are not what their guests build**, and nothing in the
   gate compiles a guest. Found twice in one day: `oag.slipstream`'s guest tests did not compile
   (a `PortalView` exit had become a `Distance`; the module committed before that still loaded),
   and `just check-guests` on 2026-11-10 reports every example stale (`checkpoint_circuit.wasm`
   9,301 bytes committed against 10,264 built, `energy_turbo` 3,649 against 4,482, `hillside_rain`
   1,696 against 2,464, `tether` 3,828 against 3,972, `wall_penalty` 1,662 against 2,392, `window`
   4,045 against 4,114; all seven guests' tests pass). The cause was not looked for: an SDK
   that grew, or a toolchain bump, since the bytes are a function of both. They load, so the ABI
   revision they were built at still matches. To do: rebuild the six (`guest/build.sh`), look at
   one diff to say which cause it was, then put `check-guests --strict` in the gate or in CI's
   job list; it needs the wasm target, builds seven small crates twice cold and takes half a
   second once they are built.
   A toolchain bump will then stale all of them at once, which is the honest answer. Size S (70).
2. **A guest names its shaders, effects, sounds and icons by their place in name order**
   (`SHADER_WORMHOLE: u16 = 1`, effect indices 0 to 4 in `Look::of`, `SOUND_*`). Adding a file
   whose name sorts earlier renumbers the rest and nothing fails: the wrong shader draws. Each
   guest carries a test that parses its own manifest to pin the order (Slipstream has one for
   its sounds, and a third kind for its shader's header).
   A key resolved by the host, as `title!("wipeout-hd")` is, would end it: `shader!("wormhole")`
   hashed at compile time, the host mapping key to place when the module loads and faulting on
   one the manifest lacks. An ABI change (the commands carry a `u16` place today); v1 is edited
   in place before release. Size M (50).
3. **A shader has four scalars, each its own command, each re-sent every tick for every view.**
   The tunnel sends `phase`, `transit` and `speed` (it had a fourth before it lost a use for it),
   two of them constants for the whole flight, beside the fill and the field of view: five
   commands a tick a craft for the tunnel, and two more for the portal's own view and scalar,
   of `MAX_COMMANDS` 96. Two things, either alone helps:
   a scalar that holds until it is sent again or the fill stops (most are constants), and one
   command for all of a shader's scalars. The colour's alpha was pressed into service as a fifth
   scalar for a while, which is the sign. Size S (50).
4. **A fired mod effect cannot be stopped or given a life**: `lifetime` is in its TOML. The
   portal's sprite stands for 60 ticks whatever happens: after a refused approach it outlives
   the portal by up to 10 ticks, and after the craft goes in it stands for another 45. A
   `stop-mod-effect` for a held surface (the stage already addresses one by generation,
   `Stage::orient_surface`) or a life on `fire-mod-effect-ahead`. Size S (50).
5. **One screen fill a slot a tick.** With the tunnel a backdrop, the same mod cannot also wash
   the view (a flash at the exit was dropped for this, and because a fill cannot be both). A
   second fill a slot, or a backdrop and an overlay as two commands. Size S (40).
6. **A shader cannot tell a view that has no engine picture from one that just lost it.**
   `engine_texture_bound()` is false for PORTAL VIEW Low, for a split's second seat, and for
   every view the tick after the mod stops asking. So the light at the far end of a tunnel
   with no picture cannot fade across the exit: it would have to know the picture is gone for
   good, and Low cuts from light to the road. Keeping the last picture bound for a few frames
   after the request stops, with its age readable, would let a shader fade it. Size S (30).
7. **`state_bytes` in the manifest repeats `Blob::BYTES`.** It changed three times in a day
   (40, 48, 80) and each guest carries a test that parses its manifest to compare. `export!`
   could export the size and the host read it from the module, the manifest's number becoming a
   cap or going away. Size S (60).
8. **A frame between two ticks at a cut.** `input.between` runs -1..0 on every tick, the one a
   craft is put elsewhere on included, where the camera does not blend but a count of ticks a
   shader reads still does: for that tick's frames the shader is a tick behind the picture. The
   guest skips a count (`Flight::Through`: no tick is `LAST + 1`) so no frame shows the far side
   early. It works and is written in the skill's terms nowhere but the guest; either say it in
   `mod-shaders.md` or give a shader the view's cut (a `between` of 0 on a cut frame). Size XS (60).
9. **No helper for the view's shape.** A fill's `uv` is 0..1 both ways; a circle needs the
   aspect. The first tunnel took it from `dpdx`/`dpdy` of `uv`; `eye_ray`'s scale now has it
   (`oag_frame.lens.x / lens.y`), one line as `view_aspect()`. Size XS (80).
10. **`Craft::progress()` is last tick's and a transit's frozen craft keeps reporting it**, which
    the guest relies on (the pair it asks for is from where the craft stands). Nothing says a
    frozen craft's progress is stable; one line in `hooks.md` and a test. Size XS (70).

## Next Steps

1. Item 1 first: it is the only one where what ships can differ from what the source says.
   `just check-guests`, rebuild, read one diff, decide gate or CI.
2. Items 2, 3 and 7 are ABI edits, so do them together and rebuild every guest once
   (`check-guests` then says they are fresh). `oag-mod-author`'s skill changes with each.
3. Items 4 to 6 are what the next visual mod will ask for; take them when one does.
