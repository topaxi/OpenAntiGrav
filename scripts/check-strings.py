#!/usr/bin/env python3
"""A ratchet on invented English shipping with no translatable id.

`assets/ui/menu.toml`'s own header settles that an invented menu tree is in
scope for this project - it is not a transcription of the disc's own XML.
What it does not settle on its own is *translation*: a row's `label` is
literal English unless it names a `string_id` that
`crate::menu::definition::resolved` looks up in `assets/ui/strings/<language>
.toml`, and nothing forced a new row to name one. The pilot editor landed a
whole screen - an AI PILOTS page, an on-screen keyboard, a confirm dialog -
before anything checked that. This script is that check.

It mirrors `check-file-size.py`'s own shape on purpose, for the same reason:
82 of this file's label rows and 9 of its page titles already had no id
before this script existed, and failing the gate on all of them turns the
gate off. So existing debt was frozen in `BASELINE_LABELS`/`BASELINE_TITLES` -
a row already there when the rule landed could stay untranslated, but nothing
*new* could join it, and a row that gained a `string_id` had to have its
baseline entry deleted, the same "graduated" rule `check-file-size.py`
enforces. **Both sets are empty as of 2026-09-08** - the last eight pages'
worth of debt converted in one pass - but they stay declared rather than
removed, so the mechanism (and a future regression) still has somewhere to
be checked against.

Three things are checked, offline and without a Ghidra bridge or a built
binary:

1. **Every row's `label` and every page's `title` in `assets/ui/menu.toml`
   names a `string_id`/`title_string_id`, unless `BASELINE_LABELS`/
   `BASELINE_TITLES` names it as pre-existing debt.** A new row or page with
   neither is the failure this script exists to catch.
2. **Every id actually named - by a row, a page, or one of the hand-picked
   Rust call sites in `STRING_CONSUMERS` below - resolves to real text in
   `assets/ui/strings/english.toml`.** English is the base language and
   `check-strings.py` allows it no gap: a `string_id` that resolves to
   nothing is a typo or a forgotten entry, not a language that has not
   caught up, and it is a hard failure either way.
3. **Every *other* language file under `assets/ui/strings/` covers the same
   ids** - translated under its own `[strings]`, or named, explicitly, under
   an `[untranslated] ids = [...]` table. A missing entry is a hard failure;
   a marked one is not. See `assets/ui/strings/english.toml`'s own doc
   comment for why the second is honest and the first is not. A machine
   translation may fill a row, marked `human = false`; the per-file count of
   those is reported below, never failed on. `assets/ui/strings/french.toml` is the first language file to
   exercise this rule for real: a real translation for the ids the
   maintainer could translate with confidence, and the rest named under its
   own `[untranslated]`.

`STRING_CONSUMERS` is deliberately a short, hand-maintained list rather than
a scan of every `.rs` file in the tree: this project's own env vars
(`OAG_REQUIRE_GAME_DATA`, `OAG_SWEEP_LOOK`, `OAG_IMAGE`, ...) share the same
`OAG_` prefix by convention, and a blanket regex over `crates/game/src`
would flag every one of them as an unresolved string id. Each file named
here was read by hand and confirmed to hold nothing but `say`/`say_of`/
`resolved` lookups against a real `StringTable` - adding a new consumer
means doing that reading once, then adding its path to the list.
"""

from __future__ import annotations

import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MENU_TOML = ROOT / "assets/ui/menu.toml"
STRINGS_DIR = ROOT / "assets/ui/strings"
BASE_LANGUAGE = "english"

# Pre-existing debt: `(page id, label)` pairs allowed to carry no
# `string_id`. Recorded on 2026-09-07, the day this script landed; **empty**
# as of 2026-09-08 - every row that predated the rule now names an id, the
# last eight pages converted in the same pass that added the AI PILOTS page's
# own axis preview. Left as an explicit empty set rather than deleted, so a
# row that regresses (a new one added with no id) fails here rather than
# silently starting a fresh pile of debt with nothing recording that the
# baseline was ever cleared.
BASELINE_LABELS: set[tuple[str, str]] = set()

