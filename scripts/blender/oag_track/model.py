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


@dataclass
class Track:
    id: str
    title: str
    point_version: int
    paths: list
    junctions: list
    start: Start
    nodes: list
    speedup_pads: list = field(default_factory=list)
    weapon_pads: list = field(default_factory=list)
    notes: list = field(default_factory=list)
