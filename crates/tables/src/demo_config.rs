//! `Data/xml/DemoConfig.xml`: what 2048 and Omega author for their attract demo.
//!
//! Three things are read, each a `value` attribute: `AttractStartTimeout` (seconds the main menu
//! waits), `AttractEndTimeout` (seconds a demo runs) and the `AttractTrack` entries of
//! `AttractCycle` (a circuit named by its environment directory). The file's music, play list and
//! ship elements are not read: nothing here plays them. Scanned as text, like the other one-
//! document tables (`audio_config`). The file is byte-identical on 2048 and Omega
//! (`docs/ui/attract-demo.md`).

/// What the file authors.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DemoConfig {
    /// `AttractStartTimeout`: seconds without a press before a demo starts.
    pub start_seconds: Option<f32>,
    /// `AttractEndTimeout`: seconds a demo runs.
    pub end_seconds: Option<f32>,
    /// The `AttractTrack`s of `AttractCycle`, in order: each a circuit's environment directory.
    pub tracks: Vec<String>,
}

/// Reads the document. Anything missing stays `None` or empty.
#[must_use]
pub fn parse(xml: &str) -> DemoConfig {
    let xml = without_comments(xml);
    DemoConfig {
        start_seconds: value(&xml, "AttractStartTimeout").and_then(|text| text.parse().ok()),
        end_seconds: value(&xml, "AttractEndTimeout").and_then(|text| text.parse().ok()),
        tracks: elements(&xml, "AttractTrack")
            .filter_map(|element| attribute(element, "value"))
            .map(str::to_string)
            .collect(),
    }
}

fn without_comments(xml: &str) -> String {
    let mut out = String::with_capacity(xml.len());
    let mut rest = xml;
    while let Some(open) = rest.find("<!--") {
        out.push_str(&rest[..open]);
        rest = match rest[open..].find("-->") {
            Some(close) => &rest[open + close + 3..],
            None => "",
        };
    }
    out.push_str(rest);
    out
}

/// The text of each `<name ...>` start tag.
fn elements<'a>(xml: &'a str, name: &'a str) -> impl Iterator<Item = &'a str> {
    let open = format!("<{name}");
    let mut from = 0;
    std::iter::from_fn(move || {
        loop {
            let at = from + xml[from..].find(&open)?;
            let tag_end = at + xml[at..].find('>')?;
            from = tag_end;
            let after = xml[at + open.len()..].chars().next();
            // `<AttractTrack` and not `<AttractTracks`.
            if after.is_some_and(|c| c.is_whitespace() || c == '/' || c == '>') {
                return Some(&xml[at..tag_end]);
            }
        }
    })
}

fn value<'a>(xml: &'a str, name: &'a str) -> Option<&'a str> {
    attribute(elements(xml, name).next()?, "value")
}

fn attribute<'a>(element: &'a str, name: &str) -> Option<&'a str> {
    let key = format!("{name}=\"");
    let from = element.find(&key)? + key.len();
    Some(&element[from..from + element[from..].find('"')?])
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHAPE: &str = r#"<DemoConfig>
        <AttractStartTimeout value="12.5"/>
        <AttractEndTimeout value="90"/>
        <!-- <AttractTrack value="commented"/> -->
        <AttractCycle><AttractTrack value="alpha"/><AttractTrack value="beta"/></AttractCycle>
        <DemoPlayTrackList><PlayTrack value="ignored"/></DemoPlayTrackList>
    </DemoConfig>"#;

    #[test]
    fn the_two_timeouts_and_the_cycle_are_read_and_a_comment_is_not() {
        assert_eq!(
            parse(SHAPE),
            DemoConfig {
                start_seconds: Some(12.5),
                end_seconds: Some(90.0),
                tracks: vec!["alpha".into(), "beta".into()],
            }
        );
    }

    #[test]
    fn a_document_with_none_of_it_reads_as_nothing_authored() {
        assert_eq!(parse("<DemoConfig></DemoConfig>"), DemoConfig::default());
    }
}
