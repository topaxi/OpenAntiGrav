use oag_tables::fexml;

#[test]
#[ignore = "needs a disc image in data/images/"]
fn dump() {
    let Some(path) = oag_testdata::image("data/images/pulse-ps2-eu.chd") else {
        return;
    };
    let mut archives = oag_assets::Archives::open(&path.to_string_lossy(), oag_pulse::TITLE)
        .expect("the archives open");
    let raw = archives
        .read_name(r"Data\Plugins\PI001\GUI\Selection_Definition.xml")
        .expect("the file");
    let xml = fexml::text(&raw).expect("text");
    let screens = oag_ui::screen::Screens::from_xml(&xml);
    let screen = screens.by_name("Team SelectionSplit").expect("split");
    for t in &screen.texts {
        println!("text {:?} {:?} x={} y={}", t.name, t.idstring, t.x, t.y);
    }
    for i in &screen.images {
        println!("image {:?} x={} y={} w={:?} h={:?}", i.name, i.x, i.y, i.width, i.height);
    }
    for f in &screen.fills {
        println!("fill {:?} x={} y={} w={:?} h={:?}", f.name, f.x, f.y, f.width, f.height);
    }
}
