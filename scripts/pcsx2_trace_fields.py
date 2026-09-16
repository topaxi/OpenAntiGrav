"""Where a PS2 capture's columns live in `SCES_547.48`'s memory.

The PS2 sibling of `scripts/psp_trace_fields.py` - same purpose, same reason it
is its own module (a hyphenated script cannot import another hyphenated
script, and two copies of an offset table is exactly the kind of drift a
capture's columns cannot afford). `scripts/pcsx2-trace.py` is the only reader;
this is only the address book.

**Every offset below is cited against
`docs/ghidra/functions/ps2-pulse-eu/craft-update.md`, and nothing here is
carried over from the PSP build.** That page is explicit that the craft
struct is laid out differently between the two binaries even where the
*physics terms* are identical - `craft+0x320` (PS2) and `craft+0x2ec` (PSP)
are the same quantity at different offsets, and `Ship_ApplyQuadraticDrag`'s
own `craft+0x320` doubles as the PSP's *local force accumulator* at that
offset, so carrying a PSP craft offset across would silently read the wrong
field. The rigid **body** struct is the opposite case - see `BODY_FIELDS`
below - and is reused verbatim because the page confirms it position for
position.
"""

# Ship_UpdateCraft, docs/ghidra/functions/ps2-pulse-eu/craft-update.md. Recorded
# for citation and for a future breakpoint-capable transport; PINE has no
# register-read at all, so nothing in this module's own capture path breaks
# here the way psp_trace_fields.py's SHIP_UPDATE_CRAFT does; see
# `CRAFT_POINTER_NOT_LOCATED` below for what that actually costs.
SHIP_UPDATE_CRAFT = 0x001596F8

# **No automatic way to find one - PINE exposes no CPU registers at all**
# (`docs/reverse-engineering/pcsx2-debugger.md`), so there is nothing
# equivalent to PPSSPP's breakpoint-and-read-`a0`. `scripts/pcsx2-trace.py`
# takes the craft pointer as a required `--craft` argument, and the address
# is only good for the savestate it was found under - PCSX2's savestate
# loading is bit-exact, so a fixed grid save's craft address is a fixed
# address, but a different save or a fresh boot may give a different one.
#
# **A live session on 2026-09-16 found one and the method is now proven
# fast**, not just sketched: from a savestate on the grid, anchor twice with
# `pcsx2-drive.py frames N --from-state <slot>` - once holding `cross` for
# `N` frames, once holding nothing - and diff the two resulting memory images
# word for word. Every AI-controlled and every static/authored value is
# bit-identical between the two runs (PCSX2's frame-advance is verified
# deterministic, so nothing but the player's own input differs), which cut a
# naive "diff two snapshots of one run" search - thousands of plausible
# vector-shaped false positives, mostly other craft and track/camera data -
# down to **1,743 differing words out of 2,097,152** on this session's grid
# save. Filtering those for a lone scalar reading ~0 at rest, a plausible
# speed after 30 frames of thrust, and ~0 again after 30 frames of nothing
# held found exactly `craft+0x320` (`speed_cached`, already known) plus one
# more candidate 1000 bytes earlier - `craft = candidate - 0x320`. Confirmed,
# not just plausible, by three independent checks at that address: the
# cached basis at `craft+0x180/0x190/0x1a0` came back orthonormal and
# matched `body+0x00/0x10/0x20` read through `craft+0x1ec` bit for bit; the
# body's own position read `(x, 4.004, z)` - within 0.06% of the PSP's own
# independently-established hover height of 3.978-4.002
# (`oag-trace.md`'s "the suspension was never the problem" section); and the
# controls block through `craft+0x98` read every axis and the button mask as
# exactly zero, matching a craft at rest with nothing held.
#
# **On this session's grid savestate, `craft = 0x00720f20`.** This is
# instance data, not a structural fact - a different savestate, a different
# boot, or a different disc region may give a different address, and it does
# not belong in `craft-update.md` or `names.tsv` for that reason. It is
# recorded here only as a worked example of the method, and because a
# savestate slot loaded on the same machine without a fresh boot in between
# is likely, though not guaranteed, to reproduce it.
CRAFT_POINTER_NOT_LOCATED = True

# The body pointer, craft-relative. Confirmed live in
# `Ship_ApplyVerticalDamping`'s own disassembly: `lw a0,0x1ec(a0) ; a0 =
# craft+0x1ec, the body`. This is the PS2's equivalent of `psp-drive.py`'s
# `BODY_POINTER` (PSP: `craft+0x1cc`) - a different offset, the same role.
BODY_POINTER = 0x1EC

