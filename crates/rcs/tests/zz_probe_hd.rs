mod rcsmodel_common;
use oag_rcs::rcsmodel;
use rcsmodel_common::image;
#[test]
#[ignore]
fn hd() {
    let image = image().unwrap();
    let mut out = std::collections::BTreeSet::new();
    for archive in [
        "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
    ] {
        let spec = format!("{}:PS3_GAME/USRDIR/{archive}.PSARC", image.display());
        let mut open = oag_assets::psarc::Archive::open(&spec).unwrap();
        let paths: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmodel"))
            .cloned()
            .collect();
        for path in paths {
            let Ok(bytes) = open.read_path(&path) else {
                continue;
            };
            let Ok(model) = rcsmodel::Model::parse(&bytes) else {
                continue;
            };
            for m in &model.materials {
                let n = m
                    .name
                    .rsplit('/')
                    .next()
                    .unwrap_or_default()
                    .trim_end_matches(".rcsmaterial")
                    .to_ascii_lowercase();
                out.insert(format!(
                    "{n}\t{:08x}\t{:04x}\t{:04x}",
                    m.state, m.src_factor, m.dst_factor
                ));
            }
        }
    }
    std::fs::write(
        std::env::var("PROBE_OUT").unwrap(),
        out.into_iter().collect::<Vec<_>>().join("\n"),
    )
    .unwrap();
}
