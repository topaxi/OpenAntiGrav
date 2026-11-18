//! Every number the mapping chose. **Chosen, not measured**, none of them: a
//! foreign track's units are its own, and the host's physics is tuned to the
//! host's. A wrong choice shows as a ship flying too high or low, or a track too
//! long, and is retuned here.

/// Uniform scale applied after the track's own placement (the content's
/// per-track `[ Track Configuration ]` scale is read, not chosen).
///
/// Chosen so Lion's Den's floor, 4.6 wide after its own 1.15, comes to 40.0 -
/// the median full width of the Pulse circuits `01`, `10` and `16`. Width and
/// hull size drive collision and feel; the lap comes out longer than a Pulse lap
/// (Lion's Den's loop 723.8 units x 8.7 = 6,297 against Pulse's 4,176 to 5,093).
pub const WORLD_SCALE: f32 = 8.7;

/// The AI corridor as a fraction of the floor half-width, each side of the
/// centre line.
///
/// Chosen from Pulse's `16_Track`, where the corridor is 56 to 61 percent of the
/// floor half-width. The foreign content authors no corridor.
pub const AI_CORRIDOR_FRACTION: f32 = 0.6;

/// Least distance the corridor straddles the racing line by, as Pulse's loader
/// clamps it.
pub const AI_CORRIDOR_MARGIN: f32 = 0.1;

/// How far below the box floor and above the box ceiling a pad's trigger volume
/// reaches, in world units: Pulse's `Pad_Bind` literals (`min.y -= 2`,
/// `max.y += 8`), taken at Pulse's scale because [`WORLD_SCALE`] is chosen to
/// match it.
pub const PAD_BELOW: f32 = 2.0;
/// See [`PAD_BELOW`].
pub const PAD_ABOVE: f32 = 8.0;

/// A gap between consecutive lap sections, as a multiple of the median spacing,
/// past which it is reported as a jump in the track.
pub const JUMP_FACTOR: f32 = 3.0;
