# ADR-0083: Race Remix is the official mod `oag.remix`

## Status

**Accepted** (2026-11-02, maintainer). **Amends
[ADR-0078](0078-a-mod-is-turned-off-by-id-live-and-the-built-in-never.md)'s
consequence** "the LAN fingerprint counts only the mods that are on": from now it
counts only the mods that are on **and change a race** (decision 4 below). The rest of
ADR-0078 stands, including decision 4 (`oag-extras` cannot be turned off), which does
not cover `oag.remix`. Depends on
[ADR-0082](0082-a-mod-owns-pages-settings-queries-and-one-launch-action.md) (what a mod
page may be). Relates to [ADR-0034](0034-a-race-may-open-two-titles-at-once.md) and
[ADR-0035](0035-a-craft-pick-may-fall-back-to-a-title-that-reships-the-same-roster.md),
whose plumbing stays in the engine, and to
[ADR-0066](0066-the-menu-is-the-disc-plus-mods.md) (our own rows are a mod). ADR-0066
and ADR-0078 are not edited.

## Context

RACE REMIX, a race whose track and craft come from two titles, is a project feature, not
a disc's. Its page, five saved picks, pickers and launch are in the engine
(`assets/ui/menu.toml`, `oag_game::settings::Remix`, `session/remix.rs`), and a 2048
REMIX tile and three page-id hooks are coded to it. Only its row is in the built-in
`oag-extras`. The maintainer wants it moved into a standalone mod in
`OpenAntiGrav/official-mods` and the engine left with the plumbing.

Two things stand in the way beyond ADR-0082's vocabulary:

- **The LAN fingerprint.** `Mods::content_fingerprint` (`oag_game::mods`) hashes
  `id@version` of every mod that is on, except those under `oag_mod::EXAMPLES`. A
  machine with `oag.remix` on and one with it off would fingerprint differently and
  `Compat::OtherContent` would refuse the join, for a mod that changes no race. Today's
  example mods are simulation or track mods, so this has not shown.
