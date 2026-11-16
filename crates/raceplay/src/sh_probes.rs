//! The spherical-harmonic ambient an Omega hull is lit from.
//!
//! The circuit's `track.shprobes` ([`oag_vex::shprobes`]) blended at each craft's position, one
//! row of a small table per grid slot: nine rgb coefficients, then `(K, Ks, 1, 0)` with
//! `Ship diffuse brightness` and its `(shadow)` twin. The hull's pixel program evaluates
//! `(Ks + (K - Ks) * shadow) * max(0.01, SH(n))` from its row
//! (`mesh/shaders/shade.wesl`, `sh_hull_ambient`; `docs/ghidra/functions/ps4-omega-eu/hull-tone.md`).
//! **Not a render of our scene**: the original's coefficients are this file's too, so a circuit
//! with no file binds no table and leaves every hull on its constant ambient.
//!
//! The table is bound beside the shadow maps (`mesh_render::ShadowMaps::sh_probes`) and a hull
//! names its row through the same `env_layer` HD's environment probes use.

use std::cell::RefCell;

use oag_gpu::writes::Writes;
use oag_mesh::mesh_render::{SH_PROBE_BYTES, SH_PROBE_ROW, SH_PROBE_ROWS};
use oag_vex::shprobes::{ShProbes, Taps};

/// A circuit's probes and the two weights its `.envsettings` authors.
#[derive(Debug, Clone)]
pub struct HullSh {
    pub probes: ShProbes,
    /// `Lighting.Ship diffuse brightness`.
    pub k: f32,
    /// Its `(shadow)` twin, `k` where the file authors none.
    pub ks: f32,
}

const CRAFTS: usize = SH_PROBE_ROWS;
const TEXELS: usize = SH_PROBE_ROW;

impl HullSh {
    /// The table row of the craft at `position`: [`TEXELS`] rgba texels.
    #[must_use]
    pub fn row(&self, position: [f32; 3]) -> [f32; TEXELS * 4] {
        self.row_of(self.probes.locate(position))
    }

    fn row_of(&self, taps: Taps) -> [f32; TEXELS * 4] {
        let coefficients = self.probes.blend(taps);
        let mut row = [0.0; TEXELS * 4];
        for (texel, rgb) in row.chunks_exact_mut(4).zip(coefficients.chunks_exact(3)) {
            texel[..3].copy_from_slice(rgb);
        }
        row[(TEXELS - 1) * 4..].copy_from_slice(&[self.k, self.ks, 1.0, 0.0]);
        row
    }
}

/// The table on the GPU and what each craft's row was last written from.
#[derive(Debug)]
pub struct ShAmbient {
    hull: HullSh,
    buffer: wgpu::Buffer,
    /// The taps each row holds, so a craft that has not moved between probes writes nothing.
    written: RefCell<[Option<Taps>; CRAFTS]>,
}

impl ShAmbient {
    /// The table, every row unflagged until its craft writes it.
    #[must_use]
    pub fn new(device: &wgpu::Device, hull: HullSh) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("spherical-harmonic ambient"),
            size: SH_PROBE_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            hull,
            buffer,
            written: RefCell::new(Default::default()),
        }
    }

    /// The table, for `ShadowMaps::sh_probes`.
    #[must_use]
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Writes the row of grid slot `craft` for a craft at `position`.
    pub fn write(&self, queue: &dyn Writes, craft: usize, position: [f32; 3]) {
        if craft >= CRAFTS {
            return;
        }
        let taps = self.hull.probes.locate(position);
        if self.written.borrow()[craft] == Some(taps) {
            return;
        }
        let row = self.hull.row_of(taps);
        queue.write_buffer(
            &self.buffer,
            (craft * TEXELS * 16) as u64,
            bytemuck::cast_slice(&row),
        );
        self.written.borrow_mut()[craft] = Some(taps);
    }
}
