//! Wipeout HD's engine flare: the per-team model, and the two groups its own
//! node tree splits it into.
//!
//! **HD's flare is a model, not a sprite.** Pulse and Pure name one texture -
//! `grabbedEngineFlare128x64x8.mip` - and draw six camera-facing vertices with
//! it. HD carries no such name, which is why every HD load report used to say
//! "the flare falls back to a procedural glow". What it carries instead is
//! `Data\Ships\<Team>\engineflare.vex` and the `.rcsmodel` beside it, on all
//! fourteen craft: 961 to 2,141 triangles of authored flame geometry, every
//! one of them through the same shared material,
//! `data/materials/ships/engines/flame_test.rcsmaterial`, whose factor pair is
//! `SrcAlpha`/`One` - additive, the same equation the ribbon's material names.
//!
//! **And the boost plume is a subtree of it.** A sweep for a `shipboost.vex`
//! equivalent on the PS3 disc finds nothing, which read as "HD has no boost
//! plume" until the node tree was walked: `root` carries exactly two
//! `Transform` children, `EF_Main` and `EF_Boost`, five shapes each. The plume
//! is not a file there, it is a group. See
//! [`oag_title::flare::Authored`] for the tree and
//! `docs/rendering/trail-ribbon.md` for the evidence.

use super::*;

/// What one craft's `engineflare` pair yields: the always-on flame and the
/// boost group, each already at the nozzle.
#[derive(Debug, Default)]
pub(super) struct Flare {
    /// `EF_Main` - drawn for as long as the craft is in the race.
    pub always: Option<Model>,
    /// `EF_Boost` - this engine's boost gate is what reveals it; see
    /// [`per_team`]'s report line.
    pub boost: Option<Model>,
}

/// Loads a team's authored flare and splits it into its two groups.
///
/// **Placed by baking the nozzle translation into the vertices**, rather than
/// by a draw-time offset. The model is authored about its own origin - every
/// node transform in the file is the identity, so nothing in the file places
/// it, and the craft's `Engine Flare` locator is what says where on the hull it
/// hangs. Baking keeps the draw path identical to the plume's, which already
/// takes the craft's own model matrix and nothing else.
///
/// **No locator, nothing drawn**, on the same rule the flare quad follows: a
/// flare at the hull's origin sits in the middle of the fuselage, and a
/// plausible wrong picture is worse than a reported absence.
pub(super) fn per_team(
    archives: &mut oag_assets::Archives,
    team: &str,
    authored: &oag_title::flare::Authored,
    nozzle: Option<Vec3>,
    report: &mut Vec<String>,
) -> Flare {
    let name = ship_entry_name_for(team, authored.stem);
    let Ok(blob) = archives.read_name(&name) else {
        report.push(format!(
            "{name}: not in the archive set - no engine flare model for this team"
        ));
        return Flare::default();
    };
    let Some(sibling) = mesh::rcs::sibling_name(&name) else {
        report.push(format!(
            "{name}: names no .rcsmodel beside it - no engine flare model for this team"
        ));
        return Flare::default();
    };
    let Ok(geometry) = archives.read_name(&sibling) else {
        report.push(format!(
            "{sibling}: named beside {name} but not in the archive set - no engine \
             flare model for this team"
        ));
        return Flare::default();
    };
    let built = mesh::rcs::build(
        &name,
        &blob,
        &geometry,
        &mut |path| archives.read_name(path).ok(),
        |c| c.mesh,
    );
    let (mut model, built) = match built {
        Ok(pair) => pair,
        Err(error) => {
            report.push(format!(
                "{name}: does not build ({error}) - no engine flare model for this team"
            ));
            return Flare::default();
        }
    };
    report.push(format!("{name}: {}", built.describe()));
    let Some(nozzle) = nozzle else {
        report.push(format!(
            "{name}: this craft authors no Engine Flare locator, so the flare model \
             has nowhere to hang and is not drawn"
        ));
        return Flare::default();
    };
    alpha_ramp(&mut model);
    shading(&mut model, &geometry, report);
    translate(&mut model, nozzle);
    let groups = [authored.always, authored.boost];
    let parts = match mesh::groups::split(&model, &blob, &groups) {
        Ok(parts) => parts,
        Err(error) => {
            report.push(format!(
                "{name}: its node tree will not walk ({error}) - no engine flare model \
                 for this team"
            ));
            return Flare::default();
        }
    };
    let mut out = Flare::default();
    for (index, part) in parts.into_iter().enumerate() {
        let triangles = part.triangles();
        let what = if index == 0 { "always" } else { "boost" };
        if triangles == 0 {
            report.push(format!(
                "{name}: authors no geometry under {} - the {what} half of this \
                 craft's flare draws nothing",
                part.group
            ));
            continue;
        }
        report.push(format!(
            "{name}: {} - {triangles} triangle(s), {}",
            part.group,
            if index == 0 {
                "drawn additively at the Engine Flare locator for as long as the craft \
                 is racing"
                    .to_string()
            } else {
                // **The mechanism is read; what starts it is not.**
                // `EngineFlare_Update` (`0x002a3100`) counts a timer down at
                // `+0x12c`, and `EngineFlare_PlaceShapes` snaps a blend to 1.0
                // while that timer is above a threshold and decays it
                // exponentially otherwise - the same three parts as the
                // *Pulse* gate this engine already runs on (`boost_timer`,
                // `BOOST_GATE`, `BOOST_DECAY`). So reusing `plume_visible` is
                // a matched mechanism rather than a guess off a node name.
                // What writes the timer is outside the flare's own vtable and
                // is unread, so how long HD's boost lasts is still ours.
                // Confidence 88 - see
                // docs/ghidra/functions/ps3-hdfury-eu/engine-flare.md.
                "revealed on this engine's boost gate (Exhaust::plume_visible, \
                 Pulse's recovered one). HD's own gate is the same shape - a \
                 countdown, a snap, an exponential decay - but what starts its \
                 timer is unread, so the duration is this project's"
                    .to_string()
            }
        ));
        if index == 0 {
            out.always = Some(part.model);
        } else {
            out.boost = Some(part.model);
        }
    }
    out
}

