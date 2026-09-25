# HD's Cannon round: `hd_muzzleflash` is a muzzle flash, not a round body

2026-09-25. Read to wire Wipeout HD's own Cannon visuals. Pulse's reading is
[`psp-pulse-usa/cannon-quake-leachbeam.md`](../psp-pulse-usa/cannon-quake-leachbeam.md):
there, `pulse_muzzleflash.vex` rides the round for its whole flight and the
flash quad is centred on the round. **HD differs on both.** Its
`CannonBullet` shows `hd_muzzleflash` only for the round's first `0.1` s, at
the firing craft's own `cannon_flash` locator, and hides it after. The flash
quad sits on the same locator. After the first tenth of a second, the round
in flight is the bolt streak alone.

Read headless (`analyzeHeadless -readOnly` against a scratch copy of the
project) and with `llvm-objdump` for the AltiVec runs Ghidra leaves as
`<unknown>`/`halt_baddata`. Every `r2`-relative load below resolves against
TOC `0x008ad4d8`, the value every OPD entry of this class carries
(`0x00876960`..`0x00876a18`). See [memory.md](memory.md) for why a Ghidra
`r2` reference in this program can't be trusted as printed.

## The chain

```
CannonManager_FireRound      @ 0x0010fbd0  picks a pooled round and a muzzle, calls Fire
  -> CannonBullet_SetMuzzles @ 0x0012fb60  stores the two muzzle nodes and the side
  -> CannonBullet_Fire       @ 0x00130ef8  places the round and the model at the muzzle
CannonBullet_Update          @ 0x00131478  per tick: age, the 0.1 s flash window
CannonBullet_DrawQuads       @ 0x001329e8  the bolt streak and the flash quad
CannonBullet_GetMuzzleMatrix @ 0x00130b50  the muzzle's world matrix, read every call
CannonBullet_Construct       @ 0x001322c8  the model and the two textures
ShipCannonFlash_AttachToShip @ 0x002d8ec0  sorts cannon_flash nodes into left/right
ShipMuzzle_AttachToShip      @ 0x002dae70  the one Ship Muzzle node
```

## `CannonBullet_Construct` (`0x001322c8`), confidence 85

Tagged `"CannonBullet.cpp"` (`0x00784518`, TOC `0x008aa704`). It allocates a
`0x2080`-byte scene node and loads `Data\Weapons\hd_muzzleflash.vex`
(TOC `0x008aa708` -> `0x00784530`) into it through `0x002c1ec8`, whose
other arguments are not read. It binds
`hd_muzzleflash.rcsmodel` (`0x008aa70c` -> `0x00784550`) and stores the node
at `+0x240`. Two textures follow: `Data\Weapons\Textures\Cannon_bolt` at
`+0x1f4` and `..\Cannon_muzzle_flash` at `+0x1f8`. They have no extension;
they resolve on `DATA02` as `cannon_bolt.gtf`/`cannon_muzzle_flash.gtf`. The
constructor also seeds the four bolt colour floats `+0xf4..+0x100` to `1.0`
and the flash colour word `+0x124` to `0x00ffffff`.

`0x00132658` is the same body under a second OPD (`0x008769f8`), most
likely the base-object/complete-object constructor pair seen elsewhere in
this binary (see [weapons.md](weapons.md)). It is not renamed.

The vtable is `0x00864188` (TOC `0x008aa678`). The slots read here:
`+0x1c` -> `CannonBullet_DrawQuads`, `+0x38` -> `0x00130cf8` (a wrapper that
calls `CannonBullet_SetMuzzles` and then `+0x3c`), `+0x3c` ->
`CannonBullet_Fire`. `0x001322c0` is a one-instruction branch to
`CannonBullet_Update`.

## `CannonManager_FireRound` (`0x0010fbd0`), confidence 75

It takes the next pooled round (`manager + 0x148`, indexing the pool at
`manager + 0x58`) and reads one bit of the firing craft's `+0x164` (seeded
from the weapon-stats table at `0x0010f33c`). It then calls the round's
vtable `+0x38` with three arguments:

