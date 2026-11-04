# Controller rumble

What the originals do with the controller's motors, per title. A wire lane should
need nothing else. Built so far: the player and the device backend, the RUMBLE
setting and a feel-test probe ([Player and device](#player-and-device)). **No race
event fires a pattern yet.**

Read 2026-11-04 from the executables and the assets, emulators off, so **nothing
here has a runtime trace** and every score caps at 84. Where HD and Omega agree the
law has its second binary. Evidence pages:
[HD](../ghidra/functions/ps3-hdfury-eu/rumble.md),
[Omega](../ghidra/functions/ps4-omega-eu/rumble.md),
[PS2](../ghidra/functions/ps2-pulse-eu/rumble.md). File format and census:
[rumble-xml.md](../formats/rumble-xml.md).

## Provenance per title

| Title | Motors | What the original does | Status for us |
| --- | --- | --- | --- |
| Pulse PSP | none | nothing | **chosen**, nothing to measure |
| Pure PSP | none | nothing | **chosen**, nothing to measure |
| Pulse PS2 | large 8-bit, small on/off | code: damage, destruction, a menu preview | measured (84) |
| HD/Fury | large 8-bit, small on/off | 64 patterns, 42 call sites | measured (84) |
| 2048 | none on a Vita | ships 69 files, reads none | **inherited** from HD via Omega |
| Omega | large and small 8-bit | HD's library, plus impacts and explosions | measured (84) |

**2048 and Omega: checked, differs.** Same 69 patterns, byte for byte in
`<RumbleData>`. Omega has the reader, 2048 has not.

`docs/formats/pure-status.md`'s `rumble` is a sound cue in Pure's `ship.bnk`, not a
pad effect.

## Playback law

Confidence 84: both binaries, same offsets in spirit, same constants.

**A frame is one pad update.** `<FPS>` is not read. HD and Omega update once per
frame, so a track plays at 60 Hz when the game holds 60 Hz, and slows with it.

**Voices.** Each pad has a ring of 20 voices. A request takes the next free voice;
with all 20 busy it is **dropped**.

**A voice per update:**

1. `frame += 1`, from -1.
2. If `frame` has reached both track lengths, the voice ends.
3. If the release flag is clear and `frame >= LoopEnd`, `frame = LoopStart`.
4. Output each motor's byte at `frame`, or 0 past that track's end.
5. Scale the large byte by the positional gain, if positional.
6. Both bytes are 0 if the class is masked, or if paused and the class is not UI.

**`Type` alone does not loop.** A request is a one-shot (release flag set) or held
(clear). A `Looping` pattern queued as a one-shot plays through once. A held voice
wraps until the game either **finishes** it (sets the flag: it runs to the end of
the track) or **stops** it (clears active: silence at once).

**Class gate.** The mask is 1 outside a race and 3 in one, so `Game` patterns stop
with the race and `UI` patterns play anywhere, paused included.

**Mix.** With up to 5 voices active, all count. With 6 or more, voices are walked in
ring order into five slots; a voice goes into an empty slot, else it overwrites the
lowest priority of the five, even when it is lower itself. Then:

- large motor: sum of the five, clamped to 255.
- small motor, HD: **on if any of the five is non-zero**.
- small motor, Omega: the sum, as bytes (1..5 of 255). Close to nothing on a
  DualShock 4. **Differs from HD.**

**Local player only.** Every ship site tests `ship+0x628c == 0` (HD) or
`ship+0x68fc == 0` (Omega) and goes to that ship's pad. Explosions and wall hits use
`Rumble_PlayOnLocalPads`: one voice on the active pad, or one on each of the two
pads in split screen.

**A one-player race drops the position.** `Rumble_PlayOnLocalPads` copies the world
position to the voice only in its two-player branch (Omega: `DAT_02037d00`, set to
`1 < DAT_01f999e8`, the local player count; HD: `g_PhysicsHalfStep` clear takes the
same no-position path, `0x0033fee8`). With one player a positional voice keeps a
zero position, so the gain measures the ship's distance from the **world origin**:
ids 48, 49, 50, 51 and 53 give no large motor away from it, on most of a circuit.
The small track is not scaled by the gain. Confidence 78, static. Id 59 is the
exception: its site writes the position itself after queueing (`0x013009c7`).

**No master setting** in HD or Omega: none in the executable's strings or in the
front-end files searched; the console's own setting applies. Confidence 70. PS2
has one, `VibOnOff`.

### Positional gain

Omega (HD's branch is unreadable, so **inherited**): `d` is the distance from the
voice's position to the active ship.

```text
d <= inner:             1
inner < d < outer:      (inner - d) / outer + 1
otherwise:              0
```

Applied to the large motor only. With the shipped 10 and 20 the gain falls from 1
to 0.5 and then cuts to 0. Where `inner >= outer` it is a hard edge at `inner`.

## Triggers

`id` as on the [format page](../formats/rumble-xml.md#every-pattern). Unless the
row says otherwise a request is a one-shot on the ship's own pad and needs the
local player. "Omega" names the function holding the site.

| id | Pattern | Event | HD | Omega | Conf |
| ---: | --- | --- | --- | --- | ---: |
| 0..6 | `collision_*_looping` | wall contact, held; [below](#contact-and-landing) | `0x000d1348` | `0x012f8821` | 82 |
| 7..10 | `landing_*` | landing; [below](#contact-and-landing) | `0x000d123c` | `0x012f8bb0` | 82 |
| 12 | `pad_speed` | speed pad | `0x0029fe98` | `0x016f2a96` | 75 |
| 13 | `pad_weapon` | weapon pad taken, local | `0x000d42d4` | `0x01305715` | 80 |
| 14, 15 | `RLR`, `LRL_barrell_roll` | barrel roll, by direction | `0x000dad14`, `0x000dacf4` | `0x012f56a5`, `0x012f5a49` | 78 |
| 17 | `Wrong_way` | wrong-way warning shown | `0x000943d0` | `0x0126151b` | 84 |
| 18 | `Turbo_Start_Rumble` | a boost effect starts, event unread | `0x0029fc70` | `0x0129aa57` | 60 |
| 19 | `Fire_Engine` | afterburner ignition | `0x000d6328` | `0x01307b7b` | 78 |
| 20 | `Enter_Mag_Rumble` | mag strip, rising edge | `0x000d0248` | `0x01315c36` | 84 |
| 21 | `Travel_Mag_Rumble` | over the strip, held | `0x000d0290` | `0x01315e87` | 84 |
| 22 | `Exit_Mag_Rumble` | mag strip, falling edge; finishes 21 | `0x000d01a0` | `0x01312c86` | 84 |
| 23 | `Absorb_Any_Pickup_Rumble` | weapon absorbed | `0x000e9bd8` | `0x013072fe`, `0x0176c804` | 78 |
| 24 | `Turbo_Fired_Rumble` | Turbo pickup fired | `0x0029f8e8` | `0x016f3756` | 75 |
| 25 | `Autopilot_Active_Rumble` | Autopilot engaged, held | `0x00129c0c`, `0x0012bd08` | `0x01375946` | 80 |
| 27 | `Bomb_Fired_Rumble` | weapon fired | `0x0012b910`, `0x0012bca0` | `0x01375fce`, `0x01377585` | 84 |
| 28 | `Cannon_Fired_Rumble` | weapon fired | `0x0012b9cc` | `0x01375c15` | 84 |
| 29 | `Leech_Fired_Rumble` | weapon fired | `0x0012bac0` | `0x0137701f` | 84 |
| 30 | `Mine_Fired_Rumble` | weapon fired | `0x0012bb14` | `0x01376a8e` | 84 |
| 31 | `Missile_Fired_Rumble` | weapon fired | `0x0012bc00` | `0x01376361` | 84 |
| 32 | `Plasma_Fired_Rumble` | weapon fired | `0x0012bb60` | `0x013766fe` | 84 |
| 33 | `Quake_Fired_Rumble` | weapon fired | `0x0012bbb0` | `0x01376531` | 84 |
| 34 | `Rockets_Fired_Rumble` | weapon fired | `0x0012bc50` | `0x01376196` | 84 |
| 35 | `Repulser_fired_rumble` | weapon fired | `0x0012ba70` | `0x013771e1` | 84 |
| 36 | `Shuriken_fired_rumble` | weapon fired | `0x0012b960` | `0x013773b1` | 84 |
| 37..44 | `*_Impact_Rumble` | hit by a weapon | **none** | `Ship_ReportWeaponHit` | 75 |
| 47 | `Leeching_Energy_Rumble` | Leech beam on the local ship, held | `0x001535e4`, `0x00153e88` | `0x0134169e`, `0x013418b0` | 78 |
| 48 | `mine_explosion_rumble` | mine explodes, positional | `0x00138e98`, `0x00140750` | `0x0133ac43`, `0x0134ded2` | 78 |
| 49 | `Missile_Rebound_Wall_Rumble` | missile wall hit, positional | **none** | `0x01352367`, `0x013547ae` and 5 more | 70 |
| 50 | `bomb_explosion_rumble` | bomb explodes, positional | **none** | `0x0133781c`, `0x01348d80`, `0x0135a578` | 75 |
| 51 | `plasma_hits_wall` | Plasma hits, positional | `0x00146204`, `0x00146264` | `PlasmaManager_Update`, 4 sites | 78 |
| 53 | `rocket_hits_wall` | rocket wall hit, positional | `0x0014bd48`, `0x0014bee4` | `0x0136b182`, `0x0136b33d` | 78 |
| 56 | `Lockon_Rumble` | reticle locks on | `Hud_UpdateMissileSight`, `Hud_UpdateLeachBeamSight` | `0x0125f743`, `0x01260956` | 80 |
| 59 | `Ship_Exploding_Rumble` | ship explodes | **none** | `0x0130097e` | 65 |
| 60 | `Ship_Exploding_Player_Rumble` | player ship explodes | **none** | `FUN_012ff700`, 4 sites | 70 |
| 62, 63 | `side_shift_left`, `_right` | side shift | `0x000daeb8`, `0x000daef0` | `0x012f5f84`, `0x012f604c` | 78 |
| 64 | `heartbeat` | three beats of a counter in `FUN_00055848`, via `Rumble_PlayOnLocalPads` | `0x000558dc` and 2 more | **none** | 60 |

**Loaded and never fired, in either binary**, from every call site of the four
entry points (HD, confidence 75) and the ring walk (Omega, 70): 16
`Energy_Critical_Rumble`, 26 `Shield_Active_Rumble`, 45 `Repulser_Impact_Rumble`,
46 `Shuriken_Impact_Rumble`, 52 `Quake_Incoming_Rumble`, 54
`repulser_out_of_range`, 55 `Shuriken_Hits_wall`, 57 `Lock_on_Warning`, 58
`Weapon_Impact_Shield_Rumble`, 61 `photo_click`.

Omega's impacts (37..44) and side shifts sit behind `*(race + 0x19b20) == 2`
(`0x012f5f62`); what that value means is unread, and it decides whether they fire in
a normal race. Two more Omega reads are open: which case of `FUN_012ff700` fires 59
and which 60, and whether the shield, Quake-incoming and energy patterns were reached by a route
the sweep cannot see. A live watch on the five voices settles both.

### Contact and landing

One function, `Ship_UpdateContactRumble`, same constants in both binaries
(confidence 82). Called once per update for the local player, with a side sign and
`dt`; `m` is the contact measure, `ship+0x64d4` on Omega, `ship+0x5f78` on HD.

**Walls, held voices.**

| `\|m\|` | Pattern |
| --- | --- |
| above 55 | 0 `collision_heavy_looping` |
| 30 to 55 | 1 right med, or 4 left med |
| 10 to 30 | 2 right light, or 5 left light |
| 2 to 10 | 3 right very light, or 6 left very light |
| 2 or less | nothing |

Left when the side argument is negative. The gap timer clears on every update in
contact. So in sustained contact the held voice is replaced **only when the band
changes**, and a new contact after more than 0.3 s of gap replaces it too. After
1.5 s of continuous contact the voice stops. With `m` at 0, after 0.1 s the voice
stops, and after 0.4 s the contact timer clears.

A pattern is held only if its `LoopStart < LoopEnd`.

**Landing, one-shots.** `v` is the craft's velocity along its up axis.

| `v` | id |
| --- | --- |
| above 6 | 7 heavy |
| 4.75 to 6 | 8 med |
| 3.5 to 4.75 | 9 light |
| 3 to 3.5 | 10 very light |
| 3 or less | 11, no pattern |

A landing plays only if it is stronger than the one armed (a lower id). Playing one
clears a timer, and 1.0 s later the armed value returns to 11. A stronger landing
finishes the previous voice. Id 11 is why that slot has no file. The sign and unit of
`v` are unread: if down is negative the table reads the other way.

## PS2 Pulse

A different system: one level, one timer, and two motors. Full reading on
[its page](../ghidra/functions/ps2-pulse-eu/rumble.md).

- **Request**: `(duration, strength 0..255)`. Dropped during a vibration and for
  0.3 s after one.
- **Strength 240 or more** also runs the small motor, and for 4 times as long.
- **Triggers**: `Ship_Damage`, 0.1 s at `damage * 1800`, human craft; ship destroyed,
  0.5 s at 255, so 2.0 s; the `VibOnOff` option changing, 0.1 s at 255, so 0.4 s.
- **Master**: `VibOnOff`, `FE_OFF` disables. So the preview is felt only when the
  option is switched on: the off value makes the trigger return.

## Player and device

Built on `oag_input::rumble` (`Player`, `MotorOutput`, `Mode`) and
`oag_input::Pad`'s `gilrs` backend. A pattern is played by id: its index in the
`oag_tables::rumble::Library` the player owns.

**Choices the law does not state, each marked chosen, not measured:**

- **The ring walk starts at index 0.** Voices are mixed in ring order; a new voice
  takes the first free slot of the five, else overwrites the first of the lowest
  priority.
- **A held voice wraps over a real loop range** (`LoopStart < LoopEnd`), by the release
  flag and not by `Type`. Taken literally, a held `OneShot` (0 to 0) would sit on frame 0
  for ever, a stuck motor; with an empty range it plays through once.
- **The small motor is a byte.** `PS3` (HD): full on if any mixed voice is non-zero.
  `PS4` (Omega): the sum of the mixed bytes clamped to 255, 1..5 of 255 with the
  shipped 0/1 tracks.
- **A voice ends at `frame >= both lengths`** ("reached"); the original's page says
  "passed", so a trailing zero frame may differ by one.

**The setting.** `controls.rumble` is `none`, `original`, `ps3` or `ps4`, default
`ps4`. `original` is the booted title's own law (`oag_title::rumble::Rumble`: HD
`OnOff`, Omega `Sum`) and is dropped from the row on Pulse, Pure and 2048
(`Definition::drop_unavailable_rumble_original`). A title whose library is empty
plays nothing at any value; nothing loads a library into a race yet.

**Device limit, measured.** `gilrs` runs its force-feedback effects on a server
thread that ticks every 50 ms. On a virtual `uinput` pad (Linux 7.2, `gilrs` 0.11.2)
a 60 Hz pattern reached the device as one upload per 50 ms (3.933, 3.983, 4.033 s ...):
**20 Hz**, so a one-frame small-motor blip can be lost or land a tick late. Whether
that reads as authored on a real motor is a feel test; the way out is `gilrs-core`
direct or SDL3 behind the same `MotorOutput`. The effects run only while a motor is
on, since a running one is re-sent every 50 ms level or not.

Reproduce without hardware (needs `/dev/uinput` writable; `uv` fetches `evdev`). The
virtual pad reports `Xbox 360 Controller` with rumble and prints each request:

```sh
uv run --with evdev python3 -I scripts/virtual-ff-pad.py 30 &
oag-game --no-audio --rumble-pattern heartbeat data/images/hdfury-ps3-eu.iso
```

Read the `UPLOAD ... strong= weak=` and `EV_FF` lines: one pair per 50 ms while a motor is
on, then `EV_FF value=0` and `ERASE` once the probe has silenced it.

**Silence** (`Pad::silence`, via `Controls::silence_motors`) runs on a pause, a
window losing focus or hiding, leaving a race or the front end, exit
(`ApplicationHandler::exiting`), a pad disconnect (its effects are forgotten) and in
a panic hook (`oag_input::pad::silence_on_panic`).

**Feel-test probe** (`dev-cli` builds): `oag-game --no-audio --rumble-pattern
<Name|file.toml> [--rumble-hold SECS] <disc image>` opens the pad with no window,
plays one pattern at 60 Hz through the same player and backend, prints each level
it sends and silences the motors. A looping pattern is held for `--rumble-hold`
seconds (default 2), then finished.
