# The Shield pickup's shell on HD/Fury: same law, two colours moved

**Binary:** `ps3-hdfury-eu` `EBOOT.elf`.
**Status:** the constructor, the activate, the per-frame update and the hit
flash are read statically and cross-checked constant for constant against
[`psp-pulse-usa/shield-pickup.md`](../psp-pulse-usa/shield-pickup.md). The
raised shell and the amber hit flash are watched on RPCS3 and paired against
ours (last section). The static-only sections below were scored before that
runtime leg.

This page was opened because the user, who plays the originals, reported that
HD/Fury's shield "renders and animates, it just does not look exactly like in
the original (shape/color)." The rendering code already drew HD's shield
through Pulse's own recovered constants (`crates/render/src/shield.rs`,
before this page), title-blind - nobody had read HD's own `Shield.cpp`.

## Found by its own debug tag, the same way Pulse's memory-mapped strings were

`search_strings "Shield"` on `/hdfury/EBOOT-ps3-hdfury-eu.elf` turns up
`Shield.cpp` at `0x00783b10`, sitting in a run of literal strings that
includes `%s\vr_shield_cockpit.vex` (`0x00783b38`), `%s\%sshield.vex`
(`0x00783b90`) and `Data\Weapons\vr_shield_cockpit.vex` (`0x00783ed8`) - the
same two format strings and the same literal fallback path
[`shield-pickup.md`](../psp-pulse-usa/shield-pickup.md) reads off Pulse's
binary. `Data\Weapons\vr_shield_cockpit.vex` is byte-for-byte the path
`oag_livery::cockpit_shield` already hardcodes from
`oag_pulse::race::COCKPIT_SHIELD` - which this page confirms is not a Pulse
constant leaking onto HD by accident, since HD's own executable builds the
identical literal.

`0x00783b10` is a PPC64 class carrying its own `__FILE__` at object offset
`0x30`, the pattern [`README.md`](README.md) records project-wide. Its
constructor is found the same way Pulse's was: `get_xrefs_to` on the string.

## `FUN_00126418` is `ShipShield_Construct`

```c
void ShipShield_Construct(ShipShield *self, Craft *craft)
{
    CObject_Construct(self);           // base class init
    self->file = "Shield.cpp";         // object+0x30, the __FILE__ tag
    self->craft = craft;               // object+0x3c (param_1[0xf])

    team_model = config("FE_TeamModel") ?: "ship";     // literal default
    cockpit_path = "%s\\vr_shield_cockpit.vex" % "Data\\Weapons";
    self->cockpit = load_model(cockpit_path);          // object+0xfc

    shell_path = "%s\\%sshield.vex" % (craft->team_dir, team_model);
    self->shell = load_model(shell_path);               // object+0x100
}
```

Both format-string builds and both model loads are the same shape as Pulse's
`ShipShield_Construct` (`0x0885db38`): a fixed cockpit path with no per-team
axis, and a team-directory-plus-config-prefix path for the shell, with the
same literal `"ship"` fallback for the `FE_TeamModel` key. Confidence **80**:
every string and every call target reads directly off the binary, the same
static-only ceiling Pulse's own Construct carries.

A second OPD entry, `0x001267b0`, decompiles byte-identical to this one - two
vtable/OPD slots resolving to the same code and the same TOC. Left unnamed:
nothing distinguishes what the second slot is *for* (a copy constructor? a
second construction path reached from one of the many game-mode branches in
its one known caller, `FUN_000dbdc8`?) without reading that caller in full,
which its size and this pass's TOC-corrupted decompile did not allow.

## `FUN_00125ae8` is `ShipShield_Activate`, and here the law moves

```c
void ShipShield_Activate(ShipShield *self)   // one argument (2026-11-03: no colour parameter)
{
    self->rgba = g_colours.activation;      // 0x00994000+0x10, see below
    self->rgba_target = target;             // an ARGUMENT, not a constant
    self->colour_rate = 0.15;               // DAT_008aa1e8, matches Pulse
    self->swell = g_swell_on_activate;      // a second runtime global, 0x008c18a4
    self->swell_target = 1.0;               // literal 0x3f800000, matches Pulse
    self->swell_rate = 0.2;                 // DAT_008aa1c8, matches Pulse
    self->active = true;
    self->fading = false;
}
```

**(Corrected 2026-11-03, see "The steady-state colour is read" below: `ShipShield_Activate` takes no colour argument; the target is a per-team material value.)**

**The steady-state colour is a call argument on HD, not a hardcoded
constant.** Pulse's `ShipShield_Activate` writes `TARGET_COLOUR` (white)
itself; HD's takes it from the caller. The one located caller,
`FUN_000d5610`, is the same function `get_xrefs_to` on `Shield.cpp` and
`vr_shield_cockpit.vex` both named as the constructor's own caller's sibling -
consistent with it being HD's `Shield_Fire` counterpart - but it is large and
this binary's TOC defect (see [`README.md`](README.md)'s "code xrefs are
sound, data xrefs are not") makes its own literal reads unreliable without
`scripts/ps3-toc.py`, which this pass did not run against it. A `ShieldColour`
config key exists and is parsed elsewhere (`FUN_000dba30`, confirmed by the
literal string `ShieldColour` at `0x00782148`/`0x007833a8`/`0x00783b00`), so a
per-team override is a live hypothesis, not a confirmed one. **Left as
`TARGET_COLOUR`, Pulse's white, in the port - chosen, not measured**; see
`oag_render::shield::HD_PALETTE`'s own doc comment.

`0.15` and `0.2` are the exact same literals Pulse's Activate writes for
[`COLOUR_RATE`](../../../../crates/render/src/shield.rs) and
[`SWELL_RATE`](../../../../crates/render/src/shield.rs), and `1.0` is the
same swell target. Confidence **82**.

## `FUN_00125660` is `ShipShield_Hit`

