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

# **Not yet located, and this is the real gap.** Every offset in this file is
# relative to a live craft pointer, and nothing here says how to find one
# without already having it: PPSSPP's capture reads it straight off the `a0`
# register at a breakpoint on `Ship_UpdateCraft`'s entry, and PINE exposes no
# CPU registers at all - memory read/write and savestates only
# (`docs/reverse-engineering/pcsx2-debugger.md`). `scripts/pcsx2-trace.py`
# therefore takes the craft pointer as a required `--craft` argument rather
# than discovering it, and the address is only good for the savestate it was
# found under - PCSX2's savestate loading is bit-exact
# (`pcsx2-debugger.md`'s "Deterministic capture" section), so a fixed grid
# save's craft address is a fixed address, but a different save or a fresh
# boot is a different heap layout and a different one.
#
# `docs/reverse-engineering/pcsx2-debugger.md`'s own "what is worth doing
# next" item 2 names this exact gap: "the ship state block is the obvious
# first target and is not yet located in the PS2 corpus." Finding it once is
# a live session: pause on the grid, and either walk a known-shape memory
# scan (a position near the track's own start line, cross-referenced against
# `oag-trace track`'s spline output) or diff EE RAM the way the frame counter
# itself was found - a word that moves along `velocity` while the ship
# accelerates, the same technique that found `FRAME_COUNTER` in
# `pcsx2-drive.py`.
CRAFT_POINTER_NOT_LOCATED = True

# The body pointer, craft-relative. Confirmed live in
# `Ship_ApplyVerticalDamping`'s own disassembly: `lw a0,0x1ec(a0) ; a0 =
# craft+0x1ec, the body`. This is the PS2's equivalent of `psp-drive.py`'s
# `BODY_POINTER` (PSP: `craft+0x1cc`) - a different offset, the same role.
BODY_POINTER = 0x1EC

# Confirmed craft-relative offsets, all from craft-update.md, all cited where
# they appear on that page. **Deliberately partial**: `steer` and `brake`'s
# own ramped, persisted state - the values `Ship_UpdateSteering` and
# `Ship_UpdateBrakes` write back each frame - are never assigned a craft
# offset anywhere on that page, only described as local pseudocode variables,
# and a search of `names.tsv`/`handling-xml.md`/`README.md` for the same
# binary turns up nothing further. Guessing an offset for either is exactly
# what ADR-0005 (below 50 confidence, do not rename at all - the same rule
# applied to data, not just functions) exists to prevent, so neither is here.
# `scripts/pcsx2-trace.py` writes both as `0.0` with a loud warning rather
# than a plausible-looking number.
#
# `dt` has the same problem for a different reason: nothing on this page
# gives it a craft offset either, and unlike `steer`/`brake` there may be no
# offset to find - the PSP measures a real per-frame duration
# (ADR-0007), and whether the PS2 build does the same or integrates a fixed
# PAL step has not been checked. `scripts/pcsx2-trace.py` writes a fixed
# `1/50` (the PAL rate this title boots at, confirmed live) and says so on
# every capture; that is a stated assumption; not a measurement.
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