/// Reads the flame's own shader parameters off the material and hangs them on
/// the model.
///
/// **The material record carries the values, not the `.rcsmaterial`.** The
/// shader file declares that the program takes a `power1` and a `scale1`; what
/// they *are* is authored per model, in the table
/// [`oag_formats::rcsmodel::material::parameters`] reads. Every craft on the
/// disc ships the same six, and they are read per craft anyway - a shared value
/// that is read is a fact about the data, and a shared value that is assumed is
/// a constant in disguise.
///
/// **A material short one of them leaves the model shaded the ordinary way**,
/// and the report says so: see [`mesh::Flame::from_material`] for why the set
/// is all-or-nothing.
fn shading(model: &mut Model, geometry: &[u8], report: &mut Vec<String>) {
    let Ok(parsed) = oag_formats::rcsmodel::Model::parse(geometry) else {
        return;
    };
    let Some(material) = parsed.materials.first() else {
        return;
    };
    match mesh::Flame::from_material(material) {
        Some(flame) => {
            report.push(format!("{}: {}", material.name, flame.describe()));
            model.flame = Some(flame);
        }
        None => report.push(format!(
            "{}: declares {} parameter(s) and not the set the flame program reads, so \
             this flare is shaded the ordinary way - see oag_render::mesh::Flame",
            material.name,
            material.parameters.len()
        )),
    }
}

/// Moves the colour set's fourth byte from the sun-occlusion slot into the
/// vertex **alpha**, which is where this material's own program reads it.
///
/// **Read out of the microcode rather than guessed.** `flame_test`'s vertex
/// program declares four attributes, and the hashes name three of them
/// outright - `position`, `normal`, `Uv1`, and slot 3 is `VertexColour1`
/// (`0x7493d450` = `~crc32("VertexColour1")`, the preimage technique
/// `docs/ghidra/functions/ps3-hdfury-eu/renderer.md` used for `fogColour`).
/// It writes `o[TC2] = (world position .xyz, VertexColour1.w)`, and the
/// fragment program's last two instructions are
///
/// ```text
/// @0x16  MOV H0.w, f[TC2]        <- the vertex colour's own alpha
/// @0x22  MUL H0.w, H1, H0 END    <- times the rim term, and that is the output
/// ```
///
/// so the fourth byte **is** the flame's opacity ramp: 1.0 at the nozzle, 0.0
/// at the tip on every craft measured. `mesh/rcs.rs` puts it in
/// [`GpuVertex::sun_mask`] instead, which is the right reading for the circuit
/// materials it was measured on (`renderer.md`, "The sun is real and it is
/// masked") and the wrong one here - it gates a sun term this program does not
/// have, and leaves `mesh.wgsl`'s `texel.a * in.colour.a` multiplying by a
/// constant 1.0. Left there the flame draws at the texture's own alpha
/// everywhere and has no shape at all.
///
/// **Model-scoped, and only this model's.** Nothing else is remapped: this is
/// one material whose program was read, not a change to what the colour set
/// means in general.
fn alpha_ramp(model: &mut Model) {
    for vertex in &mut model.vertices {
        vertex.colour[3] = vertex.sun_mask;
        // Unmasked, which is `sun_mask`'s own default for a chunk with no
        // colour set - the value it would carry if the sun term were the only
        // reader. See `mesh.wgsl`.
        vertex.sun_mask = 1.0;
    }
}

/// Moves every vertex by `offset`, and the bounds that are derived from them.
///
/// Both are needed: a draw call's `bounds` is what a frustum check culls by, so
/// leaving it behind would cull the flare against the hull's origin instead of
/// against the nozzle.
fn translate(model: &mut Model, offset: Vec3) {
    for vertex in &mut model.vertices {
        vertex.position = (Vec3::from_array(vertex.position) + offset).to_array();
    }
    for call in model
        .draws
        .iter_mut()
        .chain(&mut model.alpha_tested_draws)
        .chain(&mut model.transparent_draws)
    {
        call.bounds.centre = (Vec3::from_array(call.bounds.centre) + offset).to_array();
    }
    model.centre = (Vec3::from_array(model.centre) + offset).to_array();
}

/// `Data\Ships\<Team>\<stem>.vex`, the way every other per-team model is
/// composed - see [`crate::race::ship_entry_name`].
fn ship_entry_name_for(team: &str, stem: &str) -> String {
    oag_pulse::race::ships::entry_name(team, stem)
}