```c
void ShipShield_Hit(ShipShield *self)
{
    self->rgba = g_colours.hit;    // 0x00994000+0x00, see below
    self->swell = 1.1;             // DAT_008aa1ac, matches Pulse's SWELL_ON_HIT
}
```

Five stores, the same shape as Pulse's `ShipShield_Hit` (`0x0885eb04`) - a
colour write and a swell write, nothing else. `1.1` is the literal Pulse's own
Hit writes for [`SWELL_ON_HIT`](../../../../crates/render/src/shield.rs).
Confidence **88** (80 static, raised by the live read of the values it writes,
last section).

Two callers, `FUN_000e8de0` and `FUN_000eadb8` - the same dual-consumer shape
`shield-pickup.md`'s "two drains and a contact loop" section documents for
Pulse (a weapon-damage absorb and a contact-loop absorb). Neither decompiled
in this pass; recorded as a corroborating shape, not independent evidence, and
left unnamed.

## `FUN_00126108` is `ShipShield_Update`, read almost line for line against Pulse's

```c
bool ShipShield_Update(double dt, ShipShield *self)
{
    if (!self->active) return false;                  // object+0xf4

    steps = (int)(dt * 59.9999924);                    // DAT_008aa20c, ~60 Hz
    for (i = 0; i < steps; i++)                         // colour, object+0xc0..0xcc
        self->rgba[c] += (self->rgba_target[c] - self->rgba[c]) * self->colour_rate;
    for (i = 0; i < steps; i++)                         // swell, object+0xe4
        self->swell += (self->swell_target - self->swell) * self->swell_rate;

    n = flicker(self->time);                            // object+0xf8, a wrapped call
    alpha = self->rgba.a * (n * K1 + K2);                // K1/K2 corrupted by the TOC
                                                          // defect in THIS function's
                                                          // own decompile - see below
    if (craft->external_camera_flag == 0)                // craft+0x5f42
        ShipShield_DrawCockpit(self);                    // FUN_001258d8
    else
        ShipShield_DrawShell(self);                      // FUN_00125d98, unread

    self->time += dt;
    if (self->fading && self->rgba.a <= 0.1)             // literal in the InitColours
        self->active = self->fading = false;             // block, see below
    return self->active;
}
```

Every sub-computation lines up with Pulse's `ShipShield_Update`
(`0x0885e254`) field for field: the fixed-60 Hz substep loop (Pulse's own
`(int)(dt / 0.016666668)`, HD's reciprocal `dt * 59.9999924` - the same
boundary, multiplied instead of divided), the per-channel colour lerp at
`object+0xc0..0xcc` toward `object+0xd0..0xdc`, the swell lerp at
`object+0xe4` toward `object+0xe8` at rate `object+0xec`, and the external
`craft+0x5f42` camera flag choosing between two draw calls exactly the way
Pulse's `craft+0x6d` does.

