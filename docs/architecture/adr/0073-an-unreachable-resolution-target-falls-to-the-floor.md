# ADR-0073: An unreachable resolution target falls to the floor

## Status

**Proposed (2026-10-19)**, for the maintainer to accept or change. Supersedes the
"holds where it is" half of ADR-0042's "The unreachable target" section; the rest of
ADR-0042, ADR-0044 and ADR-0045 stands.

## Context

The maintainer reported, on the Steam Deck: "the game always renders full resolution,
no matter the value, on a PS2 Pulse game; Pulse PSP goes down in render resolution as
expected." The cause is not the platform. The Deck was driving a 3840x2160 output and
the PS2 row is the only Pulse row with `reconstruction = "fsr3"` and `target_fps =
"90"`. FSR 3.1's presented-resolution half is a fixed cost: 11.99 ms at 4K against an
11.11 ms period. `Controller::record` found no budget left, called the target
unreachable and returned without a step on every frame, so the scale stayed at the
ceiling and the minimum resolution never mattered. ADR-0042 chose that on the premise
that "every pixel given up would buy nothing".

Measured on the Deck (`docs/rendering/dynamic-resolution.md`, "The fixed cost over the
period"): the same race at the ceiling ran 15 fps, and 41 fps at the floor. The pixels
buy frames: the target is out of reach, the frame time is not.

## Decision

When the fixed cost alone fills the period, the controller treats the frame as
infinitely late and falls as it does for any overrun, `FALL_STEPS` per cooldown, to the
profile's floor and no further. `Controller::unreachable` still reports it, so the one
`out of reach` warning still prints. Nothing is changed when a budget remains.

## Consequences

- A profile whose fixed cost exceeds the period sits at its minimum, which is what the
  minimum row says. The way back up is to lower the target or the fixed cost
  (reconstruction, MSAA), as the warning already says.
- A transient fixed-cost spike drives the scale down by a step per cooldown and it
  climbs back only through the normal earned rise once a budget returns. Loads are
  already kept out by `frame_seconds = None`.
- The `Target::at_most` guard against a target above the limiter is untouched.
- The reason is said in the log once per spell, naming the reconstruction and the output
  size ("fixed cost ... (fsr3 resolving a 3840x2160 output) ... held at its minimum").
  A menu note is not built: the menu's `warn_when` conditions compare settings values
  and nothing in it reads a measurement.
