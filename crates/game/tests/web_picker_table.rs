//! The disc table on the web page's picker (`web/index.html`) and the one in
//! `docs/overview/status.md`, section 9, say the same thing, row for row.
//!
//! The page is the data (plain HTML, no script needed to read it); the status
//! page is the claim a reader can follow to its evidence. A row added, dropped
//! or re-worded on one side fails here until the other follows.

use std::path::Path;

fn read(relative: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join(relative)).unwrap_or_else(|e| panic!("{relative}: {e}"))
}

/// The first five cells of every row in the page's `<table id="images">`.
fn page_rows(html: &str) -> Vec<Vec<String>> {
    let table = html
        .split("<table id=\"images\">")
        .nth(1)
        .and_then(|rest| rest.split("</table>").next())
        .expect("the picker has its disc table");
    table
        .split("<tr>")
        .filter(|row| row.contains("<td"))
        .map(|row| {
            row.split("<td")
                .skip(1)
                .map(|cell| {
                    let text = cell.split_once('>').map_or("", |(_, rest)| rest);
                    text.split("</td>").next().unwrap_or("").trim().to_string()
                })
                .take(5)
                .collect()
        })
        .collect()
}

/// The rows of the table under `## 9. In the browser`.
fn status_rows(markdown: &str) -> Vec<Vec<String>> {
    let section = markdown
        .split("## 9. In the browser")
        .nth(1)
        .and_then(|rest| rest.split("\n## ").next())
        .expect("status.md has section 9");
    section
        .lines()
        .filter(|line| {
            line.starts_with('|') && !line.starts_with("| ---") && !line.starts_with("| Game")
        })
        .map(|line| {
            line.trim_matches('|')
                .split('|')
                .map(|cell| cell.trim().to_string())
                .collect()
        })
        .collect()
}

#[test]
fn the_picker_table_and_the_status_page_agree() {
    let page = page_rows(&read("web/index.html"));
    let status = status_rows(&read("docs/overview/status.md"));
    assert!(
        page.len() >= 6,
        "the page lists every supported disc: {page:?}"
    );
    assert_eq!(page, status);
}

#[test]
fn every_row_says_how_far_it_gets() {
    for row in page_rows(&read("web/index.html")) {
        assert!(
            ["Races", "Front end only", "Not yet"].contains(&row[4].as_str()),
            "{row:?}"
        );
    }
}