**`K1`/`K2` were not read directly** - `get_xrefs_to` on the `0.25`/`0.75`
literal pool addresses (`0x008aa210`/`0x008aa214`, found by direct search
against the same literal pool that pinned every other constant on this page)
returns only the TOC-corrupted `cosf4`/`cosf4fast` false attributions this
binary is known for (see [`README.md`](README.md)'s "code xrefs are sound,
data xrefs are not"), not `ShipShield_Update` itself. They are recorded as
[`ALPHA_FLICKER`](../../../../crates/render/src/shield.rs)/
[`ALPHA_MID`](../../../../crates/render/src/shield.rs) anyway on **structural**
evidence: they sit in the identical literal-pool block as every other
constant this page confirmed by direct consumer (`1.1`, `0.2`, `1.8`, `0.012`,
`0.15`, `0.1`, `1.0`, all exact bit-for-bit matches to Pulse's own values),
and the shape of `Update`'s alpha computation - one input colour's alpha times
a `flicker*K1 + K2` term - has no other candidate pair in the same pool.
Confidence **84** for the function as a whole (the load-bearing structural
match), noted separately because the two constants themselves are corroborated
rather than directly read.

**The flicker call itself is unread**, unlike Pulse's, where it decompiled
straight to `sinf`. HD's read as `FUN_00676fe8(time, active_flag, 0)` - three
arguments where Pulse's `sinf` takes one - so it is *not* asserted to be a
bare `sin`. `oag_render::shield::flicker` is kept as `sin` in the port anyway,
on the same literal-pool proximity argument as `K1`/`K2`: nothing in the
surrounding law changed shape, and a wrapped `sin` with a per-object active
gate (the second argument) would produce identical output while active, which
is the only state this page or the port's tests exercise. Flagged, not
resolved.

## `FUN_00125778` is HD's colour-table initialiser, `ShipShield_InitColours`

Pulse hides its three colour vec4s in `.bss`, written once by a
`.cplinit`-listed static initialiser no `jal` reaches
([`shield-pickup.md`](../psp-pulse-usa/shield-pickup.md#their-one-writer-is-a-static-initialiser-no-call-reaches)).
HD's equivalent is a plain function, gated on two literal arguments
(`param_1 == 1 && param_2 == 0xffff`, unexplained - possibly a one-shot
init-order token) that writes the same shape of data to a runtime global at
`0x00994000`:

```c
void ShipShield_InitColours(int a, int b)
{
    if (a != 1 || b != 0xffff) return;
    g_colours.hit        = (1.0, 0.2, 0.0, 1.0);   // +0x00..0x0c
    g_colours.activation  = (0.547126, 0.1, 0.1, 0.0); // +0x10..0x1c
    g_colours.fade         = (0.0, 0.0, 0.0, 0.0);      // +0x20..0x2c, all zero
}
```

Every literal is an instruction immediate, read directly: `0x3f800000` (1.0),
`DAT_008aa1c8` (0.2, the same address `ShipShield_Activate` reads for
`SWELL_RATE` - one literal pool, two consumers), `0x00000000`, again
`0x3f800000`; and `DAT_008aa1c0`/`DAT_008aa1c4` (`0.547126`/`0.1`) for the
second vec4. `ShipShield_Hit` copies the first vec4 verbatim on a hit;
`ShipShield_Activate` copies the second vec4 verbatim as the colour it starts
at. The third vec4, all zero, is never read by any function this pass found -
structurally the fade target, matching Pulse's `FADE_COLOUR`, but unconfirmed
by a consumer. Confidence **78**: the values are as solid as anything else on
this page, one notch down only because the gating arguments and the third
vec4's consumer are unexplained.

### The two colours that moved

| | Pulse | HD | |
| --- | --- | --- | --- |
| Hit flash | `(0, 1, 1, 1)` cyan | `(1.0, 0.2, 0.0, 1.0)` **amber** | a real, confirmed difference |
| Activation start | `(0, 0, 0, 0)` transparent black | `(0.547, 0.1, 0.1, 0.0)` dim red, alpha 0 | still invisible at alpha 0; only the hue the fade-up starts from moves |
| Steady-state / target | `(1, 1, 1, 1)` white | unresolved (see above) | ported as Pulse's white, **chosen not measured** |

Cyan against HD's mesh's own authored vertex colour (unread on this pass -
[`shield-pickup.md`](../psp-pulse-usa/shield-pickup.md) records Pulse's as
`(0.55, 0.50, 0.91, 0.50)`, blue-violet) would have read as a brightening;
amber against the same family of colour reads as a warmer flash rather than a
hue-shift-to-cool - a genuinely different effect, not a re-styling of the
same one.

## `FUN_001258d8` is `ShipShield_DrawCockpit_q`

Confidence **68** (`_q`, per [ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md)):
the scale arithmetic is confirmed exactly, the draw call itself is not fully
traced. Called from `ShipShield_Update`'s cockpit branch with the object's
own flicker value as an argument (`param_1`, a double); the body computes

```c
cockpit_scale = (swell + 0.012 * (1 + flicker)) * 1.8;
```

where `0.012` (`DAT_008aa1d0`) is Pulse's [`SCALE_BASE`/`SCALE_FLICKER`
(`crates/render/src/shield.rs`)](../../../../crates/render/src/shield.rs) -
the same literal doing both jobs, since they are equal on HD exactly as they
are on Pulse - and `1.8` (`DAT_008aa1d4`) is Pulse's
[`COCKPIT_SCALE`](../../../../crates/render/src/shield.rs), bit-for-bit. The
algebra is `swell + BASE + flicker*FLICKER` restated with `BASE == FLICKER`
factored out, which is exactly Pulse's `ShipShield_Update` formula for
`cockpit_scale()`. `FUN_00125d98`, its sibling on the external-camera branch,
was not decompiled in this pass; named nowhere and left as `FUN_00125d98`.

## Names recovered

All below in `names.tsv` in this change.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x00126418` | `ShipShield_Construct` | 80 |
| `0x00125ae8` | `ShipShield_Activate` | 82 |
| `0x00125660` | `ShipShield_Hit` | 80 |
| `0x00126108` | `ShipShield_Update` | 84 |
| `0x00125778` | `ShipShield_InitColours` | 78 |
| `0x001258d8` | `ShipShield_DrawCockpit_q` | 68 |
| `0x00125d98` | `ShipShield_DrawShell` | 80 |
| `0x002c32c8` | `Model_SetNodeTint_q` | 65 |

`ShipShield_Hit` is scored 88: the values it writes are read live (92), the
attribution to this function is static (no program counter observed).

## What is not verified

- **No breakpoint on the object's functions.** The runtime leg reads the
  shield object's fields and the frame, not the program counter.
- **The steady-state target colour's real source.** `ShipShield_Activate`
  takes it as an argument; the call site (`FUN_000d5610`, HD's likely
  `Shield_Fire`) was not traced through this binary's TOC defect. A per-team
  `ShieldColour` config key exists and is parsed (`FUN_000dba30`) but is not
  confirmed to feed this argument.
- ~~The flicker function and the external-camera draw call~~ both read, see
  the last section.
- **The two `ShipShield_Hit` callers**, `FUN_000e8de0` and `FUN_000eadb8`.
  Their existence and count match Pulse's two-drain shape; their bodies are
  unread.
- **`0x001267b0`**, the second OPD entry that decompiles identically to
  `ShipShield_Construct`. Left unnamed.
- **HD's own per-team `<team>_shield.rcsmaterial`**, unread since before this
  page - see [`shield-pickup.md`](../psp-pulse-usa/shield-pickup.md#the-same-asset-across-the-three-titles-checked-on-all-three).
  An untextured additive shell is brighter and flatter than the disc's
  regardless of which palette tints it; the loader report says so on every
  HD boot.

## Carried to Omega and 2048

Omega and 2048 draw HD's two read colours (amber hit flash, dim-red
activation) by inheritance: `Looks.shield_palette` is `ShieldPalette::Hd`
labelled `InheritedFrom("Wipeout HD")` on both. Check status: inherited,
unread. Neither title's own shield code has been read, so this is a stand-in
law of the closest relative, not a measurement; reading Omega's and 2048's
shield constructors and HD's steady-state target colour are queued as
separate lanes.

## The steady-state colour is read (2026-11-03, `hd-shield-read`)

**Result: the target colour is the `ShieldColour` material parameter authored
in each team's `shipshield.rcsmodel`, a vec4 per team. It is neither a
constant nor a `ShieldColour` config key.** Confidence **90** (static read of
all three functions plus a live read of the shield object on RPCS3, below).

### Correction to the section above

`ShipShield_Activate` (`0x00125ae8`) takes **one** argument, the shield. Its
only caller, `FUN_000d5610`, passes `*(craft+0x6f6c)` and nothing else; the
earlier "target is an argument" reading came from a decompile whose prototype
invented a second parameter. The target is copied from a field of the shield
object itself, `object+0x104..0x110`:

```text
Activate (0x00125ae8, TOC 0x008ad4d8, exact):
  object+0xc0..0xcc   = g_colours.activation   // 0x00994010, (0.547, 0.1, 0.1, 0.0)
  object+0xd0..0xdc   = object+0x104..0x110    // the target, set at construction
  object+0xe0 = 0.15 (colour rate), object+0xe4 = [0x008c18a4] (swell start),
  object+0xe8 = 1.0, object+0xec = 0.2, object+0xf4 = 1 (active), object+0xf8 = 0
  both models (object+0xfc cockpit, object+0x100 shell): FUN_002c32c8(model,
      packed ARGB of g_colours.activation * 255)   // model tint, see open items
```

### Where `object+0x104` comes from: `FUN_00125690`, `ShipShield_BindColourParam`

`ShipShield_Construct` calls `0x00125690` once per model, cockpit first
(`0x0012676c`, model `+0xfc`) and the shell last (`0x00126694`, model
`+0x100`), so **the shell's value is the one left in `object+0x104`**:

```text
void ShipShield_BindColourParam(ShipShield *self, Model *model)
{
    mat = FUN_003e4df8(model + 0x204c);            // 0x00677008, the model's material
    h   = ~crc32("ShieldColour");                  // 0x00676ff8 on the string at 0x00783b00
    p   = FUN_005d4c68(mat, h);                    // 0x00677c88, find the param record
    self->target_colour = *p;                      // object+0x104..0x110, the authored vec4
    Material_SetInstanceParamPointer(mat, h, &self->live_colour);  // object+0xc0
}
```

So the live colour (`object+0xc0`, lerped by `Update`) is *bound as the
shader's `ShieldColour` instance parameter* on both the cockpit and the shell,
and the value it lerps toward is whatever the model's own material authors.
That is the whole "parameter": the `ShieldColour` string at `0x00782148` that
`FUN_000dba30` parses is a different use of the same name (it binds a pointer
on the craft's own models at `craft+0x7a50`), not the source of this colour.
Constructor default, before the bind, is `(1, 1, 1, 1)` (`0x008aa1f4` = 1.0,
stored to `+0xc0`, `+0xd0`, `+0x104`).

`ShieldColour` is `0xaaf53119` (`~crc32`); the `.rcsmodel` material record
carries it as a `0x20`-byte parameter entry whose value offset (`+0x18`) points
at four big-endian floats
([`rcsmodel.md`](../../../formats/rcsmodel.md), "A material record carries the
shipped values"). Read off every shield model on the disc:

| Team (`Data\Ships\<team>\shipshield.rcsmodel`) | `ShieldColour` RGBA | Archive |
| --- | --- | --- |
| feisar | (0.0000, 0.0314, 0.9961, 1) | DATA02 |
| assegai | (0.2110, 0.6567, 1.0000, 1) | DATA02 |
| piranha | (1.0000, 0.5333, 0.0000, 1) | DATA02 |
| qirex | (0.7294, 0.1843, 0.5490, 1) | DATA02 |
| goteki | (0.7490, 0.2039, 0.7725, 1) | DATA02 |
| egx | (0.2196, 1.0000, 0.2196, 1) | DATA02 |
| ag_systems | (0.2652, 1.0000, 1.0000, 1) | DATA02 |
| triakis | (0.4078, 0.3804, 0.1137, 1) | DATA02 |
| zone | (0.4078, 0.3804, 0.1137, 1) | DATA02 (DATA00's older copy: (1.0, 0.3022, 0.0, **0.0**)) |
| auricom | (1.0000, 1.0000, 1.0000, 1) | DATA03 |
| harimau | (0.0000, 1.0000, 1.0000, 1) | DATA03 |
| icaras | (0.0000, 0.0314, 0.9961, 1) | DATA03 |
| mirage | (1.0000, 0.1142, 1.0000, 1) | DATA03 (DATA06's copy: (0.0, 0.0314, 0.9961, 1)) |
| `vr_shield_cockpit.rcsmodel` | (0.2652, 1.0000, 1.0000, 1) | DATA02 (its own, not used as the target) |

The DATA06 `_c1`/`_n1` variants each carry their own value (24 files, all
alpha 1; for example `feisar_c1` (0.0, 1.0, 0.7177), `goteki_c1`
(1.0, 1.0, 0.0), `assegai_c1` (0.7790, 0.7450, 0.1387)); the full census is 81 rows (every `shipshield`, `absorbeffect` and cockpit
model), by walking each shield `.rcsmodel` for entry hash `0xaaf53119`, kind
`!= 0x8001`, and reading four floats at the entry's `+0x18` offset.

The shell of the live race below, Feisar `feisar_c1` hull on the Fury campaign
default, read `(0.0, 0.031, 0.996, 1.0)` at `object+0x104` (and later at
`+0xd0`), which is the DATA02/DATA03 `feisar`/`icaras` value, not `feisar_c1`'s
(0.0, 1.0, 0.7177): so the race loaded the DATA02 `Data\Ships\Feisar\shipshield`
model, matching `shield_entry_names`' per-team path (`%s\%sshield.vex` with
`team_dir = Feisar`), the `_c1` hull variant not being a separate shield
directory. Which of the DATA06 `_c1`/`_n1` values a HD variant race would read is
**not** measured: only the base-team directory was seen live.

### Live on RPCS3 (state 5 with triangle; `scripts/rpcs3-hd-weapon.py` recipe)

Boot, Fury campaign Talon's Junction, write pickup state `5` into the slot at
`*(craft+0x5edc)+0x204`, press triangle, pause the guest every 0.6 s and read
`0x120` bytes at `*(craft+0x6f6c)` (a copy of `rpcs3-hd-weapon.py` that samples instead of recording only):

| t (s) | active | live `object+0xc0` | target `+0xd0` | swell |
| --- | --- | --- | --- | --- |
| before fire | 0 | (1, 1, 1, 1) | (1, 1, 1, 1) | 0 |
| 0.79 | 1 | (0.547, 0.100, 0.100, 0.0) | (0.000, 0.031, 0.996, 1.0) | 0.70 |
| 1.39 | 1 | (0.035, 0.036, 0.940, 0.937) | same | 0.993 |
| 2.00 | 1 | (0.001, 0.032, 0.994, 0.998) | same | 1.000 |
| 2.60 on | 1 | (0.000, 0.031, 0.996, 1.0) | same | 1.000 |

This confirms in one run: the activation start `(0.547, 0.1, 0.1, 0)` and the
constructor default; `+0xd0` takes the per-team value, which equals the
authored Feisar `ShieldColour` read statically from the disc; the colour lerp
reaches the target in about 2 s (rate 0.15 per 1/60 s step) and holds; the
swell starts near 0.7 and settles at 1.0. The recorded frames show a Feisar hull wrapped in a
**blue** translucent shell, with a purple-red tinge on the activation frame
(kept untracked under the lane's scratch directory). The amber hit flash was not captured in that run; it is in
the last section.

A later sample after the same press with state `6` showed the fade path
(`fading = 1`, target `(0, 0, 0, 0)`, swell `1.166` then `1.194`, then
`active = 0`), consistent with the third colour vec4 being the fade target; the
trigger that ended the shield was a second fire press, not a measured timer, so
this is left at the structural reading.

### What differs in shape and colour from our port

- **Colour (measured):** ours draws every HD team with `HD_PALETTE.target` =
  white; the original lerps to the team's authored colour, blue for Feisar. The
  wire lane needs per-team data: read `ShieldColour` out of the team's
  `shipshield.rcsmodel` material record (the reader
  `oag_rcs::rcsmodel::material::parameters` already returns it) instead of the
  palette's constant. Zone's `shipshield` has two values on disc (DATA00
  `(1, 0.3022, 0, 0)` with alpha 0; DATA02 `(0.4078, 0.3804, 0.1137, 1)`): HD
  loads the later archive's, to be confirmed with the existing archive-priority
  rule.
- **Activation tint:** `Activate` also writes the packed activation colour onto
  both models through `FUN_002c32c8`; every drawn frame then overwrites it (last
  section), so it is not a separate effect.
- **Shape (not measured here):** the original's shell comes from the
  `.rcsmodel` (the per-team `.vex` is a 2 KB wrapper); the existing loader
  report says ours draws it untextured and additive. No side-by-side of our
  frame was made in this lane (`--give shield` was not reliable, see the
  handover thread). The visible difference in the original frames is the
  hexagonal translucent pattern from `shieldhexagonal_alpha.gtf`, which an
  untextured shell cannot show.

### Names

| Address | Name | Confidence |
| --- | --- | --- |
| `0x00125690` | `ShipShield_BindColourParam` | 90 |
| `0x000d5610` | `Craft_ActivateShield` | 82 |

`FUN_000d5610` reads `GameState`, queues the `shieldactive` announcer cue when
the race manager's mode is 2, calls `ShipShield_Activate`, and then starts the
`_SHIELD` HUD message (`FUN_002fcc20`); it is reached from two fire handlers
(`0x00129690`, `0x001297d0`, handler-table rows at `0x00876560` and
`0x00876570`) that clear the slot's state to `-1` and set slot bit `0x10`. Live,
state 5 plus triangle ran it. Which of the two handlers state 5 selects is
unread.

### Wired (2026-11-03, `hd-shield-wire`)

**Ported on HD.** `oag_livery::shield` reads `ShieldColour` (`0xaaf53119`) off the
shell's `.rcsmodel` material at load (`Livery::shield_colour`); the race
builds each slot's `ShipShield` with that colour as the palette `target`
(`oag_raceplay::Setup::shield_targets`). A shell with no such parameter (every
PSP and PS2 shell, a `.vex`-only source) keeps its title palette's target.
Ground truth: `crates/raceplay/tests/hd_shield_colour_ground_truth.rs` checks all
13 table rows through `oag_livery::load` and lerps a shell to each. In a race
the live colour reads `(0.0, 0.031, 0.996)` for Feisar once settled.

**Why a plain vertex-colour tint showed nothing (found in review).** On an HD
model the vertex colour is a baked light term, not a tint (`mesh.wesl`'s
`colour_is_light` path forces the tint to white), so multiplying it by the
shield colour left the shell white. The shell is now a `Model::shield_shell`
(set when its material authors `ShieldColour`): its vertex RGB is white at load
(the alpha is the authored `VertexColour1.w`, last section), the race writes the
shield colour into it each frame, and `shade.wesl` multiplies the composed
output by it (`shield_shading`). The multiply is the stand-in for
"`ShieldColour` is a shader instance parameter" (shell frames: Feisar deep
blue, Piranha orange, EGX green).

**Zone.** Our archive order mounts `DATA00` first, so a Zone shell loads
DATA00's copy `(1.0, 0.302, 0.0, 0.0)`. Normal Zone mode has no shield (maintainer), so
the alpha-0 target is consistent and not a bug. Zone Battle (Fury) may differ;
unverified. The ground-truth test pins what ours resolves.

### Carried to Omega and 2048

**Checked, applies, not wired.** Both ship `shipshield.rcsmodel` in the PS4/Vita
container (magic bytes `ed ad 5c ca`, 8-byte pointers) with a `ShieldColour`
parameter entry (`0xaaf53119`, offset 1520 on Omega's feisar/assegai/piranha/
qirex/goteki/egx/ag_systems/triakis/auricom/mirage, 1528 on harimau/icaras; offset
1252 on 2048's feisar, piranha, auricom, mirage). The entry's value pointer is a
relocation-style offset that reads zeros when followed naively, so the values are
**not decoded**. Neither title draws a shield shell today: the PS3 shell build
refuses these files (`version 0xedad5cca, expected 0x000a0000`, the loader-report
line `shipshield.vex: its .rcsmodel does not build`), so there is no shell to
tint. Both stay on the inherited palette until the Vita/PS4 shell build and the
value decode land.

## The lattice scrolls

2026-11-26, lane `shield-anim`. The shell and the cockpit sphere both use
`data/materials/ships/hexagonalshield_rich.rcsmaterial` (every team, cockpit
included; the one exception is below). Its fragment program samples the one
texture **twice**, read with `scripts/ps3-microcode.py fp-file` (confidence
**90**: static, but the same literal in all 25 variants that read the clock and
the same law as the 2048 and Omega recompiles; no RPCS3 frame of a raised
shield was compared):

```text
unlit block #1 (0x3450), the shape every colour-writing variant repeats:
@0x00  MOV R0.y, f[TC0].w          // u  (vertex program: o[TC0].w = Uv1.x)
@0x01  MOV R1.w, {time}            // parameter 0x906b67ba
@0x06  TEX H2.w, R0.zwzz           // alpha at (u, v)
@0x08  MAD R0.z, R1.w, 3, R0.y     // u + 3 * time
@0x0b  TEX H0.xyz, R0.zwzz         // colour at (u + 3 * time, v)
@0x0c  MUL H0.xyz, H0, ShieldColour
@0x0e  MUL H1.w, H2, ShieldColour  // alpha: tex.a * ShieldColour.a
@0x10  MUL H0.w, H0, H1            // times VertexColour1.w
@0x11  MUL H0.xyz, H0, constantAmbientColour
```

The shell's texture (`<team>/livery1/shieldhexagonal_alpha.gtf`, 256 x 128)
holds **the hexagon lattice in alpha and five brightness bands in RGB** (white to
black, left to right). So the lattice stands still and a highlight sweeps through
it, three texture widths a second (one wrap every third of a second). The
cockpit sphere's `data/weapons/textures/shieldhexagonal_alpha_internal.gtf` uses
the same material. `time` is the engine clock
([rcsmaterial.md](../../../formats/rcsmaterial.md), "A surface scrolls off an
engine `time`").

**Wired** (`oag_mesh::mesh::rcs::shield_scroll`): the rate is followed through
the program's data flow (`oag_rcs::rcsmaterial::fragment::Program::clock_scroll_rate`:
the `time` patch, a `MAD` by an unpatched literal, a texture read of that lane as
`u`), never typed. It rides the glow table as `PROGRAM_SHIELD_LIGHTNING` with a
zero lightning weight. Unwired from the same program: `constantAmbientColour`,
the lit variants' light terms and the fog (our shell reads `Scene::off`).

**It had never moved for a second reason.** The shield shell and cockpit sphere
kept their own scene block, which nothing wrote, so `scene.time` stood at zero in
every title's shader-side shield term, including the Omega lightning already
wired. They now read the flame block (`Scene::off` plus the race's animation
clock, `Race::world_clock_seconds`: tick-driven, never the wall clock).
Pinned by `raceplay::tests::scene_blocks::the_shield_shell_and_sphere_read_the_clocked_block`,
`raceplay::hd_shield_colour_ground_truth`, `rcs::rcsmaterial_ground_truth::the_shield_shell_material_scrolls_its_colour_at_three_a_second`
and `game::shield_shell_scroll_ground_truth`.

**Zone differs.** Our archive order loads `DATA00`'s Zone shell, whose material is
`zonebattle_shield` (two textures, a plasma), not `hexagonalshield_rich`. Its
program is unread and nothing scrolls it. Plain Zone raises no shield and Zone
Battle has no `Mode` here, so no race of ours shows it.

**2048 and Omega: ported** (their own recompiles of the same tap, plus a
lightning layer: [Vita](../vita-2048-eu-v104/shield.md),
[PS4](../ps4-omega-eu/shield.md), "The lattice scrolls").

## Watched on RPCS3: the raised shell and the amber flash (2026-11-27)

Lane `hd-shield-flash`. RPCS3 0.0.43-20574 restored from the weapons-on Talon's
Junction grid state (Racebox Single Race, Venom, Feisar), Pilot Assist byte
written off, the `hd-fury` compare profile (scale 100). The pickup slot is
written with state 5 and TRIANGLE raises the shield; the craft rests on its
grid slot. Reproduce with `scripts/rpcs3-hd-shield.py` (`steady`, `flash`,
`plain`); pairs with `scripts/hd-shield-pair.py`.

### The hit flash, live

An absorbed hit needs no rival. `Ship_ApplyPendingDamage_q` (`0x000e8de0`,
static) calls `ShipShield_Hit` when the pickup slot's shield bit
(`slot+0x200 & 0x10`) is set, the shield object is active (`+0xf4`) and the
pending damage word `slot+0x120` is positive; it then zeroes the word. Writing
`10.0` there while paused and stepping one game frame:

| | live colour `+0xc0` | swell `+0xe4` |
| --- | --- | --- |
| before | (0.000, 0.031, 0.996, 1.0) | 1.000 |
| one frame after the write | (0.7225, 0.1532, 0.2764, 1.0) | 1.0640 |

`(0.7225, 0.1532, 0.2764)` is exactly `target + (hit - target) * 0.85^2` for
`hit = (1.0, 0.2, 0.0)`, and `1.064` is `1 + 0.1 * 0.8^2`: the amber
`(1.0, 0.2, 0.0, 1.0)` and the swell `1.1` after two substeps, to four
decimals, identical on two boots. Confidence **92** for both values and the
lerp (runtime read, one binary, no breakpoint).

On screen the shell turns salmon pink with a large hexagon lattice, then
magenta, and is blue again within about a quarter second. The picture trails the
RAM read: the frame paused 3 substeps after the hit still shows the blue shell,
so the lag is at least 3 ticks there.

### The shell draw: `FUN_00125d98` and the flicker

`ShipShield_Update` calls the external-camera draw with the colour
`(live.r, live.g, live.b, alpha)`, `alpha = live.a * (n * K1 + K2)`:

- `K1 = 0.25` (`0x008aa210`) and `K2 = 0.75` (`0x008aa214`), read through the
  function's own TOC (`0x008ad4d8`), confidence **92**;
- `n` is `_FSin(time)`: the thunk `0x00676fe8` reaches `0x0042fbf0`, the C
  library sine, with quadrant offset 0, confidence **90**.

`ShipShield_DrawShell` (`0x00125d98`, **80**; the block after its first call is
not disassembled by default, because the callee is marked no-return) draws model
`+0x100` at `swell + 0.012 + n * 0.012`, packs that colour times 255 as
`r | g<<8 | b<<16 | a<<24` and hands it to `Model_SetNodeTint_q` (`0x002c32c8`,
**65**), which stores the word at `+0xf0` of each of the model's mesh nodes.

So `FUN_002c32c8` is not an extra activation tint: `Activate` writes it once and
every frame the external camera draws overwrites it with the live colour and
flickered alpha.

### What reaches the shell's pixels

Both sides of the fragment program's `MUL H0.w, VertexColour1.w, ...` were read
off the disc (`hexagonalshield_rich.rcsmaterial`):

- the vertex program (block `0x3300`) copies attribute slot 3, `0x7493d450` =
  `VertexColour1`, `.w` into the varying the fragment alpha reads
  (`MOV R0.w, v[3].wwww` then `MOV o[TC2], R0`); it has no colour uniform;
- each hexagon-lattice shell declares `VertexColour1` as its own stream (four
  normalised bytes; Feisar: RGB a flat 127/255, `.w` from 0 to 178/255, most
  vertices at the top).

So the alpha is `texture.a * ShieldColour.a (live, unflickered) * VertexColour1.w`
off the stream. The per-node tint can only reach the program as that same
attribute, and a bound stream wins. Confidence **85** that the authored `.w` is
the alpha (vertex program and declaration), **75** that the flickered tint
reaches no pixel (the measurement below; node `+0xf0`'s consumer is unread). The
flicker still scales the shell, and the cockpit sphere (an inline chunk with no
declaration) keeps its flickered colour in ours, unread.

### The pairs

Ours before this lane flickered the shell's alpha and dropped the authored `.w`
(the mesh build carries it as the sun mask). Blue of the shell's own light
(shield frame minus the `plain` frame at the same lap clock, each side
separately), mean 8-bit levels over the 640 x 420 craft box at (640, 560), one
original run of each age:

| shield clock | `sin` factor | original | ours before | ours after |
| --- | --- | --- | --- | --- |
| 1.73 s | 1.00 | 19.6 | 23.6 | 14.9 |
| 3.21 s | 0.75 | 20.3 | 16.9 | 13.7 |
| 4.75 s | 0.50 | 17.3 | 10.8 | 12.7 |
| last / first | | 0.88 | 0.46 | 0.85 |

The prediction written before the wire was that ours' last-over-first ratio
would move from 0.46 to the original's 0.88 or above, and its peak level fall.

| pair | whole frame RMSE before, after | craft box RMSE before, after |
| --- | --- | --- |
| raised, shield clock 4.0 s | 0.0527, 0.0524 | 0.0973, 0.0958 |
| 1 substep after the hit (salmon pink, hex lattice) | 0.0755, 0.0743 | 0.1521, 0.1474 |

**The level, closed.** Ours added about 70 % of the original's blue at every
age here. The cause is in the next section, "The shell's draw state".

Without a shield (`plain`) the craft box matches: ours minus original is
(-0.2, 0.0, 0.0) mean levels at lap clock 7.6 and (-0.6, -0.7, -2.0) at 8.6.

### 2048 and Omega

- **Hit colour and swell:** ported. Each executable's own value is the same
  ([Vita](../vita-2048-eu-v104/shield.md), [PS4](../ps4-omega-eu/shield.md),
  read statically) and ours already takes it through `OMEGA_PALETTE`; no
  title-specific hit test, not watched live.
- **Alpha off the stream, no flicker:** checked, applies, not wired. Their shell
  programs also read the stream's `VertexColour1.w` (their pages) and their
  `Update` writes the same flickered tint per submesh; whether that tint reaches
  a pixel there is not measured, and ours' PS4/Vita shells are unchanged.

## The shell's draw state

Lane `hd-shield-blue` (2026-11-27). The raised shell's draws were read off one
frame's RSX command ring, same save, circuit and ship as the pairs (Talon's
Junction, Feisar, RPCS3 0.0.43-20574), on **two boots**. Confidence **92** for
the draw state and the patched constants (runtime, one binary). Reproduce with
`scripts/rpcs3-hd-shield.py draws --out <dir>`, then
`scripts/rsx-draw-list.py <dir> s00 --samplers`; the shell's draws are the ones
that bind the 256 x 128 hexagon texture.

| state | original | ours before |
| --- | --- | --- |
| draws | one pass in index chunks: 3 draws, 1024 triangles (boot 1), 4 draws, 1019 (boot 2), of the 2193 authored; none in a frame with no shield | 2193 triangles |
| face cull | on, `GL_BACK` (`0x405`), front `CCW` | both faces |
| blend | `SRC_ALPHA`, `ONE`, `ADD` | the same |
| colour write mask | `0x00010101`: **alpha masked off** | wrote destination alpha (the glow mask) |
| depth | test `LEQUAL`, no write | the same |
| store | `SET_SHADER_PACKER` 1: sRGB-encoded before the blend | the same |
| textures | unit 0 only: the hexagon DXT45, decoded to linear on fetch | the same texture |
| shadow maps, probes | bound on units 1 to 5 from earlier draws; the program samples none of them | received the craft shadow map |
| program | block #2 of `hexagonalshield_rich` (`+0x3770`), lit and fogged | **no light of its own: an invented stand-in rig** |

The triangle counts are about half the authored 2193; what removes the rest
before the ring is not read.

### The program

The fragment program patched into the ring, in linear light:

```text
alpha = VertexColour1.w * tex(u, v).a * ShieldColour.a
light = constantAmbientColour + directionalLight0Colour * sat(N . L)
lit   = ShieldColour.rgb * tex(u + 3 t, v).rgb * light
rgb   = lerp(fogColour, lit, exp2(-(distance * density)^2 * 1.44269)) / 2
```

The patched values on Talon's Junction, each equal to its
`environments/talons_junction/track.envsettings` to six digits: ambient
`(0.403922, 0.392157, 0.509804)`, sun colour `(2.0, 1.82745, 0.886275)`, sun
direction `(-0.7776, 0.58152, -0.23911)`, fog `(0.262745, 0.215686, 0.380392)`
at density `0.001`. `ShieldColour` was `(0, 0.031, 0.996, 1)`, the settled
Feisar value. At chase distance the fog factor is about 0.9998.

The vertex program in the ring (20 instructions, byte-identical on both boots)
is not the disc's 13-instruction block: it passes the **world-space** normal
and position, so `N . L` is the plain world dot product. Confidence **90** for
the law.

### What ours changed, one at a time

Ours lit the shell on the flame surfaces' block, which has no light, so
`shade.wesl` fell to its stand-in `0.15 + 0.75 key + 0.25 fill` rig, multiplied
in gamma space, with no ambient, sun, fog or `/2`. Each change below was
predicted before it was rendered; the blue is the shell's own light (shield
frame minus plain frame, mean 8-bit levels over the 640 x 420 craft box at
(640, 560)) at shield clocks 1.73, 3.21 and 4.75 s.

| ours | blue light | raised: grey, RMSE, box RMSE | hit flash: grey, RMSE, box RMSE |
| --- | --- | --- | --- |
| original | 19.6, 20.3, 17.3 | | |
| before | 14.9, 13.7, 12.7 | 3.438, 0.0523, 0.0958 | 6.634, 0.0743, 0.1474 |
| + back faces culled | 13.5, 12.6, 11.4 | 3.429, 0.0521, 0.0947 | 6.538, 0.0730, 0.1421 |
| + alpha masked, no shadow | identical pixels | | |
| + the lit program | 19.5, 18.6, 16.9 | 3.352, 0.0508, 0.0894 | 6.497, 0.0722, 0.1390 |

Ours over the original is now 1.00, 0.91 and 0.98 (was 0.76, 0.68, 0.73), and
last over first is 0.87 against 0.88. The program is the cause; the cull takes
blue away, and the alpha mask changes no pixel in these frames.

At 3.21 s the original's shell light also carries red and green that ours does
not (original `(2.9, 6.8, 20.3)`, ours `(0.0, 3.5, 18.6)`); one original run
per age, cause unread.

### What ours draws

`oag_title::Looks::shield_shell_lit` (HD only). A shell that authors
`ShieldColour` and runs the hexagon program culls by its own material state
and draws with `Model::shield_lit`: the law above on a block carrying the
craft's light rig and fog and the flame surfaces' clock, writing no glow mask
and receiving no shadow map. Where a circuit authors no rig, the shell draws as
before.

Left as before, because their draws are unread: the cockpit sphere, Zone
Battle's `zonebattle_shield`, and our own shield.

### 2048 and Omega, the draw state

- **Lit program:** checked, differs. The PS4's shell program is
  `ShieldColour * constantAmbientColour * diffuse` plus a lightning layer,
  with lit variants adding `liveLighting0` and `shCoeffs` terms
  ([PS4](../ps4-omega-eu/shield.md), "The two shell programs"), not HD's
  ambient-plus-sun. The Vita's lightning law matches the PS4's; its ambient
  factor is not read. Ours' PS4 and Vita shells are unchanged.
- **Back-face cull:** checked, applies, not wired. The Feisar shell's material
  state carries the cull bit `0x10` on Omega and 2048 (`0x3d`), as does 2048's
  own magnetic shell (`0x39`), and ours draws both faces there.
- **Alpha mask, blend:** not checkable: no live draw of either shell has been
  read (the Vita and PS4 emulators give frames, no command ring read here).

## History

- 2026-11-27: `hd-shield-blue`. The shell's draw state and lit program read off the command ring on two boots and wired; ours' shield blue moves from about 70 % to 91 to 100 % of the original's.
- 2026-11-27: `hd-shield-flash`. Raised shell and amber flash watched and paired; shell draw, flicker constants and sine read; shell alpha wired to the authored `VertexColour1.w` without the flicker.
- 2026-09-16: page created. `Shield.cpp`'s constructor, activate, update, hit
  and colour-table initialiser read and cross-checked against
  `psp-pulse-usa/shield-pickup.md` field for field; two colours confirmed to
  differ (the hit flash and the activation start), one confirmed to be
  parameterized rather than hardcoded (the steady-state target, ported as
  Pulse's white pending the real call site). Ported behind a
  `oag_render::shield::Palette` selected by title in
  `crates/raceplay/src/load.rs`.
- 2026-11-26: `shield-anim`. The lattice's colour scroll read off `hexagonalshield_rich` and wired; the shell's clock bug fixed.
- 2026-11-03: `hd-shield-wire`. Per-team target wired on HD; Omega and 2048 checked.
- 2026-11-03: `hd-shield-read`. Target colour read (per-team `ShieldColour` material parameter), `Activate`'s prototype corrected, live RPCS3 shield object read.
