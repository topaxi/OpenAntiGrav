"""Where a capture's columns live in the original's memory.

Split out of `scripts/psp-trace.py` when a second tool needed the same tables:
`scripts/psp-autopilot.py` breaks on the same function, reads the same two
structures and writes the same CSV, and two copies of an offset table is exactly
the kind of drift that produces a capture whose columns silently mean something
else. `psp-trace.py` remains the authority on what a capture *is*; this is only
the address book.

(The hyphenated script names cannot be imported, which is why this module has an
underscore in it.)
"""

# Ship_UpdateCraft, docs/ghidra/functions/psp-pulse-usa/engine.md. **The craft is a0
# and the body is a1**, not a1/a2 as that page's signature says: at a breakpoint
# on entry, `craft+0x1cc` holds exactly the value in a1, dt at `craft+0x1c8`
# reads as a plausible frame time and `grounded` at `craft+0x2b0` as 1.0, while
# the same offsets off a1 are nonsense. a2 holds the function's own address,
# which is what the vtable dispatch at the call site leaves there.
SHIP_UPDATE_CRAFT = 0x08849618

# Offsets into the craft, from engine.md.
#
# `stun_timer` and `timer_2e0` are the two gates on `Ship_UpdateEngine`'s early
# return (`0x0884c634`, confidence 88): with flag `0x200` clear, either one above
# zero means **no thrust and a zeroed throttle state** for that frame.
# `craft+0x290` is the collision stun timer - `Ship_ApplyCollisionImpulse`
# (`0x0883f274`) arms it with `+= 0.5` on a hit and `Ship_ApplyLateralGrip`
# (`0x08848b78`) returns early while it runs, so a stunned craft also slides
# (confidence 85). `craft+0x2e0`'s arming condition has never been read, so it
# keeps its offset for a name rather than a guess dressed as one - engine.md's
# own suggestion is that it fits a capture taken near a race start.
#
# engine.md asks for exactly these two columns by name: without them a capture
# cannot say whether the gate fired at all, and the force-balance argument that
# the missing "12x of resistance" is really thrust the original never applied
# stays an inference. They are also the cheapest wall-contact indicator the
# already-documented fields offer.
#
# `shield` at +0x88 is the energy pool, from
# docs/ghidra/functions/psp-pulse-usa/shield.md - the field `Ship_Shield` reads
# and `Ship_SetShield` writes. It is here to give the recovered contact-damage
# law `|p| * 0.035` its runtime leg: on a capture that touches a wall, the drop
# in this column is predicted by the same impulse magnitude the trace's own
# velocity change implies, and no other column can test it. Remember the
# halving - `Ship_Damage` scales by 0.5 whenever the race has weapons off,
# which a time trial does.
CRAFT_FIELDS = [
    ("dt", 0x1C8),
    ("grounded", 0x2B0),
    ("throttle", 0x2B8), ("brake", 0x2BC), ("steer", 0x2C0),
    ("airbrake_l", 0x2C4), ("airbrake_r", 0x2C8),
    ("speed_cached", 0x2EC),
    ("stun_timer", 0x290), ("timer_2e0", 0x2E0),
    ("shield", 0x088),
]

# Offsets into the rigid body, measured at runtime by diffing successive frames.
# Rows 0/1/2 are orthonormal, and velocity times the frame time reproduces the
# position delta to within 0.007 units per tick over a 200-tick capture.
#
# **`speed` at +0x398 is not the velocity's own length**, which an earlier pass
# recorded here and a real capture disproved: over 200 ticks of Talon's Junction
# it runs a steady 3.67 % high (ratio 1.0367, sd 0.0026), about 0.86 units/s.
# Nor is it the forward projection (ratio 1.0402, sd 0.0028) - the two candidates
# are within each other's spread here, so this capture cannot tell them apart.
# What +0x398 holds is not established, and the capture's own speed range is only
# 3 %, so a constant offset and a constant factor cannot be separated either.
#
# `speed_cached` on the craft **is** the previous tick's `dot(velocity, forward)`
# - the forward-projected speed, not the velocity's magnitude. That is what
# engine.md says at confidence 95, and a shorter pass wrote the magnitude reading
# here in contradiction of it; this supersedes that. The same 200 ticks put the
# residual at a mean 3.3e-6 and a max 9.7e-6 - the `%.7g` below at a speed of 24,
# so indistinguishable from exact - against a mean 0.077 for the stale magnitude,
# and 1.3e-3 for `dot(velocity(t-1), forward(t))`, which says both vectors are the
# previous tick's rather than only one. The distinction matters as soon as the
# ship is not travelling straight ahead: sliding through a corner, the two
# readings differ by the cosine of the slip angle, and no capture had been taken
# there until the autopilot's lap.
#
# Both columns are recorded; neither is assumed.
#
# `avel_*` at +0x160 is the body-frame angular **momentum**, confirmed at
# instruction level in `Body_Integrate` (conf 88) - torque integrates into it
# with no inertia and no mass divide, while the basis is advanced by `+0x150`.
# See `docs/physics/angular-velocity-column.md` and rigid-body.md. Nothing is
# negated or rotated on the way out of memory; `oag_trace::trace::AngularReading`
# is where the readings are enumerated.
#
# `omega_*` at +0x150 is the other half of that pair and is recorded because
# `avel = -I * omega` is a *kinematic identity* that a capture can check without
# any force law entering: `Body_Integrate` advances the basis by `+0x150`, so
# recording it says directly whether the basis advance is `I^-1 * (+0x160)` or
# something else. `docs/physics/cornering-ground-truth.md` names this column as
# the one measurement that settles whether the momentum column is the *only*
# input to the attitude - its lap fit finds pitch and roll rotating more than
# `+0x160` accounts for, while the low-speed pitch captures find the identity
# holding to a fraction of a percent, and only this column can tell a second
# rotation source from a wrong tensor.
BODY_FIELDS = [
    ("right_x", 0x000), ("right_y", 0x004), ("right_z", 0x008),
    ("up_x", 0x010), ("up_y", 0x014), ("up_z", 0x018),
    ("fwd_x", 0x020), ("fwd_y", 0x024), ("fwd_z", 0x028),
    ("pos_x", 0x030), ("pos_y", 0x034), ("pos_z", 0x038),
    ("vel_x", 0x140), ("vel_y", 0x144), ("vel_z", 0x148),
    ("speed", 0x398),
    ("avel_x", 0x160), ("avel_y", 0x164), ("avel_z", 0x168),
    ("omega_x", 0x150), ("omega_y", 0x154), ("omega_z", 0x158),
]

