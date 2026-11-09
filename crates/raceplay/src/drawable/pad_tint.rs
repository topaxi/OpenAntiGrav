//! The weapon pads' ready and cooldown colours, written into their own vertices. See
//! [`Drawable::tint_weapon_pads`].

use super::*;

impl Drawable {
    /// Recolours each weapon pad's own geometry by whether it currently hands
    /// out a pickup - see [`oag_render::weapon_pad`] for the recovered
    /// mechanism this reproduces. `ready[i]` pairs with this drawable's
    /// `i`-th [`mesh::Model::node_vertex_ranges`] entry - positional, not by
    /// name, for the reason that field's own doc comment gives.
    ///
    /// A no-op on a model with no weapon pads - every track and the sky -
    /// since `ready` is empty there.
    ///
    /// **Also a no-op on a Wipeout HD model**, and that gate is the point of
    /// [`mesh::Model::vertex_colour_is_light`] being read here rather than a
    /// title enum. Two independent reasons, either sufficient:
    ///
    /// 1. The keyframe table is *Pulse's*, read out of the PSP executable at
    ///    `0x08ac00c8` (see [`oag_render::weapon_pad`]). HD's own `Weapon Pad`
    ///    does cycle, but through its own table at `0x008c26a0` and into the
    ///    light bars' constant rather than the mesh - see
    ///    [`Self::glow_weapon_pads`]. `ds_weaponup_cs.gtf` is a grey plate with
    ///    a **red** cross on it, where Pulse's `weapon_under.tga` is neutral and
    ///    takes its colour entirely from `pad+0x6c`.
    /// 2. On an HD model `colour` is not a tint at all. It is the baked
    ///    per-vertex light the fragment program **adds** inside its authored
    ///    lighting sum - see `oag_mesh::mesh::rcs::emit`. Writing a palette
    ///    entry there is not "the wrong colour", it is a different quantity.
    ///
    /// What HD does instead is unrecovered - see `docs/rendering/pads.md`. An
    /// HD pad therefore draws its authored texture under the circuit's own
    /// light rig, unarmed and armed alike, which is a visible absence rather
    /// than an invented state.
    pub(crate) fn tint_weapon_pads(
        &self,
        queue: &dyn oag_gpu::writes::Writes,
        seconds: f32,
        ready: &[bool],
        tinted: &mut Vec<mesh::GpuVertex>,
    ) {
        if self.model.vertex_colour_is_light {
            return;
        }
        self.recolour_nodes(
            queue,
            ready.iter().map(|&is_ready| {
                if is_ready {
                    oag_render::weapon_pad::ready_colour(seconds)
                } else {
                    oag_render::weapon_pad::COOLDOWN_COLOUR
                }
            }),
            tinted,
        );
    }

    /// Rewrites each of this drawable's [`mesh::Model::node_vertex_ranges`]
    /// entries to its own flat `colour`, one per iterator item, positional
    /// and zipped the way [`Self::tint_weapon_pads`]' own doc comment
    /// describes.
    ///
    /// Rewrites the *replacement* colour, not a multiply against the pad's
    /// authored one - `pad+0x6c` overwrites the mesh's own GE colour command
    /// every tick in the original (see `oag_render::weapon_pad`), and this
    /// mirrors that shape for the speed pad's own, unrecovered override.
    /// Always from `self.model.vertices`, never from the last frame's
    /// buffer, for the same accumulation reason [`Self::deflect_airbrakes`]
    /// gives.
    ///
    /// `tinted` is scratch the caller owns and this refills - see
    /// `Scene::scratch`.
    fn recolour_nodes(
        &self,
        queue: &dyn oag_gpu::writes::Writes,
        colours: impl Iterator<Item = [f32; 3]>,
        tinted: &mut Vec<mesh::GpuVertex>,
    ) {
        let stride = std::mem::size_of::<mesh::GpuVertex>() as u64;
        // A Quake's bump rolling over a pad has to survive the pad's own
        // rewrite from the authored vertices - see `Ripple::displace`.
        let ripple = lock(&self.ripple);
        recolour_runs(
            &self.model.node_vertex_ranges,
            &self.model.vertices,
            colours,
            |index, vertex| {
                if let Some(ripple) = ripple.as_ref() {
                    ripple.displace(index, vertex);
                }
            },
            tinted,
            |first, vertices| {
                queue.write_buffer(
                    &self.vertices,
                    u64::from(first) * stride,
                    bytemuck::cast_slice(vertices),
                );
            },
        );
    }
}

/// Each of `ranges` of `base` in its own flat colour (one per `colours` item, alpha
/// kept, then `displace`d), handed to `write` as `(first vertex, vertices)` once per run
/// of ranges that follow each other without a gap.
///
/// A circuit's weapon pads are its `Weapon Pad` nodes built back to back
/// (`oag_mesh::mesh::build_class`), so the whole set is one run and one write, where a
/// write per pad cost a queue write and its staging nine times a view on Talon's
/// Junction. Every vertex gets exactly the bytes the per-pad write gave it; a vertex
/// between two ranges is never written, since it is not a pad's to recolour.
///
/// `tinted` is the caller's, refilled per run - see `Scene::scratch`.
fn recolour_runs(
    ranges: &[std::ops::Range<u32>],
    base: &[mesh::GpuVertex],
    colours: impl Iterator<Item = [f32; 3]>,
    displace: impl Fn(u32, &mut mesh::GpuVertex),
    tinted: &mut Vec<mesh::GpuVertex>,
    mut write: impl FnMut(u32, &[mesh::GpuVertex]),
) {
    tinted.clear();
    let mut first = None;
    let mut end = 0;
    for (range, colour) in ranges.iter().zip(colours) {
        let Some(pad) = base.get(range.start as usize..range.end as usize) else {
            continue;
        };
        if let Some(at) = first
            && range.start != end
        {
            write(at, tinted);
            tinted.clear();
            first = None;
        }
        first.get_or_insert(range.start);
        end = range.end;
        tinted.extend(pad.iter().zip(range.start..).map(|(v, index)| {
            let mut out = *v;
            out.colour = [colour[0], colour[1], colour[2], out.colour[3]];
            displace(index, &mut out);
            out
        }));
    }
    if let Some(at) = first {
        write(at, tinted);
    }
}

#[cfg(test)]
mod tests;
