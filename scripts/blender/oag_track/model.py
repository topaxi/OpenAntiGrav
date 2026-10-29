"""The track model, mirroring `oag_trackfile::Track` field for field.

Every float held here is an exact `f32` value widened to a Python float, so
packing it back with `struct` is bit-exact. Nothing in this module imports
`bpy`, which is what lets the format code run (and be tested) without Blender.
"""

from dataclasses import dataclass, field

SHORT_POINT_VERSION = 0x107
HEADER_WORD = 0xFFFFFFFF

# The six surface tags `oag_trackfile` knows, in the glb's `oag_kind` extra.
COLLISION_KINDS = ("floor", "wall", "reset", "mag_floor", "cage", "track_wall")


@dataclass
class SplinePoint:
    pos: tuple
    tangent: tuple
    down: tuple
    lateral: tuple
    progress: float
    half_width_left: float
    half_width_right: float
    ai_bound_left: float
    ai_bound_right: float
    racing_line: float
    section_id: int
    flags: int
    light_scale: tuple
    # (lane band low, lane band high, speed floor) or None.
    ai_bytes: tuple = None
    # The collision cage's eight signed bytes, or None when the point authors none.
    cage: tuple = None


@dataclass
class Path:
    points: list
    max_spacing: float
    entry: int = None
    exit: int = None


@dataclass
class Junction:
    prev: tuple
    next: tuple


@dataclass
class Start:
    position: tuple
    left: tuple
    up: tuple
    forward: tuple


@dataclass
class Pad:
    to_world: tuple  # 16 floats, row-major, row-vector convention
    min: tuple
    max: tuple
    disabled: float = 0.0


@dataclass
class CollisionMesh:
    vertices: list
    triangles: list
    scalars: list


@dataclass
class CollisionNode:
    kind: str
    meshes: list
    name: str = None
    version: int = HEADER_WORD


# The titles a track may name as its host (`[track] host`) or offer itself on
# (`[track] titles`), spelled the way `oag_source::layers::slug` spells a title's
# name. The host is the disc the art borrows from (host textures, the sky);
# required exactly when it borrows, and on another title such a track runs as a
# Race Remix with the host as the track source (ADR-0076, decision 13).
HOSTS = ("wipeout-pulse", "wipeout-pure", "wipeout-hd", "wipeout-omega-collection", "wipeout-2048")


@dataclass
class Track:
    id: str
    title: str
    host: str
    titles: list
    point_version: int
    paths: list
    junctions: list
    start: Start
    nodes: list
    speedup_pads: list = field(default_factory=list)
    weapon_pads: list = field(default_factory=list)
    notes: list = field(default_factory=list)
    art: "Art" = None


# The art section (ADR-0076): what the renderer draws and the simulation never reads.

BLENDS = ("opaque", "cutout", "alpha", "additive", "unblended")
SHADOW_ROLES = ("both", "cast", "receive", "none")


@dataclass
class ArtMesh:
    """One draw: `oag/art/<n>`. Colours are linear RGB plus coverage alpha."""

    name: str
    positions: list
    normals: list
    texcoords: list  # texture v down, as the disc's
    colours: list
    indices: list
    texture: int = None
    blend: str = "opaque"
    culled: bool = True
    lit: bool = False
    shadow: str = "both"
    alpha_ref: int = None
    layer: int = 0
    # Rest pose ((tx, ty, tz), (qx, qy, qz, qw), (sx, sy, sz)) in the format's axes, only on a mesh an
    # animation moves; its vertices are then in the node's own space.
    transform: tuple = None
    pad: str = None  # "speedup" or "weapon": a pad plate, drawn with the host's pad models


PAD_PLATES = ("speedup", "weapon")
REPEATS = ("loop", "clamp")
PATHS = ("translation", "rotation", "scale")


@dataclass
class Channel:
    """One glTF animation channel. `target` is an art mesh index or "sun"; `values` are tuples."""

    target: object
    path: str
    times: list
    values: list
    interpolation: str = "LINEAR"


@dataclass
class Animation:
    name: str
    channels: list = field(default_factory=list)
    repeat: str = "loop"


@dataclass
class Sun:
    direction: tuple  # unit, towards the sun
    colour: tuple  # linear RGB
    intensity: float = 1.0
    angular_diameter: float = 0.0  # degrees
    shadows: bool = True


@dataclass
class DirectionalLight:
    direction: tuple
    colour: tuple
    intensity: float = 1.0


@dataclass
class LensFlare:
    atlas: int = None
    colour: tuple = None
    direction: tuple = None


@dataclass
class Texture:
    """Exactly one of `image` (a file beside the manifest) or `host` (entry, name)."""

    image: str = None
    host: tuple = None


@dataclass
class Skybox:
    """The track's own sky (ADR-0076 decision 14): a texture row per cube face
    (+X, -X, +Y zenith, -Y nadir, +Z, -Z; None draws the gradient) and/or the
    gradient from `horizon` to `zenith`, linear RGB."""

    faces: tuple = (None,) * 6
    zenith: tuple = None
    horizon: tuple = None


@dataclass
class Art:
    meshes: list = field(default_factory=list)
    textures: list = field(default_factory=list)
    sky: str = None
    skybox: Skybox = None
    sun: Sun = None
    ambient: tuple = None
    lights: list = field(default_factory=list)
    lens_flare: LensFlare = None
    animations: list = field(default_factory=list)
    sun_rotation: tuple = None  # the oag/sun node's rest rotation, exactly when the sun turns
    draw_collision: bool = False  # [art] draw_collision: the flat-shaded collision under the art

    def borrows_from_a_disc(self):
        """A host texture or a sky: the track must name its host."""
        return self.sky is not None or any(t.host is not None for t in self.textures)
