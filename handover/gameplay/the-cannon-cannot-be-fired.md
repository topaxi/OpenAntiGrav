# The Cannon cannot be fired, and the recovered reading says that is correct

2026-09-07, reported from play by the maintainer: **the Cannon cannot be
fired.** Asked to be specific, they describe the original's behaviour as
shooting projectiles **either by holding the fire button or by tapping it
repeatedly**.

That is incompatible with what this project implemented on the same day.
[`cannon-quake-leachbeam.md`](../../docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md)
concluded the Cannon is **self-firing**: a press sets `Weapon_RequestFire`'s
bit `0x2000`, nothing reads it, and `Cannon_UpdateReload` (`0x0883f424`) runs
every frame off its own countdown regardless. `oag_game::race::weapons`'s
Cannon arm implements exactly that - it `return`s on a press without spending
the pickup, and `Race::advance_cannons` does the firing from `race::tick`.

So in this build a press does nothing, by design. The player's experience of
that is "the weapon is broken", which is the bug as reported.

## Open

- **Which is right: the play report or the executable reading?** The play
  report is the more reliable source on this project's record, but the
  executable reading is not vague either - it has decompiled pseudocode and
  three independent facts tying held-id 3 to the Cannon. Both can be partly
  right, and the next bullet is how.
- **The prime suspect is a naming assumption, not a measurement.** The first
  line of `Cannon_UpdateReload` is
  `if (ship->weapon_pad_flags->0x16 == 0) return;`, on `ship+0x94+0x78`. It was
  read as a **track** weapon-pad flag. If it is instead the **input pad** and
  `+0x16` is the fire button, then every other fact on that page survives
  untouched and only the conclusion flips: bit `0x2000` is genuinely dispatched
  by nothing, *and* the countdown only advances while fire is held - which is
  precisely "hold to fire, or tap to fire". **This is a hypothesis raised from
  a variable name, carrying no confidence score; it has not been checked.**
- **Whether this build's self-firing path works at all is separately
  unverified from play.** If `Race::advance_cannons` fires correctly, a player
  holding a Cannon should see rounds leave with no press. The report says the
  weapon cannot be fired, which does not distinguish "fires only without a
  press, which surprised me" from "never fires at all". `cannon_ground_truth.rs`
  asserts a round leaves with no `SQUARE` pressed, so the path has passed a
  disc-backed test - but that test is the *implementation's* own claim restated,
  and it would pass identically whether or not the original gates on the pad.

## Next Steps

- **Resolve what `ship+0x94+0x78` points at and what `+0x16` holds.** Needs the
  Ghidra bridge on `psp-pulse-usa`. Confirm or refute the input-pad reading;
  either answer settles the whole thread and neither requires guessing.
- **Do not wire press-to-fire before that read lands.** It would be
  implementing from a plausible reading, which this project's methodology rules
  out - and the play report, while strong evidence that *something* is wrong,
  does not by itself say which mechanism is right.
- Once settled, correct all three places that restate the current conclusion
  together: that Ghidra page, `oag_game::race::weapons`'s Cannon arm, and
  `oag_gameplay::pickup`'s module docs.
- If the read confirms the pad gate, `cannon_ground_truth.rs`'s "a round leaves
  with no `SQUARE` pressed" assertion is asserting the bug and must be inverted
  in the same change.
