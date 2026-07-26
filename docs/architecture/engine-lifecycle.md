# Engine lifecycle

> **Status: design intent, not discovered fact.** The original's lifecycle has
> not been read from the binary yet; that is an M2 task. This document describes
> how *our* engine is intended to be structured, and will be revised once the
> original's structure is known. Nothing here should be cited as a finding about
> Wipeout Pulse.

## Frame structure

Rendering and simulation are decoupled. The simulation runs at a fixed rate; the
renderer runs as fast as the display allows and interpolates.

```
loop {
    poll window and input events
    input_snapshot = input.sample()

    ticks = clock.advance(elapsed_nanos)      // 0, 1, or several
    for _ in 0..ticks {
        previous = world.clone()               // for interpolation
        world.tick(&input_snapshot)
    }

    alpha = clock.interpolation_alpha()
    renderer.draw(&previous, &world, alpha)
}
```

Three consequences worth stating explicitly:

- **`ticks` can be zero.** On a 144 Hz display with a 60 Hz simulation, most
  frames simulate nothing and only interpolate.
- **`ticks` can be several,** after a stall. `TickClock` caps the burst at eight
  so one long pause does not cascade into a freeze.
- **`world.tick` takes an input snapshot, not the input system.** That is what
  lets a replay feed recorded snapshots in, and what lets the verification
  harness feed scripted ones.

## States

```
        Boot
          |
      LoadAssets
          |
      FrontEnd  <-------------+
          |                   |
      LoadRace                |
          |                   |
        Race ---> RaceEnd ----+
          |
        Paused
```

The original's state machine is expected to be more granular. Recovering it is
an M2 deliverable and this diagram will be replaced by what is actually found.

## Startup

1. Parse the command line, including `--assets psp|ps2`.
2. Open the disc image or extracted asset directory.
3. Mount the asset registry, normalising PSP or PS2 assets into shared runtime
   types.
4. Create the window and the wgpu device.
5. Enter the state machine.

Asset normalisation is what makes `--assets` a presentation choice rather than a
gameplay one. See [ADR-0004](adr/0004-asset-pipeline.md).

## Shutdown

Deterministic and explicit: stop the simulation, flush any replay being
recorded, release GPU resources, close the asset registry. No reliance on
destructor ordering across crates.
