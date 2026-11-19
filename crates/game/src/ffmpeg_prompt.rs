//! The words of the ffmpeg download: the offer's prompt and the status line while it runs.
//!
//! Plain functions over a lookup, so the session (`app::session::ffmpeg`) and the headless
//! `--menu-page --menu-prompt ffmpeg` capture draw the same text.

use oag_ui_screens::prompt::ConfirmLabels;

/// The status line for a download at `done` of `total` bytes, in the words `say` returns.
pub(crate) fn status_text(done: u64, total: u64, say: impl Fn(&str, &str) -> String) -> String {
    if total > 0 && done >= total {
        return say("OAG_FFMPEG_UNPACKING", "UNPACKING FFMPEG...");
    }
    let size = if total > 0 {
        format!("{} OF {} MB", done / 1_000_000, total / 1_000_000)
    } else {
        format!("{} MB", done / 1_000_000)
    };
    say(
        "OAG_FFMPEG_DOWNLOADING",
        "DOWNLOADING FFMPEG %s - BACK CANCELS",
    )
    .replace("%s", &size)
}

/// The yes/no prompt's words for a download of `megabytes`.
pub(crate) fn prompt_labels(megabytes: u64, say: impl Fn(&str, &str) -> String) -> ConfirmLabels {
    ConfirmLabels {
        title: say("OAG_FFMPEG_TITLE", "DOWNLOAD FFMPEG"),
        message: say(
            "OAG_FFMPEG_ASK",
            "MUSIC AND MOVIES NEED FFMPEG, WHICH THIS COMPUTER LACKS. DOWNLOAD A FREE \
             LGPL BUILD (%s) FROM GITHUB FOR THIS GAME ONLY?",
        )
        .replace("%s", &format!("{megabytes} MB")),
        yes: say("OAG_FFMPEG_YES", "DOWNLOAD"),
        no: say("OAG_FFMPEG_NO", "NO THANKS"),
    }
}

#[cfg(test)]
mod tests;
