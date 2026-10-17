//! Décodage des octets et détection du séparateur (R-144).

use encoding_rs::{UTF_8, UTF_16BE, UTF_16LE, WINDOWS_1252};

use super::{CsvEncoding, CsvError};

/// Texte décodé : contenu, encodage retenu, présence de séquences
/// invalides remplacées par U+FFFD.
pub(crate) struct Decoded {
    pub(crate) text: String,
    pub(crate) encoding: CsvEncoding,
    pub(crate) malformed: bool,
}

/// Encodage détecté : BOM UTF-8 (retiré) ou UTF-16 (FF FE, FE FF) ; sinon
/// UTF-8 s'il est valide ; sinon Windows-1252.
fn detect(bytes: &[u8]) -> CsvEncoding {
    match bytes {
        [0xEF, 0xBB, 0xBF, ..] => CsvEncoding::Utf8,
        [0xFF, 0xFE, ..] => CsvEncoding::Utf16Le,
        [0xFE, 0xFF, ..] => CsvEncoding::Utf16Be,
        _ if std::str::from_utf8(bytes).is_ok() => CsvEncoding::Utf8,
        _ => CsvEncoding::Windows1252,
    }
}

/// R-144 : décode les octets avec l'encodage imposé ou détecté (BOM retiré
/// s'il correspond). Une séquence invalide est remplacée par U+FFFD.
///
/// # Errors
///
/// [`CsvError::Encoding`] si l'encodage est imposé et que les octets n'y
/// sont pas valides (mauvais choix de l'appelant : refus plutôt qu'un texte
/// faux).
pub(crate) fn decode(bytes: &[u8], forced: Option<CsvEncoding>) -> Result<Decoded, CsvError> {
    let encoding = forced.unwrap_or_else(|| detect(bytes));
    let codec = match encoding {
        CsvEncoding::Utf8 => UTF_8,
        CsvEncoding::Utf16Le => UTF_16LE,
        CsvEncoding::Utf16Be => UTF_16BE,
        CsvEncoding::Windows1252 => WINDOWS_1252,
    };
    let (text, malformed) = match encoding {
        CsvEncoding::Windows1252 => codec.decode_without_bom_handling(bytes),
        _ => codec.decode_with_bom_removal(bytes),
    };
    if malformed && forced.is_some() {
        return Err(CsvError::Encoding);
    }
    Ok(Decoded {
        text: text.into_owned(),
        encoding,
        malformed,
    })
}

/// R-144 : séparateur de la ligne d'en-tête, hors guillemets : le plus
/// fréquent de `;`, tabulation, `,` ; à égalité `;` puis tabulation puis
/// `,` ; aucun : `;`.
pub(crate) fn detect_separator(text: &str) -> u8 {
    let mut counts = [(b';', 0_u32), (b'\t', 0), (b',', 0)];
    let mut quoted = false;
    for c in text.chars() {
        match c {
            '"' => quoted = !quoted,
            '\n' | '\r' if !quoted => break,
            _ if !quoted => {
                for (sep, n) in &mut counts {
                    if u32::from(*sep) == u32::from(c) {
                        *n = n.saturating_add(1);
                    }
                }
            }
            _ => {}
        }
    }
    let mut best = (b';', 0_u32);
    for (sep, n) in counts {
        if n > best.1 {
            best = (sep, n);
        }
    }
    best.0
}
