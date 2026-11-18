# oag-raceplay

A race as the player sees and hears it: the scene, effects, HUD readout and
sound, driven by the simulation's tick.

Renderer-coupled by design, so it is classified with the drawing crates: it sits
above the render, HUD and sound crates and below `oag-game`, and no gameplay crate
may depend on it. The rules the tick obeys live in the simulation crates.
