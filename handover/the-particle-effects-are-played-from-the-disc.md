# The particle effects are played from the disc, and most of them have no recovered trigger

2026-08-12. `oag_formats::pob` parses every emitter tree, `oag_render::psys::Library` loads any `Data\Psys\<name>.POB` by name, and `psys::Stage` plays any number at once (`attach`/`follow`/`detach` for one riding a moving owner, `play` for a burst). **The mechanism is generic and finished; what is per-effect is the trigger**, which is reverse-engineering and not code. Wired today, all four in `race::RACE_EFFECTS`: `WO_SHIP_COLL_SPARK_DAMAGE`, `WO_ROCKET_FLARE`, `WO_ROCKET_EXPLO_TRACK`, `WO_ROCKET_EXPLO`. **Asset exists, trigger not recovered, so deliberately unwired** (PSP disc, 35 systems): `WO_SHIP_COLL_SPARK_NODAMAGE` (named in `sparks.rs`, and `ShipCollisionFx_Trigger` picks it when the contact dealt no damage - this engine has no damage flag at the contact yet), `WO_SHIP_EXPLOSION`, `WO_SHIP_DEATH_SPARKS`, `WO_SHIP_FXNODE_EXPLO`, `WO_SHIP_SPARK_DAMAGE_LEACHBEAM`, `WO_CANNON_SPARKS`, `WO_MINE_EXPLO`, `WO_MISSILE_HEAD`/`_EXPLO`/`_BOUNCE`, `WO_PLASMA_HEAD`/`_FLASH`, `WO_SHURIKEN_HEAD`/`_TRAIL`/`_BOUNCE`/`_EXPIRE`, `WO_LEACHBEAM_CHARGING`/`_ENERGY`, `WO_REPULSER`/`_BLAST`, `WO_QUAKE`, `WO_WEAPON_ABSORB`, `WO_BOMB_SMOKERING`, `WO_BLUE_WELDER`, `WO_MODESTO_STEAM_A`, `WO_RAIN`/`_LENS`, `WO_SNOW`. Most of the weapon ones need the weapon itself built first; the four environmental ones (`RAIN`, `SNOW`, `MODESTO_STEAM_A`, `BLUE_WELDER`) need to know which track places them and where, which nothing has read. **Do not fire any of them on a guess** - see the do-not-invent rule in `CLAUDE.md`. **What is still not implemented in the interpreter**, each authored in files that already parse: the sprite atlases and textures (a procedural falloff stands in), billboard roll, the emitter extent (particles spawn at the anchor), the emission-scale channel, and the animated-attribute array. Two instance-level scales read out of the executable on 2026-08-12 and also unmodelled: alpha is `particle_alpha * instance[+0x40]` and drawn size is `particle_size * instance[+0x34]` (`ParticleSystem_UpdateParticles`, `0x088f635c`); severity is the second of those and the first is not fed by anything here.

## Open

- Most of the 35 disc particle systems have a parsed asset but no recovered trigger, so they stay deliberately unwired.
- The interpreter does not yet implement sprite atlases/textures, billboard roll, emitter extent, the emission-scale channel, or the animated-attribute array.
- The two instance-level scales (`instance[+0x40]` alpha, `instance[+0x34]` size) are read but unmodelled; the alpha factor is not fed by anything here.
- **The environmental four are narrower than they looked, and the class is now
  confirmed live rather than dead code - only the effect *selection* is still
  open.** `weatherPos` `0x3da`'s registration is found
  (`WeatherPos_RegisterClass`, `0x0892c684`), and so is a genuine runtime
  constructor for it (`FUN_0892c404`, allocates and tags a real instance).
  Every static search for its caller came back empty, by four independent
  methods including a raw byte-pattern scan for its address as data - and
  that result was reported as "genuinely unreached" until a **live PPSSPP
  capture found the caller in minutes**: a breakpoint on `0x0892c404` fires
  while loading a real Time Trial on Talon's Junction, called through a
  generic per-class spawner (`FUN_08908f98`) resolving `weatherPos`'s own
  `+0x7c` `init` slot at runtime - invisible to every static search because
  the call is indirect, through a value only ever loaded into a register, not
  written anywhere as an immediate. So the class *is* exercised; what still
  isn't recovered is which of `WO_RAIN`/`WO_SNOW`/`WO_MODESTO_STEAM_A`/
  `WO_BLUE_WELDER` an instance carries, or whether Talon's Junction's 14 all
  carry the same one - no node payload, Maya name, or `PI_Track` attribute has
  a signal, and the live capture's own `param_3` (`2048.0` as `f32`, stored at
  the new instance's `+0x4c`) is the strongest untraced lead. Detail,
  including the class-identity-tag mechanism and the full live-capture method,
  in
  [`docs/ghidra/functions/psp-pulse-usa/weatherpos.md`](../docs/ghidra/functions/psp-pulse-usa/weatherpos.md).
  Separately, `WO_MODESTO_STEAM_A` may not even be a Pulse trigger at all -
  `modesto_heights` matches a **Pure** circuit, not a Pulse one.
- **`pob.md`'s "dead end" reading of `ParticleSystem`'s registered slot
  (`FUN_08a6bd18`) may be the same mistake `weatherpos.md` made and then
  corrected.** Not re-checked this session - `weatherpos.md` found that the
  identical-shaped value for `weatherPos` is a live class-identity tag, read
  by `Vex_LoadModel`'s per-class node gather, not inert. Worth the same check
  before trusting `pob.md`'s framing on `ParticleSystem`.

## Next Steps

- Recover triggers for the unwired effects - weapon ones need the weapon itself built first. For the four environmental ones, the constructor's call chain is confirmed live; what's left is reading which effect a `weatherPos` instance selects - `param_3`/`+0x4c` and `FUN_08908f98`'s per-record class-resolution cursor are the concrete leads, per `psp-pulse-usa/weatherpos.md`'s Open section.
- Re-check `pob.md`'s `ParticleSystem` "dead end" claim against the class-identity-tag mechanism `weatherpos.md` found, before relying on it.
- Implement the missing interpreter features (atlases/textures, billboard roll, emitter extent, emission-scale channel, animated-attribute array).
- Do not fire any effect on a guess - follow the do-not-invent rule in `CLAUDE.md`.
