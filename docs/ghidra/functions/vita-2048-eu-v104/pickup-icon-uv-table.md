# The held pickup's per-weapon icon: a UV table, not a widget name

Functions and data in `eboot.elf` (WipEout 2048, Vita, `PCSF00007` patch v1.04),
image base `0x81000000`. **The names here are applied**, from [names.tsv](names.tsv).
Found while wiring `oag_hud::draw::pickup_sprites` for 2048:
`docs/formats/2048-hud.md`'s "What is not done" section already established that
`HUD_pickups.xml` names exactly one icon widget (`PickupIcon`, not thirteen
per-weapon ones the way Pulse and HD's own dialects do), so the runtime has to be
rewriting that one widget's UV rect per weapon - this settles how.

## `Hud_BindWidgets` - `0x81192cd0`

**Confidence: 80**

A one-time, per-HUD-instance widget binder, found from the single xref to the
`"PickupIcon"` string (`search_strings`, one hit). Roughly ninety named widgets -
`EnergyBar`, `EnergyBg`, `EnergyBarDelay`, `PickupParent`, `PickupIcon`,
`PickupText`, `DenyPickup`, `GiftCannonBullet0`-`14`, `GiftCannon`,
`PickupBgFrame`, `Radar`, `RadarDot0`-`7`, `ObjectivePoint0`-`2`, `PilotAssist`,
and more - are each looked up by C-string name through `FUN_8106aef4` (a
"find widget by name" call, unread past this evidence) and the returned pointer
stored at a fixed offset of the struct passed in `param_1`. Called from
`Hud_InitZoneSpeedClassWidget` (`0x81197c24`, [zone-environment-fallback.md](zone-environment-fallback.md))
and two further `FUN_*` constructors below the renaming floor - each is presumably
the same binder for a different HUD skin/instance (arcade vs. Zone vs. split-screen).

The offsets this page's own two widgets bind to: `PickupIcon` at `param_1+0xa4`,
`PickupParent` at `+0xa0`, `PickupBgFrame` at `+0xa8`, `PickupText` at `+0xac`,
`DenyPickup` at `+0xb0`, `EnergyBar` at `+0x24`, `EnergyBg` at `+0x20`,
`EnergyBarDelay` at `+0x28`, `EnergyText` at `+0x1c`.

## `Hud_UpdatePickupIcon` - `0x81194f9c`

**Confidence: 85**

Reached from `Hud_BindWidgets`'s struct layout: this is one of several per-frame
update functions a dirty-flag dispatcher (`FUN_81195cdc`, itself called from
`FUN_81197fd0`, neither renamed - the flag bits are not independently confirmed
enough to cross the floor) calls when bit `0x10` of `*(param_1+0x10)` is set.
Reads the held weapon's internal id from `*(int*)(*(int*)(param_1+0xc)+0x58)` and:

- Switches a caption id on it - `case 1: "FE_ROCKETS"`, `2: "FE_MISSILE"`,
  `3: "FE_QUAKE"`, `4: "FE_TURBO"`, `5: "FE_SHIELD"`, `6: "FE_AUTOPILOT"`,
  `7: "FE_PLASMA"`, `8: "FE_MINES"`, `9: "FE_BOMB"`, `10: "FE_CANNON"`,
  `11: "FE_LEECHBEAM"` - this title's own internal weapon-id order, not
  `oag_tables::weapons::Weapon::ALL`'s (Cannon and Turbo, and Bomb and Mine,
  swap places between the two).
- On a change from the previous frame's id (`*(param_1+0x230)`), indexes a
  12-slot table at `g_pickup_icon_uv_table` (`0x81489070`, see below) by
  `id * 0x10` and writes the four values - converted `int` to `float` in
  place, the values themselves are plain integers - into `PickupIcon+0xc8`,
  `+0xcc`, `+0xd0`, `+0xd4`.
- Weapon id `1` (Rockets) is special-cased: instead of the table above, it
  reads a *second* table at `0x81489160`, indexed by a "tier" value read off
  `*(int*)(*(int*)(piVar1[0x1590]+0x80)+0x4a0)` - unresolved past the read
  itself; entry `1` of that table (`203,201,85,86`) is byte-identical to
  `g_pickup_icon_uv_table`'s own Rockets entry, and entry `0` is a different
  disc (`399,301,85,86`), so this is plausibly an upgradeable rocket tier
  with its own icon, not measured further. Confidence 60 on this half only -
  below the renaming floor, cited here by address.
- The `iVar5 == 0` branch (no pickup held, or one just consumed) restores
  `PickupParent`'s cached original `x`/`y` (`param_1+0x204`/`+0x208`,
  captured from `PickupParent`'s own fields at bind time) and hides it -
  the counterpart to `oag_2048::hud::ALWAYS_ON`'s doc comment noting
  `PickupBgFrame` is absent whenever nothing is held.

