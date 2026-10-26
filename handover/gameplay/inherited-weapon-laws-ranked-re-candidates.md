---
status: re-first
titles: [pure, hd, 2048, omega]
updated: 2026-10-19
summary: "Ranked list of weapon laws inherited from Pulse or HD: the fire law on four titles first, then 2048 LeachBeam"
---

# Weapon laws this engine inherits from Pulse or HD, ranked for reading

2026-10-19, lane `weapon-lineage`. The matrix is
[`docs/gameplay/weapon-lineage.md`](../../docs/gameplay/weapon-lineage.md); this
thread ranks its **inherited** cells that decide play. The ranking signature is
the one that cost a day on 2026-10-19: *a number HD, 2048 or Omega authors very
differently from Pulse, spent by a Pulse law nobody has read on that title*.
A value diff of the race tables (Pulse EU PSP and PS2, Fury `DATA00`, 2048's
`_Race_2048`, Omega's two copies, 2026-10-19, confidence 90; the fields and the
commands are in `docs/formats/weapon-stats.md`, "The race tables compared across titles") found **every weapon block equal
to Pulse's across Pulse, HD and Omega except the ones below**; so the risk is
not in the numbers, it is in what spends them.

## Open

Ranked, most play impact first. Each names what would settle it.

1. **The fire law on Pure, HD, 2048 and Omega.** Inherited from Pulse on four
   titles and run in every race with weapons on, 38 to 288 opponent projectiles
   per 240 s race (`docs/gameplay/ai.md`, "An original opponent never fired").
   Only HD's table loader is read (`ps3-hdfury-eu/weapon-ai-stats.md`, 85);
   its consumer (`RaceManager + 6240`) is not. Settles it: HD's decision
   function, then a Pulse-vs-HD diff of the roll gates. Also open from the same
   page: the absorb half's decision cadence (every tick or four a second).
2. **LeachBeam on 2048 and Omega.** 2048's own table authors `repair="30.0"`,
   fifteen times HD's 2.0 (HD and Omega's HD-named copy: 2.0). The per-trip law
   is HD's, carried by inheritance, and `energy_multiplier` is applied on the
   first trip only (2.0 here, 50 on Pulse). A 15x `repair` through a law no one
   has read on 2048 is the same signature as the bug. Settles it: 2048's
   `LeachBeamManager` update on Vita3K (the memory-poll recipe in
   `docs/reverse-engineering/vita3k-capture.md`) for the transfer per trip and
   what `repair` does to the shooter's shield. Omega is not checkable here.
3. **Quake speed per class on HD, 2048 and Omega.** Their tables author
   `venomspeed 1.0 / flashspeed 1.1 / rapierspeed 1.3 / phantomspeed 1.5`
   (2048 and Omega add `superphantomspeed 1.5`) on the Quake, which Pulse's
   authors none of; `QuakeStats` decodes no such field, so a Quake there
   travels at this engine's fixed rate whatever the class. Settles it: HD's
   `QuakeManager` update, whether the four numbers multiply the speed.
4. **`slowdown_time` on 2048.** Every weapon's is half Pulse's and HD's
   (Rocket 0.4 against 0.8, Plasma 0.7 against 1.3), against an unchanged
   global `slowdown_limit` of 1.3. `Ship_AddSlowdown`'s stacking law is Pulse's
   and unread on 2048. Settles it: the Vita3K poll of one hit's timer.
5. **`SuperPhantom` on 2048 and Omega.** Both author a `superphantomspeed` on
   Rocket, Missile, Plasma, Shuriken and Quake, and Omega a fifth `<Pickupodds>`
   class; `Weapon` decodes neither, so a SuperPhantom race flies those weapons
   at an unknown speed. Whether the class reaches the weapon table at all is
   unread (`vita_2048_original_ai_ground_truth.rs` lists the class for the AI).
6. **Cannon on HD, 2048 and Omega.** `recharge_time` (HD 0.5; 2048's
   `slowdown_time` is 0.05 against 0.1) and the Detonator's rebound and
   recharge fields are authored and unread; HD's Cannon is a muzzle flash, not
   a round, which the visual wiring knows and the law may not. Settles it:
   `CannonManager_Construct`'s update.
7. **Screen shake on 2048.** `screen_shake_amount` and `screen_shake_time`
   (0.2 to 0.3 amount, 0.3 to 0.6 s) on eight weapons; nothing decodes them.
   A look, but a hit is felt through it. Settles it: 2048's camera-shake
   consumer on Vita3K.
8. **Mine and Bomb on HD.** `Mine.slowdown_time` 0.10 against Pulse's 0.16 and
   the Bomb's `number_of_shots_to_destroy` (1). Low risk, cheap to read.
9. **Weapon cue sites on HD, 2048 and Omega.** Pulse's cue reading applied to
   them on a bet; confirmed on Pure for eight of nine race cues
   (`psp-pure-usa/rocket-and-collision-fx.md` and siblings). Settles it:
   a bank-by-bank `oag-wad sounds` census plus the trigger sites.
10. **Stale comments found while building the matrix** (not RE, a `.rs` lane):
    `Weapons::elimination`'s doc and the Pure and HD `lib.rs` comments say
    nothing opens the Eliminator table, but `load_weapons` has since
    2026-09-08; `WeaponModels::bomb_blast_pulse`'s doc says HD's Bomb
    detonation is unread, and `bomb_blast_hd` has drawn it since 2026-10-07.

Not ranked, because the matrix says `?` and a census would settle it first:
Omega's Turbo, Shield, Autopilot, Shuriken and Repulser (no constructor or
`.cpp` tag found), and every 2048 weapon constructor.

## Next Steps

1. Start at 1 or 2: the first changes every title's opponents, the second is
   the cheapest to prove (one Vita3K poll of one beam).
2. When any item lands, change its cell in the matrix to **M** in the same
   change; `just check-weapon-lineage` catches only the checked axes, so the
   aspect tables rely on the lane doing it.
3. Delete this file, and its `HANDOVER.md` line, when items 1 to 9 each have
   an **M** cell or a recorded negative.