| bit 0 of `ship+0x164` | node passed as `r4` | side byte `r6` |
| --- | --- | --- |
| clear | `ship+0x5f38` (the `left` node, below) | `1` |
| set | `ship+0x5f3c` (the other) | `0` |

It also passes `r5 = ship+0x5f34`, the `Ship Muzzle` node. `f1` is a float
read through the craft and scaled by a TOC constant, not read further.

## `ShipCannonFlash_AttachToShip` (`0x002d8ec0`), confidence 85

This is the `cannon_flash` (`0x3eb`) node's attach step, next to
`ShipCannonFlash_Importer.h`'s registration at `0x002d8fd8`. It walks up the
parents to the craft node. It lowercases its own name (`0x0016cf58`) and
calls `strstr` (`0x00676a88` -> `0x0043e0c0`) against `"left"`
(`0x007a1958`, TOC `+0x69d4`). A match stores the node at **`ship+0x5f38`**;
anything else stores it at `ship+0x5f3c`. With no craft ancestor it hides
itself (`+0x34 &= ~2`). All 39 `Locators.vex` on the disc name both
`cannon_flash_left` and `cannon_flash_right`; this was checked by grepping
for the names, not by walking the nodes.

`ShipMuzzle_AttachToShip` (`0x002dae70`, confidence 80) is the same walk for
`Ship Muzzle` (`0x3e2`). It stores to `ship+0x5f34`.

## `CannonBullet_GetMuzzleMatrix` (`0x00130b50`), confidence 80

It reads the owning craft (`round+0x12c`) and its byte `+0x5f42`:

- **clear**: returns node `+0x108`'s world matrix (`+0x38`) verbatim. That
  is the `cannon_flash` node `CannonManager_FireRound` chose.
- **set**: returns node `+0x104`'s (the `Ship Muzzle`) matrix, with
  translation moved by `-2.0 * row1` (TOC `0x008aa680`). It is then moved by
  `+/- 2.5 * row0` (`0x008aa688`), the sign taken from the side byte
  `+0xf0`.

What sets `ship+0x5f42` is **not read**. The engine takes the clear branch.

The node's matrix is read every call, so the flash **follows the craft**
rather than staying where the shot was fired.

## `CannonBullet_Update` (`0x00131478`), confidence 75

At `0x00131f60` it adds `dt` to the age at `+0x244`. While the model at
`+0x240` exists:

- **age < `0.1`** (`0x008aa6b0` = `0x3dcccccd`, the same constant Pulse's
  window uses): it branches to `0x001320e8`. There it calls
  `CannonBullet_GetMuzzleMatrix` and orthonormalises the result: row 0 is
  normalised, row 1 has its row-0 part removed and is normalised, and row 2
  becomes their cross product. It then rolls three values with
  `0x0028c660` (a float range):
  - `+0x24c` = roll(`0`, `2*pi`): the flash quad's rotation.
  - `+0x248` = roll(`0.25`, `1.0`). The bounds are `0x008aa700` and a
    `lis 0x3f80` held in `r31` since `0x00131d7c`. The draw uses this as the
    flash quad's half-size before `* 3.0`. **All three model axes are scaled
    by it too.**
  - roll(`0.7`, `1.3`) (`0x008aa670`/`0x008aa6e0`) scales row 2 alone, so
    the model is stretched along its own Z.

  The model is then placed with that matrix (`0x00327500`).
- **otherwise**: it clears the low three bits of the model node's `+0x34`,
  which hides it.

Every tick, it then adds `roll(150, 255) << 24` (`0x0028c570`) to the flash
colour word `+0x124`. Pulse *writes* its alpha fresh; HD *adds* to the
previous word, so the top byte wraps. The result is still a
pseudo-random alpha, and this engine draws it from Pulse's
`[150, 255]` range.

`hd_muzzleflash.vex`'s own bounds are `x, y +/-0.6` and `z -0.18..3.27`: a
cone along `+Z`. All sixteen `cannon_flash` locators on the eight teams a
race loads have Z equal to the hull's `+Z`, from this engine's own load
report. The hull's `+Z` is forward: Assegai's `engine_flare` sits at
`z = -7.18`. So the
model is a forward flash at the barrel, not a dart.

## `CannonBullet_DrawQuads` (`0x001329e8`), confidence 80

