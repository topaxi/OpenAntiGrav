# The AI Cannon's gate is an uninitialised byte, and its value is chosen

2026-09-08. This replaces "An AI craft cannot fire a Cannon here, and in the
original it can", whose conclusion was wrong and whose search was blind by
construction. The mechanism is now read end to end and the implementation
landed: **`Race::advance_cannons` runs every slot**, not just the player's.

The read is on
[`cannon-quake-leachbeam.md`](../../docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md),
section "The AI's fire-held byte is never written by anything". In short:

- The route to an AI cannon round is **forced**. `Weapon_FireCannon` has exactly
  one caller (`Weapons_DispatchFire`, on bit `0x4000` of `craft+0x1b8`), and bit
  `0x4000` has exactly one producer image-wide (`Cannon_UpdateReload`), under a
  scan that covers wide stores, combined immediates and R-type `or`.
- `field 0x16` could never have found a producer: no owner addresses the control
  record at offset `0x16`. `PlayerInput` embeds it at `controller+0x44` (held
  byte `+0x5a`); **`Ai` embeds it at `Ai+0x08`** (held byte **`Ai+0x1e`**).
- **Nothing writes `Ai+0x1e` at any width**, `Ai_Update` clears the byte *before*
  it every frame and skips this one, and the `Ai` object is allocated with **no
  zero-fill** while the craft object thirty lines away in `Craft_Construct_q` is
  explicitly memset.

So the gate is uninitialised heap. Recovered mechanism, undefined value.

## Open

- **Taking the byte as non-zero for every opponent is *chosen, not measured*,
  and carries no confidence score.** There is no fact about the disc to score:
  the value depends on what the heap block held. A race's allocation sequence is
  deterministic, so in the original it is reproducible per boot and per grid
  slot - but that is not recoverable statically, and the only two honest options
  are "always fires" and "never fires". The play report picks the first.
- **The reading makes a prediction that would falsify it, and it has not been
  checked from play.** If the byte is non-zero, `Cannon_UpdateReload`'s countdown
  advances on *every* frame the AI holds a Cannon, with **no fire decision
  involved** - so an opponent should empty its authored `rounds="30"` at
  `rate="20"`/s the moment it picks the weapon up: about a second and a half of
  unaimed continuous fire. `WeaponAi_DecideFireOrAbsorb`'s own choice writes
  `record+0x15`, which sets bit `0x2000`, which nothing reads. **If AI cannon
  fire in the original looks aimed and intermittent instead, this reading is
  wrong and something else sets that byte.**
- **Whether *every* opponent fires, or only some.** The original's answer is per
  heap block; this port says all of them. That is the coarsest part of the
  choice and the easiest to revisit if play says otherwise.
- **The round's hit and damage path is still unread** - carried over unchanged.
  `oag_gameplay::projectile::cannon::direct_hit` applies `damage_per_bullet` off
  the schema's own shape rather than off a handler. But see
  [`a-fired-cannon-round-is-drawn-by-nothing.md`](a-fired-cannon-round-is-drawn-by-nothing.md):
  `Data\Psys\WO_CANNON_SPARKS.POB` and the strings `CANNONEXPLSHIP` /
  `CANNONEXPLWALL` are now located, and the function that loads the first
  (`FUN_0886593c`, `0x0886593c`) is the obvious place to read the hit path from.

- **`craft_sticking_ground_truth::no_craft_pair_sticks_together_on_a_real_grid`
  is RED because of this change, and the bound was deliberately not raised.**
  Worst overlap streak 46 -> 95. The cause is measured and it is *not* sticking:
  every craft finishes the run `Racing` and active, and the two most
  conservative alternatives - running the countdown without spawning a round,
  and absorbing the Cannon instead of holding it - both give **68**. The old 46
  was measured in a regime where several opponents sat frozen on a useless
  pickup all race, so any fix to that moves it. Re-measuring the *pathology*
  under the new regime (`Driver::social` reverted to `Field::behind` only) gives
  **90** against the fixed driver's 95 - the statistic no longer separates the
  bug from the fix, so no bound can. The distribution still does: 10 sticking
  pairs with five over 25 ticks pre-fix, 7 with two post-fix. **That file needs
  a better statistic, not a bigger number**, and it is written up in its own
  header. This is a maintainer decision, not something to fix from here.

## Next Steps

- **Ask the maintainer the one question that settles it**: does AI cannon fire in
  the original look like an automatic burst on pickup, or aimed and intermittent?
  That is a five-second observation and it either confirms the reading or sends
  the search back out.
- If it is a burst: nothing more to do here, and the "chosen" label on the byte's
  *value* stays permanently - it is undefined behaviour in the original.
- If it is aimed: the reading is wrong. The next places to look are a store into
  the record through a pointer held somewhere other than `Ai+0x08`
  (`FUN_0893c9d4`'s registry hands the same pointer to anyone who asks for
  `"AI input %d"`), and the PS2 build, whose `Ai_Construct` may zero the byte and
  whose behaviour would then differ from the PSP's.
- **Decide what to do about the sticking tripwire**, which is the one thing this
  change leaves red. Three options, in the order they look defensible: change
  the statistic (total overlapped pair-ticks, or pairs over a floor - both
  separate 10-with-five-over-25 from 7-with-two cleanly); give every personality
  a minimal collision-avoidance floor, which the file's own header already names
  as the known follow-up and which would be *invented* tuning; or hold the AI
  Cannon behaviour until one of those lands. Do **not** just raise the bound -
  the measurement above shows it cannot separate the pathology from the fix any
  more.
- **`AiManager_Update` (`0x08834b14`) is worth a page of its own.** It is the
  per-frame chain for the whole AI subsystem - the `Ai` list at `manager+0x44`
  and the `WeaponAi` list at `manager+0x78` - and nothing in `docs/gameplay/ai.md`
  describes it yet.