# Confirmed craft-relative offsets. Most are cited against craft-update.md
# directly; `steer` and `brake` are not - that page only ever shows them as
# local pseudocode variables in `Ship_UpdateSteering`/`Ship_UpdateBrakes`,
# never assigned a craft offset, and a search of
# `names.tsv`/`handling-xml.md`/`README.md` turned up nothing further. Both
# are now confirmed anyway, live, on 2026-09-16, by the same session that
# found `craft` itself (see `CRAFT_POINTER_NOT_LOCATED` above) - not from
# reading more decompiled code, but from diffing the craft block itself
# across a held input and back out again:
#
# - **`steer` (`craft+0x2f0`)**: reads exactly `0.0` at rest, ramps to
#   `-64.425` after 20 verified frames holding `left`, ramps to **exactly**
#   `+64.425` after 20 frames holding `right` from the same anchor (the sign
#   flip is exact, not approximate - the same magnitude both signs), and
#   decays back to exactly `0.0` within 60 frames of releasing. All three
#   match `Ship_UpdateSteering`'s pseudocode verbatim: ramps from zero,
#   symmetric about centre, "clamped to exactly 0" on release. Confidence
#   90 - four independent live measurements, all consistent, no ambiguity
#   about which of a handful of candidates it could be (it was the only word
#   in the whole craft block that moved this way under `left`/`right`).
# - **`brake` (`craft+0x2ec`)**: reads `0.0` at rest, ramps smoothly to
#   `35.4` after 20 frames holding **both** `l1` and `r1` together (not a
#   snap to a fixed value the way `airbrake_l`/`airbrake_r` at
#   `craft+0x2d8`/`craft+0x2dc` do, which is what told it apart from six
#   other candidates that also moved under the same input), reaches exactly
#   `100.0` by 60 frames (the documented clamp), decays to exactly `0.0`
#   within 60 frames of release, and - the decisive check -  **stays at
#   `0.0` for 20 frames holding `l1` alone**, matching
#   `Ship_UpdateBrakes`'s `controls[0x08] > 0 && controls[0x0c] > 0` gate
#   exactly: one airbrake does nothing, both together ramp it. Confidence
#   90, same basis as `steer`.
#
# `dt` is different again: nothing on craft-update.md gives it a craft
# offset either, and there is no register read on this transport to fall
# back on (PINE exposes memory and savestates only). The same 2026-09-16
# session found indirect evidence for the fixed-PAL-step assumption
# `scripts/pcsx2-trace.py` already made: comparing all of EE RAM across one
# single verified frame-advance, 58 words read exactly `0.02` (`1/50`) and
# **none** of them changed - and no word anywhere in the 0.01-0.03 range
# changed at all between the two frames. That is consistent with a fixed
# step and is not proof of one: this transport only ever advances in
# whole verified frames regardless of real elapsed wall-clock time between
# taps, so a *measured* dt reading a hardware timer tied to the emulated
# video clock could look identically stable under this exact test even if
# the original genuinely measures. No craft offset was identified for
# whichever of the 58 words (if any) is the real one, so this remains an
# assumption - now an evidence-backed one, confidence 55, not a confirmed
# reading.
CRAFT_FIELDS = [
    # `up` axis, from `Ship_ApplyVerticalDamping`'s `lqc2 vf2,0x180(a0)` and
    # `Ship_HoverFourCorner`'s hover-spring call site, both reading the same
    # offset the same way. Not part of REQUIRED_COLUMNS (the body's own
    # `up_x/y/z` covers that), kept for cross-checking the two against each
    # other on a capture.
    ("craft_up_x", 0x180), ("craft_up_y", 0x184), ("craft_up_z", 0x188),
    # row 0 (right/left), from `Ship_ApplyLateralGrip`'s
    # `dot(velocity, craft+0x190)` - "craft+0x190 is therefore row 0, the axis
    # engine.md's runtime measurement identifies as pointing left."
    ("craft_row0_x", 0x190), ("craft_row0_y", 0x194), ("craft_row0_z", 0x198),
    # forward, from `Ship_ApplyWeathervaneTorque`'s
    # `cross(craft+0x1a0, craft+0x1b0)`, matched term for term against
    # engine.md's `cross(forward, velocity)`.
    ("craft_fwd_x", 0x1A0), ("craft_fwd_y", 0x1A4), ("craft_fwd_z", 0x1A8),
    # velocity, same call site - "craft+0x1b0, the linear velocity". A cached
    # craft-side copy, not the body's own `vel_*` (which BODY_FIELDS already
    # covers at `body+0x140`); kept for the same cross-check `craft_up_*`
    # gives.
    ("craft_vel_x", 0x1B0), ("craft_vel_y", 0x1B4), ("craft_vel_z", 0x1B8),
    # Grounded contact flag, bit 0. "Flag bit 0 of craft+0x1e0 is ORed in per
    # contact, matching the PSP's craft+0x1c0 bit 0."
    ("grounded_flags", 0x1E0),
    # magLockBlend, from the gravity section: "grounded is craft+0x2e0 and
    # magLockBlend is craft+0x2b0, both already fixed by other terms on this
    # page."
    ("mag_lock_blend", 0x2B0),
    # Craft mode, read by `Ship_ApplyAngularDamping` (`craft->mode`, gating
    # `-5.0`/`-2.0` roll damping) and by `Ship_UpdateSteering` (mode 5/6
    # lockout).
    ("mode", 0x2D4),
    # The two airbrake state values, from `Ship_ApplyLateralGrip`'s
    # `max(craft+0x2d8, craft+0x2dc)`.
    ("airbrake_l", 0x2D8), ("airbrake_r", 0x2DC),
    # `brake`, confirmed live 2026-09-16 - see the comment above this table.
    ("brake", 0x2EC),
    # The grounded **fraction** (0..1, four-corner hover accumulates +0.25 per
    # contacting probe), read directly in `Ship_ApplyVerticalDamping` and
    # `Ship_ApplyGravity`. This is `REQUIRED_COLUMNS`' `grounded` - the PS2
    # equivalent of the PSP's `craft+0x2b0`, at a different offset because the
    # PSP's own `craft+0x2b0` is `magLockBlend` here.
    ("grounded", 0x2E0),
    # The cached forward speed - "craft+0x320 is the cached forward speed in
    # the PS2 layout (the PSP's craft+0x2ec)", from the quadratic-drag
    # section. `REQUIRED_COLUMNS`' `speed_cached`.
    ("speed_cached", 0x320),
    # `steer`, confirmed live 2026-09-16 - see the comment above this table.
    ("steer", 0x2F0),
]

