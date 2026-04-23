import { useEffect, useState } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { api } from "@/lib/api";
import { errMsg } from "@/lib/err";
import type { Visit } from "@/lib/types";

/// Widok do drukowania/eksportu PDF.  Lekarz klika „Eksport PDF", otwiera
/// się ta strona i automatycznie wywołuje `window.print()`.  System
/// (Windows/macOS/Linux) pokazuje dialog drukowania z opcją „Save as PDF" —
/// to najczystszy sposób na poprawne polskie znaki bez osadzania fontów TTF.
export function PrintView() {
  const { id } = useParams();
  const navigate = useNavigate();
  const [visit, setVisit] = useState<Visit | null>(null);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    if (!id) return;
    api
      .getVisit(id)
      .then(setVisit)
      .catch((e) => setErr(errMsg(e)));
  }, [id]);

  useEffect(() => {
    if (!visit) return;
    const t = setTimeout(() => window.print(), 300);
    return () => clearTimeout(t);
  }, [visit]);

  if (err) {
    return <div className="p-6 text-sm text-red-600">{err}</div>;
  }
  if (!visit) {
    return <div className="p-6 text-sm text-slate-500">Wczytywanie…</div>;
  }

  const summary = visit.summary as
    | {
        chiefComplaint?: string;
        mentalStateExam?: string;
        diagnosis?: string;
        recommendations?: string;
        medications?: string;
      }
    | undefined;

  return (
    <div className="print-page">
      <div className="print-toolbar no-print">
        <button className="btn-secondary" onClick={() => navigate(-1)}>
          ← Wróć
        </button>
        <button className="btn-primary" onClick={() => window.print()}>
          Drukuj / zapisz jako PDF
        </button>
        <span className="text-xs text-slate-500">
          W dialogu drukowania wybierz „Save as PDF" jako drukarkę.
        </span>
      </div>

      <div className="watermark">SZKIC AI — ZWERYFIKUJ</div>

      <header className="print-header">
        <h1>Szkic dokumentacji wizyty</h1>
        <div className="meta">
          <div>Pacjent: <strong>{visit.patientId}</strong></div>
          <div>Data: <strong>{new Date(visit.startedAt).toLocaleString("pl-PL")}</strong></div>
          {visit.modelUsed && <div>Model: {visit.modelUsed}</div>}
        </div>
      </header>

      {summary && (
        <section className="print-section">
          <h2>Wywiad / Chief complaint</h2>
          <p>{summary.chiefComplaint || "—"}</p>

          <h2>Stan psychiczny</h2>
          <p>{summary.mentalStateExam || "—"}</p>

          <h2>Rozpoznanie</h2>
          <p>{summary.diagnosis || "—"}</p>

          <h2>Zalecenia</h2>
          <p>{summary.recommendations || "—"}</p>

          <h2>Leki</h2>
          <p>{summary.medications || "—"}</p>
        </section>
      )}

      {visit.acceptedCodes && visit.acceptedCodes.length > 0 && (
        <section className="print-section">
          <h2>Kody ICD-10</h2>
          <ul>
            {visit.acceptedCodes.map((c) => (
              <li key={c}>
                <strong>{c}</strong>
              </li>
            ))}
          </ul>
        </section>
      )}

      <footer className="print-footer">
        <p>
          Niniejszy dokument jest <strong>szkicem wygenerowanym przez AI</strong> na
          podstawie transkryptu rozmowy lekarz-pacjent. Wymaga weryfikacji i
          akceptacji lekarza przed umieszczeniem w oficjalnej dokumentacji medycznej
          (EDM). Wygenerowano aplikacją rpstr.
        </p>
      </footer>
    </div>
  );
}
