//! Scratch probe: how much of every `.vex` on a disc the node walk reaches.

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let source = args.next().unwrap_or_else(|| "hd".into());
    assert_eq!(source, "hd", "only the PS3 disc is swept here");
    let min: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(64);
    let mut blobs: Vec<(String, Vec<u8>)> = Vec::new();
    if source == "hd" {
        for archive_index in 0..7 {
            let spec = format!(
                "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA{archive_index:02}.PSARC"
            );
            let Ok(mut psarc) = oag_assets::psarc::Archive::open(&spec) else {
                continue;
            };
            let paths: Vec<String> = psarc
                .paths()
                .iter()
                .filter(|p| p.ends_with(".vex"))
                .cloned()
                .collect();
            for path in paths {
                if let Ok(blob) = psarc.read_path(&path) {
                    blobs.push((path, blob));
                }
            }
        }
    }
    let (mut total, mut read, mut files) = (0u64, 0u64, 0usize);
    let mut between: std::collections::BTreeMap<(&str, &str), (usize, u64)> = Default::default();
    let mut worst: Vec<(usize, String)> = Vec::new();
    for (path, blob) in &blobs {
        let seen = oag_formats::vex::coverage(blob);
        if seen.is_empty() {
            continue;
        }
        files += 1;
        total += seen.len() as u64;
        read += seen.claimed() as u64;
        let gaps = seen.gaps(min);
        worst.push((gaps.iter().map(|g| g.len).sum(), path.clone()));
        for gap in gaps {
            let row = between.entry((gap.after, gap.before)).or_default();
            row.0 += 1;
            row.1 += gap.len as u64;
        }
    }
    println!(
        "{files} .vex file(s): {read} of {total} byte(s) reached ({:.2}%)",
        read as f64 / total as f64 * 100.0
    );
    let mut rows: Vec<_> = between.into_iter().collect();
    rows.sort_by_key(|(_, (_, bytes))| std::cmp::Reverse(*bytes));
    for ((after, before), (count, bytes)) in rows.iter().take(6) {
        println!("  {bytes:>10} byte(s) in {count:>5} run(s)  between {after} and {before}");
    }
    worst.sort_by_key(|(n, _)| std::cmp::Reverse(*n));
    for (n, path) in worst.iter().take(3) {
        println!("  most unreached: {n:>8} byte(s)  {path}");
        let Some((_, blob)) = blobs.iter().find(|(p, _)| p == path) else {
            continue;
        };
        let seen = oag_formats::vex::coverage(blob);
        let mut gaps = seen.gaps(min);
        gaps.sort_by_key(|g| std::cmp::Reverse(g.len));
        let Some(gap) = gaps.first() else { continue };
        let run = &blob[gap.at..gap.at + gap.len];
        let zeros = run.iter().filter(|b| **b == 0).count();
        println!(
            "    {} byte(s) at {:#x}: {:.1}% zero, {} distinct value(s)",
            gap.len,
            gap.at,
            zeros as f64 / run.len() as f64 * 100.0,
            run.iter().collect::<std::collections::BTreeSet<_>>().len(),
        );
        print!("   ");
        for b in &run[..32.min(run.len())] {
            print!(" {b:02x}");
        }
        println!();
    }
    Ok(())
}
