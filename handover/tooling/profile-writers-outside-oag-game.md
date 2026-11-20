---
status: ready-to-wire
titles: [pulse, pure, hd, 2048, omega]
updated: 2026-11-20
summary: "ADR-0095 made every oag-game profile write atomic; two writers in crates that cannot reach profile::write still truncate-then-write"
---

# Profile writers outside oag-game still truncate then write

ADR-0095 (`docs/architecture/adr/0095-every-file-under-the-profile-folder-is-written-atomically.md`)
put every `oag-game` write of the profile folder behind `profile::write`, `write_bytes` and
`write_private`. Two writers live in crates below `oag-game` and keep `std::fs::write`.

## Open

- `crates/disc/src/ps3_crypt.rs` (~421): the PS3 disc-key cache, `<config dir>/oag/keys/<name>.dkey`.
  A short file reads as a wrong key and the player is asked for the key again; nothing is lost.
- `crates/mod/src/campaign.rs` (~710-712): the in-game campaign editor writes a mod's manifest and
  campaign file into the mods folder under `<data dir>/oag/mods`. A crash while overwriting an
  existing campaign loses the player's authored campaign. This is the one that matters.
- Photos (`capture::write_png`) and the caches (`custom_tracks`, `prefetch`, movie transcodes) were
  judged not to need it: a new file per shot, or regenerated.

## Next Steps

1. Move the atomic helper (`crates/game/src/profile/native.rs`) to a crate both `oag-game` and
   `oag-mod` reach (it needs only `std`), keep `profile::{write, write_bytes, write_private}` as
   thin callers, and point `campaign.rs` and `ps3_crypt.rs` at it. About an hour once the home is
   chosen; the choice is the open question (a new leaf crate needs its milestone, `oag-bytes` is
   about byte order).
