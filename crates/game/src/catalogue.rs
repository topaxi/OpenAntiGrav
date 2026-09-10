//! What a source offers to race on, read from the source.
//!
//! The menus need a list of circuits, and the two obvious ways to get one are
//! both wrong. Hard-coding names would put shipped content in this repository,
//! which [ADR-0006](../../../docs/architecture/adr/0006-no-copyrighted-content.md)
//! forbids. Listing `Data\Environments\*` would list *directories*, and a
//! directory is not a race: `16_Track` and `32_Track` are two entries on the
//! menu and one folder on the disc, the second being the first driven the other
//! way.
//!
//! So the list comes from where the original's own front end gets it:
//! `Data\Plugins\PI001\Definition.xml`, whose `PI_Track` nodes each carry an
//! id, the environment they load and whether they run reversed. The name a
//! player sees is the id looked up in the language's string table, so it is as
//! localised as the rest of the front end and never appears in this repository.
//!
//! ```text
//! <PI_Track name="32_Track">
//!   <Values type="Race" soundregister="2" location="Data\Environments\16_Track"
//!           Reversed="True" availableInZone="true"/>
//! </PI_Track>
//! ```
//!
//! Confidence **88**: the structure is read off the shipped file and every
//! entry on the PSP disc resolves to an environment that exists, but nothing
//! here has been watched running under an emulator, and what `soundregister`,
//! `Grid` and `availableInZone` select is not established. See
//! `docs/formats/fexml.md`.
//!
//! # Teams come from the same file, and so does downloadable content
//!
//! `PI_Team` sits beside `PI_Track` in that definition and is read the same
//! way, for the same reason: the roster is data. A [DLC
//! pack](../../../docs/formats/dlc-pack.md) declares its additions as a
//! *fragment of this exact schema*, so mounting one adds nothing to parse -
//! [`teams`] and [`tracks`] run over the pack's manifest unchanged, and
//! [`all_teams`] / [`all_tracks`] concatenate the results.
//!
//! ## An id is not a name
//!
//! `PI_Team name="Mantis"` is an **id**: it is the folder under `Data\Ships\`,
//! it is what a setting stores, and it is not what a player reads. The name on
//! screen comes from the string table, keyed by that id - the disc's own
//! `entries.xml` maps `Mantis` to the name the game shows for it, in every
//! language the release carries. Spelling either one here would put shipped
//! content in this repository; see
//! [ADR-0006](../../../docs/architecture/adr/0006-no-copyrighted-content.md).

use oag_tables::fexml::{Node, parse};

/// One thing a player can pick on the Race page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    /// The plugin's own id, e.g. `32_Track`. This is what a setting stores: it
    /// names the *race*, where a directory names only the geometry.
    pub id: String,
    /// The environment directory, e.g. `Data\Environments\16_Track`.
    pub location: String,
    /// Whether this entry is the circuit run the other way round.
    pub reversed: bool,
    /// Whether a Zone race can be run on this circuit.
    ///
    /// **Read off the entry's own `availableInZone="true"`, and it is a
    /// load-correctness fact rather than a menu one.** On a title whose Zone
    /// circuits are the race ones with a prefixed file
    /// ([`oag_title::ZoneCircuit::Prefixed`]), a circuit without this attribute
    /// carries no `zone_track.vex` at all, so a Zone race on it asks the archive
    /// for a name that hashes to nothing.
    ///
    /// That correlation is measured rather than assumed: all 24 of Pulse's
    /// `PI_Track` entries were probed by name against `pulse-psp-usa.chd` and
    /// the attribute predicts the file **24 times out of 24** - the sixteen that
    /// declare it have their variant, the eight that do not have none.
    /// `crates/game/tests/zone_ground_truth.rs` is that sweep.
    ///
    /// `false` on a title that declares no such attribute anywhere, which is
    /// every title whose Zone circuits are separate - Pure marks its four
    /// `type="Zone"`, HD/Fury marks its own four `zone="true"` on an
    /// otherwise ordinary `type="Race"` entry, and [`tracks_named`] is what
    /// finds either by the name each title already knows its own circuits
    /// by, rather than by which of the two spellings the disc used.
    pub available_in_zone: bool,
}