It transforms the current position (`+0x230`, into `v30`) and the previous
one (`+0xe0`, into `v31`) by the top of the matrix stack. Then (`0x00132bbc`)
it computes `v31 += vsel(0, v30 - v31, mask)` with the mask at `0x00769cd0`.
That mask is `00000000 00000000 00000000 ffffffff`: the **w lane only**. So
the far end keeps the previous position's xyz and only takes the current
`w`. There is no fraction anywhere between the two loads and the quad build.
**The streak covers the whole previous-to-current segment**, where Pulse
leaves the near fifth uncovered.

The half-width is `0.25` (`0x008aa700`, loaded at `0x00132bac`) against
Pulse's `0.35`. It draws two crossed quads through `0x002c4ad0` with the
`+0x1f4` texture, as Pulse does.

The flash quad draws **only while `age < 0.1`** (`r2-0x2e28`). It is
centred on the translation of `CannonBullet_GetMuzzleMatrix`, not on the
round. Its half-size is `+0x248 * 3.0` (`0x008aa6f0`), it is rotated by
`+0x24c`, and it uses the `+0x1f8` texture.

**Not read:** the blend state and depth state `0x002c4ad0` binds. The engine
reuses Pulse's measured additive pipeline for both quads. `cannon_muzzle_flash.gtf`
decodes to opaque alpha with a mean red of `15/255`: a dim additive sprite
with a bright core. Much of that core sits inside the boom at the locator.

## `0x001310e8`: not renamed

This is vtable `+0x14`. It computes a view-space depth key (`0x4d30....`)
and appends the round to a render queue (`+0x630`/`+0x44b0` of a global).
It then calls `0x006778c8` with a position, a colour vector and two floats
(`r2-0x2e18`, `1.0`). The call is made in both branches, at the round
before `0.1` s and at the muzzle after. The shape suggests a dynamic point
light, but nothing pins what `0x006778c8` does. **Hypothesis, below 50: a
per-round light.** Left as `FUN_001310e8`.

## The two HD-only muzzle effects have no trigger

`WO_CANNON_MUZZLEFLASH` and `WO_CANNON_HOTSPOT` are on `DATA02`
(`/data/psys/wo_cannon_muzzleflash.pob`, `wo_cannon_hotspot.pob`). **Nothing
loads them.** The following were all searched:

- `strings -a` over `EBOOT.elf` for `muzzle` and `hotspot`, case-insensitive.
  This finds only the two `.vex`/texture paths above. There is no
  `WO_%s`-style format string that could build either name.
- The same over `DFENGINE.SPRX`.
- `grep -a` over every `.vex`, `.xml`, `.txt`, `.ini`, `.lua`, `.cfg`,
  `.dat`, `.bin` and `.pob` entry on all seven PSARCs. The only hits are
  the two `.pob` files' own internal name fields.

The executable names `WO_CANNON_SPARKS` and `WO_CANNON_SPARKS_DETONATOR`
and no other Cannon effect. These two stay unwired, and this page records
why. Confidence **80** that the retail EBOOT never plays them. An on-device
check (the psys loader at `"Data\Psys\%s.POB"`, `0x007a10b0`, never
receiving either name during a Cannon shot) would settle it.

## What the engine draws from this

`oag_hd::race::CANNON_LOOK` carries the reading above. The draw side is
`oag_game::race` (`weapons/visuals/cannon.rs`), with the locators from
`crate::livery::cannon_flash`. Two things are **chosen, not measured**:

- **Which side flashes.** The engine's round is spawned by
  `oag_gameplay::projectile::cannon::launch` a quarter-hull left or right of
  the nose, off the same kind of parity bit. The draw reads the side back
  from which side of the craft the round is on, and picks the node named
  `left` for the left. Whether that matches the original's bit sense is not
  checked against a capture.
- **Where the round starts.** The original fires the round from the
  `cannon_flash` locator itself (`CannonBullet_Fire` copies the muzzle
  matrix into `+0x200..+0x230`). The engine's spawn still uses Pulse's
  quarter-hull offset, because moving it changes the simulation and its
  hashes. So on HD the streak starts near the nose while the flash sits at
  the locator, about `2` units to the side on Assegai.
