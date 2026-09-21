//! Following a front-end root's `<LoadXML>` includes, for a title whose
//! screens live there rather than in the root.
//!
//! Wipeout 2048 is that title: `NEWGUI/Skin.xml` declares globals and six
//! includes and not one screen, so [`super::screens::load_screens`] on its
//! own yields a `Screens` the boot chain cannot find a single step in - and
//! `walked` comes out empty, which is a panic in `Frontend::booting`, not a
//! boot. See `oag_2048::frontend::includes` for the shape of the include
//! tree and which three files of it are followed.
//!
//! Pulse, Pure and HD are **not** routed through this: their roots declare
//! the boot screens directly, their includes are the menu definitions this
//! build does not draw, and following them would add every menu's images to
//! a sprite sheet nothing samples. `boot::load_shell` calls this only for a
//! title that fills [`oag_title::FrontEnd::touch`].

use oag_ui::screen::{Include, Screens};

use super::screens::load_included_screens;

/// Merges every screen in the includes `followed` names into `screens`,
/// recursively - an include is followed only when its own `SrcRel`/`src`
/// spelling (before any region suffix) is in the list, at whatever depth
/// it appears.
///
/// A screen already present by `path` is not added twice - `Definition.xml`
/// nests `Intro_Definition.xml` inside `newFEshell`, and the same file read
/// on its own would spell the same screens under a different parent, so
/// `name` is what a caller looks a boot step up by (`Screens::by_name`) and
/// the first spelling wins. An include that will not read is a report line
/// and a skipped subtree, never a failed boot: the root's own report already
/// names what was found, and a chain step whose screen is missing is
/// reported by name by the walk that follows.
pub(super) fn follow(
    archives: &mut oag_assets::Archives,
    root: &str,
    screens: &mut Screens,
    followed: &[&str],
    localised_suffix: &str,
    report: &mut Vec<String>,
) {
    // The root's globals are the fallbacks every include resolves
    // `FEGlobals->` against; cloned once so the includes' own additions do
    // not shadow them for the next include.
    let root_screens = screens.clone();
    let mut pending: Vec<(String, Include)> = screens
        .includes
        .iter()
        .filter(|include| followed.contains(&include.src.as_str()))
        .map(|include| (root.to_string(), include.clone()))
        .collect();
    while !pending.is_empty() {
        let mut next: Vec<(String, Include)> = Vec::new();
        for (from, include) in pending.drain(..) {
            let name = resolve(&from, &include, localised_suffix);
            if include.direct_embed {
                embed(archives, &name, &include, screens, &root_screens, report);
                continue;
            }
            match load_included_screens(archives, &name, &root_screens) {
                Ok(included) => {
                    let mut added = 0usize;
                    for screen in included.screens {
                        if screens.screens.iter().any(|s| s.name == screen.name) {
                            continue;
                        }
                        screens.screens.push(screen);
                        added += 1;
                    }
                    let (follow, skip): (Vec<Include>, Vec<Include>) = included
                        .includes
                        .into_iter()
                        .partition(|include| followed.contains(&include.src.as_str()));
                    report.push(format!(
                        "include {name}: {added} screen(s), {} include(s) below it, {} not followed",
                        follow.len() + skip.len(),
                        skip.len()
                    ));
                    next.extend(follow.into_iter().map(|include| (name.clone(), include)));
                }
                Err(error) => report.push(format!("include {name}: {error:#} - skipped")),
            }
        }
        pending = next;
    }
}

/// A `DirectEmbed` include: the file's bare widgets, read as if they were
/// written inside the screen the `LoadXML` sits in, and appended to that
/// screen's own.
///
/// `TitleScreen`'s `Legal_Line_Definition_EU.xml` is the case: one `<Text>`
/// and no `<Screen>`, so parsed on its own it yields nothing. Wrapping it in
/// the including screen's name is exactly what the attribute says the
/// engine does.
fn embed(
    archives: &mut oag_assets::Archives,
    name: &str,
    include: &Include,
    screens: &mut Screens,
    root: &Screens,
    report: &mut Vec<String>,
) {
    let Some(screen_name) = include.screen.as_deref() else {
        report.push(format!(
            "include {name}: DirectEmbed outside any screen - skipped"
        ));
        return;
    };
    let blob = match archives.read_name(name) {
        Ok(blob) => blob,
        Err(error) => {
            report.push(format!("include {name}: {error} - skipped"));
            return;
        }
    };
    let xml = match super::xml::expand(&blob) {
        Ok(xml) => xml,
        Err(error) => {
            report.push(format!("include {name}: {error:#} - skipped"));
            return;
        }
    };
    let globals: Vec<(&str, &str)> = root
        .globals
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    let wrapped = format!("<Screen name=\"{screen_name}\">{xml}</Screen>");
    let parsed = Screens::from_xml_with_fallback_globals(&wrapped, &globals);
    let Some(host) = screens.screens.iter_mut().find(|s| s.name == screen_name) else {
        report.push(format!(
            "include {name}: embeds into {screen_name:?}, which is not loaded - skipped"
        ));
        return;
    };
    let mut widgets = 0usize;
    for embedded in parsed.screens.into_iter().filter(|s| s.name == screen_name) {
        widgets += embedded.texts.len() + embedded.images.len() + embedded.fills.len();
        host.texts.extend(embedded.texts);
        host.images.extend(embedded.images);
        host.fills.extend(embedded.fills);
        host.redirects.extend(embedded.redirects);
        host.touch_buttons.extend(embedded.touch_buttons);
    }
    report.push(format!(
        "include {name}: {widgets} widget(s) embedded into {screen_name:?}"
    ));
}

/// The archive path an include names, from the file that names it.
///
/// `SrcRel` is beside the including file; `localised` takes the region
/// suffix before the extension. Both spellings are the disc's own.
fn resolve(from: &str, include: &Include, localised_suffix: &str) -> String {
    let src = if include.localised {
        match include.src.rfind('.') {
            Some(dot) => format!(
                "{}{localised_suffix}{}",
                &include.src[..dot],
                &include.src[dot..]
            ),
            None => format!("{}{localised_suffix}", include.src),
        }
    } else {
        include.src.clone()
    };
    if !include.relative {
        return src;
    }
    match from.rfind(['\\', '/']) {
        Some(at) => format!("{}{}", &from[..=at], src),
        None => src,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_relative_localised_include_lands_beside_its_root_with_the_suffix() {
        let include = Include {
            src: "Bootup_Definition.xml".to_string(),
            relative: true,
            localised: true,
            direct_embed: false,
            screen: None,
        };
        assert_eq!(
            resolve(r"Data\Plugins\Frontend\NEWGUI\Skin.xml", &include, "_EU"),
            r"Data\Plugins\Frontend\NEWGUI\Bootup_Definition_EU.xml"
        );
    }

    #[test]
    fn an_absolute_include_is_used_verbatim() {
        let include = Include {
            src: r"Data\Plugins\Frontend\Gui\MainMenu_Definition.xml".to_string(),
            relative: false,
            localised: false,
            direct_embed: false,
            screen: None,
        };
        assert_eq!(
            resolve(r"Data\Plugins\Frontend\Gui\Skin.xml", &include, "_EU"),
            r"Data\Plugins\Frontend\Gui\MainMenu_Definition.xml"
        );
    }
}
