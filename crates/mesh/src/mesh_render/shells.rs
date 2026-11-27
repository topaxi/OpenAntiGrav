//! The pipeline constants of the models that wrap a craft: HD's absorb shell and shield
//! shells, and our own shield.

use crate::mesh::Model;

/// The overrides `model`'s pipelines are built with, beyond every model's.
pub(super) fn constants(model: &Model) -> impl Iterator<Item = (&'static str, f64)> {
    [
        // HD's absorb shell program, whose two numbers are literals in its own microcode
        // rather than material parameters (`oag_fx::absorb_shell`).
        (model.absorb_shell, "absorb_shading"),
        (model.shield_shell, "shield_shading"),
        (model.shield_lit, "shield_lit"),
        (model.own_shield, "own_shield_shading"),
    ]
    .into_iter()
    .filter_map(|(on, name)| on.then_some((name, 1.0)))
}
