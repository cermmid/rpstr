//! Lokalny preprocessing transkryptu PRZED wysyłką do Claude (albo lokalnego LLM).
//!
//! Cele:
//!   1. Pseudonimizacja — usunięcie imion, PESEL, adresów. WYMAGANE dla BYOK (RODO art. 9).
//!   2. Redukcja inputu — filler words, powtórzenia, nadmiarowe ciszy. Cel: -15-25% tokenów.
//!
//! Plan implementacji:
//!   - Pseudonimizacja: regex na PESEL (11 cyfr), adresy (ulica|al\.|os\.)…, numery
//!     telefonów; + słownik polskich imion z `names-crate` albo bundlowany CSV z NIW/GUS.
//!     Zamień na placeholdery: `[IMIĘ]`, `[PESEL]`, `[ADRES]`.
//!   - Filler removal: regex `\b(yyy+|eee+|mhm|no|yhm)\b`, dedup powtórzeń ("tak tak tak" → "tak"),
//!     kompresja sekwencji znaków interpunkcyjnych.

pub struct PreprocessResult {
    pub text: String,
    pub size_before: usize,
    pub size_after: usize,
    pub removed_phi: u32,
}

pub fn preprocess(transcript: &str) -> PreprocessResult {
    let before = transcript.len();
    // TODO: pełna implementacja. Póki co pass-through, żeby UI działało end-to-end.
    let text = transcript.to_string();
    PreprocessResult {
        text,
        size_before: before,
        size_after: before,
        removed_phi: 0,
    }
}