# The player camera, captured only under `psp-trace.py --camera`. Method from
# docs/ghidra/functions/psp-pulse-usa/camera.md: break at 0x0883c13c inside
# Camera_UpdatePlayerView (0x0883c0cc), read s7, and the camera node is
# *(s7 + 0x3c) - a 4x4 with rows at +0x00/+0x10/+0x20. Its +0x30 holds the
# **negated** eye position; the capture negates it back at write time, so the
# CSV's `cam_pos_*` is a world position like `pos_*`. The offsets below are
# into the node, not the craft.
CAMERA_UPDATE_BREAK = 0x0883C13C
CAMERA_NODE_OFFSET = 0x3C

CAMERA_FIELDS = [
    ("cam_right_x", 0x000), ("cam_right_y", 0x004), ("cam_right_z", 0x008),
    ("cam_up_x", 0x010), ("cam_up_y", 0x014), ("cam_up_z", 0x018),
    ("cam_fwd_x", 0x020), ("cam_fwd_y", 0x024), ("cam_fwd_z", 0x028),
    ("cam_pos_x", 0x030), ("cam_pos_y", 0x034), ("cam_pos_z", 0x038),
]

# The craft's `Engine Flare` node, captured only under `psp-trace.py --flare`.
# Two hops, both from docs/ghidra/functions/psp-pulse-usa/pads.md: the object
# `Ship_ApplySpeedupPad` reaches through `craft+0x1c4` holds the flare at
# `+0x78`, and `ExhaustFlare_OnSpeedupPad` (0x08904f10) stores its `0.8`
# literal into `flare+0xb8`. The field offsets are exhaust.md's, off
# `Exhaust_Update` (0x089058b0).
#
# **The two hops are verified by a pointer identity, not assumed**: the flare's
# own owner field, `flare+0xc0`, reads back as exactly the pointer
# `craft+0x1c4` held, so the walk lands on the object the flare itself claims
# as its owner. Measured live 2026-08-08 on Talon's Junction, craft
# `0x09a04170` -> `0x09a031b0` -> flare `0x09a2af50`, `flare+0xc0` =
# `0x09a031b0`. Worth one note for a future reader: pads.md calls that middle
# object the **racer** and exhaust.md calls it the **craft**, and this identity
# says they are the same pointer under two names - it is *not* the craft that
# `Ship_UpdateCraft` takes in `a0`, which is what `SHIP_UPDATE_CRAFT` above
# breaks on and what `CRAFT_FIELDS` is indexed off.
#
# Corroboration that the offsets are right, from the same live read at rest:
# `colour` came back `0xd7ffffff` - white with alpha `0xd7` = 215, inside
# `Exhaust_Update`'s recovered `rand_int(200, 255)` - and `half_size` `1.0586`,
# inside `(0 * 0.6 + 0.4) * 2.5 = 1.0` times its recovered `rand(0.75, 1.25)`
# flicker. Two independently-derived formulas landing on two live values is
# what makes this an address book rather than a guess.
RACER_POINTER = 0x1C4
FLARE_POINTER = 0x78
FLARE_OWNER = 0xC0

# `colour` is deliberately absent: it is a packed ABGR8888 word, not a float,
# and the alpha is re-randomised every frame anyway, so a column of it would
# record the flicker and nothing else. `half_size` already carries the other
# half of the same randomisation.
FLARE_FIELDS = [
    ("boost_timer", 0x0B8),
    ("plume_timer", 0x088),
    ("intensity", 0x0BC),
    ("half_size", 0x0C4),
    ("engine_on", 0x094),
    ("flare_speed_kmh", 0x08C),
    ("speed_ramp", 0x090),
    ("boost_accum", 0x060),
]
