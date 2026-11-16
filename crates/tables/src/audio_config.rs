//! The two things of `AudioConfig.xml` the rear low-pass reads.
//!
//! `Data\sound\GlobalAudioConfig.xml` carries `<RearOcclusion>` with one
//! `max_frequency` per output layout, and a circuit's own `AudioConfig.xml`
//! may carry a `<DistanceFilter low_pass="0">` (Moa Therma's), which reads as a
//! high-pass rather than a low-pass (`docs/ghidra/functions/ps4-omega-eu/audio-3d.md`,
//! "The rear low-pass"). Scanned as text, like the rest of the sound tables:
//! one authored document, attributes of single elements.

/// The `max_frequency` of `<RearOcclusion>`'s `child` (`Stereo` or `Surround`), in hertz.
#[must_use]
pub fn rear_occlusion_max_frequency(xml: &str, child: &str) -> Option<f32> {
    let open = xml.find("<RearOcclusion")?;
    let body = &xml[open..];
    let body = &body[..body.find("</RearOcclusion>").unwrap_or(body.len())];
    let from = body.find(&format!("<{child} "))?;
    let element = &body[from..from + body[from..].find('>')?];
    attribute(element, "max_frequency")
}

/// Whether the circuit's `<DistanceFilter>` has its low-pass switched off
/// (`low_pass="0"`), which is the one circuit whose filter is not a low-pass.
#[must_use]
pub fn distance_filter_low_pass_off(xml: &str) -> bool {
    let Some(open) = xml.find("<DistanceFilter") else {
        return false;
    };
    let element = &xml[open..open + xml[open..].find('>').unwrap_or(xml.len() - open)];
    attribute(element, "low_pass") == Some(0.0)
}

fn attribute(element: &str, name: &str) -> Option<f32> {
    let key = format!(" {name}=\"");
    let from = element.find(&key)? + key.len();
    let value = &element[from..from + element[from..].find('"')?];
    value.trim_end_matches('f').trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const GLOBAL: &str = r#"<AudioConfig>
  <RearOcclusion>
    <Stereo max_frequency="14000.0"/>
    <Surround max_frequency="24000.0"/>
  </RearOcclusion>
  <Gamma value="1"/>
</AudioConfig>"#;

    #[test]
    fn each_layout_reads_its_own_value() {
        assert_eq!(
            rear_occlusion_max_frequency(GLOBAL, "Stereo"),
            Some(14000.0)
        );
        assert_eq!(
            rear_occlusion_max_frequency(GLOBAL, "Surround"),
            Some(24000.0)
        );
        assert_eq!(rear_occlusion_max_frequency(GLOBAL, "Mono"), None);
    }

    #[test]
    fn a_file_with_no_block_reads_nothing() {
        assert_eq!(
            rear_occlusion_max_frequency("<AudioConfig/>", "Stereo"),
            None
        );
    }

    #[test]
    fn only_a_zero_low_pass_is_off() {
        let off = r#"<DistanceFilter near_cutoff="0.0" far_cutoff="999.0" low_pass="0"/>"#;
        let on = r#"<DistanceFilter near_cutoff="23999.0" far_cutoff="23999.0" low_pass="1"/>"#;
        assert!(distance_filter_low_pass_off(off));
        assert!(!distance_filter_low_pass_off(on));
        assert!(!distance_filter_low_pass_off("<AudioConfig/>"));
    }
}
