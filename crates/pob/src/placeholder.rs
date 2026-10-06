//! The slot an emitter holds in the output before its children are walked.

use super::{Channel, ChannelMode, Emitter};

/// An emitter's slot in the output before its children have been walked.
/// Never observable: every placeholder is overwritten by the record it
/// stands in for.
pub(super) fn emitter_placeholder() -> Emitter {
    let channel = || Channel {
        period: 0.0,
        mode: ChannelMode::Constant,
        lo: 0.0,
        hi: 0.0,
        keys: Vec::new(),
    };
    Emitter {
        name: String::new(),
        offset: 0,
        flags: 0,
        duration_ticks: 0.0,
        shape: 0,
        extent: 0.0,
        extent_unread: [0.0; 2],
        radius_mode: 0,
        velocity_mode: 0,
        speed_per_tick: (0.0, 0.0),
        elevation: 0.0,
        azimuth: 0.0,
        cone_degrees: 0.0,
        lifetime_ticks: (0, 0),
        interval_ticks: (0, 0),
        per_emission: (0, 0),
        gravity_per_tick2: 0.0,
        live_cap: 0,
        render_mode: 0,
        colour_mode: 0,
        blend_class: 0,
        colours: Box::new([[0; 4]; 256]),
        size: channel(),
        alpha: channel(),
        rotation_speed: channel(),
        stretch: None,
        aspect: 1.0,
        distort_strength: 0.0,
        frame_rate: channel(),
        emission_scale: channel(),
        playback_rate: 0.0,
        child_velocity_inherit: 0.0,
        child_spawn_probability: 0.0,
        animated_attributes: 0,
        attribute_animations: Vec::new(),
        atlas_grid: (0, 0),
        atlas_frames: 0,
        modifiers: Vec::new(),
        death_child: None,
        particle_child: None,
        initial_particles: Vec::new(),
    }
}
