//! `oag-mod.toml`: who a mod is, and under what licence it travels.

use oag_tables::pickup_look::{Colour, Icon, PickupLook};
use serde::Deserialize;

/// The manifest's name at a mod's root.
pub const MANIFEST: &str = "oag-mod.toml";
/// The schema kind the header names.
pub const SCHEMA_KIND: &str = "mod";
/// The one schema major this build reads.
pub const SCHEMA_MAJOR: u32 = 1;

/// What is wrong with a manifest.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Not TOML, or not shaped like a manifest.
    #[error("{0}")]
    Toml(#[from] toml::de::Error),
    /// No `[toml-schema]` header, or one naming another kind or major.
    #[error("{0}")]
    Schema(#[from] oag_schema::Error),
    /// A key with a value that cannot be right.
    #[error("[mod] {key}: {problem}")]
    Bad {
        /// The key.
        key: &'static str,
        /// What is wrong.
        problem: String,
    },
}

/// A mod's licence, as stated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum License {
    /// `license = "<SPDX expression>"`.
    Spdx(String),
    /// `license-file = "<path in the mod>"`.
    File(String),
    /// Neither: the mod loads, and says so wherever it is listed.
    None,
}

impl License {
    /// The words a mods list and the load report use.
    #[must_use]
    pub fn report(&self) -> String {
        match self {
            Self::Spdx(expression) => expression.clone(),
            Self::File(path) => format!("see {path}"),
            Self::None => "no license stated".to_string(),
        }
    }
}

/// A checked manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// Unique and stable: `topaxi.example`.
    pub id: String,
    /// The name a player reads.
    pub name: String,
    /// The mod's own semver release.
    pub version: semver::Version,
    /// Who made it.
    pub authors: Vec<String>,
    /// The licence.
    pub license: License,
    /// One or two sentences.
    pub description: String,
    /// Where the source lives.
    pub repository: Option<String>,
    /// A page about it.
    pub homepage: Option<String>,
    /// Search words.
    pub keywords: Vec<String>,
    /// The titles it is for (their lowercased hyphenated names); empty means any.
    pub titles: Vec<String>,
    /// The game versions it works with.
    pub oag: Option<semver::VersionReq>,
    /// `[extension]`: the WebAssembly modules the mod runs, if any.
    pub extension: Option<Extension>,
    /// `[icons]`: the mod's own PNGs by name, as paths in the mod. A module's `hud-icon`
    /// may name one.
    pub icons: std::collections::BTreeMap<String, String>,
    /// `[effects]`: the mod's own particle effects by name, as `.toml` paths in the mod
    /// (`schemas/v1/effect.tosd`). A module's `fire-mod-effect` names one by its index in
    /// this map's name order.
    pub effects: std::collections::BTreeMap<String, String>,
    /// `[pickup]`: the look of the mod's own pickup, required of a simulation mod that
    /// hands one out.
    pub pickup: Option<PickupLook>,
    /// `[channels]`: the event channels the mod's modules publish and listen to.
    pub channels: Channels,
}

/// `[channels]` (`docs/architecture/wasm-extensions/hooks.md`, "Events between mods"):
/// namespaced names, `<mod id>/<name>`. A module may `emit` only on a channel in
/// `publishes`, which lies in the mod's own namespace; `listens` is what the mod hears,
/// from any mod or track. Listening to a channel no loaded mod or track publishes is
/// warned about on the mods page, never refused: the publisher may simply be off.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Channels {
    /// Channels the mod sends on.
    pub publishes: Vec<String>,
    /// Channels the mod hears.
    pub listens: Vec<String>,
}

/// Most channels one mod publishes, or one listens to. Chosen.
pub const MAX_CHANNELS: usize = 32;

/// `[extension]` (`docs/architecture/wasm-extensions/README.md`, section 1.1). Read
/// here; the `.wasm` it names is opened only by the race that runs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extension {
    /// `simulation = true`: the mod changes the race, so the session is MODDED.
    pub simulation: bool,
    /// The ABI major the modules were built against.
    pub api: u32,
    /// The guest's maximum linear memory, 64 KiB pages.
    pub memory_pages: u32,
    /// The host-owned state blob's length, bytes.
    pub state_bytes: u32,
    /// `[extension.rules]`: a rules module, which may act on every craft of a race.
    pub rules: Option<Rules>,
    /// `[extension.track]`: a module that runs only on one track file the mod ships.
    pub track: Option<Track>,
}