impl Track {
    /// The archive entry name of the `.vex` a *race* loads on this circuit.
    ///
    /// The four-file rule from `docs/formats/track.md`: a circuit ships
    /// `track.vex`, `track_reversed.vex` and the two zone variants, and a
    /// `PI_Track` picks between the first two with `Reversed`.
    ///
    /// **The Zone pair is reached by rewriting this**, not by a second method
    /// here: which rewrite applies is a property of the title rather than of the
    /// circuit, so it lives in [`oag_title::ZoneCircuit::variant_of`] and this
    /// stays the one name a `PI_Track` names by itself.
    #[must_use]
    pub fn entry_name(&self) -> String {
        let file = if self.reversed {
            "track_reversed"
        } else {
            "track"
        };
        format!(r"{}\{file}.vex", self.location)
    }
}

/// Reads every raceable `PI_Track` out of a plugin definition, in file order.
///
/// File order on purpose: it is the order the original's own front end would
/// walk, and it puts `16_Track` first, which is the circuit every capture under
/// `data/traces/` was taken on.
///
/// Entries whose `type` is not `Race` are skipped rather than guessed at - what
/// another value means is a per-title question, and [`tracks_of_kind`] is how a
/// caller that knows the answer asks it.
#[must_use]
pub fn tracks(definition_xml: &str) -> Vec<Track> {
    tracks_of_kind(definition_xml, "Race")
}

/// Every `PI_Track` of one `type`, in file order.
///
/// [`tracks`] is this with `"Race"`, which is every circuit both PSP titles let
/// a player race normally. The other values the shipped definitions use are
/// **per-title vocabulary**, which is why this takes the kind rather than
/// offering a variant per name: Pure declares `Zone` (its four Zone-mode
/// circuits) and `Classic` (the Wipeout 1 and 2097 tracks it re-ships), and
/// Pulse declares neither - all 24 of its entries are `Race`, and its Zone
/// circuits are a second file inside those same directories rather than
/// entries of their own.
///
/// So a caller wanting Zone's circuits has to know which title it opened before
/// it knows which question to ask, and [`oag_title::ZoneCircuit`] is what tells
/// it. That dispatch is deliberately **not** wrapped up here: it belongs to the
/// caller that has the title in hand, and a helper doing it would be a title
/// axis living in the engine.
///
/// An entry with no `type` attribute counts as `Race` - no shipped definition
/// omits it, so this is the reading of an absence rather than a rule anything
/// relies on.
#[must_use]
pub fn tracks_of_kind(definition_xml: &str, kind: &str) -> Vec<Track> {
    let root = parse(definition_xml);
    let mut out = Vec::new();
    collect(&root, Some(kind), &mut out);
    out
}

/// Every `PI_Track` in `definition_xml` whose own entry name matches one of
/// `names`, regardless of `type` - kept in `names`' own order rather than file
/// order, since [`oag_title::ZoneCircuit::Separate`] indexes that order with
/// `[0]` for `default_track` and a menu offering the list wants that same
/// circuit shown first.
///
/// **The reason this exists alongside [`tracks_of_kind`]: `type` is not a
/// reliable way to ask "which are this title's Zone circuits".** Pure
/// declares its four `type="Zone"`, which `tracks_of_kind(xml, "Zone")`
/// finds; HD/Fury declares its own four `type="Race"` with a separate
/// `zone="true"` flag - measured 2026-08-28 against
/// `hdfury-ps3-eu-dec.iso`'s `Data\Plugins\frontend\definition.xml`, all four
/// and only the four pointing at `Zone_1`..`Zone_4`, out of 28 `PI_Track`
/// entries total - so the same kind query answers empty there. Matching by
/// the title's own declared name list (`oag_hd::race::ZONE_TRACKS`) sidesteps
/// the question entirely: it does not care what `type` said, only whether the
/// entry names the circuit the title says is its own.
#[must_use]
pub fn tracks_named(definition_xml: &str, names: &[&str]) -> Vec<Track> {
    let root = parse(definition_xml);
    let mut all = Vec::new();
    collect(&root, None, &mut all);
    names
        .iter()
        .filter_map(|&name| {
            all.iter()
                .find(|track| paths_match(&track.entry_name(), name))
                .cloned()
        })
        .collect()
}

