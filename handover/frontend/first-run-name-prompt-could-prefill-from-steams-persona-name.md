---
status: ready-to-wire
titles: [all]
updated: 2026-11-06
summary: "Shared name and Pulse/Pure tag are built and stored; Steam persona prefill is open, plus a Deck keyboard call and a phone check"
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

- **Pulse's `ENTER YOUR NAME` title is nearly invisible**: dark glyphs on the dark panel in the `TagInput` prompt (read on `--menu-prompt player-name`, Pulse EU). `Prompt::frame` asks for white, so the face's own colour wins there; HD's grid draws it white. The pilot rename shares the frame, so check it too.
- **Split-screen tags still say `Player 1`**: `Race::tag_name` (`crates/raceplay/src/player_tags.rs`) does not read `settings.player.name()`. Seat 1 takes the stored name through `Race::set_person_names`, from the race start in `oag-game`.
- **2048 and Omega never ask**: `Title::profile` follows the originals, so a player who only runs those has no name for the lobby but the login name. Decide whether the shared name is asked for there too.
- **Built (2026-11-06, `docs/ui/player-name.md`).** `[player] name`/`tag` in settings, `Title::profile`,
  the first-run prompt (Pulse's `TagInput` row, the grid elsewhere), OPTIONS rows, the footer tag, the
  lobby default and the desk keyboard. Prefill from Steam is **not** built and stays open below.
- **Where the prefill hooks in**: `session::player_name::Session::ask_player` opens the name prompt with
  an empty initial text; a prefill passes `clean_name(persona)` as `initial` in
  `open_player_prompt`. The charset rule (`settings::player::clean_name`) already drops what a font
  cannot draw, so a persona name with other characters comes out cut, and an empty result falls
  through to the empty prompt.
- **Paths unchecked.** Linux `~/.local/share/Steam/config/loginusers.vdf`, `~/.steam/steam`, Flatpak
  `~/.var/app/com.valvesoftware.Steam/.local/share/Steam`; Windows from the registry's Steam path.
  macOS and the Deck not looked at. Android, iOS and web have no Steam: the prefill is a no-op for Steam.
- **OS name source unchosen.** Login name (`USER`/`USERNAME`, `whoami`) is easy but often `root`,
  `deck` or lowercase; the full name (GECOS, Windows display name) is nicer but needs platform code
  or a crate. A generic login (`deck`, `root`, `user`, `administrator`) is probably worth skipping.
  Android, iOS and web have no useful OS user name: empty prompt there.
- **Not seen.** The file's exact format on the maintainer's install has not been read yet.
- **Steam Deck virtual keyboard: not wired.** Whether `steam://open/keyboard` opens it from a non-Steam
  shortcut is unrun; steps are in `docs/ui/player-name.md`, "Keyboards". The controller path works there.
- **Disc read on the render thread**: `pulse_tag_entry` (inherited from the pilot rename); see
  `docs/ui/player-name.md`, "Open".
- **Phone soft keyboard: written, not driven.** `soft_keyboard::sync_soft_keyboard` (winit
  `set_ime_allowed`); the Android target could not be built where it was written (no NDK clang).
- **Desk typing into a name is as typed** (capitals kept); on the grid it folds to the grid's case.
  Pulse's footer tag colour and face are the file's colour and the ticker's face, unchecked against
  the original's `AAA`.
- **Not measured.** Pure's own name and tag entry (no front-end XML on its disc): it takes the grid.
  Precedent for reading the Steam file (Playnite, SteamTinkerLaunch, Steam ROM Manager, from memory)
  is unverified; the file is undocumented by Valve and may change.

## Next Steps

1. Read `loginusers.vdf` on the maintainer's install (redact `AccountName`) to confirm the format.
2. Add a small reader in the crate that owns profile defaults, with a test over a fixture VDF string,
   and feed it to `open_player_prompt` as the first-run `initial`.
3. On a Deck, run the `steam://open/keyboard` check from `docs/ui/player-name.md`; on a phone, open
   PLAYER NAME and see the soft keyboard rise.
