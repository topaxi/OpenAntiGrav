---
status: re-first
titles: [pulse]
updated: 2026-10-30
summary: "Plasma matched pair ranked: blast envelope and head orb differ, detonation 3 ticks early, craft-hit second blast unwired; head and blast law unread"
---

# The Pulse Plasma's head and blast differ from the original's

2026-10-30 (`pulse-plasma-fidelity`, measure lane). Matched pairs on Talon's Junction, Time
Trial, Venom, Assegai, fire at GO + 120 (speed 106.2), original on PPSSPP software, ours at tick
`391 + k`, a no-fire control each side, three original restarts. The ranked table, method and
numbers are in `docs/ghidra/functions/psp-pulse-usa/plasma.md`, "2026-10-30: the Plasma matched
against the original". Pairs: `data/reference/pulse-plasma-pairs/`. Scoring tool:
`scripts/weapon-pair-score.py`. The craft trajectory matches to about 3 units through fire+180,
so these are effect gaps, not scene gaps.

## Open

1. **Blast envelope (3.7 M + 1.9 M px-grey, the largest).** Ours flashes at full strength 2
   frames before the original and without its two-frame ramp; the original has a white-out
   (mean G 211 to 253 over k89-93) that **cuts at k94**; ours has none; the purple tail
   (k100-140) is weaker (R at k120 125 against 190). Law: the three models of
   `PlasmaBlast_Update` (age x rate scrub, tints, **camera basis at confidence 55, ours is a stand-in
   billboard**). Needs a GE dump of the original at k88, k93 and k94 to see which model's
   draw ends at the cut.
2. **Head and charge orb (0.24 M in a 7,200 px box, 33 grey).** The original is a growing purple
   orb with a dark core and swirling arcs (about 50 px at k44-72); ours a 6 px star. The root
   emitter's texture bind of `WO_PLASMA_HEAD` is the open item (`oag_pob::texture`).
   Recipe: `psp-weapon-pair.py --ge-dump-k 36 --shots 36 plasma`, same with `--no-fire`,
   `psp-ge-dump.py census`, diff the prims inside the head's screen box.
3. **Detonation 3 ticks early** (ours 73 ticks after the press, the original 76: launch at +59,
   detonation at +76). Probe ours' launch tick and the bolt's per-tick distance against the
   original's `--probe plasma_launch` / `plasma_det` (and `Plasma_Update`, `0x0885c6cc`). Ready
   to wire once the short leg is found; confidence 60.
4. **Craft hit spawns two detonations in the original and one in ours** (`Plasma_SweepCraftHit`
   and the teardown pass, `plasma.md`, confidence 88): `Race::blast_for` returns a single
   `(Trigger, Vec3)` and `ignite_blast` spawns one model blast. Ready to wire (about 1 h with a
   test). Not captured on the original: Time Trial has no opponent; a capture needs a race with a
   craft ahead on the bolt's line.

## Next Steps

- Spawn a `oag-re` member on item 1 (GE dump of the blast frames), then a wire lane for 1-3.
- A wire lane may take item 4 alone.
- Pulse PS2 is unmeasured here; its pair is `scripts/pcsx2-pulse-pair.py`.