## `g_pickup_icon_uv_table` - `0x81489070`

**Named here and in `names.tsv`; not live-renamed in Ghidra.** This session's
ghidra-mcp bridge rejects a data rename outright (`name_quality`) unless it
carries a Hungarian type prefix after `g_` (`g_dwFoo`, `g_pFoo`) - a rule this
project's own existing rows (`g_zone_speed_class_thresholds`,
`g_zone_blended_stage_colour`) do not follow, so either the bridge's linter is
stricter than when those applied or is simply a mismatched convention. Left at
its raw address in the live database rather than fought further; `just
apply-names` against a fresh import may hit the same rejection.

**Confidence 90 on the address, structure and the eleven populated
entries; 90 each on the Rocket and Missile rows specifically (cross-checked
against a live frame, below).** Twelve slots (indices `0`-`11`, matching
`Hud_UpdatePickupIcon`'s weapon-id range and its 11 `case` labels), 16 bytes
each: four little-endian `int32`s, `U`, `V`, `W`, `H`, read off
`data/extracted/vita/PCSF00007/patch-v104/eboot.elf` directly:

| id | Weapon | U | V | W | H |
| ---: | --- | ---: | ---: | ---: | ---: |
| 0 | (none) | 0 | 0 | 0 | 0 |
| 1 | Rockets | 203 | 201 | 85 | 86 |
| 2 | Missile | 301 | 201 | 85 | 86 |
| 3 | Quake | 692 | 201 | 85 | 86 |
| 4 | Turbo | 7 | 301 | 85 | 86 |
| 5 | Shield | 105 | 301 | 85 | 86 |
| 6 | Autopilot | 790 | 201 | 85 | 86 |
| 7 | Plasma | 399 | 201 | 85 | 86 |
| 8 | Mines | 594 | 201 | 85 | 86 |
| 9 | Bomb | 105 | 201 | 85 | 86 |
| 10 | Cannon | 7 | 201 | 85 | 86 |
| 11 | LeachBeam | 496 | 201 | 85 | 86 |

Every `W`/`H` is `85, 86` - the same size `PickupIcon`'s own authored default
(`U="6" V="201"`, `HUD_pickups.xml`) carries, and index 10 (Cannon)'s `(7, 201)`
is that same authored default to within the ±1 px this table's `U`/`V` and the
layout's own attribute rounding can differ by - so the layout's placeholder rect
is the Cannon icon specifically, not an arbitrary default. Two slots past the
switch's range (indices 12, 13) read as all-zero; two more past *that* (14, 15)
read non-zero but outside anything `Hud_UpdatePickupIcon`'s switch reaches, so
they are left unclaimed rather than assumed to be more weapons - `Weapon::Repulser`
and `Weapon::Shuriken` have no entry here, consistent with `docs/gameplay/pickups.md`
recording neither as implemented on this title.

**Cross-checked against the running original**, not just decoded from the
table: `crates/game/examples/vita_2048_hud_atlas.rs` cuts each `(U,V,85,86)`
rect out of the decoded `hud_2048.gxt` and the result is eleven distinct,
semantically correct icons (a shield glyph for `Shield`, a compass for
`Autopilot`, a chevron stack for `Turbo`, ...). Two of the eleven are checked
against a live frame directly: the Missile grant announcement
(`~/.cache/oag/2048-hud/frames/35-w-2.png`, cropped to `PickupIcon`'s
authored top-centre rect) is pixel-identical to the table's `(301,201)` tile,
and the Rocket held in the bottom-left slot
(`~/.cache/oag/2048-hud/frames/50-r4-20.png`) matches the table's `(203,201)`
tile in shape and colour (the frame's own scene lighting dims it, so the
match is shape/position, not raw RGB).

**The held icon's on-screen rect is not read from this function** -
`Hud_UpdatePickupIcon` only ever writes the four UV fields, never `PickupIcon`'s
own `x`/`y`. Measured instead, off `50-r4-20.png`: the icon's pixel bounding box
(`x: 102-186`, `y: 405-488`, both frames scanned column/row for the disc's own
olive tint) is within 2-3 px on every edge of "`PickupIcon`'s native 85x86,
centred inside `PickupBgFrame`'s own authored rect" - `(87,400,110,95)` centred
gives `(99.5, 404.5, 85, 86)`. Confidence 80: one frame, one weapon, a close but
not exact pixel match, and no decompiled evidence for *why* it lands there -
plausibly `PickupParent`'s position is retargeted somewhere in the update chain
this pass did not fully trace (candidates: `Hud_UpdatePickupIcon`'s own
`iVar5==0`/`else` split, or the flag dispatcher's other branches), but the
measured rect is enough to wire the destination without resolving the mechanism.

## `Hud_UpdateEnergyBar` - `0x811957a2`

**Confidence: 85.** Found as the dirty-flag dispatcher's bit-`0x2` sibling of
`Hud_UpdatePickupIcon`, and originally read only for its "confirms the crop"
summary; **re-read in full on 2026-09-25** to wire the two corrections below
rather than leave them recorded and unwired
(`docs/formats/2048-hud.md#energybg-tints-energybardelay-is-wired-and-text_for-gets-2048s-arms-2026-09-25`).
Fixed-point throughout: every fraction is a `0..100` percentage read as
`SceLibm_6BBFEC89(shield * 100.0 / max)`, clamped to `0.0` on a negative or
`NaN` result, then scaled by `0.01` where the crop needs `0..1` - `fVar11` is
that percentage, `fVar12` its `0..1` form, in the variable names below.

**The crop, confirmed independently of the frame comparison the original
pass ran.** `EnergyBar`'s (`hud+0x24`) height is set to `orig_height *
fVar12` and its position (a vtable setter at `vtable+100`, not a raw field)
to `orig_y + orig_height * (1 - fVar12) * 0.5` - a bottom-anchored crop *if*
the position setter takes a rect centre rather than a corner, since a centre
shifted down by half the removed height leaves the bottom edge fixed and
moves the top down by the full removed height, matching
`oag_hud::draw::crop_vertically`'s own formula exactly. The identical
pair of computations (`height = orig * fraction`, `origin = orig +
orig*(1-fraction)`) is applied a second time to `EnergyBar`'s own two raw UV
fields (`+0xcc`, `+0xd4`), mirroring the destination-rect transform - the
same dual application (`rect` and `uv` both cropped by the same formula)
`crop_vertically` already implements.

**`EnergyBarDelay` (`hud+0x28`) gets the identical rect-plus-uv crop a second
time, fed a *lagging* fraction instead of `fVar12` directly** - read off the
same shared `orig_height`/`orig_y`/`orig_uv` fields `EnergyBar` used, so the
two widgets share one rect and source rectangle and differ only in which
fraction crops them. The lagging fraction (`hud+0x1ec`) is
`lagging = lagging + (target - lagging) * 0.1` every tick, unconditionally -
**not** gated on the shield falling, correcting this page's own first
reading. `target` (`hud+0x1e8`) is what makes that read plausible from a
summary alone: a branch sets `target = fVar12` outright the instant the
percentage *rises* versus the previous tick (`hud+0x25c`), and a second,
unconditional branch lowers `target` to `fVar12` whenever `fVar12 < target` -
which together mean `target` equals the current fraction on every tick,
rise, fall or flat, with no lag of its own. So the widget itself lags -
`lagging` chases whatever `target` last became - but the *target* never
does; the rate is symmetric in both directions, not falling-only. A `hud+0x30`
flag bit on `EnergyBar`'s own struct is set the tick `target` snaps up and
cleared once `lagging` closes to within 1% of `target`
(`fVar5 * 0.99 <= lagging`) - plausibly a "still catching up" flag some other,
undecompiled draw call reads, not chased further here.

**`EnergyBg` (`hud+0x20`), not `EnergyBar`/`EnergyBarDelay`, is what turns
red.** `FUN_8109cfae(energyBg, 0xffa7a5a7, 0)` (opaque light grey) normally,
or `0xffff0000` (opaque red) whenever `fVar11 <= 20.0` (matching this
project's own `oag_physics::damage::CRITICAL_PERCENT`) **or** a second,
independent timer (`hud+0x260`) is still running. That timer arms when the
*truncated* percentage (`(int)fVar11 < (int)fVar9`, `fVar9` last tick's own
`fVar11`) drops versus the previous tick, counts up by the tick's own `dt`,
and resets to `0` past `1.0` - structurally
`crate::hud::Readout::shield_flashing`'s own ~1s arm-on-drop shape (Pulse's
`hud+0x11c`, `docs/ghidra/functions/psp-pulse-usa/shield.md`), differing
only in comparing truncated ints rather than raw floats. Wired by reusing
that existing field rather than porting a second, near-identical timer.
Inside the red branch, a *third*, separate timer (`hud+0x1c4`) advances and
its `floor(t * 8.0)` parity clears a `hud+0x30` flag bit on a widget at
`hud+8` - not `EnergyBg`, `EnergyBar` or `EnergyBarDelay` - an 8 Hz blink on
some fourth widget this pass did not identify past the offset, so it is not
reproduced. `docs/formats/2048-hud.md` and `oag_2048::hud::ALWAYS_ON`'s own
doc comment used to read `EnergyBg` as a fixed translucent constant with
"nothing tints" - both corrected in the same change that wired this.

**Still open: what makes `EnergyBar`/`EnergyBarDelay` themselves read white
at runtime**, rather than their authored green-at-half-alpha and opaque
white respectively - no colour write to either was found in this function,
so that gap in `2048-hud.md` stands as recorded.

## `Hud_UpdateWidgets` - `0x81195cdc`

**Confidence: 82** (decompile plus the Thumb-2 disassembly of the branches
read below; no live trace of the widget values).

The per-frame HUD dispatcher the sections above are called from, entered with
`dt` in `s0` and the HUD in `r0`; each block runs when its bit of
`*(hud+0x10)` is set. The three it adds to this page, all drawn by
`oag_hud::dialect_2048::state_sprites`:

- **`SpeedBar0`-`4`** (bit `1`, bound at `hud+0x2c..0x3c`). When the float at
  `*(*(*(hud+8)+0x5640)+0x80)+0x46c` is `<= 0` (`vcmpe`, `bls 0x81195f48`),
  segment `i` is made visible (flag bit `4` of `widget+0x30`) while
  `speed >= (i + 1) * hud[0x1d0]`; `Hud_BindWidgets` stores `0x430c0000`
  (140.0) there. `speed` is the first float of the PLAYER HUD data block
  (`*(hud+0xc)`), which `Player_UpdateHudData` writes at `player+0x48` as
  `|v| * 3.6` - kilometres an hour.
- **`ThrustBar`** (bit `0x80`, bound at `hud+0x68`). `hud+0x1c8` chases the
  float at `*(hud+0xc)+8`: when below it, `+= dt * 100 * 1.4` (`0x3fb33333`),
  when at or above it `-= dt * 100`, each clamped onto the target. Then
  `ThrustBar+0xa8` and `+0xd0` (rect width and UV width) are set to
  `hud[0x1cc] * value * 0.01`, `hud[0x1cc]` being the widget's authored width
  captured at bind: a left-anchored horizontal crop.
  **The target is unresolved**: `*(hud+0xc)+8` is `player+0x50`, copied from
  the ship's `+0x578`, which `FUN_811cf532` (ShipCTRL.cpp) ramps by per-class
  rates read through `+0x84` that this project has not recovered.
- **`PilotAssist`** (bound at `hud+0x184`). Visible while the global byte
  `DAT_81545468 + 0x3260e` is non-zero, scale `1 + 0.1 * sin(phase)` with
  `phase += dt * 10` wrapping to zero at pi.
- **The `SpeedPad*` family is a bind-time choice.** `Hud_BindWidgets` binds
  `SpeedPadThrustBar` and `SpeedPadBar0`-`9` (`hud+0x40`, drawn from
  `proto_HUD.gxt`) instead of `ThrustBar`/`SpeedBar` when the same
  `+0x46c` float is positive, and the update then lights pad bar `i` while
  `i < ship+0x5e0`. That float is part of the ship definition, not a
  speed-pad contact, so the purple swoosh seen on a pad is **not** explained
  here and stays open.

## `Player_UpdateHudData` - `0x811b1ee2`

**Confidence: 75.** The `Backend/Ships/Player.cpp` update (vtable
`0x81511c48`, slot 3) that fills the block the HUD reads: `player+0x48` is
the speed in km/h (`sqrt(v.v) * 3.6` on `*(ship+0x5f84)+0x90`),
`player+0x50` the thrust level (`ship+0x5640 -> +0x578`), `player+0x68`
`+0x53c` of the same.

## `ZoneLight0`-`9`

Bound by `Hud_InitZoneSpeedClassWidget` (`0x81197c24`), driven by the tail of
`Hud_UpdateZoneSpeedClassWidget` (`0x81197d6c`): with the flash counter
`hud+0x738` at zero, light `i` is hidden while `i < 9 - (int)X` and `X != 0`,
`X` being the float at `hud+0x73c`; so `X + 1` lights, the **highest indices**,
are up, and all ten when `X == 0`. While the counter is positive it instead
blinks every light on `counter % 10 >= 6` and counts down. `X` is written
elsewhere and not read: the Zone frames show two lights at zone 2 and five at
zone 5 (the bottom dashes, `68-zone-5.png`), so `X` is the zone minus one
there. Wired as one light per zone, last index first, capped at ten:
chosen, not measured, past what those two frames show.

## See also

- [race-hud-selection.md](race-hud-selection.md) - the sibling finding this
  page's method matches: decompile the construction/update path rather than
  guess from a frame alone
- [2048-hud.md](../../../formats/2048-hud.md) - the HUD's full write-up,
  including the frame-based evidence this page's table cross-checks against
- [../psp-pulse-usa/shield.md](../psp-pulse-usa/shield.md) - `CRITICAL_PERCENT`'s
  own origin, the constant `Hud_UpdateEnergyBar`'s red threshold matches