fn collect(node: &Node, kind: Option<&str>, out: &mut Vec<Track>) {
    for child in &node.children {
        if child.name == "PI_Track" {
            if let Some(track) = read_track(child, kind) {
                out.push(track);
            }
        } else {
            collect(child, kind, out);
        }
    }
}

/// Whether two entry names address the same archive entry.
///
/// Mirrors [`oag_title::race`]'s own `paths_match`, not shared: `oag-game`
/// does not depend on `oag-title` for this, and this is a small enough fold
/// (leading `/` trimmed, `\` folded to `/`, ASCII-lowercased) that repeating
/// it here costs less than a dependency would.
fn paths_match(a: &str, b: &str) -> bool {
    fn normalise(path: &str) -> String {
        path.trim_start_matches('/')
            .replace('\\', "/")
            .to_ascii_lowercase()
    }
    normalise(a) == normalise(b)
}

/// One track of the soundtrack, as the plugin declares it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Music {
    /// The plugin's own id. **This one is a title, not a folder id**, unlike
    /// [`Track::id`] and [`Team::id`] - the disc spells the piece of music out
    /// here rather than keying a string table with it, which is why nothing in
    /// this repository may repeat it. Read at run time, printed at most in a
    /// load report.
    pub id: String,
    /// The directory the track lives in, e.g. `Data\Music\SomeArtist`.
    pub location: String,
}

impl Music {
    /// The archive entry name of the audio this track plays.
    ///
    /// `file` is [`oag_title::DeclaredTracks::file`] - `MusicManager.cpp`'s own
    /// `%s\%s` join, the title package supplying the second `%s`. It is a
    /// parameter rather than a constant because a title whose soundtrack this
    /// build has not recovered by name has no answer to give.
    #[must_use]
    pub fn entry_name(&self, file: &str) -> String {
        format!(r"{}\{file}", self.location)
    }
}

/// Reads every `PI_Music` out of a plugin definition, in file order.
///
/// File order on purpose, and it is the one thing this buys over finding the
/// same entries by what they *are*: it is the order the original's own
/// `MusicSelection` screen walks, so "the first track" means something the disc
/// decides rather than something the archive directory happens to do.
///
/// The `Artist` and `Label` an entry also carries are deliberately **not**
/// read, the same way [`teams`] skips `PI_TeamModel`: nothing displays them,
/// and a public field holding shipped text that no caller reads is worse than
/// one that appears when it is needed.
#[must_use]
pub fn music(definition_xml: &str) -> Vec<Music> {
    let root = parse(definition_xml);
    let mut out = Vec::new();
    collect_music(&root, &mut out);
    out
}

fn collect_music(node: &Node, out: &mut Vec<Music>) {
    for child in &node.children {
        if child.name == "PI_Music" {
            if let Some(track) = read_music(child) {
                out.push(track);
            }
        } else {
            collect_music(child, out);
        }
    }
}

fn read_music(node: &Node) -> Option<Music> {
    let id = node.attr("name")?.to_string();
    let values = node.children_named("Values").next()?;
    Some(Music {
        id,
        location: values.attr("location")?.to_string(),
    })
}