/// `[extension.track]`: one module bound to one track file of the mod. It runs only in
/// a race of that file, never on a disc circuit, and may send the same commands as a
/// rules module (the race-wide outputs are what it is for).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    /// The module's path in the mod.
    pub module: String,
    /// The `.track.toml` it runs on, as a path in the mod.
    pub track: String,
}

/// `[extension.rules]`: one module that runs in every race the mod is on for, on any
/// grid slot. The README binds a rules module to a mode file (`modes/*.toml`), which
/// does not exist yet; until it does, a mod's rules module applies to every race but
/// the original campaigns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rules {
    /// The module's path in the mod.
    pub module: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(rename = "toml-schema")]
    header: Option<oag_schema::Header>,
    #[serde(rename = "mod")]
    module: Table,
    extension: Option<ExtensionTable>,
    channels: Option<ChannelsTable>,
    #[serde(default)]
    icons: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    effects: std::collections::BTreeMap<String, String>,
    pickup: Option<PickupTable>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PickupTable {
    icon: IconTable,
    colour: ColourTable,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IconTable {
    title: Option<String>,
    #[serde(rename = "mod")]
    module: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ColourTable {
    title: Option<String>,
    rgba: Option<String>,
}

/// The weapon a lowercase `title =` names, in the spelling of
/// `Weapon::as_type` lowercased (`leachbeam`, `repulser`).
fn weapon(key: &'static str, name: &str) -> Result<oag_tables::weapons::Weapon, Error> {
    oag_tables::weapons::Weapon::ALL
        .into_iter()
        .find(|weapon| weapon.as_type().eq_ignore_ascii_case(name))
        .ok_or_else(|| {
            let known: Vec<String> = oag_tables::weapons::Weapon::ALL
                .iter()
                .map(|weapon| weapon.as_type().to_ascii_lowercase())
                .collect();
            bad(
                key,
                format!("{name:?} is not a weapon: {}", known.join(", ")),
            )
        })
}

fn pickup(
    table: PickupTable,
    icons: &std::collections::BTreeMap<String, String>,
) -> Result<PickupLook, Error> {
    let icon = match (table.icon.title, table.icon.module) {
        (Some(title), None) => Icon::Title(weapon("pickup.icon", &title)?),
        (None, Some(name)) if icons.contains_key(&name) => Icon::Mod(name),
        (None, Some(name)) => {
            return Err(bad(
                "pickup.icon",
                format!("mod = {name:?} is not a key of [icons]"),
            ));
        }
        _ => {
            return Err(bad(
                "pickup.icon",
                "exactly one of `title = \"<weapon>\"` and `mod = \"<icon>\"`",
            ));
        }
    };
    let colour = match (table.colour.title, table.colour.rgba) {
        (Some(title), None) => Colour::Title(weapon("pickup.colour", &title)?),
        (None, Some(hex)) => {
            let digits = hex.len() == 8 && hex.bytes().all(|b| b.is_ascii_hexdigit());
            let word = digits
                .then(|| u32::from_str_radix(&hex, 16).ok())
                .flatten()
                .ok_or_else(|| {
                    bad(
                        "pickup.colour",
                        format!("rgba = {hex:?} is not eight hex digits, RRGGBBAA"),
                    )
                })?;
            Colour::Rgba(word.to_be_bytes())
        }
        _ => {
            return Err(bad(
                "pickup.colour",
                "exactly one of `title = \"<weapon>\"` and `rgba = \"RRGGBBAA\"`",
            ));
        }
    };
    Ok(PickupLook { icon, colour })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChannelsTable {
    #[serde(default)]
    publishes: Vec<String>,
    #[serde(default)]
    listens: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExtensionTable {
    #[serde(default)]
    simulation: bool,
    api: u32,
    #[serde(default = "default_memory_pages")]
    memory_pages: u32,
    #[serde(default)]
    state_bytes: u32,
    rules: Option<RulesTable>,
    track: Option<TrackTable>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TrackTable {
    module: String,
    track: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RulesTable {
    module: String,
}

/// 1 MiB. Chosen.
fn default_memory_pages() -> u32 {
    16
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Table {
    id: String,
    name: String,
    version: String,
    authors: Vec<String>,
    license: Option<String>,
    #[serde(rename = "license-file")]
    license_file: Option<String>,
    description: String,
    repository: Option<String>,
    homepage: Option<String>,
    #[serde(default)]
    keywords: Vec<String>,
    #[serde(default)]
    titles: Vec<String>,
    oag: Option<String>,
}

fn bad(key: &'static str, problem: impl Into<String>) -> Error {
    Error::Bad {
        key,
        problem: problem.into(),
    }
}

impl Manifest {
    /// Reads and checks a manifest. `has` says whether the mod holds a path,
    /// for `license-file`.
    ///
    /// # Errors
    ///
    /// Anything in [`Error`].
    pub fn parse(text: &str, has: &dyn Fn(&str) -> bool) -> Result<Self, Error> {
        let file: File = toml::from_str(text)?;
        oag_schema::check(file.header.as_ref(), SCHEMA_KIND, SCHEMA_MAJOR)?;
        let extension = file.extension.map(|e| extension(e, has)).transpose()?;
        let pickup = file
            .pickup
            .map(|table| pickup(table, &file.icons))
            .transpose()?;
        let t = file.module;
        for (name, path) in &file.icons {
            if !valid_key(name) {
                return Err(bad(
                    "icons",
                    format!("{name:?} is not a name: letters, digits, '-', '_'"),
                ));
            }
            if !path.to_ascii_lowercase().ends_with(".png") || !has(path) {
                return Err(bad(
                    "icons",
                    format!("{name}: {path:?} is not a .png in the mod"),
                ));
            }
        }

        for (name, path) in &file.effects {
            if !valid_key(name) {
                return Err(bad(
                    "effects",
                    format!("{name:?} is not a name: letters, digits, '-', '_'"),
                ));
            }
            if !path.to_ascii_lowercase().ends_with(".toml") || !has(path) {
                return Err(bad(
                    "effects",
                    format!("{name}: {path:?} is not a .toml in the mod"),
                ));
            }
        }
        if file.effects.len() > MAX_EFFECTS {
            return Err(bad(
                "effects",
                format!("{} effects, at most {MAX_EFFECTS}", file.effects.len()),
            ));
        }

        if !valid_id(&t.id) {
            return Err(bad(
                "id",
                format!(
                    "{:?} is not an id: lowercase letters, digits, '.', '-' and '_', starting with a letter",
                    t.id
                ),
            ));
        }
        if oag_ext_abi::channel_key(&t.id) == oag_ext_abi::TRACK_SENDER {
            return Err(bad(
                "id",
                format!(
                    "{:?} hashes to the track's sender key; rename the mod",
                    t.id
                ),
            ));
        }
        if t.name.trim().is_empty() {
            return Err(bad("name", "empty"));
        }
        let version = semver::Version::parse(&t.version).map_err(|e| {
            bad(
                "version",
                format!("{:?} is not a semver release: {e}", t.version),
            )
        })?;
        if t.authors.is_empty() || t.authors.iter().any(|a| a.trim().is_empty()) {
            return Err(bad("authors", "at least one non-empty author"));
        }
        if t.description.trim().is_empty() {
            return Err(bad("description", "empty"));
        }
        let license = match (t.license, t.license_file) {
            (Some(_), Some(_)) => {
                return Err(bad("license", "name `license` or `license-file`, not both"));
            }
            (Some(expression), None) => {
                spdx_shape(&expression).map_err(|p| bad("license", p))?;
                License::Spdx(expression)
            }
            (None, Some(path)) => {
                if !has(&path) {
                    return Err(bad("license-file", format!("{path:?} is not in the mod")));
                }
                License::File(path)
            }
            (None, None) => License::None,
        };
        let oag = t
            .oag
            .map(|req| {
                semver::VersionReq::parse(&req)
                    .map_err(|e| bad("oag", format!("{req:?} is not a version requirement: {e}")))
            })
            .transpose()?;
        let channels = channels(file.channels, &t.id)?;
        Ok(Self {
            id: t.id,
            name: t.name,
            version,
            authors: t.authors,
            license,
            description: t.description,
            repository: t.repository,
            homepage: t.homepage,
            keywords: t.keywords,
            titles: t.titles,
            oag,
            extension,
            icons: file.icons,
            effects: file.effects,
            pickup,
            channels,
        })
    }

    /// Whether this build of the game is one the mod says it works with.
    #[must_use]
    pub fn works_with(&self, game: &semver::Version) -> bool {
        self.oag.as_ref().is_none_or(|req| req.matches(game))
    }
}

/// The `[channels]` table, each name checked by `oag_ext_abi::check_channel`; a published
/// one must lie in the mod's own namespace, and none may repeat or hash like another.
fn channels(table: Option<ChannelsTable>, id: &str) -> Result<Channels, Error> {
    let Some(table) = table else {
        return Ok(Channels::default());
    };
    for (key, list) in [
        ("channels.publishes", &table.publishes),
        ("channels.listens", &table.listens),
    ] {
        if list.len() > MAX_CHANNELS {
            return Err(bad(
                key,
                format!("{} channels, at most {MAX_CHANNELS}", list.len()),
            ));
        }
        for (i, name) in list.iter().enumerate() {
            oag_ext_abi::check_channel(name).map_err(|why| {
                bad(
                    key,
                    format!("{name:?} is not a channel name ({why}); write `<mod id>/<name>`"),
                )
            })?;
            if list[..i].iter().any(|seen| seen == name) {
                return Err(bad(key, format!("{name:?} is listed twice")));
            }
            let hash = oag_ext_abi::channel_key(name);
            if let Some(other) = list[..i]
                .iter()
                .find(|seen| oag_ext_abi::channel_key(seen) == hash)
            {
                return Err(bad(
                    key,
                    format!("{name:?} and {other:?} hash alike; rename one"),
                ));
            }
        }
    }
    if let Some(name) = table
        .publishes
        .iter()
        .find(|name| oag_ext_abi::namespace(name) != id)
    {
        return Err(bad(
            "channels.publishes",
            format!(
                "{name:?} is not in this mod's namespace; a channel it publishes starts `{id}/`"
            ),
        ));
    }
    Ok(Channels {
        publishes: table.publishes,
        listens: table.listens,
    })
}

fn extension(e: ExtensionTable, has: &dyn Fn(&str) -> bool) -> Result<Extension, Error> {
    let bad_extension = |key: &'static str, problem: String| Error::Bad { key, problem };
    let rules = e.rules.map(|r| Rules { module: r.module });
    if let Some(rules) = &rules {
        if !e.simulation {
            return Err(bad_extension(
                "extension.simulation",
                format!(
                    "{} is a simulation module; say `simulation = true`",
                    rules.module
                ),
            ));
        }
        if !has(&rules.module) {
            return Err(bad_extension(
                "extension.rules.module",
                format!("{:?} is not in the mod", rules.module),
            ));
        }
    }
    let track = e.track.map(|t| Track {
        module: t.module,
        track: t.track,
    });
    if let Some(track) = &track {
        if !e.simulation {
            return Err(bad_extension(
                "extension.simulation",
                format!(
                    "{} is a simulation module; say `simulation = true`",
                    track.module
                ),
            ));
        }
        for (key, name) in [
            ("extension.track.module", &track.module),
            ("extension.track.track", &track.track),
        ] {
            if !has(name) {
                return Err(bad_extension(key, format!("{name:?} is not in the mod")));
            }
        }
    }
    Ok(Extension {
        simulation: e.simulation,
        api: e.api,
        memory_pages: e.memory_pages,
        state_bytes: e.state_bytes,
        rules,
        track,
    })
}

/// A string key or icon name: letters, digits, '-', '_'.
/// What `fire-mod-effect`'s index (15 bits of an argument) can name, kept well inside it.
const MAX_EFFECTS: usize = 256;

fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

pub(crate) fn valid_id(id: &str) -> bool {
    let mut chars = id.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '-' | '_'))
}

/// The shape of an SPDX licence expression: identifiers joined by `AND`, `OR`
/// and `WITH`, with parentheses. The identifier list is not checked.
fn spdx_shape(expression: &str) -> Result<(), String> {
    let spaced = expression.replace('(', " ( ").replace(')', " ) ");
    let mut depth = 0_i32;
    let mut want_operand = true;
    let mut seen = false;
    for token in spaced.split_whitespace() {
        match token {
            "(" if want_operand => depth += 1,
            ")" if !want_operand => {
                depth -= 1;
                if depth < 0 {
                    return Err(format!("{expression:?}: unbalanced ')'"));
                }
            }
            "AND" | "OR" | "WITH" if !want_operand => want_operand = true,
            id if want_operand
                && id
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'))
                && !matches!(id, "AND" | "OR" | "WITH" | "(" | ")") =>
            {
                want_operand = false;
                seen = true;
            }
            other => {
                return Err(format!(
                    "{expression:?}: unexpected {other:?} (an SPDX expression such as \"MIT OR Apache-2.0\")"
                ));
            }
        }
    }
    if !seen || want_operand || depth != 0 {
        return Err(format!("{expression:?} is not a complete SPDX expression"));
    }
    Ok(())
}