# Pre-existing debt: page ids allowed to carry no `title_string_id`. **Empty**
# as of 2026-09-08, for the same reason and in the same change as
# `BASELINE_LABELS` above.
BASELINE_TITLES: set[str] = set()

# Pre-existing debt: `(page id, subtitle)` pairs allowed to carry no
# `subtitle_string_id`. `subtitle` is optional per row - most rows in this
# file carry none at all, and an empty `subtitle` is never checked - so this
# set is about the ones that *do* name one. **Empty since `subtitle` itself
# was introduced 2026-09-15**, the same "declared rather than deleted" reason
# `BASELINE_LABELS` gives.
BASELINE_SUBTITLES: set[tuple[str, str]] = set()

# Files read by hand and confirmed to hold nothing under an `OAG_` string
# literal but a real lookup against a `StringTable` - see this module's own
# doc for why this is a named list and not a tree-wide regex.
#
# `hints.rs` and `wording.rs` had already wired the *mechanism* -
# `OAG_HINTS_*`/`OAG_LOADING_*` lookups with a fallback - before
# `english.toml` carried real text for any of them, which is exactly the gap
# this script exists to close. Their 23 ids now have entries there too, so
# both are scanned rather than carved out; the fallback literal at each call
# site is what was copied in, so the two must be kept in step by hand until
# either file gains its own generated manifest.
STRING_CONSUMERS = [
    "crates/game/src/main/session/pilot_editor.rs",
    "crates/game/src/main/session/pilot_editor/tests.rs",
    "crates/game/src/capture/menu_page.rs",
    "crates/game/src/main/hints.rs",
    "crates/game/src/loading/wording.rs",
    "crates/game/src/main/rebind.rs",
    "crates/game/src/main/session/campaign.rs",
]

ID_LITERAL = re.compile(r'"(OAG_[A-Z0-9_]+)"')


def load_menu() -> dict:
    with MENU_TOML.open("rb") as handle:
        return tomllib.load(handle)


def load_strings(
    path: Path, problems: list[str]
) -> tuple[dict[str, str], list[str], int]:
    """A language file's `[strings]` texts, its `[untranslated].ids` and how
    many entries say `human = false`.

    Every entry must be `ID = { text = "...", human = <bool> }` - the shape
    `crates/ui/src/strings.rs` parses with `deny_unknown_fields` and no default,
    so a bare string or a missing flag is named here instead of dropping the
    whole language at runtime. The `human = false` count is a report, never a
    failure: a model-written translation is allowed, as long as it says so.
    """
    with path.open("rb") as handle:
        data = tomllib.load(handle)
    texts: dict[str, str] = {}
    machine = 0
    for id_, entry in data.get("strings", {}).items():
        if (
            not isinstance(entry, dict)
            or set(entry) != {"text", "human"}
            or not isinstance(entry["text"], str)
            or not isinstance(entry["human"], bool)
        ):
            problems.append(
                f"{path.name}: {id_!r} is not `{{ text = \"...\", human = <bool> }}`"
            )
            continue
        texts[id_] = entry["text"]
        machine += not entry["human"]
    return texts, data.get("untranslated", {}).get("ids", []), machine


def ids_from_consumers() -> set[str]:
    found: set[str] = set()
    for relative in STRING_CONSUMERS:
        path = ROOT / relative
        if not path.exists():
            continue
        found |= set(ID_LITERAL.findall(path.read_text(encoding="utf-8")))
    return found


def report(heading: str, rows: list[str], advice: str) -> None:
    if not rows:
        return
    print(f"\n{heading}\n")
    for row in sorted(rows):
        print(f"  {row}")
    print(f"\n{advice}")


