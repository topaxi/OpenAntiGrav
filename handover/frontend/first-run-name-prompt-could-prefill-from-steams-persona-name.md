---
status: ready-to-wire
titles: [all]
updated: 2026-11-06
summary: "One shared player name, a tag prompted on Pulse and Pure only: decided, not built; Steam prefill follows; no stored name exists yet"
size: S (40)
difficulty: easy (40)
---

# The first-run name prompt could prefill from Steam's persona name

2026-10-27. Idea only, nothing built. The originals ask for a name on first run (and a handle on some
titles). A player who launches this port as a non-Steam game from Steam already has a name Steam knows.

## Decision so far

- **Name and tag entry works with a controller, a phone keyboard, a physical keyboard and the Steam Deck's virtual keyboard; the phone and Deck keyboards open by themselves when that is easily detectable** (maintainer, 2026-11-06). Detection and the opening call are unmeasured on every platform: if one is not easy, the entry still works with a controller.
- **One shared player name across all titles; the 3-character tag is prompted only on titles that use it, Pulse and Pure** (maintainer, 2026-11-06). Neither is stored today: `settings.rs` and `schemas/v1/settings.tosd` hold no player name, so the stored name comes before any prefill. The tag fills Pulse's footer strip, which draws nothing today (`docs/ui/menus-original.md`).
- **Read `PersonaName` from `loginusers.vdf`, not the Steamworks API.** A non-Steam shortcut has no
  real AppID, so `SteamAPI_Init` does not work as itself. Spacewar (480) is tolerated for testing but
  not sanctioned for a shipped build (friends see "playing Spacewar"). A real AppID needs Steam Direct,
  store review and a $100 fee, and a reimplementation that needs the player's own disc image is a poor
  fit. Unverified against Valve's current terms.
- **Prefill only.** Once, when no profile exists. The player edits it on the usual keyboard before
  confirming. Never overwrite a saved name.
- **`PersonaName` only.** `AccountName` is the Steam login; never read it into the profile and never
  put it on the wire (netplay lobby: name + token).
- **Fall back to the OS user name.** No Steam (missing file, parse error, no `MostRecent` entry, or a
  name the HUD font cannot draw once sanitised) suggests the system user name instead (login name or
  full name, which to use is open). Same rules: prefill only, player edits, same sanitising. If that
  is empty or unusable too, the normal empty prompt. Take the Steam entry with `MostRecent "1"`.
- **Outside the simulation.** Profile and UI input only, so no `World` or hash impact.

## Open

- **Paths unchecked.** Linux `~/.local/share/Steam/config/loginusers.vdf`, `~/.steam/steam`, Flatpak
  `~/.var/app/com.valvesoftware.Steam/.local/share/Steam`; Windows from the registry's Steam path.
  macOS and the Deck not looked at. Android, iOS and web have no Steam: the prefill is a no-op for Steam.
- **OS name source unchosen.** Login name (`USER`/`USERNAME`, `whoami`) is easy but often `root`,
  `deck` or lowercase; the full name (GECOS, Windows display name) is nicer but needs platform code
  or a crate. A generic login (`deck`, `root`, `user`, `administrator`) is probably worth skipping.
  Android, iOS and web have no useful OS user name: empty prompt there.
- **Not seen.** The file's exact format on the maintainer's install has not been read yet.
- **Where the hook goes** is unlocated: the first-run name entry has not been found in
  `oag-ui` / `oag-ui-screens`. Per-title name length and charset limits (and which titles ask for a
  handle) come from the original's own entry, which this thread has not measured. Which titles
  keep a name and a 3-character tag is in `docs/formats/fexml.md`, "Which titles keep a name and a
  tag": Pulse and Pure both, HD the name only, 2048 and Omega neither.
- **Precedent** (Playnite's Steam plugin, SteamTinkerLaunch, Steam ROM Manager read it, from memory)
  is unverified. The file is undocumented by Valve and may change.

## Next Steps

1. Find the first-run name entry and its length/charset limits per title.
2. Read `loginusers.vdf` on the maintainer's install (redact `AccountName`) to confirm the format.
3. Add a small reader in the crate that owns profile defaults, with a test over a fixture VDF string.
