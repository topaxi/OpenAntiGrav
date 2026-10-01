# PS2 Pulse's shield drew solid; it is textured and additive now, and unverified against a capture

2026-09-17. Reported from play by the user: on the PS2 Pulse source the
Shield pickup's shell "renders solid and not animated", where the PSP source
draws it right (the additive shell that swells on impact and breathes -
`oag_render::shield`, recovered on
[shield-pickup.md](../../docs/ghidra/functions/psp-pulse-usa/shield-pickup.md)).

## Diagnosed and fixed, 2026-09-24

Reproduced with `oag-game data/images/pulse-ps2-eu.chd --race --no-audio
--give shield --hold cross --press square --ticks 420 --screenshot ...`
against the same on `pulse-psp-eu.chd`: the PS2 shell was an opaque pale-blue
dome hiding the craft. `oag-view ... --mesh 'Data\Ships\Feisar\shipshield.vex'
--draws` on both discs showed two causes, both on the PS2 path alone:

1. **No texture.** The PS2 shell and `vr_shield_cockpit.vex` each declare one
   `Texture` node and embed nothing, so both bound the white 1x1.
   `livery::shield::shield_model` now takes the plume's `ps2_skin` on the same gate;
   all twelve teams' shells and the sphere decode `1 of 1`. See
   [ps2-texture.md](../../docs/formats/ps2-texture.md).
2. **No blend class.** The PS2 shell's batches (`0x1031`, `0x18b1`, `0x10b2`)
   carry no `0x0700` bit where the PSP shell's (`0x1232`) carry `0x200`, so
   they drew opaque. With the shell opaque, the alpha breath could not show,
   which accounts for "not animated". `livery::shield::blend_additively`, gated on
   the PS2 platform, moves them to the additive class. Evidence, confidence
   70: `ShipShield_Construct` (`0x00169168`) builds the shell through the
   same constructor, sort word `0x7d000000` and arguments as the plume, whose
   blending a PCSX2 capture settled. See
   [batch-draw-state.md](../../docs/ghidra/functions/ps2-pulse-eu/batch-draw-state.md#the-shield-shell-reaches-the-same-unfollowed-draw-2026-09-24).

`crates/game/tests/ps2_shield_ground_truth.rs` pins both halves on the real
disc.

## Open

**2026-10-01, a raised PS2 shell captured** (see
`docs/ghidra/functions/ps2-pulse-eu/shield-pickup.md`): the shield object is
`*(craft+0x958)`, its law is the PSP's at `+0x80`, and **writing what
`ShipShield_Activate` writes over PINE raises a shell on a savestate** - the
2026-09-25 blocker is gone. Measured against a bit-identical control: the PS2
shell **never darkens a pixel** (additive, not alpha-over: the blend inference is
now a reading, confidence 88), looks like soft cyan ring bands, and **ours looks
nothing like it** - a crisp hexagon lattice over the whole craft.

- **The lattice against rings gap is the open item.** Candidates, untested:
  the GS samples a coarser mip level of the tiled 256x256 `PSMT4` lattice, the
  texture is not the one the original binds, or the texel alpha (128 = 1.0)
  combines differently with the vertex alpha. A GS dump of the shell's draw
  (`TEX0`/`TEX1`/`ALPHA`/`TEST`) would read it. The UVs are `u 0.09..7.58`,
  `v 0.39..6.91` over eleven rings; PSP's are `u 0..0.5` over four rows.
- **One 66-triangle band may draw twice.** Unread; the PS2 shell has three additive
  ranges of 124, 141 and 124 triangles in ours.
- **Colour and brightness against a pixel-matched frame** are not compared: ours
  was not posed at the savestate's craft.
- **The "not animated" half** is answered by the captured frames: band brightness
  and position change between frames. Ours' own PS2 shell animation (the `sin`
  breath, the scroll) was not diffed against it.

## Attempted, 2026-09-25: no capture obtained, but the road there is shorter now

Set out to take the PCSX2 capture the "Open" section above asks for, on Assegai/Moa
Therma (`Data\Environments\03_Track\track.vex`) to match the walk
[`pcsx2-debugger.md`](../../docs/reverse-engineering/pcsx2-debugger.md) already
documents. Built an isolated harness first (own Xvfb `:100`, own PINE slot `45007`,
own `~/.cache/oag-pcsx2-shield` data path, seeded from the existing grid savestate at
`~/.cache/oag-pcsx2/PCSX2/sstates/SCES-54748 (F8AE6FF2).01.p2s`) so this ran
concurrently with other lanes' PCSX2/RPCS3 use without colliding on the stock
`scripts/pcsx2-drive.py` paths or its `pkill -x pcsx2-qt`. That harness worked cleanly
- boot, savestate load, verified frame-stepping and screenshots all behaved exactly as
`pcsx2-debugger.md` describes.

**The blocker was reaching a raised Shield in the first place, not capturing it once
raised.** The PS2 disc has no `--give` equivalent, so the pickup has to come from an
actual Weapon Pad on the track:

- `oag-trace pads --source data/images/pulse-ps2-eu.chd --track
  'Data\Environments\03_Track\track.vex'` lists Moa Therma's weapon pads; pad 0 sits at
  world `(-516.05, 7.18, -329.96)`, push direction `(-0.544, -0.002, 0.839)`, progress
  333.9 (i.e. before the first speedup pad).
- Manual real-time driving (holding cross via XTEST, `data/scratch/drive-2026-09-25/ps2-shield/throttle.py`)
  crashed or spun the craft ("WRONG WAY") on every attempt before reaching that area -
  Moa Therma's early corners are unforgiving on cross-only input, consistent with
  `pcsx2-debugger.md`'s own note that the same holds true on the following straight.
- `oag-trace plan --gate <pad 0 centre> --gate-dir <pad 0 direction> --look-min 10
  --look-speed 0.2 --gate-half-width 10` (`crates/trace`'s pursuit planner, the same
  mechanism `pcsx2-debugger.md`'s Talon's Junction speed-pad example uses) found a
  script that crosses inside that synthetic gate at tick 563, only 5.08 units off
  centre - but replaying it into PCSX2 by **verified** frame-stepping (not wall clock;
  `data/scratch/drive-2026-09-25/ps2-shield/replay_verified.py`, pause/loadstate/
  re-pause/`advance_frames` per row, per the doc's own method) still left the craft off
  the racing line ("WRONG WAY") rather than over the pad. The plan transfers for a
  straight speed-pad approach; it did not transfer cleanly through Moa Therma's early
  turns for this gate. No weapon was ever picked up.
- Looked for a PINE-write shortcut (write the pickup/fire-request bit directly, the PS2
  equivalent of `--give shield --press square`) before spending more time driving.
  `ShipShield_Hit`'s two callers on `SCES_547.48` (`FUN_00153328`, `FUN_00155188`) both
  test `craft+0x1b8 & 0x10` for "shield running" - the same offset and bit the PSP's
  `shield-pickup.md` records for its own `fire_flags`. **That match is coincidental,
  not a transferable offset**: `scripts/pcsx2_trace_fields.py`'s independently-measured
  `CRAFT_FIELDS` table (confidence 90, live-verified 2026-09-16) puts `craft_vel_z` at
  that same `0x1b8` on the documented PS2 craft layout, and reading it live off the grid
  savestate (`craft = 0x00720f20` for that savestate, per that module's own worked
  example) came back as a tiny near-zero float, not a small bitmask - confirming it is
  velocity, not `fire_flags`. So the object `FUN_00153328`/`FUN_00155188` call "craft" is
  a *different* struct from the physics `Craft` `Ship_UpdateCraft` uses, and the PSP's
  `fire_flags`/`held` offsets do not carry over. The real PS2 fire-dispatch function (the
  counterpart of `Weapons_DispatchFire`) and the held-weapon field were not found in the
  time available.

**What is now in place for the next attempt**, none of it committed since it is all
`data/scratch/` (gitignored):

- `data/scratch/drive-2026-09-25/ps2-shield/pcsx2_isolated.py` - the isolated harness,
  reusable as-is.
- `data/scratch/drive-2026-09-25/ps2-shield/replay_verified.py` - verified-frame-exact
  script replay, reusable for any `oag-trace plan` output.
- Moa Therma's weapon pad 0 coordinates and direction, above - reusable for a retuned
  `--gate`/`--look-*` sweep without re-deriving them.

## Next Steps

0. Pose ours at the savestate's craft (`--pose`) and compare, then take a PCSX2 GS
   dump of the shell's draw. The raising recipe is in
   `docs/ghidra/functions/ps2-pulse-eu/shield-pickup.md`; the harness is
   `scripts/pcsx2-drive.py` with its globals patched for a private datapath.

1. Either retune the `--gate` pursuit plan through Moa Therma's early corners (tighter
   `--deadband`/`--brake-at`, or planning leg by leg rather than start-to-pad in one
   call), or drive it by hand with an actual pad/joystick rather than XTEST taps -
   Moa Therma's opening turns look to be the harder part, not the capture.
2. Alternatively, find the real PS2 fire-dispatch function and held-weapon field
   (read-only, `program=SCES_547.48`, starting from `Weapons_DispatchFire`'s structural
   role rather than from the PSP's offsets) and write the fire-request bit over PINE -
   the PS2 equivalent of `--give shield --press square`. Validate any offset found by
   reading it back before capturing anything, the way the `0x1b8` guess above was
   caught rather than shipped.
3. Once a raised shield is reachable either way, capture an A/B pair (no fire vs fire)
   from the same savestate by verified frame-stepping, per `pcsx2-debugger.md`, and
   compare against `oag-game`'s own `--give shield --press square` frame on the same
   team/circuit.
4. Follow `0x0029a3a0`'s draw chain (the `obj+0x38` second-base thunks) on
   `SCES_547.48`. It would turn both the plume's `draw_additive` and the
   shell's `blend_additively` into a decode at once.
