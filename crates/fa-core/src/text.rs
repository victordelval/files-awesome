//! Lossless text decoding/encoding.
//!
//! Editors work on `\n`-separated text. Files on disk may use CRLF and/or a
//! UTF-8 BOM. We detect those on read and restore them on write so that an
//! untouched document round-trips byte for byte.

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

const UTF8_BOM: &[u8] = b"\xEF\xBB\xBF";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineEnding {
    #[default]
    Lf,
    Crlf,
}

impl LineEnding {
    /// Platform convention, used for brand-new documents.
    pub fn native() -> Self {
        if cfg!(windows) { Self::Crlf } else { Self::Lf }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextFormat {
    pub line_ending: LineEnding,
    pub bom: bool,
    /// The file mixed several line-ending styles. It will be normalised to
    /// `line_ending` on the next save, so the UI should say so.
    pub mixed_line_endings: bool,
}

impl TextFormat {
    pub fn new_document() -> Self {
        Self {
            line_ending: LineEnding::native(),
            ..Self::default()
        }
    }
}

/// Decode raw file bytes into `\n`-normalised text plus the detected format.
pub fn decode(bytes: &[u8]) -> Result<(String, TextFormat), std::str::Utf8Error> {
    let (bom, body) = match bytes.strip_prefix(UTF8_BOM) {
        Some(rest) => (true, rest),
        None => (false, bytes),
    };
    let raw = std::str::from_utf8(body)?;

    let (mut crlf, mut lf, mut cr) = (0usize, 0usize, 0usize);
    let b = raw.as_bytes();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'\r' if b.get(i + 1) == Some(&b'\n') => {
                crlf += 1;
                i += 1;
            }
            b'\r' => cr += 1,
            b'\n' => lf += 1,
            _ => {}
        }
        i += 1;
    }

    let line_ending = if crlf > lf + cr {
        LineEnding::Crlf
    } else {
        LineEnding::Lf
    };
    let styles_used = [crlf, lf, cr].iter().filter(|n| **n > 0).count();
    let text = if crlf == 0 && cr == 0 {
        raw.to_owned()
    } else {
        normalize(raw).into_owned()
    };

    Ok((
        text,
        TextFormat {
            line_ending,
            bom,
            mixed_line_endings: styles_used > 1,
        },
    ))
}

/// Encode editor text back into the on-disk representation.
pub fn encode(text: &str, format: &TextFormat) -> Vec<u8> {
    let text = normalize(text);
    let mut out = Vec::with_capacity(text.len() + 3 + text.len() / 32);
    if format.bom {
        out.extend_from_slice(UTF8_BOM);
    }
    match format.line_ending {
        LineEnding::Lf => out.extend_from_slice(text.as_bytes()),
        LineEnding::Crlf => {
            for (i, part) in text.split('\n').enumerate() {
                if i > 0 {
                    out.extend_from_slice(b"\r\n");
                }
                out.extend_from_slice(part.as_bytes());
            }
        }
    }
    out
}

fn normalize(text: &str) -> Cow<'_, str> {
    if text.contains('\r') {
        Cow::Owned(text.replace("\r\n", "\n").replace('\r', "\n"))
    } else {
        Cow::Borrowed(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(bytes: &[u8]) -> Vec<u8> {
        let (text, format) = decode(bytes).expect("valid utf-8");
        encode(&text, &format)
    }

    #[test]
    fn lf_roundtrips_exactly() {
        let src = b"# T\xC3\xADtulo\n\nP\xC3\xA1rrafo con *\xC3\xA9nfasis*.\n";
        assert_eq!(roundtrip(src), src);
    }

    #[test]
    fn crlf_roundtrips_exactly() {
        let src = b"# Title\r\n\r\n- a\r\n- b\r\n";
        let (text, format) = decode(src).unwrap();
        assert_eq!(text, "# Title\n\n- a\n- b\n");
        assert_eq!(format.line_ending, LineEnding::Crlf);
        assert!(!format.mixed_line_endings);
        assert_eq!(encode(&text, &format), src);
    }

    #[test]
    fn bom_is_preserved() {
        let src = b"\xEF\xBB\xBFhola\n";
        let (text, format) = decode(src).unwrap();
        assert_eq!(text, "hola\n");
        assert!(format.bom);
        assert_eq!(encode(&text, &format), src);
    }

    #[test]
    fn edge_cases_roundtrip() {
        for src in [
            &b""[..],
            b"no newline",
            b"\n",
            b"\r\n",
            b"a\n\n\n",
            b"\xEF\xBB\xBF",
        ] {
            assert_eq!(roundtrip(src), src, "input {src:?}");
        }
    }

    #[test]
    fn edited_crlf_text_is_written_as_crlf() {
        let (mut text, format) = decode(b"a\r\nb\r\n").unwrap();
        text.push_str("c\n");
        assert_eq!(encode(&text, &format), b"a\r\nb\r\nc\r\n");
    }

    #[test]
    fn mixed_line_endings_are_flagged_and_normalised_to_majority() {
        let (text, format) = decode(b"a\r\nb\r\nc\nd\r").unwrap();
        assert_eq!(text, "a\nb\nc\nd\n");
        assert!(format.mixed_line_endings);
        assert_eq!(format.line_ending, LineEnding::Lf); // 2 CRLF vs 1 LF + 1 CR: tie → LF
        let (_, format) = decode(b"a\r\nb\r\nc\r\nd\n").unwrap();
        assert_eq!(format.line_ending, LineEnding::Crlf);
    }

    #[test]
    fn invalid_utf8_is_rejected_not_mangled() {
        let err = decode(b"ok \xFF bad").unwrap_err();
        assert_eq!(err.valid_up_to(), 3);
    }
}
