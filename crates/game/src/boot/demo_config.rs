//! The attract demo's authored file, for the titles that ship one (`oag_title::demo::Demo::config`).

use oag_tables::demo_config::{DemoConfig, parse};

/// Reads the title's `DemoConfig.xml`, or `None` when the title names none or the file will not
/// read; the report says which.
pub fn load(
    archives: &mut oag_assets::Archives,
    title: &oag_title::Title,
    report: &mut Vec<String>,
) -> Option<DemoConfig> {
    let name = title.demo.config?;
    let xml = match archives
        .read_name(name)
        .map_err(|error| format!("{error:#}"))
        .and_then(|bytes| String::from_utf8(bytes.to_vec()).map_err(|error| error.to_string()))
    {
        Ok(xml) => xml,
        Err(error) => {
            report.push(format!("{name}: {error}, so the attract demo takes the title's own timing"));
            return None;
        }
    };
    let config = parse(&xml);
    report.push(format!(
        "{name}: attract starts after {:?} s, runs {:?} s, cycles {:?}",
        config.start_seconds, config.end_seconds, config.tracks
    ));
    Some(config)
}
