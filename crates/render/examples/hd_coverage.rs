//! Scratch probe: how much of each `.rcsmodel` on the disc `oag_formats`
//! actually reads, and what the largest unread runs sit between.

use oag_rcs::rcsmodel;

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    let min: usize = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(64);
    let (mut total, mut read) = (0u64, 0u64);
    let mut between: std::collections::BTreeMap<(&str, &str), (usize, u64)> = Default::default();
    let mut worst: Vec<(usize, String)> = Vec::new();
    for archive_index in 0..7 {
        let spec = format!("{image}:PS3_GAME/USRDIR/DATA{archive_index:02}.PSARC");
        let Ok(mut psarc) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let paths: Vec<String> = psarc
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmodel"))
            .cloned()
            .collect();
        for path in paths {
            let Ok(blob) = psarc.read_path(&path) else {
                continue;
            };
            let seen = rcsmodel::coverage(&blob);
            if seen.is_empty() {
                continue;
            }
            total += seen.len() as u64;
            read += seen.claimed() as u64;
            let gaps = seen.gaps(min);
            let unclaimed: usize = gaps.iter().map(|g| g.len).sum();
            worst.push((unclaimed, path.clone()));
            for gap in gaps {
                let row = between.entry((gap.after, gap.before)).or_default();
                row.0 += 1;
                row.1 += gap.len as u64;
            }
        }
    }
    println!(
        "{read} of {total} byte(s) read across every .rcsmodel on the disc ({:.2}%)",
        read as f64 / total as f64 * 100.0
    );
    println!("unread runs of {min}+ bytes, by what they sit between:");
    let mut rows: Vec<_> = between.into_iter().collect();
    rows.sort_by_key(|(_, (_, bytes))| std::cmp::Reverse(*bytes));
    for ((after, before), (count, bytes)) in rows.iter().take(12) {
        println!("  {bytes:>12} byte(s) in {count:>6} run(s)  between {after} and {before}");
    }
    worst.sort_by_key(|(n, _)| std::cmp::Reverse(*n));
    println!("the files with the most unread:");
    for (n, path) in worst.iter().take(5) {
        println!("  {n:>10} byte(s)  {path}");
    }
    Ok(())
}