/// One team a player can race for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Team {
    /// The ship folder, e.g. `Feisar`: the leaf of [`Self::location`].
    ///
    /// What a setting stores, what `--team` accepts, the key the string table's
    /// display name is under, and the component every `Data\Ships\...` path a
    /// race builds is assembled from - **not** the display name itself. See the
    /// module docs.
    ///
    /// **Read off the location rather than the `name` attribute**, which is the
    /// same word on Pulse and on all four packs and is not on Wipeout Pure:
    /// that disc declares `name="AG Systems"` over
    /// `location="Data\Ships\AG_Systems"`, and composing a path from the space
    /// missed both the hull and the handling stats, so a founding team was
    /// dropped from Pure's roster on both pressings. See [`Self::name`].
    pub id: String,
    /// The declared name, where the disc spells it differently from [`Self::id`].
    ///
    /// `None` whenever the two agree, which is every team on every other
    /// source. So this is not "the label": it is the label the *definition*
    /// carries, for when the string table cannot be asked. The table is still
    /// asked first, and this is what stops Pure's menu reading `AG_Systems`
    /// while its string tables go unread.
    pub name: Option<String>,
    /// The ship directory, e.g. `Data\Ships\Feisar`.
    pub location: String,
    /// The string-table id of the team's description, e.g. the `helpText`
    /// attribute. `None` for a team that declares none.
    pub help_text: Option<String>,
    /// The alternate paint jobs this team declares, in file order.
    ///
    /// Every `PI_ModelSkin` nested under this team's
    /// `PI_TeamModel name="Normal"`, which on every source measured so far is
    /// exactly two - `Alternative` and `Eliminator`. Empty for a team that
    /// declares none, and for a source whose definition has no
    /// `PI_TeamModel` at all.
    ///
    /// **The hull axis is not here.** `PI_TeamModel`'s own `Concept`/`Zone`
    /// entries name a *file stem* inside the team directory and are
    /// [`oag_title::race::HullVariant`]'s, which needs no per-team data
    /// because every team offers the same two stems. A skin is the other
    /// thing entirely: a texture swap on the same geometry, addressed by a
    /// full path the team declares for itself.
    pub skins: Vec<ModelSkin>,
    /// The four ratings the front end's `Team Selection` bars show, off the
    /// team's own `<FE speed=".." thrust=".." handling=".." shield="..">`
    /// element. `None` for a definition that authors none.
    ///
    /// **This is the table `race-setup.md` recorded as unlocated** - the
    /// per-craft record `TeamSelection_Update` reads through `FUN_08808664`
    /// at `+0xb4..+0xc0` - and it was in the plugin definition all along,
    /// under a child element named for the front end. Assegai's authored
    /// `8/8/9/7`, Qirex's `8/7/8/9`, AG Systems' `7/9/9/8` and Piranha's
    /// `10/6/6/9` are exactly the four bars a 2026-09-09 PPSSPP capture of the
    /// screen shows (`docs/ui/selection-screens.md`), which is what settles
    /// it as the source rather than a coincidence. Distinct from
    /// `HandlingStats.xml`: these are the *advertised* ratings, not the
    /// physics.
    pub rating: Option<Rating>,
}

/// The disc's own string id for a team's baseline paint - `Classic` on
/// Pulse's `Team Selection`, resolved through the language table like every
/// other label and falling back to the id itself, which is the English
/// spelling. What the livery row calls the entry that names no
/// [`ModelSkin`].
pub const BASELINE_SKIN: &str = "Classic";

/// A team's four front-end ratings, each `0..=10`. See [`Team::rating`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rating {
    pub speed: u8,
    pub thrust: u8,
    pub handling: u8,
    pub shield: u8,
}

/// One alternate paint job a team declares - a `PI_ModelSkin`.
///
/// **A skin is a texture swap on the same geometry, not a second model**:
/// [`Self::location`] names a `.dat` of four palette-plus-pixels blocks that
/// go over the hull's own `texture1.tga`..`texture4.tga`. See
/// [`oag_texture::ship_skin`] and
/// `docs/ghidra/functions/psp-pulse-usa/ship-skin.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelSkin {
    /// The declared name, e.g. `Alternative` or `Eliminator`.
    ///
    /// **Per-title vocabulary, kept as text** for the reason
    /// [`tracks_of_kind`] takes its `kind` as a `&str`: an enum here would
    /// hard-code two shipped names into this repository and would have
    /// nothing to say about a third a pack might declare.
    pub name: String,
    /// The archive entry the skin's bytes are in, e.g.
    /// `Data\Ships\Assegai\ship_alt.dat`.
    ///
    /// **A full path, used verbatim - never composed.** This is the same
    /// `location` attribute whose sibling on `PI_TeamModel` holds a bare file
    /// *stem* (`"ship"`), which is the naming trap `docs/formats/dlc-pack.md`
    /// and `crate::livery`'s module docs both record. Nothing may `format!`
    /// this path out of a team id.
    pub location: String,
}

