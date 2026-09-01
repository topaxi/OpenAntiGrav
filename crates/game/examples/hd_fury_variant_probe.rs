//! Scratch probe, the reproducer for `oag_hd::race::TEAM_VARIANTS`: every one
//! of Wipeout HD's twelve teams ships three directories, not one -
//! `<team>`, `<team>_c1` (Fury's concept reskin) and `<team>_n1` (Fury's
//! nitro reskin) - none of which the plugin definition's flat `PI_Team` list
//! (`crate::catalogue::teams`) exposes as a separate pickable entry.
//! `detonator` is excluded: it is Detonator mode's own craft, not a variant
//! of any of the twelve.

fn main() -> anyhow::Result<()> {
    let opened =
        oag_game::title::open_source("data/images/hdfury-ps3-eu-dec.iso", Vec::new(), Vec::new())?;
    let mut dirs: Vec<String> = Vec::new();
    let containers = std::iter::once(&opened.archives.data)
        .chain(opened.archives.fe.iter())
        .chain(opened.archives.extra.iter());
    for container in containers {
        let oag_assets::Container::Psarc(archive) = container else {
            continue;
        };
        for path in archive.paths() {
            let lower = path.to_lowercase();
            let Some(at) = lower.find("/ships/") else {
                continue;
            };
            let rest = &path[at + "/ships/".len()..];
            // Only a real subdirectory, not a `.rcsmaterial`/`.txt` sitting
            // directly under `Ships/` at the same path depth.
            let Some((dir, _)) = rest.split_once(['\\', '/']) else {
                continue;
            };
            if !dirs.iter().any(|d| d == dir) {
                dirs.push(dir.to_string());
            }
        }
    }
    dirs.sort();
    println!("{} ship director(y/ies) under Ships/:", dirs.len());
    for dir in &dirs {
        println!("  {dir}");
    }
    Ok(())
}
