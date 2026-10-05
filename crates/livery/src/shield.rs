//! The shield pickup's two models: the per-team shell and the cockpit sphere.
//!
//! Split out of `livery` because the shield is the one model family that takes
//! a per-source branch at load on top of the PSP/PS3 geometry split: the PS2
//! disc's shield needs its external texture set, and its shell is a different
//! file from the PSP's. See [`shell`].

use super::*;

/// One `.vex` by name, from whichever of the two geometry layouts the source
/// uses, or a sentence saying why not.
///
/// **The two layouts are a per-title fact, not a per-model one.** A PSP `.vex`
/// carries its own geometry; a PS3 one is a header whose vertices live in the
/// `.rcsmodel` beside it, which is why [`one`] already branches this way for the
/// hull. Every shield model this file loads needs the same branch, so it is here
/// once rather than copied per caller.
///
/// `Ok` carries a description of what was built, for the loader report; `Err`
/// carries the line to report instead.
fn shield_model(
    archives: &mut oag_assets::Archives,
    name: &str,
) -> Result<(Model, String), String> {
    let blob = archives
        .read_name(name)
        .map_err(|_| format!("{name}: not in the archive set"))?;
    if !mesh::geometry_is_external(&blob) {
        let mut model = mesh::build_with_textures(name, &blob, None)
            .map_err(|error| format!("{name}: {} bytes, does not parse ({error})", blob.len()))?;
        let mut note = format!("{} triangle(s)", model.indices.len() / 3);
        // The PS2 signature [`plume`] takes, on the same gate: every `Texture`
        // node present and none decoded. Both PS2 shield models - the per-team
        // shell and `vr_shield_cockpit.vex` - declare one `Texture` node and
        // carry no pixels, so without this they bind the white 1x1 and the
        // shell's `_ADD` texture never reaches the picture. A PSP shield has its
        // texture embedded and never enters the branch.
        if !model.textures.is_empty() && model.textures.iter().all(Option::is_none) {
            let mut skin = Vec::new();
            ps2_skin(archives, name, &blob, &mut model, &mut skin);
            for line in skin {
                note.push_str("; ");
                note.push_str(line.strip_prefix(&format!("{name}: ")).unwrap_or(&line));
            }
        }
        return Ok((model, note));
    }
    let sibling = mesh::rcs::sibling_name(name);
    let geometry = sibling
        .as_deref()
        .and_then(|s| archives.read_name(s).ok())
        .ok_or_else(|| format!("{name}: a PS3 .vex with no .rcsmodel beside it"))?;
    let (model, built) = mesh::rcs::build(
        name,
        &blob,
        &geometry,
        &mut |path| archives.read_name(path).ok(),
        |c| c.mesh,
    )
    .map_err(|error| format!("{name}: its .rcsmodel does not build ({error})"))?;
    // **Said out loud because the picture does not say it.** A shield model
    // draws additively and `mesh::rcs` resolves no material - HD authors one per
    // team (`/data/materials/ships/<team>_shield.rcsmaterial`) and nothing here
    // reads it. So the geometry is the disc's and the *brightness* is not: an
    // untextured additive surface at full white washes the hull out, where the
    // PSP shell's own vertices carry a blue-violet at half alpha. Drawing it is
    // still right - it is real decoded geometry, and the substitute is for the
    // missing material alone - but a reviewer looking at an HD capture has to be
    // told which half is recovered.
    Ok((
        model,
        format!(
            "{}, and no material - HD's own .rcsmaterial is not read, so it is \
             brighter and flatter than the disc's",
            built.describe()
        ),
    ))
}

/// The shell a fired Shield pickup raises, from the first of
/// [`entry::shield_entry_names`] the source actually carries.
///
/// **Absence is reported rather than fatal**, exactly as [`plume`]'s is: a
/// source without either model is a source without a shield visual, which is a
/// missing feature and not a broken load. The report names which of the two
/// names answered, because they mean different things - the per-team one is what
/// `ShipShield_Construct` (`0x0885db38`) assembles, the shared one is this
/// project's reading of what a Pure source must mean by the same effect.
///
/// **No texture-transform track and no vertex rework**, unlike the plume: the
/// model is one mesh with one `_ADD` texture, and the animation is entirely in
/// the transform and the vertex colour that
/// `oag_render::shield::ShipShield` computes. Anything done to the mesh here
/// would be a second, invisible place for the look to come from. The PS2
/// branch in [`shield_model`] touches only which texture a draw binds, never a
/// vertex, and the PS2 shell is `extrashield.vex`, not the PSP's `shipshield.vex`
/// (see [`entry::shield_entry_names`]).
pub(super) fn shell(
    archives: &mut oag_assets::Archives,
    team: &str,
    ship_dir: &str,
    team_model: Option<&str>,
    report: &mut Vec<String>,
) -> Option<Model> {
    let names =
        crate::entry::shield_entry_names(ship_dir, team, archives.layout.platform, team_model);
    for (index, name) in names.iter().enumerate() {
        let provenance = if index == 0 {
            "the team's own"
        } else {
            "shared - this source carries no per-team shell"
        };
        match shield_model(archives, name) {
            Ok((model, note)) => {
                report.push(format!("{name}: {note} - the shield shell, {provenance}"));
                return Some(model);
            }
            Err(line) => report.push(line),
        }
    }
    report.push(format!(
        "{}: no shield shell for this team",
        names.join(" nor ")
    ));
    None
}

/// The sphere the original draws **instead of** the shell when the camera is
/// inside the craft, `Data\Weapons\vr_shield_cockpit.vex`.
///
/// Not per team and not per track - one entry serves every craft, which is why
/// it loads once beside the rocket's model rather than per [`Livery`]. On the
/// disc for all three titles: Pulse and Pure carry it whole, HD as the usual
/// `.vex`/`.rcsmodel` pair.
///
/// **`ShipShield_Update` chooses between the two on `craft+0x6d`**, the same
/// byte `oag_display::display::CameraView::draws_own_ship` reads - see that function,
/// whose confidence-70 note this is a second consumer for.
pub fn cockpit_shield(
    archives: &mut oag_assets::Archives,
    report: &mut Vec<String>,
) -> Option<Model> {
    let name = oag_pulse::race::COCKPIT_SHIELD;
    match shield_model(archives, name) {
        Ok((model, note)) => {
            report.push(format!(
                "{name}: {note} - the shield seen from inside the cockpit"
            ));
            Some(model)
        }
        Err(line) => {
            report.push(format!("{line} - no cockpit shield"));
            None
        }
    }
}
