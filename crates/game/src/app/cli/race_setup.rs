//! What a `--race` flies and where: sources, track, team, hull and mode.

/// What a `--race` flies and where: sources, track, team, hull and mode.
#[derive(clap::Args, Debug)]
pub(crate) struct RaceSetupArgs {
    /// A second disc image or extracted directory the craft, HUD and grid
    /// roster load from instead of `source` - a two-title race from the command
    /// line. Left out, the craft comes from `source` like every other race.
    /// See `oag_source::pair::SourcePair`.
    #[arg(long)]
    pub(crate) craft_source: Option<String>,

    /// The `HUD Style` skin for this run, one of the title's own list (`HD`, `2097`,
    /// `WIP3OUT` on Wipeout HD and Omega), over the saved `display.hud_style`. A name
    /// the title does not list draws its default skin and says so in the loader report.
    #[arg(long)]
    pub(crate) hud_style: Option<String>,

    /// Race as a downloadable campaign's event (`HD` or `Fury` on Wipeout 2048): the HUD reads
    /// that campaign's own layouts for the mode. What a cell launched from the campaign map sets.
    #[arg(long)]
    pub(crate) add_on_campaign: Option<String>,

    /// The track's `.vex` entry name, for either way into a race. The same name
    /// on both **Pulse** releases, PSP and PS2.
    ///
    /// **Left out, it comes from whichever title the source turns out to be**,
    /// which is why there is no `default_value` here: the two titles share no
    /// circuit directory at all, so a constant default would name a path that is
    /// not on a Pure disc and fail at the archive with a message about a missing
    /// entry rather than about a missing circuit. See [`default_track`].
    #[arg(long)]
    pub(crate) track: Option<String>,

    /// Race a Unity track the player already has instead of a circuit of the
    /// source: a path to a `.wtrk`, or a name matched against the files under
    /// the own-format track directories (`$OAG_TRACKS`, `data/tracks`, `<data dir>/oag/tracks`) and the BallisticNG ones (`$OAG_BALLISTICNG`, `data/ballisticng`, beside the
    /// program, `<data dir>/oag/ballisticng`; `tracks/*.wtrk` in each). The source
    /// still supplies the ship, rules and physics. See `oag_raceplay::foreign`.
    ///
    /// Only in a build with the `ballisticng` feature, which no shipped build
    /// enables.
    #[cfg(feature = "ballisticng")]
    #[arg(long)]
    pub(crate) ballisticng_track: Option<String>,

    /// Race a track in this project's own format (ADR-0063): the path of its
    /// `*.track.toml` manifest, with the `.glb` it names beside it. The source
    /// still supplies the ship, rules and physics. Conflicts with
    /// `--ballisticng-track` where a build has it. See `oag_trackfile`.
    #[cfg_attr(feature = "ballisticng", arg(conflicts_with = "ballisticng_track"))]
    #[arg(long)]
    pub(crate) track_file: Option<std::path::PathBuf>,

    /// The team, which selects both the handling stats and the model.
    ///
    /// A team **id**, which is the folder under `Data\Ships\` and not always
    /// the name on screen - the Mirage pack's team is `Mantis`. Teams a mounted
    /// pack adds are accepted here like any other; see `--dlc`.
    /// Left out, it comes from the source's own title, the same way `--track`
    /// does: Pulse's default team is not on a Pure disc under that spelling.
    #[arg(long)]
    pub(crate) team: Option<String>,

    /// The player's own hull file, for a title whose
    /// `oag_title::race::HullVariant` axis offers one - `"extra"` for
    /// Pulse's Concept model, left out for the baseline `Ship.vex`. A team
    /// with no such axis, or a stem the team does not offer, races the
    /// baseline instead and says so in the load report.
    #[arg(long)]
    pub(crate) variant: Option<String>,

    /// The player's own alternate paint job: a `PI_ModelSkin` name the team
    /// declares - `Alternative` or `Eliminator` on both Pulse releases, left
    /// out for the hull's own textures.
    ///
    /// A **skin**, not a hull: the same geometry with four texture slots
    /// swapped out of a `.dat` the definition names. A team that declares no
    /// such skin races the baseline and says so in the load report.
    ///
    /// **Which skin a race flies is this project's choice, not the
    /// original's**, and no unlock is checked - see
    /// `oag_livery`'s `ship_skin` module docs.
    #[arg(long)]
    pub(crate) skin: Option<String>,

    /// The livery folder the player's craft is painted from on Wipeout HD/Fury and Omega: a
    /// `PI_TeamModel` `texturelocation` other than its directory's own (`livery2` is Feisar's
    /// `SKIN1`, with `--team Feisar`). What Ship Select's model row writes after the `@` of a
    /// variant id. A folder that lacks a texture the hull names draws that part untextured and
    /// says so in the load report.
    #[arg(long)]
    pub(crate) livery: Option<String>,

    /// What a split-screen race's second and later people fly, one per flag in
    /// seat order: `TEAM` or `TEAM:SKIN` (a `PI_ModelSkin` name). The headless
    /// counterpart of picking on each seat's own Ship Select; with `--players N`,
    /// `--seat-team B --seat-team C` puts the player on `--team`, slot 1 on `B`
    /// and slot 2 on `C`. A team the source does not offer is the same case
    /// `--team` has: the load reports it. Chosen, not measured.
    #[arg(long = "seat-team", value_name = "TEAM[:SKIN]")]
    pub(crate) seat_team: Vec<String>,

    /// A directory holding downloadable content: a pack's `.edat` files, or the
    /// `.zip` they were downloaded as. Repeatable.
    ///
    /// Left out, it is searched for: `data/dlc/` in the current directory, then
    /// beside the AppImage, then `<data dir>/oag/dlc`. `$OAG_DLC` short-circuits
    /// that, and `oag_source::source::resolve_dlc` documents the order.
    ///
    /// **Any pack works against any release.** The original locked a pack to
    /// its own territory's disc; this does not. See `docs/formats/dlc-pack.md`.
    #[arg(long)]
    pub(crate) dlc: Vec<String>,

    /// The speed class: venom, flash, rapier or phantom - or vector (Wipeout
    /// Pure) or superphantom (Wipeout 2048), which only those titles author.
    ///
    /// Checked here against every ladder measured so far, which is
    /// spell-checking rather than availability: whether *this* source's files
    /// author the rung is settled when the race loads, and a source that does
    /// not names the ladder it does carry.
    #[arg(long, default_value = "venom")]
    pub(crate) class: String,

    /// The race mode: time_trial, speed_lap, zone or single_race.
    ///
    /// Needed on the `--race` path in particular, which skips the menus and so
    /// has no other way to pick one. `single_race` is the only one that races
    /// with weapons, so it is the only one where a `Weapon Pad` draws or hands
    /// anything out.
    #[arg(long, default_value = "time_trial")]
    pub(crate) mode: String,
    /// A Wipeout 2048 campaign event by name - see `oag_raceplay::load_event`.
    #[arg(long, value_name = "NAME")]
    pub(crate) event: Option<String>,
}
