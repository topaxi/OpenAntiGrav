# Pure's weapon models, laid poses and Bomb blast

Reads what Pure's weapons draw, against `psp-pure-usa` (`/pure/BOOT-psp-pure-usa.BIN`, image base
`0x08804000`), with matched captures from PPSSPP 1.20.4 running Pure USA (software GL, 2026-10-08,
`scripts/psp-pure-weapon-fire.py`). No function was renamed, so this page adds no `names.tsv` rows.
Decompile-level readings, so 84 is the ceiling; rows below are 80 unless stated.

## Pure ships its own weapon models, and ours asked for Pulse's

`oag_pure::TITLE.weapon_models` carried Pulse's literals (`Pulse_Mine.vex`, `Pulse_Bomb.vex`,
`pulse_muzzleflash.vex`, `pulse_plasma_*.vex`), none of which is in Pure's `Data.wad` by hash, so a
Pure Mine and Bomb were never drawn and fell back to a billboard. Every path Pure's executable names
under `Data\Weapons\` and `Data\Psys\` resolves in `Data.wad` by hash (50 of 50, USA):

| Weapon | Loader | Entry | Wired |
| --- | --- | --- | --- |
| Rocket | `0x0885ed34` | `Rocket.vex`, `WO_ROCKET_FLARE`, `WO_ROCKET_EXPLO`, `WO_TRACK_ROCK_DEBRIS` | yes (before) |
| Mine | `0x0885bc28` (ctor `0x0885bab4`) | `Mine.vex` (`vr_Mine.vex` when a flag at `0x08a8..` is set), `WO_MINE_EXPLO` | model **now**; effect trigger open |
| Bomb | `0x08857fc8` (ctor `0x08857e1c`) | `Bomb.vex`, `WO_BOMB_GLOW`, `WO_BOMB_SMOKERING` | model **now**; the effects open |
| Bomb blast | `0x08858864`, `0x08858a44` (ctor `0x08858710`) | `explosion_hemisphere.vex`, `Bomb_Shockwave.vex` | **now** |
| Plasma blast | `0x0885b4f4`, `0x0885b5b0`, `0x0885b66c` (ctor `0x0885b2a0`) | `plasma_halo`, `plasma_hemisphere_noglow`, `plasma_hemisphere` | **open**: three 255-to-0 key tables of its own, not Pulse's baked track |
| Missile | - | `WO_MISSILE_HEAD`, `_EXPLO`, `_BOUNCE` | open (triggers unread) |
| Disruptor | `0x08859c70`, `0x0885a3b0`, `0x0885a8fc` | `disruptor_effect.vex`, `disruptor_cockpit.vex`, `electric_halo2.vex`, `explosion_gaseous.vex`, `WO_DISRUPTOR_*` | open |
| Quake | - | `WO_QUAKE`, `WO_QUAKE_NEW_ROCKS`, `WO_QUAKE_VR` | open |
| Cannon | - | none: no `Cannon` string | `cannon: None` |

## The laid Mine is Pulse's code; the laid Bomb is not

- **Mine** (confidence 80): `Mine_Construct` `0x0885bab4` writes `0.6` (`0x3f19999a`) at `+0xd0`;
  `Mine_Init` `0x0885bd0c` rolls a random unit axis into `+0xc0` (three `U(-1,1)`, normalised with
  `vrsq`) and calls the pose node `0x0885bf4c`, which turns by `4 x` the fuse at `+0x50` and scales by
  `+0xd0`. That is `Mine_PoseNode`'s shape on Pulse. Wired as `Looks::laid_pose_scaled`
  (`Origin::InheritedFrom`: not yet pinned to live matrices).
- **Bomb** (scale 84, spin 60): `Bomb_Init` `0x088580d8` probes the floor 10 units down, stores that
  normal at `+0x70`; the constructor `0x08857e1c` writes `+0xd0 = 0.25`, `+0xd4 = -4.0`,
  `+0xd8 = 0.5`, `+0xdc = 0.4`. `Bomb_UpdateSpin` `0x088583d8` builds a scale matrix (**`0.4` live, corrected below; the first pass read `0.25`**), two turns
  by `+0xd4 x age` and `+0xd8 x age` (each wrapped to `2 pi`), then aligns to the floor normal with
  `+0xdc`. Pulse's Bomb is square to world `+Z` at scale 1 and does not tumble. **Ours draws the
  `0.4` scale (below) and the combined yaw `-3.5 rad/s`; the two axes are not resolved - chosen,
  not measured.**
- Without it a Pure Bomb drew at scale 1 and filled the chase camera (`ours/crop-bomb.png`); at the first pass's `0.25`
  it passes under the camera in about two game updates, as the capture does.

## The Bomb blast is Pulse's pair, minus the fade

`0x08858710` writes `4.0` (`+0x18c`), `2.0` (`+0x190`), `0.1` (`+0x194`), `12.0` (`+0x198`), `0` and
`0.075` (`+0x1a0`) - Pulse's hemisphere `2 -> 4` at `0.1` and shockwave `0 -> 12` at `0.075`. Its
update `0x08858ad0` has the `1.55 s` hemisphere hide, the `0.1 s` shockwave gate and the `4.0 s`
retire, and **no vertex-colour write**: Pulse's shockwave alpha fade is absent. The Bomb has no fuse
on Pure (`weapon-stats.md`) but `BombPool_Detonate` `0x0884f214`, the only caller, fires on a craft
entering `trigger_radius`, so the blast is wired (`PulseBombBlast::shockwave_fades = false`). Pure
never loads `Bomb_Shockwave.vex` for a ship explosion (one code xref), and `wreck_fx` already throws
nothing on a title with no wreck anchors, so no ship shockwave appears. Not yet seen detonating on
either side.

## Pure lays both charges at the rear anchor, and the Bomb is drawn at 0.4 (lane pure-laid-pose, 2026-10-08)

The first pass drew a spike filling the chase camera for the Mine and a canister filling it for the Bomb,
and doubted the picture. Matched stationary captures (Time Trial, Venom, Feisar, Vineta K, craft held still,
PPSSPP 1.20.4 software, `scripts/psp-pure-weapon-fire.py`, plus ad-hoc probes in
a scratch directory, not kept) show the original draws the same thing; ours was wrong in two ways:

- **Drop point (confidence 80, one craft, two circuits).** The Mine entity's translation and `Bomb_Init`'s `a1`
  both equal the anchor matrix at `holder+0xa0` (`*(ship+0xd0)`), not the body matrix at `ship+0x40`: body
  `38.64, 37.47, -96.69`, anchor and first mine `43.52, 37.45, -96.70` on Vineta K; body `-203.14, 23.82, -84.73`,
  anchor `-198.28, 23.76, -84.42` on the second circuit. That is `4.875` (`6.5` hull units at the `0.75` craft
  scale) behind the body along `-forward`, level. Pulse's anchor is the body itself (`mine.md`, 2026-10-01),
  so this is Pure's own. Wired as `Looks::laid_from_rear` (`Measured`, Pure), offset
  `REAR_ANCHOR_BACK = 4.875` in `oag_raceplay`. **Only Feisar was measured**; other teams' hulls are assumed
  alike, chosen, not measured. Both callers of `Mine_Init` (`0x088516c8`, `0x088518e4`) pass `*param_3`, the
  anchor, so the Mine and Bomb share it. `Bomb_Init`'s floor probe only stores the normal at `+0x70`; it does
  not move the bomb.
- **Bomb scale (confidence 80).** The Bomb's node matrix (entity `+0x90`, read at every `Bomb_UpdateSpin` hit)
  has rows `0.40` long in all four samples, so the scale is `0.4` (the value at `+0xdc`), not `0.25`. Its
  position is `4.8` behind the body and wobbles `0.23` in z with the tumble. The tumble axes stay unread
  (chosen yaw).
- **Result.** With both fixed our stationary Mine sits over the rear of the hull as the original's cluster does,
  and our Bomb is the same size and place as the original's canister (`sheet-nb2.png`, `sheet-cmp2.png` in the
  scratch directory). Still missing against the original: the red `WO_BOMB_GLOW` halo around the Bomb (trigger
  unread) and our Mine's darker shading. A moving craft shows a charge for a frame or two behind it, as the
  original does.
- **Cross-title.** HD: checked, differs (its anchor is in another binary). 2048 and Omega: not checkable here.

## Capture method and what it showed

Pure USA boots in PPSSPP and walks to Time Trial by hand (language, `NEW` profile name and tag,
Single Player, Time Trial, Venom, Alpha, Vineta K, Feisar, confirm). `Ship_UpdateWeapons`
(`0x0892935c`, `a0` the ship) is hit once per game update, about 30 a second at 60 emulated Hz here.
At hit 150 after RESTART the script writes the id into `*(ship+0xd0)+0x1d8` and a `1` at
`*(*(ship+200)+0x48)+0x15`; the fire branch runs the same frame. The craft was at about 229 km/h
(Pure's HUD, `2.1 s` race time) when it fired. Sequences: a scratch directory, not kept (rocket,
mine, bomb, plasma). One boot per weapon after RESTART, repeated rocket twice; a number seen on one
boot is reported as seen once.

- **Rocket**: flare at the nose, flies off, bright orange blast on the wall (confirms Pure's own
  `WO_ROCKET_*`). **Mine**: a small spiky light-grey body drops under the tail and tumbles out of view in
  about four updates; ours is the same shape but **darker** (shading differs, open). **Bomb**: a
  grey octagonal canister with red lamps and a red glow drops under the camera for one update.
  **Plasma**: a purple ring charges at the nose, the bolt leaves with a crackle of lightning.
- Our frames: a scratch directory, not kept, fired at tick 404 of `weapon-after-go.inputs`.
  The craft state is matched by race time (about `2.1 s` after GO), not by speed: Pure's HUD km/h and
  our speed unit differ, so the pair is **time-matched**, not state-matched.

## Checked against HD and the other titles

Pure's laid poses and Bomb blast are Pure-only (HD's are in another binary): **checked, differs** for
HD. 2048 and Omega share no Pure code: **not checkable** here.
