//! Which circuits are drawn through the depth prepass: the two that shade
//! several fragments a pixel (measured 2026-10-18: Omega 3.8, HD about 2),
//! and neither Pulse nor Pure, whose CPU-bound frames the second pass of draws
//! would only slow at 1080p.

use crate::scene::circuit_prepass;

#[test]
fn hd_and_omega_circuits_take_the_prepass_and_the_psp_titles_do_not() {
    assert!(circuit_prepass(true, false), "HD's bloom chain");
    assert!(circuit_prepass(false, true), "Omega's tonemap chain");
    assert!(!circuit_prepass(false, false), "Pulse, Pure");
}
