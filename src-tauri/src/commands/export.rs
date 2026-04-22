//! Eksport szkicu do PDF (ze znakiem wodnym "szkic AI — zweryfikuj").
//! Plan (D10): generowanie po stronie frontend (jsPDF) jest prostsze i ma pełną
//! kontrolę nad layoutem; backend zwraca treść, frontend renderuje. Tu rezerwuję
//! command na wypadek gdybyśmy przeszli na backendowe generowanie (printpdf).

use crate::error::Result;

#[tauri::command]
pub async fn export_pdf(visit_id: String) -> Result<String> {
    // Zwraca sugerowaną ścieżkę zapisu. TODO(D10).
    eprintln!("[rpstr] export_pdf {visit_id} (stub)");
    Ok(format!("rpstr-szkic-{}.pdf", visit_id))
}
