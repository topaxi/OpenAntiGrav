# The Shield pickup's shell on HD/Fury: same law, two colours moved

**Binary:** `ps3-hdfury-eu` `EBOOT.elf`.
**Status:** the constructor, the activate, the per-frame update and the hit
flash are read statically and cross-checked constant for constant against
[`psp-pulse-usa/shield-pickup.md`](../psp-pulse-usa/shield-pickup.md). No
runtime leg (no RPCS3 breakpoint), which caps every claim here at **88** per
the [confidence rubric](../../../reverse-engineering/confidence-rubric.md),
the same ceiling Pulse's own page carries for the same reason.

This page was opened because the user, who plays the originals, reported that
HD/Fury's shield "renders and animates, it just does not look exactly like in
the original (shape/color)." The rendering code already drew HD's shield
through Pulse's own recovered constants (`crates/render/src/shield.rs`,
before this page), title-blind - nobody had read HD's own `Shield.cpp`.

## Found by its own debug tag, the same way Pulse's memory-mapped strings were

`search_strings "Shield"` on `/ps3-hdfury-eu/EBOOT.elf` turns up
`Shield.cpp` at `0x00783b10`, sitting in a run of literal strings that
includes `%s\vr_shield_cockpit.vex` (`0x00783b38`), `%s\%sshield.vex`
(`0x00783b90`) and `Data\Weapons\vr_shield_cockpit.vex` (`0x00783ed8`) - the
same two format strings and the same literal fallback path
[`shield-pickup.md`](../psp-pulse-usa/shield-pickup.md) reads off Pulse's
binary. `Data\Weapons\vr_shield_cockpit.vex` is byte-for-byte the path
`oag_game::livery::cockpit_shield` already hardcodes from
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
void ShipShield_Activate(ShipShield *self, Colour target)
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
Confidence **80**.

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

## What is not verified

- **No runtime leg.** Nothing on this page has been watched in RPCS3, so
  nothing exceeds 88 - the same ceiling Pulse's own shield page carries.
- **The steady-state target colour's real source.** `ShipShield_Activate`
  takes it as an argument; the call site (`FUN_000d5610`, HD's likely
  `Shield_Fire`) was not traced through this binary's TOC defect. A per-team
  `ShieldColour` config key exists and is parsed (`FUN_000dba30`) but is not
  confirmed to feed this argument.
- **The flicker function**, `FUN_00676fe8`, takes three arguments where
  Pulse's plain `sinf` takes one. Not decompiled.
- **`FUN_00125d98`**, the external-camera draw call `ShipShield_Update`
  dispatches to. Not decompiled at all.
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

## History

- 2026-09-16: page created. `Shield.cpp`'s constructor, activate, update, hit
  and colour-table initialiser read and cross-checked against
  `psp-pulse-usa/shield-pickup.md` field for field; two colours confirmed to
  differ (the hit flash and the activation start), one confirmed to be
  parameterized rather than hardcoded (the steady-state target, ported as
  Pulse's white pending the real call site). Ported behind a
  `oag_render::shield::Palette` selected by title in
  `crates/game/src/race/load.rs`.