# The controls block, craft-relative pointer at `craft+0x98` - "every control
# term reaches its input through `*(craft+0x98)`", with member offsets
# matching the PSP's `craft+0x78` block (`+0x00` steering, `+0x04` thrust,
# `+0x10` pitch, `+0x44` buttons; `+0x08`/`+0x0c` are the two airbrake axes,
# from `Ship_UpdateBrakes`'s `controls[0x08] > 0 && controls[0x0c] > 0`).
# **These are the raw target inputs the ramps chase, not the ramped output
# state** - `throttle` is the one exception, because `Ship_UpdateEngine`'s own
# ramp is dead (see the module docstring's citation) and overwrites its
# `craft+0x2e8` accumulator with `controls->thrust` verbatim before using it,
# so the raw input *is* the value the force law reads. `steer` and `brake` get
# no such shortcut: both ramps are live, so their controls-block reading is
# the *target*, not the current ramped value, and is not written to the
# `steer`/`brake` capture columns for that reason - see the module docstring.
CONTROLS_POINTER = 0x98
CONTROLS_FIELDS = [
    ("controls_steer_x", 0x00),
    ("throttle", 0x04),
    ("controls_pitch", 0x10),
    ("controls_airbrake_l", 0x08),
    ("controls_airbrake_r", 0x0C),
    ("controls_buttons", 0x44),
]

# The rigid body, reached through `BODY_POINTER`. **Reused from the PSP's own
# `BODY_FIELDS` on purpose, not by default**: craft-update.md's "Cross-platform"
# table states the four accumulators at `+0x100`/`+0x110`/`+0x120`/`+0x130`
# are at identical offsets on both builds, and three more pairs are each
# confirmed independently for the PS2 specifically, matching the PSP's own
# offsets exactly:
#
# - `body+0x30` is position, from `Body_SetPosition`'s own write.
# - `body+0x140` and `body+0x150` are velocity and angular velocity together,
#   from `Body_ResolveContactPair`'s "relative velocity at the contact, from
#   body+0x140 and body+0x150" - the standard `v_point = v_linear +
#   cross(omega, r)` construction, which only makes sense if `+0x140` is
#   linear velocity and `+0x150` is angular velocity.
# - `body+0x160` is `angularVelocityLocal`, from `Ship_ApplyAngularDamping`'s
#   own `w = body->angularVelocityLocal; /* body+0x160 */`.
#
# The basis rows at `body+0x00`/`+0x10`/`+0x20` are confirmed to exist and to
# be orthonormal (`Body_Integrate`'s orthonormaliser, `row0 = row1 x row2`),
# but **which row is which axis is not confirmed for the body specifically**
# the way it is for the PSP's own body (engine.md's runtime `cross(row0,
# row1) = row2` measurement, on the PSP capture, informs `--basis` in
# `oag-trace run`) - only that `craft_row0_*`/`craft_up_*`/`craft_fwd_*`
# above give a cross-check once a capture exists. Recorded as `right_x` etc.
# to match `REQUIRED_COLUMNS`' own naming, not as a confirmed identity.
#
# `speed` (`REQUIRED_COLUMNS`) has no known PS2 memory equivalent at all - the
# PSP's own `body+0x398` is not confirmed to exist on this build, let alone at
# the same offset, and nothing on craft-update.md reads a body-relative speed
# scalar. `scripts/pcsx2-trace.py` writes `|velocity|` instead, computed from
# `vel_x/y/z` at write time - a derived column, not a memory read, and
# labelled as one in the script's own output.
BODY_FIELDS = [
    ("right_x", 0x000), ("right_y", 0x004), ("right_z", 0x008),
    ("up_x", 0x010), ("up_y", 0x014), ("up_z", 0x018),
    ("fwd_x", 0x020), ("fwd_y", 0x024), ("fwd_z", 0x028),
    ("pos_x", 0x030), ("pos_y", 0x034), ("pos_z", 0x038),
    ("vel_x", 0x140), ("vel_y", 0x144), ("vel_z", 0x148),
    ("avel_x", 0x160), ("avel_y", 0x164), ("avel_z", 0x168),
    ("omega_x", 0x150), ("omega_y", 0x154), ("omega_z", 0x158),
]