def main() -> int:
    menu = load_menu()

    uncovered_labels: list[str] = []
    uncovered_titles: list[str] = []
    uncovered_subtitles: list[str] = []
    graduated_labels: list[str] = []
    graduated_titles: list[str] = []
    graduated_subtitles: list[str] = []
    menu_ids: set[str] = set()
    seen_labels: set[tuple[str, str]] = set()
    seen_titles: set[str] = set()
    seen_subtitles: set[tuple[str, str]] = set()

    for page in menu.get("page", []):
        page_id = page["id"]
        seen_titles.add(page_id)
        title_id = page.get("title_string_id")
        baselined_title = page_id in BASELINE_TITLES
        if title_id is None:
            if not baselined_title:
                uncovered_titles.append(f"page {page_id!r}: title {page['title']!r}")
        else:
            menu_ids.add(title_id)
            if baselined_title:
                graduated_titles.append(page_id)

        for entry in page.get("entry", []):
            label = entry.get("label", "")
            if not label:
                continue
            key = (page_id, label)
            seen_labels.add(key)
            string_id = entry.get("string_id")
            baselined = key in BASELINE_LABELS
            if string_id is None:
                if not baselined:
                    uncovered_labels.append(f"page {page_id!r} label {label!r}")
            else:
                menu_ids.add(string_id)
                if baselined:
                    graduated_labels.append(f"{key!r}")

            subtitle = entry.get("subtitle")
            if not subtitle:
                continue
            subtitle_key = (page_id, subtitle)
            seen_subtitles.add(subtitle_key)
            subtitle_id = entry.get("subtitle_string_id")
            baselined_subtitle = subtitle_key in BASELINE_SUBTITLES
            if subtitle_id is None:
                if not baselined_subtitle:
                    uncovered_subtitles.append(
                        f"page {page_id!r} subtitle {subtitle!r}"
                    )
            else:
                menu_ids.add(subtitle_id)
                if baselined_subtitle:
                    graduated_subtitles.append(f"{subtitle_key!r}")

    vanished_labels = [f"{key!r}" for key in BASELINE_LABELS if key not in seen_labels]
    vanished_titles = [page_id for page_id in BASELINE_TITLES if page_id not in seen_titles]
    vanished_subtitles = [
        f"{key!r}" for key in BASELINE_SUBTITLES if key not in seen_subtitles
    ]

    all_ids = menu_ids | ids_from_consumers()

    base_path = STRINGS_DIR / f"{BASE_LANGUAGE}.toml"
    shape_problems: list[str] = []
    human_report: list[str] = []
    base_strings, base_untranslated, base_machine = load_strings(
        base_path, shape_problems
    )
    human_report.append(
        f"{base_path.name}: {base_machine} of {len(base_strings)} human = false"
    )
    if base_untranslated:
        # The base language is the one place a gap cannot be marked instead
        # of filled - see `english.toml`'s own doc comment.
        print(
            f"\n{base_path}'s own [untranslated] names "
            f"{sorted(base_untranslated)} - the base language may not defer "
            "a translation, only ship one.\n"
        )
        return 1

    missing_in_base = sorted(id_ for id_ in all_ids if not base_strings.get(id_))

    other_language_problems: list[str] = []
    for path in sorted(STRINGS_DIR.glob("*.toml")):
        if path.stem == BASE_LANGUAGE:
            continue
        strings, untranslated, machine = load_strings(path, shape_problems)
        human_report.append(
            f"{path.name}: {machine} of {len(strings)} human = false"
        )
        untranslated_set = set(untranslated)
        both = untranslated_set & {id_ for id_ in strings if strings.get(id_)}
        for id_ in sorted(both):
            other_language_problems.append(
                f"{path.name}: {id_!r} is marked [untranslated] and also translated - "
                "delete it from [untranslated]"
            )
        stale = untranslated_set - all_ids
        for id_ in sorted(stale):
            other_language_problems.append(
                f"{path.name}: [untranslated] names {id_!r}, which nothing in "
                "menu.toml or STRING_CONSUMERS references any more - delete the row"
            )
        for id_ in sorted(all_ids):
            if strings.get(id_) or id_ in untranslated_set:
                continue
            other_language_problems.append(
                f"{path.name}: {id_!r} has neither a translation nor an "
                "[untranslated] marker - a player reading this language sees "
                "nothing this script can vouch for"
            )

    # A title's disc-keyed translations: same entry shape, ids are the disc's
    # own, so they must never carry our `OAG_` prefix (that space is the
    # project file's) and are not required to cover anything.
    for path in sorted((STRINGS_DIR / "disc").glob("*/*.toml")):
        strings, _, machine = load_strings(path, shape_problems)
        rel = path.relative_to(STRINGS_DIR)
        human_report.append(f"{rel}: {machine} of {len(strings)} human = false")
        for id_ in sorted(strings):
            if id_.startswith("OAG_"):
                shape_problems.append(
                    f"{rel}: {id_!r} is one of ours; it belongs in the "
                    "project file, not a title's disc-keyed one"
                )

    report(
        "string file entries of the wrong shape:",
        shape_problems,
        "Every entry is `ID = { text = \"...\", human = false }`; see "
        "assets/ui/strings/english.toml's header.",
    )
    report(
        "menu row(s) with no string_id and not in BASELINE_LABELS:",
        uncovered_labels,
        "A new row needs a string_id and its English text in "
        "assets/ui/strings/english.toml, in the same change. If this row "
        "predates that rule, add it to BASELINE_LABELS instead and say why.",
    )
    report(
        "page title(s) with no title_string_id and not in BASELINE_TITLES:",
        uncovered_titles,
        "A new page needs a title_string_id and its English text in "
        "assets/ui/strings/english.toml, in the same change. If this page "
        "predates that rule, add its id to BASELINE_TITLES instead and say why.",
    )
    report(
        "menu row(s) with a subtitle but no subtitle_string_id and not in "
        "BASELINE_SUBTITLES:",
        uncovered_subtitles,
        "A new subtitle needs a subtitle_string_id and its English text in "
        "assets/ui/strings/english.toml, in the same change. If this row "
        "predates that rule, add it to BASELINE_SUBTITLES instead and say why.",
    )
    report(
        "BASELINE_LABELS row(s) that now carry a string_id - delete them:",
        graduated_labels,
        "The baseline only ever gets shorter. Leaving a row here keeps an exemption alive.",
    )
    report(
        "BASELINE_SUBTITLES row(s) that now carry a subtitle_string_id - delete them:",
        graduated_subtitles,
        "The baseline only ever gets shorter. Leaving a row here keeps an exemption alive.",
    )
    report(
        "BASELINE_TITLES row(s) that now carry a title_string_id - delete them:",
        graduated_titles,
        "The baseline only ever gets shorter. Leaving a row here keeps an exemption alive.",
    )
    report(
        "BASELINE_LABELS row(s) naming a page/label that no longer exists:",
        vanished_labels,
        "Delete them, or a rename carries the old exemption forward under a new label.",
    )
    report(
        "BASELINE_SUBTITLES row(s) naming a page/subtitle that no longer exists:",
        vanished_subtitles,
        "Delete them, or a rename carries the old exemption forward under a new subtitle.",
    )
    report(
        "BASELINE_TITLES row(s) naming a page that no longer exists:",
        vanished_titles,
        "Delete them, or a rename carries the old exemption forward under a new id.",
    )
    report(
        f"string_id(s) with no real text in {base_path.relative_to(ROOT)}:",
        missing_in_base,
        "Every id used anywhere must resolve in the base language - this is "
        "either a typo in the id or a forgotten entry in english.toml.",
    )
    report(
        "other-language string file problem(s):",
        other_language_problems,
        "Every id the base language carries needs either a real translation "
        "or an explicit [untranslated] marker in this file - never a silent "
        "gap. A machine translation is allowed, marked human = false.",
    )

    failed = any(
        [
            uncovered_labels,
            uncovered_titles,
            uncovered_subtitles,
            graduated_labels,
            graduated_titles,
            graduated_subtitles,
            vanished_labels,
            vanished_titles,
            vanished_subtitles,
            missing_in_base,
            other_language_problems,
            shape_problems,
        ]
    )
    if failed:
        return 1

    print(
        f"OK: {len(seen_labels)} menu row(s) ({len(BASELINE_LABELS)} baselined), "
        f"{len(seen_titles)} page title(s) ({len(BASELINE_TITLES)} baselined), "
        f"{len(seen_subtitles)} subtitle(s) ({len(BASELINE_SUBTITLES)} baselined), "
        f"{len(all_ids)} string id(s) all resolve in {BASE_LANGUAGE}.toml"
    )
    for line in human_report:
        print(f"  human flag: {line}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
