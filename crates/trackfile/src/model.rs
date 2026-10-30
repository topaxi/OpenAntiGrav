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
    /// Boxes a mod hears a craft enter and leave. The engine does nothing else with
    /// them.
    pub triggers: Vec<Trigger>,
    /// The art: meshes, textures, sun, sky, flare. `None` races as flat-shaded
    /// collision. The simulation never reads it, so it is in no world hash.
    pub art: Option<Art>,
    /// Free text the producer recorded: what it chose, ignored or could not map.
    /// Carried through a round trip so a loader report can print it.
    pub notes: Vec<String>,
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
