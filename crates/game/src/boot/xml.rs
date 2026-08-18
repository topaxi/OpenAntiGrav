//! Reading a front-end XML file out of an archive, whatever it is encoded in.
//!
//! Split out of [`super`] under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. One function,
//! because the finding behind it is worth a page of its own and does not belong
//! among the loaders that call it.

use anyhow::Result;
use oag_formats::fexml;

/// Reads a front-end XML as text, expanding the name dictionary if it has one.
///
/// # Not every one of these files is UTF-8
///
/// **A whole language went missing over one byte.** Wipeout HD's sixteen
/// language plugins are plain XML, and fifteen of them are UTF-8; `Portuguese`
/// writes its own name as `Portugu\xe9s`, which is Latin-1 and not valid UTF-8,
/// so `from_utf8` failed, `load_languages` skipped the plugin, and the boot
/// reported fifteen languages with no line saying one had been dropped. Spanish
/// on the same disc is genuinely UTF-8 (`Español`), so this is a mixed-encoding
/// release rather than a Latin-1 one.
///
/// UTF-8 is still tried first and still wins, so nothing that decoded before
/// decodes differently now. Latin-1 is the fallback because it is *total* -
/// every byte is a codepoint - so this can no longer return an encoding error at
/// all, and because on the bytes at issue it agrees with CP1252, the other
/// candidate. What it cannot do is tell a Latin-1 file from a corrupt UTF-8 one;
/// the alternative was losing the file outright.
pub(super) fn expand(blob: &[u8]) -> Result<String> {
    if fexml::is_fexml(blob) {
        return fexml::expand(blob).map_err(|e| anyhow::anyhow!("{e}"));
    }
    Ok(String::from_utf8(blob.to_vec())
        .unwrap_or_else(|e| e.into_bytes().into_iter().map(char::from).collect()))
}