impl Team {
    /// The declared skin of that name, matched case-insensitively.
    ///
    /// Case-insensitive because the name comes from a player - a `--skin`
    /// argument, or a stored setting - and the disc's own spelling
    /// (`Alternative`) is not what anyone types. `None` when the team
    /// declares no such skin, which a caller reports rather than
    /// substituting another team's.
    #[must_use]
    pub fn skin(&self, name: &str) -> Option<&ModelSkin> {
        self.skins
            .iter()
            .find(|skin| skin.name.eq_ignore_ascii_case(name))
    }

    /// What to show a player for this team.
    ///
    /// **The order is the whole content of this function**, and the middle step
    /// is the one that is easy to get wrong: the string table wins, because
    /// `Mantis` is labelled `Mirage` there and a build that preferred the
    /// definition's own spelling would show the folder name for a pack the
    /// packaging calls something else. [`Self::name`] is only reached for when
    /// the table cannot answer, which on Wipeout Pure is always - its string
    /// tables go unread - and the id is the last resort rather than the first.
    #[must_use]
    pub fn label<'a>(&'a self, strings: &'a oag_ui::language::StringTable) -> &'a str {
        strings
            .get(&self.id)
            .or(self.name.as_deref())
            .unwrap_or(&self.id)
    }
}

/// Reads every raceable `PI_Team` out of a plugin definition, in file order.
///
/// **The `PI_ModelSkin`s under `PI_TeamModel name="Normal"` are collected**
/// into [`Team::skins`], and nothing else about `PI_TeamModel` is: its own
/// `Concept`/`Zone` entries are [`oag_title::race::HullVariant`]'s axis,
/// which needs no per-team data. The `Unlock` pair each skin carries is
/// **deliberately not read** - what `loyalty` accumulates is untraced, so a
/// field holding it would be a number no caller could interpret. See
/// `docs/formats/dlc-pack.md`'s "Entry 0 is a manifest" section for the
/// schema.
#[must_use]
pub fn teams(definition_xml: &str) -> Vec<Team> {
    let root = parse(definition_xml);
    let mut out = Vec::new();
    collect_teams(&root, &mut out);
    out
}

fn collect_teams(node: &Node, out: &mut Vec<Team>) {
    for child in &node.children {
        if child.name == "PI_Team" {
            if let Some(team) = read_team(child) {
                out.push(team);
            }
        } else {
            collect_teams(child, out);
        }
    }
}

fn read_team(node: &Node) -> Option<Team> {
    let declared = node.attr("name")?.to_string();
    let values = node.children_named("Values").next()?;
    if values.attr("type").is_some_and(|kind| kind != "Race") {
        return None;
    }
    let location = values.attr("location")?.to_string();
    // The **folder**, not the declared name. Every path a race builds for a team
    // is `Data\Ships\<folder>\...`, so the folder is the id and the declared
    // name is a label - which is the split `Mantis` already forced, its pack
    // being sold as Mirage. Taking the name would work on all twelve of Pulse's
    // teams and on none of the cases where the two differ.
    let id = location
        .rsplit(['\\', '/'])
        .next()
        .filter(|leaf| !leaf.is_empty())?
        .to_string();
    // Carried only where it says something the id does not. On Pulse and on
    // every pack the two are the same word and this is `None`, which keeps the
    // label lookup below on the string table where it belongs.
    let name = (declared != id).then_some(declared);
    Some(Team {
        id,
        name,
        location,
        help_text: values.attr("helpText").map(str::to_string),
        skins: read_skins(node),
        rating: read_rating(node),
    })
}

