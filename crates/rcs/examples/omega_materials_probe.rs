//! Scratch probe for the Omega race lane: list the materials of a PS4
//! `.rcsmodel` with the textures each names, and how many submeshes use it.
//!
//! `cargo run -p oag-rcs --example omega_materials_probe -- <file.rcsmodel> [substring]`

use oag_rcs::rcsmodel::psp2;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("usage: omega_materials_probe <file> [substring]")?;
    let filter = args.next();
    let model = psp2::parse(&std::fs::read(&path)?)?;
    let mut uses = vec![0usize; model.materials.len()];
    let mut tris = vec![0usize; model.materials.len()];
    let mut strides = vec![std::collections::BTreeSet::new(); model.materials.len()];
    for s in &model.submeshes {
        if let Some(m) = s.material {
            uses[m] += 1;
            tris[m] += s.triangle_count();
            strides[m].insert(s.stride);
        }
    }
    for (i, m) in model.materials.iter().enumerate() {
        let line = format!(
            "{i:4} {:>4} use(s) {:>8} tri(s) strides {:?} {} -> {:?}",
            uses[i],
            tris[i],
            strides[i],
            m.name.rsplit('/').next().unwrap_or(&m.name),
            m.textures
                .iter()
                .map(|t| t.rsplit('/').next().unwrap_or(t))
                .collect::<Vec<_>>()
        );
        if filter.as_deref().is_none_or(|f| line.contains(f)) {
            println!("{line}");
        }
    }
    Ok(())
}
