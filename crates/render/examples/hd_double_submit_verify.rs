//! Scratch probe: build the real model through `mesh::rcs::scene_from` (the
//! actual production path, not a reimplementation) and count how many draw
//! calls land near a known world-baked chunk's own bias - the direct check
//! that the node-loop skip in `build` actually stops the double submission
//! `hd_double_submit_check.rs` measured as a risk against the old code.

use oag_render::mesh;

const TARGETS: &[(&str, u32, [f32; 3])] = &[
    // sol_2 cf_startbeam_glow, read earlier in this investigation.
    (
        "data/environments/12_sol_2/track.vex",
        0x5c066070,
        [287.125, -5.226_562_5, 397.265_63],
    ),
];

fn main() -> anyhow::Result<()> {
    let spec = "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA02.PSARC";
    for &(name, hash, bias) in TARGETS {
        let data = mesh::read_blob(spec, name)?;
        let (model, report) = mesh::rcs::scene_from(spec, name, &data)?.unwrap();
        println!("{name}: {}", report.describe());
        let mut found = 0;
        for d in model.transparent_draws.iter().chain(model.draws.iter()) {
            let c = d.bounds.centre;
            let dist =
                ((c[0] - bias[0]).powi(2) + (c[1] - bias[1]).powi(2) + (c[2] - bias[2]).powi(2))
                    .sqrt();
            if dist < 0.1 {
                found += 1;
                println!(
                    "  draw at dist {dist:.3} of target hash {hash:#010x}: centre {c:?} radius {}",
                    d.bounds.radius
                );
            }
        }
        println!("  {found} draw(s) within 1 unit of the target - expect exactly 1");
        assert_eq!(found, 1, "double submission for {hash:#010x}!");
    }
    println!("no double submission found");
    Ok(())
}
