---
status: re-first
titles: [omega, 2048]
updated: 2026-11-03
summary: "Omega/2048 shell, colour and Omega Lightning wired; open: Omega ambient and lit variants, Zone shell, 2048-content modes, Vita program"
size: M (65)
difficulty: medium (60)
---

# Omega and 2048 draw HD's shield palette; the real values and the shell build are read

Evidence: [`ps4-omega-eu/shield.md`](../../docs/ghidra/functions/ps4-omega-eu/shield.md) and
[`vita-2048-eu-v104/shield.md`](../../docs/ghidra/functions/vita-2048-eu-v104/shield.md) (static reads,
confidence 85 at most). HD's wire is merged; this thread carries the same wire to the two titles whose
`.rcsmodel` is the Vita/PS4 container.

## Open

1. **Build the shell.** `shield_model` in `crates/livery/src/shield.rs` takes the PS3 route
   (`mesh::rcs::build`) for any `.vex` with external geometry. It needs the branch the hull path in
   `crates/livery/src/lib.rs` has: `mesh::rcs::psp2::is_psp2(&geometry)` then
   `mesh::rcs::psp2::build(name, &geometry, None, &mut |path| archives.read_name(path).ok())`.
   `psp2::build` was run on `hdships/<team>/shipshield`, `ships/<team>2048/<n>/shipshield` and both
   `vr_shield_cockpit` files on Vita and Omega and builds all of them (counts in the 2048 page).
   Textures were not exercised (the probe passed a callback that returns nothing).
2. **Read the colour.** `shield_colour()` uses the PS3 `rcsmodel::Model::parse`, which rejects the
   container, so it returns `None`. Use `oag_rcs::rcsmodel::psp2::parse(..).materials[*].params`
   (hash `oag_fx::absorb_shell::SHIELD_COLOUR`, `Param::bits` as `f32`). **Flush half subnormals to
   `0.0` on Vita** (the game does; the reader widens them: `Feisar_c1`, `Param.bits` `0x384e0000`).
   Omega's value is f32; the reader's pool cut can drop the fourth component of a value that ends the
   pool (only the 2048 magnetic shield, alpha `0` on Vita).
3. **Drop the HD palette inheritance for the per-team target.** The values differ from HD on seven
   teams (feisar yellow, piranha yellow, qirex magenta, triakis, auricom, mirage, zone). Omega's and
   2048's own tables are in the two pages; the base teams agree across the two titles apart from half
   quantisation.
4. **The colour rate depends on the mode.** `0.15` for HD-content modes (id `< 0x17`), `0.05` for
   2048-content modes. HD's `Palette` has one rate; the Omega/2048 palette needs the mode-keyed pair.
   Swell start `0.7`, target `1.0`, rate `0.2`, flicker `sin` with `n*0.25+0.75`: the same as HD.
5. **Target alpha is `max(cockpit.a, shell.a)`** with a missing model counting `1.0`, so an HD-content
   shield always targets alpha `1` (the `Data/Weapons` cockpit authors `1`), and a 2048-content one
   targets `0`. HD's page records no such compare; if HD has it (likely), Zone's alpha-0 shell is
   moot there too - check with a one-line read of `FUN_00125690`'s caller before relying on it.
6. **The cockpit sphere's path depends on the mode**: `Data/Weapons/vr_shield_cockpit` for modes
   `< 0x17`, `Data/Weapons2048/vr_shield_cockpit` otherwise (the latter is the magnetic shield, a
   different model with two submeshes).

## The one value unread

How the `2048_Magnetic_Shield` program (2048-content modes, all 2048 craft, RGB `(0, 4.59, 4.59)`,
alpha `0`) uses `ShieldColour`. If it multiplies the output by `.a`, a 2048-content shell is invisible
and the wire must not do that. Recipes: Omega,
`cargo run -p oag-rcs --example gcn_dump -- data/extracted/ps4/omega-eu/uroot/data00.psarc data/art/published/ships/materials/2048_Magnetic_Shield.rcsmaterial`
(entry name case as listed); Vita, `oag_rcs::gxp` over the material's programs, looking for the
constant named `ShieldColour` and its readers. Does not block 1-4 for HD-content modes.

## Wired

Items 1, 2, 3 and 5 (HD-content target alpha) landed in lane `omega-2048-shield-wire`; the program read below shows `.a` is unused by the pixel program. Open now: 4 and 6 are unreachable (nothing launches a mode `>= 0x17`; `Palette::colour_rate` and `CONTENT_2048_COLOUR_RATE` are ready), the Vita program of `2048_Magnetic_Shield` is unread, and the HD target-alpha compare is still unrecorded for HD.

Landed in lane `omega-shield-lightning`: Omega's `2048_shipshield_old` Lightning layer (scrolled diffuse colour plus untinted `cf_plasma` term; `ps4-omega-eu/shield.md`, "The two shell programs"). Open from it: `constantAmbientColour` (implicit 1) and the lit variants' terms unread, Zone's `zonebattle_shield` program unread (no lightning there), and the Uv2 pair assignment is by picture (confidence 60).

## Next Steps

1. Wire items 1-2 and 6 behind the existing `Model::shield_shell` path; screenshot a Race Box Assegai
   on Omega in an HD-content mode (colour known: `(0.211, 0.657, 1.0)`).
2. Add the mode-keyed rate and the per-title tables as Title data with provenance.
3. Read the magnetic shield program, then draw 2048-content modes.
4. Unmeasured: which of `hdships/<team>_c1` or `ships/<team>2048/<n>` our `ship_dir` selects for each
   variant, as on HD.
