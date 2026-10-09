# A wrecked craft draws its wreck, explodes, and the player is cut to the circuit's own camera

2026-10-01, lanes `pulse-wreck` and `pulse-wreck-2`. Evidence pages:
[ship-wreck-model.md](../../docs/ghidra/functions/psp-pulse-usa/ship-wreck-model.md),
[camera.md](../../docs/ghidra/functions/psp-pulse-usa/camera.md) ("The destroy camera"),
[screen-flash-callers.md](../../docs/ghidra/functions/psp-pulse-usa/screen-flash-callers.md).

**Landed.** `Ship_SetState` case 5 makes `shipwreck.vex` the live model
(`CraftState::Eliminated`; `oag_raceplay::scene::wreck`), and `race::wreck_fx` throws
`WO_SHIP_FXNODE_EXPLO` and `WO_SHIP_DEATH_SPARKS` at each wreck locator on that edge,
then `WO_SHIP_EXPLOSION` 1.5 s later at the wreck's own model matrix moved
`4.0` rows along `-up` (`FUN_088407b0`, read and measured live: the matrix rows were 0.75
long and the translation `4.0 * 0.7499` below the craft). The player is cut to the
circuit's own authored `Camera` node nearest the wreck's aim point, zoomed to frame 35
units (`oag_render::camera::destroy`, pinned to the running original's per-frame
field of view and view matrix; `destroy_camera_ground_truth.rs` pins it on the disc). The
player's own state-5 wash is no longer skipped and the player's two explosion shakes are
armed. **The fireballs read white because twelve PSP sprites were 4 bits per pixel and the
reader refused them** (`oag_pob::texture`); fixed, and the picture now matches the
original's orange textured fireball frame for frame at 480x272 (the opponent wreck,
`cap-opp` against ours, k 36 to 140).

## Open

