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
/// writes its own name as `Portugu\xeas`, which is Latin-1 and not valid UTF-8,
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
pub(crate) fn expand(blob: &[u8]) -> Result<String> {
    if fexml::is_fexml(blob) {
        return fexml::expand(blob).map_err(|e| anyhow::anyhow!("{e}"));
    }
    Ok(String::from_utf8(blob.to_vec())
        .unwrap_or_else(|e| e.into_bytes().into_iter().map(char::from).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The byte that dropped a language, and the one beside it that did not.
    ///
    /// Both spellings of the same letter, so this is the mixed-encoding release
    /// in two lines: `0xea` alone is Latin-1 and `0xc3 0xaa` is UTF-8, and a
    /// reader that insists on the second loses every file written the first way.
    #[test]
    fn a_latin_1_byte_is_read_rather_than_losing_the_file() {
        let utf8 = expand("Espa\u{f1}ol".as_bytes()).expect("valid UTF-8 decodes as itself");
        assert_eq!(utf8, "Español");

        let latin1 = expand(b"Portugu\xeas").expect("Latin-1 no longer fails");
        assert_eq!(latin1, "Português");
    }

    /// A file with a `<code>` dictionary still goes through the expander, and a
    /// broken one is still an error rather than a mojibake string.
    ///
    /// The fallback deliberately does not reach this branch: a `.fexml` whose
    /// dictionary will not read is a *parse* failure, and decoding its bytes as
    /// Latin-1 would hand the caller a document full of one-letter tag names.
    #[test]
    fn a_dictionary_file_is_expanded_and_a_broken_one_still_fails() {
        let expanded = expand(br#"<code as="alpha"></code><a beta="1"></a>"#).expect("expands");
        assert!(expanded.contains("alpha"), "{expanded}");
        assert!(expand(b"<code").is_err(), "an unterminated dictionary");
    }
}