/// The `<FE>` child's four ratings, or `None` unless all four are there.
fn read_rating(team: &Node) -> Option<Rating> {
    let fe = team.children_named("FE").next()?;
    let number = |attr: &str| fe.attr(attr)?.trim().parse::<u8>().ok();
    Some(Rating {
        speed: number("speed")?,
        thrust: number("thrust")?,
        handling: number("handling")?,
        shield: number("shield")?,
    })
}

/// Every `PI_ModelSkin` nested under this team's `PI_TeamModel name="Normal"`.
///
/// **Only under `Normal`.** `dlc-pack.md` measured the shape across all eight
/// disc teams and a pack manifest: `Normal` is the one variant with skins, and
/// `Concept`/`Zone` are each a single unlockable model carrying their `Unlock`
/// directly. A skin found under either of those would be a shape nothing has
/// seen, and collecting it here would quietly widen a measured schema.
fn read_skins(team: &Node) -> Vec<ModelSkin> {
    team.children_named("PI_TeamModel")
        .filter(|model| model.attr("name") == Some("Normal"))
        .flat_map(|model| model.children_named("PI_ModelSkin"))
        .filter_map(|skin| {
            Some(ModelSkin {
                name: skin.attr("name")?.to_string(),
                location: skin
                    .children_named("Values")
                    .next()?
                    .attr("location")?
                    .to_string(),
            })
        })
        .collect()
}

/// Every team across `documents`, the first spelling of an id winning.
///
/// Pass the source's own `Definition.xml` first and each mounted pack's
/// manifest after it, which is the order
/// [`oag_pulse::open_with_packs`] mounts them in. First-wins
/// then means the same thing here as it does there: the disc is authoritative
/// and a pack can only add.
#[must_use]
pub fn all_teams(documents: &[String]) -> Vec<Team> {
    let mut out: Vec<Team> = Vec::new();
    for document in documents {
        for team in teams(document) {
            if !out.iter().any(|seen| seen.id == team.id) {
                out.push(team);
            }
        }
    }
    out
}

/// Every race across `documents`, the first spelling of an id winning.
///
/// The counterpart of [`all_teams`], with one caveat that matters for a partial
/// pack set: a `PI_Track` is only a *declaration*, and a player who owns one
/// pack of a set can hold a declaration for a circuit whose geometry is in a
/// pack they do not have. Callers filter on
/// [`oag_assets::Archives::locate`] of [`Track::entry_name`] for that
/// reason - the check belongs where the archives are, not here.
///
/// [`all_tracks_of_kind`] with `"Race"`.
#[must_use]
pub fn all_tracks(documents: &[String]) -> Vec<Track> {
    all_tracks_of_kind(documents, "Race")
}

/// Every circuit of one `type` across `documents`, the first spelling of an id
/// winning.
///
/// [`all_tracks`] is this with `"Race"`. A caller that knows its title's Zone
/// circuits are declared apart from the race ones -
/// [`oag_title::ZoneCircuit::Separate`] - asks for `"Zone"` instead, the same
/// dispatch [`oag_title::ZoneCircuit::menu_tracks`] makes on the caller's
/// behalf.
#[must_use]
pub fn all_tracks_of_kind(documents: &[String], kind: &str) -> Vec<Track> {
    let mut out: Vec<Track> = Vec::new();
    for document in documents {
        for track in tracks_of_kind(document, kind) {
            if !out.iter().any(|seen| seen.id == track.id) {
                out.push(track);
            }
        }
    }
    out
}

/// [`tracks_named`] across `documents`, the first spelling of an id winning -
/// on the same partial-pack-set terms as [`all_tracks`].
#[must_use]
pub fn all_tracks_named(documents: &[String], names: &[&str]) -> Vec<Track> {
    let mut out: Vec<Track> = Vec::new();
    for document in documents {
        for track in tracks_named(document, names) {
            if !out.iter().any(|seen| seen.id == track.id) {
                out.push(track);
            }
        }
    }
    out
}

