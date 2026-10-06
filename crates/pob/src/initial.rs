//! The particles an emitter starts with: `+0x9a4` and the list at `+0x9a8`.
//!
//! **Read 2026-09-30 off Pulse's PSP `BOOT.BIN` and confirmed against a live
//! struck craft.** `FUN_088f58a4`, the emitter instance's initialiser
//! (`ParticleSystem_PrepareInstance` in effect), tests `res+0x9a4` for
//! non-zero, then walks the list at `res+0x9a8`, following each record's
//! `+0x8cc` to the next until it is zero. Every record gets one 0x80-byte
//! particle from `ParticleSystem_InitParticleFields` (`0x088f79b4`) the moment
//! the instance exists, before its first update. The count is only a gate;
//! the list ends on the null link.
//!
//! `WO_SHIP_COLL_SPARK_DAMAGE.POB` carries two, and they are what the struck
//! hull's warm envelope is made of: a white `shazam` flash (half-size `3.9`, six
//! ticks) hung on the `WO_SHIP_COLL_SPARK` emitter and an orange `glow`
//! (half-size `0.3..2.0`, 40 ticks, its size loop repeating every ten) hung on
//! the `WO_SHIP_COLL_SPARK_TRAIL`. Neither is reachable through the emitter tree
//! ([`super::ParticleSystem::emitters`] never saw them), so until now nothing
//! drew them.
//!
//! # The record
//!
//! Not an emitter: a sprite template of `0x900` bytes. Confidence **85** for
//! every field named here - each is matched against the live particle's size,
//! colour and lifetime, frame by frame:
//!
//! | offset | field |
//! | --- | --- |
//! | `+0x00` | name, NUL-terminated |
//! | `+0x10` | **size** channel (the emitter channel-block format) |
//! | `+0x1d0` | **alpha** channel, `0..=255` |
//! | `+0x390` | roll channel |
//! | `+0x470` | 256-entry colour table, walked over the particle's life |
//! | `+0x874` | render mode index (`2`: draw class 3, a rotating billboard) |
//! | `+0x878` | blend class (`2`: additive) |
//! | `+0x880` | atlas grid, columns then rows in one word |
//! | `+0x884` | flags |
//! | `+0x888`, `+0x88c` | lifetime centre and spread, ticks |
//! | `+0x8cc` | next record, or 0 |
//!
//! `+0xf0` is the sprite's **stretch** (its draw aspect) and `+0x2b0` the
//! atlas-frame rate; `+0xf0` is read here as [`Emitter::stretch`], `+0x2b0` is
//! constant `1.0` in every record and unused on a one-cell grid. All of
//! them draw as class 3, a rotated quad - see `pob.md`. The sprite is the
//! record's **own** (`+0x890`, see [`super::texture::TEMPLATE_TEXTURE_OFFSET`]),
//! which on the collision sparks happens to be the pool their parent shares.
//!
//! **Live confirmation.** A struck craft at severity `2.4` drew `shazam` at
//! half-sizes `9.36, 9.36, 8.73, 6.54, 4.33` (`3.9 * 2.4` held to `0.287` of
//! six ticks, then to zero) and `glow` at `0.748, 3.184, 4.8, 4.8, 1.606 ...`
//! (`2.4 * (0.3 + 1.7 v)` for the size channel's `v` at `fract(age / 10)`), in
//! colours `fff5f5ff` walking to `ffe3f2ff` over its life.

use oag_formats::ByteOrder;

use super::{
    Channel, ChannelMode, Emitter, MAX_CHANNEL_KEYS, MAX_EMITTERS, NAME_LEN, parse_channel,
};

/// Bytes of a template record that must be present.
const RECORD_LEN: usize = 0x8d0;

/// The list `emitter`'s `+0x9a4`/`+0x9a8` describe, as [`Emitter`]s that carry
/// only the fields a sprite template has.
///
/// `emitter_record` is the emitter's own record. **A record that does not
/// validate ends the list and returns what was read**, empty for a file whose
/// bytes at `+0x9a8` are not this format (the other executables' files are not
/// read here) - never an error, since it is an addition to what the emitter
/// already parsed.
pub(super) fn parse_list(
    data: &[u8],
    order: ByteOrder,
    base: usize,
    emitter_record: &[u8],
) -> Vec<Emitter> {
    let mut out = Vec::new();
    if order.u32(emitter_record, 0x9a4) == 0 {
        return out;
    }
    let mut next = order.u32(emitter_record, 0x9a8) as usize;
    let mut seen = Vec::new();
    while next != 0 && out.len() < MAX_EMITTERS && !seen.contains(&next) {
        seen.push(next);
        let start = base + next;
        let Some(record) = data.get(start..).filter(|r| r.len() >= RECORD_LEN) else {
            break;
        };
        let Some(parsed) = parse_one(record, order, next) else {
            break;
        };
        out.push(parsed);
        next = order.u32(record, 0x8cc) as usize;
    }
    out
}

