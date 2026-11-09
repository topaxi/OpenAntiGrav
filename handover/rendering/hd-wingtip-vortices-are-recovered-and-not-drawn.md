---
status: ready-to-wire
titles: [hd, 2048, omega]
updated: 2026-11-09
summary: "HD wingtip vortices read and measured live (pad trigger, drift, opacity, colour, blend, texture); ours draws none"
---

# HD's wingtip vortices: recovered and measured, not drawn

Evidence: [`wingtip-vortices.md`](../../docs/ghidra/functions/ps3-hdfury-eu/wingtip-vortices.md) (the update, the
`VortexLine` push, update, opacity, config, colour, blend; live reads) and
[`hd-wingtip-contrails.md`](../../docs/rendering/hd-wingtip-contrails.md) (frames and the live trigger runs).

## Open

- **Ours draws none of it**: no `wingtip` locator is read (`oag_vex::vex::class_names` names it, nothing consumes it).
- **Turbo half is code-only (80)**: no craft held a Turbo in the live runs; flag `0x40` is unnamed.
- **The Zone picture** is not isolated: the Zone ship's nodes run the effect on pads (live, 75), but no frame separates
  the streaks from wall-scrape sparks.
- **2048 and Omega**: the ribbon's config ships in both (2048 byte for byte; Omega colour and floats, other layout);
  their update and push are not read (2048: importer string; Omega: `FUN_016fb470`).

## Next Steps

1. **Wire lane (HD), the exact wire**, everything below measured unless marked:
   - Per craft, the two `wingtip_left`/`wingtip_right` locators of the hull's `Locators.vex` (class `0x3ec`); left is
     side `1` (`+phase`), right side `0` (`-phase`).
   - Per locator a 1.3 s timer: refilled to `1.3` every tick the craft's speed-pad force time is above zero (ours: the
     pad force window; or the Turbo flag, code-only), not while the craft state is 4, 5 or 6; counts down by `dt`.
   - While the timer is above zero, every tick: strength `= ramp(forward speed) * timer / 1.3`, ramp `0` below 50,
     `(v - 50) * 0.006` to 100, `0.3` from 100 (units/s); push 3 nodes at the locator's world position, each with life
     `0.7` s, width `0.05` growing linearly to `1.5` at death, and a constant drift velocity
     `-2.5 * normalize(cos(th) * row1 + sin(th) * (row2 x row1))`, `th = s * phase + i * pi/3`, `s = +1` left, `-1`
     right, rows the locator's world basis as stored (they carry the craft's 0.75 scale; `row2` the forward axis);
     then `phase += 0.18` (`0.06` per chain). Phase starts at `0` when the ribbon is taken; the ribbon is dropped when
     the timer runs out and its nodes live out their 0.7 s.
   - Draw: `oag_fx::fat_line`'s camera-facing two-sided strip, one strip per chain, texture
     `/data/tex/smoke_trails_frame1_alpha.gtf` (`DATA02.PSARC`, not the `ribboneffects/textures/` copy),
     `u = arc length * 0.025`, colour `(1, 1, 1)` times the texture, alpha `= tex.a * t * strength` with
     `t = life / 0.7` and the strength the **current** one (whole ribbon), the two end nodes and the two second-from-end
     nodes of each chain at alpha 0, blend `SRC_ALPHA, ONE_MINUS_SRC_ALPHA`.
   - A pool of 18 ribbons (two per craft).
   - Test: a fixture that drops the 2.5 drift, the side mirror or the `t * strength` opacity fails.
2. RE lane (later): the Turbo half live (give a craft the Turbo, poll `+0xe4`), and 2048's and Omega's update.