- **Delivery.** The default search path is `data/mods`, `assets/mods/examples`
  (relative to the working directory, so a checkout only), `mods` beside the program
  (the AppImage's directory first) and `<data dir>/oag/mods`. `oag-mod`'s search is
  `std::fs::read_dir`; `oag_mod::pack` is `cfg(not(wasm32))`. `oag-extras` is
  embedded, which a mod that can be turned off should not be.

## Decision

1. **Race Remix is the mod `oag.remix`**, folder `remix/` in
   `OpenAntiGrav/official-mods` with its own `oag-mod.toml`. It carries the REMIX row,
   the `remix` page, the `OAG_REMIX_*` and `OAG_MENU_REMIX*` strings in every language
   file, its settings under `[mod_settings."oag.remix"]` and its rows in the 2048 tile
   list. It uses ADR-0082's pages, queries and `launch_race`, and has no module.
2. **`OpenAntiGrav/official-mods` is a public repository.** It is a submodule at
   `assets/mods/official`, mirroring `assets/mods/examples`, and is added to the default
   search list beside `EXAMPLES` as a constant in `oag-mod`. CI may check it out with the
   other submodule and needs no token.
3. **Enabled by default; can be turned off.** No entry in `[mods] disabled` means on
   (ADR-0078 decision 1); the MODS page switches it live (ADR-0078 decision 3).
   Unlike `oag-extras` it is not on the player's way back to any page.
4. **The LAN content fingerprint counts only race-changing mods:** a mod with a
   simulation module, a track or a campaign. A mod with only pages, rows, strings,
   pilots or other presentation is left out. The filter lives in `oag-game`
   (`Mods::content_fingerprint`); `oag_net::discovery::content_fingerprint` takes a list
   of strings and knows no mods. A page-only mod on one machine no longer changes the
   hash, and a simulation mod still does.
5. **What the engine keeps.** The two-source plumbing (`oag_source::remix`,
   `oag_raceplay::load`'s two-source open), the custom-track scans and `set_mod_files`,
   the speed-class union as title data, the generic queries and `launch_race`. The
   engine names no mod and no page of this mod.
6. **The game runs without the mod.** With it absent or off there is no REMIX row, no
   2048 REMIX tile and no CUSTOM title on that page; nothing else changes. The 2048
   tile list comes from the racebox rows the loaded mods add, not a hard-coded
   `["racebox", "remix", "multiplayer"]`, so a missing mod is not a crash. Custom tracks
   still race from campaign events (ADR-0078 decision 5) and `--ballisticng-track`;
   Hillside and the other track mods still appear on the MODS page. The ordinary RACE
   page is unchanged.
7. **Delivery: a folder on disk, found by the search path, copied beside the
   executable in packaged builds.** It is not embedded. A build made without the
   submodule is a build without Remix, which is a supported game. Per target:
   - **Linux AppImage** (`packaging/appimage/`, built by `scripts/build-appimage.sh`
     from `release.yml`): the search path reads `mods` in the directory that holds the
     `.AppImage` file (`APPIMAGE` is the file's path), not inside the image. So the
     folder is either added to the download (a `mods/` next to the file, for example in
     an archive) or put inside the image and found by a new search entry. The
     implementing lane picks one against `mods_search_path`. The AppImage ships no mods
     today.
   - **AUR** (`openantigrav-bin`, `openantigrav-git`): installed to a directory the
     search path reaches, `/usr/share` is not on it today. Either the PKGBUILD installs
     to `mods` beside `/usr/bin/oag-game` (reached by the `current_exe` rule, which
     holds for a symlink-free install), or the search path gains a system data
     directory. Left to the packaging lane to pick, checked against the PKGBUILD;
     neither PKGBUILD mentions mods yet.
   - **Windows and macOS builds** (`release.yml`): the folder is copied beside the
     executable (macOS: beside the bundle's executable, which is what `current_exe`
     resolves; not checked against the bundle layout). Not read in detail; the macOS job cannot run under `act`
     and is checked by its recipe only.
   - **Android and iOS** (`oag-mobile`): the search path reads the file system and the
     app's data directory; whether a packaged app has a `mods` folder there is **not
     read**. Delivery is the folder unpacked into the app's data directory at install
     or first start, **not yet designed**; until it is, a phone build has no Remix and
     is a supported game by decision 6.
   - **Web** (`scripts/build-web.sh`): `std::fs` does not exist on `wasm32`, so
     `read_dir` finds nothing and the folder cannot be on a search path. Delivery would
     be the folder fetched or bundled as files and read through a container the web
     build has. **Not read in this ADR** (whether `oag_mod`'s container supports a
     memory-backed mod); until it is, the web build has no Remix.
   - **Steam Deck build**: the Linux AppImage's `steamdeck` flavour; same as the
     AppImage.
   The three unread targets are an open question, not a decision to omit: each
   gets a named hand-over entry with the check that would settle it.
8. **CI and release.** `ci.yml`'s first job already sets `submodules: true`; the new
   public submodule adds no secret. `release.yml` and `nightly.yml` do not set
   `submodules` today, so they build without the examples; shipping Remix in a release
   means setting it there in the lane that edits the recipe.
9. **The mod author skill is updated by the implementing lanes** that change the
   mod format, not by this ADR.

## Alternatives considered

- **Keep Remix in the engine.** Rejected by the maintainer; the plumbing stays, the
  page does not.
- **Embed the folder like `oag-extras`** (`include_bytes!`). Delivery on every target
  at once, web and phones included. Rejected: it makes the submodule a compile-time
  requirement, it makes a mod that can be turned off part of every binary, and it
  removes the on-disk search that a player uses to replace it.
- **A private repository, with a token in CI.** Raised by the scout's reading of the
  workflow; the maintainer made the repository public, so the token problem does not
  arise. Forks check it out as they check out the examples.
- **Not a submodule: download a pinned release asset at packaging time.** No checkout
  cost, but a second fetch path and a version pin to keep. The submodule mirrors the
  examples and needs nothing new.
- **A per-mod "affects races" flag in the manifest** for the fingerprint. A mod could
  lie in either direction. The manifest already says whether a module, a track or a
  campaign exists; the filter reads that.
- **Count only the mods with a simulation module.** A track or campaign mod changes the
  race a join would start; leaving it out would let two machines enter the same lobby
  with different circuits.

## Consequences

- **A first-run player has Remix only where delivery exists.** Linux packages and the
  desktop builds, once their recipes copy the folder; web, Android and iOS have none
  until their delivery is designed. That is a visible regression for those targets if
  Remix is on their menu today, and the lane that moves Remix must not ship before
  the desktop recipes do.
- **A fingerprint change is silent.** Two machines that differed only in a page-only
  mod now match and can join. Wanted, and the lobby shows no mod difference for it; a
  machine's MODS page does.
- **The fingerprint also stops counting a track or campaign mod that is off**, as
  ADR-0078 already said: only the filter changes.
- **Tests that drive the menus must pass with and without the submodule** (the lead's
  gate has it; a worktree without it must not fail). Tests that name `remix` move or
  skip when the folder is absent, as the disc-backed tests do.
- **`oag-extras` loses its REMIX row** when the page moves; a built-in row and the
  mod's row must not both appear.
- The saved picks (`[remix]`) are lost on the move; pre-release, no migration.
- A mod in a public repository is code the project's maintainers must review as
  they do `mod-examples`: it is declarative only, so there is no sandbox question.

**Revisit if:** the web or a phone build needs Remix before its delivery is designed
(then embedding for those targets alone is its own ADR), or a second official mod makes
a folder per mod in one repository awkward.