fn read_track(node: &Node, wanted: Option<&str>) -> Option<Track> {
    let id = node.attr("name")?.to_string();
    let values = node.children_named("Values").next()?;
    if let Some(wanted) = wanted
        && values.attr("type").unwrap_or("Race") != wanted
    {
        return None;
    }
    Some(Track {
        id,
        location: values.attr("location")?.to_string(),
        // The shipped file spells it `Reversed="True"`, capitalised, which is
        // why this goes through `flag` rather than comparing to "true".
        reversed: values.flag("Reversed").unwrap_or(false),
        // Lowercase `true` here, where `Reversed` is capitalised - the disc is
        // inconsistent about it and `flag` accepts either, which is the whole
        // reason it exists.
        available_in_zone: values.flag("availableInZone").unwrap_or(false),
    })
}

/// The `FE_REVERSE` key: the front end's own word for a circuit driven the
/// other way.
///
/// A key rather than a word, so what is appended below is the disc's own text
/// in the player's own language - `REVERSE` in english, `RÜCKWÄRTS` in german.
/// Present in all five of Wipeout HD's copies of the string table.
const REVERSE_KEY: &str = "FE_REVERSE";

/// What the RACE page shows for one circuit.
///
/// The name out of [`oag_ui::language::CircuitNames`], falling back to the
/// string table and then to the id - so a circuit whose name nothing on the
/// source resolves shows the id, which is a visible absence rather than a blank
/// row.
///
/// # The one thing this composes, and why it is not an invention
///
/// **Wipeout HD names a circuit and its reverse identically.** `01_Track` and
/// `09_Track` are Vineta K forward and backward, and the copy of the string
/// table that pairs with HD's circuit list reads `VINETA K` for both - eight
/// such pairs among the numbered circuits and four more among the Fury ones.
/// The older copies carried a suffix (`13_TRACK_OLD` is `VINETA K REVERSE`) and
/// the newer numbering dropped it, which is the renumbering saying so in the
/// data: the original draws the direction beside the name rather than inside
/// it, and ships [`REVERSE_KEY`] to draw it with.
///
/// This build's RACE page has one column, so twelve pairs of identical rows
/// would be a menu a player cannot use. So where a reversed circuit resolves to
/// **the same name as the circuit it shares an environment with**, that key's
/// own string is appended. Where the names already differ - which is every
/// circuit on both PSP titles, whose tables name `32_Track` separately from
/// `16_Track` - nothing is appended and this is the name unchanged.
///
/// Two of the disc's own strings joined by this build, then, rather than a word
/// this build made up, and only where the data itself is ambiguous. See
/// `docs/formats/hd-frontend.md`.
#[must_use]
pub fn label(
    track: &Track,
    names: &oag_ui::language::CircuitNames,
    strings: &oag_ui::language::StringTable,
    every: &[Track],
) -> String {
    let name = names
        .get(&track.id)
        .unwrap_or_else(|| strings.get_or_id(&track.id));

    if !track.reversed || !shares_a_name_with_its_forward_twin(track, name, names, strings, every) {
        return name.to_string();
    }
    match strings.get(REVERSE_KEY) {
        Some(reverse) => format!("{name} {reverse}"),
        None => name.to_string(),
    }
}

/// Whether the circuit this one reverses is named the same thing.
fn shares_a_name_with_its_forward_twin(
    track: &Track,
    name: &str,
    names: &oag_ui::language::CircuitNames,
    strings: &oag_ui::language::StringTable,
    every: &[Track],
) -> bool {
    every
        .iter()
        .filter(|other| !other.reversed && other.location == track.location)
        .any(|other| {
            names
                .get(&other.id)
                .unwrap_or_else(|| strings.get_or_id(&other.id))
                == name
        })
}

#[cfg(test)]
mod tests;
