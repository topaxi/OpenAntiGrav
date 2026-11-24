#[test]
#[ignore = "scratch"]
fn dump() {
    let Some(p) = oag_testdata::image("pure-psp-usa.chd") else {
        return;
    };
    let mut a = oag_pure::open(p.to_str().unwrap()).unwrap();
    for t in [
        "Environments\\08_Sinucit",
        "Environments\\10_Sebenco_Climb",
        "Environments\\01_Vineta_K",
    ] {
        let blob = a.read_name(&format!(r"Data\{t}\track.vex")).unwrap();
        let nodes = oag_vex::vex::nodes(&blob).unwrap();
        let mut hist = std::collections::BTreeMap::new();
        for n in &nodes {
            *hist.entry(n.class_id).or_insert(0) += 1;
        }
        eprintln!(
            "{t}: sections {:?} 0x38c {:?} 0x38b {:?} 0x38d {:?}",
            oag_vex::vex::classes_of(&blob).unwrap().section,
            hist.get(&0x38c),
            hist.get(&0x38b),
            hist.get(&0x38d)
        );
        for n in nodes.iter().filter(|n| n.class_id == 0x38c).take(3) {
            eprintln!(
                "   wp name {:?} parent {:?} size {} children {}",
                n.name,
                n.parent.map(|i| (nodes[i].class_id, nodes[i].name.clone())),
                n.data_size,
                n.child_count
            );
        }
        let high: Vec<_> = hist.iter().filter(|(k, _)| **k >= 0x380).collect();
        eprintln!("   classes >= 0x380: {:?}", high);
    }
}
