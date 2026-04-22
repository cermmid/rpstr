//! Wyszukiwanie w bundlowanej bazie ICD-10 (FTS5).
//! Plan: baza `resources/icd10_psychiatric.sqlite` z tabelą `codes(code, label_pl, description)`
//! + virtual `codes_fts USING fts5(label_pl, description, content='codes')`.

use crate::error::Result;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Icd10Match {
    pub code: String,
    pub label_pl: String,
}

#[tauri::command]
pub async fn search_icd10(query: String) -> Result<Vec<Icd10Match>> {
    // TODO(D8): FTS5 MATCH z rankowaniem BM25. Na razie mały in-memory stub dla UI dev.
    let q = query.to_lowercase();
    let seed = [
        ("F32.0", "Epizod depresji łagodny"),
        ("F32.1", "Epizod depresji umiarkowany"),
        ("F32.2", "Epizod depresji ciężki bez objawów psychotycznych"),
        ("F33.1", "Zaburzenia depresyjne nawracające, epizod umiarkowany"),
        ("F41.0", "Zaburzenie lękowe z napadami paniki"),
        ("F41.1", "Zaburzenie lękowe uogólnione"),
        ("F43.1", "Zespół stresu pourazowego (PTSD)"),
        ("F31.1", "Zaburzenie afektywne dwubiegunowe, epizod manii bez objawów psychotycznych"),
        ("F10.2", "Zespół uzależnienia od alkoholu"),
        ("F51.0", "Bezsenność nieorganiczna"),
    ];
    Ok(seed
        .iter()
        .filter(|(_, label)| label.to_lowercase().contains(&q) || q.is_empty())
        .map(|(c, l)| Icd10Match {
            code: (*c).into(),
            label_pl: (*l).into(),
        })
        .collect())
}
