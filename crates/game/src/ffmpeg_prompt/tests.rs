use super::*;

fn english(_: &str, english: &str) -> String {
    english.to_string()
}

#[test]
fn the_prompt_names_the_size_and_the_source() {
    let labels = prompt_labels(135, english);
    assert!(labels.message.contains("135 MB"), "{}", labels.message);
    assert!(labels.message.contains("GITHUB"), "{}", labels.message);
    assert!(labels.message.contains("LGPL"), "{}", labels.message);
    assert!(!labels.message.contains("%s"));
}

#[test]
fn the_status_line_counts_megabytes_and_then_says_unpacking() {
    assert_eq!(
        status_text(37_000_000, 135_000_000, english),
        "DOWNLOADING FFMPEG 37 / 135 MB - BACK CANCELS"
    );
    assert_eq!(status_text(5_000_000, 0, english), "DOWNLOADING FFMPEG 5 MB - BACK CANCELS");
    assert_eq!(status_text(135_000_000, 135_000_000, english), "UNPACKING FFMPEG...");
}
