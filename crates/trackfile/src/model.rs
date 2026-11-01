use crate::art::Art;
use crate::props::{Props, Trigger};
use oag_vex::collision::CollisionNode;
use oag_vex::pads::PadVolume;
use oag_vex::track::{AiTrack, StartPosition};

/// What format 1.x has no field for. A loader that reads a track file names
/// these in its report rather than borrowing them from the host title: absent
/// means absent (CLAUDE.md, "Never invent what the assets already author").
/// The art meshes, their textures, the sky, the sun, the ambient and the lens
/// flare are [`Track::art`] (ADR-0076), present or not per file, and so is
/// keyframed node motion (`Art::animations`); a texture's own animation is not.
pub const ABSENT: [&str; 8] = [
    "fog",
    "point lights",
    "clouds",
    "texture animation",
    "track particle emitters",
    "sound emitters",
    "visibility sets (PVS)",
    "the Zone variant",
];

/// A track: the plain data a race consumes from a circuit, whichever source it
/// came from.
///
/// Collision geometry is in world space already (a disc circuit's is, and the
/// race builds its colliders from it as it stands), so [`CollisionNode::node_index`]
/// has no meaning for a file and is written as the node's position in the list.
#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    /// A stable identifier: keys records and ghosts. Not empty.
    pub id: String,
    /// The author's title.
    pub title: String,
    /// The title whose disc the track's art borrows from (host textures, the
    /// sky), as `oag_source::layers::slug` spells it (`wipeout-pulse`).
    /// Required when the art borrows anything ([`crate::art::Art::borrows_from_a_disc`]);
    /// `None` for a track that borrows nothing, which runs on any title. A track
    /// that names one is pinned to that disc: raced on another title it runs as
    /// a Race Remix with the host's disc as the track source (ADR-0076,
    /// decision 13).
    pub host: Option<String>,
    /// The titles the author offers the track on, slugs as [`Self::host`];
    /// empty is every title. An optional filter, nothing more.
    pub titles: Vec<String>,
    /// The spline graph.
    pub ai: AiTrack,
    /// The start slot (the engine lays its own grid from it).
    pub start: StartPosition,
    /// Floor, wall, reset and magstrip collision.
    pub nodes: Vec<CollisionNode>,
    /// Speed pads.
    pub speedup_pads: Vec<PadVolume>,
    /// Weapon pads.
    pub weapon_pads: Vec<PadVolume>,
    /// The track's own properties, for a simulation mod (`crate::props`).
    pub props: Props,
    /// One property list per speed pad, or none at all.
    pub speedup_pad_props: Vec<Props>,
    /// One property list per weapon pad, or none at all.
    pub weapon_pad_props: Vec<Props>,
    /// One property list per spline path (`ai.paths`' order), or none at all. A mod
    /// finds a way round a fork by them (a pit lane's `pit`).
    pub path_props: Vec<Props>,
    /// Boxes a mod hears a craft enter and leave. The engine does nothing else with
    /// them.
    pub triggers: Vec<Trigger>,
    /// The channels the track publishes (`[channels] publishes`): a trigger's `oag_emit`
    /// names one of them. `<id>/<name>`, validated by `oag_ext_abi::check_channel`.
    pub channels: Vec<String>,
    /// The art: meshes, textures, sun, sky, flare. `None` races as flat-shaded
    /// collision. The simulation never reads it, so it is in no world hash.
    pub art: Option<Art>,
    /// The track's own loading screen (`[loading]`): what it supplies of it, the rest being
    /// the running title's. `None` is the title's screen as it is.
    pub loading: Option<Loading>,
    /// Free text the producer recorded: what it chose, ignored or could not map.
    /// Carried through a round trip so a loader report can print it.
    pub notes: Vec<String>,
}

/// What a track supplies of its own loading screen (`[loading]`, `docs/formats/screen-layout.md`).
///
/// **Names, not content.** `layout` and `feature_image` are file names relative to the manifest,
/// read by whoever draws the screen (as `[[art.texture]] image` is); the four strings are the
/// text itself. Every key is optional and a missing one falls back to the running title's.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Loading {
    /// A screen-layout file (`screen = "loading"`) that replaces the title's.
    pub layout: Option<String>,
    /// The circuit's name on the caption line; absent is `track.title`.
    pub caption: Option<String>,
    /// The picture the layout's feature image draws, a PNG.
    pub feature_image: Option<String>,
    /// The feature's name, over the picture.
    pub feature_title: Option<String>,
    /// The heading over the paragraph.
    pub feature_heading: Option<String>,
    /// The paragraph.
    pub feature_text: Option<String>,
}

impl Loading {
    /// Whether any of the three feature texts or the picture is given: the track then **is** the
    /// feature, and the title's random draw is not run.
    #[must_use]
    pub fn supplies_feature(&self) -> bool {
        self.feature_image.is_some()
            || self.feature_title.is_some()
            || self.feature_heading.is_some()
            || self.feature_text.is_some()
    }

    /// Refuses what a track may not name: an empty string, or a file outside its own folder (an
    /// absolute path, a `..`).
    ///
    /// # Errors
    ///
    /// The first thing wrong, named.
    pub fn validate(&self) -> Result<(), crate::Error> {
        for (key, file) in [
            ("layout", &self.layout),
            ("feature_image", &self.feature_image),
        ] {
            let Some(file) = file else { continue };
            let path = std::path::Path::new(file);
            let own = !file.is_empty()
                && path.is_relative()
                && path
                    .components()
                    .all(|part| matches!(part, std::path::Component::Normal(_)));
            if !own {
                return Err(crate::Error::Manifest(format!(
                    "loading.{key} {file:?} is not a file inside the track's own folder"
                )));
            }
        }
        for (key, text) in [
            ("caption", &self.caption),
            ("feature_title", &self.feature_title),
            ("feature_heading", &self.feature_heading),
            ("feature_text", &self.feature_text),
        ] {
            if text.as_deref().is_some_and(|t| t.trim().is_empty()) {
                return Err(crate::Error::Manifest(format!("loading.{key} is empty")));
            }
        }
        Ok(())
    }
}

/// A read track and what the read itself has to say.
#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    /// The track.
    pub track: Track,
    /// Manifest keys this reader does not know (added to the schema since), and glb nodes
    /// it did not read, each named.
    pub unread: Vec<String>,
}
