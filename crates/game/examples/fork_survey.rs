//! Every circuit's forks: paths, junctions, and each [`oag_race::course::Branch`]
//! with its length and how far it strays from the ring.
//!
//! The census behind `docs/gameplay/ai.md`'s "Branch choice at a fork": which
//! circuits have a fork an opponent can take, and whether the alternate is a
//! second line over the same stretch or a real detour.
//!
//! ```sh
//! cargo run -p oag-game --example fork_survey -- data/images/pulse-psp-usa.chd
//! cargo run -p oag-game --example fork_survey -- data/extracted/vita/PCSF00007
//! ```

use oag_race::Course;
use oag_raceplay::catalogue;
use oag_vex::{track, vex};

fn main() {
    let source = std::env::args()
        .nth(1)
        .expect("usage: fork_survey <source>");
    let is_2048 = source.contains("vita");
    let (mut archives, entries): (oag_assets::Archives, Vec<(String, String)>) = if is_2048 {
        let mut archives = oag_2048::open(&source).expect("mounting 2048");
        let blob = archives
            .read_name(r"data\plugins\tracks\Definition.xml")
            .or_else(|_| archives.read_name("data/plugins/tracks/Definition.xml"))
            .expect("the track plugin definition");
        let text = String::from_utf8_lossy(&blob);
        let mut entries = Vec::new();
        for part in text.split("<PI_Track").skip(1) {
            if let Some(name) = part
                .split("name=\"")
                .nth(1)
                .and_then(|rest| rest.split('"').next())
            {
                let entry = format!(
                    r"Data\art\published\environments\{}\track.vex",
                    name.to_lowercase()
                );
                entries.push((name.to_string(), entry));
            }
        }
        (archives, entries)
    } else {
        let mut archives = oag_pulse::open(&source).expect("mounting the disc");
        let blob = archives
            .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
            .expect("the game plugin definition");
        let definition = oag_tables::fexml::expand(&blob).expect("expanding it");
        let entries = catalogue::tracks(&definition)
            .into_iter()
            .filter(|t| !t.reversed)
            .map(|t| (t.id.clone(), t.entry_name()))
            .collect();
        (archives, entries)
    };
    for (id, entry) in entries {
        let Ok(file) = archives.read_name(&entry) else {
            println!("{id:<16} unreadable: {entry}");
            continue;
        };
        let Ok(nodes) = vex::nodes(&file) else {
            println!("{id:<16} no vex");
            continue;
        };
        let Some(node) = track::find_node(&file, &nodes) else {
            println!("{id:<16} no WO Track");
            continue;
        };
        let Ok(ai) = track::parse(&file[node.payload()]) else {
            println!("{id:<16} WO Track does not parse");
            continue;
        };
        let forks = ai
            .junctions
            .iter()
            .filter(|j| j.next[0].is_some() && j.next[1].is_some())
            .count();
        let Some(course) = Course::from_track(&ai, None) else {
            println!(
                "{id:<16} {} paths {} junctions {forks} forks, no ring",
                ai.paths.len(),
                ai.junctions.len()
            );
            continue;
        };
        println!(
            "{id:<16} {} paths {} junctions {forks} forks, ring {} pts {:.0} units, order {:?}, {} branch(es)",
            ai.paths.len(),
            ai.junctions.len(),
            course.len(),
            course.length(),
            course.path_order(),
            course.branches().len()
        );
        let order = course.path_order();
        for (j, junction) in ai.junctions.iter().enumerate() {
            if let (Some(primary), Some(alternate)) = (junction.next[0], junction.next[1]) {
                let entering: Vec<String> = junction
                    .prev
                    .iter()
                    .flatten()
                    .map(|p| {
                        format!(
                            "{p}{}",
                            if order.contains(&(*p as u16)) {
                                "*"
                            } else {
                                ""
                            }
                        )
                    })
                    .collect();
                let mut chain = vec![alternate];
                let mut at = alternate;
                for _ in 0..ai.paths.len() {
                    let Some(next) = ai.paths[at]
                        .exit
                        .and_then(|e| ai.junctions.get(e))
                        .and_then(|e| e.next[0])
                    else {
                        break;
                    };
                    if order.contains(&(next as u16)) {
                        chain.push(next + 1000);
                        break;
                    }
                    chain.push(next);
                    at = next;
                }
                println!(
                    "    fork j{j}: from {entering:?} primary {primary}{} alternate {alternate}{}, alt chain {chain:?} (1000+ = rejoins ring)",
                    if order.contains(&(primary as u16)) {
                        "*"
                    } else {
                        ""
                    },
                    if order.contains(&(alternate as u16)) {
                        "*"
                    } else {
                        ""
                    },
                );
            }
        }
        for (k, r) in course.routes().iter().enumerate() {
            let near = |p: oag_core::math::Vec3| {
                (0..course.len())
                    .filter_map(|i| course.position(i))
                    .map(|c| (c - p).length())
                    .fold(f32::INFINITY, f32::min)
            };
            let diverge = r.positions.iter().position(|p| near(*p) > 8.0);
            println!(
                "    route {k} paths {:?}: {} samples, first sample more than 8 units off the ring: {diverge:?}",
                r.paths,
                r.len()
            );
        }
        for (k, b) in course.branches().iter().enumerate() {
            let stray = b
                .centres
                .iter()
                .map(|p| {
                    (0..course.len())
                        .filter_map(|i| course.centre(i))
                        .map(|c| (c - *p).length())
                        .fold(f32::INFINITY, f32::min)
                })
                .fold(0.0f32, f32::max);
            let ring_span = course.progress_at(b.merge).unwrap_or(0.0)
                - course.progress_at(b.split).unwrap_or(0.0);
            println!(
                "    branch {k}: split {} (path {:?}) merge {} (path {:?}), {} samples, ring span {:.0}, strays up to {stray:.1} from the ring",
                b.split,
                course.path_of(b.split),
                b.merge,
                course.path_of(b.merge),
                b.len(),
                ring_span.rem_euclid(course.length()),
            );
        }
    }
}