- ~~**A finished race freezes the world**~~ **Closed 2026-10-02 (pulse-wreck-3).** A race a wreck ended (Single Race, Zone) now
  steps its cosmetics under the results (`Race::tick_cosmetics`, called by `Session::frame` and the headless capture where they stopped
  stepping a finished race): the player's state-5 shake (armed in every mode now), the wreck's fire, the big explosion 1.5 s on, its
  ring, the screen flashes and the destroy camera's ease. It writes nothing under `sim` (`world.tick` included), so the standings and
  the board stay frozen; `wreck_finished` pins the hash. Seen at 480x272: the camera
  stays on the wreck, the explosion's fireball and the kind 7 wash play under the panels, the picture is clear again by about 100
  frames; a Zone wreck plays the same (`sheet_z.png`; Zone's own overexposed circuit look). **Over it the results panel is drawn at once**,
  and the original's is not: a scratch directory, not kept (the original, a player wreck after GO) shows no panel at 40, 80 or 120 frames
  with the circuit's camera and the fire/wreck clear, so the sequence this change plays is **hidden by ours** for the player
  (`cmp_original_vs_ours.png`: same camera composition, panel on ours only). That is the
  `Race End Photo` state (`after-the-finish.md`: about a second of clean view, then the legend, then the panels on X), which the
  EndRace lane owns. **2026-10-02 (pulse-end-photo): not that state after all** - the original's front end stays on `InGame` after a player wreck for 17,800+ frames with no panel and no legend (the field races on), so the line's `Race End Photo` does not help here. **Resolved by the maintainer's "hold, then results" (2026-10-02, pulse-end-photo)**: a Single Race wreck now runs the world on, shows no panel for 61 ticks and then the legend, so the explosion (at 90 ticks) plays under the legend alone, not a panel; Zone still shows its panels at once. **Chosen, not measured:** only the player's wreck was looked at, so the opponents, shots in flight and trails stay
  frozen under it (the original goes on running them).
- ~~**The Eliminator respawns the craft at the same tick the big explosion goes off**~~ **Closed 2026-10-02 (pulse-wreck-3).** State 8's
  update is `Ship_UpdateRespawn` (`FUN_088418e0`'s per-state jump table at `0x08a7bb88`: state 8 goes to the call at `0x08841e44`),
  so the Eliminator waits state 5's `1.5` s and then state 8's `1.0` s (the player) or `0.8` s (anyone else; was `2.0`, corrected by a live run 2026-10-02):
  `eliminator::eliminator_respawn_delay`. The destroy camera is on the wreck as the explosion goes off and lets go with the respawn
  (`sheet_e.png`: the fireball seen from the circuit's camera, then the RESET wash and the chase
  camera). Confidence 88: the table and the call read, no live run of an Eliminator wreck.
- ~~**State 6 is `FUN_08840500`, a bare `+0x874` countdown; does a single race's wrecked AI craft ever come back?**~~ **Closed
  2026-10-02 (pulse-state6): no.** Live on PPSSPP, two runs (`Ship_SetState(4)` on slot 1, `Ship_Damage` on slot 2), the craft goes
  4 (0.5 s), 5 (1.5 s), 6 and stays in 6 with the destroyed bit set and the wreck at rest for 13 s past the 0.8 s timer; a write
  watchpoint on `entity+0x8C` saw only `Ship_SetState`'s two writes (4 to 5, 5 to 6). `eliminator.rs` now leaves a destroyed craft
  down outside the Eliminator (confidence 85). Still open from it: what `Race_FinishAllCrafts` does to a wreck at the flag, whether
  the standings drop a wreck, and a death by a real weapon or wall rather than a debugger call
  (`shield.md`, "State 6 measured on PPSSPP").
- ~~The explosion's smoke is thinner and its fire brighter and longer in ours at k 160 to
  180 (2.2x to 2.4x).~~ **Closed 2026-10-01 (pulse-fx-recheck)**: the matrix's `0.75` rows scale every root
  emitter's spawn offset and velocity (and not sizes), an emitter's run is one tick short, a template dies one tick
  early, and `FUN_088407b0`'s `Bomb_Shockwave.vex` ring was never drawn
  ([ship-shockwave.md](../../docs/ghidra/functions/psp-pulse-usa/ship-shockwave.md),
  [particle-system.md](../../docs/ghidra/functions/psp-pulse-usa/particle-system.md#the-instance-matrix-scales-a-root-emitters-spawn-and-a-run-emits-one-tick-short-2026-10-01)).
  Fire-coloured pixels now read `1.04, 1.08, 1.23, 1.24, 1.57` of the original's at 140 to 180 and smoke `1.08,
  0.96, 1.0, 0.87, 0.72`. **2026-10-01, pulse-fx-3: the first five frames and the dim ring are closed.** The white, hard-edged band is the
  `Glow` template's quad (a view-space additive sprite, `90 x 60` units at depth `31.5`, its lower edge cut by the floor's depth
  test); ours drew it with its *parent's* sprite (`SHIP_DEBRIS`'s grey atlas, centre alpha `0`) because `ParticleSystem_DrawParticle` binds the
  texture header in the **template record** (`+0x890`) and the port took the emitter's. All 28 PSP templates now draw their own
  ([particle-system.md](../../docs/ghidra/functions/psp-pulse-usa/particle-system.md#a-sprite-templates-own-sprite-and-what-the-explosions-first-five-frames-are-2026-10-01-pulse-fx-3)):
  the wash, its extent and its hard edge match at frames 122-125. The ring was never 1.5-2x dim in
  its draw: at equal step count the horizon band reads `185, 158, 110, 73` against `176, 148, 105, 76` (within 7 %); the original's `ShipShockwave_Update`
  steps `(int)(dt / (1/60))` with no remainder, which on PPSSPP skipped 29 of 81 frames (`ship-shockwave.md`). **Still open here:** the mode-2 emitters
  (`SHIP_DEBRIS`, `FIRESPIKES`) spread 15 to 30 % more than the uniform frame scale gives; the fire still
  holds a little at the tail (`1.5x` at 180); our slot is about `25` px from the original's.
- With the default profile (motion blur on) the cut frame and the state-5 shake frames show a
  white diagonal smear: the shake turns the view by more than the 4 degree field, the blur's
  clamp does the rest. The cut now resets the upscaler's history; the blur has no history to reset.
- ~~`FUN_088407b0` also builds a `Data\Weapons\Bomb_Shockwave.vex` object - read, not drawn.~~ Drawn 2026-10-01
  (`bomb_blast::BlastKind::ShipExplosion`), law read and logged live; its alpha ease reaches the draw as the ambient light's alpha and is applied. The ring's late dimness was the original's scheduler (`ship-shockwave.md`), closed.
- ~~**The HUD fade.**~~ **Closed 2026-10-01 (pulse-fx-3): there is no fade, and the 160 frames were an injection artefact.** The HUD is hidden by
  `Hud_Hide` (`0x0881a128`): `g_hud + 0x168` goes `0x100 -> 0x101` and `+0x2c` loses bits `2` and `4` in one frame, called from
  `ArcadeRace_UpdateRacing` when the player's destroyed bit is set while the mode state is racing (`shield.md`, "Who ends a single race on
  the destroyed bit"). Injected **after GO** (`--inject-frame 330 --hud`, `hudT`), the flag flips one frame after state 5 begins (`k = 31 -> 32`)
  and the frame at `k = 40` has no HUD; injected **during the countdown** (`--inject-frame 40`, `hudS`, the earlier lane's method) the mode state
  is not yet racing, so the check waits for GO and the hide comes 316 frames after the call - the "160 frames after state 6". Each case seen
  once (confidence 85: the decompile plus the after-GO frame). Ours ends the race at state 5 and so hides the HUD at the same point; nothing to port. The
  race that follows is the original's own (it keeps running under the circuit's camera; ours freezes: first Open item).
- The camera lets go "when the craft is racing again" (chosen); the original's hand-over to
  another craft after ten seconds, and `Camera_RepickNearSubject_q`, are read, not ported.
  The focus rate `0.4` was seen on Venom only (Flash `0.5`, Rapier and Phantom `0.6` read).
- The wreck's two authored `Trail` nodes ([exhaust.md](../../docs/ghidra/functions/psp-pulse-usa/exhaust.md)):
  not looked for in the original's wreck frames.
- Under a wreck our blob shadow, shield shell and absorb overlays still draw as for a hull; the
  engine flare quad is hidden (seen, mechanism unread). `+0x2c` bit `0x1000000` appears on the wreck
  at state 6, unread.
- PS2 Pulse, Pure and HD: none of their wrecks or `Ship_SetState` is read; Zone's `zonewreck.vex`
  loads but no Zone craft was wrecked on the original.
- Not confirmed against a craft a race eliminated (`scripts/psp-wreck-capture.py` calls
  `Ship_SetState(entity, 4)`).

- The hull's own collision sparks and every other effect parented to a craft's node ride the same 0.75-row matrices and are still
  played at frame scale 1.0: probably the same law, **unmeasured** (`particle-system.md`, 2026-10-01).

## Next Steps

1. ~~Let a finished race keep stepping its cosmetics~~ Done 2026-10-02 (`Race::tick_cosmetics`). Left: the opponents, shots and trails
   under a wreck's results (measure a Single Race the player wrecks in, on the original, for what the field does).
2. ~~Give the Eliminator the original's state-8 wait~~ Done 2026-10-02 (`1.0` s player, `0.8` s others - the `2.0` was a misread branch delay, fixed by a live opponent run, pulse-state6). The player's `1.0` s was then run live too (`e3`, 4, 5, 8 at 1.0, 1). (the one-craft run needs the campaign Eliminator or a `Ship_SetState(entity, 8)` poke).
3. ~~Compare the explosion emitter by emitter, then read what scale the matrix applies.~~ Done 2026-10-01
   (`scripts/psp-wreck-capture.py --pools/--templates/--hits/--ge-dump-k`). ~~Why the first five frames read white~~ done
   (the `Glow` template's own sprite). Next: the mode-2 spawn spread, and what delays the explosion's particles by about 2.7 frames
   behind the ring (`particle-system.md`, measured once, mechanism unread).
4. ~~`--state 5` on an Eliminator craft~~ Done for an opponent 2026-10-02 (`scripts/psp-state6-watch.py`, `Ship_Damage`): 4, 5, 8 at 0.8, 1. The player too (`e3`: 8 at 1.0 s).
5. ~~Settle state 6~~ Done 2026-10-02: it never comes back (see the Open item above).
