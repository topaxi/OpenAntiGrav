//! Scratch probe: every vertex attribute declared by every `.rcsmodel` on the
//! disc, so a selector written against one circuit can be checked against all.
use oag_formats::rcsmodel;
fn main() -> anyhow::Result<()> {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "models".into());
    let mut census: std::collections::BTreeMap<(u32, u8, u8), usize> = Default::default();
    let mut files = 0usize;
    let mut stack = vec![std::path::PathBuf::from(dir)];
    while let Some(p) = stack.pop() {
        for e in std::fs::read_dir(&p)? {
            let e = e?;
            let path = e.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|s| s.to_str()) != Some("rcsmodel") {
                continue;
            }
            files += 1;
            let blob = std::fs::read(&path)?;
            let Ok(model) = rcsmodel::Model::parse(&blob) else {
                continue;
            };
            for chunk in &model.meshes {
                let Some(decl) = chunk.decl.as_ref() else {
                    continue;
                };
                for a in &decl.attributes {
                    *census
                        .entry((a.name_hash, a.components, a.rsx_type))
                        .or_default() += 1;
                }
            }
        }
    }
    println!("{files} model(s)");
    println!("\nfour normalised bytes (the selector's candidates):");
    for ((h, c, t), n) in &census {
        if *c == 4 && *t == 4 {
            println!(
                "  {h:#010x} {:14} {n:6} chunk(s)",
                rcsmodel::vertex_decl::Attribute {
                    name_hash: *h,
                    components: *c,
                    rsx_type: *t,
                    offset: 0
                }
                .name()
                .unwrap_or("?")
            );
        }
    }
    Ok(())
}