fn parse_one(record: &[u8], order: ByteOrder, offset: usize) -> Option<Emitter> {
    let nul = record[..NAME_LEN].iter().position(|&byte| byte == 0)?;
    let name = String::from_utf8_lossy(&record[..nul]).into_owned();
    // The blocks' key counts bound the walk; a count past the block is not
    // this format.
    let blocks = [0x10, 0x1d0, 0x390];
    for block in blocks {
        let count = order.u32(record, block + 0x08) as usize;
        if count == 0 || count > MAX_CHANNEL_KEYS {
            return None;
        }
    }
    let size = parse_channel(record, order, 0x10).ok()?;
    let alpha = parse_channel(record, order, 0x1d0).ok()?;
    let rotation_speed = parse_channel(record, order, 0x390).ok()?;
    let stretch = parse_channel(record, order, 0xf0).ok();
    let mut colours = Box::new([[0u8; 4]; 256]);
    for (entry, bytes) in colours.iter_mut().zip(record[0x470..].as_chunks::<4>().0) {
        *entry = *bytes;
        if order == ByteOrder::Big {
            entry.reverse();
        }
    }
    let constant = || Channel {
        period: 0.0,
        mode: ChannelMode::Constant,
        lo: 0.0,
        hi: 1.0,
        keys: Vec::new(),
    };
    let grid = order.u32(record, 0x880);
    Some(Emitter {
        name,
        offset,
        flags: order.u32(record, 0x884),
        // One emission at the first update, then nothing: the particle is
        // created with the instance.
        duration_ticks: 1.0,
        shape: 0,
        extent: 0.0,
        extent_unread: [0.0; 2],
        radius_mode: 0,
        velocity_mode: 0,
        speed_per_tick: (0.0, 0.0),
        elevation: 0.0,
        azimuth: 0.0,
        cone_degrees: 0.0,
        lifetime_ticks: (
            order.u32(record, 0x888) as i32,
            order.u32(record, 0x88c) as i32,
        ),
        interval_ticks: (1, 1),
        per_emission: (1, 1),
        gravity_per_tick2: 0.0,
        live_cap: 0,
        render_mode: order.u32(record, 0x874),
        colour_mode: 0,
        blend_class: order.u32(record, 0x878),
        colours,
        size,
        alpha,
        rotation_speed,
        stretch,
        aspect: 1.0,
        distort_strength: 0.0,
        frame_rate: constant(),
        emission_scale: constant(),
        playback_rate: 1.0,
        child_velocity_inherit: 0.0,
        child_spawn_probability: 0.0,
        animated_attributes: 0,
        attribute_animations: Vec::new(),
        atlas_grid: ((grid >> 16) as u16, (grid & 0xffff) as u16),
        atlas_frames: 0,
        modifiers: Vec::new(),
        death_child: None,
        particle_child: None,
        initial_particles: Vec::new(),
    })
}

impl super::ParticleSystem<'_> {
    /// `template`'s own embedded sprite - see [`super::texture::TEMPLATE_TEXTURE_OFFSET`].
    ///
    /// `template` must be an entry of an [`Emitter::initial_particles`] list
    /// read from the same `data`. **Not the parent emitter's sprite**: the
    /// explosion's `Glow` hangs on `SHIP_DEBRIS` (a 128x64 grey debris atlas)
    /// and binds its own 32x32 radial glow; the collision sparks' templates
    /// carry the same pool as their parent, which is why that was read as a
    /// rule. `None` when the record carries no header-shaped block.
    #[must_use]
    pub fn template_texture<'d>(
        &self,
        data: &'d [u8],
        template: &Emitter,
    ) -> Option<super::texture::EmbeddedTexture<'d>> {
        super::texture::parse_template(data, self.order, self.resource_base(), template.offset)
    }
}
