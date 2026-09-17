# Graphics settings should split by title *and* original platform, not title alone

2026-09-17. Reported by the user: "pulse ps2 and pulse psp currently share the
graphics settings, but should be split; the split currently is across title,
but it should be title + original platform."

`crates/game/src/settings.rs` keeps `[render_profiles.<title>]` one profile
per title, on the stated reasoning that the right defaults trade off against
how expensive that title's own scene is to render. A PS2-sourced Pulse race
and a PSP-sourced one are different scenes (different geometry payloads,
textures, the PS2's own engine flare and `WO_SHIP_ENGINEFLARE`), so the same
reasoning that split the titles applies one level down: the key should be
`(title, original platform)` - Pulse-PSP, Pulse-PS2, Pure-PSP, HD-PS3,
2048-Vita, Omega-PS4 - not `title`.

## Open

- The key's spelling in the TOML (`[render_profiles.pulse-ps2]`? a nested
  `[render_profiles.pulse.ps2]`?) and the migration from a file that has a
  bare `[render_profiles.pulse]`: a bare title key should seed both platform
  rows so nobody loses a tuned profile on upgrade - `settings::migrate` is
  the existing shape for this.
- Whether `oag_display`'s source-space vocabulary already carries the
  platform (it distinguishes the coordinate space a source authors in), so
  the key can be derived rather than declared.

## Next Steps

1. Change the profile key to title + platform with a migration and a test
   that an old file round-trips into both rows.
2. The menus read/write the row for the *open source's* platform; nothing in
   the UI needs to explain it, on the same argument the per-title split made.
