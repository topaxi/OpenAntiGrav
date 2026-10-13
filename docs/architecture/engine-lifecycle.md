# Engine lifecycle

> **Status: design intent, not discovered fact.** The original's lifecycle has
> not been read from the binary yet; that is an M2 task. This document describes
> how *our* engine is intended to be structured, and will be revised once the
> original's structure is known. Nothing here should be cited as a finding about
> Wipeout Pulse.

## Frame structure

Rendering and simulation are decoupled. The simulation runs at a fixed rate; the
renderer runs as fast as the display allows and draws between the last two
ticks.

```
loop {
    poll window and input events               // keyboard, mouse, resize

    image = surface.acquire()                  // may block: vsync, a full queue
    elapsed = now() - last                     // read after the wait

    ticks = clock.advance(elapsed_nanos)      // 0, 1, or several
    for _ in 0..ticks {
        input_snapshot = input.sample()        // polls the pads here
        race.tick(&input_snapshot)             // first keeps this tick's draw poses
    }

    alpha = if paused { 1.0 } else { clock.interpolation_alpha() }
    race.set_draw_alpha(alpha)                 // the draw reads poses blended by it
    renderer.draw(&race, image)                // nothing drawn if no image
    present(image)
    race.set_draw_alpha(1.0)
}
```

**The swapchain image is acquired before the clock is read** (since
2026-10-13; `Gpu::acquire`, called from `Session::frame`). Under a blocking
present mode (`vsync = on`, or `smooth` falling back to Fifo) the acquire is
where the loop waits for a free image; taken first, that wait counts toward the
frame's `elapsed`, so the ticks, the pad input they sample and the draw alpha
all describe the moment after it, where taken at the top of the draw they were
a wait older. Window events (the keyboard) that arrive during the wait are
still delivered after the frame, as before. The `PRESENT` row carries the
acquire's wait and the `CPU` row does not, as before.

`World` is never cloned for this: each tick path copies only the poses a draw
reads (the camera, every craft, every projectile) into `RaceView`, outside the
state hash, and the race's camera and model-matrix accessors blend them with
the current tick's. Captures and traces advance one tick per drawn frame and
draw at `1.0`, the tick exactly. See [drawing between
ticks](../rendering/between-ticks.md) for what is blended, what stays stepped
and the snaps on a cut or a respawn.

Three consequences worth stating explicitly:

- **`ticks` can be zero.** On a 144 Hz display with a 60 Hz simulation, most
  frames simulate nothing and only draw a new point between the same two
  ticks.
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
